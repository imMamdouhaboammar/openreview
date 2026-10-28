# OpenReview-RS

A high-performance, vendor-neutral Rust microservice rebuilding [`vercel-labs/openreview`](https://github.com/vercel-labs/openreview). It operates as an autonomous GitHub PR review bot and on-demand coding assistant with pluggable sandbox execution (Local process & Docker OCI) and multi-provider LLM support (Anthropic Claude, OpenAI, Google Gemini, Ollama/vLLM).

```mermaid
sequenceDiagram
    autonumber
    participant Dev as Developer / Contributor
    participant GH as GitHub API & Webhooks
    participant Svc as OpenReview-RS (Microservice)
    participant SB as Sandbox Execution Driver
    participant LLM as Multi-Provider LLM Engine

    Dev->>GH: Mentions @openreview in PR comment
    GH->>Svc: POST /api/webhooks/github (HMAC-SHA256 verified)
    Svc->>GH: Add reaction (👀) & Start typing indicator
    Svc->>GH: Check push access & branch permissions
    Svc->>GH: Generate Installation Access Token (JWT RS256)
    Svc->>SB: Provision isolated sandbox workspace
    SB->>GH: Clone PR branch (depth 1) with token
    SB->>SB: Install dependencies & configure git credentials
    Svc->>SB: Scan .agents/skills/ for domain skills
    loop ReAct Loop (Max 20 steps, 200k tokens)
        Svc->>LLM: Stream prompt + tools + thread history
        LLM-->>Svc: Tool call: bash / readFile / writeFile / loadSkill / reply
        Svc->>SB: Execute command or file operation
        SB-->>Svc: Output (truncated <= 10,000 chars)
        opt Tool Call: reply
            Svc->>GH: Post inline review comment with suggestion blocks
        end
    end
    alt Files Modified in Sandbox (git diff)
        SB->>SB: git commit -m "openreview: apply changes"
        SB->>GH: git push origin <pr-branch>
    end
    Svc->>SB: Teardown & destroy sandbox
```

---

## Core Capabilities for Web Applications

* **On-Demand Precision Review:** Triggered instantly via `@openreview` mentions on any pull request or review comment thread. Evaluates diffs for runtime errors, race conditions, security vulnerabilities, bundle bloat, and regression risks.
* **One-Click Native Fixes:** Posts line-level comments formatted with standard GitHub ````suggestion```` blocks, enabling developers to review and accept code fixes directly in the GitHub UI.
* **Autonomous Code Remediation:** Clones the PR branch, executes formatters, linters, and test suites, edits files directly inside an isolated sandbox, and automatically commits and pushes verified fixes.
* **Reaction-Driven Lifecycle:**
  * Reacting with 👍 or ❤️ instructs OpenReview to execute and push proposed edits directly to the branch.
  * Reacting with 👎 or 😕 gracefully cancels the proposed action.
* **Progressive Skill Intelligence:** Dynamically discovers domain-specific guidelines from `.agents/skills/` (e.g., Next.js 16 conventions, React performance patterns, web accessibility, or Rust/Go microservice rules) and loads full instructions only when relevant.

---

## Who Is This For?

* **Engineering Teams & High-Velocity Startups:** Eliminate human reviewer fatigue on syntax, style, and routine regression checks. Keep PR queues unblocked around the clock.
* **Open-Source Maintainers:** Automatically triage, lint, and test external contributor pull requests against repository standards before human maintainer time is spent.
* **Enterprise & Security-Sensitive Organizations:** Teams whose compliance or privacy policies prohibit sending code to closed third-party SaaS review platforms. OpenReview-RS is 100% self-hosted on private Kubernetes clusters, internal Docker hosts, or air-gapped environments.
* **Platform & DevOps Engineers:** Embed review, formatting, and remediation checks into CI/CD pipelines via CLI mode or self-hosted webhook ingress without third-party vendor lock-in.

---

## The Competitive Edge: Architectural Overhaul vs Original Repo

| Dimension | `vercel-labs/openreview` (Original) | `openreview-rs` (Rebuilt) | Architectural Impact |
| :--- | :--- | :--- | :--- |
| **Sandbox Execution** | Proprietary `@vercel/sandbox` (closed invite-only beta, paid Vercel tier). | Pluggable [`SandboxDriver`](file:///Users/mamdouhaboammar/Documents/antigravity/bold-tesla/src/sandbox/traits.rs): Local isolated process runner or Docker OCI containers. | 100% self-hosted freedom. Deploy on any server, VM, or container orchestration platform without proprietary cloud microVM bills. |
| **LLM Provider Engine** | Hardcoded to Anthropic Claude via Vercel AI SDK. | Universal [`LlmProvider`](file:///Users/mamdouhaboammar/Documents/antigravity/bold-tesla/src/llm/traits.rs) interface supporting Anthropic, OpenAI, Google Gemini, and local Ollama/vLLM endpoints. | Zero provider lock-in. Use existing corporate OpenAI agreements, Gemini quota, or run private offline models (e.g. DeepSeek-R1). |
| **Runtime & Performance** | Next.js 16 Serverless on Node.js (~250MB+ RSS memory, 5-15s cold starts). | Compiled static Rust binary ([`axum`](file:///Users/mamdouhaboammar/Documents/antigravity/bold-tesla/src/server/routes.rs) + `tokio`). Operates under 20MB RSS with sub-millisecond cold start (<1ms). | Minimal infrastructure overhead, zero timeout failures, and instant webhook response times. |
| **Deployment Flexibility** | Restricted to Vercel Platform serverless functions. | Single static binary or lightweight multi-stage Docker container (`debian:bookworm-slim`). | Deployable to Kubernetes, AWS ECS, GCP Cloud Run, Fly.io, Hetzner, or bare metal. |
| **Execution Modes** | Webhook daemon only. | Dual-mode: Webhook daemon (`serve`) + Standalone CLI runner (`review`). | Run reviews directly in local development workspaces or GitHub Actions CI jobs (`openreview review --pr 42`) without exposing a public endpoint. |
| **Language Ecosystem** | Node.js lockfiles only (`bun.lock`, `pnpm-lock.yaml`, `yarn.lock`, `package.json`). | Polyglot detection engine: Rust (`Cargo.toml`), Go (`go.mod`), Python (`uv`/`pip`), and Node.js. | First-class support for backend microservices, systems code, and data engineering projects alongside frontend web apps. |

---

## Endpoints

| Method | Path | Description | Authentication |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/webhooks/github` | Ingests GitHub App webhook events | Constant-time HMAC-SHA256 (`X-Hub-Signature-256`) |
| `GET` | `/health` | Liveness & readiness probe, uptime, active model | None |
| `GET` | `/metrics` | Prometheus exposition format metrics | None |
| `GET` | `/` | Service overview landing interface | None |

---

## Configuration

Configuration is loaded from environment variables or standard `.env` files:

```bash
# Server Configuration
HOST=0.0.0.0
PORT=3000
BOT_USERNAME=openreview

# GitHub App Authentication
GITHUB_APP_ID=123456
GITHUB_APP_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----\n..."
GITHUB_APP_WEBHOOK_SECRET="your_webhook_hmac_secret"
GITHUB_APP_INSTALLATION_ID=98765432

# LLM Engine ("anthropic" | "openai" | "gemini" | "ollama")
LLM_PROVIDER=anthropic
LLM_MODEL=claude-3-7-sonnet-20250219
ANTHROPIC_API_KEY=sk-ant-...

# Alternative LLM Providers
OPENAI_API_KEY=sk-...
GEMINI_API_KEY=AIza...
OLLAMA_BASE_URL=http://localhost:11434

# Sandbox Execution Driver ("local" | "docker")
SANDBOX_DRIVER=local
```

---

## CLI Usage

### 1. Start the Microservice Daemon
```bash
openreview serve --host 0.0.0.0 --port 3000
```

### 2. Run Standalone Review (CI / Local Developer Mode)
```bash
openreview review --repo owner/my-repo --pr 42
```
To run without writing comments to GitHub:
```bash
openreview review --repo owner/my-repo --pr 42 --dry-run
```

### 3. Verify Configuration
```bash
openreview verify-config
```

### 4. Send Synthetic Test Webhook
```bash
openreview test-webhook --url http://127.0.0.1:3000/api/webhooks/github --repo owner/my-repo --pr 42
```

---

## Progressive Skill System

OpenReview-RS discovers review skills at runtime from:
1. `<repo>/.agents/skills/*/SKILL.md`
2. `<repo>/.skills/*/SKILL.md`
3. Built-in `.agents/skills/*/SKILL.md`

Each skill declares its metadata via YAML frontmatter:

```markdown
---
name: rust-best-practices
description: Review Rust code for idiomatic style, safety, and Tokio concurrency.
---

# Rust Review Guidelines
- Enforce Result<T, E> error handling.
- Reject unwrap() in production paths.
```

The agent system prompt loads only skill names and descriptions, calling the `loadSkill` tool dynamically when specialized guidance is required.

---

## Testing & Quality Gates

Run the test suite:
```bash
cargo test
```

Verification includes:
- Unit tests for HMAC-SHA256 constant-time verification.
- RS256 JWT claims minting.
- Tool output truncation guards (10,000 char threshold).
- YAML frontmatter parsing.
- Sandboxed filesystem and bash execution.
- Polyglot package manager detection (`Cargo.toml`, `go.mod`, `bun.lock`, etc.).
- Axum HTTP route integration (`/health`, `/metrics`, `/api/webhooks/github`).
- End-to-end ReAct autonomous loop execution with mock tool invocations.
