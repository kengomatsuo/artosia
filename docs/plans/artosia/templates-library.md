# Template library: eight new families (TPLLIB)

Designer's plan, 2026-11-15, for the announcement slide library of Artosia. It adds eight layout families beside the five in `crates/artosia-slides/assets/warta/directions` (d1 Editorial greige, d2 Pill poster, d3 Neon night, d4 Split panel, d5 Simple) and the minimal default candidates another designer draws in `docs/plans/artosia/templates-default.md`. A template is a family plus a colourway (`docs/templates-is-colourways-and-archetypes.md`), so each family below is one layout file plus one tokens file, and four colourways of JSON.

## What this is and how to read it

- This file is the spec. Every value a wireframe shows lives in `docs/plans/artosia/templates-library/spec.json`; the sixteen SVGs, the contact sheet and the generated tables of this file are drawn from it by `docs/plans/artosia/templates-library/build/build.py`. A number that is only in a picture is a defect, and `build.py check` prints any.
- Wireframes (1920 x 1080 SVG, default colourway): `docs/plans/artosia/templates-library/<family>-S02-birthdays.svg` and `<family>-S03-offerings.svg` for the eight families, and one contact sheet, `docs/plans/artosia/templates-library/contact-sheet.png`. The contact sheet was rendered from the SVGs by headless Google Chrome 157 (no resvg, Inkscape or ImageMagick is installed on this machine); photos and glows are stand-ins. The SVGs name font families, not files: to see one outside the build, install the cuts that `build/make_fonts.py` writes, or look at the PNG.
- Screens are the IDs of `C:\Users\kenne\Shared\inspo\artosia-templates\screens.md`: S01 schedule, S02 birthdays, S03 offering, S04 transfer, S05 verse, S06 notice, S07 gallery, S08 default. Birthdays and offerings are drawn; schedule, account, verse and notice are specified and measured (the `kinds` tables) but not drawn, as the brief asks.
- Every reference is a file under `C:\Users\kenne\Shared\inspo\` (`artosia-templates` and `artosia-notice-designs`), opened this run. Colours of the references are eyeballed, not sampled; the hex values of every colourway are this designer's picks, gated by the contrast tables below (a builder's guess is replaced, not kept).
- Fixtures (the worst case of the brief): a schedule of 8 rows over 4 days with 1 special row; 30 birthday names in three columns, 7 highlighted, a page from 22 Nov to 20 Des (two months), longest name 30 characters; 6 funds with amounts up to 999.999.999.999 and a derived total of 13 digits; a 16-digit account number with and without a QR; a 60-word verse (Psalm 121, KJV, a length fixture: no 60-word Indonesian verse is on disk, so widths are multiplied by 1.12); a 45-character notice title. All names, amounts and the account number are invented.
- Words on the slides are the engine's fixed words (`assets/warta/shared.json` `copy`, `Words` in `data.rs`): `Warta jemaat`, `Ulang tahun`, `Pekan ini`, `Kas jemaat`, `Jumlah persembahan`, `Khusus`, month names. Words new to the fixed set are listed per family where a family needs them.

## The eight in one table

Two pulls drew them. **Pull A, what an Indonesian church already knows** (the church lane: Jatibaru, Pelita Kasih, Bukit Zaitun, Tamalanrea, Paulus, Abdiel Zion): Diagonal, Liturgi, Hangat. **Pull B, what a Canva or Slidesgo gallery sells** (the galleries and products lanes): Kaca, Blok, Cahaya, Majalah, Angka. Where the two met, the church reference set the layout and the gallery reference set the finish.

<!-- gen:compare -->
| # | family | pull | ground and colour logic | layout skeleton | type voice | highlight | total | ornament | engine needs |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Diagonal | Indonesian church media | pale tinted page, one deep brand colour (blue, red, green, violet) | diagonal header block, three columns of date tiles under it, a slanted total plate at the foot | Montserrat 800 caps headlines over Barlow Semi Condensed rows | tile turns solid brand, row gets a tint band and bold name | brand parallelogram, white figure | one parallel stripe, an arrow | no polygon primitive: a rotated rect masked by the page |
| 2 | Liturgi | Indonesian church media | dark vestment colour (wine, sapphire, emerald, amethyst) with gold and parchment | centred axis inside a double gold frame; three columns split by gold hairlines; a parchment plate for the total | Cinzel capitals for titles and labels over Source Serif 4 for names and figures | names lie on a parchment band in dark ink, bold | parchment plate across the width, dark figure | double frame with corner diamonds, a small cross between two rules | none: rects, lines, rotated squares |
| 3 | Kaca | photo-led | the photo under one dark scrim; colour comes from the photo, the colourway tints the scrim, plate and accent | title block on the photo, one rounded plate holding the data, a solid accent block for the total | Jost light caps and medium rows, Allura script for the one title word | accent tint band with an accent bar, bold name | solid accent block over the photo, white figure | none; the photo is the ornament | none for the plate (flat translucent rect, no blur); photo fallback is the colourway's own gradient |
| 4 | Blok | bold colour-block | near-black body, one signal band, one deep block (yellow, orange, lime, pink) | signal band on top with the title, a deep colour block beside it, the body on black, a full-width signal foot for the total | Unbounded 800 caps (wide) over IBM Plex Sans Condensed rows | signal colour fills the row, black bold text | full-width signal foot, black figure up to 100 px | none: edges only | none: flat rects |
| 5 | Cahaya | gradient / soft light | a dark two-colour gradient (indigo to plum, navy to teal, wine to ember, forest to moss) with one large soft orb | light caps title over the orb, the month in light caps beneath, three borderless columns under month headings, the total as a very large light figure at the lower right | Sora 300 tracked caps and figures over Instrument Sans | a translucent white capsule with an accent bar, bold name | hero figure in Sora 300 up to 150 px on the gradient | the orb only | a radial glow: a stack of translucent circles or one baked PNG per colourway (UNSURE: banding) |
| 6 | Majalah | magazine editorial | ink (forest, midnight, oxblood) with cream, or newsprint with black; one ochre or vermilion accent | masthead under a double rule, three columns split by vertical hairlines, folio lines in the four corners, a double rule above the total line | Libre Caslon Display 150 px headline over Libre Caslon Text | an inverted block (cream on ink, ink on cream), bold | one line under a double rule, large accent figure at the right | double rule, vertical hairlines | none: rects and lines |
| 7 | Angka | data-first | paper or charcoal, one signal colour for the total, six fixed fund hues | ruled register in three banks under text headers, numerals in the date column with a colour bar per week; funds in one table with a share bar; a full-width signal foot | Big Shoulders Display 900 numerals and headline over IBM Plex Sans Condensed cells | signal fill, a 3 px ink outline around the run, bold | full-width signal foot, Big Shoulders figure up to 200 px | ruled cells, week bars, share bars | six fund hues per colourway; six new fixed words (Tgl, Nama, No, Dana, Bagian, Jumlah) |
| 8 | Hangat | seasonal-neutral warm | one warm hue in steps: cocoa, plum, evergreen, ivory; amber or clay as the only accent | title and italic month at the top left over faint arches, three rounded cards, a cream plate that overlaps the offering card for the total | Fraunces 400/600 and italic over Instrument Sans for figures | an accent capsule across the run, dark text, bold | cream plate overlapping the card, Fraunces figure | nested arch outlines at 12% opacity | one open path (arch): a rounded-top rect, or an asset |
<!-- /gen:compare -->

Coverage of the brief's spread: church lane at least twice (Diagonal, Liturgi, and Hangat leans on Paulus too); photo-led (Kaca); bold colour-block (Blok); gradient or soft light (Cahaya); magazine editorial (Majalah); data-first (Angka); seasonal-neutral warm (Hangat).

Not duplicated: the five built looks (L1 pill poster, L2 editorial greige, L4 neon, L7 band, L9 split panel of the notice-designs README) and the four default candidates of `docs/plans/artosia/templates-default.md`, which this designer read after the first drawing and checked each family against: **A Serif Italic** (serif roman and italic on warm off-white, a wash rising from the bottom edge) is why Cahaya is a dark gradient with an orb and light sans caps, not the Slidesgo off-white gradient; **B Swiss Numerals** (a dark date column, one red accent) and **D Civic Board** (a black header band, mono figures, inverted rows) are why Angka sets its numerals on white ruled cells under text headers, colours its weeks and funds, and ends in a signal foot; **C Quiet Tint** (one sans, a solid pill under the special row) shares nothing with the eight. The README's still unbuilt looks are used: L3 duotone poster and L5 grain paper inform Blok and Kaca, L6 laporan table is Angka, L8 calendar as picture is Kaca. Dropped as a duplicate: a second red family from GPIB Bukit Zaitun (Liturgi keeps red as one of four vestment colourways, Diagonal keeps it as one of four brand colourways).

## Rules every family keeps

**Sizes.** Every size is a fraction of the sheet height (the tables give px on 1080 and the percentage); the sheet is 1920 x 1080 and a gallery tile is the same drawing scaled. Body text (names, events, funds, notes, verse) is never under 40 px; labels and kickers never under 32 px; a layout that cannot hold its data at the floor splits the card into halves (`paginate`, depth 3) and never shrinks below it. The type tables give each role's ceiling: at 30 names the fit lowers the name size to 40 or 41 px (the fits table below).

**Names.** A name takes the longest form that fits its cell: full; middle names as initials; every name but the surname as an initial; first initial and surname; all initials (`ladder` in `build/lib.py` models the engine's `fit::ladder` with one extra level, first initial plus surname, which keeps a surname readable where the engine would drop to bare initials). The fits table shows how many of 30 take initials.

**Date rule on a page across two months.** The engine writes each day with its month (`Data::day_of`: `29 Nov`, `01 Des`). That costs 64 to 94 px of every name cell (measured per family from the date role). Three families (Cahaya, Majalah, Hangat) instead write the month once as a heading at the top of each column and where it changes, and each row writes its day only. It is one pure rule (`month_runs`, a Rule unit in `data.rs`), used by all three. The numbers (names that take initials out of 30):

<!-- gen:fits -->
| family | date rule used | name px | names shortened of 30 | if every row named its month | name cell px | longest full name px | fund labels on two lines of 6 | amount column px | total figure px |
|---|---|---|---|---|---|---|---|---|---|
| Diagonal | month on every row | 41 | 5 | - | 445.4 | 517.62 | 0 | 473.01 | 859.49 |
| Liturgi | month on every row | 40 | 8 | - | 444.45 | 564.12 | 0 | 402.02 | 685.6 |
| Kaca | month on every row | 41 | 10 | - | 437.22 | 587.08 | 2 | 438.73 | 536.04 |
| Blok | month on every row | 40 | 5 | - | 457.35 | 545 | 0 | 442.84 | 1139.6 |
| Cahaya | month heading per column | 40 | 6 | 17 | 498.67 | 597.52 | 0 | 427.15 | 1473 |
| Majalah | month heading per column | 40 | 6 | 20 | 509.33 | 632.74 | 0 | 423.34 | 721.63 |
| Angka | month on every row | 40 | 8 | - | 421.51 | 545 | 0 | 420.25 | 1201.78 |
| Hangat | month heading per column | 40 | 5 | 13 | 501.33 | 592.64 | 0 | 422.07 | 842.98 |
<!-- /gen:fits -->

This is the owner's call: the engine's per-row month is commit 4c44e0b's rule. The three families show the alternative; the other five keep the per-row month.

**Highlight.** Fill, weight and a mark: a highlight never relies on a fill alone. The mark (a solid tile, a bar, an outline, an inverted block) passes 3:1 against its ground in every colourway, which the contrast tables below test. It must read when the run is first, last, in the middle of 30, across a column break (the fixture crosses one), all of them or none.

**Total.** Reserved before the rows are laid out; at least as large as a row; derived from the funds (`Data::total`), never typed over; `Rp` has its own place before the figure and the digits are tabular where a column stacks them. On a continuation every part carries the card's whole total (`whole`). A total never clips: its size is solved to the room it has (the `fits` and `mock` tables give the measured width).

**Continuation.** A card cut into parts draws `n/m` and the range the part holds at the foot, in the family's foot colour and place:

<!-- gen:foot -->
| family | x | vertical middle | anchor | colour |
|---|---|---|---|---|
| 1 Diagonal | 56 | 1046 | start | mute |
| 2 Liturgi | 960 | 1024 | middle | gold |
| 3 Kaca | 96 | 1048 | start | white |
| 4 Blok | 40 | 1062 | start | mute |
| 5 Cahaya | 72 | 1046 | start | mute |
| 6 Majalah | 1856 | 1046 | end | mute |
| 7 Angka | 40 | 1054 | start | mute |
| 8 Hangat | 56 | 1050 | start | mute |
<!-- /gen:foot -->

**Date range.** Written by the engine's `Words::range` and never composed by a family: `8–14 November` inside a month, `29 Nov – 5 Des` across two; only its place and chip differ (each family's reads table).

**Figures.** A figure column uses a face whose digits are uniform (the `figures` column of each font table says `tabular` or `proportional`); proportional faces set single figures only (a total, an account number).

**QR and picture.** A QR is black on white with a four-module quiet zone and is never recoloured by a colourway (a value an outside reader scans is a literal the slide owns). A picture goes where the family says; text on a picture sits on a plate or a scrim, never bare; no picture means the layout redraws, not a hole.

**Fonts.** Faces marked `bundled` ship already (`assets/warta/fonts`); `new` faces are SIL OFL families from github.com/google/fonts (`ofl/<family>/`), each with its reason; each cut is a static file made by `build/make_fonts.py`, which also bakes tabular digits where the face has `tnum`. The uncut files are 60 to 420 KB each (3.3 MB for the 22 new cuts, Unbounded and Montserrat the largest at 370 to 420 KB); subset them to Latin before bundling (UNSURE of the final size, measure the first). Each family needs its OFL text in `assets/warta/licences/` like the existing ones.

## Family 1 Diagonal

<!-- gen:head:diagonal -->
**Idea.** A heavy caps headline cut by one diagonal into a deep colour block, a date range led by an arrow, white tiles for dates: the warta card every Indonesian church already knows, drawn at hall size.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/church/id-yt-gbis-jatibaru-warta-maxresdefault.jpg`: diagonal cut, heavy deep-blue caps in two lines, an arrow before the date range
- `artosia-templates/church/id-yt-gpib-bukitzaitun-warta-maxresdefault.jpg`: a red wedge across the top and a band at the foot: a limited two-colour set looks premium
- `artosia-templates/church/ch-cp-slide-photo3.jpg`: the white diagonal wedge is the readable plate, text never sits on a face
- `artosia-templates/church/id-yt-gbi-pelitakasih-warta-maxresdefault.jpg`: airy white card with one blue diagonal sweep at the edge
- `artosia-templates/church/id-yt-gbi-pelitakasih-warta-hq2.jpg`: the time is the loudest element of a service panel

**Ornaments** (only those the references show): parallel stripe beside the header block (`artosia-templates/church/id-yt-gbis-jatibaru-warta-maxresdefault.jpg`); arrow before the date range (`artosia-templates/church/id-yt-gbis-jatibaru-warta-maxresdefault.jpg`).
<!-- /gen:head:diagonal -->

