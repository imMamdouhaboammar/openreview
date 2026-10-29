# Poteto Mode Invariants in OpenReview

This document maps the core engineering principles of Poteto Mode to the OpenReview architecture and agent behavior.

---

## 1. Principle Mapping Matrix

| Poteto Principle | OpenReview Application | Enforcement Mechanism |
| :--- | :--- | :--- |
| **`principle-boundary-discipline`** | Review mode has zero write/shell tools; Fix mode is sandboxed with dropped root capabilities and unmounted Git credentials. | Enforced by Rust daemon (`WorkflowMode::Review` vs `WorkflowMode::Fix`) and Docker OCI profile. |
| **`principle-prove-it-works`** | Never declare a task complete or push commits without affirmative test execution logs. | Pre-publish verification gate requires exit code 0 AND positive test count output. |
| **`principle-laziness-protocol`** | Generate the smallest diff that solves the issue. Reject speculative refactors and unrelated code formatting. | Diff reviewer agent checks that modified lines map strictly to authorized tasks. |
| **`principle-subtract-before-you-add`** | Remove dead code, redundant abstractions, and unused imports before adding new logic. | Code cleanliness audit passes over modified files. |
| **`principle-model-the-domain`** | Represent review findings as structured severity objects (Blocker, Suggestion, Nit) with typed metadata. | Formatted via `format_review.py` schema validator. |
| **`principle-fix-root-causes`** | When PR tests break, do not patch symptoms or add blind retries. Isolate the minimal failing case via `ddmin`. | Causal handoff to `deep-digger-min` before generating fixes. |
| **`principle-minimize-reader-load`** | Technical payloads immediately under domain headers. Zero transitional small talk or conversational narration. | Direct markdown payloads and GitHub suggestions. |

---

## 2. The Verification Gate Contract

In `Fix` mode, the orchestrator detects the project's native build and test harness:

1. **Cargo (Rust):** `cargo test --all-targets --locked`
   - Verification assertion: Output contains `test result: ok. X passed; 0 failed`.
2. **Bun / NPM (TypeScript/JavaScript):** `bun test` or `npm test`
   - Verification assertion: Output contains passing test suite counts.
3. **Pytest (Python):** `pytest -v` or `python3 -m unittest`
   - Verification assertion: Output contains `passed` or `OK` with non-zero test count.
4. **Go:** `go test ./...`
   - Verification assertion: Output contains `ok <package>`.

If the test suite fails, times out, or produces zero executed tests:
- The push is **blocked immediately**.
- A diagnostic failure report is returned to the review thread.
- The PR branch is preserved in its original clean state.
