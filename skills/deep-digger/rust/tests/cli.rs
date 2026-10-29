use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../evals/fixtures/failing_input.txt")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn missing_output_parent_fails_before_oracle_invocation() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let missing_parent = std::env::temp_dir().join(format!(
        "deep-digger-missing-{}-{nonce}",
        std::process::id()
    ));
    let result = Command::new(env!("CARGO_BIN_EXE_deep-digger-min"))
        .args(["--input", &fixture(), "--output"])
        .arg(missing_parent.join("result.txt"))
        .args(["--allow-exec", "--", "nonexistent-oracle", "{input}"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("output parent directory must exist"));
}

#[test]
fn excessive_timeout_fails_preflight() {
    let result = Command::new(env!("CARGO_BIN_EXE_deep-digger-min"))
        .args([
            "--input", &fixture(), "--output", "unused-output", "--timeout", "601",
            "--allow-exec", "--", "nonexistent-oracle", "{input}",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("--timeout must be between"));
}