**Lane.** Indonesian church media.

### Type

<!-- gen:type:diagonal -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| h8 | Montserrat | 800 | Montserrat-800.ttf | new: the geometric sans Indonesian church designers already use (Pelita Kasih, Basilea refs); static cut of the OFL variable file | tabular |
| h7 | Montserrat | 700 | Montserrat-700.ttf | new, same file | tabular |
| t5 | Barlow Semi Condensed | 500 | BarlowSemiCondensed-500.ttf | bundled | tabular |
| t6 | Barlow Semi Condensed | 600 | BarlowSemiCondensed-600.ttf | bundled | tabular |
| t7 | Barlow Semi Condensed | 700 | BarlowSemiCondensed-700.ttf | bundled | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Barlow Semi Condensed 700 | 32 | 3.0% | 0.14 | caps | sky |
| title | Montserrat 800 | 92 | 8.5% | 0.01 | caps | on_brand |
| sub | Barlow Semi Condensed 600 | 40 | 3.7% | 0.02 | as typed | sky |
| day | Montserrat 800 | 44 | 4.1% | 0 | as typed | brand |
| time | Montserrat 800 | 48 | 4.4% | 0 | as typed | ink |
| event | Barlow Semi Condensed 600 | 50 | 4.6% | 0 | as typed | ink |
| tag | Barlow Semi Condensed 700 | 32 | 3.0% | 0.1 | caps | on_brand |
| name | Barlow Semi Condensed 600 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | Barlow Semi Condensed 700 | 42 | 3.9% | 0 | as typed | ink |
| bday_day | Barlow Semi Condensed 700 | 36 | 3.3% | 0 | as typed | brand_dk |
| fund | Barlow Semi Condensed 600 | 46 | 4.3% | 0 | as typed | ink |
| amount | Montserrat 700 | 46 | 4.3% | 0 | as typed | ink |
| rp | Barlow Semi Condensed 700 | 34 | 3.1% | 0.04 | as typed | mute |
| total_label | Barlow Semi Condensed 700 | 36 | 3.3% | 0.14 | caps | on_brand |
| total | Montserrat 800 | 84 | 7.8% | 0 | as typed | on_brand |
| bank | Barlow Semi Condensed 700 | 46 | 4.3% | 0.12 | caps | brand |
| number | Montserrat 800 | 150 | 13.9% | 0 | as typed | brand_dk |
| holder | Barlow Semi Condensed 600 | 54 | 5.0% | 0 | as typed | ink |
| note | Barlow Semi Condensed 500 | 42 | 3.9% | 0 | as typed | mute |
| caption | Barlow Semi Condensed 700 | 32 | 3.0% | 0.12 | caps | mute |
| ref | Montserrat 800 | 56 | 5.2% | 0.04 | caps | brand |
| verse | Barlow Semi Condensed 500 | 56 | 5.2% | 0 | as typed | ink |
| verse_n | Montserrat 800 | 56 | 5.2% | 0 | as typed | brand |
| n_title | Montserrat 800 | 112 | 10.4% | 0.01 | caps | brand_dk |
| n_body | Barlow Semi Condensed 500 | 48 | 4.4% | 0 | as typed | ink |
| foot | Barlow Semi Condensed 700 | 32 | 3.0% | 0.1 | as typed | mute |
<!-- /gen:type:diagonal -->

### Grid

<!-- gen:grid:diagonal -->
| token | value | what it sets |
|---|---|---|
| mx | 56 | left and right margin |
| hdr_h | 262 | header block height |
| hdr_w | 1130 | header block width at the top edge |
| cut | 120 | horizontal run of the diagonal over the header height |
| per | 10 | names per column (3 columns) |
| gutter | 24 | gap between name columns |
| tile_w | 112 | date tile width |
| body_top | 312 | first name row top |
| body_bot | 1010 | last name row bottom |
| rows_top | 306 | first fund row top |
| rows_bot | 852 | last fund row bottom |
| total_top | 872 | total plate top edge |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:diagonal -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:diagonal -->
| role | biru (Biru) | merah (Merah) | hijau (Hijau) | ungu (Ungu) |
|---|---|---|---|---|
| ground | `#E9EEF5` | `#F2EDEA` | `#EAF1EC` | `#F0ECF5` |
| panel | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| ink | `#0E1A33` | `#211515` | `#10241A` | `#1E1230` |
| mute | `#46536D` | `#5C4A48` | `#455A4D` | `#54486A` |
| brand | `#0B3FA6` | `#B3141F` | `#0E6B43` | `#5B2A9B` |
| brand_dk | `#082B73` | `#7C0D16` | `#09482D` | `#3E1B70` |
| on_brand | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| sky | `#A9C6F2` | `#F3B4B0` | `#A6DBBF` | `#CDB6F0` |
| tint | `#D3E1F8` | `#F8D9D6` | `#D2EBDC` | `#E3D8F5` |
| line | `#BCC8DA` | `#D8C9C6` | `#BDD0C4` | `#CBC3D8` |

| text on ground | needs | where it is drawn | biru | merah | hijau | ungu |
|---|---|---|---|---|---|---|
| ink on ground | text | names, event rows | 14.83 | 15.28 | 14.19 | 15.22 |
| mute on ground | text | 'Pekan ini' label, notes | 6.63 | 7.15 | 6.48 | 7.17 |
| brand on ground | text | date range beside the arrow, bank line | 7.91 | 5.95 | 5.71 | 8.01 |
| ink on panel | text | names on a card | 17.29 | 17.76 | 16.29 | 17.73 |
| brand_dk on panel | text | date in an unmarked tile | 13.10 | 10.90 | 10.63 | 13.16 |
| ink on tint | text | highlighted name on its band | 13.09 | 13.43 | 12.92 | 13.01 |
| on_brand on brand | text | highlighted tile, number tile, total plate | 9.23 | 6.92 | 6.56 | 9.34 |
| on_brand on brand_dk | large | title on the header block | 13.10 | 10.90 | 10.63 | 13.16 |
| sky on brand_dk | text | kicker and month on the header block | 7.51 | 6.21 | 6.83 | 7.24 |
| mute on ground | large | caption | 6.63 | 7.15 | 6.48 | 7.17 |
| brand on ground | mark | arrow and stripe against the page | 7.91 | 5.95 | 5.71 | 8.01 |
| brand_dk on ground | mark | header block against the page | 11.23 | 9.39 | 9.25 | 11.29 |
| brand on tint | mark | highlighted tile against its band | 6.98 | 5.23 | 5.20 | 6.85 |

Gallery swatch: ground `ground`, accent `brand` (the `swatch` keys of the tokens file).

Least ratio per colourway: biru 6.63, merah 5.23, hijau 5.20, ungu 6.85. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:diagonal -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | Header block with the title and the week range; one group per day, a dark day tile (day name in caps, date numeral) spanning its rows; per row the time in Montserrat 800 in brand colour, then the event; 16 px between groups. The special row takes the tint band, a solid brand time and a `Khusus` tag on a brand fill at its right end. | Rows share one pitch (1.45 x event size); the event wraps to two lines before the size drops; below the 40 px floor the card splits in halves (1/2, 2/2) at a day boundary where it can. |
| S02 birthdays | Drawn: `diagonal-S02-birthdays.svg`. Header block, arrow chip with the range, three columns of date tiles and names. | Tile as wide as the widest 'dd Mon'; names take initials by the ladder; 7 tiles solid, one band per run. |
| S03 offerings | Drawn: `diagonal-S03-offerings.svg`. Numbered brand tiles, hairlines, amounts with `Rp` in its own place, a slanted total plate. | The plate is reserved first; the amount column is measured from the widest amount; a long fund name takes two lines. |
| S04 account | Header `REKENING GEREJA`. Left: bank in brand caps, the number as large as one line allows (never under 96 px), `atas nama`, holder. Right: the QR on a 520 px white card with a 3 px brand frame offset 24 px and the scan caption below. Without a QR the column spans the page. | Digits are tabular; the QR is black on white with a four-module quiet zone, never recoloured; under 96 px the number breaks into two groups of eight digits, it never shrinks further. |
| S05 verse | Reference in the header block in Montserrat caps; the verse in Barlow Semi Condensed beside a brand bar, verse numbers raised in brand colour. With a picture it sits in a parallelogram at the right cut by the header's diagonal. | Largest size from 56 down to 44 that fits balanced lines; the reference is outside the fitted box. |
| S06 notice | Header block with the notice kicker; the title in Montserrat 800 caps on at most three lines, body in Barlow Semi Condensed. With a picture: the parallelogram photo at the right; without: the title block fills the width and the second stripe fills the corner. | Title from 112 px down to 64 px; the body shrinks to its 40 px floor or the card splits. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:diagonal -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 56); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 904.05 px in a 1150 px cell: 1 line |
| S04 account | 16-digit number | with QR: 101 px on 1 line in 1228 px; without: 145 px on 1 line in 1752 px |
| S05 verse | 60 words (width x1.12) | with picture 56 px in 7 lines (556.64 of 670 px); without 56 px in 5 lines (397.6 of 670 px) |
| S06 notice | a 45-character title | with picture 105 px on 3 lines (role 112); without 112 px on 3 lines |
<!-- /gen:kinds:diagonal -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | Date tile solid brand with white text, a tint band behind the run, a bold name. Never colour alone: the solid tile is the 3:1 mark (the contrast table lists brand on tint and on the page). |
| Total | A slanted brand plate across the foot, label left, `Rp` in its own place and the figure in Montserrat 800 white; reserved before the rows are laid out. |
| Continuation | `1/2  22 Nov – 5 Des` at the foot, left, muted; position in the foot table. |
| Date range | An arrow then `29 NOV – 5 DES` in Montserrat 800 over `PEKAN INI`; one month reads `8–14 NOVEMBER`; two months name both, as the engine's `range` does. |

### Mock values to measure the build against

<!-- gen:mock:diagonal -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 92 | 2 px |  |  |
| birthday name size at 30 names (px) | 41 | 1 px |  |  |
| birthday row pitch (px) | 69.8 | 2 px |  |  |
| date column (px) | 129.27 | 4 px |  |  |
| name cell (px) | 445.4 | 4 px |  |  |
| names shortened of 30 | 5 | exact |  |  |
| offering row pitch (px) | 91 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 473.01 | 4 px |  |  |
| total figure width (px) | 859.49 | 4 px |  |  |
| left margin (px) | 56 | 1 px |  |  |
| corner radius (px) | 8 | 1 px |  |  |
| ground | #E9EEF5 | exact |  |  |
| accent | #0B3FA6 | exact |  |  |
<!-- /gen:mock:diagonal -->

Card: TPLLIB1, below.

## Family 2 Liturgi

<!-- gen:head:liturgi -->
**Idea.** A centred, symmetrical order-of-service card: deep vestment colour, a double gold frame, one small cross, Roman capitals over a book serif.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/church/id-yt-n0wii0tpfDs.jpg`: Gereja Toraja Tamalanrea: deep red field, small gold serif reference above a large white display serif title
- `artosia-templates/church/id-yt-gpib-bukitzaitun-warta-maxresdefault.jpg`: seal, thin vertical rule, tracked caps title: a lockup with a two-colour limit
- `artosia-templates/church/cv-golden-a.png`: gold on near black with fine lines: premium without heavy ornament
- `artosia-templates/galleries/sg-night-s33.jpg`: gold outline table, days in a gold serif on charcoal, readable over a dim hall
- `artosia-templates/church/cv-dimensional-c.png`: small tracked caps over a larger serif, a rule below: centred, two sizes

**Ornaments** (only those the references show): double gold frame, corner diamonds (`artosia-templates/church/cv-golden-a.png`); small cross between two rules, centred (`artosia-templates/church/id-yt-n0wii0tpfDs.jpg`); gold hairlines between columns (`artosia-templates/galleries/sg-night-s33.jpg`).
<!-- /gen:head:liturgi -->

**Lane.** Indonesian church media.

### Type

<!-- gen:type:liturgi -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| c7 | Cinzel | 700 | Cinzel-700.ttf | new: Roman inscription capitals, the lettering of Bibles and liturgy, read at distance in caps only | proportional |
| c6 | Cinzel | 600 | Cinzel-600.ttf | new, same file | proportional |
| s4 | Source Serif 4 Text | 400 | SourceSerif4Text-400.ttf | bundled | tabular |
| s6 | Source Serif 4 Text | 600 | SourceSerif4Text-600.ttf | new cut of the bundled family (weight 600 for highlighted names and amounts) | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Cinzel 600 | 32 | 3.0% | 0.2 | caps | gold |
| title | Cinzel 700 | 88 | 8.1% | 0.1 | caps | cream |
| sub | Source Serif 4 Text 400 | 40 | 3.7% | 0.02 | as typed | mute |
| day | Cinzel 700 | 40 | 3.7% | 0.08 | caps | gold |
| time | Source Serif 4 Text 600 | 48 | 4.4% | 0 | as typed | gold |
| event | Source Serif 4 Text 400 | 50 | 4.6% | 0 | as typed | cream |
| tag | Cinzel 600 | 32 | 3.0% | 0.14 | caps | gold |
| name | Source Serif 4 Text 400 | 42 | 3.9% | 0 | as typed | cream |
| name_hl | Source Serif 4 Text 600 | 42 | 3.9% | 0 | as typed | ink |
| bday_day | Source Serif 4 Text 600 | 36 | 3.3% | 0 | as typed | gold |
| fund | Source Serif 4 Text 400 | 46 | 4.3% | 0 | as typed | cream |
| amount | Source Serif 4 Text 600 | 48 | 4.4% | 0 | as typed | gold |
| rp | Cinzel 600 | 34 | 3.1% | 0.08 | as typed | mute |
| total_label | Cinzel 700 | 34 | 3.1% | 0.18 | caps | ink |
| total | Cinzel 700 | 76 | 7.0% | 0.02 | as typed | ink |
| bank | Cinzel 700 | 44 | 4.1% | 0.14 | caps | gold |
| number | Cinzel 700 | 140 | 13.0% | 0.04 | as typed | cream |
| holder | Source Serif 4 Text 600 | 54 | 5.0% | 0 | as typed | cream |
| note | Source Serif 4 Text 400 | 42 | 3.9% | 0 | as typed | mute |
| caption | Cinzel 600 | 32 | 3.0% | 0.16 | caps | gold |
| ref | Cinzel 700 | 52 | 4.8% | 0.14 | caps | gold |
| verse | Source Serif 4 Text 400 | 56 | 5.2% | 0 | as typed | cream |
| verse_n | Source Serif 4 Text 600 | 56 | 5.2% | 0 | as typed | gold |
| n_title | Cinzel 700 | 108 | 10.0% | 0.06 | caps | cream |
| n_body | Source Serif 4 Text 400 | 48 | 4.4% | 0 | as typed | cream |
| foot | Cinzel 600 | 32 | 3.0% | 0.14 | as typed | gold |
<!-- /gen:type:liturgi -->

### Grid

<!-- gen:grid:liturgi -->
| token | value | what it sets |
|---|---|---|
| frame | 30 | outer gold frame inset from every edge |
| frame_gap | 12 | distance between the two gold lines |
| pad | 72 | left and right text margin |
| per | 10 | names per column (3 columns) |
| gutter | 36 | gap between name columns, a gold hairline in the middle |
| day_w | 108 | date column width |
| body_top | 346 | first name row top |
| body_bot | 1008 | last name row bottom |
| rows_top | 350 | first fund row top |
| rows_bot | 872 | last fund row bottom |
| total_top | 892 | total plate top edge |
| total_h | 112 | total plate height |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:liturgi -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:liturgi -->
| role | anggur (Anggur) | safir (Safir) | zamrud (Zamrud) | ametis (Ametis) |
|---|---|---|---|---|
| ground | `#6B1424` | `#14274F` | `#14392C` | `#3F1F56` |
| ground2 | `#480C17` | `#0B1731` | `#0B2119` | `#26113A` |
| gold | `#E3C06C` | `#E3C06C` | `#DDBC6C` | `#E6C77C` |
| cream | `#F7EBD3` | `#F5EEDC` | `#F4EEDC` | `#F5ECDD` |
| ink | `#3C0A13` | `#0B1731` | `#0B2119` | `#26113A` |
| mute | `#EAD3C8` | `#CFD8EC` | `#D2E3D8` | `#DDCDEE` |

