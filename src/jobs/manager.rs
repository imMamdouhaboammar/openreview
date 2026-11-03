use crate::error::{AppError, Result};
use crate::jobs::types::{JobRecord, JobStatus};
use crate::workflow::command::WorkflowMode;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::info;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
struct ReviewReservation {
    actor: String,
    repo_full_name: String,
    installation_id: Option<u64>,
    admitted_at: DateTime<Utc>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct StoredState {
    jobs: HashMap<String, JobRecord>,
    deliveries: HashMap<String, DateTime<Utc>>,
    #[serde(default)]
    review_reservations: Vec<ReviewReservation>,
}

/// Maximum number of admitted paid reviews in a sliding 24-hour window.
/// Quotas are charged when the job is accepted, including failed jobs, so
/// repeated failures cannot be used to bypass the operator's spend cap.
#[derive(Clone, Copy, Debug)]
pub struct ReviewQuotaLimits {
    pub actor_per_day: usize,
    pub repo_per_day: usize,
    pub installation_per_day: usize,
}

impl Default for ReviewQuotaLimits {
    fn default() -> Self {
        Self {
            actor_per_day: 2,
            repo_per_day: 10,
            installation_per_day: 30,
        }
    }
}

pub struct JobManager {
    state: Arc<Mutex<StoredState>>,
    persistence_path: Option<PathBuf>,
    concurrency_semaphore: Arc<Semaphore>,
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new(10) // default max 10 concurrent active jobs
    }
}

impl JobManager {
    const MAX_PENDING_JOBS: usize = 500;

