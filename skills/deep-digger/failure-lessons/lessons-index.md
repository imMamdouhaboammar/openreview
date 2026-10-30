# Master Lessons Index

This index catalogs all codified failure lessons and architectural invariants established in `deep-digger`.

---

## Catalog of Lessons

| # | Lesson Title | Domain | Failure Class | Status | Document |
|---|---|---|---|---|---|
| 1 | Build-Artifact Contamination in Static Security Scanners | Build & Packaging | False Positive / Supply Chain | Resolved | [npm-dist-packaging-and-agent-distribution.md#build-artifact-contamination-in-static-security-scanners](npm-dist-packaging-and-agent-distribution.md#build-artifact-contamination-in-static-security-scanners) |
| 2 | Documentation Script Piping & Agent Path Snooping Violations | Documentation & Policy | Supply Chain Policy | Resolved | [npm-dist-packaging-and-agent-distribution.md#documentation-script-piping--agent-path-snooping-violations](npm-dist-packaging-and-agent-distribution.md#documentation-script-piping--agent-path-snooping-violations) |
| 3 | Asynchronous npm Ingestion Queue vs. Premature Republish Conflicts | Release & CI/CD | Distributed Registry Race | Resolved | [npm-dist-packaging-and-agent-distribution.md#asynchronous-npm-ingestion-queue-vs-premature-republish-conflicts](npm-dist-packaging-and-agent-distribution.md#asynchronous-npm-ingestion-queue-vs-premature-republish-conflicts) |
| 4 | Source-Code Leakage in Hybrid Multi-Language npm Packages | Packaging & Distribution | Bundle Bloat / Dev Leakage | Resolved | [npm-dist-packaging-and-agent-distribution.md#source-code-leakage-in-hybrid-multi-language-npm-packages](npm-dist-packaging-and-agent-distribution.md#source-code-leakage-in-hybrid-multi-language-npm-packages) |

---

## Rules We Now Enforce

1. **Pre-Audit Compiler Cleanup:** Always execute `cargo clean` and purge local build caches before running security audits (`scan_skill_security.py`) or package bundling.
2. **Deterministic Dist Isolation:** npm packages distributing hybrid runtime tools must assemble an isolated `dist/` bundle via `scripts/build.js`, restricting package manifests to `dist/` and top-level manifests.
3. **No Curl Piping in Documentation:** Public documentation for tools and agent skills must recommend official registry commands (`npm`, `bun`, `npx skills add`) and avoid `curl | sh` or literal agent paths.
4. **Registry Staging Patience:** Respect npm `HTTP 202 Accepted` responses as successful staging entries; allow 60–120 seconds before checking `npm view` or updating tags, and never retry publishing over a staged version.