| text on ground | needs | where it is drawn | anggur | safir | zamrud | ametis |
|---|---|---|---|---|---|---|
| cream on ground | text | names, fund labels, event rows | 10.16 | 12.66 | 10.97 | 11.63 |
| gold on ground | text | dates, amounts, kicker, reference | 6.87 | 8.39 | 6.95 | 8.32 |
| mute on ground | text | month and notes | 8.37 | 10.25 | 9.52 | 9.11 |
| ink on cream | text | highlighted name and date on the parchment band | 14.28 | 15.36 | 14.54 | 14.65 |
| cream on ground | large | title | 10.16 | 12.66 | 10.97 | 11.63 |
| gold on ground | mark | frame, rules, cross, chip outline | 6.87 | 8.39 | 6.95 | 8.32 |
| gold on ground2 | mark | frame against the outer border | 8.97 | 10.18 | 9.22 | 10.48 |
| ink on gold | text | special tag on a gold fill | 9.66 | 10.18 | 9.22 | 10.48 |
| ink on cream | large | total figure on the parchment plate | 14.28 | 15.36 | 14.54 | 14.65 |

Gallery swatch: ground `ground`, accent `gold` (the `swatch` keys of the tokens file).

Least ratio per colourway: anggur 6.87, safir 8.39, zamrud 6.95, ametis 8.32. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:liturgi -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | Reads as an order of service: day name in Cinzel gold caps at the left of each group, time in Source Serif 600 gold, event in cream, gold hairlines between groups. The special row lies on a parchment band in dark ink with a gold `Khusus` tag. | As Diagonal: one pitch, two-line events, halves below the floor. |
| S02 birthdays | Drawn: `liturgi-S02-birthdays.svg`. Three columns split by gold hairlines, dates in gold, parchment band for the highlighted run, a gold outlined pill with the range. | Source Serif is wide: 8 of 30 names take initials (see the fits table); a month heading rule would not change that much here. |
| S03 offerings | Drawn: `liturgi-S03-offerings.svg`. Fund labels in cream, amounts in gold Source Serif 600, hairlines, a parchment plate for the total. | Plate reserved first; amounts tabular (Source Serif digits are uniform). |
| S04 account | Inside the double frame: bank in Cinzel gold caps, the number in Cinzel 700 cream (up to 140 px), a double rule, holder in Source Serif 600; QR on a cream square with a gold frame at the right, 500 px. | Same number and QR rules as every family. |
| S05 verse | Reference in Cinzel gold caps between two gold rules, the verse centred in Source Serif 56 down to 44 on balanced lines with gold raised numbers. A picture sits in a gold-framed rectangle at the right 36%. | Without a picture the verse takes the 1500 px centre column. |
| S06 notice | The cross and rules as the header; title in Cinzel caps up to 108 px on at most three lines, centred; body in Source Serif centred; picture in a gold-framed rectangle at the right 36%, none: text centred. | Title down to 64 px; body to its floor or the card splits. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:liturgi -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 52); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 1008.5 px in a 1040 px cell: 1 line |
| S04 account | 16-digit number | with QR: 102 px on 1 line in 1100 px; without: 140 px on 1 line in 1700 px |
| S05 verse | 60 words (width x1.12) | with picture 54 px in 8 lines (613.44 of 650 px); without 56 px in 7 lines (556.64 of 650 px) |
| S06 notice | a 45-character title | with picture 101 px on 3 lines (role 108); without 108 px on 3 lines |
<!-- /gen:kinds:liturgi -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | A parchment band under the run, dark ink on it, bold name, date in ink. The band against the page is the mark. |
| Total | A parchment plate across the width, Cinzel caps label, the figure in Cinzel 700 in dark ink. |
| Continuation | `1/2  ...` centred in gold Cinzel just inside the frame. |
| Date range | A gold outlined pill at the right of the month line: `PEKAN INI 29 NOV – 5 DES`. |

### Mock values to measure the build against

<!-- gen:mock:liturgi -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 88 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 66.2 | 2 px |  |  |
| date column (px) | 109.55 | 4 px |  |  |
| name cell (px) | 444.45 | 4 px |  |  |
| names shortened of 30 | 8 | exact |  |  |
| offering row pitch (px) | 87 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 402.02 | 4 px |  |  |
| total figure width (px) | 685.6 | 4 px |  |  |
| left margin (px) | 72 | 1 px |  |  |
| corner radius (px) | 6 | 1 px |  |  |
| ground | #6B1424 | exact |  |  |
| accent | #E3C06C | exact |  |  |
<!-- /gen:mock:liturgi -->

Card: TPLLIB2, below.

## Family 3 Kaca

<!-- gen:head:kaca -->
**Idea.** The church's own photograph fills the slide, darkened once; every word sits on one frosted plate, a script word is the only voice on the picture.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/church/id-yt-gbi-pelitakasih-warta2-maxresdefault.jpg`: a frosted plate over a washed congregation photo keeps strong type calm
- `artosia-templates/church/id-yt-lightmedia-sample-hq1.jpg`: a plate reads over a busy bokeh photo
- `artosia-templates/galleries/canva-bull-10.jpg`: light weight type over a dark photo with a time block at the right
- `artosia-templates/galleries/canva-bull-15.jpg`: very thin caps over a landscape with a script line: scale and weight contrast
- `artosia-templates/church/ch-cp-slide-photo3.jpg`: a plate keeps text off faces
- `artosia-notice-designs/slides/101-gereja-toraja-bintaro-ulang-tahun-one-name-calendar-slide.webp`: dusk photo band over a white data area, the reference for photo plus data

**Ornaments** (only those the references show): none beyond the photo, the plate and the script word (`artosia-templates/church/id-yt-gbi-pelitakasih-warta2-maxresdefault.jpg`).
<!-- /gen:head:kaca -->

**Lane.** photo-led.

### Type

<!-- gen:type:kaca -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| j3 | Jost | 300 | Jost-300.ttf | new: a Futura-like geometric sans whose thin caps are the Canva bulletin look; legible only at 40 px and up in the light cut | tabular |
| j5 | Jost | 500 | Jost-500.ttf | new, same file | tabular |
| j6 | Jost | 600 | Jost-600.ttf | new, same file | tabular |
| a4 | Allura | 400 | Allura-400.ttf | new: a formal script, used for one word at 150 px, never for data (Basilea, Pelita Kasih refs) | proportional |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Jost 500 | 32 | 3.0% | 0.28 | caps | white |
| title | Allura 400 | 150 | 13.9% | 0 | as typed | white |
| sub | Jost 300 | 40 | 3.7% | 0.3 | caps | white |
| day | Jost 600 | 44 | 4.1% | 0 | as typed | accent |
| time | Jost 600 | 48 | 4.4% | 0 | as typed | accent |
| event | Jost 500 | 50 | 4.6% | 0 | as typed | ink |
| tag | Jost 600 | 32 | 3.0% | 0.12 | caps | on_accent |
| name | Jost 500 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | Jost 600 | 42 | 3.9% | 0 | as typed | ink |
| bday_day | Jost 600 | 36 | 3.3% | 0 | as typed | accent |
| fund | Jost 500 | 46 | 4.3% | 0 | as typed | ink |
| amount | Jost 600 | 46 | 4.3% | 0 | as typed | ink |
| rp | Jost 500 | 34 | 3.1% | 0.06 | as typed | mute |
| total_label | Jost 500 | 34 | 3.1% | 0.2 | caps | on_accent |
| total | Jost 600 | 58 | 5.4% | 0 | as typed | on_accent |
| bank | Jost 600 | 44 | 4.1% | 0.2 | caps | accent |
| number | Jost 600 | 140 | 13.0% | 0.02 | as typed | ink |
| holder | Jost 500 | 54 | 5.0% | 0 | as typed | ink |
| note | Jost 500 | 42 | 3.9% | 0 | as typed | mute |
| caption | Jost 500 | 32 | 3.0% | 0.2 | caps | mute |
| ref | Allura 400 | 84 | 7.8% | 0 | as typed | accent |
| verse | Jost 500 | 54 | 5.0% | 0 | as typed | ink |
| verse_n | Jost 600 | 54 | 5.0% | 0 | as typed | accent |
| n_title | Jost 300 | 120 | 11.1% | 0.04 | caps | white |
| n_body | Jost 500 | 48 | 4.4% | 0 | as typed | ink |
| foot | Jost 500 | 32 | 3.0% | 0.2 | caps | white |
<!-- /gen:type:kaca -->

### Grid

<!-- gen:grid:kaca -->
| token | value | what it sets |
|---|---|---|
| mx | 96 | title left margin on the photo |
| plate_x | 48 | plate left edge |
| plate_y | 262 | plate top edge |
| plate_h | 754 | plate height |
| plate_rx | 28 | plate corner radius |
| plate_pad | 28 | plate side padding |
| per | 10 | names per column (3 columns) |
| gutter | 24 | gap between name columns inside the plate |
| day_w | 104 | date column width |
| name_floor | 40 | smallest name size before the slide splits |
| plate_w | 1096 | offering plate width (the photo stays visible right of it) |
| tot_x | 1200 | total block left edge |
| tot_y | 640 | total block top edge |
<!-- /gen:grid:kaca -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:kaca -->
| role | senja (Senja) | fajar (Fajar) | rimba (Rimba) | mawar (Mawar) |
|---|---|---|---|---|
| scrim | `#0E1B38` | `#2E1B0C` | `#082219` | `#3B1425` |
| plate | `#FFFFFF` | `#FFFAF2` | `#FAFFFC` | `#FFF8F9` |
| ink | `#101827` | `#2A1A10` | `#0E2118` | `#2A1019` |
| mute | `#4B5568` | `#6A5444` | `#476055` | `#6B4A55` |
| accent | `#A24B00` | `#A33F14` | `#0C6B49` | `#A31D4B` |
| on_accent | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| white | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| dusk_a | `#263A6B` | `#8A5A32` | `#1F5A45` | `#6A2B48` |
| dusk_b | `#E08A4F` | `#F0C27A` | `#A8D08A` | `#F0A3A8` |
| accent_tint | `#FCE3CB` | `#FBDDC8` | `#D4EEDF` | `#FAD7DD` |

Layers (a translucent fill over its ground; contrast is measured on the composite, over a black and a white photo pixel where it sits on a photo): `scrim_photo` = scrim at 0.62 over photo; `plate_photo` = plate at 0.92 over photo

| text on ground | needs | where it is drawn | senja | fajar | rimba | mawar |
|---|---|---|---|---|---|---|
| ink on plate_photo | text | names, fund labels on the plate | 14.90 | 13.49 | 13.94 | 14.13 |
| mute on plate_photo | text | 'Rp' and notes on the plate | 6.29 | 5.71 | 5.67 | 6.13 |
| accent on plate_photo | text | dates, week chip text, bank line on the plate | 4.97 | 5.16 | 5.42 | 5.91 |
| accent on plate_photo | mark | highlight bar against the plate | 4.97 | 5.16 | 5.42 | 5.91 |
| ink on accent_tint | text | highlighted name on its tint | 14.37 | 12.98 | 13.68 | 13.34 |
| accent on accent_tint | text | highlighted date on its tint | 4.80 | 4.97 | 5.32 | 5.58 |
| on_accent on accent | text | total block text, special tag | 5.93 | 6.41 | 6.53 | 7.41 |
| white on scrim_photo | text | kicker and month on the photo (worst pixel: white) | 4.82 | 4.69 | 4.78 | 4.71 |
| white on scrim_photo | large | script title on the photo | 4.82 | 4.69 | 4.78 | 4.71 |
| accent on accent_tint | mark | bar against its own tint | 4.80 | 4.97 | 5.32 | 5.58 |

Gallery swatch: ground `dusk_a`, accent `accent` (the `swatch` keys of the tokens file).

Least ratio per colourway: senja 4.80, fajar 4.69, rimba 4.78, mawar 4.71. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:kaca -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | One plate holds the week: day name as a small tinted pill at the left of each group, time in Jost 600 in accent, event in Jost 500; the special row on the accent tint with the accent bar and a `Khusus` pill. The photo shows in the margins and under the title. | No chip rows: day pills sit in the time column, so 8 rows use the plate's full height. |
| S02 birthdays | Drawn: `kaca-S02-birthdays.svg`. The Allura word on the photo, a white pill with the range, one plate with three columns. | Jost is wide: 10 of 30 names take initials; month headings would cut that (see the fits table). |
| S03 offerings | Drawn: `kaca-S03-offerings.svg`. A plate of six rows left, the photo right, a solid accent block with the total over the photo. | The block is sized to the widest total: `Rp` above, digits below at the largest size that fits (58 px for 13 digits). |
| S04 account | A plate at the left (about 1000 px): bank in accent caps, the number in Jost 600 up to 140 px, holder; the QR on its own white plate over the photo at the right, 480 px, with the caption. No QR: the plate centres. | A 16-digit number does not fit one line beside a QR at 96 px, so it breaks into two groups of eight at 140 px. |
| S05 verse | Reference in Allura 84 px accent above the verse on the plate; verse in Jost 500 down to 44 px with raised accent numbers; the plate is about 1000 px wide so the photo shows at the right. | With no photo the dusk gradient of the colourway shows. |
| S06 notice | Title in Jost 300 caps (or one Allura word for a welcome) at 120 px, white, straight on the scrim, no plate; body on a plate below. The picture is the ground itself. | No picture: the colourway's gradient is the ground and nothing else changes, so there is no hole. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:kaca -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 56); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 1014.6 px in a 1100 px cell: 1 line |
| S04 account | 16-digit number | with QR: 140 px on 2 lines in 1000 px; without: 140 px on 1 line in 1600 px |
| S05 verse | 60 words (width x1.12) | with picture 53 px in 9 lines (677.34 of 690 px); without 54 px in 6 lines (460.08 of 690 px) |
| S06 notice | a 45-character title | with picture 120 px on 3 lines (role 120); without 120 px on 3 lines |
<!-- /gen:kinds:kaca -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | Accent tint band, a 6 px accent bar at its left edge, bold name, accent date; the bar is the mark. |
| Total | A solid accent block over the photo, white figure; never on bare photo. |
| Continuation | `1/2  ...` in white under the plate, on the scrim. |
| Date range | A white pill straddling the scrim with accent text: `PEKAN INI · 29 NOV – 5 DES`. |

