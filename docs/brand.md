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
- App icon (Windows, Linux, macOS): a 1024 canvas, a black plate 824 square at inset 100 with corner radius 185.4, the mark 560 wide in white and centred, the cut showing the plate. `artosia-icon.svg` and `icons/macos/artosia-macos.svg` are the same drawing. A bare mark is no longer an icon: black vanishes on a dark taskbar, so the plate carries it.
- Small icon: under 24 px (16, 20 and 22) the A closes up at 560 units, so those sizes draw `artosia-icon-small.svg`, the same plate with the mark 700 wide. Checked at 800% on the 16, 20, 22, 24 and 32 px renders: at 16, 20 and 22 the counter and crossbar run together at 560 and stay apart at 700; at 24 the crossbar already reads at 560. The .icns 16 px entry is the same drawing (`icons/macos/artosia-macos-16.svg`).
- Mono glyph: a 16-unit square, the mark 16 wide and centred vertically, one path filled with `currentColor`. The counter, slit and notch form one cut that opens at the base, so the glyph is a single outline with no inner subpath and draws the same under the nonzero and even-odd fill rules. The app header draws the mono glyph in the window's text colour.

## Colour

**Decision, 2026-11-12.** The brown mark clashed with the program's neutral window. The owner offered two ways out, a black logo or a program themed in the brand's colours, and the black logo was picked. The mark's meaning is its shape, a capital A cut out of a round loaf; the program's colours already carry meaning (red is live, yellow is a warning or something hidden); and the homepage is ink on white. The four browns and creams the brand had before (Crust, Ink, Crumb and Mid crust; their values are in git history) left the mark and the icons. No coordinate of the mark, the wordmark or the lockups changed.

The logo has two colours and no hue.

| Name | Hex | OKLCH | Use |
|---|---|---|---|
| Black | `#000000` | `oklch(0 0 0)` | Mark, wordmark and both lockups on white and light grounds; the plate of the app icon. |
| White | `#ffffff` | `oklch(1 0 0)` | Mark, wordmark and both lockups on dark grounds; the mark on the plate. |

Contrast, WCAG 2: black on white 21.00:1; white on the app's dark surface `#16181d` 17.76:1; the white mark on the black plate 21.00:1. The plate's edge against a light taskbar `#f3f3f3` is 18.93:1 and against the macOS light Dock `#e8e8e8` 17.14:1. Against a dark taskbar `#202020` it is 1.29:1 (1.18:1 on `#16181d`), so on a dark ground the plate melts into the bar and the icon reads as the white mark, which is 16.29:1 on `#202020`. No outline is added to hold the plate there; that is a do-not.

Black has no hue, so it cannot be taken for the app's red (hue 25) or yellow (hue 92).

## Typography

- Typeface: Vollkorn by Friedrich Althausen, Medium (wght 500), a static instance cut from the variable font. Licence: SIL Open Font License 1.1. Source: https://github.com/FAlthausen/Vollkorn-Typeface (also on Google Fonts).
- The wordmark is "Artosia" in plain Vollkorn Medium outlines, with no letter changed, by owner decision. It ships as outlines, so nothing needs the font installed.
- Spacing: the font's advance widths plus these pair adjustments, in font units of a 1000-unit em: Ar -28, rt -2, to -12, os -7, si +4, ia -8. The font's own kerning is not applied on top.

## Clear space and minimum sizes

- Clear space: a quarter of the mark's width on every side. In a lockup, measure it from the mark's width in that lockup.
- Horizontal lockup: the mark is 2.026 times the wordmark's cap height, stands on the wordmark's baseline, and sits 0.5 cap heights from the A.
- Stacked lockup: the mark is exactly as wide as the wordmark, 0.42 cap heights above it.
- Smallest sizes: mark and app icon 16 px, mono glyph 16 px (the header uses 18 pt), wordmark 80 px wide, horizontal lockup 120 px wide, stacked lockup 72 px wide.

## Files

All in `assets/brand/`. Each PNG is the SVG named beside it, scaled by a plain SVG renderer.

- A shortcut on a PC that already had Artosia goes on showing the icon it first drew, so the program also keeps `artosia.ico` in its install folder under a name made from its bytes and points its shortcuts there ([setup.md](setup.md), "The icon after an update"). A new icon needs nothing more than a new `artosia.ico`.
- `artosia.ico`: frames 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256, transparent, each rendered at its exact size from `artosia-icon.svg`, or from `artosia-icon-small.svg` under 24 px (32-bit frames with alpha below 256, PNG at 256). `crates/artosia/build.rs` embeds it.
- `artosia-icon-{16,24,32,48,64,128,256,512}.png`: the same rule. `crates/artosia/src/main.rs` loads the 256 as the window icon.
- `artosia-icon.svg`: the app icon master, a black plate with the white mark. `artosia-icon-small.svg`: the same plate with the 700-unit mark, for sizes under 24 px.
- `icons/macos/artosia-macos.svg`: the same drawing as `artosia-icon.svg`; `icons/artosia.icns` is built from it; the 16 px entry draws `icons/macos/artosia-macos-16.svg`, the same drawing as `artosia-icon-small.svg`.
- `icons/macos/artosia-macos-square.svg` and `icons/macos/artosia-macos-square-1024.png`: the same art on a full-bleed black square with no corners, for Icon Composer, which applies its own mask.
- `icons/linux/hicolor/<n>x<n>/apps/artosia.png` for 16, 22, 24, 32, 48, 64, 128, 256, 512: the same rule as the icon PNGs; `scalable/apps/artosia.svg` is `artosia-icon.svg`.
- `artosia-mark.svg` (black), `-on-dark` (white), `-black`, `-white`; `artosia-mark-mono.svg` (16 by 16, `currentColor`).
- `artosia-wordmark.svg` (black), `-on-dark` (white), `-black`, `-white`.
- `artosia-lockup-horizontal.svg` and `artosia-lockup-stacked.svg`, each in the same four versions. Since the logo went black, the plain file and `-black` draw the same thing, and so do `-on-dark` and `-white`; the pairs stay so that every path that names one still works.
- `png/black/`, `png/white/`: the mark at 16, 32, 64, 128, 256, 512, 1024 px wide; the wordmark and both lockups at 256, 512, 1024, 2048 px wide.
- `explorations/`: `candidates.png` and `compare.png`.

## Do not

- Colour the mark. It is black or white. The app keeps red for live and yellow for a warning or something hidden.
- Fill the A or close the notch. The cut stays open to whatever is behind it.
- Put the black mark on a dark ground (it is 1.18:1 on `#16181d`) or the white mark on a light one. Plain files go on light grounds, `-on-dark` files on dark ones, and the plate carries the icon on any ground.
- Draw a ring around the circle, outline the mark, or put it inside a second shape, except the icon's plate.
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

It rewrites every PNG under `assets/brand/` (icons, `png/`, `icons/linux/`, `icons/macos/artosia-macos-square-1024.png`), `artosia.ico` and `icons/artosia.icns` from the SVGs. Icon sizes under 24 px draw `artosia-icon-small.svg` and every other size draws `artosia-icon.svg`; a macOS size with its own `icons/macos/artosia-macos-<n>.svg` draws that file. The tool holds no colour: the fills live in the SVGs. The SVGs themselves were generated from a construction spec outside the repo; edit them by hand only with the numbers in Construction.

## Checked for look-alikes

Blind reviewers read circle b first as a setting sun, then a mountain, then an A, and scored it 6 of 10 for professionalism. The A survives at 16 px. The circle-A echo is noted: the anarchy sign is also an A in a circle. The loaf read was traded for small-size legibility by owner choice. This is not a trademark search.
