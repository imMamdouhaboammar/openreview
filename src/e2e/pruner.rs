use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static SCRIPT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid script regex"));

static STYLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid style regex"));

static NOSCRIPT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").expect("valid noscript regex")
});

static HTML_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").expect("valid comment regex"));

static SVG_BODY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<svg([^>]*)>.*?</svg>").expect("valid svg regex"));

static TAG_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<([a-zA-Z0-9]+)([^>]*)>(.*?)</[a-zA-Z0-9]+>|<([a-zA-Z0-9]+)([^>]*)/>"#)
        .expect("valid tag regex")
});

static ROLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\brole=["']([^"']+)["']"#).expect("valid role regex"));

static ARIA_LABEL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\baria-label=["']([^"']+)["']"#).expect("valid aria-label regex")
});

static ID_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\bid=["']([^"']+)["']"#).expect("valid id regex"));

static TESTID_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\bdata-testid=["']([^"']+)["']"#).expect("valid testid regex"));

static TYPE_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\btype=["']([^"']+)["']"#).expect("valid type regex"));

static NAME_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\bname=["']([^"']+)["']"#).expect("valid name regex"));

static VALUE_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\bvalue=["']([^"']+)["']"#).expect("valid value regex"));

static PLACEHOLDER_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bplaceholder=["']([^"']+)["']"#).expect("valid placeholder regex")
});

static HIDDEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(hidden|aria-hidden=["']true["']|style=["'][^"']*(?:display:\s*none|visibility:\s*hidden)[^"']*["'])"#)
        .expect("valid hidden regex")
});

static STRIP_TAGS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid strip tags regex"));

/// A compact, semantic representation of an interactive or informational node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticElement {
    pub id: usize,
    pub tag: String,
    pub role: Option<String>,
    pub name: Option<String>,
    pub text: String,
    pub test_id: Option<String>,
    pub input_type: Option<String>,
    pub value: Option<String>,
    pub placeholder: Option<String>,
}

impl SemanticElement {
    pub fn to_compact_string(&self) -> String {
        let role_or_tag = self.role.as_deref().unwrap_or(&self.tag);
        let mut desc = format!("[{}] {}", self.id, role_or_tag);

        if !self.text.is_empty() {
            desc.push_str(&format!(" \"{}\"", self.text));
        } else if let Some(name) = &self.name {
            desc.push_str(&format!(" \"{name}\""));
        }

        if let Some(placeholder) = &self.placeholder {
            desc.push_str(&format!(" (placeholder: \"{placeholder}\")"));
        }

        if let Some(test_id) = &self.test_id {
            desc.push_str(&format!(" [data-testid=\"{test_id}\"]"));
        }

        if self.value.as_deref().is_some_and(|value| !value.is_empty()) {
            // A form value may contain arbitrary private data, regardless of type.
            desc.push_str(" (value: \"[REDACTED_FORM_VALUE]\")");
        }

        desc
    }
}

/// Prunes bloated raw HTML DOM into an ultra-lean, token-efficient semantic tree.
#[derive(Debug, Clone)]
pub struct DomPruner {
    pub max_nodes: usize,
    pub max_token_estimate: usize,
}

impl Default for DomPruner {
    fn default() -> Self {
        Self {
            max_nodes: 250,
            max_token_estimate: 2500,
        }
    }
}

impl DomPruner {
    pub fn new(max_nodes: usize, max_token_estimate: usize) -> Self {
        Self {
            max_nodes,
            max_token_estimate,
        }
    }

    /// Clean raw HTML by stripping scripts, styles, SVG internals, and comments.
    pub fn clean_raw_html(&self, html: &str) -> String {
        let no_comments = HTML_COMMENT_RE.replace_all(html, "");
        let no_scripts = SCRIPT_RE.replace_all(&no_comments, "");
        let no_styles = STYLE_RE.replace_all(&no_scripts, "");
        let no_noscript = NOSCRIPT_RE.replace_all(&no_styles, "");
        // Preserve svg tags as single interactive elements without inner path bloat
        let compact_svg = SVG_BODY_RE.replace_all(&no_noscript, "<svg$1 data-icon=\"true\"></svg>");
        compact_svg.to_string()
    }

