use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};

/// Target locator for UI interactions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Locator {
    /// Target by data-testid attribute.
    TestId(String),
    /// Target by role and accessible name (e.g. role="button", name="Submit").
    Role { role: String, name: String },
    /// Target by CSS selector or ID.
    Selector(String),
    /// Target by visible text content.
    Text(String),
    /// Target by input placeholder.
    Placeholder(String),
}

/// First-class actions executable in an E2E step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum E2eAction {
    /// Navigate to a specific URL or relative path.
    Navigate { url: String },
    /// Click an element, with automatic multi-layer piercing through styled wrappers.
    Click { locator: Locator },
    /// Type plain text into an input field.
    Fill { locator: Locator, value: String },
    /// Type sensitive text with fail-closed credential masking.
    FillSecret {
        locator: Locator,
        secret_name: String,
        value: String,
    },
    /// Toggle a checkbox or radio button (piercing styled spans).
    Check { locator: Locator, checked: bool },
    /// Select an option in a dropdown.
    Select { locator: Locator, value: String },
    /// Assert that the current URL matches an expected path or regex pattern (Fixes #1514).
    AssertUrl { expected_pattern: String },
    /// Assert that an HTTP status code matches.
    AssertStatusCode { expected_status: u16 },
    /// Assert that an element is visible on screen.
    AssertVisible { locator: Locator },
    /// Assert that an element contains expected normalized text.
    AssertText {
        locator: Locator,
        expected_text: String,
    },
}

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionResult {
    pub success: bool,
    pub action_name: String,
    pub error_message: Option<String>,
    pub pierced_wrapper: bool,
    pub duration_ms: u64,
}

/// Multi-layer element piercing inspector.
/// Identifies when target elements are hidden behind styled UI wrappers
/// (e.g. Element Plus, Radix, Headless UI, SVG overlay backdrops) and resolves the effective target.
#[derive(Debug, Clone, Default)]
pub struct ActionEngine;

impl ActionEngine {
    pub fn new() -> Self {
        Self
    }

    /// Check if target HTML element is intercepted by a styled wrapper span or overlay,
    /// and resolve the effective dispatch target (piercing the wrapper).
    pub fn resolve_pierced_target(&self, html_snippet: &str) -> (String, bool) {
        // Detect styled radio/checkbox pattern: <label class="el-radio">...<input type="radio" style="display:none">...<span class="el-radio__inner">
        if html_snippet.contains("el-radio") || html_snippet.contains("el-checkbox") {
            return (
                "input[type=\"radio\"], input[type=\"checkbox\"]".to_string(),
                true,
            );
        }

        // Detect Radix / Headless UI hidden input wrappers
        if html_snippet.contains("data-radix-") || html_snippet.contains("data-headlessui-") {
            return ("[data-state], input".to_string(), true);
        }

        // Detect SVG backdrop intercepting clicks: <svg class="backdrop"> or pointer-events: none
        if html_snippet.contains("<svg") && html_snippet.contains("pointer-events") {
            return ("svg > *, [data-interactive=\"true\"]".to_string(), true);
        }

        ("direct".to_string(), false)
    }

    /// Validate URL assertion against the actual active URL.
    pub fn verify_url_assertion(&self, current_url: &str, expected_pattern: &str) -> Result<()> {
        if current_url == expected_pattern
            || current_url.ends_with(expected_pattern)
            || current_url.contains(expected_pattern)
        {
            Ok(())
        } else {
            Err(AppError::Workflow(format!(
                "URL assertion failed: current URL '{current_url}' does not match expected pattern '{expected_pattern}'"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolves_pierced_element_plus_radio_wrappers() {
        let engine = ActionEngine::new();
        let snippet = r#"<label class="el-radio"><span class="el-radio__input"><input type="radio" value="pro" style="display:none"></span><span class="el-radio__label">Pro Plan</span></label>"#;
        let (target, pierced) = engine.resolve_pierced_target(snippet);
        assert!(pierced);
        assert_eq!(target, "input[type=\"radio\"], input[type=\"checkbox\"]");
    }

    #[test]
    fn test_resolves_radix_headless_wrappers() {
        let engine = ActionEngine::new();
        let snippet = r#"<button data-radix-switch data-state="unchecked"><span data-radix-thumb></span></button>"#;
        let (target, pierced) = engine.resolve_pierced_target(snippet);
        assert!(pierced);
        assert_eq!(target, "[data-state], input");
    }

    #[test]
    fn test_verifies_url_assertion() {
        let engine = ActionEngine::new();
        assert!(
            engine
                .verify_url_assertion(
                    "https://app.example.com/settings/billing",
                    "/settings/billing"
                )
                .is_ok()
        );
        assert!(
            engine
                .verify_url_assertion("https://app.example.com/dashboard", "/settings/billing")
                .is_err()
        );
    }
}
