# OpenReview / The Margins — Creative Direction v1

**Line:** Every change deserves a closer look.
**Tone:** Curious, precise, restrained, quietly mischievous. Never smug. The character should be honest about uncertainty and exuberant only when evidence earns it.

## Concept shortlist / decision
Scale 1–5, assessed against repo semantics and 82 page-reference audit. Scores reflect art-direction judgment, not independent research metrics.

| Direction | Product relevance | Originality | Clarity | Narrative range | Implementation ease | Total |
|---|---:|---:|---:|---:|---:|---:|
| The Margins: diff sheets become physical stages | 5 | 5 | 5 | 5 | 4 | 24 |
| Evidence Trail: investigation path as a map | 5 | 4 | 4 | 4 | 4 | 21 |
| Patchwork Desk: tiny workbench of code tiles | 4 | 4 | 3 | 4 | 3 | 18 |

**Selected:** The Margins. Evidence Trail becomes a supporting scene structure and Patchwork Desk contributes a few prop ideas only.

## Visual thesis
Rovi belongs at the edges of developer work, not pasted onto an unrelated generic illustration. A diff becomes a sheet with a tangible margin. A review note is a paper slip. A status chip is legible without the character. Rovi leans in, follows, suggests, pauses, and only celebrates after documented proof.

## Core construction
- **Primary silhouette:** the original white asymmetric head path, enlarged compared with a compact bean torso.
- **Defining face:** integrated black magnifying-glass eye; simple separate alert eye; check-inspired beak; right page-fold, left flutter.
- **Anatomy:** thin articulated soft arms ending in circular mittens; paired short legs and oval ink shoes. No metal joints, antennae or generic robo torso.
- **Colors:** warm paper #FAF7F0, ink #171717, accent terracotta #CE7058, amber #E8BE70, sage #639781.
- **Graphic language:** mostly straight UI cards; 2–4 px UI strokes, 6 px main character contour; large negative space and sparing annotated accent.
- **Text hierarchy:** modern bold system sans for product-like hierarchy; compact monospace labels for code/status; avoid imitating Capy's distinctive display type.

## Seven product moments
1. Hero: Rovi inspects the highlighted PR line.
2. Review: suspicious candidate triggers a *draft* comment.
3. Investigation: signal → repro → NOT RUN test card.
4. Fix: small *proposed* patch, never auto-published.
5. Verification: distinguish test PASS from FAIL.
6. Empty: nothing pending; resting character.
7. CTA: Rovi directs attention to a clear action edge.

## Artistic rules
- Align to a consistent 1600×900 illustration canvas for main scenes; output responsive HTML thumbnails and SVG assets.
- Never replace real developer status copy with emotion alone. Text carries proof status; character carries a reaction.
- Buglets are incidental and scarce; no endless expanded cast.
- Favor semantic motion: head tilt or note movement; avoid background confetti, neon, gradients and independent floating icons.
- Stay visibly independent from Capy's capybara/eyewear world. Similarities of general editorial storytelling are acceptable; do not replicate identifiable art or compositions.

## First visual QA review
Initial actual Chrome renders (exports/qa-hero-preview.jpg, qa-review-preview.jpg, qa-investigation-preview.jpg) show consistent linework and palette with readable product labels. Defects to address before master approval: seated/resting poses need a clearer basis, emotion sheet has subtle near-duplicates, and hero could use stronger physical interaction with the PR edge. These are not hidden or marked final.
