# Season marks and the missing seasonal colourways (SEASONMARK1)

Designer's plan, 2026-11-15, for card SEASONMARK1 (parent SEASONS1, `docs/church-year.md`; follows TPLCOLOURS1). Status: design for the owner to approve. No source code was changed; the only files written are under `docs/plans/artosia/seasons/` and this page.

Files, all under `docs/plans/artosia/seasons/`:

| File | What it is |
|---|---|
| `marks/<name>.svg` (16) | the twelve marks and the four Advent weeks, 24 unit grid, `currentColor` only |
| `marks/icons.rs.txt` | the same sixteen bodies as `Icon { view: 24.0, .. }` lines for the builder to paste; not compiled from here |
| `contact-sheet.png` | the twelve marks (and the four Advent weeks) at 160 px, and four families in Christmas and Easter with the mark placed |
| `marks-sizes.png` | all sixteen marks at 48, 64, 96 and 160 px on a light and a dark ground, and a stroke weight test |
| `designs-compare.png` | design A against design B on the same two cards |
| `colourways-sheet.png` | all 17 families by five seasons: the {{n_new}} new colourways beside the 25 already tagged |
| `wireframes/*.svg` | the ten wireframes at 1920 x 1080 (eight for the contact sheet, two for design B) |
| `colourways/d1.json` to `d17.json` | the new colourways of each family, ready to paste into its `colourways` list |
| `spec.json` | the build spec: grid, every glyph body, every placement, every palette and ratio, every wireframe element |
| `build/` | `glyphs.py`, `palettes.py`, `spec_data.py`, `wire.py`, `build.py`, `docs.py`, `check.py` and this page's template |

Renderer: resvg-js 2.6.2-beta.1 (`bunx @resvg/resvg-js-cli`, system fonts off, the bundled faces of `crates/artosia-slides/assets/warta/fonts`). Every sheet is composed as SVG and rendered once; no raster editor. Text widths in the wireframes come from fontTools advance widths without kerning, so a kicker's end is within a few percent of the engine's. The marks were rendered by resvg, not by Rune's own icon painter; `check.py` keeps them inside the subset that painter accepts (`crates/rune/src/text/icon.rs`) and the card's first test renders them through `icon_coverage`.

## 1. What the references say

The brief's finding holds on every cover read. Each source below is a file under `~/Shared/inspo/artosia-templates/seasons/` (row numbers are those of `notes.md`).

- GKI Bromo, Advent 1 and 2 (`yt-4Nzs8kJy2YU.jpg`, `yt--5cdDw3leRQ.jpg`, rows 8 and 9): one white candle icon after "MINGGU ADVEN 1", two the next week. The icon is 28 px on a 720 px cover (3.9%), as high as the caps beside it. The season's colour is the purple strip at the right quarter; the title and date stay plain type.
- GKI Kebayoran Baru, year start (`yt-3frYjkCRTs0.jpg`, row 164): a gold sparkle and a scalloped border on cream. The ornaments sit at the edge and the title is untouched.
- HKBP Tanjung Priok, Pentecost (`yt-SaBMHGmaESI.jpg`, row 129): a flame 4% of the slide stands in for the letter o. The whole motif is one swapped letter.
- The ash cross (`cvs-ash-wednesday-ministry-graphics.jpg`, row 46): the cross alone is the mark and the season.
- GBI Galaxi, Good Friday (`yt-FZXgpO9QgOY.jpg`, row 84): a crown of thorns behind the title. A mark cannot sit behind Artosia's data, so ours puts the crown at the foot of the cross.
- GKR Gedong (`yt-sRCenena5Kg.jpg`, row 156): wheat beside a thin frame, restrained. HKBP Taman Mini, Easter (`yt-TXsWxiCvxH0.jpg`, row 105): a lit cave mouth; the picture holds a robed figure, which our tomb leaves out.
- The over-full end, kept as counter-examples: HKBP Jambi (row 74), HKBP Jakarta (row 128), GKI Zaenal Zakse (row 73): many motifs and bank rows on one picture.