    /// Extract interactive and semantic elements from HTML.
    pub fn extract_semantic_tree(&self, html: &str) -> Vec<SemanticElement> {
        let cleaned = self.clean_raw_html(html);
        let mut elements = Vec::new();
        let mut id_counter = 1;
        let sanitizer = crate::e2e::SecuritySanitizer::new();

        let interactive_tags = [
            "button", "a", "input", "select", "textarea", "form", "option", "summary", "dialog",
            "h1", "h2", "h3", "h4", "h5", "h6", "nav", "main", "header", "footer", "p", "li",
        ];

        for cap in TAG_ATTR_RE.captures_iter(&cleaned) {
            if elements.len() >= self.max_nodes {
                break;
            }

            let tag = cap
                .get(1)
                .or_else(|| cap.get(4))
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default();

            let attrs = cap
                .get(2)
                .or_else(|| cap.get(5))
                .map(|m| m.as_str())
                .unwrap_or_default();

            let inner = cap.get(3).map(|m| m.as_str()).unwrap_or_default();

            // Skip hidden elements
            if HIDDEN_RE.is_match(attrs) {
                continue;
            }

            let role = ROLE_RE.captures(attrs).map(|c| c[1].to_string());
            let is_interactive = interactive_tags.contains(&tag.as_str())
                || role.is_some()
                || attrs.contains("onclick")
                || attrs.contains("data-testid");

            if !is_interactive && inner.trim().is_empty() {
                continue;
            }

            let test_id = TESTID_ATTR_RE.captures(attrs).map(|c| c[1].to_string());
            let input_type = TYPE_ATTR_RE.captures(attrs).map(|c| c[1].to_string());
            // Preserve only the fact that a value exists, never its contents.
            let value = VALUE_ATTR_RE
                .captures(attrs)
                .map(|_| "[REDACTED_FORM_VALUE]".to_string());
            let placeholder = PLACEHOLDER_ATTR_RE
                .captures(attrs)
                .map(|c| c[1].to_string());
            let name = ARIA_LABEL_RE
                .captures(attrs)
                .or_else(|| NAME_ATTR_RE.captures(attrs))
                .or_else(|| ID_ATTR_RE.captures(attrs))
                .map(|c| c[1].to_string());

            // Extract plain text snippet
            let clean_inner = STRIP_TAGS_RE.replace_all(inner, " ").trim().to_string();

            let display_text = if matches!(tag.as_str(), "textarea" | "select" | "input")
                && !clean_inner.is_empty()
            {
                "[REDACTED_FORM_VALUE]".to_string()
            } else {
                crate::e2e::truncate_with_ellipsis(&clean_inner, 80)
            };

            if is_interactive || !display_text.is_empty() {
                let mut element = SemanticElement {
                    id: id_counter,
                    tag,
                    role,
                    name,
                    text: display_text,
                    test_id,
                    input_type,
                    value,
                    placeholder,
                };
                element.text = sanitizer.sanitize(&element.text);
                for text in [
                    &mut element.role,
                    &mut element.name,
                    &mut element.test_id,
                    &mut element.input_type,
                    &mut element.value,
                    &mut element.placeholder,
                ]
                .into_iter()
                .flatten()
                {
                    *text = sanitizer.sanitize(text);
                }
                elements.push(element);
                id_counter += 1;
            }
        }

        elements
    }

