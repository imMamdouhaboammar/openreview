# OpenReview

A high-performance, vendor-neutral Rust microservice and autonomous AI Code Review & Fix Agent. OpenReview connects to GitHub as a GitHub App or CLI tool, providing evidence-based pull request code reviews and autonomous, verified code remediation within isolated sandboxes.

```mermaid
sequenceDiagram
    autonumber
    participant Dev as Collaborator / Contributor
    participant GH as GitHub API & Webhooks
    participant Svc as OpenReview Daemon
    participant SB as Isolated Sandbox
    participant LLM as Multi-Provider LLM

    Dev->>GH: Mentions @openreview [review | fix <instructions>]
    GH->>Svc: POST /api/webhooks/github (HMAC-SHA256 verified)
    Svc->>Svc: Check X-GitHub-Delivery deduplication & parse command
    Svc->>GH: Verify actor permissions (write/admin required for Fix)
    Svc->>GH: Fetch dynamic Installation Access Token (scoped per install)
    Svc->>GH: Fetch authoritative PR diff & changed files
    Svc->>SB: Provision sandbox & clone branch (token stripped from git config)
    loop ReAct Loop (Mode-Enforced Tools)
        Svc->>LLM: Prompt + authoritative diff + permitted tools
        LLM-->>Svc: Tool invocation (readFile, bash, writeFile [Fix only], reply)
        Svc->>SB: Execute inside sanitized sandbox
        SB-->>Svc: Unicode-safe truncated output
    end
    opt Mode == Fix & Changes Present
        Svc->>SB: Execute automated verification gate (tests/linter)
        alt Tests Passed
            Svc->>GH: Commit & push verified changes to branch
            Svc->>GH: Post verification report & commit SHA
        else Tests Failed
            Svc->>GH: Block push & report failure diagnostic to PR
        end
    end
    Svc->>SB: Teardown & destroy ephemeral sandbox
```

---

## Core Capabilities

### Capability A — AI Code Review (Read-Only)
* **On-Demand Precision Review:** Triggered via `@openreview review` or unqualified `@openreview` mentions.
* **Evidence-Based Findings:** Analyzes authoritative PR diffs for runtime bugs, security vulnerabilities, performance regressions, race conditions, missing validation, and test gaps.
* **Native Suggestions:** Provides line-referenced findings formatted with GitHub ````suggestion```` markdown blocks.
* **Strict Read-Only Guarantee:** Review mode has file-writing tools disabled and is strictly prevented from creating commits or modifying remote branches.

### Capability B — Autonomous Code Fixing (Verified Remediation)
* **Explicit Autonomous Fixing:** Triggered via `@openreview fix` or `@openreview fix <instructions>`.
* **Collaborator Authorization Required:** Evaluates the requesting user's GitHub repository permissions. Only users with `write`, `maintain`, or `admin` access can initiate Fix workflows. Unauthorized requests are rejected.
* **Pre-Publish Verification Gate:** Runs repository test suites (Cargo, Bun, NPM, Go, Pytest) inside the sandbox before committing. If tests fail, automated publishing is blocked and a diagnostic report is posted to the PR.
* **Attributable Commits:** Verified fixes are committed and pushed to the PR branch with an audit trail linking the commit to the requesting actor.

---

## Command & Interaction Model

OpenReview recognizes commands in PR issue comments and review comment threads:

| Command | Workflow Mode | Permission Required | Behavior |
| :--- | :--- | :--- | :--- |
| `@openreview review` | `Review` | Read | Conducts read-only code review; publishes findings. |
| `@openreview suggest` | `Review` | Read | Suggests improvements and optimizations without editing files. |
| `@openreview fix` | `Fix` | Write / Admin | Investigates issues, modifies files, runs verification tests, and pushes verified fixes. |
| `@openreview fix <task>` | `Fix` | Write / Admin | Applies specific remediation (e.g. `@openreview fix remove unwrap()`). |
| `@openreview <text>` | `Review` | Read | Unqualified mention defaults safely to read-only Review mode. |

### Reaction-Driven Approvals
* Reacting with 👍 or ❤️ to an OpenReview proposal by an authorized write collaborator triggers the autonomous Fix workflow to apply and verify the proposed changes.
* Reacting with 👎 or 😕 acknowledges and cancels the proposal without making repository modifications.

---

## Security Architecture & Sandbox Isolation

