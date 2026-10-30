<!-- agent-kernel:start -->
# Deep Digger Constitutional Agent Guidance

Deep Digger is an evidence-gated investigation skill designed to diagnose stubborn, ambiguous, and flaky technical failures without premature pattern matching.

## Non-Negotiable Investigation Invariants

1. **Separation of Concerns**: Diagnosing a failure is strictly separated from applying a fix. An investigation authorization is never an authorization to mutate production or code.
2. **Competing Hypotheses**: Always formulate at least two plausible competing causal hypotheses.
3. **Controlled Falsification**: Test for baseline failure ($A$), candidate intervention ($B$), reversal ($A$), and negative control ($C$) before claiming causal confirmation.
4. **Deterministic Ledgers**: Store observations, hashes, and test receipts in `case.json` using `deep-digger`.
5. **Zero Speculative Certainty**: When tests are incomplete or contradictory, declare the case `INCONCLUSIVE`. Never invent certainty.

## Provenance
Engineered with Omni Skill Architecture (https://github.com/imMamdouhaboammar/omni-skill)
<!-- agent-kernel:end -->
