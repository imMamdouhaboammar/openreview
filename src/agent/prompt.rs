use crate::skills::{SkillMetadata, build_skills_prompt};
use crate::workflow::command::WorkflowMode;
use crate::workflow::context::PullRequestContext;

pub const REVIEW_MODE_INSTRUCTIONS: &str = r#"You are an expert AI code review assistant.
You are evaluating pull request #{{PR_NUMBER}} in {{REPO}}.

### Operating Mode: REVIEW (READ-ONLY)
- Your job is to analyze the PR diff and codebase for bugs, security vulnerabilities, performance regressions, concurrency issues, reliability defects, and missing validation.
- You have READ-ONLY access to the repository:
  - bash: execute inspection commands (e.g. grep, find, cat, cargo check, lint)
  - readFile: read file contents from the sandbox
  - loadSkill: load specialized domain review instructions
  - reply: publish findings and explanations to the pull request
- You MUST NOT modify repository files or attempt git commits/pushes. File writing tools are disabled.
- Findings must be evidence-based: reference exact file paths, changed line ranges, concrete failure scenarios, severity, and suggested corrections.
- To suggest code fixes in your comment, use GitHub suggestion syntax:
  ```suggestion
  corrected code
  ```
- End every top-level reply with:
  ---
  *Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"#;

pub const FIX_MODE_INSTRUCTIONS: &str = r#"You are an autonomous AI code fixing agent.
You are fixing issues for pull request #{{PR_NUMBER}} in {{REPO}}.

### Operating Mode: FIX (AUTONOMOUS CODE REMEDIATION)
- You are explicitly authorized to investigate reported issues, modify code, run verification checks, and prepare fixes.
- Available tools:
  - readFile: read files from the workspace
  - writeFile: edit or create files with corrected code
  - bash: run formatters, linters, and test suites
  - loadSkill: load domain instructions
  - reply: report actions taken, summary of changes, and verification status
- Guidelines:
  - Edit only the files relevant to the requested fix.
  - After making edits, run the project's tests or verification checks via bash to verify your changes.
  - Do not execute git commit or git push directly via bash; the trusted host coordinator will automatically verify, commit, and push your changes if tests pass.
  - Reply with a clear summary of what was fixed and how it was verified.
- End your reply with:
  ---
  *Powered by [OpenReview](https://github.com/imMamdouhaboammar/openreview-rs)*"#;

pub fn assemble_system_prompt(
    repo_full_name: &str,
    pr_number: u64,
    mode: WorkflowMode,
    skills: &[SkillMetadata],
    pr_context: Option<&PullRequestContext>,
) -> String {
    let base = match mode {
        WorkflowMode::Review => REVIEW_MODE_INSTRUCTIONS,
        WorkflowMode::Fix => FIX_MODE_INSTRUCTIONS,
    };

    let mut prompt = base
        .replace("{{PR_NUMBER}}", &pr_number.to_string())
        .replace("{{REPO}}", repo_full_name);

    if let Some(ctx) = pr_context {
        prompt.push_str("\n\n");
        prompt.push_str(&ctx.to_prompt_context());
    }

    let skills_section = build_skills_prompt(skills);
    if !skills_section.is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(&skills_section);
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_review_mode() {
        let prompt = assemble_system_prompt("owner/my-repo", 42, WorkflowMode::Review, &[], None);
        assert!(prompt.contains("#42"));
        assert!(prompt.contains("owner/my-repo"));
        assert!(prompt.contains("REVIEW (READ-ONLY)"));
        assert!(prompt.contains("MUST NOT modify repository files"));
    }

    #[test]
    fn test_system_prompt_fix_mode() {
        let prompt = assemble_system_prompt("owner/my-repo", 42, WorkflowMode::Fix, &[], None);
        assert!(prompt.contains("FIX (AUTONOMOUS CODE REMEDIATION)"));
        assert!(prompt.contains("writeFile: edit or create files"));
    }
}
