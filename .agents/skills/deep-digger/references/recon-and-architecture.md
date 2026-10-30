# Reverse Engineering & Architecture: Fault Localization & Causal Analysis

A comprehensive synthesis of fault localization, delta debugging, and causal inference architecture reverse-engineered from industry leaders (Zeller's Delta Debugging, Sentry, Datadog APM, rr, and Pearl's Causal Framework).

---

## 1. Architectural Topology

```
+-------------------------------------------------------------------------+
|                           INGESTION BOUNDARY                            |
|  - CLI Commands (--allow-exec safe isolation)                           |
|  - OpenTelemetry Traces (Span tree, error events, latencies)            |
|  - HAR Network Captures (HTTP 4xx/5xx, request/response digests)        |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                         CAUSAL REASONING ENGINE                         |
|  - Hypothesis Graph (DAG with cycle detection)                          |
|  - Discriminating Experiment Generator                                  |
|  - Statistical Flake Classifier (Empirical P(FAIL) estimation)          |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                         MINIMIZATION & ISOLATION                        |
|  - Zeller ddmin Algorithm (1-minimality verification)                   |
|  - Zero-Dependency Rust Minimizer (deep-digger-min)                     |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                         DURABLE EVIDENCE LEDGER                         |
|  - case.json (Tamper-evident SHA-256 artifact hashes)                   |
|  - Atomic filesystem persistence (mkstemp + fsync + os.replace)         |
+-------------------------------------------------------------------------+
```

---

## 2. Reverse-Engineered Competitive Analysis

| System / Tool | Core Strength | Key Vulnerability | Deep Digger Synthesis |
|---|---|---|---|
| **Sentry / Bugsnag** | Error grouping & fingerprinting | Grouping collapses distinct root causes into one cluster | Deep Digger isolates discrete counterfactual mechanisms |
| **Datadog / APM** | Distributed trace waterfalls | Correlates downstream timeouts without proving upstream cause | Ingests span error events as falsifiable evidence |
| **Zeller ddmin** | Rigorous 1-minimal input reduction | Assumes monotonic oracle; hangs on non-deterministic bugs | Bounded test budgets & explicit unresolved verdict handling |
| **rr / Valgrind** | Deterministic record & replay | Massive recording overhead, Linux-only, heavy memory | Lightweight, multi-platform, portable Python & Rust |
| **Ad-Hoc Agent Fixes** | Rapid syntax edits | 80% regression rate due to premature pattern-matching | Four-phase counterfactual verification ($A/B/A/C$) |

---

## 3. Mathematical Foundations of Delta Minimization

Given a failing input sequence $c \in C$ such that $\text{oracle}(c) = \text{FAIL}$ and $\text{oracle}(\emptyset) = \text{PASS}$, the objective of delta minimization ($ddmin$) is to find a 1-minimal subset $c^* \subseteq c$:

$$\forall x \in c^*, \quad \text{oracle}(c^* \setminus \{x\}) \neq \text{FAIL}$$

### Complexity Bounds
- **Best case (binary search)**: $O(2 \log_2 |c|)$ oracle invocations.
- **Worst case (every element required)**: $O(|c|^2)$ invocations.
- **Deep Digger Guard**: Enforces an explicit `max_tests` budget. When the budget is exhausted, it outputs the reduced intermediate input and marks `one_minimal_verified: false` to prevent false claims of minimality.

---

## 4. Causal DAG Formalism

Hypotheses in Deep Digger form a Directed Acyclic Graph (DAG) $G = (V, E)$:
- **Vertices ($V$)**: Discrete causal hypotheses ($H_1, H_2, \dots$).
- **Edges ($E$)**: Semantic causal relations:
  - `causes`: $H_A$ directly induces $H_B$ under all conditions.
  - `triggers`: $H_A$ provides the temporal or operational trigger for latent condition $H_B$.
  - `enables`: $H_A$ weakens a systemic defense, allowing $H_B$ to manifest.

Cycle prevention is enforced deterministically via depth-first search (DFS) topological ordering before edge insertion.
