# J-Space Cognitive Architecture for Deep Causal Investigation

When autonomous coding agents tackle stubborn, ambiguous, or recurring bugs, raw statistical token prediction frequently produces premature pattern matching. An agent hallucinates a plausible cause from a nearby stack trace, generates a symptom-masking patch, and stops before establishing proof.

Operating Deep Digger within the **J-Space** inner cognitive workspace ensures that internal deliberations remain rigorous, falsifiable, and grounded in observable artifacts.

---

## 1. The Five J-Space Workspace Properties in Debugging

```
+-------------------------------------------------------------------------+
|                          J-SPACE INNER REGISTER                         |
|                                                                         |
|  [Capacity]          Only 2 competing hypotheses admitted to stage      |
|  [Introspection]     Check unspoken assumptions before proposing fixes  |
|  [Directed Focus]    Hold invariant truth through mechanical steps      |
|  [Broadcast]         Propagate disproven hypotheses to full causal DAG  |
|  [Markers & Settle]  Halt on contradictory oracle run -> recalibrate    |
+-------------------------------------------------------------------------+
                                    |
                                    v
+-------------------------------------------------------------------------+
|                        TAMPER-EVIDENT EVIDENCE LEDGER                   |
|                        case.json (SHA-256 Digest)                       |
+-------------------------------------------------------------------------+
```

### A. Stage Capacity (Max 2 Competing Hypotheses)
Never overload the reasoning stage with a dozen speculative theories. Admit at most two competing, mutually exclusive explanations ($H_1$ vs $H_2$). Hold both until a discriminating experiment refutes one.

### B. Introspection Before Utterance
Before outputting any causal diagnosis or code patch:
1. Examine what thought formed internally.
2. Ask: *"Did I observe this mechanism directly, or am I inferring it from a symptom?"*
3. If inferred, tag it `[INFERRED]` and require an empirical test.

### C. Directed Focus (Goal Preservation)
During long mechanical verification loops (delta debugging over 50 iterations, binary bisecting 10 commits), agents tend to suffer context drift. Directed focus keeps the observable failure signature pinned as the immutable test predicate.

### D. Broadcast on Refutation
When an experiment yields an unexpected verdict (e.g., intervention still fails, or negative control fails unexpectedly), broadcast the refutation across all dependent causal nodes immediately. Never cling to an invalidated premise.

### E. Markers & Settle
A marker fires whenever an oracle outcome contradicts prior evidence:
- **Action**: Freeze execution.
- **Inspect**: Was the baseline flaky? Did environment state leak?
- **Settle**: Update the hypothesis status to `INCONCLUSIVE` and adjust confidence before proceeding.

---

## 2. Epistemological Claim Ladder

Every finding produced by Deep Digger must declare its exact epistemological status:

```
[OBSERVED] ------------> Raw artifact captured (trace, log, crash dump with SHA-256)
     |
     v
[REPRODUCED] ----------> Signature reproduced under controlled baseline conditions
     |
     v
[CANDIDATE] -----------> Falsifiable mechanism stated with discriminating prediction
     |
     v
[CORROBORATED] --------> Full A/B/A/C counterfactual matrix satisfied with zero contradictions
     |
     v
[INCONCLUSIVE] --------> Contradiction detected, oracle flaked, or permission denied
```

---

## 3. The Flake Rejection Protocol

A passing test run is not proof of a causal fix when the defect has non-zero intermittent flakiness.

If an incident exhibits intermittent behavior:
1. Run `deep-digger flake <case> --hypothesis <id> --oracle <cmd> --repeats 10 --allow-exec`.
2. Compute empirical failure probability:
   $$P(\text{FAIL}) = \frac{k}{N}$$
3. If $0 < P(\text{FAIL}) < 1$, calculate the sample size $N$ required to prove an intervention fixed the bug at a 99% confidence level:
   $$N \ge \frac{\ln(1 - 0.99)}{\ln(1 - P(\text{FAIL}))}$$
4. An intervention cannot be accepted until $N$ consecutive passing runs are recorded.
