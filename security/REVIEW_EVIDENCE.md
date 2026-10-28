# PR #1 Security Verification Evidence

Verified on 2026-10-28, including GitHub remediation commit
`ca02adabdcbe997ac53b18020c5fa0d703e345dd` and the later
GitHub App prefilled registration-URL generator. This is **evidence of the
checks actually executed**, not a certification of the runtime isolation
boundary. The final pushed head must be independently checked by GitHub
Actions as well.

## Tools, commands, and results

| Tool and version | Exact command | Scope and finding | Exit |
| --- | --- | --- | --- |
| Rust 1.99.1 rustfmt | `cargo fmt --check` | Entire Rust workspace: formatted | 0 |
| Rust 1.99.1 Clippy | `cargo clippy --all-targets -- -D warnings` | All Rust targets: no warnings | 0 |
| Rust 1.99.1 test runner | `cargo test --all` | 51 library unit, 1 binary unit, and 19 integration tests: **71 passed**, zero ignored/failed | 0 |
| Rust 1.99.1 compiler | `cargo build --release` | Release binary built | 0 |
| cargo-audit 0.22.2 / RustSec (1,381 cached advisories) | `cargo audit --no-fetch --db /root/.cargo/advisory-db` | 260 Cargo.lock dependencies; zero known vulnerabilities. One **allowed warning**, RUSTSEC-2024-0449 (`unsafe-libyaml` 0.2.11 unmaintained via `serde_yaml`) | 0 |
| Git | `git diff --check` | Changed source lines: no whitespace errors | 0 |
| ripgrep | See exact command below | Matched only README and .env.example sample PEM delimiters, manually checked as placeholders, not live private keys. Pattern-based scan, not a dedicated secret scanner | 0 |
| OpenReview binary | `./target/release/openreview verify-config` | Configuration inspection succeeded; fake/missing live credentials reported as warnings | 0 |
| OpenReview binary | `GITHUB_APP_ID=test GITHUB_APP_PRIVATE_KEY=dummy GITHUB_APP_WEBHOOK_SECRET=dummy ANTHROPIC_API_KEY=dummy JOB_API_TOKEN=ci-only-token-abcdefghijklmnopqrstuvwxyz012345 ./target/release/openreview serve --host 127.0.0.1 --port 4000` | Refused startup without a Docker execution backend (safe failure) | 1, expected |
| Docker CLI | `docker --version`, `docker compose config`, `docker build -f Dockerfile -t openreview-service:verify .` | Not available in local runner (`docker: command not found`) | 127 each |
| Bash syntax check | `bash -n scripts/verify-rootless-sandbox.sh` | New rootless-runtime smoke script parses correctly, but **was not executed successfully** against Docker | 0 |
| Rootless smoke runner | `SANDBOX_WORKSPACE_DIR=/tmp ./scripts/verify-rootless-sandbox.sh` | Refused with `Docker CLI is unavailable`. This is a failed prerequisite, **not a passing isolation test** | 2 |
| OpenReview binary | `cargo run --quiet -- github-app-url --domain https://review.example.org --name 'OpenReview Mamdouh' --org my-org --public` | Generated a GitHub-owned HTTPS registration link using only public configuration. Four new tests cover parameter accuracy, organization/public choice and invalid domain input. GitHub's live form was **not** submitted or independently exercised | 0 |

Exact pattern-based secret scan:

```bash
rg -l '(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{50,}|-----BEGIN (RSA|EC|OPENSSH) PRIVATE KEY-----)' src tests .github README.md Dockerfile docker-compose.yml .env.example
```

