# OpenReview Character Universe — Execution Playbook v1

**Project:** Rovi / The Margins • **Local root:** rovi-identity/openreview-brand-kit • **Execution:** 2026-11-04
**Policy:** All permanent files stay inside this identity kit; never overwrite existing source assets without an explicit versioned alternative; no pushes or releases; respect unrelated Git changes. Verification evidence is mandatory.

## Workflow contract
1. Read the live code and actual product documentation, then inspect visuals in a local browser.
2. Separate first-party research screenshots from original OpenReview production artwork. Do not import Capy-owned art into a production directory.
3. Document creative decisions and state constraints before declaring visuals final.
4. Preserve Rovi identity using an approved image master as visual reference: asymmetrical ears, face, proportions, palette, linework and accessories. The historical exact vector-path requirement is superseded.
5. Generate original raster illustrations with native transparency; inspect actual results at target sizes, edit specific defects, and verify alpha and margins. SVG is optional for functional icons and logos; never auto-trace artwork.
6. Distinguish RED/GREEN software verification from visual art direction; an XML-valid SVG is not automatically visually approved.
7. Use honest states: Pending, In progress, Needs revision, Verified, Blocked. Log actual commands and their outcomes.
8. A phase is Verified only after its acceptance checks pass, not merely when a file exists.
9. Technical integration is optional unless a suitable UI surface is confirmed; no Rust bot behavior changed to display art.
10. Retain original images and Git tree. Never publish without explicit instruction.

## Initial baseline
See research/BASELINE.md. Existing kit was untracked on branch fix/issue-15-review-admission-budget, at 58382b6b58e1e251e41f1a16ed4e203fe321c0fb. Bun not available in current PATH. Node, Python Playwright, Chrome available. No root AGENTS.md found.

