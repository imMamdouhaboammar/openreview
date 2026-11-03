use regex::Regex;
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

static ORDINAL_DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2}(st|nd|rd|th)\b")
        .expect("valid ordinal date regex")
});

static RELATIVE_TIME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(just now|\d+\s*(?:sec|second|min|minute|hr|hour|day|week|month|year)s?\s*ago|yesterday|tomorrow)\b")
        .expect("valid relative time regex")
});

static ISO_TIMESTAMP_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?\b")
        .expect("valid iso timestamp regex")
});

static TIME_CLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{1,2}:\d{2}(?::\d{2})?\s*(?:AM|PM|am|pm)?\b").expect("valid clock time regex")
});

static MINTED_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]{2,6}-\d{3,8}\b").expect("valid minted id regex"));

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
        .expect("valid uuid regex")
});

static NEXTJS_ROUTE_ANNOUNCER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)<next-route-announcer[^>]*>.*?</next-route-announcer>|Route changed to [^\n\r"]+"#,
    )
    .expect("valid nextjs route announcer regex")
});

static WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("valid whitespace regex"));

/// Canonical semantic state normalizer that cleanses dynamic non-deterministic
/// tokens (timestamps, relative times, ordinal dates, minted IDs, and route announcer noise)
/// to produce bulletproof deterministic state hashes for replay caching.
#[derive(Debug, Clone, Default)]
pub struct SemanticNormalizer;

impl SemanticNormalizer {
    pub fn new() -> Self {
        Self
    }

    /// Normalize arbitrary UI text, DOM attributes, or state strings by replacing
    /// dynamic, non-deterministic values with stable semantic placeholders.
    pub fn normalize_text(&self, input: &str) -> String {
        // 1. Strip Next.js route announcer noise and alerts
        let mut s = NEXTJS_ROUTE_ANNOUNCER_RE.replace_all(input, "").to_string();

        // 2. Replace ordinal dates (e.g. "October 26th" -> "[DATE]")
        s = ORDINAL_DATE_RE.replace_all(&s, "[DATE]").to_string();

        // 3. Replace ISO timestamps (e.g. "2026-11-03T11:40:00Z" -> "[TIMESTAMP]")
        s = ISO_TIMESTAMP_RE.replace_all(&s, "[TIMESTAMP]").to_string();

        // 4. Replace relative times (e.g. "5 minutes ago" -> "[RELATIVE_TIME]")
        s = RELATIVE_TIME_RE
            .replace_all(&s, "[RELATIVE_TIME]")
            .to_string();

        // 5. Replace clock times (e.g. "12:30 PM", "14:22:10" -> "[TIME]")
        s = TIME_CLOCK_RE.replace_all(&s, "[TIME]").to_string();

        // 6. Replace minted entity IDs (e.g. "PROJ-016", "INV-1092" -> "[MINTED_ID]")
        s = MINTED_ID_RE.replace_all(&s, "[MINTED_ID]").to_string();

        // 7. Replace UUIDs (e.g. "123e4567-e89b-12d3-a456-426614174000" -> "[UUID]")
        s = UUID_RE.replace_all(&s, "[UUID]").to_string();

        // 8. Collapse repeated whitespace and trim
        let s = WHITESPACE_RE.replace_all(&s, " ").to_string();
        s.trim().to_string()
    }

    /// Compute a deterministic SHA-256 semantic hash of a normalized screen/DOM state.
    pub fn compute_semantic_hash(&self, raw_state: &str) -> String {
        let normalized = self.normalize_text(raw_state);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Compare two screen states for semantic equivalence.
    pub fn states_semantically_equal(&self, state_a: &str, state_b: &str) -> bool {
        self.normalize_text(state_a) == self.normalize_text(state_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizes_ordinal_dates() {
        let normalizer = SemanticNormalizer::new();
        let input1 = "Invoice created on October 26th for client";
        let input2 = "Invoice created on October 27th for client";
        assert_eq!(
            normalizer.normalize_text(input1),
            "Invoice created on [DATE] for client"
        );
        assert_eq!(
            normalizer.normalize_text(input1),
            normalizer.normalize_text(input2)
        );
    }

    #[test]
    fn test_normalizes_relative_times() {
        let normalizer = SemanticNormalizer::new();
        let input1 = "Last updated 5 minutes ago by admin";
        let input2 = "Last updated just now by admin";
        let input3 = "Last updated 2 days ago by admin";
        assert_eq!(
            normalizer.normalize_text(input1),
            normalizer.normalize_text(input2)
        );
        assert_eq!(
            normalizer.normalize_text(input1),
            normalizer.normalize_text(input3)
        );
    }

    #[test]
    fn test_normalizes_minted_ids_and_uuids() {
        let normalizer = SemanticNormalizer::new();
        let input1 = "Record PROJ-016 created with id 550e8400-e29b-41d4-a716-446655440000";
        let input2 = "Record PROJ-999 created with id 123e4567-e89b-12d3-a456-426614174000";
        assert_eq!(
            normalizer.normalize_text(input1),
            normalizer.normalize_text(input2)
        );
        assert_eq!(
            normalizer.normalize_text(input1),
            "Record [MINTED_ID] created with id [UUID]"
        );
    }

    #[test]
    fn test_strips_nextjs_route_announcer_noise() {
        let normalizer = SemanticNormalizer::new();
        let input = "Dashboard Content <next-route-announcer>Route changed to /dashboard</next-route-announcer>";
        assert_eq!(normalizer.normalize_text(input), "Dashboard Content");
    }

    #[test]
    fn test_semantic_hash_deterministic() {
        let normalizer = SemanticNormalizer::new();
        let state1 = "Status: Saved at 10:45 AM on October 26th. Ticket: TASK-101";
        let state2 = "Status: Saved at 11:20 PM on October 27th. Ticket: TASK-888";
        assert_eq!(
            normalizer.compute_semantic_hash(state1),
            normalizer.compute_semantic_hash(state2)
        );
    }
}
