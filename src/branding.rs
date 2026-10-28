//! Consistent identity for PR conversation comments.
//!
//! Keep this presentation-only: it must never change review findings,
//! verification outcomes, or the authority needed to publish a fix.

const LEGACY_FOOTER: &str =
    "*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*";
const BRAND_HEADER: &str = "<img src=\"https://raw.githubusercontent.com/imMamdouhaboammar/openreview/main/assets/openreview-mark.svg\" alt=\"OpenReview logo\" width=\"24\" height=\"24\" /> **[OpenReview](https://github.com/imMamdouhaboammar/openreview)**";
const BRAND_FOOTER: &str = "_OpenReview · Review with evidence. Fix with proof._";

pub fn format_pr_comment(body: &str) -> Option<String> {
    let mut content = body.trim();
    if content.is_empty() {
        return None;
    }

    // The older agent prompt includes a legacy signature. Remove only the
    // exact trailing signature and optional separator, not findings or prose.
    if let Some(before) = content.strip_suffix(LEGACY_FOOTER) {
        content = before.trim_end();
        if let Some(before_separator) = content.strip_suffix("---") {
            content = before_separator.trim_end();
        }
    }

    // The formatter may be used again on retries; do not duplicate branding.
    if content.starts_with(BRAND_HEADER) && content.ends_with(BRAND_FOOTER) {
        return Some(content.to_string());
    }

    Some(format!("{BRAND_HEADER}\n\n{content}\n\n{BRAND_FOOTER}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_only_the_trailing_legacy_footer() {
        let body = format!(
            "### Review\n\nP1 · `src/lib.rs:12`: panic on empty input.\n\n---\n{LEGACY_FOOTER}"
        );
        let formatted = format_pr_comment(&body).unwrap();
        assert!(formatted.contains("P1 · `src/lib.rs:12`: panic on empty input."));
        assert!(!formatted.contains("openreview-rs"));
        assert_eq!(formatted.matches("OpenReview logo").count(), 1);
        assert_eq!(format_pr_comment(&formatted).unwrap(), formatted);
    }

    #[test]
    fn rejects_empty_and_preserves_plain_text_findings() {
        assert!(format_pr_comment("  \n  ").is_none());
        let formatted = format_pr_comment("No confirmed findings in the inspected diff.").unwrap();
        assert!(formatted.contains("No confirmed findings in the inspected diff."));
        assert!(formatted.ends_with(BRAND_FOOTER));
    }
}