So a season is carried by three small things: a sign of about 4% of the height, a colour (a strip, a ground, an accent), and the data left clean. Artosia's colourways already carry the colour; this card adds the sign and the colours that are missing.

The brand page `docs/brand.md` is the Core Brand and was read first. Its Do not list (no cross, dove, flame, halo, open Bible, wheat in the mark) governs the Artosia logo, its wordmark and its lockups. These season marks are slide content: they never appear in a lockup or on the app icon, and they take palette roles, not the logo's black and white. No new brand page was written.

## 2. Two designs under two pulls

Both were drawn on the same two cards (`designs-compare.png`).

![designs compared](seasons/designs-compare.png)

Pull A is "keep the data clean": a 48 px sign beside the kicker, the colourway carries the colour. Pull B is "make the season unmistakable from the back of the hall": a 160 px emblem in a corner, as the church packs do.

<!-- gen:compare -->
<!-- /gen:compare -->

Recommendation: A. B is not wrong, it is a poster size: it fits where the card has an empty corner (d11 and d2) and the church packs that use it carry no table. A keeps the six kinds as they are and costs one line of the kicker.

## 3. The twelve marks

![the marks](seasons/contact-sheet.png)

**One line style.** A 24 unit grid with the live area from 2 to 22. Every outline is 2 units, round caps and joins, no fill. Two kinds of thing are solid, in the same colour: a light (flame, star, spark, the half sun) and anything under 5 units across (candle bodies, wheat grains). Nothing else is filled. There is one colour (`currentColor`), no opacity, no gradient, and only `path`, `rect`, `circle` and `ellipse` with the commands M L H V C Q A Z, which is what Rune's icon painter accepts. The weight was chosen on `marks-sizes.png`: 1.5 units is 3 px at 48 px beside a 32 px label whose stems are about 4.5 px, so it reads light; 2.5 closes the thorn crown, the wheat and the Bible's lines; 2 keeps every counter open.

**No faces, no figure.** The dove is the only creature. The tomb is empty. The Ascension has a cloud and a light, no feet and no figure. Nothing carries a halo, rays round a cross or a pair of sparkles.

<!-- gen:marks -->
<!-- /gen:marks -->

Hard choices, in the order they were met:

- Christmas is an eight point star, not five points (a five point star reads as a rating) and not the four point sparkle (it reads as a magic or AI sign today). New Year takes that sparkle alone, with no second one beside it. Epiphany and Ascension carry the same four point light, small over a book and tall over a cloud, so "light" has one shape across the set.
- Good Friday went through more than ten crowns. A ring of thorns fused with the cross read as a badge, a tilted ellipse read as an eye, a spiky band read as a zigzag. A flat ring with five thorns on its far edge and the cross standing in it reads as both objects at 48 px.
- Reformation is the open Bible with its lines of text. A cross standing on the book was drawn and dropped: it made Reformation and Epiphany the same shape (a book with something above it).
- Easter went through several tombs. A block with a door read as a house. The hill with the open door, the stone rolled aside and the sunrise reads as an Easter morning to a congregation that knows the story, and as a hut with a sun to one that does not. The sunrise is what keeps it from being a hut.
- Known weakness, shown on `marks-sizes.png`: at 48 px the Pentecost flame is a speck and the palm is a leafy stroke. The bird carries the first, the branch's diagonal the second. If the owner wants more at 48 px, the flame grows to 6 units wide and the dove loses its second wing line; that is a one line change in `build/glyphs.py`.

## 4. Which season draws which mark

<!-- gen:season_mark -->
<!-- /gen:season_mark -->

- **Advent's week.** {{advent_week_rule}}. In 2026 the Sundays are 29 November, 6, 13 and 20 December.
- **Harvest and New Year** are not seasons of the engine (`Season` has eleven values, none of them these), so they are marks a person picks by hand. This card draws them and does not add a picker; the owner decides in section 9.
- **Epiphany, Holy Week, Good Friday, Ascension and Reformation** have a mark but no colourway (the brief names five seasons). Until they have one, a family without a tagged colourway keeps its own, as `colourway_for_season` does now.

