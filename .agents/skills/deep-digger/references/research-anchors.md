# Method sources and limits

These are primary sources used to ground the Skill's diagnostic choices. Check their current versions when investigating contemporary tooling, and distinguish what a paper demonstrates from what the Skill itself has been tested to achieve.

1. Andreas Zeller and Ralf Hildebrandt, **Simplifying and Isolating Failure-Inducing Input**, IEEE Transactions on Software Engineering 28(2), 2002. Delta debugging and 1-minimal failing test reduction; minimization does not by itself prove the internal root cause. [Publisher](https://doi.org/10.1109/32.988498) · [University paper](https://www.st.cs.uni-saarland.de/publications/details/zeller-tse-2002/)
2. **Causal Testing: Understanding Defects' Root Causes**, ICSE 2020. Controlled causal testing and contrastive examples can help developers understand causes; its reported study results apply to the studied tasks, not all software failures. [Research paper](https://people.cs.umass.edu/~brun/pubs/pubs/Johnson20icse.pdf) · [Conference summary](https://conf.researchr.org/details/icse-2020/icse-2020-papers/86/Causal-Testing-Understanding-Defects-Root-Causes)
3. Google, **Site Reliability Engineering, Monitoring Distributed Systems**, distinction between symptoms and causes, black-box and white-box monitoring. [SRE book](https://sre.google/sre-book/monitoring-distributed-systems/)
4. OpenTelemetry, **Logging specification**, correlation by TraceId, SpanId, resource and time. [Official specification](https://opentelemetry.io/docs/specs/otel/logs/)
5. Git, **git-bisect official manual**, good/bad revisions and exit-code semantics including skip=125. [Official manual](https://git-scm.com/docs/git-bisect/)
6. Python, **subprocess management**, shell-free argument vectors, exit codes, timeouts and security considerations. [Official documentation](https://docs.python.org/3/library/subprocess.html)
7. Microsoft, **Well-Architected Incident Management**, mitigation, incident analysis, telemetry and blameless review. [Official guide](https://learn.microsoft.com/en-us/azure/well-architected/design-guides/incident-management)
8. Rust, **Cargo init and build system**, default package initialization and tests. [Cargo Book](https://doc.rust-lang.org/cargo/commands/cargo-init.html)

These sources inform the design. They do **not** establish a published benchmark of Deep Digger, automated root-cause certainty, exhaustive investigation, or safe use on any arbitrary production target.
