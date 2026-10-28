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

## Architectural Comparison: Vercel OpenReview vs OpenReview-RS

| Dimension | `vercel-labs/openreview` (Original) | `openreview-rs` (Rebuilt) |
| :--- | :--- | :--- |
| **Runtime & Language** | Next.js 16 + TypeScript + Node.js (Vercel Serverless) | Compiled Rust binary (Axum + Tokio) |
| **Sandbox Backend** | Proprietary `@vercel/sandbox` (closed invite-only beta) | Pluggable `SandboxDriver`: Local isolated directory or Docker OCI |
| **Workflow Engine** | Proprietary `@workflow/ai` (Vercel Workflow cloud service) | Native Tokio async supervisor + state metrics |
| **Memory Footprint** | ~250MB+ RSS | < 20MB RSS |
| **Cold Start** | 5 – 15 seconds on serverless | Sub-millisecond (< 1ms) |
| **LLM Support** | Hardcoded to Anthropic Claude via AI SDK | Universal adapter: Anthropic, OpenAI, Gemini, Ollama, DeepSeek |
| **Deployment Options** | Vercel Platform only | Docker, Kubernetes, Linux VM, Bare Metal, or Local CLI |
| **CLI Mode** | None (Webhook only) | Built-in standalone CLI runner (`openreview review --pr <num>`) |
| **Ecosystem Detection** | Node.js lockfiles only (`bun`, `pnpm`, `yarn`, `npm`) | Polyglot: Rust (`cargo`), Go (`go.mod`), Python (`uv`/`pip`), Node |

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
