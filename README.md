<p align="center">
  <img src="assets/openreview-mark.svg" alt="OpenReview logo: code brackets framing a verification check" width="92" height="92" />
</p>

<h1 align="center">OpenReview</h1>

<p align="center"><strong>Review with evidence. Fix with proof.</strong></p>

<p align="center">
  <a href="https://github.com/imMamdouhaboammar/openreview/actions/workflows/ci.yml"><img src="https://github.com/imMamdouhaboammar/openreview/actions/workflows/ci.yml/badge.svg" alt="OpenReview CI" /></a>
  <img src="https://img.shields.io/badge/core-Rust-CE422B?logo=rust" alt="Rust core" />
  <img src="https://img.shields.io/badge/npm-openreview--ai-CB3837?logo=npm" alt="npm openreview-ai" />
  <img src="https://img.shields.io/badge/inference-bring%20your%20own%20key-22C5B8" alt="Bring your own AI key" />
  <img src="https://img.shields.io/badge/deployment-GitHub%20Action%20%7C%20self--hosted-101828" alt="GitHub Action or self-hosted" />
</p>

<p align="center">
  <a href="#production-value-and-operational-impact">Value</a> ·
  <a href="#quickstart-with-npm-and-bun">Quickstart</a> ·
  <a href="#see-it-in-a-pull-request">Pull requests</a> ·
  <a href="#the-investigation-workbench-deep-digger">Deep Digger</a> ·
  <a href="#the-engineering-underneath">Architecture</a> ·
  <a href="#operational-limits">Operational limits</a>
</p>

---

## Autonomous pull request review and verified remediation

Traditional AI reviewers generate long lists of speculative comments, pattern-match stack traces without running code, and cannot prove whether a suggested patch works.

OpenReview is a high-performance, Rust-based GitHub pull request reviewer, daemon microservice, and investigation workbench. It operates under a strict verification contract:

1. **Evidence-based code review:** Findings are tied to exact diff line numbers, categorized into three severity tiers (Blocker, Suggestion, Nit), and delivered without conversational filler.
2. **Authorized remediation:** File writes and test commands run exclusively in rootless, ephemeral Docker containers without network egress. Fix mode activates only when an authorized collaborator explicitly requests it.
3. **Execution proof required:** An exit code of zero is insufficient. OpenReview parses runner-specific execution logs (Cargo, Bun, Pytest, Go, NPM) to confirm tests actually ran before any branch push.
4. **Algorithmic defect minimization:** Integrated with Deep Digger to shrink massive failing inputs down to atomic tokens or lines using Zeller's delta debugging algorithm.

---

## Production value and operational impact

| Operational challenge | Standard AI reviewer | OpenReview system |
|---|---|---|
| **PR code inspection** | Verbose conversational opinions, vague suggestions | 3-tier severity taxonomy with reproduction paths and diff links |
| **Automated bug fixes** | Hallucinated code pasted directly into PR comments | Ephemeral sandbox execution with mandatory test runner verification |
| **Intermittent CI failures** | Retrying builds until they pass by accident | Empirical flake analysis calculating exact failure rate across repeated runs |
| **Massive crash payloads** | Manual bisection of inputs across hours | Sub-second token-level delta reduction to minimal reproducing syntax |
| **Supply chain security** | Unrestricted shell access and mounted host tokens | Rootless container daemon, dropped capabilities, and isolated tokens |
| **Multi-agent integration** | Vendor-locked proprietary interfaces | Universal skill package auto-registered across Claude Code, Gemini CLI, and Codex |

---

## Quickstart with npm and bun

OpenReview distributes as a standalone package with automated multi-agent skill registration.

### 1. Global package installation

```bash
# Install globally via Bun (recommended)
bun add -g openreview-ai

# Or install globally via npm
npm install -g openreview-ai
```

The installation lifecycle hook automatically executes `npx skills add imMamdouhaboammar/openreview`, registering the orchestrator and investigation skills directly into your active agent environment (Claude Code, Gemini CLI, Antigravity, and Codex).

### 2. Verify local installation

```bash
openreview doctor
```

```text
[OpenReview] Running System Preflight Diagnostics...

  [PASS] JavaScript Runtime: node v26.3.0
  [PASS] Rust Compiler (cargo): cargo 1.99.1
  [PASS] Python 3 Standard Library: Python 3.9.6
  [PASS] Docker Daemon (for Fix Sandboxing): Docker version 29.7.2
  [PASS] Git CLI: git version 2.50.1
  [PASS] Native Binary: Compiled

[PASS] System diagnostics completed.
```

### 3. Agent skill registration

```bash
# Direct skills registry installation
npx skills add imMamdouhaboammar/openreview
```

