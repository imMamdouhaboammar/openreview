# OpenReview Failure Lessons & Durable Engineering Memory

This repository documents engineering failures, security findings, and architectural lessons captured during the development, testing, and operation of OpenReview.

---

## Lessons Index

| Lesson Name | Failure Class | Impact | Status |
|:------------|:--------------|:-------|:-------|
| [Unchecked Remote Script Piping in CI](#remote-script-piping-in-ci-workflow-assets) | Supply Chain / RCE | Critical security scan rejection (`Risk Score: 51`) | Resolved |
| [YAML Folded Scalar Whitespace Normalization](#yaml-folded-scalar-whitespace-normalization) | Test & Eval Robustness | Unit test failure on description assertion | Resolved |

---

## Rules We Now Enforce for openreview

- **Rule 1 (Zero Remote Script Piping)**: Never pipe remote script downloads directly into shell interpreters (`curl | bash`, `wget | sh`) in workflow templates, documentation, or CLI installers. Always download to an isolated temporary file, verify checksum, execute explicitly, and clean up.
- **Rule 2 (Constant-Time HMAC-SHA256 Webhook Verification)**: Webhook ingress must reject unsigned or invalidly signed requests with HTTP 401 using constant-time comparison before any JSON parsing or database queries occur.
- **Rule 3 (Bounded Sandbox Execution)**: PR command execution must never execute unconstrained with host root privileges; Docker OCI containers with CPU/memory limits or ephemeral bounded directories must be used.
- **Rule 4 (Whitespace-Agnostic Semantic Assertions)**: Frontmatter and trigger assertions across agentic skills must normalize folded whitespace before checking substring containment.

---

## Detailed Failure Lessons

### Remote Script Piping in CI Workflow Assets

#### Context
GitHub Actions CI workflow template (`assets/github-action-workflow.yml`) providing an automated installer snippet for users and automated workflows.

#### What happened
The workflow included an uninspected remote script pipe pattern: `curl -fsSL https://.../install.sh | bash`. When the security audit ran (`scan_skill_security.py`), it flagged a CRITICAL finding (Rule `SC2: Remote Script Piping`), immediately yielding a `REJECT` verdict with a Risk Score of 51/100.

#### Observable symptom
```
[1] [CRITICAL] SC2: Remote Script Piping
    Category: Supply Chain
    Location: assets/github-action-workflow.yml:30
    Matched:  curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash
    Why:      Remote script downloading and execution (e.g. curl | bash, wget | python).
```

#### Impact
Security scan failure blocking release gate; potential risk of executing hijacked remote payloads if CDN or DNS spoofing occurs.

#### Incorrect assumption
Assumed standard developer installation idioms (`curl | bash`) were acceptable in automated template assets.

#### Root cause
**Status:** Confirmed  
Piping remote HTTP stream directly into bash executes unverified bytes with the runner's full environment tokens.

#### Why the architecture allowed it
Template assets lacked a pre-commit static supply-chain lint check prior to security scanning.

#### Fix
Refactored the installation step to download the script to a sandboxed temporary file, invoke bash on the local file path, and immediately remove the temporary artifact.

#### Verification
Deterministic scan via `python3 scripts/scan_skill_security.py skills/openreview/ --strict` returned `Verdict: APPROVE`, `Risk Score: 0/100`.

#### Prevention rule
Never emit `curl | bash` or `wget | sh` in any codebase, asset, documentation, or skill artifact.

#### Reusable lesson
All installer scripts in developer tooling skills must demonstrate safe download, staging, and execution practices.

#### Related code
`skills/openreview/assets/github-action-workflow.yml`

#### Related tests
`skills/openreview/tests/test_eval_suite.py`

#### Related lessons
None (initial release)

#### Status
Resolved

---

### YAML Folded Scalar Whitespace Normalization

#### Context
Eval suite assertion checking that negative trigger guidance ("Do NOT use for general manual code reviews") was present in `SKILL.md` frontmatter description.

#### What happened
The test performed a naive `self.assertIn("Do NOT use for general manual code reviews", desc)` which failed because YAML folded scalars (`>`) split long prose across multiple lines with newline characters (`\n`).

#### Observable symptom
```
AssertionError: 'Do NOT use for general manual code reviews' not found in '---\nname: openreview\ndescription: >\n  Technical specification...\n  reviews or editing pull requests directly.'
```

#### Impact
Eval harness false-negative test failure.

#### Incorrect assumption
Assumed YAML descriptions would be parsed as a contiguous single-line string when reading raw markdown text.

#### Root cause
**Status:** Confirmed  
Folded scalar line-wrapping inserted `\n` in the middle of the asserted string.

#### Why the architecture allowed it
The test read raw file contents without YAML parsing or whitespace normalization (`" ".join(text.split())`).

#### Fix
Applied whitespace normalization in the eval assertion prior to substring containment testing.

#### Verification
`test_eval_reg_01_negative_trigger_discrimination` passed with 100% reliability across all test runs.

#### Prevention rule
Always normalize multi-line strings or parse YAML frontmatter semantically before executing assertion tests.

#### Reusable lesson
Never perform literal multi-word substring assertions against markdown or YAML documents without normalizing inter-word whitespace.

#### Related code
`skills/openreview/tests/test_eval_suite.py`

#### Related tests
`test_eval_reg_01_negative_trigger_discrimination`

#### Related lessons
None

#### Status
Resolved
