use crate::llm::types::ToolDefinition;
use serde_json::json;

pub const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;

pub fn get_standard_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "bash".to_string(),
            description: "Execute bash commands in the sandbox environment. Working directory is repo root. All commands execute from this directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "readFile".to_string(),
            description: "Read the contents of a file from the repository sandbox.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read relative to repo root"
                    }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "writeFile".to_string(),
            description: "Write content to a file in the sandbox. Creates parent directories if needed.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The relative path where the file should be written"
                    },
                    "content": {
                        "type": "string",
                        "description": "The full text content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "loadSkill".to_string(),
            description: "Load specialized review instructions for a specific domain. Only use this when a request matches an available skill.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "The skill name to load"
                    }
                },
                "required": ["name"]
            }),
        },
        ToolDefinition {
            name: "reply".to_string(),
            description: "Post a top-level review comment on the pull request to share findings, explain issues, or confirm fixes.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "body": {
                        "type": "string",
                        "description": "The markdown-formatted comment body to post"
                    }
                },
                "required": ["body"]
            }),
        },
    ]
}

pub fn truncate_tool_output(output: String) -> String {
    if output.len() <= MAX_TOOL_OUTPUT_CHARS {
        output
    } else {
        let truncated = &output[..MAX_TOOL_OUTPUT_CHARS];
        let diff = output.len() - MAX_TOOL_OUTPUT_CHARS;
        format!("{truncated}\n\n... (truncated {diff} chars)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncation_under_limit() {
        let text = "short output".to_string();
        assert_eq!(truncate_tool_output(text.clone()), text);
    }

    #[test]
    fn test_truncation_over_limit() {
        let long_text = "a".repeat(12_000);
        let result = truncate_tool_output(long_text);
        assert!(result.contains("... (truncated 2000 chars)"));
        assert!(result.starts_with(&"a".repeat(10_000)));
    }
}
