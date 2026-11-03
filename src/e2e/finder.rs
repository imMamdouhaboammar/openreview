use crate::e2e::actions::E2eAction;
use crate::e2e::normalizer::SemanticNormalizer;
use crate::e2e::pruner::DomPruner;
use crate::e2e::security::SecuritySanitizer;
use serde::{Deserialize, Serialize};

/// Severity of an E2E finding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingSeverity {
    Blocker,
    Warning,
    Suggestion,
}

impl FindingSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Blocker => "🔴 BLOCKER",
            Self::Warning => "🟡 WARNING",
            Self::Suggestion => "🔵 SUGGESTION",
        }
    }
}

/// An evidence-gated End-to-End finding ready for PR review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct E2eFinding {
    pub id: String,
    pub title: String,
    pub severity: FindingSeverity,
    pub route: String,
    pub failure_signature: String,
    pub minimal_repro_steps: Vec<E2eAction>,
    pub failing_action_index: usize,
    pub localized_source_file: Option<String>,
    pub suggested_fix_diff: Option<String>,
    pub sanitized_dom_snippet: String,
}

impl E2eFinding {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        let (alert_type, badge) = match self.severity {
            FindingSeverity::Blocker => ("CAUTION", "🔴 BLOCKER"),
            FindingSeverity::Warning => ("WARNING", "🟡 WARNING"),
            FindingSeverity::Suggestion => ("NOTE", "🔵 SUGGESTION"),
        };

        md.push_str(&format!("> [!{alert_type}]\n"));
        md.push_str(&format!("> ### {badge} {}\n", self.title));
        md.push_str(&format!(
            "> **Failure Signature:** `{}`\n",
            self.failure_signature
        ));
        md.push_str(">\n");
        md.push_str("> | Diagnostic Property | Value |\n");
        md.push_str("> | :--- | :--- |\n");
        md.push_str(&format!("> | **Target Route** | `{}` |\n", self.route));
        if let Some(file) = &self.localized_source_file {
            md.push_str(&format!("> | **Causal Source File** | `{}` |\n", file));
        }
        md.push_str(&format!(
            "> | **Failing Action Index** | Step #{} |\n",
            self.failing_action_index
        ));
        md.push_str(&format!(
            "> | **ddmin Reduction** | {} minimal actions |\n\n",
            self.minimal_repro_steps.len()
        ));

        md.push_str("#### ⚡ 1-Minimal Reproduction Sequence (ddmin):\n```json\n");
        if let Ok(steps_json) = serde_json::to_string_pretty(&self.minimal_repro_steps) {
            md.push_str(&steps_json);
        }
        md.push_str("\n```\n");

        if !self.sanitized_dom_snippet.is_empty() {
            md.push_str("\n<details>\n<summary><b>🛡️ Sanitized DOM Evidence (Zero-Trust Redacted)</b></summary>\n\n```html\n");
            md.push_str(&self.sanitized_dom_snippet);
            md.push_str("\n```\n</details>\n");
        }

        if let Some(suggestion) = &self.suggest_code_block() {
            md.push_str("\n#### 🛠️ Suggested Verification Fix:\n");
            md.push_str(suggestion);
            md.push('\n');
        }

        md
    }

    pub fn suggest_code_block(&self) -> Option<String> {
        self.suggested_fix_diff
            .as_ref()
            .map(|diff| format!("```suggestion\n{diff}\n```"))
    }
}

/// Autonomous End-to-End Error Finder and Causal Delta Minimizer.
pub struct E2eErrorFinder {
    pub normalizer: SemanticNormalizer,
    pub sanitizer: SecuritySanitizer,
    pub pruner: DomPruner,
}

impl Default for E2eErrorFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl E2eErrorFinder {
    pub fn new() -> Self {
        Self {
            normalizer: SemanticNormalizer::new(),
            sanitizer: SecuritySanitizer::new(),
            pruner: DomPruner::default(),
        }
    }

    /// Perform Delta Debugging (ddmin) to minimize a failing sequence of E2E actions
    /// down to the 1-minimal subsequence that still reproduces the failure.
    pub async fn minimize_failing_sequence<F, Fut>(
        &self,
        full_sequence: &[E2eAction],
        mut failure_oracle: F,
    ) -> Vec<E2eAction>
    where
        F: FnMut(Vec<E2eAction>) -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        if full_sequence.is_empty() {
            return Vec::new();
        }

        // If even the full sequence doesn't fail, return as is
        if !failure_oracle(full_sequence.to_vec()).await {
            return full_sequence.to_vec();
        }

        // Separate leading Navigate action (if any) as an anchor
        let (prefix, mut to_minimize) =
            if matches!(full_sequence.first(), Some(E2eAction::Navigate { .. })) {
                (vec![full_sequence[0].clone()], full_sequence[1..].to_vec())
            } else {
                (Vec::new(), full_sequence.to_vec())
            };

