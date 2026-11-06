# Character illustration research — Capy, Contra, launch storytelling
2026-11-04. **Reference-only study**, not reproduction.

## Evidence status
- Contra exact requested URL was opened on local Chrome with HTTP 200 and a project title identifying Razvan Vezeteu. Initial screenshots at 0/25/50/75/95% scroll and mobile are in research/screenshots/creative/. The first pass found lazy-loading blank areas. A second progressive scroll captured additional screenshots named contra-loaded-*.jpg and recorded 26 image elements with natural image widths, 26/26 loaded successfully (see research/contra_image_inventory.json). Observed page height ~9508 px; DOM images were loaded, although no claim is made that each image was inspected at full resolution.
- Contra source: https://contra.com/p/g37kMveR-capyai-branding-illustrations-project. The public text states the project by Razvan Vezeteu includes sketch-to-high-fidelity work, product illustrations and physical mockups. Browser evidence now confirms a comic-style exploration sheet, a capybara hanging from a rope, a boat/sail scene, a capybara studying among stacked books and coffee, and physical-product mockups including an illustrated plate and shirt. The images establish how a repeatable body silhouette can survive wildly different props, scene scales and physical surfaces.
- Launch case study exact URL https://www.pattytanch.com/work/capy-ai returned HTTP 404 in local browser, although a cached search result had readable case-study text. No storyboard frames or motion interaction are claimed as inspected firsthand. The public cached text states the team chose full-motion illustration, tested pacing through slideshows, and rebuilt product UI from the live app when Figma lagged.
- Main Capy website: Chrome actually loaded 82 sitemap marketing URLs. See CAPY_SITE_AUDIT.md and capy_inventory.json; desktop/mobile screenshots are stored independently.

## Visual principles (verified vs interpretation)
1. **Identity through silhouette:** Capy establishes a stable capybara outline, a simple eye/muzzle grammar, and recognizable accessories. Transfer only the principle of a stable outline to Rovi; Rovi retains its original folded-page asymmetry, lens eye, and check facial geometry.
2. **Sketch to production:** Contra describes sketching → HiFi and product mockups, supporting a reusable character rather than unrelated one-off illustrations. OpenReview solution: one Python-generated Rovi rig, shared head path, reusable anatomy and SVG source.
3. **Expressions:** Minimal eyebrow, eyelid, mouth and body tilt carry emotional distinction. OpenReview expression rules favor attentive and uncertain faces; enthusiasm is reserved for verified outcomes.
4. **Pose semantics:** Accessories and physical UI states explain action without generic robot motifs. Rovi can read a PR page, point at highlighted code, add a comment, watch test output. A Buglet means a *hypothesis* until evidence confirms a bug.
5. **Backgrounds:** Public Capy pages pair bold display typography and very white space with discrete character moments, frequently near an edge or alongside screenshots. Copy the clarity rule, not Capy's placement or layout; OpenReview uses editorial diff paper and warmer colors.
6. **Physical object world:** Proposed Rovi-world objects: paper code cards, margin annotation tags, code line rulers, small test outcome chips, minimal reproduced examples. All objects derived from OpenReview concepts; no duplicate of Capy props.
7. **Motion:** Suggest modest glance/tilt/annotation entry of 180–420ms for interactions, longer idle 3–5s, no constant bounce. CSS animation prototype is a loop solely for demonstration. Reduced-motion mode removes movement.
8. **Product UI integrity:** The cached Patty description emphasizes reconstructing product moments from the actual shipped interface. For OpenReview, do not fabricate an app UI: illustrate abstract PR and test patterns, clearly marked illustrative.
9. **Editable artifact:** Keep live SVG paths and text, not only rasterized scenes. Export images for use but preserve source files and generator.
10. **Originality boundary:** No capybara silhouette, glasses, exclusive comic staging, palette, or mockup compositions. Rovi must read as an independent inspector made of a folded page and lens eye.

## Constraints / future art review
- Initial lazy-load blanks were addressed by a progressive re-scroll; the 26 DOM images are now loaded. Individual full-resolution image-by-image art-direction annotation remains outside this first audit, though the complete page was navigated and multiple positions visually checked.
- Patty page could not be visually audited from the exact provided URL (404). Treat public search summary as secondary textual evidence, not primary imagery.
- Full motion timing of Capy was not measured directly. Recommendations above are proposed for OpenReview, not claimed Capy measurements.
- Distinctness review must compare actual Rovi screenshots against actual Capy page examples. No third-party illustration files are copied into production.
