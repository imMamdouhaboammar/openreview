<p align="center">
  <img src="assets/openreview-mark.svg" alt="OpenReview logo: code brackets framing a verification check" width="92" height="92" />
</p>

<h1 align="center">OpenReview</h1>

<p align="center"><strong>Review with evidence. Fix with proof.</strong></p>

<p align="center">
  <a href="https://github.com/imMamdouhaboammar/openreview/actions/workflows/ci.yml"><img src="https://github.com/imMamdouhaboammar/openreview/actions/workflows/ci.yml/badge.svg" alt="OpenReview CI" /></a>
  <img src="https://img.shields.io/badge/core-Rust-CE422B?logo=rust" alt="Rust core" />
  <img src="https://img.shields.io/badge/inference-bring%20your%20own%20key-22C5B8" alt="Bring your own AI key" />
  <img src="https://img.shields.io/badge/deployment-GitHub%20Action%20%7C%20self--hosted-101828" alt="GitHub Action or self-hosted" />
</p>

<p align="center">
  <a href="#see-it-in-a-pull-request">See it in a PR</a> ·
  <a href="#start-with-github-actions">Start with GitHub Actions</a> ·
  <a href="#run-your-own-github-app">Self-host</a> ·
  <a href="#the-engineering-underneath">Architecture</a> ·
  <a href="#what-openreview-does-not-promise">Honest limits</a>
</p>

---

## AI can write a fix in seconds. Who proves it belongs in your branch?

An AI review can sound convincing and still be wrong. A proposed patch can look clean and still break a test. And a bot that can read your repository does not automatically deserve permission to modify it.

**OpenReview is an open-source, Rust-based GitHub pull request agent built around those three problems.**

It investigates changes, explains findings against the actual PR diff, and, **only when an authorized collaborator explicitly asks**, attempts a fix in an isolated environment. Before publishing code, it must pass a verification gate and confirm what actually reached the remote branch.

The idea is simple: **confidence is not evidence, and generated code is not a verified repair.**

### A note from Mamdouh, the creator

