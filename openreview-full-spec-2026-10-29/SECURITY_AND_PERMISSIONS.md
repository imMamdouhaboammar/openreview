# Security requirements, threat model and policy gates

## Core invariants (non-negotiable)

S-01. Invalid/missing GitHub webhook HMAC SHA-256: no job creation, model call or repository effect. Verify original body bytes before parsing untrusted fields.  
S-02. Every job bound to an installation, repository ID, PR number, event ID, exact head SHA, triggering actor and permitted command. Never trust string-only repo names as tenant identity.  
S-03. Review and Suggest cannot execute bash, write files or push, even if LLM invents a tool call. Check independently at tool dispatch, not only tool advertisement.  
S-04. Fix authorization from live GitHub permissions and repo policy, checked both at admission and immediately before publishing. Fork and protected-branch policies deny unsafe publishing.  
S-05. Model/sandbox runtime never receives a GitHub App private key, API write token, webhook secret, unrelated repo contents or cloud runtime credentials.  
S-06. Repository file path resolution must stay inside designated checkout; block symlinks, hardlinks, `..`, mixed separators, absolute paths, device/socket files and submodule escapes per platform.  
S-07. Sandbox must not have privileged Docker socket, host PID/network access, writable host mounts or token-bearing environment. Any local no-sandbox mode must be clearly unavailable for untrusted tenants.  
S-08. Skill/playbook and bot text never changes identity, capability or security policy; treat it as untrusted evidence, validate any executable operation separately.  
S-09. Published code must match verified workspace and expected PR SHA. No force push, direct protected default-branch write or silent head drift.  
S-10. Every tool/model/job has budget, timeout, retry classification and cancellation, with cleanup on success/error/cancel.  
S-11. No secrets in logs, prompts sent to unexpected providers, PR comments, artifacts or audit payloads. Redact known keys and avoid logging raw request headers.  
S-12. Reviewer findings are provisional until reproduced/inspected. Do not suppress required checks or dismiss security reports merely to unblock merge.

## Threat model

| Threat | Entry | Failure mode | Required defense | Test IDs |
|---|---|---|---|---|
| Spoofing | Webhook request | Forged event runs job | HMAC, install/repo identity, supported event set | SEC-01, SEC-02 |
| Cross-tenant | Installation ID/repo mismatch | Read or write another customer's repo | Per-install token + repo allowlist, no shared worker secrets | APP-04, SEC-03 |
| Prompt injection | PR diff/description/comments, bot response | Untrusted text upgrades authority | Data-only context, strict policy enforcement on tool calls | SEC-04 |
| Skill injection | Project SKILL.md/playbook | Host command execution | Source digest, paths, policy-gated typed operations | SK-04, SEC-05 |
| Remote execution | Build/test scripts inside Fix | Escape sandbox or exfiltrate env | Rootless isolation, no secrets and network policy, resource budgets | SEC-06, SEC-07 |
| Replay/loops | GitHub retries, bots responding to bots | Duplicate commits/comments / runaway cost | Delivery-id idempotency, self-bot suppression, rate caps | CONV-03, CONV-04 |
| TOCTOU | PR head moved mid-run | Stale/foreign commit pushed | Final head SHA/branch/permission checks | FIX-03, SEC-08 |
| Availability | Huge diff, rate limits, model outage | Hang, partial coverage disguised as complete | Pagination, explicit coverage, circuit breakers and quotas | OPS-02, SEC-09 |
| Evidence fraud | Model claims checks passed | Fake audit and success | Run real commands, capture exit codes/log digests | FIX-04, SEC-10 |
| Dependency/supply chain | CI action tags, image dependencies | Unreviewed execution | Pin trusted Actions by digest/SHA where practical, audit dependencies | SEC-11 |

## Command permission matrix

| Action | Read metadata/files | Comment on PR | Write feature branch | Change default branch / merge |
|---|---|---|---|---|
| Review | Granted installation read permissions | Minimum `issues`/`pull_requests` write as needed | Never | Never |
| Suggest | Same as Review | Same as Review | Never | Never |
| Fix | Read, plus actor authorized by configured policy | Yes | `contents:write` only through publisher after recheck | Never as part of Fix |
| Release/merge controller | Read checks/reviews/protection | Report only | As dictated by explicit merge operation | Only when owner authorizes and every gate green |

An installed GitHub App may require `metadata:read`, `contents:read/write`, `pull_requests:read/write`, `issues:read/write`, and events such as `issue_comment`, `pull_request`, `pull_request_review`, `pull_request_review_comment`, `installation`, `installation_repositories`. These are **candidate permissions**; verify each real API endpoint's minimum scopes, split read-only deployment if possible, and do not request organization/admin scopes by convenience.

## Security gates for changing code

1. Show failed regression test reproducing confirmed defect, or documented new feature expectation.
2. Implement smallest coherent change, rerun targeted then relevant full suites.
3. Check dependency changes and new external calls.
4. Inspect every write path, credentials crossing boundary and retained log field.
5. Test malicious PR text, bot comment and SKILL.md variants whenever touched.
6. Document independent check evidence and residual risk; do not label unknown as safe.

## Stop conditions

- Unknown Qlty blocking vulnerability details or untriaged security finding.
- Required protections/approvals inaccessible or pending.
- Webhook signature, install scope or publisher auth uncertain.
- Rootless isolation cannot be demonstrated in the target mode.
- A regression test is deliberately weakened/removed to obtain green CI.
- Diff contains unrelated user modifications, credentials or unknown file ownership.
- Publishing SHA differs from verified SHA, or PR head moved.
- Privileged operation requested by an untrusted comment, bot or skill.

On stop: preserve branch, write a precise status with evidence and unblock condition; do not bypass the condition.