### Mock values to measure the build against

<!-- gen:mock:kaca -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 150 | 2 px |  |  |
| birthday name size at 30 names (px) | 41 | 1 px |  |  |
| birthday row pitch (px) | 70.2 | 2 px |  |  |
| date column (px) | 122.11 | 4 px |  |  |
| name cell (px) | 437.22 | 4 px |  |  |
| names shortened of 30 | 10 | exact |  |  |
| offering row pitch (px) | 117 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 438.73 | 4 px |  |  |
| total figure width (px) | 536.04 | 4 px |  |  |
| left margin (px) | 48 | 1 px |  |  |
| corner radius (px) | 28 | 1 px |  |  |
| ground | #0E1B38 | exact |  |  |
| accent | #A24B00 | exact |  |  |
<!-- /gen:mock:kaca -->

Card: TPLLIB3, below.

## Family 4 Blok

<!-- gen:head:blok -->
**Idea.** Hard-edged flat blocks of one signal colour, a near-black and one deep colour; the slide is the blocks, the type is huge and wide, and a highlight is the signal colour poured over a row.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/products/keynote/appstore-iphone-01.png`: Keynote Photography Bold: a flat saturated field with heavy black type and a hard-edged picture
- `artosia-templates/galleries/canva-neon-p12.jpg`: acid ground with heavy purple caps: loud, high contrast
- `artosia-templates/galleries/sg-charm-s14.jpg`: one colour block and one phrase, the whole slide
- `artosia-templates/products/pitch/conference-talk.jpg`: black ground, grey blocks, ultra bold caps with tiny corner meta
- `artosia-templates/galleries/canva-sched-05.jpg`: a solid block marks the special row, outline marks the rest
- `artosia-templates/church/id-yt-gbi-pelitakasih-warta-hq2.jpg`: a deep colour panel with the time as the loudest element

**Ornaments** (only those the references show): none; hard edges only (`artosia-templates/products/keynote/appstore-iphone-01.png`).
<!-- /gen:head:blok -->

**Lane.** bold colour-block.

### Type

<!-- gen:type:blok -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| u8 | Unbounded | 800 | Unbounded-800.ttf | new: an extended heavy sans, the one face here wide enough to be a block by itself; used only for titles and figures | tabular |
| u7 | Unbounded | 700 | Unbounded-700.ttf | new, same file | tabular |
| p5 | IBM Plex Sans Condensed | 500 | IBMPlexSansCondensed-500.ttf | bundled | tabular |
| p6 | IBM Plex Sans Condensed | 600 | IBMPlexSansCondensed-600.ttf | bundled | tabular |
| p7 | IBM Plex Sans Condensed | 700 | IBMPlexSansCondensed-700.ttf | bundled | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.12 | caps | deep |
| title | Unbounded 800 | 96 | 8.9% | 0 | caps | deep |
| sub | IBM Plex Sans Condensed 700 | 40 | 3.7% | 0.04 | caps | deep |
| day | Unbounded 800 | 40 | 3.7% | 0 | caps | signal |
| time | Unbounded 800 | 44 | 4.1% | 0 | as typed | signal |
| event | IBM Plex Sans Condensed 600 | 50 | 4.6% | 0 | as typed | white |
| tag | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.1 | caps | deep |
| name | IBM Plex Sans Condensed 600 | 42 | 3.9% | 0 | as typed | white |
| name_hl | IBM Plex Sans Condensed 700 | 42 | 3.9% | 0 | as typed | deep |
| bday_day | IBM Plex Sans Condensed 700 | 36 | 3.3% | 0 | as typed | signal |
| fund | IBM Plex Sans Condensed 600 | 46 | 4.3% | 0 | as typed | white |
| amount | Unbounded 700 | 38 | 3.5% | 0 | as typed | white |
| rp | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.06 | as typed | mute |
| total_label | IBM Plex Sans Condensed 700 | 36 | 3.3% | 0.12 | caps | deep |
| total | Unbounded 800 | 84 | 7.8% | 0 | as typed | deep |
| bank | Unbounded 800 | 44 | 4.1% | 0 | caps | signal |
| number | Unbounded 800 | 120 | 11.1% | 0 | as typed | white |
| holder | IBM Plex Sans Condensed 700 | 54 | 5.0% | 0 | as typed | white |
| note | IBM Plex Sans Condensed 500 | 42 | 3.9% | 0 | as typed | mute |
| caption | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.12 | caps | mute |
| ref | Unbounded 800 | 48 | 4.4% | 0.02 | caps | signal |
| verse | IBM Plex Sans Condensed 600 | 56 | 5.2% | 0 | as typed | white |
| verse_n | Unbounded 800 | 48 | 4.4% | 0 | as typed | signal |
| n_title | Unbounded 800 | 104 | 9.6% | 0 | caps | deep |
| n_body | IBM Plex Sans Condensed 600 | 48 | 4.4% | 0 | as typed | white |
| foot | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.1 | as typed | mute |
<!-- /gen:type:blok -->

### Grid

<!-- gen:grid:blok -->
| token | value | what it sets |
|---|---|---|
| mx | 40 | left and right margin |
| band_h | 262 | signal band height on the birthday slide |
| band_h3 | 252 | signal band height on the offering slide |
| blue_x | 1440 | deep block left edge |
| per | 10 | names per column (3 columns) |
| gutter | 24 | gap between name columns |
| day_w | 100 | date column width |
| body_top | 292 | first name row top |
| body_bot | 1044 | last name row bottom |
| rows_top | 262 | first fund row top |
| rows_bot | 840 | last fund row bottom |
| total_top | 860 | signal foot top edge |
| total_max | 100 | largest total figure |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:blok -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:blok -->
| role | kuning (Kuning) | jingga (Jingga) | limau (Limau) | mawar (Mawar) |
|---|---|---|---|---|
| signal | `#FFC31A` | `#FF6B1F` | `#B9F13A` | `#FF5C9B` |
| deep | `#111111` | `#15100E` | `#0E1A12` | `#1B0F17` |
| field2 | `#2347FF` | `#1E2A8A` | `#0B6B4B` | `#5A1FB5` |
| white | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| mute | `#C9C9C9` | `#CFC7C3` | `#C3CFC6` | `#D2C5CD` |

| text on ground | needs | where it is drawn | kuning | jingga | limau | mawar |
|---|---|---|---|---|---|---|
| white on deep | text | names, fund labels | 18.88 | 18.88 | 17.87 | 18.63 |
| signal on deep | text | dates, kicker numerals | 11.76 | 6.63 | 13.36 | 6.44 |
| mute on deep | text | 'Rp', rules, notes | 11.40 | 11.34 | 11.11 | 11.18 |
| deep on signal | text | title, band text, highlighted rows, total label | 11.76 | 6.63 | 13.36 | 6.44 |
| deep on signal | large | title, total figure | 11.76 | 6.63 | 13.36 | 6.44 |
| white on field2 | text | range text on the deep block | 6.15 | 11.98 | 6.52 | 9.14 |
| signal on deep | mark | signal band and foot against the black | 11.76 | 6.63 | 13.36 | 6.44 |

Gallery swatch: ground `deep`, accent `signal` (the `swatch` keys of the tokens file).

Least ratio per colourway: kuning 6.15, jingga 6.63, limau 6.52, mawar 6.44. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:blok -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | Signal band with the title; on black, a signal cell with the day name in Unbounded (deep text) spans each group; time in Unbounded signal, event in Plex 50 px white; the special row filled with signal, deep bold text. | Rows share one pitch; two-line events before shrinking; halves below the floor. |
| S02 birthdays | Drawn: `blok-S02-birthdays.svg`. Signal band, a cobalt block with the range, three columns on black, signal-filled highlighted runs. | Plex Sans Condensed is narrow: 5 of 30 names take initials. |
| S03 offerings | Drawn: `blok-S03-offerings.svg`. Rows on black, a full-width signal foot with the total in Unbounded. | The figure is solved to the room the label leaves, up to 100 px, which the 13-digit fixture reaches. |
| S04 account | Band `REKENING GEREJA`; the bank in Unbounded signal caps, the number in Unbounded white (up to 120 px) on black; the QR on a white square on the cobalt block at the right, 480 px. | Unbounded is wide: with a QR the 16-digit number takes two lines of eight at 120 px. |
| S05 verse | Reference in Unbounded signal caps on the cobalt block; the verse in Plex 600 white, a signal bar at the left; a picture is a hard-edged rectangle at the right 38%. | Largest size 56 down to 44; no picture: the field widens. |
| S06 notice | Signal band holds the title in Unbounded up to 104 px on at most three lines; Plex body on black; picture: a hard-edged rectangle at the right 40% of the black field; none: full width. | Title down to 64 px. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:blok -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 60); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 976.75 px in a 1150 px cell: 1 line |
| S04 account | 16-digit number | with QR: 120 px on 2 lines in 1100 px; without: 120 px on 1 line in 1800 px |
| S05 verse | 60 words (width x1.12) | with picture 56 px in 8 lines (636.16 of 730 px); without 56 px in 6 lines (477.12 of 730 px) |
| S06 notice | a 45-character title | with picture 90 px on 3 lines (role 104); without 104 px on 3 lines |
<!-- /gen:kinds:blok -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | The row filled with the signal colour, deep text, bold; merged per column for a run. Signal on black is the mark. |
| Total | A full-width signal foot, label left and the figure right in Unbounded up to 100 px, never clipped. |
| Continuation | `1/2  ...` muted at the foot inside the black field. |
| Date range | The cobalt block at the right of the band: `PEKAN INI` over `29 NOV – 5 DES`. |

### Mock values to measure the build against

<!-- gen:mock:blok -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 96 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 75.2 | 2 px |  |  |
| date column (px) | 105.98 | 4 px |  |  |
| name cell (px) | 457.35 | 4 px |  |  |
| names shortened of 30 | 5 | exact |  |  |
| offering row pitch (px) | 96.33 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 442.84 | 4 px |  |  |
| total figure width (px) | 1139.6 | 4 px |  |  |
| left margin (px) | 40 | 1 px |  |  |
| corner radius (px) | none | 1 px |  |  |
| ground | #111111 | exact |  |  |
| accent | #FFC31A | exact |  |  |
<!-- /gen:mock:blok -->

Card: TPLLIB4, below.

## Family 5 Cahaya

<!-- gen:head:cahaya -->
**Idea.** A dark gradient field with one big soft orb of light behind the title, light tracked caps and light figures, the data in plain white: the Circular and Dimensional mood in one colour world per colourway.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/church/cv-circular-b.png`: one big warm orb on a pink-to-orange gradient with a light spaced title: one shape and one gradient make it premium
- `artosia-templates/galleries/sg-night-s15.jpg`: a giant light figure with a small caption on charcoal with soft amber glows
- `artosia-templates/church/cv-dimensional-c.png`: small tracked caps over a larger title on a dark light-burst gradient, one cool colour
- `artosia-templates/products/beautifulai/action_number_slide.jpg`: the number is the slide on an orange-to-magenta gradient with one light caption
- `artosia-templates/galleries/sg-grad-s00.jpg`: one soft gradient bleeding from a corner is the only colour

**Ornaments** (only those the references show): one soft orb behind the title and the date pill (`artosia-templates/church/cv-circular-b.png`).
<!-- /gen:head:cahaya -->

**Lane.** gradient / soft light.

### Type

<!-- gen:type:cahaya -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| s3 | Sora | 300 | Sora-300.ttf | new: a geometric sans with a very light cut whose wide caps and round digits suit big figures on a gradient; the light weight is used only at 96 px and up | tabular |
| s6 | Sora | 600 | Sora-600.ttf | new, same file | tabular |
| g5 | Instrument Sans | 500 | InstrumentSans-500.ttf | bundled | tabular |
| g6 | Instrument Sans | 600 | InstrumentSans-600.ttf | bundled | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Instrument Sans 600 | 32 | 3.0% | 0.14 | caps | mute |
| title | Sora 300 | 96 | 8.9% | 0.14 | caps | ink |
| sub | Sora 300 | 40 | 3.7% | 0.2 | caps | mute |
| day | Sora 600 | 44 | 4.1% | 0.1 | caps | accent |
| time | Instrument Sans 600 | 48 | 4.4% | 0 | as typed | accent |
| event | Instrument Sans 500 | 50 | 4.6% | 0 | as typed | ink |
| tag | Instrument Sans 600 | 32 | 3.0% | 0.08 | as typed | on_pill |
| name | Instrument Sans 500 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | Instrument Sans 600 | 42 | 3.9% | 0 | as typed | ink |
| bday_day | Instrument Sans 600 | 40 | 3.7% | 0 | as typed | accent |
| fund | Instrument Sans 500 | 46 | 4.3% | 0 | as typed | ink |
| amount | Instrument Sans 600 | 46 | 4.3% | 0 | as typed | ink |
| rp | Instrument Sans 500 | 34 | 3.1% | 0.04 | as typed | mute |
| total_label | Sora 600 | 32 | 3.0% | 0.2 | caps | mute |
| total | Sora 300 | 150 | 13.9% | 0 | as typed | ink |
| bank | Sora 600 | 40 | 3.7% | 0.2 | caps | accent |
| number | Sora 300 | 170 | 15.7% | 0 | as typed | ink |
| holder | Instrument Sans 600 | 54 | 5.0% | 0 | as typed | ink |
| note | Instrument Sans 500 | 42 | 3.9% | 0 | as typed | mute |
| caption | Sora 600 | 32 | 3.0% | 0.2 | caps | accent |
| ref | Sora 600 | 52 | 4.8% | 0.16 | caps | accent |
| verse | Instrument Sans 500 | 60 | 5.6% | 0 | as typed | ink |
| verse_n | Instrument Sans 600 | 60 | 5.6% | 0 | as typed | accent |
| n_title | Sora 300 | 120 | 11.1% | 0.1 | caps | ink |
| n_body | Instrument Sans 500 | 48 | 4.4% | 0 | as typed | ink |
| foot | Instrument Sans 600 | 32 | 3.0% | 0.12 | as typed | mute |
<!-- /gen:type:cahaya -->

### Grid

<!-- gen:grid:cahaya -->
| token | value | what it sets |
|---|---|---|
| mx | 72 | left and right margin |
| per | 10 | names per column (3 columns) |
| gutter | 32 | gap between name columns |
| day_w | 50 | date column width (a floor; the measured width wins) |
| body_top | 322 | first name row top |
| body_bot | 1012 | last name row bottom |
| rows_top | 316 | first fund row top |
| rows_bot | 776 | last fund row bottom |
| list_r | 1300 | right edge of the fund list |
| tot_fig_cy | 948 | vertical middle of the total figure |
| total_max | 150 | largest total figure |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:cahaya -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:cahaya -->
| role | aurora (Aurora) | samudra (Samudra) | bara (Bara) | pucuk (Pucuk) |
|---|---|---|---|---|
| g1 | `#1A1240` | `#07203A` | `#2B0B14` | `#08261D` |
| g2 | `#5B1F6B` | `#0F5C6B` | `#7A2418` | `#1E5A3A` |
| orb_a | `#FFB48A` | `#9DF0D0` | `#FFC27A` | `#E4F58A` |
| orb_b | `#E5457F` | `#2FB7C9` | `#F0643A` | `#6CC36A` |
| ink | `#F7F2FB` | `#EFFAF9` | `#FFF3EA` | `#F2FBEF` |
| mute | `#C9BDDA` | `#A8CFD3` | `#E3BFAE` | `#B4D3B8` |
| accent | `#FFC7A6` | `#B7F2DC` | `#FFD7A0` | `#D9F28A` |
| glass | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` | `#FFFFFF` |
| pill | `#F7F2FB` | `#EFFAF9` | `#FFF3EA` | `#F2FBEF` |
| on_pill | `#1A1240` | `#07203A` | `#2B0B14` | `#08261D` |