## 5. Where each family puts its mark

{{n_placed}} of the 17 families carry a mark; d7, d8 and d9 do not. A mark never changes the six kinds' data: its colour role and ground are a pair the family already lists, so the existing contrast test guards it in every colourway, old and new.

<!-- gen:placement -->
<!-- /gen:placement -->

- **One new primitive.** The renderer's `Sheet` holds `Item::Rect`, `Item::Image` and `Item::Text` (`crates/artosia-slides/src/warta/mod.rs` line 351); `R` has no path and `Pic` is raster. The mark needs one new item, `Item::Mark`, that draws an `Icon` at a position, size and role colour, through the same painter the operator window uses. That is the lowest rung of the creation ladder that holds.
- **"Beside the kicker" means measured.** Every layout already measures its kicker (`sl.text(..)` returns `.w`). The mark is placed at `x = kicker.x + kicker.w + gap` and the kicker's `maxw` is lowered by `size + gap` (72 px) first, so a long church name shortens before the mark is ever dropped.
- **d6 is an exception to its own rule.** `templates-default.md` candidate A says "no icon". The mark is set as a fleuron: the kicker's own colour (`mute`), 9 px clear of the rule, accent left to the highlight. This card amends that clause by one: no icon except the season mark. If the owner refuses, d6 uses the wash alone (section 9).
- **d7, d8, d9 have no room.** Their Never lists ban icons, and d7 and d9 are hard edged; a round capped sign breaks them. Their colourway (block and accent, tinted ground and pill, black band and accent) carries the season. The fallback spot if the owner overrules is in `spec.json` (`placement.fallback_if_overruled`).

## 6. Seasonal colourways

Method. Each season has one paint box in OKLCH, light and dark (`build/palettes.py`, copied to `spec.json` `paint`). A family maps its own palette keys to the box's roles, so a new colourway has the family's keys and its structure. A solver then moves lightness, never hue, until every pair the family lists passes with 0.05 to spare. The gates are the Rust test's: text 4.5, large and mark 3, WCAG 2 luminance with the 0.03928 threshold as `look.rs` computes it.

<!-- gen:paint -->
<!-- /gen:paint -->

Choices that are not obvious:

