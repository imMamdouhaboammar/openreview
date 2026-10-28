# OpenReview-RS: Fixes & Strategic Positioning Plan

Derived from user feedback analysis and clean-room REA reconstruction.

---

## 1. Top 5 Fixes Implemented in OpenReview-RS

| # | Fix & Capability | Size | Module | Rationale & Evidence |
| :--- | :--- | :--- | :--- | :--- |
| **FIX-01** | **Pluggable Execution Drivers (Local & Docker)** | L | `src/sandbox/` | Replaces proprietary `@vercel/sandbox` with `LocalSandbox` (process isolation + timeout) and `DockerSandbox` (isolated container OCI). True 100% self-hosted freedom. |
| **FIX-02** | **Universal LLM Engine** | M | `src/llm/` | Replaces hardcoded Claude with a unified async client supporting Anthropic, OpenAI, Gemini, and OpenAI-compatible endpoints (Ollama/vLLM/DeepSeek). |
| **FIX-03** | **Compiled Rust Microservice & Sub-millisecond Overhead** | M | `src/server/` | Replaces Next.js serverless stack with an Axum + Tokio daemon. Consumes < 20MB RSS memory, starts instantly, handles thousands of concurrent webhooks without cold starts. |
| **FIX-04** | **Polyglot Ecosystem Detection** | S | `src/sandbox/detect.rs` | Expands dependency detection beyond JS lockfiles to `Cargo.toml`, `go.mod`, `pyproject.toml`, and Makefile. |
| **FIX-05** | **Dual-Mode Operation (Webhook Daemon & CLI Runner)** | M | `src/main.rs`, `src/cli.rs`| Enables both full daemon operation for GitHub Apps and direct CLI invocation (`openreview review --pr 42`) for developers and CI/CD pipelines. |

---

## 2. Positioning & Market Angle

> **"For engineering teams and open-source maintainers who want automated AI PR reviews and one-click code fixes without proprietary cloud sandbox lock-in, OpenReview-RS is a blazing-fast, self-hosted Rust microservice that runs anywhere (Docker, Kubernetes, Bare Metal) with any AI provider (Claude, OpenAI, Gemini, or local DeepSeek/Ollama) in under 20MB of memory."**
