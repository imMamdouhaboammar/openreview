# Architecture decision records

## ADR-001: Dual distribution, one core

**Decision:** Existing Rust core powers both Actions composite and the self-hosted App. Keep deployment-specific event/auth adapters outside the review/fix engine. **Reason:** avoids semantic drift while preserving different secret and lifecycle models. **Tradeoff:** more cross-mode E2E required. **Revisit if:** a real product requirement needs provider-specific capability.

## ADR-002: Untrusted repository content and bot comments

**Decision:** PR descriptions, code, issue comments, skills and external bot messages are always data. Only authenticated maintainer permissions plus explicit host policy authorize mutations. **Reason:** prevent prompt injection. **Tradeoff:** fewer fully autonomous tasks for untrusted actors.

## ADR-003: Trusted publication outside agent worker

**Decision:** Model-controlled tool runtime is uncredentialed; only trusted publisher holds narrow App installation token / workflow token and rechecks head SHA before push. **Reason:** prevents exfiltration and stale commits. **Tradeoff:** extra pipeline stage.

## ADR-004: Review vs Suggest vs Fix

**Decision:** Review and Suggest are read-only; Suggest returns a diff proposal. Only Fix gets workspace write/exec and trusted publishing. **Reason:** principle of least authority. **Open implementation detail:** CLI currently has Review/Fix; choose whether Suggest maps to Review with explicit output schema or a separate enum, with tests.

## ADR-005: GitHub Actions tokens and event recursion

**Decision:** Treat `GITHUB_TOKEN`-authored commit events as generally non-retriggering, but support follow-up verification with explicit workflow dispatch/check invocation. Exceptions exist in current GitHub docs. **Reason:** do not falsely assume a successful push implies CI. Prefer verify-on-pushed-SHA via checks API and a documented dispatch mechanism where authorized.

## ADR-006: Skills are guidance; playbooks are bounded policy data

**Decision:** Project skills may guide tool choice but cannot grant access. Playbooks are parsed into constrained, testable typed actions. **Reason:** a user-controlled skill could otherwise execute arbitrary host commands.

## ADR-007: Inspect-inspired features, independent implementation

**Decision:** Borrow concepts (entity-level diff, blast-radius, validation, review lenses) only. Do not copy, vendor, translate or adapt Ataraxy-Labs/inspect code unless legal rights are established. Its `LICENSE.md` is FSL-1.1-ALv2 at the observed snapshot, with competitive-use restrictions. **Reason:** legal and technical independence. Avoid restating vendor performance claims as facts.

## ADR-008: Evidence before merge

**Decision:** CI and GitHub `CLEAN` are necessary but insufficient. Gate on security findings, App deployment, branch identity, required review, smoke evidence and permission controls. **Reason:** green unit tests can coexist with deployment or authorization defects.

## ADR-009: Store minimum durable state

**Decision:** Persist only job events/checkpoints, dedupe keys and redacted audit metadata needed for replay. No persistent raw model keys, unbounded source corpus or raw secret-bearing logs. **Reason:** tenancy and breach impact.

## ADR-010: Work in narrow release slices

**Decision:** Land existing PR stack first, then conversational core, skills/playbooks, advanced Fix, semantic review. Some independent tests/security hardening may proceed concurrently on separate branches, but a dependent phase does not ship before its gate. **Reason:** keep verified release state clear.
