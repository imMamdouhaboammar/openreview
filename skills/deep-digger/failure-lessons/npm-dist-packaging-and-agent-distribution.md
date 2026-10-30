# Failure Lessons: npm Dist-Packaging & Agent Distribution

Codified engineering failure lessons and post-mortems from establishing the npm distribution pipeline for `deep-digger`.

---

## Build-Artifact Contamination in Static Security Scanners

### Context
Running the OmniSkill security scanner (`scan_skill_security.py`) over a mixed Rust/Node/Python repository after executing test suites and local release builds.

### What happened
Running `cargo test` generated hundreds of intermediate compiled object files (`.o`, `.rcgu.o`, binary test runners) under `rust/target/debug/`. Subsequent execution of `scan_skill_security.py` analyzed all 717 files and reported 636 high-severity violations (`SC9: Unreviewable executable or binary asset`), scoring 100/100 (REJECT).

### Observable symptom
```text
Verdict: [✗] REJECT
Risk Score: 100 / 100
Findings (636): SC9: Unreviewable executable or binary asset: rust/target/debug/...
```

### Impact
Blocked skill compliance verification, generated severe false positive alarms, and risked automated CI pipeline rejection.

### Incorrect assumption
Assuming static security scanners automatically skip git-ignored directories like `rust/target/` during unconditional directory walks.

### Root cause
**Status:** Confirmed  
The static scanner performs a recursive filesystem traversal without consulting `.gitignore`. Temporary binary build targets created by the compiler were parsed as potential trojaned or unreviewable executable assets.

### Why the architecture allowed it
The test execution sequence and the security audit sequence were decoupled without an automated pre-scan cleanup requirement.

### Fix
Enforced an automated pre-scan `cargo clean --manifest-path rust/Cargo.toml` step, added comprehensive `.npmignore` exclusions, and restricted distribution bundling exclusively to `dist/`.

### Verification
Post-clean scan verified 53 clean source files, producing `Verdict: [✓] APPROVE` with `Risk Score: 0 / 100`.

### Prevention rule
Never execute repository static security audits or distribution packaging without first executing compiler cleanup (`cargo clean` or equivalent artifact purges).

### Reusable lesson
Local build outputs must be treated as ephemeral and strictly isolated from static code analysis and packaging pipelines.

### Related code
- `rust/Cargo.toml`
- `scripts/build.js`
- `.npmignore`

### Related tests
- `scan_skill_security.py`
- `quick_validate.py`

### Related lessons
None.

### Status
Resolved.

---

## Documentation Script Piping & Agent Path Snooping Violations

### Context
Authoring user-facing installation instructions in `README.md` for a multi-agent CLI and skill.

### What happened
Providing convenient installation commands using `curl` piped directly into `bash` and mentioning literal agent configuration paths in markdown documentation triggered `SC2: Remote Script Piping` (Critical) and `AS1: Agent Config Snooping` (High).

### Observable symptom
```text
[CRITICAL] SC2: Remote Script Piping (README.md: remote script piping to shell)
[HIGH] AS1: Agent Config Snooping (README.md: agent configuration directory access)
```

### Impact
Documentation triggered supply chain security rejections during automated repository audits.

### Incorrect assumption
Assuming security scanners restrict heuristic checks to code files and ignore instructions within markdown documentation.

### Root cause
**Status:** Confirmed  
Autonomous agent safety analyzers inspect documentation as potential execution prompts. Piped curl shell commands and direct agent internal configuration paths are flagged as supply chain attack surfaces.

### Why the architecture allowed it
Documentation was written using traditional Unix shell scripting conventions rather than registered package manager invocations.

### Fix
Replaced raw curl piping with standard package manager commands (`npm install -g deep-digger`, `bun add -g deep-digger`, `npx skills add imMamdouhaboammar/deep-digger`) and replaced literal agent configuration directories with descriptive platform roles.

### Verification
Re-running `scan_skill_security.py` verified 0 security findings.

### Prevention rule
Never document `curl | sh` or literal private agent configuration paths in public markdown; always document official registry installations and semantic platform labels.

### Reusable lesson
Autonomous agents treat markdown documentation as prompt context; documentation must respect the same operational security boundaries as application code.

### Related code
- `README.md`
- `package.json`

### Related tests
- `scan_skill_security.py`

