# Investigate a deep failure

## Case startup

1. Pin the requested target. For a repository, inspect local instructions, exact failing commands, relevant branches and worktree status. For an external service, specify caller-visible boundary and authorization. Never probe an unrelated host.
2. Write a testable phenotype: `Given input X in environment E, operation Y yields actual A instead of expected B, with fingerprint F`.
3. Capture what changed: code, configuration, dependency resolution, secrets policy, data, infra, traffic, time, platform, feature flags and experiment harness. Not all changes are causal.
4. Check the oracle before using it: repeatedly run a known failing case and a passing neighbor with the same environment. If mixed, pivot to flaky analysis and do not use naive binary search or ddmin.
5. Preserve existing user changes. Prefer dedicated temporary tests or a separate worktree instead of rewriting test assertions.

## The D.I.G.G.E.R. cycle

- **D**efine a precise observable failure and target authority.
- **I**nventory versioned evidence, alternative hypotheses, and unknowns.
- **G**enerate discriminating predictions before running tests.
- **G**ather controlled evidence through the cheapest safe experiment.
- **E**liminate explanations through direct refutation and reversible interventions.
- **R**echeck the causal chain and regression, or report an unresolved boundary.

Keep a matrix:

| Hypothesis | Unique prediction | Safe experiment | Result | Survives? |
| --- | --- | --- | --- | --- |
| H1 | Behavior fails only with a flag enabled | Toggle isolated flag | Unrun / PASS / FAIL / UNRESOLVED | Open |
| H2 | Behavior follows payload field size | Shrink payload, preserve signature | Unrun / PASS / FAIL / UNRESOLVED | Open |

The model must fill this matrix from case-specific evidence, not assume the example hypotheses are valid.

## Choose experiments by discrimination

When two hypotheses make the same prediction, do not run that experiment simply because it is easy. Seek a case where their predictions diverge. Avoid changing unrelated variables together. Capture negative controls and matched controls for input, dependencies, clocks, load, concurrency, resource limits and rollout state. Distinguish failure to observe a defect from proof that it cannot occur.

- **Stable code regression:** Compare known good/bad revisions after checking reproducibility. `git bisect` alters checkout state; use an isolated worktree, confirm clean state, and follow Git's exit-code contract. An introduced commit may contain multiple changes and is not automatically a unique root cause.
- **Flaky and concurrency:** Capture run count, failure frequency and variance, scheduler/seed/environment, event order and logs; reproduce with controlled seeds, timeouts or deterministic scheduler. Avoid merging failures with different signatures.
- **Distributed failures:** Correlate trace IDs and span timing with logs and metrics across services. Trace the *first violation of an invariant* upstream, including retries, timeouts and queueing. Distinguish causal trigger from underlying systemic weakness.
- **Performance:** Compare distributions and resource profiles with matched loads. Separate CPU, I/O, locking, allocations, serialization and network. Use A/B/A with a cache warmup plan and variance bounds.
- **Black box:** Use boundary-controlled metamorphic tests and differential requests. Report only observed I/O behavior when internals are inaccessible.
- **Security incident:** Stabilize containment and evidence chain before exploratory reproduction; sanitize and isolate before any experiment. Refer to specialized security guidance.

## Confirmation and stopping

Treat a mitigation (e.g., retry or scaling up) as a risk-reducing operational action, not evidence of why failure arose. If a patch is authorized, add a regression test that fails under the original mechanism, fix the implementation, and rerun adjacent tests and relevant checks without weakening assertions.

The Skill must stop when: authorization does not cover the test; the only remaining experiment has unacceptable side effects; no stable oracle can be obtained with available access; competing hypotheses remain observationally indistinguishable; or the defined budget is exhausted. Preserve the strongest explanation, falsifiers, and explicit next prerequisite rather than declaring the task impossible or solved.

## Final RCA structure

1. **Failure and blast radius:** precise signature, impact and tested environment.
2. **Causal chain:** observed trigger, proximate mechanism, upstream contributing condition; multiple causes if warranted.
3. **Evidence:** IDs, timestamps, digests, commands or trace references, controlled predictions and results.
4. **Alternatives:** what was ruled out and how; which competing explanations remain.
5. **Fix and prevention:** tested correction if authorized; safe mitigations; recurrence guard and detection gaps.
6. **Confidence boundary:** experimentally supported / candidate / unresolved, where it was tested and where it was not.