Layers (a translucent fill over its ground; contrast is measured on the composite, over a black and a white photo pixel where it sits on a photo): `glass_on_g2` = glass at 0.12 over g2

| text on ground | needs | where it is drawn | aurora | samudra | bara | pucuk |
|---|---|---|---|---|---|---|
| ink on g2 | text | names, fund labels, figures on the lighter end of the gradient | 10.28 | 7.13 | 9.20 | 7.69 |
| ink on g1 | text | the same on the darker end | 15.81 | 15.44 | 16.59 | 15.18 |
| mute on g2 | text | kicker, 'Rp', notes, month | 6.35 | 4.53 | 5.89 | 5.02 |
| accent on g2 | text | month headings, dates | 7.54 | 6.06 | 7.40 | 6.60 |
| ink on glass_on_g2 | text | highlighted name on its glass capsule | 7.39 | 5.30 | 6.83 | 5.62 |
| accent on glass_on_g2 | text | highlighted date on its capsule | 5.42 | 4.50 | 5.49 | 4.83 |
| on_pill on pill | text | range pill over the orb | 15.81 | 15.44 | 16.59 | 15.18 |
| ink on g2 | large | title, total figure | 10.28 | 7.13 | 9.20 | 7.69 |
| accent on g2 | mark | highlight bar, headings against the gradient | 7.54 | 6.06 | 7.40 | 6.60 |
| pill on g2 | mark | pill against the gradient | 10.28 | 7.13 | 9.20 | 7.69 |

Gallery swatch: ground `g1`, accent `orb_a` (the `swatch` keys of the tokens file).

Least ratio per colourway: aurora 5.42, samudra 4.50, bara 5.49, pucuk 4.83. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:cahaya -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | Two columns of two days: per day a heading in Sora caps (day name and date) in accent, then rows with the time in Instrument Sans 600 accent and the event in Instrument Sans 500 on up to two lines; the special row on a glass capsule with the accent bar. The orb sits behind the title. | Two columns carry the tallest day (3 rows) comfortably; events wrap to two lines in 720 px. |
| S02 birthdays | Drawn: `cahaya-S02-birthdays.svg`. Light tracked caps title and month, a light pill with the range over the orb, three borderless columns under month headings in accent caps. | Month headings, not a month per row: 6 of 30 names take initials, against 17 with a month on every row. |
| S03 offerings | Drawn: `cahaya-S03-offerings.svg`. Rows on the left, a date pill over the orb, the total as a very light figure at the lower right on the gradient. | Total solved to its room, up to 150 px (the 13-digit fixture reaches it); the label above in tracked caps. |
| S04 account | Bank in Sora accent caps, the number in Sora 300 up to 170 px, holder in Instrument Sans 600; the QR on a white rounded card over the lower gradient at the right, 460 px. | Sora digits are wide: with a QR a 16-digit number takes two lines of eight at 170 px; without one it holds one line at 147 px. |
| S05 verse | Reference in Sora accent caps above; the verse in Instrument Sans 500 60 px down to 44 on the gradient with accent raised numbers, wide margins, the orb behind the title area only; a picture is a tall rounded rectangle at the right 30%. | Text stays off the orb: the orb is in the upper right, above every body row. |
| S06 notice | Sora 300 caps title up to 120 px on at most three lines at the top left over the orb, Instrument Sans body; a picture is a rounded rectangle at the right 36% below the orb; none: the text widens. | Title down to 64 px. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:cahaya -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 59); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 1077.2 px in a 720 px cell: 2 lines |
| S04 account | 16-digit number | with QR: 170 px on 2 lines in 1100 px; without: 147 px on 1 line in 1700 px |
| S05 verse | 60 words (width x1.12) | with picture 55 px in 8 lines (624.8 of 670 px); without 60 px in 7 lines (596.4 of 670 px) |
| S06 notice | a 45-character title | with picture 96 px on 3 lines (role 120); without 120 px on 3 lines |
<!-- /gen:kinds:cahaya -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | A translucent white capsule (12%) behind the run with an accent bar, bold name; the bar is the 3:1 mark. |
| Total | A very light figure in Sora 300 at the lower right, `Rp` small before it, a tracked caps label above. |
| Continuation | `1/2  ...` muted at the foot, left. |
| Date range | A light pill at the top right over the orb: `Pekan ini  29 Nov – 5 Des`. |

### Mock values to measure the build against

<!-- gen:mock:cahaya -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 96 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 62.73 | 2 px |  |  |
| date column (px) | 50 | 4 px |  |  |
| name cell (px) | 498.67 | 4 px |  |  |
| names shortened of 30 | 6 | exact |  |  |
| offering row pitch (px) | 76.67 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 427.15 | 4 px |  |  |
| total figure width (px) | 1473 | 4 px |  |  |
| left margin (px) | 72 | 1 px |  |  |
| corner radius (px) | 24 | 1 px |  |  |
| ground | #1A1240 | exact |  |  |
| accent | #FFC7A6 | exact |  |  |
<!-- /gen:mock:cahaya -->

Card: TPLLIB5, below.

## Family 6 Majalah

<!-- gen:head:majalah -->
**Idea.** A magazine page: a double rule, a big text-serif headline in mixed case, four corner lines of small caps, and rows that read like a contents page, on ink or on newsprint.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/products/pitch/editorial.jpg`: dark forest ground, condensed serif headline in cream, tracked caps meta in the corners
- `artosia-templates/products/gamma/voile.jpg`: condensed serif caps with tracked kickers above and below, left aligned
- `artosia-templates/galleries/sg-bizslides-s03.jpg`: table of contents: serif numerals, bold serif labels, small captions, hairline curves
- `artosia-templates/galleries/sg-gather-s31.jpg`: inverted rows in black with cream text mark the special items
- `artosia-templates/products/beautifulai/area_chart-2.jpg`: serif title with a column of tracked-caps labels over serif figures

**Ornaments** (only those the references show): double rule under the masthead (`artosia-templates/products/pitch/editorial.jpg`); four corner meta lines (`artosia-templates/products/pitch/editorial.jpg`); vertical column hairlines (`artosia-templates/galleries/sg-bizslides-s03.jpg`).
<!-- /gen:head:majalah -->

**Lane.** magazine editorial.

### Type

<!-- gen:type:majalah -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| d4 | Libre Caslon Display | 400 | LibreCaslonDisplay-400.ttf | new: Caslon is the face of the printed Bible and of the newspaper column; the Display cut is for the headline only | proportional |
| t4 | Libre Caslon Text | 400 | LibreCaslonText-400.ttf | new: the text cut, tabular figures | tabular |
| t7 | Libre Caslon Text | 700 | LibreCaslonText-700.ttf | new, same file | tabular |
| t4i | Libre Caslon Text | 400 italic | LibreCaslonText-400i.ttf | new, same file | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Libre Caslon Text 700 | 32 | 3.0% | 0.18 | caps | mute |
| title | Libre Caslon Display 400 | 150 | 13.9% | -0.01 | as typed | ink |
| sub | Libre Caslon Text 400 i | 56 | 5.2% | 0 | as typed | accent |
| day | Libre Caslon Text 700 | 44 | 4.1% | 0 | as typed | ink |
| time | Libre Caslon Text 700 | 48 | 4.4% | 0 | as typed | accent |
| event | Libre Caslon Text 400 | 50 | 4.6% | 0 | as typed | ink |
| tag | Libre Caslon Text 700 | 32 | 3.0% | 0.14 | caps | accent |
| name | Libre Caslon Text 400 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | Libre Caslon Text 700 | 42 | 3.9% | 0 | as typed | inv_fg |
| bday_day | Libre Caslon Text 400 i | 40 | 3.7% | 0 | as typed | accent |
| fund | Libre Caslon Text 400 | 46 | 4.3% | 0 | as typed | ink |
| amount | Libre Caslon Text 700 | 46 | 4.3% | 0 | as typed | ink |
| rp | Libre Caslon Text 400 i | 36 | 3.3% | 0 | as typed | mute |
| total_label | Libre Caslon Text 700 | 34 | 3.1% | 0.18 | caps | ink |
| total | Libre Caslon Display 400 | 96 | 8.9% | 0 | as typed | accent |
| bank | Libre Caslon Text 700 | 44 | 4.1% | 0.16 | caps | mute |
| number | Libre Caslon Display 400 | 150 | 13.9% | 0 | as typed | ink |
| holder | Libre Caslon Text 400 i | 56 | 5.2% | 0 | as typed | ink |
| note | Libre Caslon Text 400 | 42 | 3.9% | 0 | as typed | mute |
| caption | Libre Caslon Text 700 | 32 | 3.0% | 0.18 | caps | mute |
| ref | Libre Caslon Text 400 i | 60 | 5.6% | 0 | as typed | accent |
| verse | Libre Caslon Text 400 | 54 | 5.0% | 0 | as typed | ink |
| verse_n | Libre Caslon Text 700 | 54 | 5.0% | 0 | as typed | accent |
| n_title | Libre Caslon Display 400 | 140 | 13.0% | -0.01 | as typed | ink |
| n_body | Libre Caslon Text 400 | 46 | 4.3% | 0 | as typed | ink |
| foot | Libre Caslon Text 700 | 32 | 3.0% | 0.16 | caps | mute |
<!-- /gen:type:majalah -->

### Grid

<!-- gen:grid:majalah -->
| token | value | what it sets |
|---|---|---|
| mx | 64 | left and right margin |
| rule_y | 98 | masthead rule top |
| title_base | 252 | headline baseline |
| per | 10 | names per column (3 columns) |
| gutter | 36 | gap between name columns, a hairline in the middle |
| day_w | 50 | date column width |
| body_top | 322 | first name row top |
| body_bot | 1008 | last name row bottom |
| rows_top | 352 | first fund row top |
| rows_bot | 880 | last fund row bottom |
| total_rule_y | 900 | double rule above the total |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:majalah -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:majalah -->
| role | hutan (Hutan) | tinta (Tinta) | anggur (Anggur) | kertas (Kertas) |
|---|---|---|---|---|
| ground | `#10201B` | `#0F1220` | `#2B0E17` | `#F3EFE5` |
| ink | `#F2EEDF` | `#F1EEE6` | `#F6ECE6` | `#17150F` |
| mute | `#B7C2B6` | `#B4B8CC` | `#D9BFC0` | `#5A554A` |
| rule | `#55695E` | `#565C7D` | `#7A4B55` | `#A9A292` |
| accent | `#E5A93C` | `#E8B04B` | `#E9B26A` | `#A9361C` |
| inv_bg | `#F2EEDF` | `#F1EEE6` | `#F6ECE6` | `#17150F` |
| inv_fg | `#10201B` | `#0F1220` | `#2B0E17` | `#F3EFE5` |

| text on ground | needs | where it is drawn | hutan | tinta | anggur | kertas |
|---|---|---|---|---|---|---|
| ink on ground | text | names, fund labels | 14.52 | 16.06 | 15.33 | 15.90 |
| mute on ground | text | corner lines, column heads | 9.17 | 9.46 | 10.33 | 6.46 |
| accent on ground | text | italic dates, month, aside | 8.09 | 9.53 | 9.37 | 5.66 |
| inv_fg on inv_bg | text | highlighted rows | 14.52 | 16.06 | 15.33 | 15.90 |
| ink on ground | large | headline | 14.52 | 16.06 | 15.33 | 15.90 |
| accent on ground | large | total figure | 8.09 | 9.53 | 9.37 | 5.66 |
| accent on ground | mark | highlight bar and rule colour | 8.09 | 9.53 | 9.37 | 5.66 |
| inv_bg on ground | mark | highlighted block against the page | 14.52 | 16.06 | 15.33 | 15.90 |

Gallery swatch: ground `ground`, accent `accent` (the `swatch` keys of the tokens file).

Least ratio per colourway: hutan 8.09, tinta 9.46, anggur 9.37, kertas 5.66. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:majalah -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | A contents page in two columns: each day a section head (day name in Caslon italic accent with the date) over a hairline, rows with the time in Libre Caslon Text 700 accent and the event in Caslon 50 px on up to two lines; the special row as an inverted block. | Events wrap to two lines in 720 px; day heads count as rows in the height. |
| S02 birthdays | Drawn: `majalah-S02-birthdays.svg`. Masthead under a double rule, an accent aside line with the range, three columns split by hairlines with month section heads. | Month heads, not a month per row: 6 of 30 names take initials, 20 with a month on every row (Caslon is the widest face here). |
| S03 offerings | Drawn: `majalah-S03-offerings.svg`. Column heads `Dana` and `Jumlah` over a hairline, six rows, a double rule, and the total on one line with the figure in Libre Caslon Display. | Total reserved under the double rule; amounts tabular. |
| S04 account | Bank in tracked small caps, the number in Libre Caslon Display up to 150 px (128 px on one line beside a QR), holder in italic; the QR on a card with a double-rule frame, a hairline between data and QR. | Same number and QR rules. |
| S05 verse | The verse in Libre Caslon Text 54 down to 44 with the reference in italic accent below it; a picture is a full-height column at the right 36% with a hairline; none: a wide measure with the corner meta lines. | Verse sizes: see the measured table; at 60 words with a picture it is the tightest of the eight. |
| S06 notice | Headline in Libre Caslon Display up to 140 px on at most three lines, body in Caslon on one measured column; a picture is a column at the right with a small-caps caption line; none: two columns of body. | Title down to 64 px. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:majalah -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 60); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 1112.45 px in a 720 px cell: 2 lines |
| S04 account | 16-digit number | with QR: 128 px on 1 line in 1100 px; without: 150 px on 1 line in 1700 px |
| S05 verse | 60 words (width x1.12) | with picture 52 px in 9 lines (664.56 of 670 px); without 54 px in 7 lines (536.76 of 670 px) |
| S06 notice | a 45-character title | with picture 140 px on 3 lines (role 140); without 140 px on 2 lines |
<!-- /gen:kinds:majalah -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | An inverted block (cream on ink, ink on cream), bold; the block against the page is the mark. |
| Total | One line under a double rule, tracked caps label left, the figure in Libre Caslon Display accent at the right. |
| Continuation | `1/2  ...` takes the bottom-right corner line, where a folio belongs. |
| Date range | An accent aside line under the church name: `PEKAN INI 29 NOV – 5 DES`, and the months in italic at the right of the headline. |

