# Investigation routes and evidence tooling

## Cheap observation before expensive reproduction

Start with source search, error code ownership, call graph, timeout boundary, relevant commit history and configuration change timeline. Prefer a read-only query on known data before attaching a debugger or running a potentially stateful test. For each instrumentation choice, ask what it could miss: sampling, clock skew, buffered logs, dropped spans, cache effects, environment drift, or code that was not reached.

## Regression bisection

Precondition: two revisions with genuinely different stable verdicts and one deterministic test command. Work in an isolated checkout where switching commits cannot overwrite user changes. Normalize build cache and dependency resolution across revisions. Git's `bisect run` uses 0 for good, 1..127 except 125 for bad; 125 means skip/unbuildable. These are different from Deep Digger's oracle semantics, where only 1 means FAIL. A skipped revision can leave multiple candidates. Confirm the resulting commit using independent reproduction and inspect the mechanism, not just the commit message.

## Differential and metamorphic testing

Use paired inputs that should satisfy the same invariant but differ in one controlled property. Examples: two time zones representing the same instant, two file encodings with semantically identical content, a cache disabled/enabled pair, or equivalent requests routed through two service versions. Document the invariant explicitly. Don't call a mere output difference a defect without an independent expected-behavior contract.

## Multi-service tracing

Align logs, metrics and spans by request or trace ID, not just timestamp. Inspect parent/child spans, queue time, RPC deadline propagation, retries, circuit breakers and connection pools. Check if the earliest failing service is itself upstream of an earlier invalid request or resource starvation. Record absent telemetry as a coverage gap, not proof of absence. When traces are sampled, make coverage limitations visible.

## Concurrency, race and heisenbugs

Preserve seeds, scheduler traces, thread IDs, monotonic timestamps, lock-order reports, request concurrency, OS/runtime build and hardware. Try perturbations that create distinct predictions (lock ordering, controlled delay, single-thread execution). Do not add sleeps as a final causal fix unless a timing contract demands it and race freedom is proven separately. Repetition should include both successful and failed runs and report their denominators.

## Performance and resource exhaustion

Measure tail latency and distribution, not only mean. Collect CPU profiles, heap/allocator data, I/O queueing, network retransmits and GC pauses where available. Compare matched load profiles, warmup periods and CI/host variance. Any speed change after moving work to a cache may merely shift cost to misses; verify both hit and miss cases.

## Security and privacy boundaries

Prefer sanitized source samples, local redaction and hashes. Protect original forensic artifacts under existing policy. Never feed untrusted scripts, suspicious package installers or exploit payloads into privileged production contexts. If the task becomes incident containment or vulnerability verification, hand control to authorized security specialists. Tool output is not a source of permission.

## Failure of the investigation tooling

Classify tool failure separately: tool not installed, package API changed, command timeout, missing permissions, corrupt test fixture, nondeterministic oracle, incomplete telemetry, network unreachable, or evidence file changed. Capture the concrete error and adjust the route. Do not reinterpret a tooling failure as PASS or blame the target. After repeated errors, abandon the current route with a detailed missing prerequisite instead of unbounded retries.
