# Use Case: Verified Code Remediation (Fix Flow)

Workflow for autonomous code fixing with pre-publish test verification.

---

## 1. Intent Detection & Authorization
- **Trigger:** `@openreview fix <instructions>` comment on GitHub PR.
- **Authorization Check:**
  - Verify requesting user has `write`, `maintain`, or `admin` permissions on the target repository.
  - If permissions are insufficient, reject immediately and post an explanation to the thread.

---

## 2. Sandbox Provisioning & Execution
- **Sandbox Isolation:**
  - Provision an ephemeral Docker container or bounded temporary workspace.
  - Mount the PR branch in writable mode with network disabled and dropped root privileges.
  - Do NOT pass GitHub App private keys or LLM API keys into the container.

---

## 3. Applying Minimal Delta (Poteto Laziness Protocol)
- Apply the targeted fix addressing the authorized issue.
- Restrict edits strictly to relevant lines; avoid formatting changes or unsolicited refactoring.

---

## 4. Pre-Publish Verification Gate
- Run the project's native test harness:
  ```bash
  # Rust projects
  cargo test --all-targets --locked
  
  # Node/TypeScript projects
  bun test || npm test
  
  # Python projects
  pytest -v || python3 -m unittest
  
  # Go projects
  go test ./...
  ```
- **Evaluation Criteria:**
  - Exit code must be `0`.
  - Runner stdout must explicitly confirm that at least one test executed and passed.
  - If tests fail, halt immediately, discard uncommitted changes, and post failure logs to the PR.

---

## 5. Attributable Commit & Non-Force Push
- Commit verified changes with actor attribution in the commit message:
  ```text
  fix(core): sanitize user query parameter in search handler
  
  Authorized-by: @collaborator-username
  Verified-by: OpenReview Pre-Publish Gate (SHA: 8a9f3b...)
  ```
- Push directly to the PR head branch using non-force push.
