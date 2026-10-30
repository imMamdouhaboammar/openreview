# Contributing to Deep Digger

Thank you for contributing to Deep Digger! We welcome bug reports, improvements to causal diagnostics, delta debugging optimizations, and documentation refinements.

## Development Workflow

1. Fork and clone the repository:
   ```bash
   git clone https://github.com/imMamdouhaboammar/deep-digger.git
   cd deep-digger
   ```

2. Run system doctor & verify environment:
   ```bash
   bun bin/cli.js doctor
   ```

3. Run the deterministic test suites:
   ```bash
   # Python Core tests
   python3 -m unittest discover -s evals -p 'test_cli.py' -v

   # Rust Minimizer tests
   cargo test --manifest-path rust/Cargo.toml
   ```

4. Submitting Pull Requests:
   - Ensure all tests pass.
   - Adhere to the strict causal invariants in `SKILL.md`.
   - Never commit sensitive logs, credentials, or mock customer data.
