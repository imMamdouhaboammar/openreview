# Causal protocol: from correlated symptoms to bounded explanation

This is a decision framework, not a guarantee of philosophical certainty. A software failure may have several necessary and sufficient conditions, a trigger, a code defect, an environmental contributor and an organizational enabling condition. Searching for a single "ultimate" cause can obscure actionable causal chains.

## Counterfactual question

For proposed cause C and observed failure F, ask: **holding relevant conditions as constant as practical, would F still occur if only C were changed?** This question is stronger than "did C change shortly before F?" but weaker than metaphysical proof. Use interventions and triangulation to approach it.

1. **A: failing baseline**. Produce the same failure signature in the environment under investigation on at least two independent runs. Record fingerprint, timestamps, versions and input. A passing run amidst failures means this is a noisy oracle; do not treat it as stable A.
2. **B: directed intervention**. Change one hypothesized factor, not a bundle. Preserve all feasible other variables. Produce repeatable passing results with comparable instrumentation. Record whether the behavior is *actually* the same path with only this factor changed.
3. **A: reversal**. Reintroduce the factor and observe the original failure again if doing so is safe and reversible. For privacy, availability or payment incidents, don't risk production to satisfy a protocol: use isolated staging and mark unavailable gates explicitly.
4. **C: negative control**. Apply a comparable but mechanistically irrelevant change. The failure should persist. If both the candidate intervention and unrelated changes suppress the failure, investigate environmental/timing confounding.
5. **Mechanism trace**. Connect input, branching behavior or violated invariant to the observed failure through source/trace/profile/state evidence. A successful workaround may bypass a path rather than correct its underlying defect.
6. **Alternative elimination**. Try to distinguish remaining explanations with independent predictions. Avoid cherry-picking a failing test that every candidate predicts equally.

## What changes the ranking of explanations?

- A result is discriminating when one candidate predicts FAIL and the other predicts PASS under the same test. A test confirming what all candidates predicted contributes little.
- A repeated result from the *same hidden state* is not independent replication. Rotate seeds or restore known snapshots when necessary, while preserving comparability.
- Negative evidence is meaningful only if the measurement would have detected the hypothesized effect. "No log line" is uninformative when sampling or instrumentation can drop the event.
- Changing several dependencies together is a *confounded intervention*. It may point to a region, but does not isolate a cause.
- Evidence from two tools may derive from one underlying event (e.g. log scraper and dashboard). Treat correlated measurements as one observation family.
- A minimal failing input is 1-minimal relative to the selected partition and oracle. It need not be globally minimal, unique or sufficient to prove the internal mechanism.
- If the symptom disappears only under high instrumentation, investigate observer effect, race conditions and sampling bias.
- After fixing, add a regression that would have failed for the original mechanism, and inspect adjacent invariants to avoid a test overfitted to one sample.

## Classification when the oracle is unreliable

| Evidence situation | Classification | Appropriate next experiment |
| --- | --- | --- |
| Stable repeated signature | REPRODUCED | Counterfactual experiment or ddmin |
| Mixed PASS/FAIL on same declared baseline | FLAKY | Capture seeds, timing, interleavings, load and rate; design a repeatable oracle |
| Exit code outside declared PASS/FAIL | UNRESOLVED | Diagnose harness/environment without modifying application assertions |
| Two interventions fix it | MULTI-CAUSE or CONFOUNDED | Try one-factor toggles and a negative control |
| Rollback seems to fix, reapply not consistently failing | INCONCLUSIVE | Reproduce under equivalent data and load; test confounders |
| Cannot observe hidden mechanism | BOUNDARY-ONLY | Report request/response invariant and needed access |

## Root chain vs contributing factors

Write a causal graph with named links: `trigger -> violated assumption -> faulty implementation behavior -> user-visible symptom`. Add environmental conditions that make this chain possible and control failures that allowed it to persist. Document observed edges separately from inferred edges. Several causes may be sufficient independently. Do not choose a single file/owner simply to make a report short.

## Bounded closure

Use the words **experimentally supported within environment E and input class I** only after observing controlled A/B/A, a negative control, mechanism evidence, and meaningful treatment of alternatives. Never generalize across untested versions, load ranges, production geography or adversarial inputs. If this level is unavailable, return the strongest candidate and an exact discriminator for the next investigation round.