### Mock values to measure the build against

<!-- gen:mock:majalah -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 150 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 62.36 | 2 px |  |  |
| date column (px) | 50 | 4 px |  |  |
| name cell (px) | 509.33 | 4 px |  |  |
| names shortened of 30 | 6 | exact |  |  |
| offering row pitch (px) | 88 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 423.34 | 4 px |  |  |
| total figure width (px) | 721.63 | 4 px |  |  |
| left margin (px) | 64 | 1 px |  |  |
| corner radius (px) | none | 1 px |  |  |
| ground | #10201B | exact |  |  |
| accent | #E5A93C | exact |  |  |
<!-- /gen:mock:majalah -->

Card: TPLLIB6, below.

## Family 7 Angka

<!-- gen:head:angka -->
**Idea.** A ruled register on paper: big condensed numerals as the date column, a colour bar per week and one colour per fund from chip to bar, and a full-width foot where the total is the largest thing on the slide.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-notice-designs/slides/108-gk-kartini-laporan-keuangan-persembahan-syukur-rows-total.webp`: Indonesian laporan: count and amount columns, ruled rows, TOTAL set apart (its total is clipped: reserved here)
- `artosia-templates/galleries/sg-gather-s24.jpg`: a black header band with white caps over a ruled table, first column bold
- `artosia-templates/galleries/canva-sched-07.jpg`: huge date numerals with a small day abbreviation, hairline row rules
- `artosia-templates/products/beautifulai/area_chart-2.jpg`: totals in a solid colour column, tracked-caps label over a large figure
- `artosia-templates/products/beautifulai/action_number_slide.jpg`: the number is the slide
- `artosia-templates/galleries/sg-planner-s23.jpg`: one huge condensed numeral and a small caption: scale contrast alone

**Ornaments** (only those the references show): ruled cells under a text header row (`artosia-templates/galleries/sg-gather-s24.jpg`); a colour bar per week and a share bar per fund (`artosia-templates/products/beautifulai/area_chart-2.jpg`); one colour per fund from chip to bar (`artosia-notice-designs/slides/108-gk-kartini-laporan-keuangan-persembahan-syukur-rows-total.webp`).
<!-- /gen:head:angka -->

**Lane.** data-first.

### Type

<!-- gen:type:angka -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| b9 | Big Shoulders Display | 900 | BigShouldersDisplay-900.ttf | new: a signage condensed with strong numerals; proportional digits, so it sets headline and the total only, never a stacked column | proportional |
| b8 | Big Shoulders Display | 800 | BigShouldersDisplay-800.ttf | new, same file | proportional |
| p5 | IBM Plex Sans Condensed | 500 | IBMPlexSansCondensed-500.ttf | bundled | tabular |
| p6 | IBM Plex Sans Condensed | 600 | IBMPlexSansCondensed-600.ttf | bundled | tabular |
| p7 | IBM Plex Sans Condensed | 700 | IBMPlexSansCondensed-700.ttf | bundled | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.12 | caps | mute |
| title | Big Shoulders Display 900 | 132 | 12.2% | 0.01 | caps | ink |
| sub | IBM Plex Sans Condensed 700 | 40 | 3.7% | 0.06 | caps | mute |
| day | Big Shoulders Display 900 | 72 | 6.7% | 0 | as typed | ink |
| time | IBM Plex Sans Condensed 700 | 48 | 4.4% | 0 | as typed | ink |
| event | IBM Plex Sans Condensed 600 | 50 | 4.6% | 0 | as typed | ink |
| tag | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.1 | caps | on_signal |
| name | IBM Plex Sans Condensed 600 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | IBM Plex Sans Condensed 700 | 42 | 3.9% | 0 | as typed | on_signal |
| bday_day | Big Shoulders Display 900 | 56 | 5.2% | 0 | as typed | ink |
| date | IBM Plex Sans Condensed 600 | 32 | 3.0% | 0.04 | as typed | mute |
| fund | IBM Plex Sans Condensed 600 | 46 | 4.3% | 0 | as typed | ink |
| amount | IBM Plex Sans Condensed 700 | 50 | 4.6% | 0 | as typed | ink |
| rp | IBM Plex Sans Condensed 600 | 34 | 3.1% | 0.04 | as typed | mute |
| total_label | IBM Plex Sans Condensed 700 | 36 | 3.3% | 0.14 | caps | on_signal |
| total | Big Shoulders Display 900 | 170 | 15.7% | 0 | as typed | on_signal |
| bank | IBM Plex Sans Condensed 700 | 46 | 4.3% | 0.1 | caps | mute |
| number | Big Shoulders Display 900 | 200 | 18.5% | 0.01 | as typed | ink |
| holder | IBM Plex Sans Condensed 700 | 54 | 5.0% | 0 | as typed | ink |
| note | IBM Plex Sans Condensed 500 | 42 | 3.9% | 0 | as typed | mute |
| caption | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.12 | caps | mute |
| ref | Big Shoulders Display 900 | 80 | 7.4% | 0.02 | caps | ink |
| verse | IBM Plex Sans Condensed 600 | 56 | 5.2% | 0 | as typed | ink |
| verse_n | Big Shoulders Display 900 | 56 | 5.2% | 0 | as typed | ink |
| n_title | Big Shoulders Display 900 | 150 | 13.9% | 0.01 | caps | ink |
| n_body | IBM Plex Sans Condensed 500 | 48 | 4.4% | 0 | as typed | ink |
| foot | IBM Plex Sans Condensed 700 | 32 | 3.0% | 0.1 | as typed | mute |
<!-- /gen:type:angka -->

### Grid

<!-- gen:grid:angka -->
| token | value | what it sets |
|---|---|---|
| mx | 40 | left and right margin |
| head_y | 214 | header strip top |
| head_h | 48 | header strip height |
| per | 10 | names per bank (3 banks) |
| gutter | 20 | gap between banks |
| day_w | 132 | date cell width |
| body_top | 262 | first name row top |
| body_bot | 1022 | last name row bottom |
| lab_x | 176 | fund label left edge |
| bar_x | 960 | share bar left edge |
| bar_w | 330 | share bar full length (100% of the total) |
| rows_bot | 846 | last fund row bottom |
| total_top | 872 | signal foot top edge |
| total_max | 200 | largest total figure |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:angka -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:angka -->
| role | terang (Terang) | arang (Arang) | laut (Laut) | pagi (Pagi) |
|---|---|---|---|---|
| ground | `#F4F1EA` | `#14171A` | `#0E2540` | `#FFF6E3` |
| cell | `#FFFFFF` | `#1E2328` | `#153354` | `#FFFFFF` |
| ink | `#15181B` | `#F1F2F3` | `#F3F6FA` | `#231B10` |
| mute | `#555B61` | `#A7AEB5` | `#A9BBD1` | `#665A47` |
| signal | `#FFD100` | `#FFD100` | `#FFB400` | `#D43B22` |
| on_signal | `#15181B` | `#14171A` | `#0E2540` | `#FFFFFF` |
| fund1 | `#C2410C` | `#FDBA74` | `#FDBA74` | `#C2410C` |
| fund2 | `#0F766E` | `#5EEAD4` | `#5EEAD4` | `#0F766E` |
| fund3 | `#1D4ED8` | `#93C5FD` | `#93C5FD` | `#1D4ED8` |
| fund4 | `#A21CAF` | `#F0ABFC` | `#F0ABFC` | `#A21CAF` |
| fund5 | `#4D7C0F` | `#BEF264` | `#BEF264` | `#4D7C0F` |
| fund6 | `#B45309` | `#FCD34D` | `#FCD34D` | `#B45309` |
| on_fund | `#FFFFFF` | `#14171A` | `#0E2540` | `#FFFFFF` |

| text on ground | needs | where it is drawn | terang | arang | laut | pagi |
|---|---|---|---|---|---|---|
| ink on ground | text | title, dates | 15.80 | 16.05 | 14.27 | 15.82 |
| mute on ground | text | month, notes | 6.09 | 8.02 | 7.89 | 6.27 |
| ink on cell | text | names, fund labels in cells | 17.82 | 14.13 | 11.86 | 16.99 |
| mute on cell | text | 'Rp', share text, caption | 6.87 | 7.06 | 6.56 | 6.73 |
| on_signal on signal | text | highlighted row, tag, total label | 12.20 | 12.31 | 8.68 | 4.72 |
| on_signal on signal | large | total figure | 12.20 | 12.31 | 8.68 | 4.72 |
| ink on ground | large | title numerals | 15.80 | 16.05 | 14.27 | 15.82 |
| on_fund on fund1 | text | fund 1 number chip | 5.18 | 10.67 | 9.17 | 5.18 |
| on_fund on fund2 | text | fund 2 number chip | 5.47 | 12.16 | 10.46 | 5.47 |
| on_fund on fund3 | text | fund 3 number chip | 6.70 | 9.98 | 8.58 | 6.70 |
| on_fund on fund4 | text | fund 4 number chip | 6.32 | 10.23 | 8.79 | 6.32 |
| on_fund on fund5 | text | fund 5 number chip | 4.99 | 13.77 | 11.84 | 4.99 |
| on_fund on fund6 | text | fund 6 number chip | 5.02 | 12.48 | 10.73 | 5.02 |
| fund1 on cell | mark | fund 1 share bar | 5.18 | 9.39 | 7.62 | 5.18 |
| fund2 on cell | mark | fund 2 share bar | 5.47 | 10.70 | 8.69 | 5.47 |
| fund3 on cell | mark | fund 3 share bar | 6.70 | 8.78 | 7.13 | 6.70 |
| fund4 on cell | mark | fund 4 share bar | 6.32 | 9.00 | 7.31 | 6.32 |
| fund5 on cell | mark | fund 5 share bar | 4.99 | 12.12 | 9.84 | 4.99 |
| fund6 on cell | mark | fund 6 share bar | 5.02 | 10.98 | 8.92 | 5.02 |

Gallery swatch: ground `ground`, accent `signal` (the `swatch` keys of the tokens file).

Least ratio per colourway: terang 4.99, arang 7.06, laut 6.56, pagi 4.72. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:angka -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | A register under a text header row (`Tgl`, `Hari`, `Jam`, `Kegiatan`) and a 3 px rule: the day numeral in Big Shoulders spans its group's rows, time in Plex 700, event in Plex 600 on up to two lines; the special row signal-filled with a 3 px ink outline and a bold event. | 8 rows hold at 50 px by height with room to spare; a second line is allowed. |
| S02 birthdays | Drawn: `angka-S02-birthdays.svg`. Three banks on ruled cells under text headers; a colour bar at the left of each bank marks the week (one hue per week, counted from the page's first Sunday); the date numeral in Big Shoulders with the month in small caps, shown once per day; runs filled with signal and outlined. | 8 of 30 names take initials; a second name on the same day leaves the date cell empty and the rule shorter. |
| S03 offerings | Drawn: `angka-S03-offerings.svg`. A table with number chips in six fixed hues, a share bar and percentage per fund, tabular amounts, and a full-width signal foot with the total up to 200 px. | Shares are amount over total to one decimal; the bar is the share of the whole; the foot is solved to the room (165 px for the 13-digit fixture). |
| S04 account | A ruled card of four rows (bank, number, holder, note), labels in tracked caps; the number in Big Shoulders up to 200 px (130 px beside a QR), the QR in a framed cell at the right. | One figure only, so the proportional digits are fine. |
| S05 verse | The reference on a signal tab at the top left in Big Shoulders caps; the verse in Plex 600 in a ruled 1200 px cell with Big Shoulders raised numbers; a picture is a framed cell at the right. | Largest size 56 down to 44. |
| S06 notice | Title in Big Shoulders up to 150 px on at most three lines, body in Plex; when the notice has a date and a place, a strip of two ruled cells under the title. | Without a picture the title may take two lines. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:angka -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 63); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 976.75 px in a 1000 px cell: 1 line |
| S04 account | 16-digit number | with QR: 130 px on 1 line in 1100 px; without: 200 px on 1 line in 1800 px |
| S05 verse | 60 words (width x1.12) | with picture 56 px in 8 lines (636.16 of 720 px); without 56 px in 6 lines (477.12 of 720 px) |
| S06 notice | a 45-character title | with picture 150 px on 3 lines (role 150); without 150 px on 2 lines |
<!-- /gen:kinds:angka -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | Signal fill with a 3 px ink outline around the run, bold name; the outline is the mark. |
| Total | A full-width signal foot, label left, the figure in Big Shoulders up to 200 px right; shares per fund stay above it. |
| Continuation | `1/2  ...` muted at the foot, left. |
| Date range | A signal tag at the top right: `PEKAN INI 29 NOV – 5 DES`. |

### Mock values to measure the build against

<!-- gen:mock:angka -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 132 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 76 | 2 px |  |  |
| date column (px) | 166.49 | 4 px |  |  |
| name cell (px) | 421.51 | 4 px |  |  |
| names shortened of 30 | 8 | exact |  |  |
| offering row pitch (px) | 97.33 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 420.25 | 4 px |  |  |
| total figure width (px) | 1201.78 | 4 px |  |  |
| left margin (px) | 40 | 1 px |  |  |
| corner radius (px) | none | 1 px |  |  |
| ground | #F4F1EA | exact |  |  |
| accent | #FFD100 | exact |  |  |
<!-- /gen:mock:angka -->

Card: TPLLIB7, below.

## Family 8 Hangat

<!-- gen:head:hangat -->
**Idea.** One warm hue in tone-on-tone steps with a cream plate, soft cards and faint arches rising behind the header: a look for any season that names no season, with Advent, Christmas and Easter as colourways.