- **Advent and Lent are both violet in a church, so they differ in chroma and hue.** Advent is a clear violet (hue 306) with candle gold; Lent is ash with mulberry (hue 348) on a ground with almost no tint (chroma 0.007 against Advent's 0.018).
- **Christmas is evergreen with gold on a dark ground, cream with evergreen and holly red on a light one.** Dark families draw it dark: a green ground with gold or cream on it, as GPdI Bulak Sereh and HKBP Menteng do (rows 33 and 37). Red and green are also the two Christmas colours of rows 27 and 28.
- **Easter is ivory and bronze gold on a light ground**, gold on a dark olive ground in the dark families (as d3's existing `paskah`). d11 turns ivory: its vestment ground becomes ivory, its frame deep gold and its bands dark with ivory words, which is the liturgical white and gold. The keys keep their names and their listed pairs; only their lightness flips.
- **Pentecost is red with a flame orange** (hue 27 and 45), light on rose paper, dark on ember.
- **{{n_retag}} of the {{n_new}} gaps are already a colourway the family ships untagged** (swatch distance under 0.05 and the same hue). Adding a copy would put two near identical tiles in the gallery, so for those the recommendation is a one word edit, not a new look. The new palette is still in `colourways/dN.json` as the alternative.

<!-- gen:retag -->
<!-- /gen:retag -->

With the retags, {{n_made}} new colourways are added and the library goes from 105 to {{n_templates}} templates. Append each new colourway at the end of its list, so existing ids and the first colourway (the family's own design) do not move.

<!-- gen:counts -->
<!-- /gen:counts -->

![the colourways](seasons/colourways-sheet.png)

How to read each family below: the snippet is the new colourways in the order advent, christmas, lent, easter, pentecost (gaps only), one object per line, each with a trailing comma; paste them after the last entry of `colourways` (drop the last comma if it ends the list). The table gives the ratio of every pair the family lists, in the same order as its `pairs`.

### d1 Editorial greige

<!-- gen:cw1 -->
<!-- /gen:cw1 -->

### d2 Pill poster

<!-- gen:cw2 -->
<!-- /gen:cw2 -->

### d3 Neon night

<!-- gen:cw3 -->
<!-- /gen:cw3 -->

### d4 Split panel

<!-- gen:cw4 -->
<!-- /gen:cw4 -->

### d5 Simple

<!-- gen:cw5 -->
<!-- /gen:cw5 -->

### d6 Serif italic

<!-- gen:cw6 -->
<!-- /gen:cw6 -->

### d7 Swiss numerals

<!-- gen:cw7 -->
<!-- /gen:cw7 -->

### d8 Quiet Tint

<!-- gen:cw8 -->
<!-- /gen:cw8 -->

### d9 Civic Board

<!-- gen:cw9 -->
<!-- /gen:cw9 -->

### d10 Diagonal

<!-- gen:cw10 -->
<!-- /gen:cw10 -->

### d11 Liturgy

<!-- gen:cw11 -->
<!-- /gen:cw11 -->

### d12 Glass

<!-- gen:cw12 -->
<!-- /gen:cw12 -->

### d13 Block

<!-- gen:cw13 -->
<!-- /gen:cw13 -->

### d14 Cahaya

<!-- gen:cw14 -->
<!-- /gen:cw14 -->

### d15 Magazine

<!-- gen:cw15 -->
<!-- /gen:cw15 -->

### d16 Figures

<!-- gen:cw16 -->
<!-- /gen:cw16 -->

### d17 Warm

<!-- gen:cw17 -->
<!-- /gen:cw17 -->

## 7. The contact sheet

`contact-sheet.png` (shown in section 3) was rendered by resvg-js through `bunx @resvg/resvg-js-cli`, as one SVG. Its top is the sixteen marks at 160 px; below are Serif italic, Neon night, Liturgy and Warm in their Christmas and Easter colourways, each with the mark placed and boxed in red. Serif italic Easter, Neon night Easter and Warm Easter use the families' existing `paskah`; the other five are new. The wireframes use bars for words, real text for the kicker, title and range, and the families' own faces.

## 8. Card SEASONMARK1

**SEASONMARK1** Season marks on the announcement cards and the missing seasonal colourways · source: owner brief 2026-11-15, the finding that real churches mark a season with a small sign and keep the data clean · parent: SEASONS1 · plan: this page · done when: sixteen marks are in the code, 14 families draw their season's mark where section 5 says, every family has all five seasonal colourways, and every pair of every colourway passes the contrast test · verify: `cargo test -p artosia-slides`, `cargo test -p artosia-core`, the snaps named in the proof below. Sections 1, 2, 4, 6 and 8 of `design-questions.md` were opened for this card.

**Contexts:** the account is n/a (the app is free and has no tiers); who acts is n/a (the mark is read only output, no role changes it); language is accepted by AC-6 and AC-21 (the kicker is a fixed word per window language and a church name of any length, so the mark follows a measured end); calendar is accepted by AC-3 and AC-4 (Western dates only, Eastern Easter is not built, `docs/church-year.md`); settings are accepted by AC-16 (Follow the church year on or off, a colourway picked by hand with or without a tag); device is accepted by AC-5, AC-17 and the proof (a projector at 1920 x 1080, a scaled gallery tile, the phone remote and tablet previews that draw a card); data is accepted by AC-6 and AC-7 (no church name, a very long one, a 30 name page, a 13 digit total, no season at all); the client is accepted by AC-20 (an older build opening a library that names a new colourway id); the entry (by hand, by PowerPoint import, by phone) is n/a: every entry draws a card through the one `sheet()` function, which AC-8 pins.

**Neighbours:** nothing here writes a record; the card writes template data and icon constants. Every exit and replay item is n/a: a template is read only data. The readers of the style and colourway lists are walked in AC-13, AC-14, AC-17 and AC-18 (`crates/artosia/src/templates.rs`, `ui/templates.rs`, `pptx_import.rs`, `ui/pptx_import.rs`, `state.rs`, `actions.rs`, `crates/artosia-core/src/store.rs`, `deck.rs`, `crates/artosia-output/src/demo.rs`, `church_year.rs`); saved cards keep their stored `family/colourway` (AC-13); the old client is AC-20; the tests that pin the old counts are named in AC-14.

**Shape (creation pattern).** Rule: `advent_week(date)` and `mark_for(season, date)` in `crates/artosia-core/src/church_year.rs` beside `advent_start(year)` and `season_on(date)`, pure functions with no UI; today's date reaches them only through `civil_today()` (`state.rs` line 1287), the one place that knows the church's zone. Data: a `"mark"` block in each `directions/dN.json` (spot, size, role, gap) beside the new colourways, and the sixteen `Icon` constants from `marks/icons.rs.txt`. Screen: one `Item::Mark` on the `Sheet` and one call per layout page function. Ladder: the Sheet cannot draw a path (rung 2 fails), the platform's icon painter exists (rung 3 holds), so the one new item calls it.

**Acceptance criteria**

Marks
- AC-1. Each of the sixteen `Icon` constants gives non-empty coverage from `icon_coverage` at 48 px, and its ink box lies inside 1 to 23 of 24 units (`check.py` measures 16 of 16 today).
- AC-2. Each body uses only `path`, `rect`, `circle` and `ellipse`, with `fill` and `stroke` either `currentColor` or `none`, and the path commands M L H V C Q A Z.
- AC-3. `advent_week` is 1 for 29 Nov to 5 Dec 2026, 2 for 6 to 12 Dec, 3 for 13 to 19 Dec, 4 for 20 to 24 Dec, and `None` on 28 Nov and 25 Dec; `mark_for(Advent, d)` gives `advent-n` on those days, and `advent` for a card in the advent colourway outside Advent. The Sunday itself belongs to the week it opens, and 24 December is the last day of week 4 (an inclusive end, written once, here).
- AC-3b. The week follows today's date from `civil_today()`, not the date a card is for: a card prepared on Saturday 5 December draws one flame until local midnight and two after. It is tested at 23:59 and 00:00 local on 5 to 6 December and on 24 to 25 December 2026, under two zones (UTC+7 and UTC-5), and one day off each boundary as well as on it.
- AC-4. `mark_for` returns the mark of section 4 for each of the eleven `Season` values (`ordinary` returns `None`); harvest and new year are never returned.

Placement
- AC-5. For each of the 14 families the mark's rectangle (size and centre) in every one of the six kinds equals the table of section 5, measured on the snap at 1920 x 1080 and on the scaled gallery tile.
- AC-6. In the six kinds at the fixtures of `templates-library.md` (8 schedule rows, 30 names and the two month page, 6 funds with a 13 digit total, a 16 digit account with and without a QR, a 60 word verse, a 45 character notice title) the mark's rectangle is at least 8 px from every other drawn text, rule, bar and plate; at the longest church name the kicker shortens first and the mark is still drawn; in each of the ten window languages the same holds.
- AC-7. d7, d8 and d9 draw no mark in any season, and their snaps are pixel equal to the snaps before this card.
- AC-8. With no season (Follow the church year off, a colourway without a tag) every family's snap is pixel equal to the snap before this card; d11 draws its cross again.
- AC-9. d3 draws the mark inside the first ring of the kind's `deco` list with a diameter of 200 or less, at half its diameter (88, 88, 88, 75, 88, 60 px for S01 to S06), in the ring's colour.
- AC-10. d11 draws the mark centred at (960, 92), 56 px, in `gold`, and its two rules still end 40 px from the centre.

Colour
- AC-11. For every family and every colourway, old and new, the mark's role on its ground passes the gate of the pair the family lists for it (`mark_pairs` in `spec.json`: 16 pairs, lowest 3.0 or more).
- AC-12. `every_colourway_of_every_direction_reads_on_every_pair_its_layout_draws` passes with the new colourways ({{n_pairs}} new pair checks; the tightest is 4.99 against a 4.5 gate).
- AC-13. Each pasted colourway has exactly its family's keys and an id not used in that family, is appended at the end of its list, and `colourway_for_season` returns the id of section 6 for advent, christmas, lent, easter and pentecost in all 17 families (85 assertions); an id saved in a library before the change draws the same colours as before.
- AC-14. `a_colourway_picks_the_palette_a_slide_draws_and_an_unknown_one_is_the_first` (it asserts `colourways(NoticeStyle::Neon).len() == 6`) is updated to the new count, and the tests that count templates (105 before, {{n_templates}} after) say so.
- AC-15. The {{n_retag}} retags change only the `season` key of an existing colourway; every palette value in those lines is unchanged.

Follow the church year
- AC-16. One function decides whether a card draws a mark: a colourway tagged with a season draws that season's mark on any date; with Follow the church year on, the season of today draws its mark; with it on in ordinary time and a plain colourway, no mark is drawn.

Neighbours
- AC-17. The gallery rows, the colour picker and the style rail show up to ten swatches of one family at phone, tablet and a small laptop width without clipping, in dark mode and in light.
- AC-18. The PowerPoint import gallery and the phone editor list the new templates; a saved card with an old style id opens unchanged.
- AC-19. The operator window's RAM with the template gallery open is measured with `tools/ram/measure.ps1` before and after, and both figures go in the close note; no figure is claimed here.
- AC-20. A library naming a colourway id this build does not know draws the family's first colourway (the existing test), and an older build ignores the `season` tag it does not read.
- AC-21. The kicker, title and range of each window language are the engine's own words; the mark follows the measured end of the kicker in all ten.

**Mock and proof.** The mock is `docs/plans/artosia/seasons/contact-sheet.png` and the wireframes it is drawn from (`wireframes/serif-italic-christmas.svg`, `serif-italic-easter.svg`, `neon-night-*.svg`, `liturgy-*.svg`, `warm-*.svg`). The proof is the builder's screenshots, not a reading of the code: the phone and tablet screenshots of the template gallery and preview that draw the card, and the 1920 x 1080 snaps of the same eight cards (`cargo run -p artosia-output -- --window audience --scene <the announcement scene> --snap out.png --size 1920x1080 --scale 1`), each laid beside its wireframe and measured against the table below. The "built" columns are left for the builder.

<!-- gen:mocktable -->
<!-- /gen:mocktable -->

**Keyboard on iPhone and Android.** n/a: the mock draws no text field. The card changes projector slides and gallery tiles; the one setting it touches, Follow the church year, already exists and has no field.

## 9. Decisions for the owner

1. **d6 and its "no icon".** Amend the clause by one (no icon except the season mark, in `mute`, beside the kicker), or keep d6 clean and let the wash carry the season. Recommended: amend.
2. **d7, d8, d9 without a mark.** Recommended: keep them clean. If the owner wants a mark everywhere, the fallback spots are in `spec.json`.
3. **The retags.** {{n_retag}} gaps are better served by tagging a colourway that already exists. Recommended: retag. If a family should have a dedicated look, its new palette is in `colourways/dN.json`.
4. **Harvest and New Year.** They have marks and no season. A hand pick in the card's look menu is the smallest way in; the engine has no date for harvest (`church-year.md` lists none), so a date rule would be a guess.
5. **The five seasons without a colourway.** Recommended mapping when the engine gets it: Holy Week and Good Friday use lent, Epiphany uses christmas, Ascension uses easter, Reformation uses pentecost (red). Not part of this card.
6. **d11 at Easter turns ivory.** It is the only colourway that flips a family's mode on its keys; the bands go dark. Recommended: keep, it is the liturgical white and gold. The dark gold version is easy to restore (`build/palettes.py`, `OVERRIDES`).

## 10. Build spec, check output and what was not verified

The build spec is `docs/plans/artosia/seasons/spec.json`; every drawing is serialised from it (`python -I build/build.py all`), and `python -I build/check.py` prints any number, numeral or colour in a drawing that the spec lacks. Its output at the last run:

<!-- gen:check -->
<!-- /gen:check -->

The check was also run once against planted gaps and flagged each (`python -I build/check.py --selftest`):

<!-- gen:selftest -->
<!-- /gen:selftest -->

Not verified: the marks through Rune's own painter (resvg drew them); text widths with the engine's kerning; the colours on a real projector and on the church's own screens; the reading of the dove and the tomb by a congregation (they were judged on a screen at 48 px, not asked of anyone); the RAM cost of {{n_made}} more templates in the gallery.

Self-critique, from `no-slop`: the sign beside the kicker came from the references, the ring and the sticker from each family's own ornaments, and the 160 px emblem from habit (the church packs), which is why it lost. Three families have no mark because their own rules say no icon, not because the design ran out. The palette of each season is one paint box used sixty times; the families differ in which keys take which role, which is the point of a colourway, and the cost is that two seasons of one light family are a lilac and an ash on near white grounds that a hall reads by their accent alone.

## 11. Brief ledger

| # | Brief item | Honoured by |
|---|---|---|
| 1 | Change no source code; write designs and SVG files only | the file list above; `git status` shows only `docs/plans/artosia/` |
| 2 | Read `notes.md`, `screens.md`, the `seasons-*.md` cards | section 1 (the seven cards and `screens.md` S01 to S08 were read) |
| 3 | Open the eight named images | section 1 cites each |
| 4 | Read `church-year.md`, `templates-is-colourways-and-archetypes.md`, the 17 `dN.json`, the three design docs | sections 4 to 6 |
| 5 | How icons are drawn (the accepted SVG subset) | section 3 and `check.py` |
| 6 | Twelve motif glyphs, one set, 24 grid, `currentColor`, 48 to 160 px | section 3, `marks/*.svg`, `marks-sizes.png` |
| 7 | Advent with four variants | `advent.svg`, `advent-1.svg` to `advent-4.svg` |
| 8 | Each of Christmas, Epiphany, Lent, Palm Sunday, Good Friday, Easter, Ascension, Pentecost, Reformation, Harvest, New Year | `marks/` and section 3's table |
| 9 | No faces, no Jesus, nothing kitsch | section 3, "No faces, no figure" |
| 10 | Per family: spot, px size, palette role, or "no room" | section 5, `spec.json` `families.*.placement` |
| 11 | Seasonal colourways for advent, christmas, lent, easter, pentecost where untagged, in the family's language, with every pair's ratio | section 6, `colourways/dN.json` |
| 12 | JSON snippets with `"season"` | section 6 snippets |
| 13 | Contact sheet PNG: twelve marks at 160 px, four families in Christmas and Easter with the mark placed, tool named | section 7, `contact-sheet.png`, renderer line above |
| 14 | At least two designs, a comparison table, a recommendation | section 2 |
| 15 | Acceptance criteria as checkable statements; `Contexts:` and `Neighbours:` lines | section 8 |
| 16 | Mock named, proof as the builder's screenshots, a table of mock values, keyboard behaviour | section 8 |
| 17 | A build spec: one data file, exact drawings, the mock drawn from it, a check script that prints missing numbers; report names both and quotes the output | `spec.json`, `build/check.py`, section 10 |
| 18 | Creation pattern shape and the least code | section 8 "Shape" |
| 19 | Never run WSL | not run |
| 20 | Report the plan in five lines and the paths | the closing message |

Rows 20, mapped 20, dropped 0.
