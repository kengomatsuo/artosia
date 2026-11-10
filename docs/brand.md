# Artosia brand

Files are in `assets/brand/`. The mark is a loaf of bread with three cuts in its crust.

## Meaning

**Sourced.** Ancient Greek ἄρτος (ártos) means a loaf of wheat bread, and collectively bread. Wiktionary gives its origin as Proto-Hellenic *ártos*, "of unknown further origin", probably a substrate word, and relates it to Mycenaean *a-to-po-qo*, "baker" (en.wiktionary.org/wiki/ἄρτος, fetched 2026-11-10). The Greek New Testament uses it for the loaf at the last supper: καὶ λαβὼν ἄρτον εὐχαριστήσας ἔκλασεν (Luke 22:19, SBLGNT via BibleGateway). English "artos" is the leavened loaf blessed in Eastern Orthodox services (en.wiktionary.org/wiki/artos). The Perseus LSJ page returned 403, so no lexicon entry beyond Wiktionary was read.

**Our reading, not sourced.** "Artosia" is not a Greek word and no entry for it was found, so it is an invented name. We read it as ἄρτος plus a common place ending, "a place of bread". Nothing in the repo records what the owner intended; the owner should confirm or replace this.

**What to say.** "Artosia comes from the Greek ἄρτος, the loaf Jesus broke at the last supper, and the mark is that loaf with a baker's cuts. The three cuts also look like three lines of lyrics, which is what the app puts on the wall."

## Construction

- Square grid of 64 units (`viewBox="0 0 64 64"`).
- Loaf: a dome 56 units wide and 33 tall (about 5:3), flat base with 4-unit rounded corners, centred on the grid.
- Cuts: three round-capped slashes, 4.6 units thick, leaning right, 10 to 11 units apart; the middle one is longest. They are real holes (an SVG mask), so the mark works on any background.
- Icon: plate `#16181d`, 62 units square with radius 14, the mark at 92% of its size.
- Mono glyph: 16-unit grid, outline loaf and two cuts, 1-unit stroke, drawn like the icons in `theme.rs`. Two cuts instead of three, because three turn to mush at 16 px.
- Wordmark: "Artosia" drawn as strokes (no font needed), 3.2-unit monoline with round caps and joins, next to the mark.

## Colour

| Name | Hex | OKLCH | Use |
|---|---|---|---|
| Blue | `#5599f3` | `oklch(0.68 0.151 256)` | the mark on dark; the app accent |
| Deep blue | `#276ed2` | `oklch(0.55 0.170 258)` | the mark on white or light grey |
| Plate | `#16181d` | `oklch(0.21 0.010 268)` | app icon background |
| Light blue | `#9ec7fe` | `oklch(0.82 0.090 256)` | direction B only |
| Ink | `#14161a` | `oklch(0.20 0.009 264)` | wordmark text on light |
| Paper | `#f3f5f8` | `oklch(0.97 0.005 258)` | wordmark text on dark |

Blue is the app's own accent. The brand uses no red (live) or yellow (warning).

## Clear space and size

- Clear space: 8 grid units (an eighth of the mark's width) on every side.
- Smallest sizes: icon 16 px, mark 16 px, mono glyph 16 pt, wordmark 96 px wide.

## Do not

- Add a cross, a circle around the loaf, rays, a halo or a gradient.
- Fill the cuts with a colour; they must stay open.
- Recolour the mark red or yellow.
- Squash, rotate or outline the mark, or set it on a busy photo.
- Use the blue on white; use the deep blue.
- Set the wordmark in another typeface.

## Checked for look-alikes (2026-11-10)

Compared by eye with the real icons of FreeShow and OpenLP (their repositories) and with the site icons of Planning Center, YouVersion, Proclaim and Renewed Vision (a lettered "R", which is the company's icon, not ProPresenter's app icon). FreeShow is nested magenta rounded rectangles, OpenLP a blue disc of light rays, Planning Center a speech-bubble "P", YouVersion a red Bible, Proclaim a green speech-bubble "P". None share a shape with a cut loaf. EasyWorship's icon (16 px only) and ProPresenter's own app icon were not seen at usable size. This is not a trademark search.

## Directions not chosen

`assets/brand/explorations/`: B, a round loaf broken in two; C, three stacked bars forming a dome, as lines of text. `sheet.png` shows all three at 256, 32 and 16 px on dark and white (the 16 px is also enlarged 4x).

A won because it says "bread" at once, survives 16 px with its cuts still visible, and carries the second reading (lines of lyrics). B is a plain disc that could be a moon or a pill. C is clean but reads as a stack or menu icon.

## Rebuild

From the repo root (a standalone tool, outside the workspace):

```
cargo run --release --target-dir target/agent-logo --manifest-path assets/brand/tool/Cargo.toml
```

It rewrites the eight `artosia-icon-<n>.png` files, `artosia.ico` (16, 24, 32, 48, 64, 256) and `explorations/sheet.png` from the SVGs.