**References it copies** (under `C:\Users\kenne\Shared\inspo\`):
- `artosia-templates/church/id-yt-meSqW3mscuM.jpg`: GPIB Paulus HUT 90: cocoa field, tone-on-tone church line drawing at the right, light serif numeral
- `artosia-templates/galleries/canva-amber-p00.jpg`: one amber field, light serif in spaced caps, a thin date line
- `artosia-templates/galleries/canva-amber-p03.jpg`: three numbered boxes with a thin cream outline on amber
- `artosia-templates/church/id-bethany-home.jpg`: Bethany Christmas banner: deep green with white caps and a tree: a seasonal colourway of a neutral look
- `artosia-templates/church/id-yt-gk-abdiel-qris-maxresdefault.jpg`: warm paper tone, the calmest of the Indonesian offering slides

**Ornaments** (only those the references show): nested arch outlines, tone on tone (`artosia-templates/church/id-yt-meSqW3mscuM.jpg`).
<!-- /gen:head:hangat -->

**Lane.** seasonal-neutral warm.

### Type

<!-- gen:type:hangat -->
| key | family | weight | file | source | figures |
|---|---|---|---|---|---|
| f4 | Fraunces | 400 | Fraunces-400.ttf | new: a soft old-style serif, warm and round, cut at the 48 optical size with SOFT 50; proportional digits, so rows use Instrument Sans for amounts | proportional |
| f6 | Fraunces | 600 | Fraunces-600.ttf | new, same file | proportional |
| f4i | Fraunces | 400 italic | Fraunces-400i.ttf | new, same family italic | proportional |
| g5 | Instrument Sans | 500 | InstrumentSans-500.ttf | bundled | tabular |
| g6 | Instrument Sans | 600 | InstrumentSans-600.ttf | bundled | tabular |

| role | face | px on 1080 | % of height | tracking em | case | colour |
|---|---|---|---|---|---|---|
| kicker | Instrument Sans 600 | 32 | 3.0% | 0.16 | caps | mute |
| title | Fraunces 600 | 112 | 10.4% | 0 | as typed | ink |
| sub | Fraunces 400 i | 56 | 5.2% | 0 | as typed | accent |
| day | Fraunces 600 | 46 | 4.3% | 0 | as typed | ink |
| time | Instrument Sans 600 | 48 | 4.4% | 0 | as typed | accent |
| event | Fraunces 400 | 50 | 4.6% | 0 | as typed | ink |
| tag | Instrument Sans 600 | 32 | 3.0% | 0.1 | caps | on_accent |
| name | Fraunces 400 | 42 | 3.9% | 0 | as typed | ink |
| name_hl | Fraunces 600 | 42 | 3.9% | 0 | as typed | on_accent |
| bday_day | Fraunces 400 i | 40 | 3.7% | 0 | as typed | accent |
| fund | Fraunces 400 | 46 | 4.3% | 0 | as typed | ink |
| amount | Instrument Sans 600 | 46 | 4.3% | 0 | as typed | ink |
| rp | Fraunces 400 i | 34 | 3.1% | 0 | as typed | mute |
| total_label | Instrument Sans 600 | 34 | 3.1% | 0.16 | caps | on_plate |
| total | Fraunces 600 | 96 | 8.9% | 0 | as typed | on_plate |
| bank | Instrument Sans 600 | 44 | 4.1% | 0.16 | caps | accent |
| number | Fraunces 600 | 150 | 13.9% | 0 | as typed | ink |
| holder | Fraunces 600 | 54 | 5.0% | 0 | as typed | ink |
| note | Fraunces 400 | 42 | 3.9% | 0 | as typed | mute |
| caption | Instrument Sans 600 | 32 | 3.0% | 0.16 | caps | mute |
| ref | Fraunces 400 i | 64 | 5.9% | 0 | as typed | accent |
| verse | Fraunces 400 | 56 | 5.2% | 0 | as typed | ink |
| verse_n | Fraunces 600 | 56 | 5.2% | 0 | as typed | accent |
| n_title | Fraunces 600 | 120 | 11.1% | 0 | as typed | ink |
| n_body | Fraunces 400 | 48 | 4.4% | 0 | as typed | ink |
| foot | Instrument Sans 600 | 32 | 3.0% | 0.12 | as typed | mute |
<!-- /gen:type:hangat -->

### Grid

<!-- gen:grid:hangat -->
| token | value | what it sets |
|---|---|---|
| mx | 56 | left and right margin |
| card_y | 316 | card top edge |
| card_h | 704 | name card height |
| card_h3 | 540 | offering card height (the plate overlaps its foot) |
| card_rx | 28 | card corner radius |
| card_pad | 16 | card side padding |
| per | 10 | names per card (3 cards) |
| gutter | 20 | gap between cards |
| day_w | 50 | date column width |
| plate_y | 836 | total plate top edge |
| plate_h | 150 | total plate height |
| name_floor | 40 | smallest name size before the slide splits |
<!-- /gen:grid:hangat -->

### Colour: four colourways and the contrast of every pair

<!-- gen:colour:hangat -->
| role | kakao (Kakao) | adven (Adven) | cemara (Cemara) | gading (Gading) |
|---|---|---|---|---|
| ground | `#3E2A22` | `#3A2340` | `#1E3A2E` | `#F2E8D6` |
| card | `#4D372D` | `#4A2E52` | `#284A3B` | `#FBF5EA` |
| ink | `#F6EAD9` | `#F7ECE3` | `#F4EBD9` | `#3A281F` |
| mute | `#D6C0A8` | `#D9C4D6` | `#C9D8C6` | `#6E5A4B` |
| accent | `#E6B36B` | `#E8BE74` | `#E1BC6C` | `#9A4A22` |
| on_accent | `#3E2A22` | `#3A2340` | `#1E3A2E` | `#FBF5EA` |
| plate | `#F6EAD9` | `#F7ECE3` | `#F4EBD9` | `#3A281F` |
| on_plate | `#3E2A22` | `#3A2340` | `#1E3A2E` | `#F2E8D6` |
| line | `#7A5E4D` | `#7C5A86` | `#5B7F6C` | `#CDBBA0` |

| text on ground | needs | where it is drawn | kakao | adven | cemara | gading |
|---|---|---|---|---|---|---|
| ink on ground | text | kicker, title | 11.35 | 12.09 | 10.43 | 11.50 |
| mute on ground | text | notes, 'Rp' | 7.67 | 8.58 | 8.31 | 5.36 |
| accent on ground | text | italic month | 7.07 | 8.06 | 6.83 | 5.12 |
| ink on card | text | names, fund labels on a card | 9.31 | 10.01 | 8.31 | 12.87 |
| mute on card | text | notes on a card | 6.30 | 7.11 | 6.62 | 6.00 |
| accent on card | text | italic dates on a card | 5.80 | 6.68 | 5.44 | 5.73 |
| on_accent on accent | text | highlighted rows, week pill | 7.07 | 8.06 | 6.83 | 5.73 |
| on_plate on plate | text | total label | 11.35 | 12.09 | 10.43 | 11.50 |
| on_plate on plate | large | total figure | 11.35 | 12.09 | 10.43 | 11.50 |
| ink on ground | large | title | 11.35 | 12.09 | 10.43 | 11.50 |
| accent on card | mark | highlight capsule against its card | 5.80 | 6.68 | 5.44 | 5.73 |
| plate on card | mark | total plate against the card | 9.31 | 10.01 | 8.31 | 12.87 |

Gallery swatch: ground `ground`, accent `accent` (the `swatch` keys of the tokens file).

Least ratio per colourway: kakao 5.80, adven 6.68, cemara 5.44, gading 5.12. Needs: text 4.5, large text and marks 3.
<!-- /gen:colour:hangat -->

### The six kinds at the worst case

| kind | layout | worst case |
|---|---|---|
| S01 schedule | Four day cards in a 2 x 2 grid, each a rounded card with the date numeral in Fraunces 600 and the day name in italic, rows with the time in Instrument Sans 600 accent and the event in Fraunces 400 on up to two lines; the special row on an accent capsule with dark text. | Events wrap to two lines in 640 px; the arches rise behind the title only. |
| S02 birthdays | Drawn: `hangat-S02-birthdays.svg`. Three rounded cards with month heads in italic accent, an accent pill with the range, faint arches behind the header. | Month heads: 5 of 30 names take initials, 13 with a month on every row. |
| S03 offerings | Drawn: `hangat-S03-offerings.svg`. One card of six rows with hairlines, a cream plate overlapping its foot with the total in Fraunces 600. | Amounts in Instrument Sans (tabular); the total in Fraunces is one figure. |
| S04 account | A card holds the bank in accent caps, the number in Fraunces 600 up to 150 px (111 px beside a QR), the holder; the QR on a cream plate at the right, 460 px, overlapping the card edge like the total plate. | Same number and QR rules. |
| S05 verse | Reference in Fraunces italic accent, 64 px, above; the verse in Fraunces 400 down to 44 px, raised accent numbers, left aligned on the ground without a card; a picture is a rounded rectangle at the right 36%. | The tone-on-tone arches fill the corner when there is no picture. |
| S06 notice | Title in Fraunces 600 up to 120 px on at most three lines, body in Fraunces; a picture is a rounded frame at the right 36%; none: the arches rise at the right. | Title down to 64 px. |

Measured with this family's own faces and the fixtures above:

<!-- gen:kinds:hangat -->
| kind | worst case | measured with this family's faces |
|---|---|---|
| S01 schedule | 8 rows over 4 days, 1 special | event 50 px (role 50, height allows 55); longest event 'Malam Pujian dan Penyembahan Khusus Natal' is 1059.45 px in a 640 px cell: 2 lines |
| S04 account | 16-digit number | with QR: 111 px on 1 line in 1100 px; without: 150 px on 1 line in 1700 px |
| S05 verse | 60 words (width x1.12) | with picture 52 px in 8 lines (590.72 of 670 px); without 56 px in 7 lines (556.64 of 670 px) |
| S06 notice | a 45-character title | with picture 120 px on 3 lines (role 120); without 120 px on 2 lines |
<!-- /gen:kinds:hangat -->

### How a highlight, a total, a continuation and a date range read

|  | reads as |
|---|---|
| Highlight | An accent capsule across the run with dark text, bold; accent on card is the mark. |
| Total | A cream plate overlapping the offering card's foot, label left, the figure in Fraunces 600 right. |
| Continuation | `1/2  ...` muted at the foot, left. |
| Date range | An accent pill under the title at the right: `PEKAN INI  29 NOV – 5 DES`. |

### Mock values to measure the build against

<!-- gen:mock:hangat -->
| value | mock | tolerance | built | measured at |
|---|---|---|---|---|
| title size (px, 1080 sheet) | 112 | 2 px |  |  |
| birthday name size at 30 names (px) | 40 | 1 px |  |  |
| birthday row pitch (px) | 60 | 2 px |  |  |
| date column (px) | 50 | 4 px |  |  |
| name cell (px) | 501.33 | 4 px |  |  |
| names shortened of 30 | 5 | exact |  |  |
| offering row pitch (px) | 82.67 | 2 px |  |  |
| amount column, widest 12-digit amount with Rp (px) | 422.07 | 4 px |  |  |
| total figure width (px) | 842.98 | 4 px |  |  |
| left margin (px) | 56 | 1 px |  |  |
| corner radius (px) | 28 | 1 px |  |  |
| ground | #3E2A22 | exact |  |  |
| accent | #E6B36B | exact |  |  |
<!-- /gen:mock:hangat -->

Card: TPLLIB8, below.

## Recommendation and build order

Build **Diagonal first**. It is the layout Indonesian churches already read as a warta card (Jatibaru, Pelita Kasih), it needs no engine capability beyond rotated rects the engine has, its body face is bundled and condensed so the fewest names take initials (5 of 30), and its four colourways (blue, red, green, violet) are the colours a church's own identity uses. Then **Blok, Liturgi and Hangat**, which are flat rects and lines only (Hangat's arch is optional and can ship later). Then **Angka and Majalah**, which each cost a rule or a few words in ten languages. Last **Cahaya** (a glow primitive) and **Kaca** (the photo fallback and plate), which are the two that need the engine to learn something.

| family | new font cuts | needs from the engine | new words | release |
|---|---|---|---|---|
| 1 Diagonal | 2 (Montserrat 700, 800) | none (rotated rect and a mask) | none | first |
| 4 Blok | 2 (Unbounded 700, 800) | none | none | second |
| 2 Liturgi | 3 (Cinzel 600, 700; Source Serif 4 600) | none | none | second |
| 8 Hangat | 3 (Fraunces 400, 600, 400 italic) | month_runs rule; an open path for the arches (optional) | none | second |
| 7 Angka | 2 (Big Shoulders 800, 900) | six fund hues; a derived share per fund; a week index per name | six: Tgl, Nama, No, Dana, Bagian, Jumlah | third |
| 6 Majalah | 4 (Libre Caslon Display 400; Text 400, 700, 400 italic) | month_runs rule | two: Dana, Jumlah | third |
| 5 Cahaya | 2 (Sora 300, 600) | radial glow: translucent circles or a baked image (UNSURE: banding); month_runs rule | none | fourth |
| 3 Kaca | 4 (Jost 300, 500, 600; Allura 400) | photo ground with a scrim and a plate; gradient fallback | none | fourth |

Open questions only the owner can answer, in the order they change the build:

1. Month on every row (the engine today) or a month heading per column (three families here)? The fits table gives the numbers: with a month on every row 17 (Cahaya), 20 (Majalah) and 13 (Hangat) of 30 names take initials, against 6, 6 and 5 with headings.
2. May a church's long name drop to first initial and surname (`Bpk. M. Lumbantobing`) before it drops to bare initials (`Bpk. M. T. L.`)? The wireframes show the first.
3. Should a fund keep one fixed hue (Angka) or take its colour from the church's own colourway? Fixed hues are the only way six funds stay distinguishable on a projector.
4. The new fixed words for Angka and Majalah need all ten languages written from the screen, not translated from Indonesian (blind-translation).
5. Kaca needs the church's own photo to look like its mock; a church with none sees the colourway gradient. Is that acceptable as a gallery tile?

## Cards

Each card is one family. Shared acceptance criteria (AC) apply to every card; a card's own criteria follow. Shape (creation-pattern.md): a family is one **Screen** unit (`crates/artosia-slides/src/warta/d<N>.rs`, composing the shared parts `fit_cells`, the name ladder, `money`, `verse` and `paginate` over `Data`) plus one **Data** unit (`assets/warta/directions/d<N>.json`: fonts, type, grid, kinds, colourways, pairs, foot) and one line each in `FAMILIES`, `family_of` and `foot`. No second copy of a rule: the month-heading rule is one function in `data.rs`. The least code: Diagonal needs no new primitive; nothing here adds a dependency except the font files.

### TPLLIB0 Shared rules of the library

Done when (AC):

1. For each of the six kinds, the fixture slide draws in the family with nothing clipped at 1920 x 1080 in Indonesian and English (`tools/notice-matrix.sh id` and `en`), or splits into parts that each pass `holds` (the worst cases: 8 rows over 4 days with 1 special; 30 names, 7 highlighted, two months; 6 funds with 12-digit amounts and a total; an account with and without QR; a 60-word verse; a notice with and without a picture).
2. A 640 x 360 render breaks every line where the 1920 x 1080 render does.
3. No body text is under 40 px and no label under 32 px at 1080; a slide that needs less splits.
4. Every pair in the family's `pairs` passes its threshold (text 4.5, large text and marks 3) in all four colourways; the existing contrast test (`every_colourway_of_every_direction_reads_on_every_pair_its_layout_draws`) runs over the new family and fails on a pair below threshold.
5. Every fixed word on a slide comes from `Words` in the window's language: an English window shows `Church news`, `29 Nov – 5 Dec`, English month headings.
6. The date range is `Words::range` unchanged.
7. A highlight is fill plus weight plus a 3:1 mark, and reads as the first, the middle, the last, every and no name of 30.
8. The total is derived, reserved first, at least row size, never clipped at 13 digits; each part of a split card carries the whole total.
9. A part of a split card draws `n/m` and its range at the family's foot place and colour.
10. A card saved with the plain family code reads as the family's first colourway; an unknown colourway reads as the first.
11. The gallery lists the family with four swatches from its `swatch` keys; the notice matrix renders all six kinds for every colourway; `family_of`, `foot`, `build` and `FAMILIES` each name the family.
12. With a picture, a notice or verse uses the family's picture rule; without, the text redraws and no hole is left.
13. A QR is black on white with a four-module quiet zone in every colourway.
14. Each new font cut loads (`fonts::face_bytes`) and its OFL text is in `assets/warta/licences/`.

