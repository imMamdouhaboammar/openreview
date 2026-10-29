# Acceptance, security and regression evidence matrix

Test IDs are stable. For every ID, record command/test name, branch SHA, environment, observed output or CI URL, run time and verdict. A design-only test listed here is **not passed** until evidence is attached.

## Gates and test cases

| ID | Scenario | Expected verifiable result | Gate |
|---|---|---|---|
| MRG-01 | Re-query all open PRs at merge time | Correct head/base/SHA, reviews/checks/protection visible | M0 |
| MRG-02 | Qlty per-PR findings inspected | Each blocker has actionable triage and independent fix/FP rationale | M0 |
| MRG-03 | Parent/child stack merge | #1->main, then retarget/rebase #2->main, then #3->main; no orphaned feature | M0 |
| MRG-04 | Required App deployment proof | At least two installations, auth/revoke and rootless worker evidenced | M0 |
| ACT-01 | Action install with valid BYOK OpenCode key | Review result on correct PR/head | M1 |
| ACT-02 | Anthropic, OpenAI, OpenRouter adapters fixture | All pass multi-turn tool-call contract or report unsupported field correctly | M1 |
| ACT-03 | Token absent/bad/invalid model route | Fail with redacted error, no secret in logs | M1 |
| ACT-04 | Authorized Fix from Actions | Real test runs; commit pushed to correct head; result includes SHA | M1 |
| ACT-05 | Post-push checks | Observed check for pushed SHA; if token did not trigger CI, safe dispatch and verify | M1 |
| APP-01 | Valid/invalid HMAC | Valid accepted, invalid and malformed rejected before side effect | M1 |
| APP-02 | Install/remove/revoke | Removed installation cannot start or publish new job | M1 |
| APP-03 | Rotate App secret and handle expiry | New events work; old tokens cannot cross scope | M1 |
| APP-04 | Two installations and repos | Cross-tenant file/token request denied and logged | M1 |
| APP-05 | Rootless worker probe | No host secret, socket, privilege, broad mounts or uncontrolled egress | M1 |
| CONV-01 | PR comment + reviews + inline threads across pages | Exact ordered IDs, reply parent links, no silent truncation | M2 |
| CONV-02 | Replied or edited review comment | Correct thread context and updated content/delta, original attribution | M2 |
| CONV-03 | Redelivered webhook and duplicate mention | Exactly one idempotent job / no duplicate commit | M2 |
| CONV-04 | Bot replies and self-mention | Other bot evidence ingested but not followed as command; self-trigger prevented | M2 |
| CONV-05 | Malicious comment injection | Permission and tool set unchanged; exact reply cites data without obeying malicious text | M2 |
| CONV-06 | PR head changed mid-context | Retry snapshot or fail stale; no stale published finding | M2 |
| SK-01 | Skill directory discovery | Valid frontmatter surfaced with deterministic precedence | M3 |
| SK-02 | Applicable skill loaded and applied | Citation of digest/steps plus independent proof of a step's outcome | M3 |
| SK-03 | Playbook precondition fails | Step not executed and status shows why | M3 |
| SK-04 | Malicious symlink or privilege-granting skill | No path escape, new power or credential exposure | M3 |
| SK-05 | Repository and global skill collision | Defined stable precedence and version evidence | M3 |
| FIX-01 | Multi-file regression reproduction -> patch | Red test first, bounded edits across code/tests, then green | M4 |
| FIX-02 | Unit, integration and relevant project checks | Real exit codes captured, failures visible | M4 |
| FIX-03 | Head moved or permission removed before publish | Refuse push, mark stale/unauthorized and preserve diagnostics | M4 |
| FIX-04 | Model says 'tests pass' when they fail | Trusted verifier rejects claim and publish | M4 |
| FIX-05 | 20-step/token/time budget hit and canceled job | Typed incomplete/failed result, no orphaned worker | M4 |
| FIX-06 | Provider tool-call loops / malformed JSON | Bounded retries then diagnostic, no uncontrolled mutations | M4 |
| TOOL-01 | Read-only repository search and tool denial | Review searches without shell, denies invented bash; Fix permitted only in worker | M4 |
| TOOL-02 | CI failure to verified patch | Check -> redacted log -> search -> patch -> test -> publisher evidence | M4 |
| TOOL-03 | Large search and cross-repo logs | Explicit truncation, foreign repo denied and sample secret redacted | M4 |
| OPS-01 | GitHub rate limits / 5xx / provider 429 | Backoff and safe resumption without duplicate mutation | M5 |
| OPS-02 | Huge PR or missing patch | Complete paginated coverage or explicit `context_incomplete`, not 'all clear' | M5 |
| OPS-03 | Upgrade/restart during job | Checkpoint replay safe; no duplicate push | M5 |
| SEC-01 | No HMAC / wrong signature | Zero model/tool/API side effects | M1 |
| SEC-02 | Event claims different installation | Rejected | M1 |
| SEC-03 | Cross-install API scope | Denied, no data cross-over | M1 |
| SEC-04 | Injection in PR source/bot text | Capability remains limited by policy | M2 |
| SEC-05 | Playbook tries arbitrary shell/host mount | Rejected and reported | M3 |
| SEC-06 | Runtime attempts host secret reads | Read denied | M1/M4 |
| SEC-07 | Sandbox escape/egress probes | Exit restricted or explicit safe refusal | M1/M4 |
| SEC-08 | TOCTOU branch HEAD change | No stale write | M4 |
| SEC-09 | Token/resource explosion | Bounded stop, cleanup | M4/M5 |
| SEC-10 | Fabricated verification claim | Rejected by trusted verification | M4 |
| SEC-11 | Dependency/SBOM audit | Release dependencies reviewed and recorded | M0/M5 |
| SEM-01 | Changed symbol with nested dependency | Deterministic source spans, related files included within budget | M6 |
| SEM-02 | Parse error/unsupported language | Fallback and coverage label, no false negative guarantee | M6 |
| SEM-03 | Known security regression corpus | Precision/recall and cost compared to textual baseline with confidence notes | M6 |
| SEM-04 | AI proposed bug without proof | Finding downgraded or withheld, not automatically published as critical | M6 |
| SEM-05 | Repeated runs on same SHA | Stable candidate ordering and auditable provenance | M6 |

