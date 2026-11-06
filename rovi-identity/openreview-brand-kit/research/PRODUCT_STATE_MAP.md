# OpenReview: product-to-illustration state map

Based on local docs/brand.md, README.md and code under src/agent/tools.rs, src/workflow/command.rs, src/workflow/coordinator.rs, and src/workflow/verification.rs. Worktree inspected 2026-11-04, head 58382b6b58e1e251e41f1a16ed4e203fe321c0fb.

| Product state | Verified product boundary | Rovi illustration | Prohibited implication |
|---|---|---|---|
| Review | Review mode exposes read-only operations (src/agent/tools.rs; tests check no writeFile or bash) | Rovi scans a diff, leans closer | Must not show a committed patch |
| Suggest | Parsing uses Review mode (src/workflow/command.rs) | Rovi opens a draft note card | Suggestion is not an applied repair |
| Potential finding | A finding needs factual support (docs/brand.md) | Terracotta-highlighted line, cautious face; possible Buglet | No fake P0 or vulnerability badge |
| Investigation | Investigation workflows documented under skills/deep-digger and src/e2e | Rovi follows signal → minimal reproducer → test result | Do not claim every possible bug was tested |
| Authorized Fix | src/workflow/coordinator.rs checks GitHub collaborator permission for Fix | Rovi holds a patch tile, authorization label | No default power to write from Review; no unauthorized push |
| Verification | src/workflow/verification.rs requires actual executed-test evidence; unverified blocks publication | Separate PASS/FAIL/NOT RUN art states | Never use green check for mere zero exit |
| Publishing | src/workflow/publisher.rs and coordinator mediate publish | A patch card can go to the branch only with verification | Never imply a publish occurred from just an art illustration |
| No work / idle | UI communication state only | Resting Rovi at an empty PR board | Not an assertion of queue implementation |

## Literal vs illustrative UI
There is no general marketing/product frontend in this Rust CLI/source tree. Illustration mock diffs are fictional teaching devices and deliberately do not claim to be exact real GitHub output. Keep labels like REVIEWING and NOT RUN readable independently from the mascot.

## Branding coexistence
Root docs/brand.md currently uses ink #101828 and verification accent #22C5B8 with assets/openreview-mark.svg. Rovi's new warm-paper/ink/terracotta/sage identity is a local **candidate**, not an approved replacement of existing deployed assets or PR comment branding. No product code was modified.
