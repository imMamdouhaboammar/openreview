use crate::error::Result;
use crate::jobs::types::{JobRecord, JobStatus};
use crate::workflow::command::WorkflowMode;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::info;
use uuid::Uuid;

pub struct JobManager {
    jobs: Arc<RwLock<HashMap<String, JobRecord>>>,
    deliveries: Arc<RwLock<HashSet<String>>>,
    concurrency_semaphore: Arc<Semaphore>,
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new(10) // default max 10 concurrent active jobs
    }
}

impl JobManager {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            deliveries: Arc::new(RwLock::new(HashSet::new())),
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    pub async fn is_delivery_processed(&self, delivery_id: &str) -> bool {
        let guard = self.deliveries.read().await;
        guard.contains(delivery_id)
    }

    pub async fn register_delivery(&self, delivery_id: &str) -> bool {
        let mut guard = self.deliveries.write().await;
        guard.insert(delivery_id.to_string())
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
    ) -> Result<JobRecord> {
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

        let mut guard = self.jobs.write().await;
        guard.insert(id, record.clone());

        info!(
            "Registered new job {} (delivery: {}) for {} #{}",
            record.id, delivery_id, repo_full_name, pr_number
        );

        Ok(record)
    }

    pub async fn update_status(&self, job_id: &str, status: JobStatus, error: Option<String>) {
        let mut guard = self.jobs.write().await;
        if let Some(job) = guard.get_mut(job_id) {
            job.status = status;
            job.updated_at = Utc::now();
            if error.is_some() {
                job.error_message = error;
            }
        }
    }

    pub async fn get_job(&self, job_id: &str) -> Option<JobRecord> {
        let guard = self.jobs.read().await;
        guard.get(job_id).cloned()
    }

    pub async fn list_jobs(&self) -> Vec<JobRecord> {
        let guard = self.jobs.read().await;
        let mut list: Vec<JobRecord> = guard.values().cloned().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
        list
    }

    pub fn semaphore(&self) -> Arc<Semaphore> {
        self.concurrency_semaphore.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_lifecycle_and_deduplication() {
        let manager = JobManager::new(5);

        assert!(!manager.is_delivery_processed("delivery-1").await);
        assert!(manager.register_delivery("delivery-1").await);
        assert!(manager.is_delivery_processed("delivery-1").await);

        let job = manager
            .create_job(
                "delivery-1",
                "issue_comment",
                "owner/repo",
                42,
                Some(12345),
                "alice",
                WorkflowMode::Review,
            )
            .await
            .unwrap();

        assert_eq!(job.status, JobStatus::Received);

        manager
            .update_status(&job.id, JobStatus::Running, None)
            .await;
        let updated = manager.get_job(&job.id).await.unwrap();
        assert_eq!(updated.status, JobStatus::Running);

        manager
            .update_status(&job.id, JobStatus::Completed, None)
            .await;
        let completed = manager.get_job(&job.id).await.unwrap();
        assert_eq!(completed.status, JobStatus::Completed);
    }
}