## Phase 01 / Discover
**Objective:** Establish truthful product, repo, assets, and public reference inventory.
**Inputs:** Current Git tree, README, docs/brand.md, original SVG/PNG, Capy homepage/sitemap, Contra, Patty.
**Tasks:** Record baseline; compare repo capabilities to proposed scenes; browser-crawl all 82 Capy sitemap URLs, collect desktop/mobile screenshots, inspect public sitemap and link discovery; attempt exact external references; record exceptions.
**Tools/skills:** A2M read/shell, local Chrome/Playwright, external web search, engineering-workflow guide.
**Expected files:** research/BASELINE.md, capy_inventory.json, CAPY_SITE_AUDIT.md, screenshots/capy/*, screenshots/creative/*.
**Acceptance:** Real Mac access, source files preserved, every discovered Capy main sitemap path visited or listed with a blocker, Contra visited or failed transparently.
**Verification:** inspect JSON counts, screenshot file existence, HTTP statuses, Git baseline.
**Risks/dependencies:** Dynamic/lazy imagery, blog pages, external site 404, download binaries, docs tree large.
**Status:** In progress — 82/82 main sitemap URLs loaded with screenshots; 10 additional links are download binaries/feed; 153 documentation routes inventoried but not all visually inspected; Patty URL returned HTTP 404 on Mac.
**Next:** Recheck lazy-loaded Contra visuals, annotate page families and missing interactions.

## Phase 02 / Understand
**Objective:** Translate implementation constraints into design-safe product metaphors.
**Inputs:** src/agent/, src/workflow/, docs/brand.md, README, source status taxonomy.
**Tasks:** Map review / potential issue / investigation / authorized fix / verification / blocked states; identify API and ownership constraints.
**Tools/skills:** A2M file inspection, architecture analysis.
**Expected files:** research/PRODUCT_STATE_MAP.md and creative direction notes.
**Acceptance:** No scene implies a verified fix without test proof; buglets never equate speculation with confirmed vulnerability.
**Verification:** Compare textual scene captions against actual source policy.
**Risks/dependencies:** README may describe aspirational features; inspect code whenever actual behavior matters.
**Status:** In progress — initial constraints confirmed in repo docs.
**Next:** Complete state map and link to relevant source modules.

## Phase 03 / Concept
**Objective:** Select a unique, usable storytelling direction.
**Inputs:** public research, legacy Rovi icon, product-state map.
**Tasks:** Compare The Margins vs Evidence Trail vs Patchwork, score product relevance/originality/clarity/flexibility, finalize narrative and motifs.
**Tools/skills:** Creative direction, character design, visual QA.
**Expected files:** CREATIVE_DIRECTION.md, research/REFERENCE_BOARD.html.
**Acceptance:** Selected concept is original; visual motifs transfer to UI and documentation without copying Capy.
**Verification:** Concept score table, compare shipped SVG to reference screenshots.
**Risks/dependencies:** Generic code imagery, too many decorative motifs.
**Status:** In progress — The Margins selected provisionally.
**Next:** Validate scenes and scores after rendered comparison.

## Phase 04 / Character development
**Objective:** Explore distinct image-generated directions, select and refine a master; produce a consistent full-body character, turnarounds, expressions and poses from the approved visual reference.
**Inputs:** Existing original mascot SVG; editable rig definition.
**Tasks:** Keep head path, checkmark and lens; build torso, arms/feet, views, expressions and pose variations; test recognition at small sizes.
**Tools/skills:** Python vector generator, Chrome SVG renderer.
**Expected files:** character/rovi-full-body-master.svg, turnaround.svg, construction-sheet.svg, expression-sheet.svg, pose-sheet.svg, expressions/*.svg, poses/*.svg.
**Acceptance:** Correct counts, no broken shapes/face collision, meaningful individual actions, inspect actual sheets.
**Verification:** XML parse + rendered sheets + at least one corrective iteration.
**Risks/dependencies:** Side/rear should not be mirrored fake front; seated/resting needs clearer posture; expressions may be subtle.
**Status:** Needs revision — first visual render shows resting/sitting and some emotion distinctions are weak.
**Next:** Adjust poses and re-render.

## Phase 05 / Visual world
**Objective:** Original editorial universe grounded in diff, annotations, and evidence.
**Inputs:** Concept, character rig, real OpenReview states.
**Tasks:** Build code-page surfaces, comment labels, line markers, minimalist evidence props, optional Buglets.
**Tools/skills:** Original SVG scene construction.
**Expected files:** scenes/*.svg; character/buglets-exploration.svg; IDENTITY_GUIDELINES.md.
**Acceptance:** Rovi fits coherent drawing system and UI is readable; Buglets are never claimed to be real verified defects.
**Verification:** Side-by-side rendered hero/review/investigation, distinct staging and state semantics.
**Risks/dependencies:** Cartoon competing with technical data, Capy resemblance.
**Status:** In progress — 3 lead scenes and 5 supporting vector compositions drafted/rendered.
**Next:** Refine scene interaction and review hierarchy.

## Phase 06 / Production
**Objective:** Editable masters and practical exports.
**Inputs:** Approved character geometry and illustration compositions.
**Tasks:** Generate all SVGs from one source; preserve handoff originals; render transparent PNG and optimized WebP at actual sizes; validate SVG paths.
**Tools/skills:** Python + Chrome Playwright + Pillow.
**Expected files:** character/*.svg, scenes/*.svg, exports/*.png, exports/*.webp, tools/build_identity.py.
**Acceptance:** Approved transparent PNG illustrations, complete intended expression/pose set, no blank or clipped outputs, genuine alpha and clean edges; WebP derivatives where useful. SVG is optional and is not an illustration acceptance requirement.
**Verification:** XML parsing, raster dimension/alpha checks and browser visual inspection.
**Risks/dependencies:** Missing manual QA, small-size facial marks disappearing.
**Status:** In progress — 39 character/scene SVGs parsed cleanly at first pass.
**Next:** Render size matrix and inspect icon detail.

## Phase 07 / Motion
**Objective:** Attentive microinteraction without distracting from code.
**Inputs:** Rig, review-note semantics, browser capabilities.
**Tasks:** Add subtle tilt/annotation/attention animations with reduced-motion fallback; avoid fabricated success status.
**Tools/skills:** Lightweight SVG/CSS animation, Playwright.
**Expected files:** motion/rovi-motion.svg, motion/motion-*.png.
**Acceptance:** Active animation in no-preference; none under reduce.
**Verification:** Browser computed animation-name checks and screenshots in both modes.
**Risks/dependencies:** Autoplay / motion distraction; must be subtle.
**Status:** In progress — generated; awaiting test check.
**Next:** Run gallery acceptance tests.

## Phase 08 / Integration
**Objective:** Provide a usable local preview without modifying unrelated bot behavior.
**Inputs:** Existing Rust CLI architecture, finished SVGs.
**Tasks:** If there is no frontend web surface, create a standalone local preview in the identity kit; avoid adding app dependencies; do not alter GitHub bot posting identity.
**Tools/skills:** HTML/CSS/JS, A2M, Playwright.
**Expected files:** preview/index.html, preview/gallery-*.png, tools/build_gallery.py.
**Acceptance:** Local gallery browsable, filters work, no broken images, desktop/mobile functional.
**Verification:** Test gallery in Chrome at 1440, 390 and 320 CSS pixels.
**Risks/dependencies:** Lazy images may not load before test; no website framework exists in primary Rust repository.
**Status:** In progress — standalone gallery generated, TDD red observed, regression checks ongoing.
**Next:** Fix test or real UI defects and rerun.

## Phase 09 / Visual QA
**Objective:** Verify rendered quality, accessibility and technical integrity.
**Inputs:** Character sheets, scene SVGs, gallery, motion, size matrix.
**Tasks:** Inspect at 24/32/64/128/256/512; contrast; clipping; transparent output; reduced motion; final Git status; retain findings.
**Tools/skills:** Playwright screenshots, Pillow alpha checks, XML parser, QA checklist.
**Expected files:** VISUAL_QA.md, exports/avatar-*.png, preview/gallery-*.png, QA issues/revisions.
**Acceptance:** All technical tests green and visual exceptions explicitly resolved or logged.
**Verification:** Captures plus fresh tests, not export command alone.
**Risks/dependencies:** Existing codebase dependencies and CI unrelated to identity should remain untouched.
**Status:** In progress.
**Next:** Finish exports and inspect.

## Phase 10 / Handoff
**Objective:** Leave a reusable identity system in the local requested kit.
**Inputs:** Verified assets, research, directions, gallery, QA logs.
**Tasks:** Document file index, adoption paths, usage limits, known gaps and integration instructions; record final status.
**Tools/skills:** Local project docs and Git diff inspection.
**Expected files:** HANDOFF.md, EXECUTION_LOG.md, IDENTITY_GUIDELINES.md.
**Acceptance:** All files located in required kit; no push; clear guidance and honest exceptions.
**Verification:** final directory tree, XML/raster/browser tests, repo status.
**Risks/dependencies:** Unapproved project-wide branding changes.
**Status:** Pending.
**Next:** Assemble verified handoff.

## Milestone evidence rules
- No scene may display test PASS unless that is explicitly an illustrative generic state, not a report of an actual OpenReview run.
- Review remains read-only; Fix requires authorized actor, sandbox and verification before publishing.
- Rovi original sources must remain intact, and generated masters live in separate folders.
- Reference screenshots are internal research; do not copy them into marketing assets.
- Each “Verified” state requires named evidence in EXECUTION_LOG.md.

---

# Final session checkpoint — 2026-11-04 (supersedes earlier in-progress snapshots)

All phase entries above were written as the work proceeded. The authoritative latest status is below; older entries document the historical checkpoints rather than the final state.

| Phase | Latest status | Acceptance evidence | Next action / unresolved dependency |
| --- | --- | --- | --- |
| 01 Discover | Verified main-site audit; Blocked exact Patty video + full docs layouts | 82/82 Capy main sitemap routes with desktop/mobile captures, 153 docs HTML URLs enumerated, Contra 26/26 images loaded, Patty 404 recorded | Revisit docs and video if more complete visual reference research is required |
| 02 Understand | Verified | Rust Review/Fix boundary checked in source and PRODUCT_STATE_MAP.md | Revalidate when product features change |
| 03 Concept | Verified for v1 proposal | CREATIVE_DIRECTION.md scoring and original Three Margins composition studies | Obtain creative stakeholder approval before public adoption |
| 04 Character | Verified editable v1; Needs revision for nuanced emotion differentiation | Master/turnaround/10 expressions/12 poses rendered and visually inspected | Strengthen several very close facial variations at 64px |
| 05 Visual World | Verified v1 | Three lead SVG scenes, five supporting scenes, and restrained Buglets | Extend only after approved direction |
| 06 Production | Verified | 41 SVGs parse; 22 browser-exported PNGs; 8 scene WebPs, size/alpha tests | Export future variations as needed |
| 07 Motion | Verified | TDD red→green for bounding box; actual animation and reduced-motion verified in Chrome | Convert demo loop to event-driven motion if deployed |
| 08 Integration | Verified local standalone gallery; production integration intentionally deferred | Gallery 41 assets, working filters, responsive widths 1440/390/320 | Await authorized frontend-specific work |
| 09 Visual QA | Verified technical tests; final creative sign-off open | VISUAL_QA.md, 159 Rust tests passed, size matrix and browser screenshots inspected | Review remaining subtle facial differences |
| 10 Handoff | Verified local candidate | HANDOFF.md, EXECUTION_LOG.md, guidelines, motion spec, preview all present | Brand adoption decision; no publication |

**Current execution status:** Local OpenReview Rovi Character Universe v1 delivered and technically verified, with documented external reference limitations and creative sign-off open. No repository source changes outside the identity directory, no commit, no push, no external publication.


# Image-first production override — 2026-11-04

This section supersedes every earlier SVG-first illustration requirement, including the historical final checkpoint. The previous vector work remains recoverable and is not approval evidence for this new image-first phase. No automatic raster tracing.

1. Generate several materially different Rovi directions with Create image, compare actual images for originality, silhouette, personality, expression, consistency and small-size readability; document the selection, refine it, and establish a master reference.
2. Generate transparent full-body, turnaround, expression, pose, supporting-character and independent prop PNGs using the master as reference. Retain head and asymmetrical ears, face, eye and body proportions, palette, line and accessories; inspect for drift and correct with targeted edits. Keep transparent margins and complete limbs.
3. Produce at least three polished original scenes grounded in actual product states; generate separate character and prop cutouts wherever independent positioning or animation is needed. Treat any rendered status as illustrative, not evidence of an actual test.
4. For each asset record objective, prompt, source references, version, visual review and corrections alongside the asset. Preserve previous approved versions. Inspect actual images and programmatically verify dimensions, alpha, bounds and edges; create optimized WebP where useful.
5. Choose appropriate motion-ready image parts, keyframes or poses and support reduced motion. Update the standalone responsive gallery and guidelines. Validate images, visual consistency, responsive gallery and motion behavior.
6. PNG is the primary illustration master; SVG is optional for logos and functional icons. No vector source or XML test is required to approve an illustration. Previous SVG metrics are historical only.

**Image-first phase status: In progress.** Prior v1 verified labels do not imply that the newly requested image-first assets have been created or approved.
