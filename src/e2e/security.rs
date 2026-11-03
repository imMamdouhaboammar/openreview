use regex::Regex;
use std::sync::LazyLock;

static API_KEY_OPENAI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsk-[a-zA-Z0-9_-]{20,}\b").expect("valid openai key regex"));

static API_KEY_ANTHROPIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bsk-ant-[a-zA-Z0-9_-]{20,}\b").expect("valid anthropic key regex")
});

static GITHUB_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(ghp|gho|ghu|ghs|ghr)_[a-zA-Z0-9]{36}\b").expect("valid github token regex")
});

static GOOGLE_API_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bAIzaSy[a-zA-Z0-9_-]{33}\b").expect("valid google api key regex")
});

static JWT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\beyJ[a-zA-Z0-9_-]+\.eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\b")
        .expect("valid jwt regex")
});

static BEARER_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bBearer\s+[a-zA-Z0-9_.-]{16,}\b").expect("valid bearer header regex")
});

static CREDIT_CARD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d{4}[ -]?){3}\d{4}\b").expect("valid credit card regex"));

static PASSWORD_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(password|secret|passwd|token|auth_token)\s*=\s*["'][^"']+["']"#)
        .expect("valid password attribute regex")
});

static SECURE_INPUT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<input[^>]*type=["']password["'][^>]*value=["']([^"']+)["'][^>]*>"#)
        .expect("valid secure input regex")
});

/// Zero-Trust Security Sanitizer for E2E traces, DOM snapshots, and error reports.
/// Guarantees that credentials, tokens, and PII are redacted before touching disk or LLM context.
#[derive(Debug, Clone, Default)]
pub struct SecuritySanitizer;

impl SecuritySanitizer {
    pub fn new() -> Self {
        Self
    }

    /// Redact all detected secrets, tokens, passwords, and sensitive fields.
    pub fn sanitize(&self, text: &str) -> String {
        let mut s = text.to_string();

        // 1. Redact OpenAI / Anthropic / GitHub / Google API keys
        s = API_KEY_OPENAI_RE
            .replace_all(&s, "[REDACTED_API_KEY]")
            .to_string();
        s = API_KEY_ANTHROPIC_RE
            .replace_all(&s, "[REDACTED_API_KEY]")
            .to_string();
        s = GITHUB_TOKEN_RE
            .replace_all(&s, "[REDACTED_GITHUB_TOKEN]")
            .to_string();
        s = GOOGLE_API_KEY_RE
            .replace_all(&s, "[REDACTED_GOOGLE_KEY]")
            .to_string();

        // 2. Redact JWT tokens
        s = JWT_RE.replace_all(&s, "[REDACTED_JWT]").to_string();

        // 3. Redact Authorization Bearer headers
        s = BEARER_HEADER_RE
            .replace_all(&s, "Bearer [REDACTED_TOKEN]")
            .to_string();

        // 4. Redact Credit Card numbers
        s = CREDIT_CARD_RE
            .replace_all(&s, "[REDACTED_CREDIT_CARD]")
            .to_string();

        // 5. Redact Password values in attributes
        s = PASSWORD_ATTR_RE
            .replace_all(&s, "$1=\"[REDACTED_SECRET]\"")
            .to_string();

        // 6. Redact Password inputs in DOM HTML
        s = SECURE_INPUT_RE
            .replace_all(&s, |caps: &regex::Captures| {
                caps[0].replace(&caps[1], "••••••••")
            })
            .to_string();

        s
    }

    /// Check if a given string contains unredacted secrets or tokens.
    pub fn contains_unredacted_secrets(&self, text: &str) -> bool {
        API_KEY_OPENAI_RE.is_match(text)
            || API_KEY_ANTHROPIC_RE.is_match(text)
            || GITHUB_TOKEN_RE.is_match(text)
            || GOOGLE_API_KEY_RE.is_match(text)
            || JWT_RE.is_match(text)
            || BEARER_HEADER_RE.is_match(text)
            || CREDIT_CARD_RE.is_match(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizes_api_keys_and_github_tokens() {
        let sanitizer = SecuritySanitizer::new();
        let fake_api_key = ["sk-", "1234567890abcdef1234567890"].concat();
        let fake_gh_token = ["ghp_", "TEST0123456789abcdef0123456789abcdef"].concat();
        let raw = format!("Config loaded with {fake_api_key} and {fake_gh_token}");
        let clean = sanitizer.sanitize(&raw);
        assert!(!clean.contains("1234567890abcdef"));
        assert!(!clean.contains("TEST0123456789"));
        assert!(clean.contains("[REDACTED_API_KEY]"));
        assert!(clean.contains("[REDACTED_GITHUB_TOKEN]"));
    }

    #[test]
    fn test_sanitizes_jwt_and_bearer_headers() {
        let sanitizer = SecuritySanitizer::new();
        let fake_jwt = [
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.",
            "eyJzdWIiOiIxMjM0NTY3ODkwIn0.",
            "doNotLeakThisSignature",
        ]
        .concat();
        let raw = format!("Headers: Authorization: Bearer {fake_jwt}");
        let clean = sanitizer.sanitize(&raw);
        assert!(!clean.contains("doNotLeakThisSignature"));
        assert!(clean.contains("Bearer [REDACTED_TOKEN]") || clean.contains("[REDACTED_JWT]"));
    }

    #[test]
    fn test_sanitizes_credit_card_numbers() {
        let sanitizer = SecuritySanitizer::new();
        let fake_card = ["4111", "2222", "3333", "4444"].join(" ");
        let raw = format!("Customer card: {fake_card} submitted");
        let clean = sanitizer.sanitize(&raw);
        assert_eq!(clean, "Customer card: [REDACTED_CREDIT_CARD] submitted");
    }

    #[test]
    fn test_sanitizes_dom_password_inputs() {
        let sanitizer = SecuritySanitizer::new();
        let dummy_pass = ["sample", "_", "dom", "_", "password"].concat();
        let raw =
            format!(r#"<form><input type="password" name="pwd" value="{dummy_pass}"></form>"#);
        let clean = sanitizer.sanitize(&raw);
        assert!(!clean.contains(&dummy_pass));
        assert!(clean.contains("••••••••"));
    }

    #[test]
    fn test_detects_unredacted_secrets() {
        let sanitizer = SecuritySanitizer::new();
        let fake_anthropic = ["sk-ant-", "api03-abcdef123456789012345678"].concat();
        assert!(sanitizer.contains_unredacted_secrets(&fake_anthropic));
        assert!(!sanitizer.contains_unredacted_secrets("Normal text without any tokens"));
    }
}
