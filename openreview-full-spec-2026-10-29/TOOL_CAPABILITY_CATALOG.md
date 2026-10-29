# Agent tool and capability catalog (proposed)

This describes the **desired** tool surface. As of the snapshot, `src/agent/tools.rs` advertises `readFile`, `loadSkill`, `reply`, and adds `bash`/`writeFile` in Fix. Do not call the proposed tools implemented until verified in source and tests.

## Read-only tools in all modes

| Proposed tool | Data returned | Key limits |
|---|---|---|
| `listRepositoryFiles` | Bounded path list with size/type and cursor | Within checked-out repository; maximum entries and depth |
| `searchRepository` | File/line/search result with head SHA and truncation | Literal/regex options with complexity limits, bounded matches and bytes; no arbitrary shell |
| `readFile` | Text + path + file digest/size | Existing code; enforce symlink and path policy, cap bytes |
| `getPullRequestSnapshot` | Diff, changed files, base/head, coverage | Exact current head SHA, complete pagination or explicit incomplete state |
| `getConversationThread` | Typed issue/review/inline events | Authenticated repo scope, cursor and privacy boundaries |
| `getPullRequestChecks` | Check state and linked run IDs for SHA | GitHub API read permissions, never infer post-push status |
| `readWorkflowLogs` | Bounded, redacted job logs for current repo/run | May contain secrets; redact before model and only allow authorized repo |
| `getRepositoryPolicy` | Protected branch and declared project constraints | Return only authorized metadata; server-side policy still authoritative |
| `loadSkill` | Parsed skill content + digest and origin | Existing tool; repo content is untrusted |
| `reply` | Idempotent publication request | Existing tool; host validates actor, thread and state |

Use API-backed read operations for checks/logs rather than giving Review a shell. Keep a normalized capability interface and test denial at dispatcher as well as at tool registration. Never trust an LLM's self-report that it did not call an unavailable tool.

## Mutating tools in Fix only

| Proposed tool | Permission boundary | Verification |
|---|---|---|
| `writeFile` | Isolated checkout only, allowed path and size | Compare pre/post file digest |
| `applyPatch` | Structured patch limited to checkout | Reject path escapes and partial apply |
| `runTests` | Uncredentialed rootless sandbox | Return structured exit status/stdout cap, timeout, resource cost |
| `bash` | Existing Fix-only escape hatch, must be strongly sandboxed and bounded | Record command digest, exit, no credentials/host mounts |
| `viewDiff` | Workspace read-only | Return bounded diff and changed files |
| `proposeCommit` | Model supplies intent only | Trusted publisher independently constructs/verifies commit |
| `publishFix` | Trusted publisher only; never a raw model-controlled `git push` | Live permissions, branch/head checks, test proof, exact SHA |

Do not expose generic `ssh`, `kubectl`, cloud console, GitHub secrets administration, force push or production deployment tools by default. Optional future DevOps tasks are explicitly scoped separate actions with owner approval and restricted host plugins, not a blank check for the model to operate infrastructure.

## Multi-step DevOps agent behavior

An agent should *reason* about CI failure, check repository workflow config, read relevant logs, locate tests, propose a fix, execute within sandbox, rerun checks, and publish a scoped PR fix if authorized. That is an agentic DevOps **development** workflow; it does not imply permission to deploy or administer servers.

Example trace:

1. `getPullRequestChecks` returns failing `cargo test` at SHA X.
2. `readWorkflowLogs` yields a redacted failed test name, capped at declared budget.
3. `searchRepository` locates test and associated implementation by symbol.
4. `readFile` establishes current logic, including repository playbook and scope.
5. Agent writes plan and a failing regression before `writeFile`/`applyPatch` in authorized Fix.
6. `runTests` produces actual exit codes; if failure persists, agent iterates within budget.
7. `viewDiff` and trusted verifier screen scope and tests; trusted publisher confirms current SHA and pushes.
8. `getPullRequestChecks` polls the resulting SHA; if no CI was triggered by token event, report exactly that and use approved explicit dispatch route.

## Determinism and negative tests

- Unknown tool, wrong mode, wrong path, wrong installation, stale PR head and exceeded budget produce hard rejection before IO.
- Arbitrary regex search can be expensive: time/match limit and regex engine safe config required.
- Tool output carries `truncated`, `result_count` and provenance; truncation cannot be presented as complete search.
- Logs always have redaction and max size; skip binary artifacts and links outside allowed repo.
- Model cannot self-publish; responses from provider are requests to authorized host tool dispatcher.

## Acceptance addition to M4

Test `TOOL-01`: Review can search and inspect a repository fixture without bash; Fix can patch multiple files only in sandbox; a fabricated `bash` call in Review gets `permission_denied`.  
Test `TOOL-02`: A failed GitHub check -> bounded redacted log -> code search -> reproducible fix -> real test -> authorized publish chain works.  
Test `TOOL-03`: Search over large repo truncates visibly; `readWorkflowLogs` refuses a different repo's run and strips fixture secrets.  
These tests join FIX-01/02/04 and must be included in M4-01 / M4-02 relevant acceptance.
