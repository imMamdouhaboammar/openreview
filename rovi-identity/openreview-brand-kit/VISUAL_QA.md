# Rovi Visual & Technical QA — 2026-11-04

## Visually inspected in actual Chrome
- exports/rovi-master.png: white asymmetric head, black lens-eye, check feature, small new torso and limbs all rendered clearly.
- exports/expressions-final.png: 10 consistent head-path expressions. Concerned/confused and neutral/concentrating remain similar at very small size, so label them where necessary.
- exports/poses-final.png: 12 reusable poses. Initial sitting/resting similarity was corrected by adding an understated horizontal seat and a sleepy face to rest.
- exports/construction-final.png: inspected front anatomy and head geometry guide.
- exports/qa-hero-preview.jpg, qa-review-preview.jpg, qa-investigation-preview.jpg: three lead editorial illustrations inspected side by side; distinct PR, draft review, and not-run evidence metaphors.
- exports/avatar-size-matrix.png: actual candidate and legacy logos in 24,32,64,128,256,512 px render sizes inspected. Simplified candidate is readable but needs creative approval before replacing existing 24px icon.
- preview/gallery-1440.png and preview/gallery-390.png inspected; 320px tested with no overflow.
- motion/motion-live-no-preference.png visually inspected, Rovi placed correctly beside highlighted code.

## Technical evidence
| Check | Execution | Observed result |
| --- | --- | --- |
| SVG syntax | Python ElementTree on character, scene, motion sources | 41 SVGs parse |
| PNG dimensions and alpha | Python tools/export_verify.py through actual Chrome/Pillow | 22 PNG renders + 8 scene WebP; dimensions match; full-body transparent PNG alpha check passes |
| Icon sizes | Chrome render of 24/32/64/128/256/512 legacy and candidate | 12 outputs of requested dimensions |
| Preview image loading | Python tools/test_gallery.py | All 43 page IMG elements have nonzero naturalWidth after eager loading |
| Preview categories | Same browser test | All/Expressions/Scenes filter controls work |
| Responsive | Same test at 1440,390,320 CSS px | No horizontal overflow |
| Motion | Chrome computed animation properties, two time-samples and bounding boxes | Movement in normal mode and no movement with reduced-motion; character inside expected region |
| Reference board | Browser screenshot-loading test | All indexed browser captures render |
| Core Rust regression | ~/.cargo/bin/cargo test --locked --quiet at repository root | 159 passed, zero failed, exit 0 |

## Recorded issues and corrections
1. Fixed 320px horizontal overflow from intro title white-space:nowrap. A browser test first failed, then CSS changed and test passed.
2. Fixed false-negative image test for lazy-loaded assets by requesting eager loads before validating naturalWidth.
3. Fixed local research-board screenshot relative paths. Browser test caught broken files before correction.
4. Fixed SVG animation transform overriding positional transform. New behavioral test first failed (wrong bbox), then nested positioning and animation groups corrected it. Re-ran dynamic frame checks and visual inspection.
5. Partially unresolved art direction: some expression differences are subtle at 64px; a future designer can strengthen eyebrow and lid grammar. This is a non-blocking visual refinement, not a code test failure.

## Contrast measurements
Against #FAF7F0 paper, measured WCAG luminance ratios: ink #171717 16.76:1; deep #263C36 11.01:1; clay #CE7058 3.23:1; sage #639781 3.13:1; darker clay text substitute #944B37 5.91:1. Original clay and sage are NOT suitable for small body text on paper. Text renderer substitutes stronger contrast colors; the gallery headline accent uses #944B37.

## Limits
This is technical acceptance of the local visual kit, not a full WCAG audit, user-tested brand approval, entire Capy documentation audit, or production frontend integration. Existing bot code was not modified. Never treat illustrative verification PASS scene as evidence of a real executed OpenReview run.
