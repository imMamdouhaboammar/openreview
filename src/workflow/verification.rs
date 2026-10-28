use crate::error::Result;
use crate::sandbox::traits::SandboxInstance;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationCheck {
    pub name: String,
    pub command: String,
    pub passed: bool,
    pub exit_code: i32,
    pub output_snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub outcome: VerificationOutcome,
    pub all_passed: bool,
    pub checks: Vec<VerificationCheck>,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationOutcome {
    Passed,
    Failed,
    Unavailable,
    Incomplete,
}

impl VerificationReport {
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("### Verification Gate Results\n\n");

        if self.checks.is_empty() {
            if let Some(reason) = &self.skipped_reason {
                md.push_str(&format!("ℹ️ **No tests executed**: {reason}\n"));
            } else {
                md.push_str("ℹ️ **No automated test suite detected** for this repository.\n");
            }
            md.push_str(
                "\n> **Outcome:** No verification executed. Automatic publishing blocked.\n",
            );
            return md;
        }

        for check in &self.checks {
            let status_icon = if check.passed { "✅" } else { "❌" };
            md.push_str(&format!(
                "- {} **{}** (`{}`): exit code {}\n",
                status_icon, check.name, check.command, check.exit_code
            ));
            if !check.passed && !check.output_snippet.is_empty() {
                md.push_str(&format!("\n```\n{}\n```\n", check.output_snippet.trim()));
            }
        }

        if self.outcome == VerificationOutcome::Passed {
            md.push_str("\n> **Outcome:** All verification checks passed successfully.\n");
        } else {
            md.push_str(
                "\n> ⚠️ **Outcome:** Verification checks failed. Automatic publishing blocked.\n",
            );
        }

        md
    }
}

pub struct ProjectVerifier;

impl ProjectVerifier {
    pub async fn run_verification(sandbox: &dyn SandboxInstance) -> Result<VerificationReport> {
        let work_dir = sandbox.work_dir();
        let mut checks = Vec::new();
        let commands = Self::detect_test_commands(work_dir);

        if commands.is_empty() {
            return Ok(VerificationReport {
                outcome: VerificationOutcome::Unavailable,
                all_passed: false,
                checks: Vec::new(),
                skipped_reason: Some(
                    "No supported test framework detected in repository".to_string(),
                ),
            });
        }

        let mut all_passed = true;
        let mut unavailable = false;
        let mut incomplete = false;

        for (name, cmd) in commands {
            info!("Running verification check '{name}': {cmd}");
            let output = match sandbox.run_bash(&cmd).await {
                Ok(output) => output,
                Err(err) => {
                    all_passed = false;
                    incomplete = true;
                    checks.push(VerificationCheck {
                        name,
                        command: cmd,
                        passed: false,
                        exit_code: -1,
                        output_snippet: err.to_string(),
                    });
                    continue;
                }
            };
            let passed = output.exit_code == 0;
            if !passed {
                all_passed = false;
                unavailable |= output.exit_code == 127;
            }

            let snippet = if !passed {
                let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", output.stdout, output.stderr);
                truncate_verification_output(&combined, 2000)
            } else {
                String::new()
            };

            checks.push(VerificationCheck {
                name,
                command: cmd,
                passed,
                exit_code: output.exit_code,
                output_snippet: snippet,
            });
        }

        Ok(VerificationReport {
            outcome: if incomplete {
                VerificationOutcome::Incomplete
            } else if unavailable {
                VerificationOutcome::Unavailable
            } else if all_passed {
                VerificationOutcome::Passed
            } else {
                VerificationOutcome::Failed
            },
            all_passed,
            checks,
            skipped_reason: None,
        })
    }

    fn detect_test_commands(dir: &Path) -> Vec<(String, String)> {
        let mut commands = Vec::new();

        if dir.join("Cargo.toml").exists() {
            commands.push(("Cargo Test".to_string(), "cargo test".to_string()));
        } else if dir.join("bun.lock").exists() || dir.join("bun.lockb").exists() {
            commands.push(("Bun Test".to_string(), "bun test".to_string()));
        } else if dir.join("package.json").exists() {
            commands.push(("NPM Test".to_string(), "npm test".to_string()));
        } else if dir.join("go.mod").exists() {
            commands.push(("Go Test".to_string(), "go test ./...".to_string()));
        } else if dir.join("pytest.ini").exists() || dir.join("pyproject.toml").exists() {
            commands.push(("Pytest".to_string(), "pytest".to_string()));
        }

        commands
    }
}

fn truncate_verification_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }
    let mut end = max_bytes;
    while !output.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... (truncated)", &output[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_cargo_tests() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        let commands = ProjectVerifier::detect_test_commands(dir.path());
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].0, "Cargo Test");
    }

    #[test]
    fn test_verification_report_markdown() {
        let report = VerificationReport {
            outcome: VerificationOutcome::Passed,
            all_passed: true,
            checks: vec![VerificationCheck {
                name: "Unit Tests".to_string(),
                command: "cargo test".to_string(),
                passed: true,
                exit_code: 0,
                output_snippet: String::new(),
            }],
            skipped_reason: None,
        };

        let md = report.to_markdown();
        assert!(md.contains("✅ **Unit Tests**"));
        assert!(md.contains("All verification checks passed successfully"));
    }

    #[test]
    fn test_multibyte_verification_output_truncation() {
        let output = format!("{}🚀error", "a".repeat(1999));
        let truncated = truncate_verification_output(&output, 2000);
        assert!(truncated.contains("... (truncated)"));
        assert!(!truncated.contains('🚀'));
    }
}