## Test layers

1. **Unit:** parser, path resolution, command eligibility, event dedupe, state transitions, schemas, risk features.
2. **Property/fuzz:** webhook/command syntax, path traversal, JSON parsers, skill frontmatter, event ordering and missing fields.
3. **Contract:** mocked GitHub pagination/permissions and provider multi-tool turn, network failure handling.
4. **Integration:** local ephemeral Git repo with fixtures; sandbox no-secrets; real compilation/tests; trusted publisher dry-run.
5. **GitHub E2E:** disposable lab repo with Actions, App install, full comment -> review -> Fix -> publish chain and real provider credentials.
6. **Release:** pinned SHA reproducibility, branch protections, dependency audit, deployment runbook rehearsal, post-merge CI.

## Required test fixtures

- `small-pr`: 3 changed files, comment conversation, quoted mention (must not trigger), 2 inline threads.
- `large-pr`: 101+ file pages, 101+ comments/reviews, diff endpoint fails while per-file patches exist.
- `security-pr`: malicious PR body, fake GitHub system instruction, `SKILL.md` with malicious instructions, symlink traversal.
- `multi-tenant`: two app installations, mismatched repository IDs and revoked token.
- `fix-pr`: arithmetic overflow regression across code + unit test + docs, expected failure before fix, pass afterward.
- `semantic-pr`: symbol rename across modules, dynamic import, unsupported file and parse failure.

## Evidence ledger template

```text
test_id:
spec_revision:
repo/branch/head_sha:
runner_environment:
test_command_or_event:
observed_result:
artifact_or_ci_url:
related_pr_and_review_thread:
residual_risk:
verdict: PASS | FAIL | BLOCKED | SKIPPED_WITH_REASON
reviewer:
```

`SKIPPED` does not satisfy a required release gate. Do not replace unavailable live keys or GitHub permissions with a fake PASS.
