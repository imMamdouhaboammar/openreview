use crate::config::Config;
use crate::workflow::WorkflowCoordinator;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

pub struct Metrics {
    pub start_time: Instant,
    pub webhooks_received: AtomicUsize,
    pub reviews_completed: AtomicUsize,
    pub reviews_failed: AtomicUsize,
    pub active_workflows: AtomicUsize,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            webhooks_received: AtomicUsize::new(0),
            reviews_completed: AtomicUsize::new(0),
            reviews_failed: AtomicUsize::new(0),
            active_workflows: AtomicUsize::new(0),
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub coordinator: Arc<WorkflowCoordinator>,
    pub metrics: Arc<Metrics>,
}
