# Specification validation and adoption

## Package-level checks

After extracting the ZIP:

```bash
find openreview-full-spec-2026-10-29 -type f | sort
find openreview-full-spec-2026-10-29 -name '*.json' -print -exec jq -e . {} \;
unzip -t OpenReview-Full-Agentic-Spec-2026-10-29.zip
```

For real JSON Schema conformance use a draft 2020-12 validator on each `CONTRACTS/*.schema.json` and validate `EXAMPLES/*` against its matching schema. For `EXECUTION_BRIEF.json` use the `get-things-done` Execution Brief v1 schema. Do not treat `jq` syntax validation alone as proof of schema adherence.

## Dependency handoff

- First owner decision: access to complete Qlty issue details and two safe App test installations. If unavailable, `M0-01` continues only read-only research/test prep.
- A PR merge instruction in this historical conversation is not permission to bypass a currently failing required gate.
- After M0, run M1 tasks with real consumer repo and operator smoke evidence, then M2-M6 by `WORK_CARDS.md` dependency graph.
- As cards finish, update `TRACEABILITY.csv` with PR links and `TEST_AND_EVIDENCE_MATRIX.md` with observed URLs, not unsupported checkmarks.

## Definition of a complete execution handoff

```text
card_id:
status: ready | in_progress | blocked | verified | merged
base_sha:
head_sha:
files_changed:
red_test_evidence:
green_test_evidence:
ci_urls:
security_review:
pending_review_threads:
provider_key_provenance: configured secret name only (no key material)
rollback_plan:
next_card_or_blocker:
```

## Suggested skill selection, not ritual use

- **Fable Plan / Superpowers writing-plans / Get Things Done:** dependencies, atomic steps and handoffs.
- **Anti-slop Engineering / Ponytail:** evidence-first, narrow changes, avoid ornamental architecture and unnecessary dependencies.
- **Review / CodeRabbit / Qodo:** independent review feedback if actually accessible; a model review never substitutes for factual tests or official security evidence.
- **GitHub / Context7 / parallel-search:** verify current repo status and platform documentation; citations must match exact commands/versions.
- **Skill Craft:** only if a card explicitly requires authoring or changing skills; a specification does not itself install a skill.

The ZIP is a plan. It does not install an App, add secrets, modify the OpenReview repository, merge PRs or promise that future agent execution will be successful without infrastructure and required approvals.
