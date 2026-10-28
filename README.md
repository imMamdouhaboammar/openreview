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
* **Review Suggestions:** Produces line-referenced findings and suggestion Markdown in PR comments. Native inline suggestions are not yet wired into the agent's reply tool.
* **Read-Only Enforcement:** Review mode exposes file reads and review comments only; arbitrary shell and file-writing tools are not available.

### Capability B — Autonomous Code Fixing (Verified Remediation)
* **Explicit Autonomous Fixing:** Triggered via `@openreview fix` or `@openreview fix <instructions>`.
* **Collaborator Authorization Required:** Evaluates the requesting user's GitHub repository permissions. Only users with `write`, `maintain`, or `admin` access can initiate Fix workflows. Unauthorized requests are rejected.
* **Pre-Publish Verification Gate:** Attempts a detected test suite (Cargo, Bun, NPM, Go, Pytest) inside the sandbox. Failures, missing executables, missing tests, and incomplete verification block publication. The sandbox image includes Cargo, npm, Go, and Pytest; Bun requires an operator-provided image.
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

### Fix Approval
An authorized collaborator approves a proposed change by leaving an explicit `@openreview fix <instructions>` comment. GitHub Apps do not receive a `reaction` webhook event; reacting with an emoji does **not** initiate or cancel a workflow. Reaction polling and proposal-specific approvals are not implemented.

---

## Security Architecture & Sandbox Isolation

1. **Mode Enforcement:** Review mode has no arbitrary shell or file-write tool and mounts the repository read-only inside Docker. Authorized Fix mode retains a writable repository and test execution.
2. **Trust Boundary:** The webhook daemon only starts with a dedicated, rootless Docker backend. Local mode is for trusted CLI development; it is not process isolation and cannot publish fixes.
3. **Credential Isolation:** The untrusted container cannot mount Git metadata, receive installation tokens, or access the service's process environment. The host controls credentials and pushing.
4. **Path Protection:** File tool paths reject `..`, absolute paths, and symlinks; Docker file operations run inside the container to avoid host path races.
5. **Container Restrictions:** Job containers have no network, read-only root filesystems, a read-only Review workspace (writable only for Fix), CPU/memory/PID limits, dropped capabilities, and teardown on drop. Missing offline dependencies block publishing.
6. **Job Metadata:** Jobs and deduplication IDs persist in `/data/openreview-jobs.json`. A restart marks interrupted jobs failed; automatic execution replay is intentionally not implemented because partial publishing is not safely idempotent.

---

## Multi-Installation Architecture

OpenReview natively supports multi-tenant GitHub App deployments:
* **Dynamic Installation Scoping:** Installation IDs are resolved directly from verified incoming webhooks (`payload.installation.id`).
* **Isolated Token Manager:** Generates and caches short-lived installation access tokens per installation ID.
* **Cross-Installation Isolation:** Repositories are validated against the installation token; cross-tenant access is strictly prohibited.
* **Operator Approval:** `ALLOWED_INSTALLATION_IDS` is a mandatory default-deny *admission policy* for webhook jobs. A valid GitHub webhook signature proves sender authenticity, not that the operator agrees to run code or pay for LLM requests. New installations are ignored until explicitly approved; an empty list accepts no jobs.
* **CLI Fallback:** Standalone CLI commands use `GITHUB_APP_INSTALLATION_ID` or `GITHUB_TOKEN` without weakening webhook security.

> **GitHub App visibility:** A **Private App** can only be installed on its owning account. To support the same self-hosted App across independent personal accounts and organizations, configure the App as **Public** in GitHub settings *and* approve the intended installation IDs in `ALLOWED_INSTALLATION_IDS`. Public visibility alone must never grant workload access. Each self-hosted operator registers and controls its **own** App credentials; do not distribute a shared central private key to customer-managed servers.

---

## Endpoints

| Method | Path | Description | Authentication |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/webhooks/github` | Ingests GitHub App webhook events | Constant-time HMAC-SHA256 (`X-Hub-Signature-256`) |
| `GET` | `/api/manifest` | Reference JSON for manual GitHub App registration (not a registration endpoint) | None |
| `GET` | `/api/jobs` | Lists job metadata across installations (operator only) | `Authorization: Bearer <JOB_API_TOKEN>` |
| `GET` | `/api/jobs/{id}` | Queries job status and diagnostics (operator only) | `Authorization: Bearer <JOB_API_TOKEN>` |
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

# Secure webhook execution requires an isolated rootless Docker engine
SANDBOX_DRIVER=docker
SANDBOX_DOCKER_SOCKET=/run/user/10001/docker.sock
SANDBOX_WORKSPACE_DIR=/srv/openreview/workspaces
JOB_API_TOKEN=replace_with_at_least_32_random_characters
MAX_CONCURRENT_JOBS=10
```

---

## Deployment Guide

### Option 1: Docker Compose (Recommended for Production)

