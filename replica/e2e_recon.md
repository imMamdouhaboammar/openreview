# Replica Reconnaissance: tester-army/e2e & OpenReview E2E Module

Reverse engineering and architectural decomposition of `tester-army/e2e` (Next-gen natural language agentic E2E framework) to guide OpenReview's End-to-End Error Finders module.

---

## 1. System Inventory: tester-army/e2e

| Subsystem / Package | Internal Role | Core Mechanisms | Known Bottlenecks / Flaws |
| :--- | :--- | :--- | :--- |
| `packages/kernel` | Core engine contracts & surface abstraction | Defines `Observation`, `SemanticNode`, `ViewportPoint`, `KeyModifier`, `ExecutionSurface` | Monolithic node serialization, byte-budget truncation cuts valid AST |
| `packages/e2e/src/cache` | Replay caching & Anchor verification | `anchors.ts`, `relocate.ts`, `template.ts`, `store.ts`, `identity.ts` | Literal string comparisons (`text`) fail on dynamic dates, relative times, minted IDs; store write exceptions crash runner (#1371) |
| `packages/e2e/src/agent` | Autonomous agent driver & Observation | `observation.ts`, `actions.ts`, `pixels.ts`, `executor.ts` | Token bloat on dense DOM trees (#1091, #1510); race condition on assert right after act (#1362) |
| `packages/e2e/src/secrets` | Credential isolation & redaction | `secrets.ts`, `AsyncLocalStorage`, `realmSlot` | Global memory leaks across attempts; masks fail in WebKit / iframes (#1376); raw secrets leak in traces (#1473) |
| `packages/e2e/src/engine` | Browser CDP / Mobile drivers | WebKit, Chromium, Firefox, Appium / xcrun | Element interception on styled spans (#1276), SVG backdrops (#1327); slow settle wait loops (#841) |
| `packages/e2e/src/expect` | Assertions & verification | `attempt.ts`, `soft.ts`, `poll.ts` | Cannot assert URL or router state (#1514); no root-cause delta debugging on failure |

---

## 2. Inferred Data Model

```
TestDefinition
 ├── name: String
 ├── timeout: Duration
 └── steps: Vec<AgentStep>
      ├── Act(goal: String)
      ├── Assert(claim: String)
      └── LocatorCheck(locator: Locator, expected: Value)

TraceRecord (Replay Cache)
 ├── semantic_hash: Sha256Hash
 ├── canonical_route: String
 ├── actions: Vec<RecordedAction>
 ├── appeared_anchors: Vec<SemanticAnchor>
 └── gone_anchors: Vec<SemanticAnchor>

E2eErrorFinding (OpenReview Integration)
 ├── id: Uuid
 ├── failure_signature: String
 ├── step_index: usize
 ├── failing_assertion: String
 ├── minimal_repro_steps: Vec<String>
 ├── localized_pr_diff: Option<DiffHunk>
 ├── redacted_dom_snippet: String
 └── suggested_patch: Option<String>
```

---

## 3. Screen & State Transition Model

```
State S0 (Initial Route)
  │
  ├── [Act: Navigate / Click / Input]
  ▼
State S1 (In-Flight Mutation: Network / rAF / React Hydration)
  │
  ├── [Adaptive Settlement Oracle (Network-Idle + DOM Debounce)]
  ▼
State S2 (Stabilized DOM)
  │
  ├── [Semantic Normalizer & Security Sanitizer]
  ▼
State S3 (Normalized Semantic State)
  ├── Cache Hit: Instant Replay (Zero LLM Tokens)
  └── Cache Miss: LLM Exploration -> Error Finding -> Deep Digger Delta Minimization
```
