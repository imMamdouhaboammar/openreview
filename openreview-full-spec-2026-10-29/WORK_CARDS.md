# Atomic work cards for coding agents

Use with `IMPLEMENTATION_PLAN.md`. Every card contains one vertical behavior slice: fixture, implementation, verification and evidence. `Owner` is suggested file ownership, not permission to modify unrelated files. Dependencies below are hard prerequisites for shipping. An agent may independently draft a test for blocked future work, but it must not merge the dependent feature early.

## Card registry

| Card | Priority | Depends on | Primary code owner | Acceptance tests | Reversal path |
|---|---|---|---|---|---|
| M0-01 | P0 | None | Evidence docs/PR state | MRG-01, MRG-02 | Correct record without rewriting code |
| M0-02 | P0 | M0-01 | `src/github`, `src/server`, `src/sandbox` | APP-01..05, SEC-01..03 | Disable App write capability, keep PR open |
| M0-03 | P0 | M0-01, M0-02 | Targeted defects only | MRG-03, MRG-04 | Corrective PR/revert on main, never rewrite |
| M1-01 | P0 | M0-03 | `src/llm`, `src/action.rs`, `action.yml` | ACT-01..05 | Revert provider-specific adapter change |
| M1-02 | P0 | M0-03 | `src/server`, `src/jobs`, deployment | APP-01..05 | Pause App installations/rollback image |
| M2-01 | P0 | M1-01, M1-02 | `src/workflow/context.rs`, `src/github/client.rs` | CONV-01, 02, 06 | Fallback read-only review; mark incomplete |
| M2-02 | P0 | M2-01 | command/coordinator/publisher | CONV-03..05, SEC-04 | Disable comment-triggered Fix, retain read-only |
| M3-01 | P1 | M2-02 | `src/skills`, `src/agent` | SK-01, 02, 04, 05 | Disable newly added skill policy route |
| M3-02 | P1 | M3-01 | `src/skills/playbook.rs`, coordinator | SK-03, 04, SEC-05 | Reject all custom playbooks, keep skills read-only |
| M4-01 | P1 | M3-02 | agent runner/tools/checkpoint | FIX-01, 05, 06, SEC-06/07 | Downgrade to Review/Suggest, disable Fix |
| M4-02 | P0 | M4-01 | verification/publisher | FIX-02..04, SEC-08/10 | Stop publisher and mark pending, preserve patch |
| M5-01 | P1 | M4-02 | jobs/server/CI/docs | OPS-01..03, SEC-09/11 | Pause queue; roll back to previous tagged release |
| M6-01 | P2 | M5-01 | new semantic parser/graph | SEM-01, 02, 05 | Opt out and textual fallback |
| M6-02 | P2 | M6-01 | new semantic risk/validation | SEM-03, 04 | Turn off risk ranking, keep baseline review |

## Ready-to-code card detail

### M0-01 | Freeze reality, triage quality blockers

**Input:** GitHub PR state, authenticated Qlty results, local status. **Output:** for each #1/#2/#3, exact SHA, base, CI, approvals, blocker IDs and reproduction. **Fail-first:** a fabricated pass with missing Qlty detail must be rejected by evidence validation. **Done:** triage fields and current timestamps exist, no unresolved unknown marked `pass`. **Stop:** security source inaccessible. **Handoff:** evidence ledger plus next blocker.

### M0-02 | Prove no credential/installation boundary crossing

**Input:** App webhook lab, two installations, rootless worker. **Output:** two isolated job results and negative tests. **Fail-first:** wrong installation must fail to fetch repo, invalid HMAC zero side effects. **Done:** rootless no-host-secret probe plus revoke, token expiry, duplicate webhook tests pass. **Stop:** any credential appears in sandbox. **Handoff:** redacted run links, policy matrix.

### M0-03 | Land verified PR stack

**Input:** cleared blocking findings and current heads. **Output:** #1/#2/#3 content on `main`, or explain blocked. **Fail-first:** blocker regression. **Done:** every PR gate recorded, stack retargeted without accidental omissions and post-merge `main` green. **Stop:** branch protection/force-push/new conflicts or missing authority. **Handoff:** merge SHAs, diff audit, residual risk.

### M1-01 | Four-provider Actions BYOK

**Input:** documented secrets in consumer workflow. **Output:** correct model tool-call conversations and bounded GitHub result. **Fail-first:** each adapter's tool-call continuation fixture. **Done:** OpenCode, OpenRouter, Anthropic and OpenAI contract suite passes; real OpenCode E2E tested using owner-provided key, no secret log. **Stop:** unknown model syntax silently routes another provider. **Handoff:** exact supported matrix and tested YAML.

### M1-02 | Self-hosted App lifecycle

**Input:** install/uninstall events and configured operator backend. **Output:** installation-scoped event routing, cancellation, health and redacted metrics. **Fail-first:** removed App installation must not publish. **Done:** two install E2E and rootless reproducibility pass. **Stop:** shared token crosses tenant. **Handoff:** operator runbook and recovery rehearsal.

