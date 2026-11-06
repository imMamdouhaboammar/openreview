# Rovi Motion Language — v1

## Principle
Rovi's motion signals attention and investigation; never implies that a speculative bug is proven or that a fix has shipped. Do not let the mascot obscure source lines, buttons or critical status labels.

## Reusable patterns
| Moment | Action | Timing | State semantics |
| --- | --- | --- | --- |
| Code attention | Tilt head by up to 4 degrees | 240–420 ms, ease-in-out | Not a conclusion |
| Draft comment | Slide a paper note by 24 px | 220–360 ms, ease-out | Draft, not confirmed |
| Review demo | Slow character tilt plus note entrance | 3.8 second illustrative loop | Demo only |
| Running tests | Calm pulse | 1.8 seconds | Keep explicit RUNNING or NOT RUN |
| Verified outcome | One-shot pleased expression | 200–400 ms | Only after real test evidence |
| Idle | Static rest or occasional blink | 200 ms | No busy decorative loop |

## Implementation
Source: motion/rovi-motion.svg, created by tools/build_gallery.py.
The outer SVG group owns translation and scale. A nested child group owns a small transform-origin:center animation with transform-box:fill-box, preventing rotation from removing the absolute position. No additional libraries or animation runtime required.

CSS prefers-reduced-motion:reduce turns off all three animation targets, leaving meaningful static text.

## Verification
tools/test_gallery.py runs on local macOS Chrome and tests normal and reduced-motion, bounding boxes in 680×520, animation-name and change over time. Initial failure caught a broken transform: Rovi moved to x≈49/y≈36 instead of its right-side position. Following repair, normal position started at (x≈426.67,y≈127.54,w≈133.43,h≈208.34) and after 1.4 s changed to x≈419.51/y≈123.15. Reduced-motion position stayed fixed at x≈426.67/y≈127.54 and had no transformation change. Visual live-browser screenshot at motion/motion-live-no-preference.png was inspected.

Demo looping is not a production recommendation near reading surfaces. Production integration should use state-triggered one-shot animation after its own usability testing.