---

## See it in a pull request

OpenReview recognizes mentions in pull request comments and review threads.

| Mention syntax | Allowed action | Prohibited action |
|---|---|---|
| `@openreview review` | Inspect the PR diff, read source files, report findings | Edit files or execute arbitrary shell commands |
| `@openreview suggest` | Recommend concrete code changes with rationale | Apply edits to the branch |
| `@openreview fix` | Investigate, apply minimal patch, run tests, publish if verified | Bypass collaborator authorization or skip tests |
| `@openreview fix handle edge case` | Target named repair in an isolated sandbox | Publish without affirmative test runner output |
| `@openreview <question>` | Respond in read-only Review mode | Escalate to Fix without explicit keyword |

Fix requests require GitHub `write`, `maintain`, or `admin` permissions on the target repository. OpenReview verifies caller authorization before initializing the sandbox and again before pushing code.

---

## Review taxonomy and feedback format

Findings follow a three-tier severity structure designed to minimize noise and highlight blocking defects:

- **Blocker:** Security vulnerabilities (SQLi, auth bypass, memory leaks), panics, unhandled exceptions, breaking public API contracts, and broken test suites. Must be resolved before merge.
- **Suggestion:** Boundary checks, performance bottlenecks, unindexed queries, and missing unit test coverage.
- **Nit:** Minor naming clarity, internal docstrings, and non-blocking stylistic consistency.

```markdown
🔴 **Blocker: Unvalidated array index access**
Line 84 in `src/parser.rs`: Missing bounds check before slice indexing.

**Why:** An empty payload triggers a runtime panic.

**Suggestion:** Check length before slicing or return a typed `ParserError::EmptyInput`.

**Verification:** Static analysis finding. Tests not run.
```

---

## The investigation workbench: Deep Digger

When defects resist quick visual inspection, OpenReview deploys [**Deep Digger**](skills/deep-digger/README.md). Deep Digger operates in two unified styles:

1. **Standalone CLI and package:** Installable via `npm install -g deep-digger` or `npx skills add imMamdouhaboammar/deep-digger`.
2. **Native OpenReview subsystem:** Invoked directly by `openreview-orchestrator` during the `INVESTIGATE` operating mode.

### Deep Digger core tools

- **Token and line delta debugging:** High-speed Rust minimizer (`deep-digger-min`) reducing reproducing files to their 1-minimal form.
- **Trace and log ingestion:** Automatically parses OpenTelemetry spans and HAR archives into structured error events.
- **Causal DAG linking:** Connects competing hypotheses into directed acyclic graphs with automated cycle prevention.
- **Statistical flake estimation:** Executes repeated oracle evaluations to calculate empirical failure probability $P(\text{FAIL})$.
- **Automated git bisection:** Coordinates non-interactive `git bisect run` workflows, recording culprit commit metadata into the evidence ledger.
- **Air-gapped diagnostic dossiers:** Exports self-contained HTML5 reports with embedded SVGs and cryptographic SHA-256 evidence digests.

```bash
# Line-level reduction for test files
deep-digger-min --input repro_large.txt --output repro_minimal.txt --allow-exec -- ./test_oracle.sh {input}

# Token-level reduction for JSON payloads and AST syntax
deep-digger-min --input bad_payload.json --output minimal.json --granularity tokens --allow-exec -- ./test_oracle.sh {input}
```

---

## Start with GitHub Actions

Run OpenReview directly on GitHub-hosted runners using your own AI provider credentials.

1. Add provider credentials under **Repository → Settings → Secrets and variables → Actions** (for example, `OPENROUTER_API_KEY`).
2. Add `.github/workflows/openreview.yml` using the template below:

```yaml
name: OpenReview

on:
  pull_request:
    types: [opened, synchronize]
  issue_comment:
    types: [created]

permissions:
  contents: write
  pull-requests: write
  issues: write

jobs:
  review:
    if: ${{ github.event_name == 'pull_request' || contains(github.event.comment.body, '@openreview') }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: imMamdouhaboammar/openreview@main
        with:
          provider: openrouter
          model: anthropic/claude-sonnet-4
          bot_username: openreview
          openrouter_api_key: ${{ secrets.OPENROUTER_API_KEY }}
```

---

## Supported inference providers

OpenReview connects to upstream models without vendor lock-in. Keys remain in the trusted host environment and are never mounted into sandbox containers.

