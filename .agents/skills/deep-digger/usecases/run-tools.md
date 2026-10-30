# Use deterministic investigation tools

The Python script is standard-library only and needs Python 3.10+. The Rust program is a separate dependency-free text-case minimizer; compile with stable Cargo. Neither is a privileged agent, a shell sandbox, or a security scanner.

## Case ledger (Python)

Run from a workspace where the user authorized a case folder. Replace `<skill-dir>` with the actual Deep Digger location and `<case-dir>` with a task-specific path. Do not run the supplied examples literally against a production system.

```bash
python3 <skill-dir>/scripts/deep_digger.py init <case-dir> --title "Wrong result" --symptom "Input X returns Y"
python3 <skill-dir>/scripts/deep_digger.py hypothesis <case-dir> --claim "Cache key collision" --test "Change only the cache key"
python3 <skill-dir>/scripts/deep_digger.py evidence <case-dir> --hypothesis H-1 --kind observation --effect supports --note "Trace matched key" --source <sanitized-evidence-file>
python3 <skill-dir>/scripts/deep_digger.py audit <case-dir>
python3 <skill-dir>/scripts/deep_digger.py next <case-dir>
python3 <skill-dir>/scripts/deep_digger.py report <case-dir> --json
```

Evidence records preserve SHA-256 of each supplied source file, not its content. File paths can still be sensitive. Sanitize before recording. `audit` checks whether file content changed since capture; absence of drift does not establish authenticity.

## Controlled repeated oracles

Only after reviewing the exact command, its side effects and environment, explicitly opt in to execution. The executable and argv are passed as a list, never through a shell. The experimental program must exit with 0 for PASS, 1 for FAIL, and any other code for UNRESOLVED. These are **defined oracle semantics**, not general shell conventions. Timeouts and missing commands are UNRESOLVED. The timeout must be finite, positive, and at most 600 seconds per command. Avoid CLI arguments that contain secrets.

```bash
python3 <skill-dir>/scripts/deep_digger.py experiment <case-dir> --hypothesis H-1 --phase baseline --repeats 3 --allow-exec -- python3 safe_reproducer.py
python3 <skill-dir>/scripts/deep_digger.py experiment <case-dir> --hypothesis H-1 --phase intervention --repeats 3 --allow-exec -- python3 safe_reproducer.py
python3 <skill-dir>/scripts/deep_digger.py experiment <case-dir> --hypothesis H-1 --phase reversal --repeats 2 --allow-exec -- python3 safe_reproducer.py
python3 <skill-dir>/scripts/deep_digger.py experiment <case-dir> --hypothesis H-1 --phase negative-control --repeats 2 --allow-exec -- python3 safe_reproducer.py
```

Each phase must actually reflect its declared environment/intervention and be independently reviewed. The CLI cannot validate that a command toggled the proposed factor; identical/misconfigured commands can falsely satisfy the shape of the gate. This is why an `experimental-pattern` automated status is only a pattern receipt, never final causal proof.

## Minimize a stable text input

The command needs a `{input}` placeholder. The script writes a candidate file in a fresh temporary directory, runs the oracle repeatedly, then writes a new output file. It refuses to overwrite the input or an existing output. Use a sandbox and a harmless oracle; untrusted oracle commands have the same authority as the caller.

```bash
python3 <skill-dir>/scripts/deep_digger.py minimize --input failing.txt --output minimal.txt --max-tests 200 --repeats 3 --allow-exec -- python3 safe_oracle.py {input}
```

`one_minimal_verified=true` means deleting any single remaining line no longer reproduces the *specified tested failure*. It does not prove the mechanism is unique. A test budget exhaustion or unresolved sample reports false, even if output is shorter.

## Rust runner

```bash
cargo test --manifest-path <skill-dir>/rust/Cargo.toml
cargo build --release --manifest-path <skill-dir>/rust/Cargo.toml
<skill-dir>/rust/target/release/deep-digger-min --input failing.txt --output minimal-rust.txt --max-tests 200 --repeats 3 --allow-exec -- python3 safe_oracle.py {input}
```

Python is the case ledger/orchestrator; Rust is the independent minimization engine. Both accept only stable oracle semantics and bounded runs. Rust uses a subprocess timeout; the user is responsible for isolation, child process trees and side effects. Neither tool autonomously invokes web research, modifies application source, or guesses a root cause.
