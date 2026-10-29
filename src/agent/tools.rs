use crate::llm::types::ToolDefinition;
use crate::workflow::command::WorkflowMode;
use serde_json::json;

pub const MAX_TOOL_OUTPUT_CHARS: usize = 10_000;

pub fn get_standard_tools(mode: WorkflowMode) -> Vec<ToolDefinition> {
    let mut tools = vec![ToolDefinition {
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
    }];

    // File writing is strictly limited to Fix mode
    if mode == WorkflowMode::Fix {
        tools.push(ToolDefinition {
            name: "bash".to_string(),
            description: "Execute commands in the isolated, uncredentialed Fix workspace. Git publishing is performed separately by the trusted host.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "command": { "type": "string" } },
                "required": ["command"]
            }),
        });
        tools.push(ToolDefinition {
            name: "writeFile".to_string(),
            description:
                "Write content to a file in the sandbox. Creates parent directories if needed."
                    .to_string(),
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
        });
    }

    tools.push(ToolDefinition {
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
    });

    tools.push(ToolDefinition {
        name: "reply".to_string(),
        description: "Post a review comment or fix report on the pull request.".to_string(),
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
    });

    tools
}

pub fn truncate_tool_output(output: String) -> String {
    if output.len() <= MAX_TOOL_OUTPUT_CHARS {
        output
    } else {
        let mut split_idx = MAX_TOOL_OUTPUT_CHARS;
        while split_idx > 0 && !output.is_char_boundary(split_idx) {
            split_idx -= 1;
        }
        let truncated = &output[..split_idx];
        let diff = output.len() - split_idx;
        format!("{truncated}\n\n... (truncated {diff} bytes)")
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
    fn test_tools_filtered_by_mode() {
        let review_tools = get_standard_tools(WorkflowMode::Review);
        assert!(!review_tools.iter().any(|t| t.name == "writeFile"));
        assert!(review_tools.iter().any(|t| t.name == "readFile"));
        assert!(review_tools.iter().any(|t| t.name == "reply"));
        assert!(!review_tools.iter().any(|t| t.name == "bash"));

        let fix_tools = get_standard_tools(WorkflowMode::Fix);
        assert!(fix_tools.iter().any(|t| t.name == "writeFile"));
        assert!(fix_tools.iter().any(|t| t.name == "readFile"));
    }

    #[test]
    fn test_truncation_over_limit() {
        let long_text = "a".repeat(12_000);
        let result = truncate_tool_output(long_text);
        assert!(result.contains("... (truncated 2000 bytes)"));
        assert!(result.starts_with(&"a".repeat(10_000)));
    }

    #[test]
    fn test_truncation_multibyte_boundary_safety() {
        // Construct string with multibyte characters (Arabic and emojis) around boundary
        let prefix = "a".repeat(MAX_TOOL_OUTPUT_CHARS - 1);
        let multibyte = format!("{prefix}مرحبا بالعالم 🚀🦀🎉");
        // This must not panic when truncating across char boundaries
        let result = truncate_tool_output(multibyte);
        assert!(result.len() > MAX_TOOL_OUTPUT_CHARS - 10);
        assert!(result.contains("truncated"));
    }
}