Contexts: language AC5; data none, one, many, more than a page AC1 AC3 AC8 AC9; device n/a: a slide is one 1920 x 1080 drawing and a gallery tile is that drawing scaled (AC2); account bought, who acts, offline, a lost answer n/a: a free desktop program draws a slide from one card and none of these reaches the drawing; settings: light and dark colourways AC4; the entry: gallery pick, style rail, saved look and imported card all store `family/colourway` AC10 AC11.
Neighbours: every exit: a card whose family is removed falls back to the default (AC10); every route to one outcome: all routes write one style string (AC10); the list is closed: every reader of the style list (gallery, rail, `family_of`, `foot`, `build`, swatches, `tools/notice-matrix.sh`, the contrast test) names the new family (AC11); the old client: an older build reading a style code it does not name draws d4 in `family_of` (`_ => "d4"`), UNSURE whether the parser rejects it first, so a card saved in a new family is checked in an older build before release; replay, partial failure, locks, audit rows n/a: the change writes one string and no row.

Mock and proof: the mock of each family is its two SVGs and `contact-sheet.png`. This is a projector program with no phone or tablet slide (the phone only edits the text of a slide in its own sheet, unchanged by a layout), so the proof is the builder's `notice-matrix.sh` renders of the same two scenes at 1920 x 1080 and at the gallery tile width, laid beside the mock PNG, with each family's mock-values table filled in (built value and where it was measured). Keyboard: n/a, the mock draws no text field; on-slide editing is the existing warta box editor.

### TPLLIB1 Diagonal

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Diagonal` as above; no new dependency except 2 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. The header block and the slanted total plate are drawn with a rotated rect masked by the page colour (no polygon primitive); the slope of the plate's left edge equals the header's (`cut` over `hdr_h`), and the render matches the mock within 2 px.
16. At 30 names the date tile is as wide as the widest 'dd Mon' plus 28 px and at most 5 names take initials; the 7 highlighted tiles are solid brand with white text.
17. The Montserrat 700 and 800 cuts are bundled with their licence and used for the title, the range, the amounts and the total only.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0.

Mock: `docs/plans/artosia/templates-library/diagonal-S02-birthdays.svg`, `diagonal-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 1 filled.

### TPLLIB2 Liturgi

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Liturgi` as above; no new dependency except 3 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. The double frame, four corner diamonds and the cross are drawn from rects, lines and rotated squares; the frame and its diamonds lie wholly inside the sheet at 1920 and at the 640 tile.
16. At 30 names at most 8 take initials; the highlighted run lies on the parchment band with ink text, over both column breaks when the run crosses one.
17. Cinzel is set in capitals only (title, labels, kicker, total); names and figures are Source Serif 4.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. A church that has a seal may later want the Bukit Zaitun lockup: n/a, the data model has no logo field, the cross is the family's mark.

Mock: `docs/plans/artosia/templates-library/liturgi-S02-birthdays.svg`, `liturgi-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 2 filled.

### TPLLIB3 Kaca

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Kaca` as above; no new dependency except 4 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. With a photo, the scrim at 0.62 keeps white text at 4.5:1 over the brightest pixel in all four colourways (the contrast table); with none the colourway's gradient is the ground and the layout does not change.
16. No text ever sits on the photo outside the scrim; every data row sits on the plate at 0.92 opacity, and the plate is flat (no blur is asked of the engine).
17. A 16-digit account number beside a QR is set in two lines of eight at 140 px, never one line under 96 px.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. The picture is the ground: `Have.photo` false draws the fallback, and a picture with a face under the plate area is the church's choice (the plate keeps text off it).

Mock: `docs/plans/artosia/templates-library/kaca-S02-birthdays.svg`, `kaca-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 3 filled.

### TPLLIB4 Blok

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Blok` as above; no new dependency except 2 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. Every block edge is placed on a whole pixel of the 1920 sheet; the highlighted run is one merged signal rect per column.
16. The total figure is solved to the room the label leaves and never clips at 13 digits (it reaches its 100 px maximum in the fixture); `Rp` sits in its own place before it.
17. Unbounded appears only in titles, the day cells, times, amounts and the total, never in names.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0.

Mock: `docs/plans/artosia/templates-library/blok-S02-birthdays.svg`, `blok-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 4 filled.

### TPLLIB5 Cahaya

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Cahaya` as above; no new dependency except 2 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. The orb is a stack of translucent circles or one baked image per colourway and shows no visible band at 1920 x 1080 (a person looks; if it bands, the baked image is used); no body text sits on the orb, which lies above the first body row.
16. A page across two months writes a month heading at the top of each column and where the month changes inside a column, and each row writes its day only; the names that take initials at 30 names are 6 or fewer.
17. Sora 300 is used at 96 px and up only (title, total, account number, notice title) and never for names; the glass capsule is white at 12% and the contrast table passes on it.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. The month-heading rule is a second reading of `spans`: one pure rule in `data.rs`, used by three families, not three copies. The orb needs a radial glow the engine does not have (a stack of circles or a baked image).

Mock: `docs/plans/artosia/templates-library/cahaya-S02-birthdays.svg`, `cahaya-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 5 filled.

### TPLLIB6 Majalah

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Majalah` as above; no new dependency except 4 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. The month-heading rule of the shared card is used (`Data::month_runs`); at 30 names 6 or fewer take initials, and a heading counts as a row in the column's height.
16. The four corner lines follow their slots (top left warta, top right church, bottom left date, bottom right the 1/2 folio) and the double rule appears under the masthead and above the total only.
17. Libre Caslon Display sets the headline, the total and the account number only; everything read at length is the Text cut.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. The month-heading rule is the same rule function as Cahaya and Hangat, not a copy.

Mock: `docs/plans/artosia/templates-library/majalah-S02-birthdays.svg`, `majalah-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 6 filled.

### TPLLIB7 Angka

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Angka` as above; no new dependency except 2 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. Six words are new to the fixed words and exist in all ten languages before release: Tgl, Nama, No, Dana, Bagian, Jumlah.
16. Each fund keeps its hue from chip to bar in order (fund 1 to 6); a share equals amount over total to one decimal and the six shares sum to 100.0 within rounding; the total is derived, never typed.
17. The week bar of a row is the week of its date counted from the page's first Sunday (`Data::week_of`, a pure rule), hue 1 to 5; a second name on the same day leaves its date cell empty, and no run of highlight is split by the bar.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. Six new words reach every language through `NoticeWords`, as the commit 8a8864b did for eight others; the week index is one pure rule in `data.rs`.

Mock: `docs/plans/artosia/templates-library/angka-S02-birthdays.svg`, `angka-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 7 filled.

### TPLLIB8 Hangat

Shape: Screen `d<N>.rs` and Data `d<N>.json` for the family `Hangat` as above; no new dependency except 3 new font cuts (the `font_source` column of its font table).

Done when: AC1 to AC14 of TPLLIB0, and:

15. The month-heading rule is used (`Data::month_runs`); at 30 names 5 or fewer take initials.
16. The arches are one open path drawn at 12% opacity behind the header only and are absent where a picture sits; no icon or season word appears on any slide (the look is neutral, the colourway carries the season).
17. The cream plate overlaps the offering card's foot by 20 px and never covers a fund row.

Contexts: language AC5; data none, one, many, over a page AC1 AC3 AC8 AC9; device n/a: one drawing scaled (AC2); account bought, who acts, offline n/a: nothing but the card's style string reaches the drawing; settings: four colourways AC4; entry: gallery, rail, saved look, imported card AC10 AC11.
Neighbours: every exit and route: AC10; the list is closed: `family_of`, `foot`, `build`, `FAMILIES`, swatches, matrix script, contrast test, gallery AC11; old client: AC10 and the d4 fall-back of TPLLIB0. The month-heading rule is the same rule function as Cahaya and Majalah, not a copy.

Mock: `docs/plans/artosia/templates-library/hangat-S02-birthdays.svg`, `hangat-S03-offerings.svg`. Proof: TPLLIB0, with the table of Family 8 filled.

## Build spec, mock proof and check output

- Spec file: `docs/plans/artosia/templates-library/spec.json` (data, families, colourways, pairs, kinds, foot, ledger; below `generated`: every scene's resolved geometry, the contrast of every pair, the measured fits).
- Tool: `docs/plans/artosia/templates-library/build/` (`make_fonts.py` cuts the static fonts, `lib.py` and `families.py` hold the layout rules, `build.py` runs `layout`, `draw`, `check`, `render`, `doc`). Run from that folder with `python -I`.
- Rendered by headless Google Chrome 157 from the SVGs, one PNG each, then a Pillow contact sheet. No source file of the application was changed, no commit made, no WSL used.

Output of `python -I build.py check`:

```
numbers: 4907 numbers in 16 SVG attributes, 0 files hold a number spec.json lacks
selftest: an invented font-size 45.37 is caught
contrast: 352 pairs over 8 families x 4 colourways, 0 below threshold
glyphs: every character of every drawn string is in its font: True
fits: every fund label on at most two lines, every name at or above the 40 px floor
ledger: rows=30 mapped=27 dropped=3 unmapped=0
check: ok
layout (same run): 16 scenes, 352 contrast pairs, 0 failing
```

## Brief ledger

Every line of the brief and of the standing instructions, and where it is honoured. This file is a spec, not a deck; the ledger is kept because the same check script that guards the numbers also fails while a row is unmapped.

<!-- gen:ledger -->
| id | source | owner item | honoured by |
|---|---|---|---|
| L01 | brief | Read the named inspo files and the colourways-and-archetypes decision first; every choice cites a saved file | section:what-this-is-and-how-to-read-it, section:family-1-diagonal |
| L02 | brief | Eight new template families, each a clearly different approach, never one layout recoloured | section:the-eight-in-one-table, file:spec.json |
| L03 | brief | Do not duplicate the minimal default candidates of templates-default.md | section:the-eight-in-one-table |
| L04 | brief | At least two drawn from Indonesian church media (church lane) | family:diagonal, family:liturgi |
| L05 | brief | One photo-led family | family:kaca |
| L06 | brief | One bold colour-block family | family:blok |
| L07 | brief | One gradient or soft-light family | family:cahaya |
| L08 | brief | One magazine editorial family | family:majalah |
| L09 | brief | One data-first family (tables and big numerals) | family:angka |
| L10 | brief | One seasonal-neutral warm family | family:hangat |
| L11 | brief | Per family: the idea in one sentence and the references it copies, by file | section:family-1-diagonal, section:family-2-liturgi, section:family-3-kaca, section:family-4-blok, section:family-5-cahaya, section:family-6-majalah, section:family-7-angka, section:family-8-hangat |
| L12 | brief | Per family: type (families, weights, px sizes on 1920x1080 for every role), grid (margins, columns), ornaments only the references show | section:family-1-diagonal, family:diagonal, family:liturgi, family:kaca, family:blok, family:cahaya, family:majalah, family:angka, family:hangat |
| L13 | brief | Per family: colour roles, four colourways with hex, contrast ratio of every text-on-ground pair (4.5 body, 3 large and marks) | section:family-1-diagonal, file:spec.json |
| L14 | brief | Per family: all six kinds at the worst case (8 rows over 4 days with one special; 30 names in three columns, 7 highlighted, two months; 6 funds with 12-digit amounts and a total; account with and without QR; 60-word verse; notice with and without a picture) | section:family-1-diagonal, section:rules-every-family-keeps |
| L15 | brief | How a highlight, a total, a continuation slide (1/2) and a date range read in each family | section:family-1-diagonal, section:rules-every-family-keeps |
| L16 | brief | Draw each family's birthday and offering slide at 1920x1080 as SVG wireframes in templates-library/ | file:diagonal-S02-birthdays.svg, file:diagonal-S03-offerings.svg, file:liturgi-S02-birthdays.svg, file:liturgi-S03-offerings.svg, file:kaca-S02-birthdays.svg, file:kaca-S03-offerings.svg, file:blok-S02-birthdays.svg, file:blok-S03-offerings.svg, file:cahaya-S02-birthdays.svg, file:cahaya-S03-offerings.svg, file:majalah-S02-birthdays.svg, file:majalah-S03-offerings.svg, file:angka-S02-birthdays.svg, file:angka-S03-offerings.svg, file:hangat-S02-birthdays.svg, file:hangat-S03-offerings.svg |
| L17 | brief | Write the spec to docs/plans/artosia/templates-library.md | section:what-this-is-and-how-to-read-it |
| L18 | brief | One contact sheet PNG of the SVGs, saying which tool rendered them | file:contact-sheet.png, section:build-spec-mock-proof-and-check-output |
| L19 | brief | A family may name another SIL OFL family from github.com/google/fonts, with the reason | section:family-1-diagonal, family:diagonal, family:liturgi, family:kaca, family:blok, family:cahaya, family:majalah, family:angka, family:hangat |
| L20 | brief | Never run WSL; change no source code | section:build-spec-mock-proof-and-check-output |
| L21 | system | Read the design-questions index; every card has a Contexts line and a Neighbours line, each item an acceptance criterion or n/a with a reason | section:cards |
| L22 | system | Draw at least two designs under different pulls, compare them in a table, recommend one | section:the-eight-in-one-table, section:recommendation-and-build-order |
| L23 | system | Every plan card lists acceptance criteria as checkable statements a tester can write failing tests from | section:cards |
| L24 | system | A screen card names its mock and states the proof is the builder's screenshots against that mock with a table of mock values against measured values | section:cards, section:family-1-diagonal |
| L25 | system | Every text field the mock draws names its keyboard behaviour on iPhone and Android | dropped: the mock draws no text field: projected slides are read-only, on-slide editing is the existing warta box editor and the phone edits in its own sheet, both unchanged |
| L26 | system | A design a builder will build ends in a build spec: one data file, exact drawings, the mock drawn from that file, a check script that prints numbers the file lacks | file:spec.json, file:build/build.py, section:build-spec-mock-proof-and-check-output |
| L27 | system | Every unit takes one of the five shapes of creation-pattern.md and is the least code that solves it | section:cards |
| L28 | system | A loop is judged by speed and acceleration across its seam | dropped: the slides are still frames; no motion is drawn or specified |
| L29 | system | Visual artwork (logo, mark, brand kit) loads the brand skill and writes a Core Brand first | dropped: no logo, mark or brand kit is drawn; the families are slide layouts and their only marks are layout ornaments cited from the references |
| L30 | system | A deck or canvas carries a source ledger with a check script that fails while a row is unmapped | section:brief-ledger, file:build/build.py |
<!-- /gen:ledger -->
