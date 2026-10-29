# Causal Investigation & Minimization with Deep-Digger

When a pull request introduces a regression, flakiness, or test crash that cannot be diagnosed from inspection alone, the orchestrator invokes the `deep-digger` subsystem.

---

## 1. When to Invoke Deep-Digger

- **Regression in Existing Tests:** A PR commit breaks tests that previously passed.
- **Flaky or Complex Input:** A failure triggers only with large payloads, large JSON inputs, or specific configurations.
- **Competing Explanations:** Multiple hypotheses exist for why an error occurred and observational evidence is needed to eliminate false theories.

---

## 2. Investigation Protocol

```text
Step 1: Ledger Initialization
        python3 skills/deep-digger/scripts/deep_digger.py init <case-dir>

Step 2: Formulate Falsifiable Hypothesis
        python3 skills/deep-digger/scripts/deep_digger.py hypothesis <case-dir> \
          --id H-1 --statement "Buffer overflow occurs when input length exceeds 1024 bytes without newline"

Step 3: Define Deterministic Oracle
        An executable test script that returns:
        - 0 (PASS): Failure did not reproduce.
        - 1 (FAIL): Specific target failure reproduced deterministically.
        - Other (UNRESOLVED): Environment issue, timeout, or unrelated error.

Step 4: Execute Input Minimization (Rust ddmin)
        deep-digger-min --input repro_large.txt --output repro_minimal.txt --allow-exec -- ./test_oracle.sh {input}

Step 5: Record Experimental Evidence
        python3 skills/deep-digger/scripts/deep_digger.py experiment <case-dir> \
          --hypothesis H-1 --phase baseline --repeats 3 --allow-exec -- ./test_oracle.sh repro_minimal.txt

Step 6: Handoff to Fix Pipeline
        Pass the 1-minimal test case into OpenReview Fix mode as the targeted regression test.
```

---

## 3. Guarantees of 1-Minimality

The Rust minimizer (`deep-digger-min`) implements Zeller's delta debugging (`ddmin`) algorithm:
- Splitting inputs into recursive granular chunks.
- Testing single chunk removal and complement subsets.
- Verifying stability across repeat runs to eliminate flaky classifications.
- Guaranteeing that removing any single remaining line or chunk causes the failure to disappear (1-minimal).
