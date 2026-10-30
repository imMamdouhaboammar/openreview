# Deterministic Delta Debugging Invariants & Statistical Oracle Bounds

## Mathematical Foundation & 1-Minimality

Delta Debugging ($ddmin$) reduces a failing input configuration $c$ to a smaller configuration $c'$ such that $c'$ still causes the target failure, and no single component of $c'$ can be removed without making the failure disappear.

Formally, a configuration $c'$ is **1-minimal** if:
$$\forall e \in c': \text{oracle}(c' \setminus \{e\}) \neq \text{FAIL}$$

### Complexity Bounds
- **Best Case** (single localized bug, power-of-two partitioning):
  $$O(2 \log_2 n) \text{ oracle invocations}$$
- **Worst Case** (worst-case interaction or fine-grained distributed defect):
  $$O(n^2) \text{ oracle invocations}$$

An upper bound test budget (`--max-tests`, default 200) guarantees termination. If the budget is exhausted before the 1-minimality check completes, the minimizer outputs `one_minimal_verified: false` to prevent false certainty.

---

## Token vs Line Granularity

Traditional line-based minimization fails when the offending input is dense, single-line, or JSON-formatted (e.g. minified bundles, AST payloads, HTTP request bodies).

Deep Digger provides two deterministic granularities:

```bash
# Line-level minimization (ideal for logs, multi-line configs, test scripts)
deep-digger minimize --input bug.txt --output min.txt --granularity lines --allow-exec -- ./oracle {input}

# Token-level minimization (ideal for single-line JSON, code blocks, regex strings)
deep-digger minimize --input payload.json --output min.json --granularity tokens --allow-exec -- ./oracle {input}
```

### The Lexical Preservation Invariant
Token-level splitting must preserve all whitespace and delimiters such that concatenation is strictly identity-preserving:
$$\text{concat}(\text{tokenize}(s)) \equiv s$$

This guarantees that removing candidate tokens produces syntactically predictable sub-configurations without corrupting un-removed boundaries.

---

## Fencing Flaky Oracles via Statistical Replication

Delta debugging assumes a monotonic, deterministic oracle. If the oracle is flaky (e.g. failing with probability $P(\text{FAIL}) = 0.60$), random passes will cause $ddmin$ to prematurely discard critical components, outputting an invalid minimized input.

To defend against flakiness:
1. **Repeated Evaluations**: Every candidate configuration is tested $R$ consecutive times ($2 \le R \le 10$). If all $R$ runs do not yield an identical verdict, the outcome is classified as `UNRESOLVED`.
2. **Conservative Pruning**: An `UNRESOLVED` verdict is treated as non-failure and is never used to confirm removal of a chunk.
3. **Flake Oracle Preflight**: Before invoking minimization, run `deep-digger flake` to empirically verify that $P(\text{FAIL}) \approx 1.0$.

---

## Sandbox & Safety Boundaries

Executing arbitrary commands to evaluate untrusted candidate inputs requires strict defensive containment:
- **Explicit Opt-in**: Commands are never executed without `--allow-exec`.
- **Private Scratch**: Scratch directories are created with restrictive permissions (`0700` on POSIX systems) to prevent symlink attacks or multi-tenant snooping.
- **Bounded Timeouts**: Timeouts are enforced per trial ($1 \le t \le 600\text{s}$) to terminate infinite loops spawned by reduced or malformed candidate payloads.
