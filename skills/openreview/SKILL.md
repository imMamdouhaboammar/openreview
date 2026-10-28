---
name: openreview
description: >
  Technical specification, setup guide, and configuration engine for OpenReview — the autonomous
  GitHub PR review bot and on-demand coding assistant. Use when a user asks to audit, install,
  configure, deploy, or troubleshoot OpenReview on any repository, configure GitHub App webhooks,
  set up sandbox execution drivers (Docker/Local), or connect LLM providers (Anthropic, OpenAI,
  Gemini, Ollama). Triggered by phrases like "setup openreview", "configure openreview bot",
  "add pr review bot", "configure review sandbox", or "openreview settings". Do NOT use for
  general manual code reviews or editing pull requests directly.
compatibility: ">=1.0.0"
metadata:
  version: "1.0.0"
  author: "Mamdouh Aboammar"
  license: "MIT"
---

# OpenReview: Setup & Configuration Specification

Deploy and configure OpenReview — a high-performance, vendor-neutral microservice that operates as an autonomous GitHub pull request review bot and coding assistant with pluggable sandbox execution and multi-provider LLM intelligence.

The system invariant execution loop is:

$$\text{PR Mention / Event} \xrightarrow{\text{HMAC-SHA256 Ingress}} \text{Isolated Sandbox} \xrightarrow{\text{LLM ReAct Loop}} \text{GitHub Suggestion / Push}$$

---

## Operating Modes

| Mode | Trigger Condition | Primary Result |
|:-----|:------------------|:---------------|
| **AUDIT** | Inspect existing repository setup or permissions | Gap report comparing project against OpenReview requirements |
| **CONFIGURE** | Create or modify configuration settings | Validated `openreview.config.json` and `.env` template |
| **DEPLOY** | Deploy OpenReview service to production | `docker-compose.yml` or GitHub Actions workflow file |
| **VERIFY** | Test existing installation against invariants | Deterministic pass/fail report from `verify_openreview.py` |

---

## 5-Step Configuration Workflow

### Step 1: Detect Project Environment & Architecture
- **Freedom**: Low
- **Action**: Inspect repository root files (`Cargo.toml`, `go.mod`, `pyproject.toml`, `package.json`) to determine repository language toolchains, package managers, and build artifacts.
- **Reference**: Consult **`references/baseline-failure.md`** to verify known failure modes to prevent.

### Step 2: Configure GitHub App & Credentials
- **Freedom**: Low
- **Action**: Ensure the target GitHub App has the mandatory permissions:
  - `contents: write`
  - `pull_requests: write`
  - `issues: write`
  - `metadata: read`
  Generate high-entropy `GITHUB_APP_WEBHOOK_SECRET` via `openssl rand -hex 32`.
- **Reference**: Consult **`references/github-app-setup.md`** for step-by-step App registration and private key conversion.

### Step 3: Select Sandbox Driver
- **Freedom**: Medium
- **Action**:
  - For multi-tenant servers, public open-source repos, or untrusted PRs: Select `docker` driver with memory cap (`2048MB`) and CPU limit (`2.0`).
  - For ephemeral single-tenant GitHub Actions runner VMs or local CLI usage: Select `local` driver.
- **Reference**: Consult **`references/sandbox-drivers.md`** for cgroups, volumes, and cleanup guarantees.

### Step 4: Configure LLM Provider & Fences
- **Freedom**: Medium
- **Action**: Select the LLM provider (`anthropic`, `openai`, `gemini`, or `ollama`) and configure model credentials. Enforce hard ReAct execution loop boundaries:
  - `max_steps`: $\le 20$
  - `max_tokens`: $\le 200,000$
  - `stdout_truncation`: $\le 10,000$ characters
- **Reference**: Consult **`references/llm-providers.md`** for model matrix and context sizes.

### Step 5: Generate & Verify Configuration
- **Freedom**: Low
- **Action**: Emit `openreview.config.json` adhering to the official JSON schema. Run deterministic verification:
  ```bash
  python3 skills/openreview/scripts/verify_openreview.py .
  ```
- **Reference**: Consult **`references/config-schema.md`** and **`references/invariants.md`**.

---

## Invariant Safety Checklist

Before approving any OpenReview deployment, verify all six non-negotiable invariants:

1. **HMAC Signature Check**: Webhook handler uses constant-time HMAC-SHA256 (`X-Hub-Signature-256`).
2. **Zero Plaintext Secrets**: RSA keys and API tokens are loaded exclusively via environment variables; `.env` is listed in `.gitignore`.
3. **Sandbox Isolation**: Commands run inside ephemeral containers or bounded temp directories; never unconstrained host root.
4. **ReAct Loop Fences**: Maximum 20 steps, 200k tokens, 10k character stdout limits.
5. **Permission Parity**: GitHub App has explicit write access to contents, PRs, and issues.
6. **Suggestion Formatting**: Code changes emit valid GitHub ````suggestion```` markdown blocks.

---

## Stop Conditions & Fallback Behavior

- **Halt Immediately If**: The user or code attempts to disable HMAC webhook signature verification, bypass sandbox isolation, or print RSA private keys in logs.
- **Fallback**: If Docker daemon is unavailable in the environment, gracefully fallback to `local` sandbox mode with explicit warning of reduced isolation boundaries.
