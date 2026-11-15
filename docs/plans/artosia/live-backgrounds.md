# Live backgrounds (LIVEBG1)

The owner asked on 2026-11-13: "Where are the live wallpapers like kaleidoscope and others?"
A live background moves and is drawn by the graphics chip while it shows, with no video file.

## What others do (research, 2026-11-15)

Notes: `~/Shared/inspo/artosia-live-backgrounds/notes.md`.

| Product | Live backgrounds | Drawn by |
|---|---|---|
| FreeShow (GPL-3.0) | 26 effects: stars, galaxy, rain, snow, bubbles, aurora, bloom, fog, rays, spotlight, fireworks, candle, wave, mesh gradient and more; a Kaleidoscope visualizer of 8 mirrored segments driven by the sound | Canvas 2D on the processor, about 30 frames a second |
| ProPresenter, MediaShout, Proclaim, Holyrics, Quelea | Video loops | Pre-rendered files |
| OpenLP | Solid, gradient, picture | Nothing moves |
| Wallpaper Engine, Lively | Shader effects | The graphics chip |

The kinds people expect most: flowing gradient, aurora, particles (stars, snow), light rays,
waves, fog and glow, rain, sound shapes. Only FreeShow names a kaleidoscope.

## Why the graphics chip

A 1080p video background takes the projector from 142 MB to about 300 MB working set
(docs/output.md, "Measured"), a decoder and its frames. A shader costs a few small buffers and
one draw per frame. Rune's own effect
crate, `rune-fx` at the pinned rev, holds tested fills drawn this way.

Measured with `rune-fx`'s gallery (`--timings`) on this PC, milliseconds of graphics time per
draw of 543,696 pixels (a 1080p screen is 2,073,600):

| Fill | Intel Iris Xe | NVIDIA RTX 4060 Laptop | At 1080p on Iris Xe, scaled |
|---|---:|---:|---:|
| Mesh gradient, 5 colours | 1.157 | 0.092 | about 4.4 |
| Waves, 5 bands | 1.024 | 0.082 | about 3.9 |
| Metaballs, 6 | 1.242 | 0.088 | about 4.7 |
| Swirl | 1.706 | 0.158 | about 6.5 |
| Voronoi (stained glass) | 1.604 | 0.141 | about 6.1 |
| Aurora (960,000 px) | 1.100 | 0.163 | about 2.4 |
| Sparkles x9 (960,000 px) | 0.729 | 0.100 | about 1.6 |

A frame at 60 a second has 16.7 ms. An older Intel HD chip is slower than Iris Xe; the real
cost is measured on the projector with `tools/ram/measure-cpu.ps1` before this ships.

## Plan

```
BASE    a background that draws itself: Background::Live { kind, colours, speed }, one more kind
        beside Solid, Gradient, Image, Video and Camera in artosia-slides/look.rs
BOUNDARY artosia-slides draws an effect slot (rune::effect) carrying the kind, colours and
        speed; artosia-warp's live module paints every slot with rune-fx; the projector's
        WarpCompositor and a plain compositor for the operator window both call it
TREE    audience::background -> a gradient of the first two colours (seen where nothing paints
        the slot) + effect slot over it
DATA    look.background (saved with the look) -> audience view -> slot mark (kind) and data
        (three colours packed as whole numbers, speed) -> compositor at each frame
STATE   no new state in App beyond the look; the compositor keeps one FxRenderer per device
MOVES   the fill moves by Rune's clock; a tile, a plan thumbnail and reduced motion draw one
        still pose and ask for no frames
PICK    Design, background row: a Live choice with one moving tile per kind, a row of colour
        sets (picked, never typed), speed as Slow, Normal, Fast
KINDS   Flowing colours (mesh gradient), Aurora, Ink (swirl), Sea (waves), Stained glass
        (voronoi), Lava (metaballs), Stars (sparkles over a night gradient), Kaleidoscope
        (Artosia's own shader: a slow fill folded into mirrored wedges)
COST    RAM of artosia-output with a live background against the still and video numbers;
        graphics time on the Intel chip at 1080p
EDGES   no graphics chip effects (software adapter): the gradient under the slot shows;
        a look saved by an older version reads as before
```
