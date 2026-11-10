# Artosia brand

## The mark in one line

A capital A cut out of a round loaf: a triangular counter, a notch through the base and one slit that breaks the crossbar, inside a circle that stands on a flat base.

## Meaning

**Sourced.** Ancient Greek ἄρτος (ártos) means a loaf of wheat bread, and collectively bread. Wiktionary gives its origin as Proto-Hellenic *ártos*, "of unknown further origin", probably a substrate word, and relates it to Mycenaean *a-to-po-qo*, "baker" (en.wiktionary.org/wiki/ἄρτος, fetched 2026-11-10). The Greek New Testament uses it for the loaf at the last supper: καὶ λαβὼν ἄρτον εὐχαριστήσας ἔκλασεν (Luke 22:19, SBLGNT via BibleGateway). English "artos" is the leavened loaf blessed in Eastern Orthodox services (en.wiktionary.org/wiki/artos). The Perseus LSJ page returned 403, so no lexicon entry beyond Wiktionary was read.

Matthew 4:4, where Jesus quotes Deuteronomy 8:3, puts bread and word in one sentence: Οὐκ ἐπ’ ἄρτῳ μόνῳ ζήσεται ὁ ἄνθρωπος, ἀλλ’ ἐπὶ παντὶ ῥήματι ἐκπορευομένῳ διὰ στόματος Θεοῦ (Nestle 1904, biblehub.com/text/matthew/4-4.htm, fetched 2026-11-10), "Man shall not live by bread alone, but by every word that comes from the mouth of God" (ESV, BibleGateway, fetched 2026-11-11). ἄρτῳ is the dative of ἄρτος (Strong's 740); ῥήματι is the dative of ῥῆμα, a spoken word (Strong's 4487).

**From the owner's brief.** The concept is a capital A cut through a loaf; the name comes from ártos, and the letter breaks the loaf.

**Our reading, not sourced.** "Artosia" is not a Greek word and no entry for it was found, so it is an invented name. We read it as ἄρτος plus a common place ending, "a place of bread". Nothing in the repo records what the owner meant by the name beyond the brief above; the owner should confirm or replace this reading. Matthew 4:4 is our addition: the app's work is the second half of that verse, the words, put on the wall for a room to sing.

**What to say.** "Artosia comes from the Greek ártos, bread. The A is cut out of a round loaf. In Matthew 4:4 we live not by bread alone but by every word from the mouth of God, and the app puts those words on the wall."

## Construction

All numbers are in mark units, the mark 1000 units wide, read from the construction spec.

- Outline: one circle, radius 500. Its lowest 190 units are shaved off, which leaves a flat base 784.6 wide. The base corners turn 51.7°. The mark is 810 tall, so its width is 1.235 times its height.
- Letter: the A is the black shape. Its inner edges run at 68° from the base line; Vollkorn's own A leans at 66.4° and 68.4°. The counter's apex sits at 720, 90 below the top.
- Crossbar: black, 100 thick, from 265 to 365.
- Slit: 104 wide (10.4% of the width), parallel to the right leg, cutting the crossbar's right end. Slit to crossbar is 1.04.
- Notch: the counter's two edges continue down through the base, which leaves two flat feet, each 101.4 wide.
- Measured on the finished path: 1 radius; straight edges only at 0° and ±68°; every corner hard (51.7°, 68°, 112°, 136°); thinnest feature 9.0% of the width; ink 0.724 of the convex hull; slit width spread 0.000.
- App icon (Windows, Linux): a 1024-unit square, the mark 896 wide with a 64-unit margin, in Mid crust, the A left transparent. One SVG serves every size; no size needed its own drawing.
- macOS icon: a 1024 canvas, a Crust plate 824 square at inset 100 with corner radius 185.4, the mark 560 wide in Crumb. In the .icns, the 16 px entry uses a 700-unit mark (`icons/macos/artosia-macos-16.svg`), because at the master size the A closed up to a smudge.
- Mono glyph: a 16-unit square, the mark 16 wide and centred vertically, one path filled with `currentColor`. The counter, slit and notch form one cut that opens at the base, so the glyph is a single outline with no inner subpath and draws the same under the nonzero and even-odd fill rules. The app header draws the mono glyph in the window's text colour.

## Colour

| Name | Hex | OKLCH | Use |
|---|---|---|---|
| Crust | `#6a3913` | `oklch(0.399 0.085 54.4)` | The mark on white and light grounds; the macOS icon plate. |
| Ink | `#251e16` | `oklch(0.241 0.018 70.5)` | The wordmark on white and light grounds. |
| Crumb | `#f0e7d6` | `oklch(0.930 0.025 83.4)` | Mark and wordmark on dark grounds; the mark on the macOS plate. |
| Mid crust | `#ad6e2f` | `oklch(0.594 0.111 62.4)` | The app icon on Windows and Linux, which must hold on light and dark taskbars. |

Contrast, WCAG 2: Crust on white 9.51:1; Ink on white 16.46:1; Crumb on the app's dark surface `#16181d` 14.47:1; Crumb on Crust 7.75:1. Mid crust on a light taskbar `#f3f3f3` 3.75:1, on a dark taskbar `#202020` 3.92:1, on `#16181d` 4.27:1, on the macOS light Dock `#e8e8e8` 3.40:1. Mid crust meets the 3:1 level for graphics everywhere and is not for text.

Mid crust moved from `#b77534` (oklch 0.620 0.115 62.2) to `#ad6e2f` (oklch 0.594 0.111 62.4). The old value gave 3.37:1 on a light taskbar and 4.35:1 on a dark one; the new one gives 3.75:1 and 3.92:1, so the worse of the two rises, and the lower lightness reads as baked crust instead of orange. The other three colours are kept as they were.

