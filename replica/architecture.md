# Architecture Specification: OpenReview-RS Microservice

High-performance, vendor-neutral Rust microservice rebuilding `vercel-labs/openreview`.

---

## 1. System Topology & Technology Stack

```
                   ┌─────────────────────────────────────────┐
                   │               GitHub API                │
                   │  - Webhooks (PR, Comments, Reactions)   │
                   │  - REST API (Pulls, Comments, Reviews)  │
                   └────────────────────▲────────────────────┘
                                        │
                         HMAC Webhook   │  REST / Git Push
                                        ▼
    ┌────────────────────────────────────────────────────────────────────────┐
    │                       OpenReview-RS (Daemon / CLI)                     │
    │                                                                        │
    │  ┌────────────────────────┐  ┌──────────────────────────────────────┐  │
    │  │ Server Layer (Axum)    │  │ GitHub Gateway                       │  │
    │  │ - POST /api/webhooks   │  │ - HMAC-SHA256 Signature Validator    │  │
    │  │ - GET /health, /metrics│  │ - RS256 JWT App Token Generator      │  │
    │  │ - GET / (Status Page)  │  │ - Octocrab / Reqwest REST Client     │  │
    │  └───────────┬────────────┘  └──────────────────┬───────────────────┘  │
    │              │                                  │                      │
    │              ▼                                  ▼                      │
    │  ┌──────────────────────────────────────────────────────────────────┐  │
    │  │ Workflow Coordinator (Tokio Async Engine)                        │  │
    │  │ - Push Access Checker                                            │  │
    │  │ - Thread History Collector                                       │  │
    │  │ - Step Supervisor & Deduplication Store                          │  │
    │  └──────────────────────────────┬───────────────────────────────────┘  │
    │                                 │                                      │
    │         ┌───────────────────────┴───────────────────────┐              │
    │         ▼                                               ▼              │
    │  ┌──────────────────────────────┐  ┌─────────────────────────────────┐ │
    │  │ ReAct Autonomous Agent       │  │ Pluggable Execution Sandbox     │ │
    │  │ - Prompt & Context Assembler │  │ - SandboxDriver Trait           │ │
    │  │ - Multi-Provider LLM Engine  │  │   ├── LocalSandbox (Isolated Dir│ │
    │  │   (Anthropic/OpenAI/Gemini)  │  │   └── DockerSandbox (OCI)       │ │
    │  │ - Progressive Skill Loader   │  │ - Dependency Detection Engine   │ │
    │  │ - Output Truncation Guard    │  │ - Authenticated Git Remote Sync │ │
    │  │ - Tool Registry              │  │ - Git Diff & Auto-Committer     │ │
    │  │   (bash, read, write, reply) │  │                                 │ │
    │  └──────────────────────────────┘  └─────────────────────────────────┘ │
    └────────────────────────────────────────────────────────────────────────┘
```

### Component Choices & Justification

| Layer | Technology | Rationale |
| :--- | :--- | :--- |
| **Language & Runtime** | Rust (Edition 2024 / 2021) + Tokio | Memory safety, zero GC overhead, sub-millisecond latency, static binary compilation. |
| **HTTP Framework** | `axum` (0.8) + `tower-http` | Ergonomic, modular, high-throughput asynchronous HTTP routing natively integrated with Tokio. |
| **GitHub App Crypto** | `ring` / `hmac` + `sha2` + `jsonwebtoken` | Constant-time HMAC verification for webhook integrity; RFC 7519 RS256 JWT minting for GitHub App installation credentials. |
| **LLM Provider Engine** | `reqwest` + `serde_json` + async trait | Zero-dependency multi-provider engine supporting Anthropic Messages API, OpenAI Chat Completions API, Google Gemini API, and local Ollama/vLLM endpoints. |
| **Sandbox Execution** | Pluggable `SandboxDriver` trait | `LocalSandbox` uses ephemeral temporary directories with isolated environment variables, timeout termination, and clean removal. `DockerSandbox` uses container isolation. |
| **Skill Engine** | Built-in YAML frontmatter parser | Scans `.agents/skills/*/SKILL.md` to progressively present names/descriptions to the LLM, loading full bodies only when `loadSkill` is called. |
| **State & Idempotency**| In-Memory Cache (DashMap / Mutex LRU) + Redis | Fast in-process state storage with optional distributed Redis backend. |
| **CLI & Operations** | `clap` (derive) | Unified binary supporting both long-running daemon (`serve`) and single-pass review (`review`). |

---

## 2. API Route Specification

| Method | Path | Function | Auth / Precondition | Request Body | Response Payload |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `POST` | `/api/webhooks/github` | Ingests GitHub webhook events | Valid `X-Hub-Signature-256` HMAC header | GitHub Webhook JSON | `{"status": "accepted", "event_id": "..."}` |
| `GET` | `/health` | Liveness & Readiness check | None | Empty | `{"status": "ok", "version": "0.1.0", "uptime_secs": 1234}` |
| `GET` | `/metrics` | Prometheus Metrics | Optional Bearer token | Empty | Prometheus exposition text format |
| `GET` | `/` | Service Overview UI | None | Empty | Clean status HTML / JSON |

---

## 3. Workflow Steps & State Machine

1. **Step 1: Ingress & Verification**
   - Webhook arrives at `/api/webhooks/github`.
   - HMAC-SHA256 signature calculated over raw body using `GITHUB_APP_WEBHOOK_SECRET` and compared via constant-time equality.
   - Event filter: checks if mention `@openreview` (or configured slug) exists in `issue_comment` or `pull_request_review_comment`, or reaction added to bot comment.
2. **Step 2: Immediate GitHub Acknowledgment**
   - Posts `👀` reaction to comment immediately.
   - Spawns background task via Tokio worker.
3. **Step 3: Access & Permission Gate**
   - Verifies repository is not archived.
   - Verifies installation has `contents:write` access.
   - Verifies branch protection rules do not ban bot pushes.
4. **Step 4: Sandbox Provisioning**
   - Mints installation token via RS256 JWT.
   - Clones target PR branch with `depth: 1` into isolated sandbox directory.
   - Detects package manager (`bun`, `pnpm`, `yarn`, `npm`, `cargo`, `go`) and installs dependencies.
   - Configures git remote with installation token and committer identity (`openreview[bot]`).
5. **Step 5: Skill Discovery & Agent Loop**
   - Scans `.agents/skills/` in repo.
   - Dispatches system instructions + available skills summary.
   - Executes ReAct tool loop (Max 20 steps, 200k tokens max):
     - `bash`: runs commands in sandbox with 10k char output guard.
     - `readFile`: reads files in sandbox.
     - `writeFile`: updates files in sandbox.
     - `loadSkill`: reads full skill body.
     - `reply`: posts markdown comments with GitHub suggestion blocks.
6. **Step 6: Diff Verification & Push**
   - Checks `git diff --name-only`.
   - If changed: commits with message `openreview: apply changes` and pushes to branch.
7. **Step 7: Cleanup**
   - Teardown sandbox workspace and report results.
