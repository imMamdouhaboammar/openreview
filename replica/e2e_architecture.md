# Replica Architecture: OpenReview End-to-End Error Finders Module

High-performance, secure, evidence-gated autonomous E2E error finder subsystem built natively in Rust for OpenReview.

---

## 1. Architectural Stack

| Layer | Technology | Rationale |
| :--- | :--- | :--- |
| **Core Engine** | Rust (`openreview::e2e`) | Sub-millisecond execution, zero-copy string parsing, memory safety, deterministic concurrency |
| **Semantic Normalizer** | Rust Regex + Token Rewriter | Canonical state hashing immune to date, clock, ID, and hydration noise |
| **Security Sanitizer** | Constant-Time Masker & Zero-Trust PII Redactor | Fail-closed credential masking across traces, DOM, and LLM payloads |
| **DOM & Token Pruner** | Rust Tree Walker + Node Filter | 85-95% token reduction on dense pages, preserving strict semantic tree hierarchy |
| **Settlement Oracle** | Async Dual-Condition Debounce (Network + DOM) | Eliminates fixed sleeps and flakiness on SPAs (Next.js, Vite, React 19) |
| **Replay Cache** | ACID Transactional Append-Only Store | Atomic writes, monotonic deadline polling, corruption resilience |
| **Causal Error Finder** | OpenReview Deep Digger (`ddmin` delta debugging) | 1-minimal error isolation and PR diff fault localization |

---

## 2. Core Subsystems

```
                                      +------------------------------------+
                                      | PR Diff & Target Web Application   |
                                      +-----------------+------------------+
                                                        |
                                                        v
                                      +------------------------------------+
                                      |     OpenReview Sandbox Runner      |
                                      +-----------------+------------------+
                                                        |
                         +------------------------------+------------------------------+
                         |                                                             |
                         v                                                             v
        +----------------------------------+                         +----------------------------------+
        |   Semantic Normalizer & PII      |                         |    Action Engine & Settlement    |
        |   - Date/Time/Ordinal Masking    |                         |    - Multi-layer Element Piercing|
        |   - Prefixed/UUID ID Tokenizer   |                         |    - Route & URL Invariants      |
        |   - Zero-Trust Secret Redactor   |                         |    - Network & DOM Debounce      |
        +----------------+-----------------+                         +-----------------+----------------+
                         |                                                             |
                         +------------------------------+------------------------------+
                                                        |
                                                        v
                                      +------------------------------------+
                                      |      Replay Cache Engine (ACID)    |
                                      |      - Monotonic Deadline Poller   |
                                      |      - Normalized State Hash Check |
                                      +-----------------+------------------+
                                                        |
                                      +-----------------+------------------+
                         [Cache Hit]  |                                    |  [Cache Miss / Anomaly]
                         +------------+                                    +------------+
                         v                                                              v
              +----------------------+                                       +----------------------+
              | Instant Verified     |                                       | Autonomous Error     |
              | Replay (0 LLM Cost)  |                                       | Exploration Agent    |
              +----------------------+                                       +----------+-----------+
                                                                                        |
                                                                                        v
                                                                             +----------------------+
                                                                             | Deep Digger Delta    |
                                                                             | Minimizer (ddmin)    |
                                                                             +----------+-----------+
                                                                                        |
                                                                                        v
                                                                             +----------------------+
                                                                             | Evidence-Gated PR    |
                                                                             | Review Finding & Fix |
                                                                             +----------------------+
```

---

## 3. The 6 Load-Bearing Invariants

1. **Semantic State Invariant**: No state hash shall incorporate unnormalized timestamps, ordinals, relative time offsets, or dynamic random IDs.
2. **Fail-Closed Security Invariant**: Secrets matching credit cards, API keys, passwords, bearer tokens, or GitHub auth shall be redacted before any trace log or network payload is serialized.
3. **Token Thrift Invariant**: Observation trees sent to LLMs must stay within the strict token budget (max 2,500 tokens) with zero structural syntax breakage.
4. **Action Piercing Invariant**: Interaction dispatch shall inspect styled wrapper spans, SVG backdrops, and pseudo-elements to reach interactive DOM elements.
5. **Atomic Store Invariant**: Replay cache mutations must use atomic temporary-file renames to prevent partial-write corruption upon sudden process abort.
6. **Causal Falsification Invariant**: No E2E bug shall be reported without a 1-minimal reproduction sequence and verified localization against the active PR diff.