### Related lessons
Build-Artifact Contamination in Static Security Scanners.

### Status
Resolved.

---

## Asynchronous npm Ingestion Queue vs. Premature Republish Conflicts

### Context
Publishing a new package release (`deep-digger@1.1.0`) to the npm public registry.

### What happened
Executing `npm publish --access public` returned `HTTP 202 Accepted` with the notice: *"Your package is being processed and may take a few minutes to become available."* A subsequent republish attempt while processing was underway failed with `HTTP 409 Conflict: Cannot publish over previously staged version "1.1.0"`.

### Observable symptom
```text
npm error code E409
npm error 409 Conflict - PUT https://registry.npmjs.org/deep-digger - Cannot publish over previously staged version "1.1.0".
```

### Impact
Risk of premature developer panic or attempting broken emergency versions under the false belief that publication failed.

### Incorrect assumption
Assuming npm registry publication is immediately synchronous across read replicas and CDN edge caches.

### Root cause
**Status:** Confirmed  
Modern npm architecture processes uploaded package tarballs through an asynchronous verification pipeline. During the staging window (60–180 seconds), the upload is stored in a staging queue (`202 Accepted`). Retrying the publish during this window triggers a 409 conflict, even while public read queries still return the previous version or 404.

### Why the architecture allowed it
Tooling assumed synchronous read-after-write consistency from an eventually consistent distributed service.

### Fix
Handled the staging window by waiting for backend processing, verifying package metadata via `npm view` with retries, and updating `dist-tags` explicitly once staged.

### Verification
Verified public package availability (`deep-digger@1.1.0`), confirmed `latest` tag pointing to `1.1.0`, and verified remote installation via `bunx deep-digger version`.

### Prevention rule
An npm `HTTP 202 Accepted` response signifies successful receipt into the ingestion queue. Allow 60–120 seconds for worker propagation before querying status or modifying tags; never attempt to republish over a staged version.

### Reusable lesson
Distributed registries decouple upload acceptance from public read availability.

### Related code
- `package.json`
- `scripts/build.js`

### Related tests
- `npm view deep-digger`
- `npx deep-digger version`

### Related lessons
None.

### Status
Resolved.

---

## Source-Code Leakage in Hybrid Multi-Language npm Packages

### Context
Distributing a multi-language agent skill (Node CLI wrapper, Python core engine, Rust delta minimizer) as a public npm package.

### What happened
Listing repository directories directly in `package.json` `files` threatened to package development test suites (`evals/`), test cases (`cases/`), CI workflows (`.github/`), and internal fixtures into the npm tarball (>570 kB).

### Observable symptom
High tarball size, potential leakage of internal test fixtures, and violation of the requirement to distribute only standalone build artifacts (`dist/`).

### Impact
Unnecessary package bloat, exposure of internal test code, and violation of production unslop delivery standards.

### Incorrect assumption
Assuming npm packages should mirror the repository source tree rather than a dedicated distribution directory.

### Root cause
**Status:** Confirmed  
Absence of a dedicated distribution build step (`scripts/build.js`) to assemble an isolated runtime bundle (`dist/`).

### Why the architecture allowed it
`package.json` had `"main": "bin/cli.js"` pointing directly into development directories rather than an artifact directory.

### Fix
1. Implemented `scripts/build.js` to assemble a pristine `dist/` directory containing executable scripts, Python core, and clean Rust source.
2. Configured `package.json` `"files": ["dist", "SKILL.md", "marketplace.json", ".skills.json", "LICENSE", "README.md"]`.
3. Added `"prepublishOnly": "node ./scripts/build.js"`.
4. Configured `.npmignore` to exclude development and build directories.

### Verification
`npm pack --dry-run` demonstrated the tarball dropped to 44.9 kB (23 total files), containing zero test files or dev commentary.

### Prevention rule
All hybrid multi-language packages must publish exclusively from an isolated `dist/` bundle generated by a deterministic build script.

### Reusable lesson
Maintain strict physical separation between repository developer workspaces and production distribution bundles.

### Related code
- `scripts/build.js`
- `package.json`
- `.npmignore`
- `bin/cli.js`
- `bin/postinstall.js`

### Related tests
- `npm pack --dry-run`
- `bun run build`

### Related lessons
None.

### Status
Resolved.