        if to_minimize.is_empty() {
            return prefix;
        }

        let mut granularity = 2;

        while to_minimize.len() >= 2 {
            let n = to_minimize.len();
            let chunk_size = n.div_ceil(granularity);
            let mut reduced = false;

            // Try each complement (removing a chunk)
            for i in (0..n).step_by(chunk_size) {
                let end = (i + chunk_size).min(n);
                let mut candidate_body = Vec::new();
                candidate_body.extend_from_slice(&to_minimize[..i]);
                candidate_body.extend_from_slice(&to_minimize[end..]);

                let mut candidate = prefix.clone();
                candidate.extend(candidate_body.clone());

                if candidate.len() < (prefix.len() + to_minimize.len())
                    && failure_oracle(candidate).await
                {
                    to_minimize = candidate_body;
                    granularity = (granularity - 1).max(2);
                    reduced = true;
                    break;
                }
            }

            if !reduced {
                if granularity >= to_minimize.len() {
                    break; // 1-minimal reached!
                }
                granularity = (granularity * 2).min(to_minimize.len());
            }
        }

        let mut result = prefix;
        result.extend(to_minimize);
        result
    }

    /// Build a complete, evidence-gated E2eFinding from failure artifacts.
    #[allow(clippy::too_many_arguments)]
    pub fn create_finding(
        &self,
        title: &str,
        route: &str,
        failure_signature: &str,
        minimal_steps: Vec<E2eAction>,
        failing_index: usize,
        source_file: Option<&str>,
        suggested_fix: Option<&str>,
        raw_dom: &str,
    ) -> E2eFinding {
        let cleaned_dom = self.pruner.clean_raw_html(raw_dom);
        let sanitized_dom = self.sanitizer.sanitize(&cleaned_dom);

        E2eFinding {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            severity: FindingSeverity::Blocker,
            route: route.to_string(),
            failure_signature: failure_signature.to_string(),
            minimal_repro_steps: minimal_steps,
            failing_action_index: failing_index,
            localized_source_file: source_file.map(|s| s.to_string()),
            suggested_fix_diff: suggested_fix.map(|s| s.to_string()),
            sanitized_dom_snippet: sanitized_dom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::e2e::actions::Locator;

    #[tokio::test]
    async fn test_ddmin_minimizes_failing_action_sequence() {
        let finder = E2eErrorFinder::new();

        // 5 steps, but only Navigate + Step 3 (Click Checkout) causes the failure
        let full_steps = vec![
            E2eAction::Navigate {
                url: "/cart".to_string(),
            },
            E2eAction::Click {
                locator: Locator::Selector("#banner-close".to_string()),
            },
            E2eAction::Fill {
                locator: Locator::Selector("#promo".to_string()),
                value: "FREE".to_string(),
            },
            E2eAction::Click {
                locator: Locator::Selector("#checkout-btn".to_string()),
            },
            E2eAction::Click {
                locator: Locator::Selector("#footer-link".to_string()),
            },
        ];

        let minimal = finder
            .minimize_failing_sequence(&full_steps, |steps| async move {
                // Fails only if #checkout-btn is clicked
                steps.iter().any(|s| matches!(s, E2eAction::Click { locator: Locator::Selector(sel) } if sel == "#checkout-btn"))
            })
            .await;

        assert!(minimal.len() < full_steps.len());
        assert!(minimal.iter().any(|s| matches!(s, E2eAction::Click { locator: Locator::Selector(sel) } if sel == "#checkout-btn")));
        assert!(matches!(minimal.first(), Some(E2eAction::Navigate { .. })));
    }

    #[test]
    fn test_finding_to_markdown_formatting() {
        let finder = E2eErrorFinder::new();
        let finding = finder.create_finding(
            "Checkout Button Intercepted by Hidden Overlay",
            "/checkout",
            "ElementNotInteractable: Element is covered by .overlay",
            vec![
                E2eAction::Navigate {
                    url: "/checkout".to_string(),
                },
                E2eAction::Click {
                    locator: Locator::TestId("pay-now".to_string()),
                },
            ],
            1,
            Some("src/components/CheckoutModal.tsx"),
            Some("pointer-events: none;"),
            r#"<div class="overlay"></div><button data-testid="pay-now">Pay</button>"#,
        );

        let md = finding.to_markdown();
        assert!(md.contains("🔴 BLOCKER Checkout Button Intercepted by Hidden Overlay"));
        assert!(md.contains("src/components/CheckoutModal.tsx"));
        assert!(md.contains("```suggestion\npointer-events: none;\n```"));
    }
}
