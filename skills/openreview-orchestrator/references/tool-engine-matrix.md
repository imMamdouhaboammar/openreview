# OpenReview Tool & Engine Matrix

Comprehensive reference of all binaries, CLI commands, scripts, and environment parameters in the OpenReview ecosystem.

---

## 1. Binaries (`~/.cargo/bin`)

| Binary | Source Path | Key Subcommands & Flags | Purpose |
| :--- | :--- | :--- | :--- |
| `openreview` | `src/main.rs` | `review`, `serve`, `action`, `verify-config`, `github-app-url`, `test-webhook` | Primary review daemon and CLI review runner |
| `deep-digger` | `skills/deep-digger/bin/cli.js` | `init`, `hypothesis`, `evidence`, `experiment`, `trace`, `link`, `flake`, `bisect`, `repro`, `minimize`, `report`, `view`, `doctor` | Unified investigation CLI (standalone npm or OpenReview skill) |
| `deep-digger-min` | `skills/deep-digger/rust/src/main.rs` | `--input <FILE> --output <FILE> --allow-exec [--granularity lines\|tokens] [--repeats N] [--timeout SEC] -- <ORACLE> {input}` | High-speed Rust input minimizer (`ddmin`) |

---

## 2. Command Flags Reference

### `openreview review`
- `--repo <OWNER/REPO>`: Target GitHub repository (required).
- `--pr <PR_NUMBER>`: Pull request number to inspect (required).
- `--branch <BRANCH>`: Branch name (optional; fetched via GitHub API if omitted).
- `--dry-run`: Performs full analysis without posting comments or modifying branches.

### `openreview serve`
- `--host <IP>`: Host IP address to bind (default: `0.0.0.0` or `$HOST`).
- `--port <PORT>`: Port to bind (default: `3000` or `$PORT`).

### `openreview verify-config`
- Validates the active configuration, environment variables, GitHub App credentials, and LLM provider connectivity.

---

## 3. Environment Variables Reference

| Variable | Required In | Allowed Values / Format | Description |
| :--- | :--- | :--- | :--- |
| `LLM_PROVIDER` | All modes | `anthropic`, `openai`, `openrouter`, `opencode`, `opencode_go`, `gemini`, `ollama` | Active LLM inference engine |
| `LLM_MODEL` | Optional / Provider | e.g. `gpt-4o`, `claude-3-7-sonnet-20250219`, `gemini-2.5-pro` | Model identifier |
| `OPENAI_API_KEY` | OpenAI mode | `sk-...` | OpenAI secret API key |
| `ANTHROPIC_API_KEY` | Anthropic mode | `sk-ant-...` | Anthropic secret API key |
| `GEMINI_API_KEY` | Gemini mode | `AIzaSy...` | Google Gemini API key |
| `SANDBOX_DRIVER` | All modes | `local`, `docker` | Isolation driver (`local` for trusted dev; `docker` for untrusted PRs) |
| `GITHUB_TOKEN` | Standalone CLI | `ghp_...` / Ephemeral token | GitHub personal access token for reading PR diffs |
| `GITHUB_APP_ID` | Webhook Daemon | Numeric ID (e.g. `123456`) | Registered GitHub App ID |
| `GITHUB_APP_PRIVATE_KEY`| Webhook Daemon | PEM string | RSA Private Key of GitHub App |
| `GITHUB_APP_WEBHOOK_SECRET`| Webhook Daemon | 64-char Hex | HMAC-SHA256 signature secret |

---

## 4. Helper Scripts

- `scripts/orchestrate.py`: Intent classifier, DAG route evaluator, and execution supervisor.
- `scripts/format_review.py`: Validates and formats findings according to `agency-code-reviewer` and `suggestion` specs.
- `skills/openreview/scripts/verify_openreview.py`: Validates repository configuration against the 6 core invariants.
- `skills/deep-digger/scripts/deep_digger.py`: CLI for hypothesis and evidence management, trace ingestion, DAG linking, flake analysis, automated git bisect, and HTML reporting.
- `skills/deep-digger/scripts/build.js`: Clean packaging script for npm distribution bundle (`dist/`).
- `scripts/scan_skill_security.py`: NVIDIA/SkillSpector-aligned static security analyzer.
