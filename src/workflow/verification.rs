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
            let output = match sandbox.run_verification(&cmd).await {
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
            let execution_confirmed =
                evidence_of_executed_tests(&name, &output.stdout, &output.stderr);
            let passed = output.exit_code == 0 && execution_confirmed;
            if !passed {
                all_passed = false;
                unavailable |= output.exit_code == 127;
                incomplete |= output.exit_code == 0 && !execution_confirmed;
            }

            let snippet = if !passed {
                let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", output.stdout, output.stderr);
                if output.exit_code == 0 && !execution_confirmed {
                    format!(
                        "No executed tests confirmed from this framework's output; publishing blocked.\n{}",
                        truncate_verification_output(&combined, 1800)
                    )
                } else {
                    truncate_verification_output(&combined, 2000)
                }
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

/// A command returning zero is not proof that a test was run. Require a
/// recognizable positive test count/report. Custom test runners whose output
/// cannot be classified are intentionally treated as incomplete verification.
fn evidence_of_executed_tests(name: &str, stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}");
    if name == "Cargo Test" {
        return combined.lines().any(|line| {
            line.trim()
                .strip_prefix("running ")
                .and_then(|rest| rest.split_whitespace().next())
                .and_then(|count| count.parse::<usize>().ok())
                .is_some_and(|count| count > 0)
        });
    }
    if name == "Go Test" {
        return combined.lines().any(|line| {
            let line = line.trim();
            (line.starts_with("ok ") || line.starts_with("ok\t"))
                && !line.contains("[no tests to run]")
        });
    }
    combined.lines().any(|line| {
        let words: Vec<&str> = line.split_whitespace().collect();
        words.windows(2).any(|pair| {
            let count = pair[0].trim_matches(|c: char| !c.is_ascii_digit());
            count.parse::<usize>().is_ok_and(|n| n > 0)
                && matches!(
                    pair[1].trim_matches(|c: char| !c.is_ascii_alphabetic()),
                    "pass" | "passed" | "passing" | "tests"
                )
        }) || words.windows(2).any(|pair| {
            pair[0].trim_matches(|c: char| !c.is_ascii_alphabetic()) == "tests"
                && pair[1]
                    .trim_matches(|c: char| !c.is_ascii_digit())
                    .parse::<usize>()
                    .is_ok_and(|n| n > 0)
        })
    })
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
    use crate::error::AppError;
    use crate::sandbox::traits::CommandOutput;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::tempdir;

    struct VerificationDispatchSandbox {
        root: tempfile::TempDir,
        called: AtomicBool,
        output: String,
    }

    #[async_trait]
    impl SandboxInstance for VerificationDispatchSandbox {
        async fn run_command(&self, _: &str, _: &[&str]) -> Result<CommandOutput> {
            Err(AppError::Sandbox(
                "agent tool path must not run verification".into(),
            ))
        }

        async fn run_bash(&self, _: &str) -> Result<CommandOutput> {
            Err(AppError::Sandbox(
                "short timeout path must not run tests".into(),
            ))
        }

        async fn run_verification(&self, command: &str) -> Result<CommandOutput> {
            assert_eq!(command, "cargo test");
            self.called.store(true, Ordering::SeqCst);
            Ok(CommandOutput {
                exit_code: 0,
                stdout: self.output.clone(),
                stderr: String::new(),
            })
        }

        async fn read_file(&self, _: &str) -> Result<String> {
            Err(AppError::Sandbox("unused".into()))
        }

        async fn write_file(&self, _: &str, _: &str) -> Result<()> {
            Err(AppError::Sandbox("unused".into()))
        }

        async fn has_uncommitted_changes(&self) -> Result<bool> {
            Err(AppError::Sandbox("unused".into()))
        }

        async fn commit_and_push(&self, _: &str, _: &str) -> Result<()> {
            Err(AppError::Sandbox("unused".into()))
        }

        fn work_dir(&self) -> &Path {
            self.root.path()
        }
    }

    #[tokio::test]
    async fn automated_tests_use_a_separate_verification_execution_budget() {
        let root = tempdir().unwrap();
        std::fs::write(root.path().join("Cargo.toml"), "[package]").unwrap();
        let sandbox = VerificationDispatchSandbox {
            root,
            called: AtomicBool::new(false),
            output: "running 1 test\ntest result: ok. 1 passed; 0 failed".into(),
        };
        let report = ProjectVerifier::run_verification(&sandbox).await.unwrap();
        assert!(sandbox.called.load(Ordering::SeqCst));
        assert!(report.all_passed);
    }

    #[tokio::test]
    async fn zero_executed_tests_cannot_authorize_automatic_publishing() {
        let root = tempdir().unwrap();
        std::fs::write(root.path().join("Cargo.toml"), "[package]").unwrap();
        let sandbox = VerificationDispatchSandbox {
            root,
            called: AtomicBool::new(false),
            output: "running 0 tests\ntest result: ok. 0 passed; 0 failed".into(),
        };
        let report = ProjectVerifier::run_verification(&sandbox).await.unwrap();
        assert!(sandbox.called.load(Ordering::SeqCst));
        assert!(!report.all_passed);
        assert_ne!(report.outcome, VerificationOutcome::Passed);
    }

    #[test]
    fn standard_test_runners_require_positive_execution_evidence() {
        let evidence = [
            ("Cargo Test", "running 2 tests\ntest result: ok. 2 passed;"),
            ("Bun Test", "3 pass\n0 fail"),
            ("NPM Test", "Tests: 2 passed, 2 total"),
            ("Go Test", "ok  example/repo 0.012s"),
            (
                "Pytest",
                "================ 4 passed in 0.02s ================",
            ),
        ];
        for (runner, output) in evidence {
            assert!(
                evidence_of_executed_tests(runner, output, ""),
                "missing evidence for {runner}"
            );
        }
        for (runner, output) in [
            ("Cargo Test", "running 0 tests\ntest result: ok. 0 passed;"),
            ("Bun Test", "0 pass\n0 fail"),
            ("NPM Test", "No tests found, exiting with code 0"),
            ("Go Test", "? example/repo [no test files]"),
            ("Pytest", "no tests ran in 0.00s"),
        ] {
            assert!(
                !evidence_of_executed_tests(runner, output, ""),
                "false positive evidence for {runner}"
            );
        }
    }

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
