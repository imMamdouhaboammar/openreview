pub mod actions;
pub mod finder;
pub mod normalizer;
pub mod probe;
pub mod pruner;
pub mod replay;
pub mod security;
pub mod settlement;
pub mod text;

pub use actions::{ActionEngine, ActionResult, E2eAction, Locator};
pub use finder::{E2eErrorFinder, E2eFinding, FindingSeverity};
pub use normalizer::SemanticNormalizer;
pub use pruner::{DomPruner, SemanticElement};
pub use replay::{RecordedStep, ReplayAnchor, ReplayEngine, ReplayManifest};
pub use security::SecuritySanitizer;
pub use settlement::{SettlementConfig, SettlementOracle};
pub use text::truncate_with_ellipsis;

use std::path::PathBuf;

/// High-level coordinator for E2E testing, error finding, and replay verification.
#[derive(Clone)]
pub struct E2eSubsystem {
    pub normalizer: SemanticNormalizer,
    pub sanitizer: SecuritySanitizer,
    pub pruner: DomPruner,
    pub action_engine: ActionEngine,
}

impl Default for E2eSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl E2eSubsystem {
    pub fn new() -> Self {
        Self {
            normalizer: SemanticNormalizer::new(),
            sanitizer: SecuritySanitizer::new(),
            pruner: DomPruner::default(),
            action_engine: ActionEngine::new(),
        }
    }

    /// Create a replay engine bound to a cache directory.
    pub fn replay_engine(&self, cache_dir: PathBuf) -> ReplayEngine {
        ReplayEngine::new(cache_dir)
    }

    /// Create an error finder instance.
    pub fn error_finder(&self) -> E2eErrorFinder {
        E2eErrorFinder::new()
    }
}