**Independent CI:** [GitHub Actions run 40678516258](https://github.com/imMamdouhaboammar/openreview/actions/runs/40678516258)
passed on `468c1ce`: Rust formatting, Clippy, all tests, release build, RustSec
audit, service Docker build, **actual execution of the built image's GitHub
App registration CLI**, sandbox Docker build, Compose static validation, and
rootless-smoke-script syntax. The next head containing documentation corrections
requires its own CI. None of these checks executes untrusted code inside a real
rootless Docker daemon or verifies a live GitHub App installation.

## Security regressions and actual mitigations

| Risk | Code boundary | Regressions and result |
| --- | --- | --- |
| Review-mode shell, Git, or network side effects | `agent/tools.rs`, `agent/runner.rs`, `sandbox/docker.rs` | Shell/write tools absent; dispatch rejects unexpected calls; commands attempting shell redirection, commit/push and HTTP POST cannot mutate review workspace; Docker Review workspace bind mount marked `readonly`, Fix remains writable (mount-argument test) |
| Unauthorized Fix publication | `workflow/coordinator.rs`, `workflow/publisher.rs` | Fix requires app client, proper collaborator rights, and a verified PR head; publisher refuses unauthenticated clients, Review mode, and dry runs do not push |
| No tests mistaken for success | `workflow/verification.rs` | `Unavailable` is non-passing and blocks publication; test asserts zero checks is not success |
| Secret exposure via untrusted repository skills | `skills/discovery.rs` | Symlinked skill directories and SKILL.md files rejected; test attempts reading outside workspace |
| Filesystem traversal | `sandbox/path.rs` | Tests reject absolute, parent traversal, valid and dangling symlinks |
| Git metadata/token exposure | `sandbox/docker.rs` | Trusted Git metadata and askpass remain outside container mount, hooks and global config disabled; regression test validates separation. No live container escape test executed |
| Webhook spoofing and duplicated deliveries | `github/webhook.rs`, `server/handlers.rs`, `jobs/manager.rs` | Invalid HMAC rejected; signed duplicate delivery accepted once; state and dedup survive restart; concurrent admissions result in one job |
| Sensitive job metadata disclosure | `server/handlers.rs` | Missing and invalid operator bearer tokens yield 401 on both API endpoints; permissive CORS removed |
| Multi-installation token cross-use | `github/auth.rs`, `workflow/coordinator.rs` | Cached installation tokens keyed by installation ID; targeted invalidation tested; signed webhook without installation metadata refused |
| Public App installation abuse | `server/handlers.rs`, `config.rs` | Signed webhook bearing an installation ID outside operator-approved `ALLOWED_INSTALLATION_IDS` is ignored before job admission; regression test covers an unapproved installation while an approved one still receives jobs |
| Unicode panic during truncation | `workflow/context.rs`, `workflow/verification.rs` | Multibyte-boundary tests pass |
| Unsafe or malformed self-hosted App registration links | `github/registration.rs`, `cli.rs` | GitHub-owned prefilled form, HTTPS origin validation, GitHub organization slug validation, no App credentials/registration secrets or untrusted redirects in the link; four regression tests pass. Live GitHub registration not performed |
| Broken service image at runtime despite successful image build | `Dockerfile`, `.github/workflows/ci.yml` | CI run 40678071221 found a real GLIBC 2.38/2.39 symbol mismatch: unqualified `rust:1-slim` used Debian Trixie while runtime used Bookworm. Builder now uses `rust:1.99-slim-bookworm`; CI run 40678516258 successfully launched the corrected container CLI. The CI regression test executes the built service image and fails if it cannot launch |

## Findings, risk acceptance, and release blockers

1. No known vulnerable Rust dependency was detected; `unsafe-libyaml` is
   unmaintained (RustSec RUSTSEC-2024-0449) and is a tracked maintenance risk.
   No exception was added to suppress the warning. Replacing the YAML stack
   is a bounded follow-up, not a claim that the library is actively maintained.
2. Webhook Local mode is forbidden and a rootless Docker backend is required.
   **Actual network/process/filesystem isolation, token confinement, shared
   path permissions, timeout/cancellation cleanup, and a complete Fix push have
   not been verified against a running rootless daemon.** Passing image builds
   alone cannot close this gap. `scripts/verify-rootless-sandbox.sh` was added
   as an operator-executed smoke test; it has **not** been run successfully in
   this runner because Docker is unavailable. The service now also refuses a
   rootless daemon without systemd cgroup v2 and checks actual per-container
   memory, PID and CPU controllers before letting untrusted tools execute.
3. The runner prohibits binding a localhost HTTP listener (EPERM). Tests that
   would have simulated GitHub permission responses over HTTP could not run;
   they were not counted as passing. The non-network checks validate permission
   mapping and rejection without a client, but live mid-run permission revocation
   and cross-installation access require integration tests with a usable
   network/runtime.
4. Persistent job metadata and delivery deduplication survive process restarts,
   but interrupted jobs are marked failed rather than automatically resumed.
   Queued work has a bound. Controlled shutdown, cancellation, retry backoff,
   and multi-process persistence locking have **not** been validated.
5. The operator job API uses an instance-wide bearer token and is intended for
   trusted deployment operators, not installation-scoped customer access.
   Protect it at the ingress proxy. Reaction-based approvals and native inline
   suggestions are not implemented or advertised as working.
   Public GitHub App registration additionally requires explicit installation
   approval through `ALLOWED_INSTALLATION_IDS` (default deny), so external
   installations cannot schedule jobs merely by installing the App.
6. ECC Tools' Security Evidence gate reported **passed** at `ca02ada`, after
   the initial remediation head requested further evidence. The ECC GitHub App
   separately lacked permission to publish a GitHub Check, so no ECC Check or
   SARIF upload is claimed. The next pushed head must be checked again.

**Release decision:** do not merge until unproven live security boundaries,
required reviews, and branch policies are independently verified.