1. Choose a public DNS name, for example `review.example.com`, pointing to your VPS. **Register your own GitHub App using a prefilled GitHub registration link** (no credentials or tokens are sent to OpenReview):

   ```bash
   # Build once using the VPS's normal Docker service-image builder.
   docker build -t openreview-service:local .
   docker run --rm openreview-service:local github-app-url \
     --domain https://review.example.com --name "OpenReview MyTeam"
   # For installation on multiple independent accounts / organizations, add --public.
   # To register the App under an organization, add --org MY_ORG.
   ```

   Open the printed `github.com` link, review the permissions/events, and create the App. GitHub still requires you to **set a strong webhook secret, note the App ID and generate a private key** in its App settings. GitHub's URL-parameter registration does not support setting a webhook secret; this is deliberately not automated. This is a prefilled setup link, **not** a completed GitHub registration or a manifest-code conversion flow ([GitHub documentation](https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-using-url-parameters)). A unique name may be required if your first choice is taken. Install the App into a test repository to obtain its numeric installation ID.

   Configure `.env` with the GitHub values, your LLM key, a random `JOB_API_TOKEN` of at least 32 characters, and only the installation IDs you explicitly authorize. Keep the private key on the host; do not paste it into GitHub issues or logs.

   ```bash
   cp .env.example .env
   # Edit .env with the registered App ID, escaped PEM private key, webhook secret,
   # allowed installation IDs, LLM API key, job API token and sandbox paths.
   ```

2. Prepare a separate rootless Docker daemon, accessible by the service's unprivileged user (UID 10001). Its socket must be the path in `SANDBOX_DOCKER_SOCKET`. Do not supply a host-rootful `/var/run/docker.sock`. Create `SANDBOX_WORKSPACE_DIR` as an absolute directory visible at the **same absolute path** to both the Compose service and rootless Docker host. The service and sandbox's mapped rootless user must both have read/write permissions there. Rootless Docker needs **cgroup v2 with systemd cgroup driver** to enforce the configured memory, CPU and PID limits; startup and job admission reject unsupported configurations ([Docker's resource-limit documentation](https://docs.docker.com/engine/security/rootless/tips/#limiting-resources)). Startup probes this bind-mount boundary and refuses service if either side cannot write to the shared workspace.

3. Build the execution image **on that rootless daemon**, then start the service:
   ```bash
   DOCKER_HOST=unix:///run/user/10001/docker.sock docker build -f Dockerfile.sandbox -t openreview-sandbox:latest .
   DOCKER_HOST=unix:///run/user/10001/docker.sock SANDBOX_WORKSPACE_DIR=/srv/openreview/workspaces \
     ./scripts/verify-rootless-sandbox.sh
   docker compose up -d
   ```

   The rootless security smoke test must pass on the **actual Docker host** before serving untrusted jobs. It exercises read-only Review mounts, writable Fix mounts, effective resource limits, disabled networking, absence of injected App tokens and cleanup. Passing this script does not replace real GitHub App permissions and Fix publication integration tests.

4. Verify service health:
   ```bash
   curl -f http://localhost:3000/health
   ```

   Compose publishes the service only on the host's loopback interface (`127.0.0.1:3000`). Place it behind an HTTPS reverse proxy with valid TLS, a stable public DNS name, request-size limits, and access to `/api/webhooks/github`. Never expose `/api/jobs`, `/api/jobs/{id}` or `/metrics` to the public internet; access the operator API through an SSH tunnel or a separately authenticated administrative route. Test the configured rootless daemon before accepting untrusted workloads.

   For example, if the VPS uses **Caddy** as its host reverse proxy, this Caddyfile routes public traffic while keeping job/metrics endpoints private:

   ```caddyfile
   review.example.com {
       @operator path /api/jobs /api/jobs/* /metrics
       respond @operator 403
       reverse_proxy 127.0.0.1:3000
   }
   ```

5. In GitHub App settings, verify that webhook URL is `https://YOUR_DOMAIN/api/webhooks/github`, the secret matches `.env`, and repository permissions include Contents (read/write), Issues (read/write), Pull requests (read/write) and Metadata (read). Confirm events include `issue_comment`, `pull_request`, `pull_request_review_comment`, `installation` and `installation_repositories`. The `/api/manifest` endpoint remains a reference template only.

6. Install the App on the desired repositories/accounts. For one owning account, **Private** visibility is sufficient; for *multiple unrelated organizations*, change visibility to **Public** in GitHub before installation. Locate each approved numeric installation ID from GitHub's installation settings or the signed installation webhook; add only those IDs, for example `ALLOWED_INSTALLATION_IDS=123456,789012`, to `.env`. Restart with `docker compose up -d` and verify an authorized `@openreview review` command in a test PR. Unapproved installations receive no jobs, even if their webhook is validly signed. Before any authorized `@openreview fix`, separately confirm the bot has installation-scoped write permission and the rootless sandbox security smoke test passes on the real daemon.

### Option 2: Standalone Binary

1. Build release binary:
   ```bash
   cargo build --release
   ```

2. Export the environment, then verify local configuration:
   ```bash
   set -a
   . ./.env
   set +a
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

## GitHub App Registration (Prefilled URL or Manual Setup)

Generate the personalized setup URL locally with:

```bash
openreview github-app-url --domain https://review.example.com --name "OpenReview MyTeam"
# Add --org ORG when the organization will own it; add --public for cross-account installs.
```

This command runs without any GitHub credentials or `.env`. It prints a URL to GitHub's own registration form, with permissions, webhook URL and events preselected. It cannot set the webhook secret or obtain a private key. If you prefer manual registration:

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
5. Choose "Only on this account" by default. Multi-organization operators can choose "Any account" explicitly, with an installation policy and LLM cost controls.
6. Inspect `GET /api/manifest` as a reference template. It contains a placeholder webhook URL and does not implement GitHub's manifest-registration POST/redirect flow.
7. Every self-hosted deployment must register and secure its **own** GitHub App and private key. Do not distribute one shared App private key to other operators. For production, the URL generator has unit coverage but GitHub's live registration form must still be verified by the deploying operator.

---

## Testing & Verification

Run the comprehensive automated test suite:
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
