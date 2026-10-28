use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowMode {
    Review,
    Fix,
}

impl std::fmt::Display for WorkflowMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Review => write!(f, "review"),
            Self::Fix => write!(f, "fix"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommand {
    pub mode: WorkflowMode,
    pub prompt: String,
    pub is_explicit_fix: bool,
}

/// Parses a comment body for bot commands like `@openreview review`, `@openreview fix <instructions>`, etc.
pub fn parse_bot_command(body: &str, bot_username: &str) -> Option<ParsedCommand> {
    let lower_body = body.to_lowercase();
    let lower_bot = format!("@{}", bot_username.to_lowercase());

    let bot_idx = lower_body.find(&lower_bot)?;
    let after_mention = &body[bot_idx + lower_bot.len()..];

    // Trim leading colons, commas, or whitespace
    let trimmed =
        after_mention.trim_start_matches(|c: char| c.is_whitespace() || c == ':' || c == ',');

    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let first_word = parts.next().unwrap_or("").trim().to_lowercase();
    let rest = parts.next().unwrap_or("").trim().to_string();

    match first_word.as_str() {
        "fix" => Some(ParsedCommand {
            mode: WorkflowMode::Fix,
            prompt: rest,
            is_explicit_fix: true,
        }),
        "review" => Some(ParsedCommand {
            mode: WorkflowMode::Review,
            prompt: rest,
            is_explicit_fix: false,
        }),
        "suggest" => Some(ParsedCommand {
            mode: WorkflowMode::Review,
            prompt: rest,
            is_explicit_fix: false,
        }),
        _ => {
            // Default unqualified mention to Review mode with full remaining text as prompt
            let full_prompt = trimmed.to_string();
            Some(ParsedCommand {
                mode: WorkflowMode::Review,
                prompt: full_prompt,
                is_explicit_fix: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_review_command() {
        let cmd =
            parse_bot_command("@openreview review please check security", "openreview").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Review);
        assert_eq!(cmd.prompt, "please check security");
        assert!(!cmd.is_explicit_fix);
    }

    #[test]
    fn test_parse_suggest_command() {
        let cmd = parse_bot_command("@openreview suggest improvements", "openreview").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Review);
        assert_eq!(cmd.prompt, "improvements");
    }

    #[test]
    fn test_parse_fix_command_with_instructions() {
        let cmd = parse_bot_command("@openreview: fix remove unwrap calls", "openreview").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Fix);
        assert_eq!(cmd.prompt, "remove unwrap calls");
        assert!(cmd.is_explicit_fix);
    }

    #[test]
    fn test_parse_fix_command_bare() {
        let cmd = parse_bot_command("@openreview fix", "openreview").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Fix);
        assert_eq!(cmd.prompt, "");
        assert!(cmd.is_explicit_fix);
    }

    #[test]
    fn test_unqualified_mention_defaults_to_review() {
        let cmd = parse_bot_command("@openreview what do you think?", "openreview").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Review);
        assert_eq!(cmd.prompt, "what do you think?");
        assert!(!cmd.is_explicit_fix);
    }

    #[test]
    fn test_custom_bot_username() {
        let cmd = parse_bot_command("Hey @myreviewer fix bug", "myreviewer").unwrap();
        assert_eq!(cmd.mode, WorkflowMode::Fix);
        assert_eq!(cmd.prompt, "bug");
    }

    #[test]
    fn test_no_mention_returns_none() {
        let cmd = parse_bot_command(
            "just a regular comment without any bot mention",
            "openreview",
        );
        assert!(cmd.is_none());
    }
}