> My background is in brand strategy, copywriting, and digital systems. In those fields, I learned to question a polished claim: *What supports it? What happens when we test it? What are we leaving out?*
>
> I brought that same instinct to AI-assisted engineering.
>
> I didn't want another reviewer that simply produces a long list of opinions. I wanted to design an agent that knows the difference between **finding a possible issue**, **being allowed to act**, **changing code**, and **proving that the change was published**.
>
> That's why OpenReview has separate Review and Fix modes. That's why the agent doesn't hold the publishing credentials. That's why a successful command exit is not automatically treated as proof that tests ran. And that's why the project reports what it *couldn't* verify, too.
>
> I have been building this in iterations: starting with the PR workflow, testing it against real GitHub interactions, hardening the permission and sandbox boundaries, adding GitHub-hosted and self-hosted routes, and introducing deeper investigation tooling. The decisions are visible in the code and commit history. **The goal is not to make AI sound certain. It's to make its work inspectable.**
>
> **Mamdouh Aboammar** · [GitHub](https://github.com/imMamdouhaboammar)

### A few decisions you can trace in public

- [**PR #1**](https://github.com/imMamdouhaboammar/openreview/pull/1): Hardened remediation, publishing checks, isolation, and multi-installation controls. A Fix needed to be an auditable operation, not a model response with write access.
- [**PR #2**](https://github.com/imMamdouhaboammar/openreview/pull/2): Added a GitHub-hosted, bring-your-own-key route. Running your own server should be a choice, not the price of trying the agent.
- [**PR #3**](https://github.com/imMamdouhaboammar/openreview/pull/3): Defined the visual identity and evidence-first language of the PR comments. How a system admits uncertainty is part of its design.
- [**PR #5**](https://github.com/imMamdouhaboammar/openreview/pull/5): Added the separate Deep Digger investigation workbench. Hard problems need hypotheses, counterevidence, and reproducible cases.

## See it in a pull request

OpenReview recognizes mentions in PR comments and review-comment threads. With the default handle:

| You write | The agent can do | It cannot do |
| --- | --- | --- |
| `@openreview review` | Inspect the PR, read relevant files, report findings | Edit files or execute arbitrary shell commands |
| `@openreview suggest` | Recommend a specific change, with supporting reasoning | Apply the recommendation |
| `@openreview fix` | Investigate, edit, run verification, and attempt to publish a passing repair | Bypass contributor authorization or verification |
| `@openreview fix handle empty input in the parser` | Work toward the named repair and report the result | Promise that an unverified patch was published |
| `@openreview <question>` | Respond in the default, read-only Review mode | Silently escalate to Fix |

For Fix, the person making the request needs GitHub `write`, `maintain`, or `admin` permission. OpenReview rechecks authorization before publishing.

<details>
<summary><strong>What a credible result looks like</strong></summary>

**Review**

> **Finding:** `src/parser.rs:84` can panic if the expected header is absent.
>
> **Reason:** The value reaches `unwrap()` without validation.
>
> **Suggested repair:** Return a typed error and cover the empty-header case.
>
> **Verification:** Static review only. Tests not run.

**Fix**

> **Changed:** Guarded the missing-header path and added a regression test.
>
> **Verification:** Test command completed with evidence that tests executed.
>
> **Publication:** The remote PR head was checked against the pushed commit.

These are illustrations of the reporting style, **not results claimed from a particular PR**. Native GitHub inline suggestions are not currently wired into the agent's reply tool; suggestions appear as PR comment Markdown.

</details>

## The decisions behind the design

OpenReview grew around a set of engineering boundaries, not a longer system prompt.

| Question I wanted the system to answer | Design decision in OpenReview |
| --- | --- |
| **Can a reviewer change something it was only asked to inspect?** | No. Review exposes `readFile`, `loadSkill`, and `reply`. `bash` and `writeFile` are reserved for authorized Fix. |
| **Can model output become a GitHub write operation by accident?** | No. The trusted host controls GitHub credentials and publication; the untrusted workspace does not receive installation tokens. |
| **What if the repository contains hostile code?** | Agent execution uses a dedicated rootless Docker daemon, disabled sandbox networking, restricted mounts, dropped capabilities, and resource limits. Unsupported isolation blocks execution. |
| **What if a test command exits successfully without running tests?** | The verification gate looks for runner-specific evidence of executed tests, not only exit code zero. Missing or failed verification blocks the push. |
| **What if permissions or the PR branch change mid-run?** | The publisher rechecks the actor, branch protection, expected head SHA, and remote result. No force push. |
| **What if a public GitHub App gets installed by someone I haven't approved?** | A valid webhook signature is necessary but not sufficient. Self-hosted operators admit installation IDs explicitly. |
| **What if the AI is uncertain?** | Report what was inspected, what failed, and what was not verified. Don't invent a green check. |

The important distinction: **an AI tool proposes actions; the workflow decides which actions are allowed and whether a result can be published.**

## Start with GitHub Actions

<strong>No VPS or GitHub App registration required.</strong> Each repository runs OpenReview on its own GitHub-hosted runner with its own AI provider key.

1. Add a provider key under **Repository → Settings → Secrets and variables → Actions**. For example, create `OPENROUTER_API_KEY`.
2. Copy [`examples/github-actions.yml`](examples/github-actions.yml) into your repository as `.github/workflows/openreview.yml`.
3. In that workflow, replace the example `@vX.Y.Z` placeholder with a **real, reviewed ref**. The following source commit is an existing immutable starting point:

   ```yaml
   - uses: imMamdouhaboammar/openreview@855730c116038d41d8c1b2028c2a7cee83a2ca91
     with:
       provider: openrouter
       model: anthropic/claude-sonnet-4
       bot_username: openreview
       openrouter_api_key: ${{ secrets.OPENROUTER_API_KEY }}
   ```

   This replaces only the `uses` step in the complete [example workflow](examples/github-actions.yml); keep its event filters, job permissions, and concurrency settings. The pinned ref identifies source code, **not a claim that a numbered release has been published**. Choose an exact model your provider account can access.
4. Confirm that repository/org policy allows the action and that the workflow has `contents: write`, `issues: write`, and `pull-requests: write` permissions. Those permissions allow Fix; the agent separately checks the requesting user's authority.
5. Open a same-repository PR and watch the **Actions** tab, or comment `@openreview review` on a PR to request a review. For a targeted repair, an authorized collaborator can comment `@openreview fix <task>`.

> **Status: experimental on GitHub-hosted runners.** The Action builds the Rust agent and a rootless sandbox, then runs an isolation smoke test. If the runner cannot enforce the required cgroup v2, filesystem, or network boundaries, it stops. A passing Docker build is not proof that this environment is safe. Validate the full workflow on your runner before relying on Fix.

**Know the GitHub Actions boundaries:**

- A workflow installed in your repository is **not** a separately registered GitHub account named `@openreview`. The mention string triggers the workflow; replies come from GitHub's Actions identity.
- Forked `pull_request` events do not receive normal repository secrets. The example skips their automatic review. A trusted maintainer may request read-only Review via a PR comment, but Fixing a fork branch is denied.
- Changes pushed with the default `GITHUB_TOKEN` normally do **not** trigger fresh `push` or `pull_request` Actions runs. Require manual CI re-runs or a separately reviewed token strategy before merging if your policy needs fresh checks.
- Hosted jobs are transient. They do not provide an always-on webhook service, a persistent job queue, or a durable cross-run deduplication store.
- Provider inference and GitHub Actions usage are subject to your account's quotas and billing.

## Bring the model you trust

OpenReview is not tied to one AI vendor. Configure an explicit model and supply the corresponding key; the key stays in the trusted workflow or service environment, outside the untrusted sandbox.

| `provider` / `LLM_PROVIDER` | Required key | Model selection |
| --- | --- | --- |
| `anthropic` | `ANTHROPIC_API_KEY` | Anthropic model ID |
| `openai` | `OPENAI_API_KEY` | OpenAI model ID |
| `openrouter` | `OPENROUTER_API_KEY` | Provider-prefixed OpenRouter model ID |
| `opencode` | `OPENCODE_API_KEY` | OpenCode Console model accessible to your account |
| `opencode_go` | `OPENCODE_API_KEY` | Eligible OpenCode Go model |
| `opencode_custom` | `OPENCODE_API_KEY`, connection URL, and protocol | Model enabled on that custom connection |
| `gemini` | `GEMINI_API_KEY` | Gemini model ID |
| `ollama` | Locally configured endpoint | Model available on your Ollama server |

OpenCode here is an **inference API integration**, not an invocation of the OpenCode CLI coding agent. OpenCode Custom supports `chat`, `responses`, `anthropic`, and `gemini` protocols. Provider access, quotas, and model names can change. See [OpenCode and Custom Provider setup](docs/opencode.md) for the exact configuration.

## Run your own GitHub App

Self-hosting is for teams that want their own app identity, HTTPS webhook endpoint, installation approvals, and persistent job metadata. **Every operator registers and secures their own GitHub App.**

### 1. Prepare the service and App

On a machine with Docker, a public DNS name, and a TLS reverse proxy:

```bash
cp .env.example .env
docker build -t openreview-service:local .
docker run --rm openreview-service:local github-app-url \
  --domain https://review.example.com --name "OpenReview MyTeam"
```

The last command prints a prefilled **GitHub-owned registration link**. Open it, create the App, then set its webhook secret and generate its private key in GitHub settings. The helper cannot create those secrets or finish registration for you. Use `--org ORG` to register under an organization, or `--public` if you intend cross-account installations.

Set these values in `.env`, using [`.env.example`](.env.example) for the full reference:

```dotenv
GITHUB_APP_ID=123456
GITHUB_APP_PRIVATE_KEY="-----BEGIN RSA PRIVATE KEY-----\n...\n-----END RSA PRIVATE KEY-----"
GITHUB_APP_WEBHOOK_SECRET=replace_with_a_random_secret
BOT_USERNAME=your_real_app_mention_handle
ALLOWED_INSTALLATION_IDS=123456
LLM_PROVIDER=openrouter
LLM_MODEL=your/provider-model
OPENROUTER_API_KEY=your_key
JOB_API_TOKEN=replace_with_at_least_32_random_characters
SANDBOX_DRIVER=docker
SANDBOX_DOCKER_SOCKET=/run/user/10001/docker.sock
SANDBOX_WORKSPACE_DIR=/srv/openreview/workspaces
```

Keep `.env` and private keys out of source control. Set `ALLOWED_INSTALLATION_IDS` only after installing the App and identifying the installation ID; **empty means no webhook jobs are admitted**. The mention handle must match the actual App account, not an assumed `openreview` username.

### 2. Separate the trusted service from the untrusted sandbox

OpenReview deliberately uses **two Docker daemons**:

- A normal host daemon builds/runs the **trusted service**, which handles GitHub credentials and publication.
- A **dedicated rootless Docker daemon**, accessible to the service's unprivileged UID 10001, executes repository code without receiving those credentials.

The workspace path must exist at the **same absolute location** for the service and the rootless daemon, and both sides must have the required permissions. Use cgroup v2 with the systemd cgroup driver so CPU, memory, and PID limits are actually enforced. Never give the service the host's rootful `/var/run/docker.sock`.

```bash
# Trusted image on the normal host daemon
DOCKER_HOST=unix:///var/run/docker.sock \
  docker build -t openreview-service:local .

# Sandbox image and security test on the dedicated rootless daemon
DOCKER_HOST=unix:///run/user/10001/docker.sock \
  docker build -f Dockerfile.sandbox -t openreview-sandbox:latest .

# Run the smoke test as the unprivileged rootless-daemon owner, not host root.
DOCKER_HOST=unix:///run/user/10001/docker.sock \
  SANDBOX_WORKSPACE_DIR=/srv/openreview/workspaces \
  ./scripts/verify-rootless-sandbox.sh

# Run the trusted service; Compose connects it only to the rootless socket.
DOCKER_HOST=unix:///var/run/docker.sock docker compose up -d
curl -f http://localhost:3000/health
```

These commands assume the rootless daemon/socket and shared workspace are already provisioned. **Do not accept untrusted jobs until the smoke test passes on the actual Docker host.** It exercises read-only Review mounts, writable Fix mounts, disabled networking, resource limits, credential absence, and teardown.

### 3. Connect the webhook

- Serve the app behind HTTPS, forwarding `https://review.example.com/api/webhooks/github` to the local service on `127.0.0.1:3000`.
- In GitHub App settings, configure that webhook URL and the secret from `.env`. Grant repository **Contents, Issues, and Pull requests: read/write**; **Metadata: read**. Subscribe to `issue_comment`, `pull_request`, `pull_request_review_comment`, `installation`, and `installation_repositories`.
- Install the App on a test repository, add the numeric installation ID to `ALLOWED_INSTALLATION_IDS`, restart Compose, and try `@YOUR_APP_HANDLE review` on a test PR.
- Keep `/api/jobs`, `/api/jobs/{id}`, and `/metrics` off the public internet. The job API requires `Authorization: Bearer <JOB_API_TOKEN>`; expose operator-only endpoints through a separately authenticated channel.

A minimal **host-side Caddy** reverse proxy example keeps operator paths private:

```caddyfile
review.example.com {
    @operator path /api/jobs /api/jobs/* /metrics
    respond @operator 403
    reverse_proxy 127.0.0.1:3000
}
```

A **Private** GitHub App can be installed only on its owning account. A cross-account operator needs **Public** app visibility **plus** explicit installation admission; public visibility never grants permission to consume your compute or model budget. The service stores job metadata and delivery IDs, but interrupted jobs are marked failed after a restart, not automatically replayed.

## The engineering underneath

```mermaid
flowchart TD
    GH["GitHub PR event"] --> Entry{"Deployment"}
    Entry -->|Self-hosted App| Signed["Verify webhook signature & installation admission"]
    Entry -->|GitHub Action| Action["Validate Actions event context"]
    Signed --> Mode{"Requested mode"}
    Action --> Mode
    Mode -->|Review| RO["Read-only agent tools"]
    Mode -->|Authorized Fix| RW["Isolated edit & test tools"]
    RO --> Report["Evidence-aware PR response"]
    RW --> Verify{"Tests actually executed and passed?"}
    Verify -->|No / unknown| Report
    Verify -->|Yes| Publish["Recheck rights, SHA & branch; non-force push"]
    Publish --> Report
```

**How the layers divide responsibility:**

| Layer | Where to look | Responsibility |
| --- | --- | --- |
| GitHub trust boundary | `src/github/`, `src/server/` | Signed webhooks, installation tokens, GitHub API, job/status endpoints |
| Agent loop & tools | `src/agent/` | Model reasoning, tool routing, mode-specific capabilities |
| Workflow policy | `src/workflow/` | Command parsing, permission gates, verification, publication |
| Execution boundary | `src/sandbox/`, `Dockerfile.sandbox` | Ephemeral isolated workspace and resource restrictions |
| AI providers | `src/llm/` | Provider adapters and model calls |
| Skills | `src/skills/`, `skills/` | Loading supported skills and optional investigation packages |
| Delivery | `action.yml`, `docker-compose.yml` | Per-repository Actions or self-hosted service |

**The Fix path has multiple stopping points.** The requester must be authorized. The sandbox must meet isolation requirements. The tests must run and pass. The branch must still be the branch the agent inspected. The actor must still be authorized to publish. Protected branches must not be silently overwritten. If a check fails or its result is unavailable, publishing stops and the response should explain why.

Test-output detection is deliberately conservative but **not tamper-proof**: repository code can fabricate runner output. Treat the built-in verification as a pre-publication check, not as a proof of test coverage, a substitute for trusted CI, or a security certification.

### Run the CLI locally

```bash
cargo build --locked --release

# Load a configured local .env. Do not commit it.
set -a
. ./.env
set +a

./target/release/openreview verify-config
./target/release/openreview review --repo owner/repo --pr 42 --dry-run
```

`--dry-run` suppresses remote comments and pushes. To run a real on-demand review, remove it after configuring GitHub access. The local sandbox driver is for **trusted development only**; it is not a secure boundary for hostile repositories and cannot publish Fix changes. For the webhook daemon, use `openreview serve --host 0.0.0.0 --port 3000` only with the required rootless Docker setup.

### HTTP surface, in brief

| Endpoint | Purpose |
| --- | --- |
| `POST /api/webhooks/github` | GitHub events, authenticated by HMAC-SHA256 |
| `GET /health` | Service health |
| `GET /metrics` | Prometheus-format metrics; restrict access |
| `GET /api/manifest` | Manual GitHub App registration reference, not an automated registration flow |
| `GET /api/jobs` and `GET /api/jobs/{id}` | Operator-only job diagnostics with bearer-token authentication |
| `GET /` | Service overview |

## Deep Digger: the investigation workbench

Some bugs resist a quick code scan because the failure crosses configuration, state, timing, dependencies, and assumptions. The repository contains [**Deep Digger**](skills/deep-digger/README.md), an evidence-gated causal investigation engine and root-cause minimizer.

Deep Digger operates seamlessly in **two complementary styles**:
1. **Standalone Skill & NPM Package:** Available as an independent, zero-dependency package (`npm install -g deep-digger` or `bun add -g deep-digger`) and registered in universal agent registries via `npx skills add imMamdouhaboammar/deep-digger`. It provides a dedicated CLI (`deep-digger`), preflight doctor diagnostics, and standalone test harness generation.
2. **Native OpenReview Investigation Workbench:** Embedded directly under `skills/deep-digger/` as OpenReview's causal debugging subsystem. When `openreview-orchestrator` detects test regressions, non-deterministic flakes, or oversized crash payloads, it transitions to `INVESTIGATE` mode, employing Deep Digger's trace ingestion, causal DAG modeling, statistical flake estimation ($P(\text{FAIL})$), Rust-accelerated delta debugging (`deep-digger-min` with `--granularity tokens|lines`), and air-gapped HTML diagnostic dossiers.

It follows a different question: **What would disprove the explanation we're currently considering?**

Deep Digger is **not automatically installed into every OpenReview run** merely because its source lives in this repository. To use it in a target repository, install it via `npx skills add imMamdouhaboammar/deep-digger`, npm, or copy `skills/deep-digger/` to `.agents/skills/deep-digger/`, then follow its own README and `SKILL.md`. It helps structure investigations; it does **not** guarantee a unique root cause or discovery of unknown vulnerabilities.

## Verify the project, not just the pitch

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The [CI workflow](.github/workflows/ci.yml) also covers container builds, rootless sandbox checks, dependency auditing, and the Deep Digger test suite. A green badge is useful evidence for those checks, **not a blanket statement about every production deployment or every PR the agent might touch**.

## What OpenReview does not promise

- **Not an infallible reviewer.** Model findings require developer judgment, and a clean review does not prove the absence of bugs or zero-days.
- **Not an unconditional auto-fixer.** Missing tools, offline dependencies, failed/absent tests, restricted branches, or an unsupported sandbox can block publication. The default sandbox image includes Cargo, npm, Go, and pytest; Bun needs an operator-supplied image.
- **Not an inline-suggestion bot yet.** Findings and proposed suggestions are posted as Markdown comments, not native GitHub suggestion blocks.
- **Not an approval-by-emoji workflow.** Fix starts from an explicit authorized command, not a reaction.
- **Not a managed SaaS.** You provide the workflow/hosting, GitHub permissions, and AI credentials. Multi-installation admission is your responsibility when self-hosting.
- **Not a claim of comprehensive test coverage.** The gate checks execution evidence; your own trusted CI and branch policies still matter.

That restraint is intentional. **The most useful thing an engineering agent can say sometimes is: "I found a plausible repair, but I cannot prove it is safe to publish."**

---

### Build with me

If you have a reproducible failure, a hostile-input test, a sandbox boundary to challenge, or a clearer way to explain verification, [open an issue](https://github.com/imMamdouhaboammar/openreview/issues) or [send a pull request](https://github.com/imMamdouhaboammar/openreview/pulls).

I am interested in the cases that make an agent's confidence uncomfortable. Those are the cases that make the system better.

<p align="center"><strong>OpenReview · Review with evidence. Fix with proof.</strong></p>
