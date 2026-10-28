use crate::agent::prompt::assemble_system_prompt;
use crate::agent::tools::{get_standard_tools, truncate_tool_output};
use crate::error::{AppError, Result};
use crate::github::client::GitHubClient;
use crate::llm::traits::LlmProvider;
use crate::llm::types::{ChatMessage, ToolCall};
use crate::sandbox::traits::SandboxInstance;
use crate::skills::SkillMetadata;
use tracing::{debug, info, warn};

const MAX_STEPS: usize = 20;
const MAX_TOTAL_TOKENS: usize = 200_000;

#[derive(Clone, Debug, Default)]
pub struct AgentRunResult {
    pub success: bool,
    pub steps_taken: usize,
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub replies_posted: usize,
    pub error_message: Option<String>,
}

pub struct AgentRunner<'a> {
    sandbox: &'a dyn SandboxInstance,
    github: Option<&'a GitHubClient>,
    llm: &'a dyn LlmProvider,
    repo_full_name: &'a str,
    pr_number: u64,
    skills: &'a [SkillMetadata],
}

impl<'a> AgentRunner<'a> {
    pub fn new(
        sandbox: &'a dyn SandboxInstance,
        github: Option<&'a GitHubClient>,
        llm: &'a dyn LlmProvider,
        repo_full_name: &'a str,
        pr_number: u64,
        skills: &'a [SkillMetadata],
    ) -> Self {
        Self {
            sandbox,
            github,
            llm,
            repo_full_name,
            pr_number,
            skills,
        }
    }

    pub async fn run(&self, initial_messages: Vec<ChatMessage>) -> Result<AgentRunResult> {
        let system_prompt =
            assemble_system_prompt(self.repo_full_name, self.pr_number, self.skills);
        let tools = get_standard_tools();

        let mut messages = initial_messages;
        if messages.is_empty() {
            messages.push(ChatMessage::user(format!(
                "Review the changes in pull request #{} on repository {}.",
                self.pr_number, self.repo_full_name
            )));
        }

        let mut total_in = 0;
        let mut total_out = 0;
        let mut replies_count = 0;
        let [owner, repo] = match self.repo_full_name.split_once('/') {
            Some((o, r)) => [o, r],
            None => {
                return Err(AppError::Config(format!(
                    "Invalid repo format: {}",
                    self.repo_full_name
                )));
            }
        };

        info!(
            "Starting autonomous agent loop for PR #{}, repo {}",
            self.pr_number, self.repo_full_name
        );

        for step in 1..=MAX_STEPS {
            if total_in + total_out >= MAX_TOTAL_TOKENS {
                warn!("Token budget exceeded: {}", total_in + total_out);
                break;
            }

            debug!("Executing ReAct agent step {step}/{MAX_STEPS}");

            let response = self.llm.complete(&system_prompt, &messages, &tools).await?;
            total_in += response.input_tokens;
            total_out += response.output_tokens;

            // If assistant produced tool calls, execute them
            if !response.tool_calls.is_empty() {
                messages.push(ChatMessage::assistant_with_tools(
                    response.content.clone(),
                    response.tool_calls.clone(),
                ));

                for call in response.tool_calls {
                    let tool_result_str = self
                        .execute_tool(&call, owner, repo, &mut replies_count)
                        .await;
                    let truncated = truncate_tool_output(tool_result_str);
                    messages.push(ChatMessage::tool_result(call.id, truncated));
                }
            } else {
                // LLM concluded turn
                if let Some(content) = response.content
                    && !content.trim().is_empty()
                    && replies_count == 0
                    && let Some(gh) = self.github
                {
                    let formatted_reply = if content.contains("Powered by [OpenReview]") {
                        content
                    } else {
                        format!(
                            "{content}\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"
                        )
                    };
                    if let Err(e) = gh
                        .post_comment(owner, repo, self.pr_number, &formatted_reply)
                        .await
                    {
                        warn!("Failed to post fallback PR comment: {e}");
                    } else {
                        replies_count += 1;
                    }
                }
                info!("Agent finished conversation in {step} steps with {replies_count} replies");
                return Ok(AgentRunResult {
                    success: true,
                    steps_taken: step,
                    total_input_tokens: total_in,
                    total_output_tokens: total_out,
                    replies_posted: replies_count,
                    error_message: None,
                });
            }
        }

        Ok(AgentRunResult {
            success: true,
            steps_taken: MAX_STEPS,
            total_input_tokens: total_in,
            total_output_tokens: total_out,
            replies_posted: replies_count,
            error_message: None,
        })
    }

    async fn execute_tool(
        &self,
        call: &ToolCall,
        owner: &str,
        repo: &str,
        replies_count: &mut usize,
    ) -> String {
        match call.name.as_str() {
            "bash" => {
                let cmd = call.arguments["command"].as_str().unwrap_or_default();
                match self.sandbox.run_bash(cmd).await {
                    Ok(out) => {
                        let combined = format!(
                            "Exit code: {}\nSTDOUT:\n{}\nSTDERR:\n{}",
                            out.exit_code, out.stdout, out.stderr
                        );
                        combined
                    }
                    Err(e) => format!("Error executing bash: {e}"),
                }
            }
            "readFile" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                match self.sandbox.read_file(path).await {
                    Ok(content) => content,
                    Err(e) => format!("Error reading file '{path}': {e}"),
                }
            }
            "writeFile" => {
                let path = call.arguments["path"].as_str().unwrap_or_default();
                let content = call.arguments["content"].as_str().unwrap_or_default();
                match self.sandbox.write_file(path, content).await {
                    Ok(_) => format!("Successfully wrote {} bytes to {path}", content.len()),
                    Err(e) => format!("Error writing file '{path}': {e}"),
                }
            }
            "loadSkill" => {
                let name = call.arguments["name"].as_str().unwrap_or_default();
                let match_skill = self
                    .skills
                    .iter()
                    .find(|s| s.name.eq_ignore_ascii_case(name));
                match match_skill {
                    Some(s) => s.content.clone(),
                    None => {
                        let available: Vec<&str> =
                            self.skills.iter().map(|s| s.name.as_str()).collect();
                        format!(
                            "Skill '{name}' not found. Available skills: {}",
                            available.join(", ")
                        )
                    }
                }
            }
            "reply" => {
                let body = call.arguments["body"].as_str().unwrap_or_default();
                let mut formatted = body.to_string();
                if !formatted.contains("Powered by [OpenReview]") {
                    formatted.push_str("\n\n---\n*Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*");
                }

                if let Some(gh) = self.github {
                    match gh
                        .post_comment(owner, repo, self.pr_number, &formatted)
                        .await
                    {
                        Ok(_) => {
                            *replies_count += 1;
                            "Successfully posted comment on pull request.".to_string()
                        }
                        Err(e) => format!("Failed to post PR comment: {e}"),
                    }
                } else {
                    *replies_count += 1;
                    format!("Dry run / standalone: comment simulated:\n{formatted}")
                }
            }
            other => format!("Unknown tool: {other}"),
        }
    }
}