    /// Render semantic elements into a concise, token-budgeted string for LLM ingestion.
    pub fn render_pruned_tree(&self, html: &str) -> String {
        let tree = self.extract_semantic_tree(html);
        let mut out = String::new();
        out.push_str("=== SEMANTIC OBSERVATION TREE ===\n");

        for el in tree {
            let line = el.to_compact_string();
            // Estimate tokens (~4 characters per token)
            if (out.len() + line.len()) / 4 > self.max_token_estimate {
                out.push_str(
                    "\n[observation safely bounded at token limit without truncation corruption]\n",
                );
                break;
            }
            out.push_str(&line);
            out.push('\n');
        }

        // This is the last boundary before agent/tool, CLI and report consumers.
        crate::e2e::SecuritySanitizer::new().sanitize(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strips_scripts_and_styles() {
        let pruner = DomPruner::default();
        let html = r#"
            <div>
                <style>.btn { color: red; }</style>
                <script>console.log("hello");</script>
                <button type="submit">Submit</button>
            </div>
        "#;
        let clean = pruner.clean_raw_html(html);
        assert!(!clean.contains("console.log"));
        assert!(!clean.contains("color: red"));
        assert!(clean.contains("<button type=\"submit\">Submit</button>"));
    }

    #[test]
    fn test_extracts_interactive_elements_with_test_ids() {
        let pruner = DomPruner::default();
        let html = r#"
            <nav>
                <a href="/home" aria-label="Home Link">Home</a>
                <button data-testid="checkout-btn">Checkout</button>
                <input type="text" placeholder="Enter your email" name="email" value="user@example.com" />
            </nav>
        "#;
        let tree = pruner.extract_semantic_tree(html);
        assert_eq!(tree.len(), 3); // a + button + input

        let btn = tree.iter().find(|e| e.tag == "button").unwrap();
        assert_eq!(btn.test_id.as_deref(), Some("checkout-btn"));
        assert_eq!(btn.text, "Checkout");

        let input = tree.iter().find(|e| e.tag == "input").unwrap();
        assert_eq!(input.placeholder.as_deref(), Some("Enter your email"));
        assert_eq!(input.value.as_deref(), Some("[REDACTED_FORM_VALUE]"));
    }

    #[test]
    fn model_visible_observation_never_contains_raw_form_values() {
        let html = r#"
            <input type="password" value="TEST_ONLY_PASSWORD_MARKER" name="password" />
            <input name='email' value='TEST_ONLY_EMAIL_MARKER' type='email'/>
            <input VALUE="TEST_ONLY_UPPERCASE_MARKER" TYPE="password" />
            <input type='hidden' name='access_token' value='TEST_ONLY_TOKEN_MARKER'/>
            <input type='text' name='postal' value='TEST_ONLY_ORDINARY_MARKER'/>
            <textarea name='note'>TEST_ONLY_TEXTAREA_MARKER</textarea>
            <option value='TEST_ONLY_OPTION_MARKER'>United States</option>
            <button data-testid="submit-btn">Submit</button>
        "#;
        let pruner = DomPruner::default();
        let tree = pruner.extract_semantic_tree(html);
        let observation = pruner.render_pruned_tree(html);
        for marker in [
            "TEST_ONLY_PASSWORD_MARKER",
            "TEST_ONLY_EMAIL_MARKER",
            "TEST_ONLY_UPPERCASE_MARKER",
            "TEST_ONLY_TOKEN_MARKER",
            "TEST_ONLY_ORDINARY_MARKER",
            "TEST_ONLY_TEXTAREA_MARKER",
            "TEST_ONLY_OPTION_MARKER",
        ] {
            assert!(
                !observation.contains(marker),
                "Leaked raw form marker {marker}"
            );
            assert!(!tree.iter().any(|node| node.value.as_deref() == Some(marker) || node.text.contains(marker)));
        }
        assert!(observation.contains("submit-btn"));
        assert!(observation.contains("Submit"));
        assert!(observation.contains("email"));
    }

    #[test]
    fn model_visible_observation_sanitizes_secrets_in_text_and_metadata() {
        let fake_key = format!("sk-{}", "a1b2c3d4".repeat(4));
        let html = format!(r#"<h2>{fake_key}</h2><input placeholder="{fake_key}" />"#);
        let observation = DomPruner::default().render_pruned_tree(&html);
        assert!(!observation.contains(&fake_key));
        assert!(observation.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn extracted_semantic_metadata_does_not_retain_personal_data_or_keys() {
        let dummy_email = "only.a.fixture@example.org";
        let dummy_key = format!("sk-{}", "AbCd1234".repeat(4));
        let html = format!(
            r#"<input name="{dummy_key}" placeholder="{dummy_email}" data-testid="{dummy_key}" /><h2>{dummy_email}</h2>"#
        );
        let tree = DomPruner::default().extract_semantic_tree(&html);
        let serialized = serde_json::to_string(&tree).unwrap();
        assert!(!serialized.contains(dummy_email));
        assert!(!serialized.contains(&dummy_key));
        assert!(serialized.contains("[REDACTED_EMAIL]"));
        assert!(serialized.contains("[REDACTED_API_KEY]"));
    }

    #[test]
    fn unicode_character_at_truncation_boundary_does_not_panic() {
        let pruner = DomPruner::default();
        let html = format!("<h1>{}é{}</h1>", "a".repeat(76), "b".repeat(8));
        let tree = pruner.extract_semantic_tree(&html);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].text.ends_with("..."));
        assert!(tree[0].text.is_char_boundary(tree[0].text.len()));
        assert!(tree[0].text.len() <= 80);
    }

    #[test]
    fn test_respects_token_budget_without_breaking_syntax() {
        let pruner = DomPruner::new(100, 50); // Very tight token budget (~200 chars)
        let mut html = String::new();
        for i in 0..50 {
            html.push_str(&format!(
                "<button id=\"btn-{i}\">Button {i} with long description text</button>\n"
            ));
        }
        let rendered = pruner.render_pruned_tree(&html);
        assert!(rendered.contains("[1] button"));
        assert!(
            rendered.contains(
                "[observation safely bounded at token limit without truncation corruption]"
            )
        );
    }
}
