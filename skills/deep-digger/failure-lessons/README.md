# Deep Digger Failure Lessons & Durable Engineering Memory

> **The Core Invariant:**
> *"Pay for an engineering mistake once. After that, the project should remember it."*

This directory contains durable engineering post-mortems, failure analyses, and architectural invariants codified from real debugging, packaging, refactoring, and verification sessions.

---

## Structure

- [lessons-index.md](lessons-index.md): Master index of all codified failure lessons, categorized by domain and failure class, along with the active prevention rules enforced across the project.
- [npm-dist-packaging-and-agent-distribution.md](npm-dist-packaging-and-agent-distribution.md): Lessons extracted from the hybrid npm packaging, distribution isolation (`dist/`), compiler artifact security filtering, and asynchronous npm staging.

---

## Operating Protocol

1. **Explicit Schema Enforcement:** Every lesson adheres to the canonical 15-field markdown schema defined in `failure-lessons`.
2. **Fact vs Hypothesis Discipline:** Root causes are classified strictly as `Confirmed`, `Strongly indicated`, or `Unresolved`. Speculation is never documented as historical fact.
3. **No Sanitization:** Real failure traces, error messages, and incorrect assumptions are preserved intact to prevent recurring mistakes.
