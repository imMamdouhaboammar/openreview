# OpenReview: Full Agentic Platform Execution Specification

**Snapshot date:** 2026-10-29 (Africa/Cairo)  
**Target:** https://github.com/imMamdouhaboammar/openreview  
**Status:** Execution-ready planning package; NOT a claim that any listed feature or merge is complete.

## Outcome

Build OpenReview into a reliable, independently deployable GitHub-native AI review and coding agent. Users supply their own model credentials. It must read complete PRs and conversations, collaborate safely with humans and other bots, follow repository skills and playbooks, reason across files, review and fix code, verify changes and publish auditable results.

Two supported deployment modes are mandatory:

1. **GitHub Actions BYOK:** repository owner installs a workflow and relevant secrets. Review and authorized Fix run in a GitHub-hosted runner with an isolated workspace.
2. **Self-hosted GitHub App:** app operator hosts webhook receiver and workers. App installations are isolated per tenant. Owner manages infrastructure and provider credentials; do not claim GitHub hosts the App backend free of charge.

An independently implemented **semantic review** extension, inspired conceptually by Ataraxy-Labs/inspect, is a later milestone, not a blocker for the initial merge.

## Read in this order

1. `BASELINE_AND_EVIDENCE.md`: real PR and source-code state, known unknowns.
2. `PRODUCT_SPEC.md`: what the application must do, user stories and non-goals.
3. `ARCHITECTURE.md`: trust boundaries, state machines and interfaces.
4. `SECURITY_AND_PERMISSIONS.md`: invariants and threat model before any new execution paths.
5. `IMPLEMENTATION_PLAN.md`: milestone dependency graph and work order.
6. `WORK_CARDS.md`: atomic implementation tickets and TDD acceptance.
7. `TOOL_CAPABILITY_CATALOG.md`: exact read/search/check/fix/DevOps capabilities and trust gates.
8. `CONTRACTS/`: proposed JSON contracts, not currently shipped public APIs.
9. `TEST_AND_EVIDENCE_MATRIX.md`: repeatable verification and release proof.
10. `MERGE_AND_RELEASE_RUNBOOK.md`: merge gates, rollback, published-state checks.
11. `AGENT_EXECUTION_PROMPT.md`: paste into a coding agent alongside the ZIP.

Other useful references: `DEPLOYMENT_BYOK_AND_APP.md`, `ADR.md`, `TRACEABILITY.csv`, `SOURCES.md`, `EXECUTION_BRIEF.json`.

## Operational rules

- **Never infer merge readiness from a green CI badge alone.** Check blockers, branch protection, Qlty findings, App E2E and explicit outstanding review conditions.
- Never merge a stacked child PR to its temporary parent branch and mistake that for landing on `main`. Follow `#1 -> #2 -> #3` after adapting the stack and verifying each base.
- Preserve user-owned changes in `codespaces-a9bc8d`; the `.gitignore` modification seen in the Codespace is uncommitted and outside this package's authority.
- No pushing or merging to the target repo is authorized by the request to produce this ZIP. This is a specification deliverable.
- No synthetic passing checks, weakening tests, bypassing required review, invented security evidence or hidden provider credentials.
- Use small coherent commits per behavior, not arbitrary commit quotas.

## Release definition

**R1: Safe usable product** = guarded merges, Actions BYOK, App installation E2E, authenticated event intake, PR reading, human interactions, bounded review and authorized Fix with reproducible proof.  
**R2: Project-aware coding agent** = policy-aware skills/playbooks, bot collaboration, multi-file planning/execution, verification and recovery.  
**R3: Semantic review** = entity-level change graph, bounded blast-radius, prioritized validated findings with measurable evaluation.

## Arabic executive note

الحزمة دي مواصفة تنفيذ، مش إعلان إن OpenReview وصل للمستوى النهائي. البداية الحقيقية هي إقفال فجوات PRs #1 و#2 و#3 بأدلة تشغيل وأمان، وبعدها نبني قدرات التفاعل مع البشر والبوتات، ثم Skills/Playbooks والـAgentic Coding، وأخيرًا المراجعة الدلالية. كل مرحلة لها اختبارات ومنع صريح للدمج لو الأدلة ناقصة.