1. **Application-Level Mode Enforcement:** Tool availability is controlled by the application, not LLM prompts. `writeFile` is completely excluded from Review mode.
2. **Secret Sanitization:** The execution environment strips all host credentials (`GITHUB_APP_PRIVATE_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, etc.) before executing commands.
3. **Git Credential Isolation:** The Git remote origin URL is sanitized immediately after cloning (`https://github.com/owner/repo.git`). Access tokens are never stored in `.git/config` where sandbox scripts could read them.
4. **Path Traversal Protection:** All file operations enforce canonical path resolution, strictly blocking directory traversal (`..`) and symlink escapes outside the workspace.
5. **Process Tree Termination:** Commands execute in isolated process groups with strict timeouts, killing all descendant processes on timeout.
6. **Docker OCI Sandbox:** When `SANDBOX_DRIVER=docker` is configured, execution runs inside ephemeral containers with CPU limits (`2.0`), memory limits (`2GB`), PID limits (`100`), dropped Linux capabilities (`--cap-drop=ALL`), and guaranteed teardown on drop.

---

## Multi-Installation Architecture

OpenReview natively supports multi-tenant GitHub App deployments:
* **Dynamic Installation Scoping:** Installation IDs are resolved directly from verified incoming webhooks (`payload.installation.id`).
* **Isolated Token Manager:** Generates and caches short-lived installation access tokens per installation ID.
* **Cross-Installation Isolation:** Repositories are validated against the installation token; cross-tenant access is strictly prohibited.
* **CLI Fallback:** Standalone CLI commands use `GITHUB_APP_INSTALLATION_ID` or `GITHUB_TOKEN` without weakening webhook security.

---

## Endpoints

| Method | Path | Description | Authentication |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/webhooks/github` | Ingests GitHub App webhook events | Constant-time HMAC-SHA256 (`X-Hub-Signature-256`) |
| `GET` | `/api/manifest` | GitHub App Manifest configuration for 1-click registration | None |
| `GET` | `/api/jobs` | Lists recent webhook jobs and states | None |
| `GET` | `/api/jobs/{id}` | Queries status and error diagnostics for a specific job | None |
| `GET` | `/health` | Liveness & readiness probe | None |
| `GET` | `/metrics` | Prometheus exposition format metrics | None |
| `GET` | `/` | Service overview landing interface | None |

---

## Configuration & Environment Variables

Copy `.env.example` to `.env`:

```bash
# Server Network Settings
HOST=0.0.0.0
PORT=3000
BOT_USERNAME=openreview

# GitHub App Authentication
GITHUB_APP_ID=123456
GITHUB_APP_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----"
GITHUB_APP_WEBHOOK_SECRET=your_hmac_webhook_secret

# LLM Engine ("anthropic" | "openai" | "gemini" | "ollama")
LLM_PROVIDER=anthropic
LLM_MODEL=claude-3-7-sonnet-20250219
ANTHROPIC_API_KEY=sk-ant-...

# Sandbox Driver ("local" | "docker")
SANDBOX_DRIVER=local
```

---

## Deployment Guide

### Option 1: Docker Compose (Recommended for Production)

1. Clone the repository and configure `.env`:
   ```bash
   cp .env.example .env
   # Edit .env with your credentials
   ```

2. Start the service:
   ```bash
   docker compose up -d
   ```

3. Verify service health:
   ```bash
   curl -f http://localhost:3000/health
   ```

### Option 2: Standalone Binary

1. Build release binary:
   ```bash
   cargo build --release
   ```

2. Verify local configuration:
   ```bash
   ./target/release/openreview verify-config
   ```

3. Start server daemon:
   ```bash
   ./target/release/openreview serve --host 0.0.0.0 --port 3000
   ```

### Option 3: Standalone CLI Review (CI / Developer Workstation)

Run an on-demand review without starting a webhook server:
```bash
openreview review --repo owner/my-repo --pr 42
```

Run in dry-run mode (guarantees zero comments or pushes to remote):
```bash
openreview review --repo owner/my-repo --pr 42 --dry-run
```

---

## GitHub App Registration (1-Click Manifest)

1. Navigate to `https://github.com/settings/apps/new`.
2. Configure your webhook URL: `https://YOUR_DOMAIN/api/webhooks/github`.
3. Set permissions:
   - **Repository permissions:**
     - `Pull requests`: Read & write
     - `Issues`: Read & write
     - `Contents`: Read & write (required for Fix mode)
     - `Metadata`: Read-only
   - **Subscribe to events:**
     - `Issue comment`
     - `Pull request`
     - `Pull request review comment`
4. Generate a private key (`.pem`) and note your App ID and Webhook Secret.
5. Alternatively, inspect `GET /api/manifest` for the pre-built manifest JSON.

---

## Testing & Verification

Run the comprehensive automated test suite (51 tests):
```bash
cargo test
```

Run static analysis and linter:
```bash
cargo clippy --all-targets -- -D warnings
```

Verify formatting:
```bash
cargo fmt --check
```
