# User Feedback & Defect Analysis: tester-army/e2e

Comprehensive synthesis of 230 GitHub issues, pull requests, and developer feedback for `tester-army/e2e`.

---

## 1. Top Developer Complaints ("What They Hate")

### A. The Brittle Replay Cache
- **Symptoms**: Tests pass initially with an AI model, but subsequent cached replays fail unexpectedly (`REPLAY_STALE`, `end-mismatch`).
- **Verbatim Issue Citations**:
  - `#1496`: "cache: a date with an ordinal suffix (October 26th) is not read as a date"
  - `#1459`: "Word-only relative times become end anchors"
  - `#1426`: "cache: replay rejects a completed delete when an unrelated clock changes"
  - `#1425`: "agent: next step sees an old record name after a successful replay"
  - `#1057`: "--strict-cache: REPLAY_STALE on unchanged pages since 0.19 — Next.js route announcer recorded as an alert anchor"
  - `#769`: "cache: short prefixed record ids like PROJ-016 are not treated as minted, so canvas steps never replay"
  - `#1371`: "cache: a thrown store write crashes test runner"
- **Root Cause**: The caching engine compares raw node text and alert elements verbatim without semantic token normalization. Any timestamp, relative date, minted entity ID, or Next.js accessibility announcement causes a cache miss.

### B. Security & Credential Exfiltration
- **Symptoms**: Passwords, API keys, and auth headers leak into error traces, markdown reports, and LLM context.
- **Verbatim Issue Citations**:
  - `#1473`: "test(mcp): run_code secrets tests flake under load / secrets leaked in traces"
  - `#1376`: "WebKit secure-field masks time out in rewritten JavaScript-URL frames"
  - `#1157`: "secrets: handle late-resolved values and prefixes consistently across steps and reports"
- **Root Cause**: Secret masking relies on in-memory async local slots and fragile browser-side masking scripts. If an iframe rewrites its origin or WebKit delays execution, values leak in cleartext into the trace log.

### C. Flakiness, Settle Races & Action Interception
- **Symptoms**: Agent clicks fail on modern UI libraries (Tailwind, Radix, Element Plus), or assertions fire before the page finishes re-rendering.
- **Verbatim Issue Citations**:
  - `#1514`: "agent: assert and waitFor cannot judge the current URL"
  - `#1362`: "agent.assert right after act causes race condition with async hydration"
  - `#1327`: "web: SVG-intercepted diagram labels cannot be double-clicked through agent point tools"
  - `#1276`: "Decision executor: check/tap fails (NOT_ACTIONABLE) on Element Plus radio-button — hidden input intercepted by styled span"
  - `#841`: "mobile/web: reads on a continuously updating screen take ~20–40 s each (no way to skip settle wait)"
- **Root Cause**: Reliance on fixed `sleep` intervals and naive point targeting. It lacks multi-layer DOM piercing (cannot click through styled wrapper spans or SVG overlays) and has no router-state awareness.

### D. Token Bloat & Latency
- **Symptoms**: E2E runs take minutes and cost substantial LLM tokens per test.
- **Verbatim Issue Citations**:
  - `#1091`: "dense screens are not cut to fit long judgment instructions, so e2e explore aborts while planning"
  - `#1510`: "web: configurable observation node budget for dense pages"
  - `#1466` & `#1467`: "perf(report): serialization bottlenecks on large DOM trees"
- **Root Cause**: Naive DOM dumping without intelligent semantic pruning.

### E. Missing Causal Root Cause Investigation
- **Symptoms**: When a test fails in CI or PR review, the developer gets an incomprehensible stack trace and a red failure icon.
- **Problem**: There is zero delta minimization (`ddmin`), no mapping back to the PR file changes, and no verified code fix suggestions.

---

## 2. OpenReview Strategic Positioning & Angle

> **For software engineering teams who want reliable, autonomous end-to-end bug hunting during pull request reviews without flaky tests, token waste, or security leaks:**
> **OpenReview's End-to-End Error Finders module delivers ultra-fast, Rust-powered E2E verification with bulletproof semantic cache normalization, fail-closed zero-trust redaction, multi-layer element piercing, and automatic Deep Digger causal fault localization.**
