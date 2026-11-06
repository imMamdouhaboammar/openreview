# OpenReview / Rovi — Local Handoff v1

## Open the visual gallery on your Mac
/Users/mamdouhaboammar/Documents/antigravity/bold-tesla/rovi-identity/openreview-brand-kit/preview/index.html

Open the file in Chrome or Safari. No server, CDNs or npm dependencies are needed. The gallery contains 41 SVG visual assets with filter categories, scene previews and editable asset links.

## Canonical editable art
- character/rovi-full-body-master.svg — original head construction with new full-body rig.
- character/turnaround.svg and character/construction-sheet.svg — view turnarounds and proportion guidance.
- character/expression-sheet.svg, character/expressions/*.svg — 10 faces.
- character/pose-sheet.svg, character/poses/*.svg — 12 actions.
- character/rovi-small-mark.svg — experimental avatar simplification.
- character/rovi-horizontal-logo.svg plus dark/light variants — candidate logo lockups.
- character/buglets-exploration.svg — three supporting figures; do not overuse.
- scenes/hero.svg, review.svg, investigation.svg — three lead scenes.
- scenes/fix-proposed.svg, verify-pass.svg, verify-fail.svg, empty-state.svg, final-cta.svg — additional conceptual UI scenes.
- motion/rovi-motion.svg — working reduced-motion-aware SVG demo.
- exports/avatar-24.png through exports/avatar-512.png — 6 pixel sizes.
- exports/avatar-size-matrix.png — comparative compact vs legacy avatar sheet.
- exports/rovi-full-body-transparent.png — transparent asset.
- exports/scene-*.png and exports/scene-*.webp — eight scene exports.

## Regeneration and QA commands
Run from this brand kit directory:
1. python3 tools/build_identity.py
2. python3 tools/build_gallery.py
3. python3 tools/export_verify.py
4. python3 tools/test_gallery.py

Mac already has Python Playwright, Pillow, and Chrome. Bun is located at ~/.bun/bin/bun; Rust Cargo at ~/.cargo/bin/cargo. No package installation was needed.

## Research and decision files
- research/CAPY_SITE_AUDIT.md with 82 main-site page audit observations and screenshot paths.
- research/capy_inventory.json with structured inventory and 153 docs HTML URLs inventoried.
- research/CHARACTER_ILLUSTRATION_RESEARCH.md covering Contra, Capy, and the inaccessible Patty case study.
- research/contra_image_inventory.json and research/REFERENCE_BOARD.html.
- research/PRODUCT_STATE_MAP.md, CREATIVE_DIRECTION.md, IDENTITY_GUIDELINES.md.
- PLAYBOOK.md, EXECUTION_LOG.md, VISUAL_QA.md and MOTION_SPEC.md.

Only new original Rovi artwork and existing OpenReview-owned source geometry should be used for future marketing. Capy/Contra screenshots are internal research, not distributable production media.

## Existing project and approvals
Parent repository is Rust-based, not a typical marketing website. Existing bot review logic, GitHub comment styling, README, primary brand icon and existing mascot files were left unchanged. The new identity is an approved-to-review creative candidate, not automatically the published brand. No push, commit, branch switch, external deployment or publication.

Initial Git state: branch fix/issue-15-review-admission-budget; HEAD 58382b6b58e1e251e41f1a16ed4e203fe321c0fb; rovi-identity/ already untracked. Preserve this unrelated ongoing work.

## Open items
- Review expression distinctions (some pairs subtle in small sizes), then decide whether to approve the new brand world for product rollout.
- Exact Patty case study URL returned HTTP 404; secondary public text was consulted, not visual playback.
- 153 Capy docs HTML URLs were inventoried but not all visited in the browser; fine-grained scroll/hover animations not fully timed.
- Product integration would require a separate approved frontend-specific implementation and tests; local gallery works.
