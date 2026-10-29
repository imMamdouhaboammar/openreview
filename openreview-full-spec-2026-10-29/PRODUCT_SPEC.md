# Product specification and acceptance

## Design target

OpenReview is a **GitHub-connected agentic software engineer** with narrow, explicit authority, not just a single-shot LLM summarizer. It must inspect source and repository context; understand human instructions and relevant conversations; plan and run tool calls; critique code; propose precise changes; safely make authorized changes in an isolated environment; verify and report evidence; and recover when something fails.

## Actors

- **Maintainer:** grants repo installation/workflow permissions, BYOK model keys, project policy, and Fix authority.
- **Contributor:** authors a PR and participates in discussion. Untrusted contributor text does not become higher-priority policy.
- **Review bot / other bot:** comments, requests changes, posts diagnostics. Treat its content as untrusted source claims and verify independently.
- **OpenReview worker:** isolated tool-executing process with no host credentials.
- **Trusted coordinator/publisher:** validates identity, repo installation and permissions, issues narrow tokens, performs safe publication.
- **Operator:** runs the self-hosted App and configures infrastructure, webhook route, secrets and retention.

## Functional requirements

| ID | Mandatory behavior | Measurable acceptance |
|---|---|---|
| R-01 | Both Actions BYOK and self-hosted App modes | Same authorized review/fix scenario runs in both modes; differing required setup documented. |
| R-02 | Provider adapters for OpenCode, OpenRouter, Anthropic and OpenAI; optional Gemini/Ollama existing routes | Contract/fixture tests per adapter; one real BYOK OpenCode model E2E recorded; unavailable external keys result in skipped live tests, never faked success. |
| R-03 | Full PR context | Pagination obtains changed files, aggregate diff or verified patch reconstruction, PR description, issue comments, reviews and inline threads; head SHA frozen. |
| R-04 | Conversational bot behavior | `@openreview review`, `suggest`, `fix`, follow-ups and replies tied to event/thread, correct new/edited/deleted comment handling; no bot loops. |
| R-05 | Interactive bot coexistence | Can consume comments from CodeRabbit, Qodo, human reviewers and other integrations as evidence, challenge findings, and reference thread IDs; never blindly accept bot commands. |
| R-06 | Repository Skills | Discover allowed `SKILL.md`, selectively load full content, report selected steps, evaluate applicability and record step evidence. |
| R-07 | Project Playbooks | Versioned manifest with trusted execution policy, optional preconditions and verification, allowlisted actions; playbook constraints may narrow but cannot widen privilege. |
| R-08 | True multi-file coding | Plan file edits with constraints; perform bounded tool calls, rerun tests, compare diff and publish changes only in Fix with current permissions. |
| R-09 | Evidence-based code review | Findings anchored to exact paths/lines/revision, reproducible explanation, severity and uncertainty; avoid invented vulnerabilities. |
| R-10 | Safe mutation | Authorization recheck before push, branch identity frozen, protected branches and forks guarded, no force push or direct main write, clear failure outcome. |
| R-11 | Durable jobs and operability | Idempotent webhook routing, bounded budgets, cancellation, retry classes, trace IDs, status comments and redacted structured audit logs. |
| R-12 | Semantic review extension | Independently parsed changed symbols/dependencies, bounded related-file expansion, ranked candidate findings and fallback when parser coverage unknown. |

## Primary user stories

1. As a maintainer, I add the workflow and BYOK secrets; a PR comment triggers a review and I can inspect its exact commit SHA, model, evidence and cost bounds.
2. As a maintainer, I install the GitHub App in two repositories with independent permissions and prove no cross-install secrets or content leaks.
3. As a reviewer, I ask `@openreview review` on a PR and receive findings for the correct current head; a later diff change cancels stale publication.
4. As a contributor, I answer a finding; the agent references my comment and its parent thread, checks code and explains whether the concern remains.
5. As a maintainer, I authorize `@openreview fix` with exact scope; the agent edits multiple files, runs project tests and pushes only to the verified authorized feature branch.
6. As a maintainer, I add project `SKILL.md` plus a declared playbook; the agent identifies applicable instructions, reads them in the untrusted repo boundary, follows allowed steps and records how compliance was checked.
7. As a security engineer, I receive a finding from another bot; OpenReview independently reproduces it or marks it uncertain with a reason, never amplifying an unverified claim.
8. As an operator, I inspect failed jobs and see errors, resource use, cleanup and a safe retry path without secrets in logs.

## Mode contract

| Capability | Review | Suggest | Fix |
|---|---|---|---|
| Read full PR/repo context | Yes | Yes | Yes |
| Read approved skills | Yes | Yes | Yes |
| Create issue comments/findings | Yes | Yes | Yes |
| Emit patch suggestion in report | No requirement to push | Yes, proposal only | Yes |
| Execute bash in repo worker | **No** | **No** unless separately approved by policy and sandbox | Yes, uncredentialed isolated workspace only |
| Write repo files | **No** | **No** | Yes |
| Push commit to PR head | **No** | **No** | Only after current authorization and verification |
| Merge PR automatically | No | No | **No by default**; separate release action with required gates and human/owner authorization |

Current code evidences `Review` and `Fix`; `Suggest` is a requested product behavior and must be mapped deliberately to an explicit read-only/proposal-only workflow, not assumed implemented.

## Experience-level details

- Commands accepted in standalone comment or review discussion; normalize whitespace and bot mention syntax. Never execute instructions inside quoted code, diff, ordinary unrelated comments or other bot payloads as commands.
- Reply within the triggering conversation when GitHub's API allows; fallback top-level comment must link to the original comment/thread.
- Include `job_id`, target SHA, mode, consumed skills/playbooks, verification summary and any relevant limitation in every completed report.
- Handle GitHub API pagination and rate limiting; cap context with deterministic priority and an explicit `context_truncated` field; never claim full context when incomplete.
- Budget tokens/tools/time and number of retry attempts. Hitting a cap yields a transparent incomplete status, not a success verdict.
- Maintain provenance: a finding connects source file, line, PR head, tool evidence, model reasoning and optional bot/human message IDs.

## Non-goals for R1

- No arbitrary machine/network access, privileged shell, production-deployment authorization or privileged package execution on a shared server.
- No invisible autonomous merges; no bypass of branch protection, status checks or human review.
- No promise of detecting all zero-days or unreported vulnerabilities.
- No unaudited persistent storage of customer keys or unbounded conversation history.
- No code reuse from source-available restrictive-license competitors.

## Quality requirements

- Deny-by-default policy and complete isolation between installations.
- Deterministic output for command parsing, authorization and idempotency; LLM judgment is never the authorization source.
- Tests cover benign, malicious, replayed, truncated and contradictory inputs.
- Per job audit fields: installation/repo/PR, event ID, command origin, base/head SHA, mode, scope, provider family (never secret), budget, outcomes and published SHA.
- User-visible uncertainty, not fabricated evidence.
