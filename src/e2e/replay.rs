use crate::e2e::actions::E2eAction;
use crate::e2e::normalizer::SemanticNormalizer;
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Anchor describing an expected DOM node delta.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayAnchor {
    pub role: String,
    pub name: Option<String>,
    pub normalized_text: String,
    pub test_id: Option<String>,
}

/// A recorded step that can be deterministically replayed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordedStep {
    pub step_index: usize,
    pub action: E2eAction,
    pub pre_state_hash: String,
    pub post_state_hash: String,
    pub appeared_anchors: Vec<ReplayAnchor>,
    pub gone_anchors: Vec<ReplayAnchor>,
    pub duration_ms: u64,
}

/// A complete replay manifest for a test suite or user journey.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayManifest {
    pub test_name: String,
    pub initial_route: String,
    pub recorded_at: String,
    pub steps: Vec<RecordedStep>,
}

/// ACID Replay Engine with atomic disk persistence and monotonic deadline polling.
pub struct ReplayEngine {
    cache_dir: PathBuf,
    normalizer: SemanticNormalizer,
    default_timeout: Duration,
}

impl ReplayEngine {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            normalizer: SemanticNormalizer::new(),
            default_timeout: Duration::from_millis(5000),
        }
    }

    /// Load a replay manifest from the cache directory.
    pub fn load_manifest(&self, test_name: &str) -> Result<Option<ReplayManifest>> {
        let file_path = self.manifest_path(test_name);
        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path).map_err(|e| {
            AppError::Workflow(format!(
                "Failed to read replay cache at {}: {e}",
                file_path.display()
            ))
        })?;

        let manifest: ReplayManifest = serde_json::from_str(&content).map_err(|e| {
            AppError::Workflow(format!(
                "Malformed replay cache JSON at {}: {e}",
                file_path.display()
            ))
        })?;

        Ok(Some(manifest))
    }

    /// Save a replay manifest atomically using a temporary file and rename.
    /// Guarantees that crashes mid-write never leave corrupt JSON on disk (Fixes #1371).
    pub fn save_manifest_atomic(&self, manifest: &ReplayManifest) -> Result<()> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            AppError::Workflow(format!(
                "Failed to create cache dir {}: {e}",
                self.cache_dir.display()
            ))
        })?;

        let target_path = self.manifest_path(&manifest.test_name);
        let temp_path = target_path.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));

        let json = serde_json::to_string_pretty(manifest)
            .map_err(|e| AppError::Workflow(format!("Serialization error: {e}")))?;

        fs::write(&temp_path, json)
            .map_err(|e| AppError::Workflow(format!("Failed to write temp cache file: {e}")))?;

        fs::rename(&temp_path, &target_path).map_err(|e| {
            AppError::Workflow(format!("Failed to atomically rename cache file: {e}"))
        })?;

        Ok(())
    }

    /// Poll for an expected end anchor until the monotonic deadline expires (Fixes #1144).
    /// Does not fail immediately under load if the expected DOM state arrives within the deadline.
    pub async fn poll_until_anchor_matches<F, Fut>(
        &self,
        timeout: Option<Duration>,
        expected_anchor: &ReplayAnchor,
        mut current_screen_probe: F,
    ) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = String>,
    {
        let deadline = Instant::now() + timeout.unwrap_or(self.default_timeout);
        let poll_interval = Duration::from_millis(50);

        while Instant::now() < deadline {
            let screen_text = current_screen_probe().await;
            let normalized = self.normalizer.normalize_text(&screen_text);

            if normalized.contains(&expected_anchor.normalized_text) {
                return Ok(());
            }

            tokio::time::sleep(poll_interval).await;
        }

        Err(AppError::Workflow(format!(
            "Replay end-anchor mismatch: expected anchor '{}' did not appear within monotonic deadline",
            expected_anchor.normalized_text
        )))
    }

    fn manifest_path(&self, test_name: &str) -> PathBuf {
        let safe_name: String = test_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.cache_dir.join(format!("{safe_name}.replay.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_atomic_save_and_load_manifest() {
        let dir = tempdir().unwrap();
        let engine = ReplayEngine::new(dir.path().to_path_buf());

        let manifest = ReplayManifest {
            test_name: "checkout_flow".to_string(),
            initial_route: "/cart".to_string(),
            recorded_at: "2026-11-03T11:45:00Z".to_string(),
            steps: vec![RecordedStep {
                step_index: 0,
                action: E2eAction::Navigate {
                    url: "/cart".to_string(),
                },
                pre_state_hash: "hash_0".to_string(),
                post_state_hash: "hash_1".to_string(),
                appeared_anchors: vec![ReplayAnchor {
                    role: "heading".to_string(),
                    name: Some("Your Cart".to_string()),
                    normalized_text: "Your Cart".to_string(),
                    test_id: None,
                }],
                gone_anchors: vec![],
                duration_ms: 120,
            }],
        };

        // Atomic write
        engine.save_manifest_atomic(&manifest).unwrap();

        // Load
        let loaded = engine
            .load_manifest("checkout_flow")
            .unwrap()
            .expect("manifest exists");
        assert_eq!(loaded.test_name, "checkout_flow");
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(
            loaded.steps[0].appeared_anchors[0].normalized_text,
            "Your Cart"
        );
    }

    #[tokio::test]
    async fn test_poll_until_anchor_matches_under_load() {
        let dir = tempdir().unwrap();
        let engine = ReplayEngine::new(dir.path().to_path_buf());

        let anchor = ReplayAnchor {
            role: "status".to_string(),
            name: None,
            normalized_text: "Order [MINTED_ID] placed on [DATE]".to_string(),
            test_id: None,
        };

        let mut attempts = 0;
        let result = engine
            .poll_until_anchor_matches(Some(Duration::from_millis(300)), &anchor, || {
                attempts += 1;
                async move {
                    if attempts < 3 {
                        "Processing order...".to_string()
                    } else {
                        "Order ORD-9921 placed on October 26th successfully!".to_string()
                    }
                }
            })
            .await;

        assert!(result.is_ok());
    }
}