| Provider (`LLM_PROVIDER`) | Required environment key | Model selection format |
|---|---|---|
| `anthropic` | `ANTHROPIC_API_KEY` | Anthropic model identifier |
| `openai` | `OPENAI_API_KEY` | OpenAI model identifier |
| `openrouter` | `OPENROUTER_API_KEY` | Provider-prefixed model identifier |
| `opencode` | `OPENCODE_API_KEY` | OpenCode Console model identifier |
| `gemini` | `GEMINI_API_KEY` | Google Gemini model identifier |
| `ollama` | Locally configured endpoint | Model available on your Ollama instance |

---

## Self-hosted service deployment

Deploy OpenReview as a persistent webhook microservice with dedicated rootless Docker isolation.

### 1. Build and configure

```bash
cp .env.example .env
docker build -t openreview-service:local .
docker run --rm openreview-service:local github-app-url \
  --domain https://review.example.com --name "OpenReview Service"
```

Configure `.env` with the generated GitHub App credentials:

```dotenv
GITHUB_APP_ID=123456
GITHUB_APP_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----"
GITHUB_APP_WEBHOOK_SECRET=your_generated_secret
BOT_USERNAME=openreview
ALLOWED_INSTALLATION_IDS=123456
LLM_PROVIDER=openrouter
LLM_MODEL=anthropic/claude-sonnet-4
OPENROUTER_API_KEY=your_key
JOB_API_TOKEN=minimum_32_random_characters
SANDBOX_DRIVER=docker
SANDBOX_DOCKER_SOCKET=/run/user/10001/docker.sock
SANDBOX_WORKSPACE_DIR=/srv/openreview/workspaces
```

### 2. Dual-daemon isolation model

OpenReview separates privileged operations from untrusted repository code using two Docker daemons:

- **Host daemon:** Runs the trusted service container with access to GitHub credentials.
- **Rootless daemon (UID 10001):** Executes repository tests and commands in an isolated sandbox with network disabled, capabilities dropped, and memory/PID limits enforced via cgroup v2.

```bash
# Start the service daemon
docker compose up -d
curl -f http://localhost:3000/health
```

---

## The engineering underneath

```mermaid
flowchart TD
    GH["GitHub PR event"] --> Entry{"Deployment"}
    Entry -->|Self-hosted App| Signed["Verify webhook signature & installation admission"]
    Entry -->|GitHub Action| Action["Validate Actions event context"]
    Signed --> Mode{"Requested mode"}
    Action --> Mode
    Mode -->|Review| RO["Read-only AST and diff tools"]
    Mode -->|Authorized Fix| RW["Rootless Docker sandbox"]
    RO --> Report["Line-referenced PR report"]
    RW --> Verify{"Runner execution output verified?"}
    Verify -->|Failed / unverified| Report
    Verify -->|Passed| Publish["Recheck branch head SHA; non-force push"]
    Publish --> Report
```

### System layers

| Layer | Implementation | Core responsibility |
|---|---|---|
| **GitHub gateway** | `src/github/`, `src/server/` | HMAC verification, installation token scoping, webhook ingestion |
| **Agent reasoning** | `src/agent/` | ReAct loop, tool dispatch, prompt synthesis |
| **Workflow engine** | `src/workflow/` | Command grammar, permission admission, verification gates, git publishing |
| **Sandbox boundary** | `src/sandbox/`, `Dockerfile.sandbox` | Ephemeral containers, network isolation, cgroup limits |
| **AI providers** | `src/llm/` | Unified provider client abstraction |
| **Investigation engine** | `skills/deep-digger/` | Token and line input minimization, causal DAGs, flake analysis |
| **Orchestration layer** | `skills/openreview-orchestrator/` | Natural language intent routing (Review, Fix, Investigate, Verify-Config) |

---

## Verification and test coverage

```bash
# Verify Rust formatting, clippy lints, and test suite
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all

# Run Deep Digger test suite and security audit
python3 -m unittest discover -s skills/deep-digger/evals -p 'test_cli.py' -v
python3 scripts/scan_skill_security.py skills/deep-digger

# Run Orchestrator test suite and security audit
python3 -m unittest discover -s skills/openreview-orchestrator/evals -p 'test_orchestrator.py' -v
python3 scripts/scan_skill_security.py skills/openreview-orchestrator
```

---

## Operational limits

- **Developer judgment required:** Automated reviews identify patterns and risks based on model inference. Human maintainers retain merge responsibility.
- **Affirmative test evidence required:** Fix mode cannot publish code unless the native test runner executes and reports passing tests. If tests are absent or cannot run offline, publication aborts.
- **Explicit authorization:** Fix mode never triggers from reactions or unauthorized contributors. Callers must hold GitHub write access.
- **Zero force pushes:** OpenReview checks the remote branch head before pushing. If the branch changed during analysis, the push aborts.

<p align="center"><strong>OpenReview: Review with evidence. Fix with proof.</strong></p>
