# Deep Digger installation and verification

`skills/deep-digger/` is the source package for the Deep Digger skill. Merely
including it in an OpenReview checkout does **not** make it automatically
discoverable by the OpenReview agent runtime.

To activate Deep Digger for a repository analyzed by OpenReview, install the
complete package under one of the repository skill roots that OpenReview reads:

- `<target-repo>/.agents/skills/deep-digger/`
- `<target-repo>/.skills/deep-digger/`

The package must retain `SKILL.md`, `agents/`, `usecases/`, `references/`,
`scripts/`, and the Rust crate. Permissions for execution and external work
still follow the host's policy. Installing the skill does not authorize its
opt-in oracle commands, and the tools are not a sandbox.

Run the deterministic checks from the OpenReview repository root:

```bash
python3 -m unittest discover -s skills/deep-digger/evals -p 'test_cli.py' -v
cargo fmt --manifest-path skills/deep-digger/rust/Cargo.toml --check
cargo clippy --manifest-path skills/deep-digger/rust/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path skills/deep-digger/rust/Cargo.toml --all-targets --locked
python3 scripts/scan_skill_security.py skills/deep-digger
```

The scenarios in `evals/scenarios.json` describe intended agent decisions;
running the commands above does not execute those behavioral evaluations.
