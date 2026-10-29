# Source and license register

## Primary repository evidence

- OpenReview repository: https://github.com/imMamdouhaboammar/openreview
- PR #1: https://github.com/imMamdouhaboammar/openreview/pull/1
- PR #2: https://github.com/imMamdouhaboammar/openreview/pull/2
- PR #3: https://github.com/imMamdouhaboammar/openreview/pull/3
- Lab E2E: https://github.com/imMamdouhaboammar/openreview-e2e-lab/actions/runs/40809709032
- CI #1: https://github.com/imMamdouhaboammar/openreview/actions/runs/40813490261
- CI #2: https://github.com/imMamdouhaboammar/openreview/actions/runs/40725463868
- CI #3: https://github.com/imMamdouhaboammar/openreview/actions/runs/40812437497
- Semantic review issue #4: https://github.com/imMamdouhaboammar/openreview/issues/4
- Source snapshots were examined on `feat/identity-review-quality`, notably `src/agent/runner.rs`, `src/agent/tools.rs`, `src/workflow/context.rs`, `src/skills/discovery.rs`, `src/github/webhook.rs`, plus CI, `action.yml` and README. Do not interpret these links as current HEAD without rechecking.

## Authoritative platform documentation

- GitHub Actions token semantics: https://docs.github.com/en/actions/concepts/security/github_token
- Safe `pull_request_target` handling: https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target
- GitHub App authentication: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-as-a-github-app
- Installation authentication: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/authenticating-as-a-github-app-installation
- GitHub App webhook security: https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries
- Actions secure use: https://docs.github.com/en/actions/reference/security/secure-use
- GitHub REST pagination: https://docs.github.com/en/rest/using-the-rest-api/using-pagination-in-the-rest-api
- Actions workflow syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
- GitHub REST issue comments: https://docs.github.com/en/rest/issues/comments
- GitHub REST pull request reviews: https://docs.github.com/en/rest/pulls/reviews
- GitHub REST pull request review comments: https://docs.github.com/en/rest/pulls/comments

## Inspect: conceptual inspiration, no code reuse

- Upstream: https://github.com/Ataraxy-Labs/inspect
- License: https://github.com/Ataraxy-Labs/inspect/blob/main/LICENSE.md
- Inspected licensing label: `FSL-1.1-ALv2` (source-available terms with competition-related restrictions). This package copies **no Inspect code**, algorithm implementation, datasets or vendor prose. The semantic design is an independently specified candidate to be built and benchmarked.
- Reported vendor performance figures are not independently verified. Do not include them in release claims.

## Evidence provenance policy

All links identify sources, not execution authorizations. PR status, Docs behavior, dependency versions and license terms may change; recheck before release. Separate a dated observation, an inference and an unverified requirement in tickets and release reports.
