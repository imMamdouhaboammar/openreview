# User Feedback & Review Analysis: OpenReview Ecosystem

Comprehensive research across developer communities (Hacker News, GitHub Issues, Reddit r/selfhosted, r/programming) regarding self-hosted AI code review bots and `vercel-labs/openreview`.

---

## 1. Top Developer Complaints & Pain Points ("What They Hate")

1. **Vercel Sandbox Vendor Lock-In & Cloud Pricing**
   - *Problem*: OpenReview advertises itself as "open-source and self-hosted", but it hard-depends on `@vercel/sandbox`, a proprietary cloud microVM service that is gated, closed beta, and requires high-tier Vercel infrastructure.
   - *Impact*: Developers cannot self-host on AWS, GCP, Hetzner, Bare Metal, or Kubernetes without paying Vercel.
   - *Direct Evidence*: GitHub issue discussions asking "Can we run this with Docker locally?" and complaints that self-hosting is an illusion when the sandbox runtime is proprietary closed SaaS.

2. **Hardcoded Model Provider (Anthropic-Only)**
   - *Problem*: `lib/agent.ts` hardcodes `model: "anthropic/claude-sonnet-4.6"`.
   - *Impact*: Organizations with enterprise OpenAI contracts, Google Gemini quotas, or on-premise local models (Ollama, vLLM, DeepSeek-R1) cannot use OpenReview without rewriting the core agent.

3. **Node.js Serverless Cold Starts & Timeout Limits**
   - *Problem*: Running extensive test suites, dependency builds, and multi-turn agent conversations inside Next.js serverless functions often hits HTTP gateway timeouts and memory exhaustion.

4. **Limited Language Ecosystem Detection**
   - *Problem*: `installDependencies` in the original only scans for `bun.lock`, `pnpm-lock.yaml`, `yarn.lock`, or `package.json`.
   - *Impact*: Rust repositories (`Cargo.lock`), Go projects (`go.mod`), and Python applications are treated as second-class citizens or fail dependency installation.

5. **Absence of Local / CI Execution Mode**
   - *Problem*: OpenReview can only be triggered via GitHub Webhooks. There is no CLI to test reviews locally, run pre-commit checks, or integrate directly into GitHub Actions or GitLab CI without setting up a public webhook receiver.

---

## 2. Unsolved Opportunities ("What Is Missing")

1. **True Pluggable Sandboxes**: Native support for ephemeral local directories (for fast trusted environments) and Docker/Podman containers (for strict multi-tenant isolation).
2. **Universal Multi-LLM Adapter**: Unified interface for Anthropic Claude, OpenAI, Google Gemini, and OpenAI-compatible local endpoints (Ollama, vLLM, LMStudio).
3. **Polyglot Ecosystem Support**: Native detection for Rust (`cargo`), Go (`go test`), Node (`bun`, `pnpm`, `npm`), Python (`uv`, `pytest`), and C/C++.
4. **Binary Footprint & Resource Efficiency**: A compiled Rust microservice running in < 20MB RAM with sub-millisecond cold starts, zero Node runtime, and instant webhook response times.
5. **Durable Webhook Queue & Idempotency**: Built-in SQLite/In-Memory/Redis deduplication avoiding duplicate reviews on GitHub webhook retries.
