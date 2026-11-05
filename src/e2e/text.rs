/// Bound display text by UTF-8 bytes while preserving complete code points.
///
/// The suffix is included in the byte budget. Calling this function on
/// untrusted page text or CLI input must never panic at a multibyte boundary.
pub fn truncate_with_ellipsis(input: &str, max_bytes: usize) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }
    if max_bytes <= 3 {
        return ".".repeat(max_bytes);
    }

    let mut end = max_bytes - 3;
    while !input.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...", &input[..end])
}

#[cfg(test)]
mod tests {
    use super::truncate_with_ellipsis;

    #[test]
    fn ascii_behavior_remains_bounded_and_compatible() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
        assert_eq!(truncate_with_ellipsis("abcdefg", 6), "abc...");
        assert_eq!(truncate_with_ellipsis("long", 0), "");
        assert_eq!(truncate_with_ellipsis("long", 2), "..");
        assert_eq!(truncate_with_ellipsis("a", 1), "a");
    }

    #[test]
    fn unicode_boundaries_never_panic_or_exceed_budget() {
        for text in [
            format!("{}é{}", "a".repeat(76), "b".repeat(8)),
            format!("{}🚀{}", "a".repeat(75), "b".repeat(8)),
            format!("{}مرحبًا{}", "a".repeat(74), "b".repeat(8)),
            format!("{}e\u{0301}{}", "a".repeat(75), "b".repeat(8)),
            "📦".repeat(40),
        ] {
            for limit in 0..=85 {
                let result = truncate_with_ellipsis(&text, limit);
                assert!(result.len() <= limit, "limit {limit}");
                assert!(result.is_char_boundary(result.len()));
                if text.len() > limit && limit >= 3 {
                    assert!(result.ends_with("..."));
                }
            }
        }
    }
}
