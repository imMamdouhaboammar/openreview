# Rovi image-first exploration — 2026-11-04

Status: **in progress; no approved master**. The original vector kit remains unchanged.

## Direction comparison
One generated three-character comparison explored (A) an elongated editorial investigator, (B) a compact bean-bodied paper character, and (C) an angular long-legged printmaking character. Preliminary choice B: the silhouette and compact body support more expressive poses; A reads too much like a conventional lab-coated investigator and C's clothing competes with the face. All three use asymmetrical paper ears and an eye/lens; this is a visual design judgment, not a user approval.

## Candidate v01
- File: `rovi-master-candidate-v01.png`; SHA-256 `ba89d54b1f411dd28819ec9676a878de233c2b340a0a463192f1e3b94683a448`.
- Objective: refine B into a single standing full-body reference.
- Reference: generated three-direction comparison, center character. The comparison itself has not been transferred into this project.
- Prompt: Refine the CENTER candidate from the supplied character exploration into one standalone full-body Rovi character master. Preserve its asymmetrical flutter ear and folded corner, expressive eye proportions, paper head, compact rounded body and editorial hand-drawn ink texture. Integrate the magnifying-glass ring naturally as a defining facial feature, not a loose prop held up to the face; small check-shaped beak. Make the character calmly curious and confidently standing, hands relaxed, both feet completely visible. Simplify: retain only a small sage scarf as a consistent accessory, remove bag and decorative marks. Warm off-white paper, black ink, tiny terracotta accent. Truly transparent background, no shadows, no glow, no background objects, full character surrounded by generous transparent margins. Original sophisticated 2D editorial illustration, no text, no 3D.
- Inspection: 1536 × 1024 RGBA; four corners alpha 0; alpha range 0–254; transparent bounds (69, 35, 1500, 994). Full figure visible but transparent margins are not comfortable vertically. Render shows an unintended diffuse halo; edge cleanliness and usable compositing require correction. Do not call this an approved master.
- Next: targeted image edit to remove halo and improve margins, then inspect on contrasting backgrounds and verify alpha. Generate separate comparisons and consistent assets only after a satisfactory master is selected.
