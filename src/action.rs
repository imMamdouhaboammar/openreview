use crate::error::{AppError, Result};
use crate::workflow::command::parse_bot_command;
use crate::workflow::{ThreadMessage, WorkflowMode, WorkflowParams};
use serde_json::Value;

/// Interpret a GitHub-hosted Actions event without trusting PR-controlled text
/// for repository identity, workflow mode, or publishing authorization.
pub fn parse_action_event(
    event_name: &str,
    payload: &Value,
    repository_from_runner: &str,
    bot_username: &str,
) -> Result<Option<WorkflowParams>> {
    if !matches!(
        event_name,
        "issue_comment" | "pull_request_review_comment" | "pull_request"
    ) {
        return Ok(None);
    }
    let advertised_repo = payload["repository"]["full_name"]
        .as_str()
        .ok_or_else(|| AppError::Unauthorized("Actions event has no GitHub repository".into()))?;
    if repository_from_runner.is_empty()
        || !advertised_repo.eq_ignore_ascii_case(repository_from_runner)
    {
        return Err(AppError::Unauthorized(
            "Actions event repository differs from runner's authenticated repository".into(),
        ));
    }
    if payload["sender"]["type"] == "Bot" {
        return Ok(None);
    }

    let action = payload["action"].as_str().unwrap_or_default();
    let (pr_number, comment_id, actor, mode, prompt, head_sha, base_branch, head_branch) =
        match event_name {
            "issue_comment" | "pull_request_review_comment" => {
                if action != "created" {
                    return Ok(None);
                }
                let issue = if event_name == "issue_comment" {
                    if !payload["issue"]["pull_request"].is_object() {
                        return Ok(None);
                    }
                    &payload["issue"]
                } else {
                    &payload["pull_request"]
                };
                if payload["comment"]["user"]["type"] == "Bot" {
                    return Ok(None);
                }
                let body = payload["comment"]["body"].as_str().ok_or_else(|| {
                    AppError::Config("Actions comment event is missing its body".into())
                })?;
                let Some(command) = parse_bot_command(body, bot_username) else {
                    return Ok(None);
                };
                let actor = payload["comment"]["user"]["login"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| {
                        AppError::Unauthorized("Actions comment author is missing".into())
                    })?;
                (
                    issue["number"].as_u64(),
                    payload["comment"]["id"].as_u64(),
                    actor.to_owned(),
                    command.mode,
                    command.prompt,
                    None,
                    String::new(),
                    String::new(),
                )
            }
            "pull_request" => {
                if !matches!(
                    action,
                    "opened" | "reopened" | "synchronize" | "ready_for_review"
                ) {
                    return Ok(None);
                }
                let actor = payload["sender"]["login"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| {
                        AppError::Unauthorized("Actions pull request sender is missing".into())
                    })?;
                let pr = &payload["pull_request"];
                (
                    pr["number"].as_u64(),
                    None,
                    actor.to_owned(),
                    WorkflowMode::Review,
                    "Review this pull request for concrete defects and security risks".into(),
                    pr["head"]["sha"].as_str().map(ToOwned::to_owned),
                    pr["base"]["ref"].as_str().unwrap_or_default().into(),
                    pr["head"]["ref"].as_str().unwrap_or_default().into(),
                )
            }
            _ => unreachable!("supported events listed above"),
        };

    let number = pr_number
        .filter(|n| *n > 0)
        .ok_or_else(|| AppError::Config("Actions event has no valid pull request number".into()))?;
    let instructions = if prompt.trim().is_empty() {
        if mode == WorkflowMode::Fix {
            format!("Investigate and fix verified problems in pull request #{number}")
        } else {
            format!("Review pull request #{number} for concrete issues")
        }
    } else {
        prompt
    };
    Ok(Some(WorkflowParams {
        repo_full_name: advertised_repo.to_string(),
        pr_number: number,
        base_branch,
        pr_branch: head_branch,
        comment_id,
        messages: vec![ThreadMessage {
            role: "user".into(),
            content: instructions,
        }],
        installation_id: None,
        mode,
        actor,
        dry_run: false,
        head_sha,
        from_webhook: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowMode;
    use serde_json::json;

    fn issue_comment(body: &str, username: &str) -> Value {
        json!({
            "action": "created",
            "repository": {"full_name": "example/project"},
            "issue": {"number": 15, "pull_request": {"url": "https://api.github.com/repos/example/project/pulls/15"}},
            "comment": {"id": 44, "body": body, "user": {"login": username, "type": "User"}},
            "sender": {"login": username, "type": "User"}
        })
    }

    #[test]
    fn explicit_fix_comment_uses_original_author_and_exact_pr() {
        let event = issue_comment("@openreview fix repair panic", "maintainer");
        let params = parse_action_event("issue_comment", &event, "example/project", "openreview")
            .unwrap()
            .expect("a user-initiated Fix request");
        assert_eq!(params.mode, WorkflowMode::Fix);
        assert_eq!(params.actor, "maintainer");
        assert_eq!(params.pr_number, 15);
        assert_eq!(params.comment_id, Some(44));
        assert_eq!(params.installation_id, None);
        assert!(!params.from_webhook);
        assert!(params.messages[0].content.contains("repair panic"));
    }

    #[test]
    fn unqualified_comment_is_review_and_nonmentions_are_skipped() {
        let review = issue_comment("@openreview please investigate", "contributor");
        let params = parse_action_event("issue_comment", &review, "example/project", "openreview")
            .unwrap()
            .unwrap();
        assert_eq!(params.mode, WorkflowMode::Review);
        let unrelated = issue_comment("@openreviewer fix something", "contributor");
        assert!(
            parse_action_event("issue_comment", &unrelated, "example/project", "openreview")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn comments_cannot_spoof_runner_repository_or_authorize_the_bot() {
        let event = issue_comment("@openreview fix", "maintainer");
        assert!(
            parse_action_event("issue_comment", &event, "another/repository", "openreview")
                .is_err()
        );
        let mut bot = event;
        bot["comment"]["user"]["type"] = json!("Bot");
        assert!(
            parse_action_event("issue_comment", &bot, "example/project", "openreview")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn pull_request_events_only_initiate_read_only_review() {
        let pr = json!({
            "action": "opened",
            "repository": {"full_name": "example/project"},
            "sender": {"login": "contributor", "type": "User"},
            "pull_request": {
                "number": 17, "head": {"sha": "abcd1234", "ref": "feature"},
                "base": {"ref": "main"}, "title": "Please execute @openreview fix"
            }
        });
        let params = parse_action_event("pull_request", &pr, "example/project", "openreview")
            .unwrap()
            .unwrap();
        assert_eq!(params.mode, WorkflowMode::Review);
        assert_eq!(params.head_sha.as_deref(), Some("abcd1234"));
        assert_eq!(params.pr_number, 17);
        let mut closed = pr;
        closed["action"] = json!("closed");
        assert!(
            parse_action_event("pull_request", &closed, "example/project", "openreview")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn published_composite_action_does_not_provision_sandbox_with_ai_secrets() {
        let action: serde_yaml::Value = serde_yaml::from_str(include_str!("../action.yml"))
            .expect("Action metadata must be valid YAML");
        assert_eq!(action["runs"]["using"].as_str(), Some("composite"));
        let steps = action["runs"]["steps"].as_sequence().unwrap();
        assert!(
            steps[0]["run"]
                .as_str()
                .unwrap()
                .contains("loginctl enable-linger")
        );
        assert_eq!(steps[1]["with"]["rootless"].as_bool(), Some(true));
        assert!(steps.iter().any(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("verify-rootless-sandbox.sh"))
        }));
        let entrypoint = steps.last().unwrap();
        assert_eq!(entrypoint["env"]["SANDBOX_DRIVER"].as_str(), Some("docker"));
        for provider in [
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENROUTER_API_KEY",
            "OPENCODE_API_KEY",
        ] {
            assert!(entrypoint["env"][provider].is_string());
            assert!(
                !steps[..steps.len() - 1]
                    .iter()
                    .any(|step| step["env"][provider].is_string()),
                "provider credentials must not be available while provisioning sandbox"
            );
        }
    }

    #[test]
    fn copyable_workflow_requires_write_permission_and_pr_event() {
        let example: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../examples/github-actions.yml"))
                .expect("example Actions workflow must be valid YAML");
        assert!(example["on"]["issue_comment"].is_mapping());
        assert!(example["on"]["pull_request"].is_mapping());
        assert_eq!(example["permissions"]["contents"].as_str(), Some("write"));
        let job = &example["jobs"]["openreview"];
        assert!(job["if"].as_str().unwrap().contains("pull_request"));
        assert_eq!(job["timeout-minutes"].as_u64(), Some(45));
    }

    #[test]
    fn hosted_action_exposes_scoped_custom_opencode_configuration() {
        let action: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../action.yml")).unwrap();
        for input in ["opencode_custom_url", "opencode_custom_protocol"] {
            assert!(
                action["inputs"][input].is_mapping(),
                "missing input: {input}"
            );
        }
        let entrypoint = action["runs"]["steps"]
            .as_sequence()
            .unwrap()
            .last()
            .unwrap();
        for variable in ["OPENCODE_CUSTOM_URL", "OPENCODE_CUSTOM_PROTOCOL"] {
            assert!(
                entrypoint["env"][variable].is_string(),
                "missing {variable}"
            );
        }
        assert_eq!(
            entrypoint["env"]["OPENCODE_API_KEY"].as_str(),
            Some("${{ inputs.opencode_api_key }}")
        );
    }

    #[test]
    fn malformed_pr_commands_fail_closed() {
        let mut event = issue_comment("@openreview fix", "maintainer");
        event["issue"]["number"] = json!(0);
        assert!(
            parse_action_event("issue_comment", &event, "example/project", "openreview").is_err()
        );
        event["issue"]["number"] = json!(15);
        event["issue"]
            .as_object_mut()
            .unwrap()
            .remove("pull_request");
        assert!(
            parse_action_event("issue_comment", &event, "example/project", "openreview")
                .unwrap()
                .is_none()
        );
    }
}