### M2-01 | Complete PR conversation graph

**Input:** PR metadata, file pages, discussion/inline review pages. **Output:** `conversation-event.schema.json` list with parent IDs, provenance and completeness. **Fail-first:** a 101-comment fixture exposes pagination truncation. **Done:** complete count checked, correct causal parents, head stable. **Stop:** unknown pagination gap. **Handoff:** API mock and sample redacted event JSON.

### M2-02 | Human and bot interaction

**Input:** event + thread graph + actor permission. **Output:** single contextual reply/job. **Fail-first:** self-authored reply, bot imitation and repeated delivery provoke no second job. **Done:** authorized follow-up works; CodeRabbit/Qodo feedback is independently checked as evidence; quoted mentions never trigger. **Stop:** command acceptance from bot-authored untrusted text. **Handoff:** event replay and permissions logs.

### M3-01 | Skill loading with proof

**Input:** repo skills and explicit request/task context. **Output:** selection metadata, SHA-256 content digest, attempted/applied/verified steps. **Fail-first:** selected but unexecuted skill cannot appear 'applied'; symlink and duplicate name safe. **Done:** known fixture skill behavior observed and verify result logged. **Stop:** skill tool tries to override safety rules. **Handoff:** proof records and precedence guide.

### M3-02 | Bounded playbook engine

**Input:** schema-validated playbook and host permission policy. **Output:** ordered step evaluation plus trustworthy status. **Fail-first:** denied prerequisite or unknown tool does nothing. **Done:** typed `check`, `review`, `propose_patch` or authorized `fix` steps only; scope cannot expand. **Stop:** arbitrary executable scripts from repository metadata. **Handoff:** reference manifests and allowlist rationale.

### M4-01 | Multi-file investigation/fix loop

**Input:** snapshot, repository policy, authorized goal. **Output:** plan, bounded read-only search/check/log traces, file edits and checkpointed evidence. **Fail-first:** cross-file regression remains broken with limited tool use, then targeted behavior is added. **Done:** one realistic two+ file fix with tests and clear budget/cancel behavior, plus TOOL-01..03. **Stop:** tool writes beyond checkout or repeated futile calls. **Handoff:** plan trace and diff.

### M4-02 | Independently trusted verification/publish

**Input:** candidate patch and current verified head. **Output:** tests/proof plus commit SHA on authorized feature branch. **Fail-first:** fake 'all tests passed' model response rejected. **Done:** post-test SHA checked, permission rechecked, no-force push, observed resulting GitHub commit. **Stop:** stale head/protected branch. **Handoff:** verification ledger.

### M5-01 | Operable and recoverable releases

**Input:** interrupted queue, rate-limit events, restart and big PR. **Output:** idempotent resumable jobs and release evidence. **Fail-first:** duplicate publish on replay and invisible truncated context. **Done:** bounded recovery and failure state, redacted telemetry, tested operator updates. **Stop:** unbounded retry/cost. **Handoff:** versioned release docs and rollback procedure.

### M6-01 | Independent symbol graph

**Input:** target PR ref and source, parser coverage map. **Output:** spans of changed entities plus bounded dependents. **Fail-first:** fixture misses caller after symbol change. **Done:** Rust, TS/JS and Python fixtures, deterministic budget, parse failure fallback. **Stop:** licensing questions or exploding graph. **Handoff:** coverage/latency and authorship/license evidence.

### M6-02 | Risk calibration and finding validation

**Input:** symbol graph, textual baseline, known bug and benign corpora. **Output:** risk-ranked *candidate* findings with verifiable links to evidence. **Fail-first:** benign refactor incorrectly ranked as proven critical. **Done:** precision/recall/cost/latency measured, no unsupported exploit claims, opt-in roll back. **Stop:** evaluation worse than baseline or unknown quality. **Handoff:** benchmark report and explicit remaining blind spots.

## Every card's implementation checklist

1. Verify prerequisites and exact repo/SHA; register ownership, unresolved questions and expected rollback.
2. Add or identify a **failing deterministic test** for the promised behavior.
3. Implement the smallest behavior change within named paths.
4. Run targeted test, integration, security regressions, and complete relevant CI.
5. Inspect staged patch for unrelated files, secrets, permission widening and silent test weakening.
6. Make atomic commit, push to a traceable PR, request independent review and document test/CI evidence.
7. Close/merge only after gate passes; next card cannot quietly assume success.

## Ticket template for GitHub issues

```markdown
## Card
M?-?? | title

## Precondition and evidence
Exact input SHA, available permissions, current failing test, related issues/PRs.

## Scope
Files/paths, interfaces consumed/produced, exclusions.

## Acceptance
- [ ] Failure reproduced before implementation.
- [ ] Behavior works in unit and integration.
- [ ] Negative security test passes.
- [ ] CI/check links and exact output SHA recorded.
- [ ] Rollback documented.

## Dependencies
Previous card IDs and reasons.

## Stop conditions
Protected branch, security blocker, stale head, credentials risk or absent approval.
```
