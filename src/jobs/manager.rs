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

#[derive(Clone, Default, Serialize, Deserialize)]
struct StoredState {
    jobs: HashMap<String, JobRecord>,
    deliveries: HashMap<String, DateTime<Utc>>,
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
        let mut guard = self.state.lock().await;
        if !delivery_id.is_empty() && guard.deliveries.contains_key(delivery_id) {
            return Ok(None);
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
        let now = Utc::now();

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