    pub fn new(max_concurrency: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(StoredState::default())),
            persistence_path: None,
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    /// Single-instance persistent metadata. Interrupted jobs are explicitly failed
    /// at restart, not silently reported as completed or resumed mid-push.
    pub fn persistent(path: impl Into<PathBuf>, max_concurrency: usize) -> Result<Self> {
        let path = path.into();
        let mut state: StoredState = if path.exists() {
            serde_json::from_slice(
                &std::fs::read(&path)
                    .map_err(|e| AppError::Workflow(format!("Cannot read job state: {e}")))?,
            )
            .map_err(|e| AppError::Workflow(format!("Corrupt job state: {e}")))?
        } else {
            StoredState::default()
        };
        // Upgrade a pre-quota state file conservatively so existing paid
        // Reviews from the last 24h still consume quota after deployment.
        if state.review_reservations.is_empty() {
            state.review_reservations = state
                .jobs
                .values()
                .filter(|job| job.mode == WorkflowMode::Review)
                .map(|job| ReviewReservation {
                    actor: job.actor.clone(),
                    repo_full_name: job.repo_full_name.clone(),
                    installation_id: job.installation_id,
                    admitted_at: job.created_at,
                })
                .collect();
        }
        for job in state.jobs.values_mut() {
            if !matches!(
                job.status,
                JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
            ) {
                job.status = JobStatus::Failed;
                job.updated_at = Utc::now();
                job.error_message =
                    Some("Interrupted by service restart; new authorized request required".into());
            }
        }
        Self::prune(&mut state);
        persist_state(&path, &state)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            persistence_path: Some(path),
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrency)),
        })
    }

    fn save(&self, state: &StoredState) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            persist_state(path, state)?;
        }
        Ok(())
    }

    fn prune(state: &mut StoredState) {
        let cutoff = Utc::now() - Duration::days(7);
        let review_cutoff = Utc::now() - Duration::hours(24);
        state
            .review_reservations
            .retain(|entry| entry.admitted_at > review_cutoff);
        state.deliveries.retain(|_, time| *time >= cutoff);
        if state.deliveries.len() > 20_000 {
            let mut items: Vec<_> = state
                .deliveries
                .iter()
                .map(|(id, ts)| (id.clone(), *ts))
                .collect();
            items.sort_by_key(|(_, ts)| *ts);
            for (id, _) in items.into_iter().take(state.deliveries.len() - 20_000) {
                state.deliveries.remove(&id);
            }
        }
        if state.jobs.len() > 5_000 {
            let mut terminal: Vec<_> = state
                .jobs
                .values()
                .filter(|j| {
                    matches!(
                        j.status,
                        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                    )
                })
                .map(|j| (j.id.clone(), j.updated_at))
                .collect();
            terminal.sort_by_key(|(_, ts)| *ts);
            for (id, _) in terminal.into_iter().take(state.jobs.len() - 5_000) {
                state.jobs.remove(&id);
            }
        }
    }

    pub async fn is_delivery_processed(&self, delivery_id: &str) -> bool {
        self.state.lock().await.deliveries.contains_key(delivery_id)
    }

    pub async fn register_delivery(&self, delivery_id: &str) -> Result<bool> {
        let mut guard = self.state.lock().await;
        if guard.deliveries.contains_key(delivery_id) {
            return Ok(false);
        }
        let before = guard.clone();
        guard.deliveries.insert(delivery_id.to_string(), Utc::now());
        Self::prune(&mut guard);
        if let Err(e) = self.save(&guard) {
            *guard = before;
            return Err(e);
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_job(
        &self,
        delivery_id: &str,
        event_type: &str,
        repo_full_name: &str,
        pr_number: u64,
        installation_id: Option<u64>,
        actor: &str,
        mode: WorkflowMode,
    ) -> Result<Option<JobRecord>> {
        self.create_job_with_limits(
            delivery_id,
            event_type,
            repo_full_name,
            pr_number,
            installation_id,
            actor,
            mode,
            None,
        )
        .await
    }

    /// Both quotas and job reservation run under the same persistence mutex.
    /// Webhook paths must pass a ReviewQuotaLimits to enforce paid work admission.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_job_with_limits(
        &self,
        delivery_id: &str,
        event_type: &str,
        repo_full_name: &str,
        pr_number: u64,
        installation_id: Option<u64>,
        actor: &str,
        mode: WorkflowMode,
        review_limits: Option<ReviewQuotaLimits>,
    ) -> Result<Option<JobRecord>> {
        let mut guard = self.state.lock().await;
        if !delivery_id.is_empty() && guard.deliveries.contains_key(delivery_id) {
            return Ok(None);
        }
        let now = Utc::now();
        if mode == WorkflowMode::Review
            && let Some(limits) = review_limits
        {
            let since = now - Duration::hours(24);
            let recent = guard
                .review_reservations
                .iter()
                .filter(|entry| entry.admitted_at > since)
                .collect::<Vec<_>>();
            if recent
                .iter()
                .filter(|job| {
                    job.actor.eq_ignore_ascii_case(actor) && job.installation_id == installation_id
                })
                .count()
                >= limits.actor_per_day
            {
                return Err(AppError::Unauthorized(
                    "Daily review quota reached for this actor".into(),
                ));
            }
            if recent
                .iter()
                .filter(|job| {
                    job.repo_full_name.eq_ignore_ascii_case(repo_full_name)
                        && job.installation_id == installation_id
                })
                .count()
                >= limits.repo_per_day
            {
                return Err(AppError::Unauthorized(
                    "Daily review quota reached for this repository".into(),
                ));
            }
            if recent
                .iter()
                .filter(|job| job.installation_id == installation_id)
                .count()
                >= limits.installation_per_day
            {
                return Err(AppError::Unauthorized(
                    "Daily review quota reached for this installation".into(),
                ));
            }
        }
        let pending = guard
            .jobs
            .values()
            .filter(|job| matches!(job.status, JobStatus::Received | JobStatus::Running))
            .count();
        if pending >= Self::MAX_PENDING_JOBS {
            return Err(AppError::Workflow(
                "Job backlog capacity reached; retry delivery after active jobs complete".into(),
            ));
        }
        let id = Uuid::new_v4().to_string();

        let record = JobRecord {
            id: id.clone(),
            delivery_id: delivery_id.to_string(),
            event_type: event_type.to_string(),
            repo_full_name: repo_full_name.to_string(),
            pr_number,
            installation_id,
            actor: actor.to_string(),
            mode,
            status: JobStatus::Received,
            created_at: now,
            updated_at: now,
            error_message: None,
        };

        let before = guard.clone();
        guard.jobs.insert(id, record.clone());
        if mode == WorkflowMode::Review && review_limits.is_some() {
            guard.review_reservations.push(ReviewReservation {
                actor: actor.to_string(),
                repo_full_name: repo_full_name.to_string(),
                installation_id,
                admitted_at: now,
            });
        }
        if !delivery_id.is_empty() {
            guard.deliveries.insert(delivery_id.to_string(), now);
        }
        Self::prune(&mut guard);
        if let Err(e) = self.save(&guard) {
            *guard = before;
            return Err(e);
        }

        info!(
            "Registered new job {} (delivery: {}) for {} #{}",
            record.id, delivery_id, repo_full_name, pr_number
        );

        Ok(Some(record))
    }

    pub async fn update_status(
        &self,
        job_id: &str,
        status: JobStatus,
        error: Option<String>,
    ) -> Result<()> {
        let mut guard = self.state.lock().await;
        let before = guard.clone();
        if let Some(job) = guard.jobs.get_mut(job_id) {
            job.status = status;
            job.updated_at = Utc::now();
            if error.is_some() {
                job.error_message = error;
            }
            if let Err(e) = self.save(&guard) {
                *guard = before;
                return Err(e);
            }
        }
        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Option<JobRecord> {
        self.state.lock().await.jobs.get(job_id).cloned()
    }

    pub async fn list_jobs(&self) -> Vec<JobRecord> {
        let guard = self.state.lock().await;
        let mut list: Vec<JobRecord> = guard.jobs.values().cloned().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        list
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.concurrency_semaphore.clone()
    }
}

fn persist_state(path: &Path, state: &StoredState) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Workflow("Invalid job store path".into()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| AppError::Workflow(format!("Cannot create job state directory: {e}")))?;
    let staging = parent.join(format!(".openreview-job-state-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec(state)
        .map_err(|e| AppError::Workflow(format!("Cannot serialize job state: {e}")))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&staging)
        .map_err(|e| AppError::Workflow(format!("Cannot create job transaction: {e}")))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| AppError::Workflow(format!("Cannot persist job transaction: {e}")))?;
    std::fs::rename(&staging, path)
        .map_err(|e| AppError::Workflow(format!("Cannot commit job state: {e}")))?;
    let _ = std::fs::File::open(parent).and_then(|dir| dir.sync_all());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_lifecycle_and_deduplication() {
        let manager = JobManager::new(5);

        assert!(!manager.is_delivery_processed("delivery-1").await);
        assert!(manager.register_delivery("delivery-1").await.unwrap());
        assert!(manager.is_delivery_processed("delivery-1").await);
        assert!(!manager.register_delivery("delivery-1").await.unwrap());

        let job = manager
            .create_job(
                "delivery-2",
                "issue_comment",
                "owner/repo",
                42,
                Some(12345),
                "alice",
                WorkflowMode::Review,
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(job.status, JobStatus::Received);

        manager
            .update_status(&job.id, JobStatus::Running, None)
            .await
            .unwrap();
        let updated = manager.get_job(&job.id).await.unwrap();
        assert_eq!(updated.status, JobStatus::Running);

        manager
            .update_status(&job.id, JobStatus::Completed, None)
            .await
            .unwrap();
        let completed = manager.get_job(&job.id).await.unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn test_restart_recovers_state_and_does_not_replay_interrupted_push() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.json");
        let mgr = JobManager::persistent(&path, 2).unwrap();
        let job = mgr
            .create_job(
                "delivery",
                "issue_comment",
                "org/repo",
                2,
                Some(42),
                "alice",
                WorkflowMode::Fix,
            )
            .await
            .unwrap()
            .unwrap();
        mgr.update_status(&job.id, JobStatus::Running, None)
            .await
            .unwrap();
        drop(mgr);
        let recovered = JobManager::persistent(&path, 2).unwrap();
        assert!(recovered.is_delivery_processed("delivery").await);
        let status = recovered.get_job(&job.id).await.unwrap();
        assert_eq!(status.status, JobStatus::Failed);
        assert!(status.error_message.unwrap().contains("Interrupted"));
    }

    #[tokio::test]
    async fn test_concurrent_registration_creates_one_job() {
        let mgr = Arc::new(JobManager::new(5));
        let mut futures = vec![];
        for _ in 0..24 {
            let mgr = mgr.clone();
            futures.push(tokio::spawn(async move {
                mgr.create_job(
                    "same-delivery",
                    "issue_comment",
                    "org/repo",
                    2,
                    Some(42),
                    "alice",
                    WorkflowMode::Fix,
                )
                .await
                .unwrap()
                .is_some()
            }));
        }
        let mut accepted = 0;
        for future in futures {
            accepted += usize::from(future.await.unwrap());
        }
        assert_eq!(accepted, 1);
        assert_eq!(mgr.list_jobs().await.len(), 1);
    }

    #[tokio::test]
    async fn distinct_concurrent_review_deliveries_cannot_exceed_actor_quota() {
        let manager = Arc::new(JobManager::new(30));
        let limits = ReviewQuotaLimits {
            actor_per_day: 2,
            repo_per_day: 3,
            installation_per_day: 4,
        };
        let mut attempts = Vec::new();
        for n in 0..24 {
            let manager = manager.clone();
            attempts.push(tokio::spawn(async move {
                manager
                    .create_job_with_limits(
                        &format!("unique-delivery-{n}"),
                        "issue_comment",
                        "org/repo",
                        41,
                        Some(101),
                        "public-user",
                        WorkflowMode::Review,
                        Some(limits),
                    )
                    .await
                    .is_ok()
            }));
        }
        let mut accepted = 0;
        for attempt in attempts {
            accepted += usize::from(attempt.await.unwrap());
        }
        assert_eq!(
            accepted, 2,
            "quota must be atomic across unique webhook deliveries"
        );
        assert_eq!(manager.list_jobs().await.len(), 2);
    }

    #[tokio::test]
    async fn distinct_actors_share_repository_and_installation_daily_caps() {
        let manager = JobManager::new(10);
        let limits = ReviewQuotaLimits {
            actor_per_day: 10,
            repo_per_day: 1,
            installation_per_day: 2,
        };
        for (delivery, repo, actor, allowed) in [
            ("one", "org/repo-one", "alice", true),
            ("two", "org/repo-one", "bob", false),
            ("three", "org/repo-two", "charlie", true),
            ("four", "org/repo-three", "dave", false),
        ] {
            let result = manager
                .create_job_with_limits(
                    delivery,
                    "issue_comment",
                    repo,
                    1,
                    Some(100),
                    actor,
                    WorkflowMode::Review,
                    Some(limits),
                )
                .await;
            assert_eq!(
                result.is_ok(),
                allowed,
                "unexpected quota outcome for {delivery}"
            );
        }
        assert_eq!(manager.list_jobs().await.len(), 2);
    }

    #[tokio::test]
    async fn review_quotas_survive_restart_and_expire_after_one_day() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.json");
        let limits = ReviewQuotaLimits {
            actor_per_day: 1,
            repo_per_day: 2,
            installation_per_day: 2,
        };
        let first = JobManager::persistent(&path, 3).unwrap();
        first
            .create_job_with_limits(
                "delivery-a",
                "issue_comment",
                "org/repo",
                4,
                Some(101),
                "alice",
                WorkflowMode::Review,
                Some(limits),
            )
            .await
            .unwrap()
            .unwrap();
        drop(first);
        let restarted = JobManager::persistent(&path, 3).unwrap();
        assert!(
            restarted
                .create_job_with_limits(
                    "delivery-b",
                    "issue_comment",
                    "org/repo",
                    4,
                    Some(101),
                    "alice",
                    WorkflowMode::Review,
                    Some(limits),
                )
                .await
                .is_err(),
            "restart must not reset spend reservations"
        );

        {
            let mut state = restarted.state.lock().await;
            state.jobs.clear(); // Simulate bounded history pruning.
            restarted.save(&state).unwrap();
        }
        assert!(
            restarted
                .create_job_with_limits(
                    "still-exhausted",
                    "issue_comment",
                    "org/repo",
                    4,
                    Some(101),
                    "alice",
                    WorkflowMode::Review,
                    Some(limits),
                )
                .await
                .is_err(),
            "quota reservations must survive truncation of job history"
        );
        {
            let mut state = restarted.state.lock().await;
            for reservation in &mut state.review_reservations {
                reservation.admitted_at = Utc::now() - Duration::hours(25);
            }
            restarted.save(&state).unwrap();
        }
        assert!(
            restarted
                .create_job_with_limits(
                    "delivery-c",
                    "issue_comment",
                    "org/repo",
                    4,
                    Some(101),
                    "alice",
                    WorkflowMode::Review,
                    Some(limits),
                )
                .await
                .unwrap()
                .is_some(),
            "old quota reservations must expire"
        );
    }

    #[tokio::test]
    async fn test_bounded_pending_queue_rejects_excess_work() {
        let mgr = JobManager::new(1);
        for index in 0..JobManager::MAX_PENDING_JOBS {
            mgr.create_job(
                &format!("delivery-{index}"),
                "issue_comment",
                "org/repo",
                1,
                Some(101),
                "alice",
                WorkflowMode::Review,
            )
            .await
            .unwrap()
            .unwrap();
        }
        assert!(
            mgr.create_job(
                "overflow",
                "issue_comment",
                "org/repo",
                1,
                Some(101),
                "alice",
                WorkflowMode::Review
            )
            .await
            .is_err()
        );
    }
}
