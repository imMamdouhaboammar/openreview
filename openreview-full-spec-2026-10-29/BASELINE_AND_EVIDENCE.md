# Baseline, evidence and confidence

Snapshot: **2026-10-29**. Observations are only valid for the named refs and timestamps. Re-query before acting. A pass on an old head SHA is not a pass on a changed head.

## Repository and PR stack

| Item | Base at snapshot | Head | Status observed | Blocker / next proof |
|---|---|---|---|---|
| PR [#1](https://github.com/imMamdouhaboammar/openreview/pull/1) | `main` | `feat/production-remediation`, `9ddca2c1d26feda1515aa8a1aa16ed72671fe482` | Open, non-draft; GitHub merge state CLEAN, CI green | PR body carries DO NOT MERGE pending genuine multi-install App/rootless hosting and permissions; Qlty status text reports 5 blocking issues, 3 vulnerabilities. |
| PR [#2](https://github.com/imMamdouhaboammar/openreview/pull/2) | `feat/production-remediation` | `feat/github-actions-byok`, `dd819f481c385a57e2eddf50b13932d98040a851` | Open draft; CLEAN, CI green | Draft; Qlty status text reports 4 blocking issues, 1 vulnerability; complete stacked-PR merge workflow and re-validate. |
| PR [#3](https://github.com/imMamdouhaboammar/openreview/pull/3) | `feat/github-actions-byok` | `feat/identity-review-quality`, `db2040eafc8cb4beb1ed62da2e845df136eb2cb1` | Open draft; CLEAN, CI green | Draft; Qlty status text reports 1 blocking issue; verify quality findings before undraft/merge. |

The Qlty counts above are **reporting text, not verified vulnerability findings**. The detailed Qlty view requires authorization and GitHub code scanning was not available to the reviewer. Fetch full finding evidence by authorized means, classify false positives and confirm fixes. Never dismiss unknown blockers by inference.

## Existing evidence

- PR #1 CI: [run 40813490261](https://github.com/imMamdouhaboammar/openreview/actions/runs/40813490261), Rust/dependency audit/container jobs passed at recorded SHA; review threads previously reviewed and resolved, which does not supersede merge blockers.
- PR #2 CI: [run 40725463868](https://github.com/imMamdouhaboammar/openreview/actions/runs/40725463868); hosted rootless sandbox test passed at recorded SHA.
- PR #3 CI: [run 40812437497](https://github.com/imMamdouhaboammar/openreview/actions/runs/40812437497); Rust, audit, images and hosted rootless dry-run passed at recorded SHA.
- Live model E2E: [openreview-e2e-lab run 40809709032](https://github.com/imMamdouhaboammar/openreview-e2e-lab/actions/runs/40809709032) passed with OpenCode Custom/Gemini; on authorized Fix it produced patch and tests and pushed commit `3c93de8138f8cd0d45bae2c4b7788d6eafcd522e`. This is **one tested path**, not proof that all providers, App installations, multi-turn conversation or bot interaction work.
- `codespaces-a9bc8d` observed online. The user-owned uncommitted `.gitignore` change is not part of a PR and must not be auto-staged.
- Inspect-inspired tracking: [OpenReview issue #4](https://github.com/imMamdouhaboammar/openreview/issues/4), scope: independent semantic entity diff, blast-radius and risk scoring.

## Source inventory, confirmed on `feat/identity-review-quality`

| Area | Code paths | Currently evident |
|---|---|---|
| Agent | `src/agent/runner.rs`, `src/agent/tools.rs`, `src/agent/prompt.rs` | 20 step and 200k token caps; Review gets readFile/loadSkill/reply; Fix additionally gets bash/writeFile. |
| PR context | `src/workflow/context.rs` | Title, description, base/head SHA, changed files and diffs fetched, completeness check against file count, refetch for head stability. |
| GitHub auth / webhook | `src/github/auth.rs`, `src/github/webhook.rs`, `src/github/client.rs` | HMAC SHA-256 signature verification; actual app installation E2E still needs proof. |
| Workflow | `src/workflow/{coordinator,command,publisher,verification}.rs` | Parse commands, orchestrate review/fix, verification and publishing checks. |
| Skills | `src/skills/{discovery,frontmatter,prompt}.rs` | Finds SKILL.md files with symlink defenses; has `loadSkill`, but policy compliance and playbook execution unproven. |
| LLM | `src/llm/{anthropic,openai,gemini,responses,custom_tests}.rs` | Provider adapters including OpenAI-compatible routes; only one real custom provider path proven in cited E2E. |
| Sandbox | `src/sandbox/{detect,docker,local,path,traits}.rs` | Rootless Docker path and local path; boundary equivalence must be tested per deployment. |
| Deployment | `action.yml`, `examples/github-actions.yml`, `Dockerfile`, `Dockerfile.sandbox`, `docker-compose.yml` | Composite Actions BYOK and self-hosting scaffolding are present. |
| Tests | `.github/workflows/ci.yml`, `tests/`, `scripts/verify-rootless-sandbox.sh` | CI and fixture tests already exist; add targeted regressions for unverified paths. |

## Important gaps, not claims of defects

1. PR comments and review threads are not yet proven to be fully retrieved, ordered, and causally linked to PR code context. Current `PullRequestContext` contains PR metadata and diff, not a durable conversation history model.
2. Collaborative bot-to-bot replies, loop controls and event dedupe are not proven E2E.
3. Skill discovery/loading exists, but no enforced proof that the agent selected, applied and checked repository playbook steps.
4. Advanced multi-file planning and debugging as a first-class durable job needs tests for cancellation, budgets, checkpoints, rebase/head movement and recovery.
5. App multi-install lifecycle (permissions, revoked/rotated tokens, webhook retries, tenant isolation, rootless uncredentialed worker) lacks required live evidence.
6. Semantic change graph and evaluated risk routing is future scope, not yet implemented.

## Revalidation commands (read-only)

```bash
gh pr list -R imMamdouhaboammar/openreview --state open --json number,title,headRefName,baseRefName,isDraft,mergeStateStatus,headRefOid
gh pr checks 1 -R imMamdouhaboammar/openreview
gh pr checks 2 -R imMamdouhaboammar/openreview
gh pr checks 3 -R imMamdouhaboammar/openreview
gh pr view 1 -R imMamdouhaboammar/openreview --json body,reviewDecision,mergeStateStatus
gh pr view 2 -R imMamdouhaboammar/openreview --json body,reviewDecision,mergeStateStatus
gh pr view 3 -R imMamdouhaboammar/openreview --json body,reviewDecision,mergeStateStatus
```

If CLI lacks authenticated permissions, use GitHub connector read tools, then label unavailable evidence as `unverified` and block dependent merges.
