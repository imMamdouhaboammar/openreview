# OpenReview / Rovi — Identity Guidelines v1

Scope: candidate mascot-led identity in this local kit only. Existing brand guidance (root docs/brand.md) remains authoritative for existing bot comments, deployed GitHub identity, and published product UI until a separate decision.

## 1 / Character invariants
- Face must always read as **Rovi**: exact asymmetric head lineage, integrated lens eye, small left alert eye, check-shaped nose/beak and folded-page ear.
- The former procedural rig is a historical v1 asset. For image-first work use the selected character PNG as the visual identity reference; preserve asymmetrical ears, integrated lens and separate eye, check beak, compact proportions, palette, scarf and drawn texture.
- Generate new illustrations with the reference attached, inspect for identity drift, and revise named issues. Do not automatically trace raster illustrations into vectors.
- No bot chrome, antennae, glowing circuitry, anime eyelashes or interchangeable animal ears.
- Full body is for 128px+ assets; at 24/32 pixels use the dedicated compact symbol only.

## 2 / Palette
| Role | HEX | Use |
|---|---|---|
| Paper | #FAF7F0 | background, illustration canvas |
| Ink | #171717 | contours, primary type, key borders |
| Character white | #FFFEFA | white paper-shaped mascot |
| Clay | #CE7058 | possible issue / attentive highlight |
| Amber | #E8BE70 | needs investigation / waiting |
| Sage | #639781 | restrained supporting accent |
| Mist | #E4EBE5 | neutral supportive panels |
| Deep | #263C36 | secondary text on light surfaces |

Never equate clay = confirmed vulnerability or sage = passing test without explicit independent status words.

## 3 / Construction / stroke
- Main character contour approx. 6 source units in original head space; face lens 9.5 units; body/limbs use the same relative hand-drawn language.
- Eyes: right eye embedded in lens, left separate. Check beak remains a facial mark, not a generic giant success badge.
- Hands are small pale circles, shoes are simple elongated ink shapes. Body carries no decorative logo.
- Avoid arbitrary random deformation, over-shading, drop shadows around the character, and detached shadow fragments.
- SVG text remains selectable/editable. When publishing externally, verify font fallbacks and convert type to paths only when actually required.

## 4 / Spacing and safe area
- Full-body artwork: at least 7% margin; avoid clipping raised hands, folded ear or lens handle.
- Avatar: at least 9% internal breathing room, especially at 24px.
- UI illustration: never place Rovi over actual editable code lines, action buttons, interactive targets or critical badges.
- Scene titles/status remain independent of Rovi; hide mascot in accessibility semantics if purely decorative.

## 5 / Expressions
Neutral = calm inspection; curious = raised brow and lens focus; suspicious = narrowed eye; concentrating = low brow; confused = uneven mouth; concerned = worried brow; relieved = softer eyes; happy = open smile; proud = assured posture; sleepy = lowered lids. See individual SVGs and expression-sheet.svg; review legibility at 64px before adopting a facial variant.

## 6 / Props and Buglets
Review notes: rectangular paper slips. Code: horizontal lines with restrained red/green markers. Test status: clearly labeled PASS, FAIL, NOT RUN, or BLOCKED. Buglets are invented symbolic creatures, not actual detected issue instances. Never show them in a confirmed-security context without specific evidence.

## 7 / Motion
A 3.8s demonstration loop lives in motion/rovi-motion.svg. For real product interactions use short 180–420ms one-shot gestures, avoid perpetual movement near reading interfaces; respect CSS prefers-reduced-motion. Never imply animation is functional without browser evidence.

## 8 / Recommended implementation
- README/project page: original square icon or candidate horizontal wordmark after brand approval.
- Marketing hero/feature: scenes/hero.svg and three lead scenes after final art approval.
- Empty states: scenes/empty-state.svg (illustrative, not wired to an existing queue).
- Docs and onboarding: smaller cropped bust/pose SVG with role="img" and meaningful alt when information is conveyed.
- UI badges: use native text/icons, not animated character expressions as sole state channel.
- Existing root assets/openreview-mark.svg is untouched. GitHub bot identity remains controlled by actual GitHub App registration/configuration.

## 9 / Accessibility/QA
Check WCAG text contrast in target compositions. Do not treat mascot strokes as body text. Use focus-visible, high-contrast content, real status labels, reduced-motion, responsive layout and full alt descriptions where needed. Verify 24, 32, 64, 128, 256 and 512px rendered icon sizes and preserve transparency.

## 10 / Originality and licensing
The 2026 reference visual audit includes Capy art screenshots **for internal comparative research only**. No Capy-owned illustration assets may ship with OpenReview. New character drawings were made using the pre-existing Rovi path from the user's OpenReview brand kit, plus original new geometry and scenes.


## Image-first working reference — 2026-11-04

The exploratory raster candidate in `image-first/exploration/rovi-master-candidate-v01.png` is a provisional production reference, not stakeholder-approved artwork. Maintain the fluttering left ear, contrasting folded right corner, integrated lens, check-shaped beak, compact body, muted sage scarf, restrained terracotta and organic ink texture. Avoid overclaiming exact reproducibility across generated poses; compare actual visual outputs. Native transparent PNG is the preferred master; WebP derivatives are optional. Confirm alpha by compositing on light, dark and saturated backgrounds: RGB values in fully transparent pixels are not themselves visible halos. Review antialiasing, margins, resolution and full silhouettes; retain source image versions and prompts. Existing vector-specific construction details above apply only to the earlier v1 vector set.
