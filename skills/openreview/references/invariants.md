# OpenReview: Core System Invariants

Every deployment and configuration of OpenReview MUST adhere to these six invariants.

---

## The 6 Invariants

| # | Invariant | Enforcement Mechanism | Rationale |
|:--|:----------|:----------------------|:----------|
| **1** | **Constant-Time HMAC-SHA256 Webhook Verification** | `subtle::ConstantTimeEq` or `crypto.timingSafeEqual` verifying `X-Hub-Signature-256` | Prevents remote webhook forgery and timing attacks. Unauthenticated requests must be rejected with 401 before any JSON deserialization or processing occurs. |
| **2** | **Absolute Credential Isolation** | RSA Private Keys and LLM API keys must only be read from environment variables or secure secret managers | Prevents secret leakage into git logs, diagnostic endpoints, or public PR review comments. Never write credentials to repository files. |
| **3** | **Enforced Sandbox Execution Boundaries** | Untrusted PR branch checkouts and build/test commands MUST execute inside a bounded sandbox (`DockerDriver` or isolated `LocalDriver` with non-root UID and restricted paths) | Pull requests can contain malicious code or poisoned build hooks (e.g., npm `postinstall` or cargo `build.rs`). Unsandboxed execution gives untrusted code access to the host machine. |
| **4** | **ReAct Loop Fencing** | `max_steps <= 20`, `max_tokens <= 200,000`, `stdout_truncation <= 10,000 characters` | Prevents runaway recursive model loops and cost explosion while protecting against denial-of-wallet attacks. |
| **5** | **Explicit GitHub App Permission Grants** | App must declare `contents: write`, `pull_requests: write`, `issues: write`, `metadata: read` | Without these specific grants, API calls to clone branches, post review comments, and create commit suggestions will fail with silent HTTP 403 errors. |
| **6** | **Standard GitHub Suggestion Block Formatting** | Code remediations must be emitted using triple-backtick `suggestion` syntax with exact line offset anchors | Allows developers to review and commit AI-generated fixes with a single click in GitHub's native web interface. |
