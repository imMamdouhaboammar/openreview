use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Configuration for the adaptive settlement oracle.
#[derive(Debug, Clone)]
pub struct SettlementConfig {
    /// Debounce duration where the state must remain unchanged.
    pub debounce_duration: Duration,
    /// Maximum time to wait before forcing resolution (bounds infinite updates).
    pub max_wait_deadline: Duration,
    /// Poll interval between state checks.
    pub poll_interval: Duration,
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(50),
            max_wait_deadline: Duration::from_millis(3000),
            poll_interval: Duration::from_millis(25),
        }
    }
}

/// Adaptive Settlement Oracle that waits for async mutations, hydration,
/// animations, and network operations to settle without arbitrary fixed sleeps.
pub struct SettlementOracle {
    config: SettlementConfig,
}

impl SettlementOracle {
    pub fn new(config: SettlementConfig) -> Self {
        Self { config }
    }

    /// Wait until a dynamic state producer (e.g. DOM snapshot function) returns
    /// an identical normalized hash for at least `debounce_duration`, or hits `max_wait_deadline`.
    pub async fn wait_for_settlement<F, Fut>(&self, state_probe: F) -> String
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = String>,
    {
        let start = Instant::now();
        let mut last_hash = state_probe().await;
        let mut stable_since = Instant::now();

        while start.elapsed() < self.config.max_wait_deadline {
            sleep(self.config.poll_interval).await;
            let current_hash = state_probe().await;

            if current_hash == last_hash {
                if stable_since.elapsed() >= self.config.debounce_duration {
                    // Stable for the required debounce duration!
                    return current_hash;
                }
            } else {
                // State mutated, reset stability timer
                last_hash = current_hash;
                stable_since = Instant::now();
            }
        }

        // Reached max_wait_deadline, return current hash with graceful settlement
        last_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_settlement_reaches_stability_quickly() {
        let oracle = SettlementOracle::new(SettlementConfig {
            debounce_duration: Duration::from_millis(30),
            max_wait_deadline: Duration::from_millis(500),
            poll_interval: Duration::from_millis(10),
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let c_clone = Arc::clone(&counter);

        // State changes 3 times then stabilizes at "hash_3"
        let hash = oracle
            .wait_for_settlement(move || {
                let c = Arc::clone(&c_clone);
                async move {
                    let val = c.fetch_add(1, Ordering::SeqCst);
                    if val < 3 {
                        format!("hash_{val}")
                    } else {
                        "hash_3".to_string()
                    }
                }
            })
            .await;

        assert_eq!(hash, "hash_3");
    }

    #[tokio::test]
    async fn test_settlement_bounds_infinite_updates_at_deadline() {
        let oracle = SettlementOracle::new(SettlementConfig {
            debounce_duration: Duration::from_millis(50),
            max_wait_deadline: Duration::from_millis(100),
            poll_interval: Duration::from_millis(10),
        });

        let counter = Arc::new(AtomicUsize::new(0));
        let c_clone = Arc::clone(&counter);

        let start = Instant::now();
        // State continuously updates (like a live ticker)
        let _hash = oracle
            .wait_for_settlement(move || {
                let c = Arc::clone(&c_clone);
                async move {
                    let val = c.fetch_add(1, Ordering::SeqCst);
                    format!("ticker_{val}")
                }
            })
            .await;

        let elapsed = start.elapsed();
        // Guaranteed to terminate around max_wait_deadline (100ms)
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed < Duration::from_millis(300));
    }
}