The app uses red at hue 25 for live and yellow at hue 92 for warning. Crust (hue 54) and Mid crust (hue 62) sit between the two at low chroma and read as brown. Crumb's hue is close to the warning hue, but its chroma is 0.025, near grey, so it reads as cream.

## Typography

- Typeface: Vollkorn by Friedrich Althausen, Medium (wght 500), a static instance cut from the variable font. Licence: SIL Open Font License 1.1. Source: https://github.com/FAlthausen/Vollkorn-Typeface (also on Google Fonts).
- The wordmark is "Artosia" in plain Vollkorn Medium outlines, with no letter changed, by owner decision. It ships as outlines, so nothing needs the font installed.
- Spacing: the font's advance widths plus these pair adjustments, in font units of a 1000-unit em: Ar -28, rt -2, to -12, os -7, si +4, ia -8. The font's own kerning is not applied on top.
- Vollkorn has no polytonic Greek. Greek quotations in the brand kit PDF are set in Times New Roman, embedded; its embedding licence is unchecked, so swap it for an OFL face before the PDF is published.

## Clear space and minimum sizes

- Clear space: a quarter of the mark's width on every side. In a lockup, measure it from the mark's width in that lockup.
- Horizontal lockup: the mark is 2.026 times the wordmark's cap height, stands on the wordmark's baseline, and sits 0.5 cap heights from the A.
- Stacked lockup: the mark is exactly as wide as the wordmark, 0.42 cap heights above it.
- Smallest sizes: mark and app icon 16 px, mono glyph 16 px (the header uses 18 pt), wordmark 80 px wide, horizontal lockup 120 px wide, stacked lockup 72 px wide.

## Files

All in `assets/brand/`. Each PNG is the SVG named beside it, scaled by a plain SVG renderer.

- `artosia.ico`: frames 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256, transparent, each rendered from `artosia-icon.svg` at its exact size (32-bit frames with alpha below 256, PNG at 256). `crates/artosia/build.rs` embeds it.
- `artosia-icon-{16,24,32,48,64,128,256,512}.png`: from `artosia-icon.svg`. `crates/artosia/src/main.rs` loads the 256 as the window icon.
- `artosia-icon.svg`: the app icon master. No size has its own file.
- `icons/macos/artosia-macos.svg`: macOS art on a 1024 canvas with its plate; `icons/artosia.icns` is built from it; the 16 px entry uses a 700-unit mark (`icons/macos/artosia-macos-16.svg`).
- `icons/macos/artosia-macos-square.svg` and `icons/macos/artosia-macos-square-1024.png`: the same art on a full-bleed square with no corners, for Icon Composer, which applies its own mask.
- `icons/linux/hicolor/<n>x<n>/apps/artosia.png` for 16, 22, 24, 32, 48, 64, 128, 256, 512: from `artosia-icon.svg`; `scalable/apps/artosia.svg` is that file.
- `artosia-mark.svg` (Crust), `-on-dark` (Crumb), `-black`, `-white`; `artosia-mark-mono.svg` (16 by 16, `currentColor`).
- `artosia-wordmark.svg` (Ink), `-on-dark` (Crumb), `-black`, `-white`.
- `artosia-lockup-horizontal.svg` and `artosia-lockup-stacked.svg`, each in the same four versions.
- `png/full-color/`, `png/black/`, `png/white/`: the mark at 16, 32, 64, 128, 256, 512, 1024 px wide; the wordmark and both lockups at 256, 512, 1024, 2048 px wide.
- `explorations/`: `candidates.png` and `compare.png`.
- `brand-kit.pdf`: this document, designed.

## Do not

- Colour the mark red, orange-red or yellow. The app keeps red for live and yellow for warning.
- Fill the A or close the notch. The cut stays open to whatever is behind it.
- Put Crust on a dark ground (it is 1.87:1 on `#16181d`); use Crumb there.
- Set text in Mid crust.
- Draw a ring around the circle, outline the mark, or put it inside a second shape, except the macOS plate.
- Stretch, rotate or crop the mark, or round off its flat base.
- Redraw, track or swap a letter of the wordmark, or set it in another typeface.
- Add a cross, dove, flame, halo, rays, an open Bible, praying hands, wheat, a gradient or a shadow.

## Directions not chosen

`explorations/candidates.png` shows three constructions on a low loaf: a, the score, a white A of slits through the loaf; b, the break, the same black A as this mark; c, crust and crumb, a white band with no crossbar, which read as a tent. `explorations/compare.png` sets b on the loaf against b in this circle at equal area. On the loaf the A closed up at 16 px; in the circle it stays readable, so the owner chose the circle.

## Rebuild

The SVGs are the masters. From the repo root (a standalone tool, outside the workspace):

```
cargo run --release --target-dir target/agent-logo --manifest-path assets/brand/tool/Cargo.toml
```

It rewrites every PNG under `assets/brand/` (icons, `png/`, `icons/linux/`), `artosia.ico` and `icons/artosia.icns` from the SVGs. A size with its own `artosia-icon-<n>.svg` or `icons/macos/artosia-macos-<n>.svg` draws that file; every other size draws the master. The SVGs themselves were generated from a construction spec outside the repo; edit them by hand only with the numbers in Construction.

## Checked for look-alikes

Blind reviewers read circle b first as a setting sun, then a mountain, then an A, and scored it 6 of 10 for professionalism. The A survives at 16 px. The circle-A echo is noted: the anarchy sign is also an A in a circle. The loaf read was traded for small-size legibility by owner choice. This is not a trademark search.
