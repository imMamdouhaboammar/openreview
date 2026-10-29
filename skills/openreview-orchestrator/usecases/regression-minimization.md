# Use Case: Regression Minimization with Deep-Digger

Workflow for isolating subtle regressions and delta-debugging reproducing inputs.

---

## 1. When Activated
- Activated when a test fails unpredictably on a PR, or when a massive input payload triggers an unhandled crash.

---

## 2. Protocol

1. **Establish Oracle:**
   Create an executable test script `oracle.sh`:
   ```bash
   #!/usr/bin/env bash
   # Returns 1 (FAIL) if target bug reproduces, 0 (PASS) if normal
   cargo test --test integration_suite -- failing_test_case "$1"
   if [ $? -eq 101 ]; then
     exit 1
   fi
   exit 0
   ```

2. **Execute Delta Minimization:**
   Run the high-speed Rust `ddmin` minimizer:
   ```bash
   deep-digger-min \
     --input ./test_cases/large_failing_payload.json \
     --output ./test_cases/minimal_failing_payload.json \
     --allow-exec \
     --repeats 3 \
     -- ./oracle.sh {input}
   ```

3. **Verify Minimality:**
   Confirm that the resulting file is 1-minimal (removing any line causes the oracle to exit 0).

4. **Handoff to Fix Engine:**
   Use the minimized reproducer as the regression test in OpenReview's Fix workflow.
