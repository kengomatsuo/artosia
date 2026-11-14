# The default announcement template: four candidates (2026-11-15)

Status: design, for the owner to pick one. Nothing here is built; no source file was changed. Every number in the tables below is generated from `templates-default/spec.json` by `build/docs.py`, and every wireframe is drawn from the same file.

Files, all in `docs/plans/artosia/templates-default/`:

| File | What it is |
|---|---|
| `contact-sheet.png` | the 12 required wireframes on one sheet: for each candidate the schedule, the 30-name birthday page and the two-month birthday page 1/2 |
| `colourways-sheet.png` | each candidate's schedule in its three colourways |
| `a-schedule.svg`, `a-birthdays.svg`, `a-birthdays-two-months-1of2.svg` (and `b-`, `c-`, `d-`) | the wireframes at 1920x1080, with the faces embedded so a browser draws them as designed |
| `other-kinds/` | offerings, account with and without QR, verse, notice with and without a picture, per candidate (24 SVG) |
| `colourways/` | schedule and birthdays in all three colourways, per candidate (24 SVG) |
| `spec.json` | the build spec: tokens, data, every element's geometry, fit results, contrast results |
| `tokens/a.json` to `d.json`, `tokens/data.json` | the hand-written part of the spec; `a.json` to `d.json` have the shape of `crates/artosia-slides/assets/warta/directions/dN.json` so a builder copies one |
| `build/` | `make_fonts.py`, `build.py`, `check.py`, `docs.py`, the four layout files and `kit.py` |

Paths in this page are relative to `C:\Users\kenne\Shared\inspo\artosia-templates\` (`products\`, `galleries\`, `church\`, `languages\`) unless they start with `notice-designs\` (`C:\Users\kenne\Shared\inspo\artosia-notice-designs\`) or `repo:` (`F:\Repositories\artosia\`). Wireframes were rendered to PNG with resvg (`bun x @resvg/resvg-js-cli`, system fonts off) and one SVG was opened in Edge to confirm the embedded faces draw. The contact sheet is a PIL montage of those PNGs.

Contexts: language is a data context (the fixed words come from `Words`, so a window in English or German reaches every candidate) and is accepted by AC-9; device is a projector at 1920x1080, a zoomed second screen and a scaled preview, accepted by AC-10; data is none, one, thirty, forty and a page of two months, accepted by AC-2, AC-3 and AC-11; the colourway is any of three or an unknown id, accepted by AC-1 and AC-12; a picture or a QRIS image present or absent, accepted by AC-6 and AC-8; the entry (a new card by hand, by import, by phone) is n/a: all three read one default through one function, accepted by AC-13.

Neighbours: every card already saved keeps its stored style and a new card gets the new default, accepted by AC-13; every reader of the style list (gallery tiles, style rail, phone editor, saved looks) is walked before the new family is added, accepted by AC-14; the old client opening a library that names the new family draws the first colourway of a family it knows, accepted by AC-12; a font the machine lacks is n/a: every face is embedded in the binary (`crates/artosia-slides/src/fonts.rs` `FACES`); replay, ownership and partial failure are n/a: a template is read-only data.

## 1. What the references say, and what is rejected

The brief and the screens: `brief.md`, `screens.md` (S08 is this card). The rejected default is `repo: crates/artosia-slides/assets/warta/directions/d5.json` (a white ground, one Instrument Sans, hairlines; its `language` field says "Plain ground, one sans, rows on hairlines") built by `repo: docs/plans/artosia/defaultstyle1.md`. Its title is 88 px over 52 px rows (under 2 to 1), it has one family and a blue accent `#1A73E8` and nothing else, which is what the owner called basic Arial. The existing `d1` (Bodoni caps, a hairline frame, greige) is a styled direction, not a default, and none of the candidates copies its frame or its didone.

What makes a slide look designed without decoration, from the saved references:

| Move | Where it is seen | Which candidate takes it |
|---|---|---|
| Scale contrast alone: one huge light numeral or title, tiny meta | `products/pitch/kpi-report.jpg` (title about 62% of height, two-tone, 6% margin, hairline strip with meta), `galleries/sg-planner-s23.jpg` (numeral about 25% of height), `galleries/sg-bizslides-s18.jpg` | B, A |
| Roman and italic of one serif as the whole type system, italic numerals, one gradient as the only colour | `galleries/sg-grad-s00.jpg`, `sg-grad-s02.jpg`, `sg-grad-s12.jpg` | A |
| A serif with tracked caps labels and a top rule | `products/powerpoint/appstore-ipad-03.png`, `products/beautifulai/area_chart-2.jpg` (light serif figure under a tracked-caps label) | A |
| The first column as the only dark element | `galleries/sg-bizslides-s21.jpg` | B |
| Big date numerals over day abbreviations | `galleries/canva-sched-07.jpg`, `galleries/canva-sched-01.jpg` | B |
| Two weights and two tones inside one title | `products/gamma/quiet-deck.jpg` | B, C |
| One flat field, two sizes of one face | `galleries/canva-amber-p00.jpg`, `canva-amber-p03.jpg`, `canva-amber-p08.jpg` | A (the amber colourway), C |
| A solid pill under the special row, the reference in a pill | `galleries/canva-sched-05.jpg`, `galleries/canva-borcelle-p01.jpg` | C |
| Mono caps on warm beige, rows on hairlines | `products/googleslides/appstore-ipad-01.png`, `appstore-ipad-03.png` | D |
| A black header band and inverted rows for the highlight | `galleries/sg-gather-s24.jpg`, `sg-gather-s30.jpg`, `sg-gather-s31.jpg` | D |

What none of them does, and why (the church lane shows the other side of the market): clip art, confetti, calendar badges, neon and busy gradients behind text (`church/cv-upcoming-events-social.jpg`, `church/cv-missions-offering.jpg`, `church/cv-john316-verse.jpg`), an event shrunk to a footnote (`church/notes.md` row 32), a dense dated block unreadable at distance (row 18), a names grid too small for a hall (row 28). The data rules every candidate keeps come from `notice-designs\README.md` section 3: a total reserved before the rows, a highlight that is shape plus weight plus colour, a name that is shortened and never clipped, an amount column measured before the labels, the real QRIS card on a light ground (`church/id-gkin-qris-persembahan1.webp` is black on white).

## 2. The four, side by side

| | Idea | Feels like |
|---|---|---|
| **A Serif Italic** | A serif in roman and italic on warm off-white, italic date numerals, one colour wash rising from the bottom edge. | a printed order of service |
| **B Swiss Numerals** | A flush-left grid where the date is the biggest thing, set in the one dark column, with one red accent. | a timetable poster |
| **C Quiet Tint** | One sans in two weights on a tinted ground; the only accent is a solid pill under whatever is special. | a calm modern deck |
| **D Civic Board** | A departure board: black header band, mono figures, condensed words, an inverted row for the highlight. | a transit board |

Measured with the real faces on the same worst-case data (generated):

{{compare}}

Read the table like this. D is the most legible from the back of a hall (largest rows, highest contrast, the strongest highlight). B has the largest hero figure. A is the warmest, its ink is 16.6:1 on the light ground and never under 6.95:1 anywhere (ink on the dark wash). It is also the heaviest to ship (section 8). C is the softest and the cheapest to ship and most likely to disagree with a church's colours (lilac and indigo). All four hold 30 names on one slide at 40 px or more when the page is one month.

**Recommendation: A, Serif Italic.** Reasons, each from this page: it is the only candidate whose colour is a ground treatment rather than a shape in the rows, so a church can keep its data slides black on off-white at 16.6:1 and still see colour; it reads as the church's own printed paper rather than software, which is what a default kept for years should do; its italic date numerals do the job B does with a dark block, with no dark mass in the room; it adds one family whose licence is already in `assets/warta/licences/sourceserif4-OFL.txt`. Its cost is size and it is the largest of the four: seven cuts, 1.9 MB full or 0.6 MB as Latin subsets (table in section 8); a Source Serif 4 Text 400 cut is already bundled but at optical size 36 (`notice-designs\directions\build\make_fonts.py` line 51), and the mock's Text cuts are optical size 24, so the builder either reuses the bundled cut for rows and re-measures, or adds the new cuts under other file names. If the owner wants the loudest, easiest-to-read-from-far default instead, pick D; its extra cost is two mono cuts (268 KB full, 45 KB Latin). B is the choice for a church that wants dates to carry the slide. The owner decides; the numbers above are the evidence.

## 3. Rules every candidate shares

- **Sheet.** 1920x1080, every size a fraction of the height (the rule in `repo: .claude/rules/output.md`), so a thumbnail and a projector draw the same slide. Margins are 5.0 to 5.8% of the width (96 to 112 px), inside the title-safe area.
- **Floors.** No text under 32 px; names, rows, funds, verse and notes at 40 px or more (`notice-designs\README.md` section 3 rule 8; `check.py` enforces it on every wireframe).
- **Fit.** The existing machinery is reused unchanged: `fit::ladder` (a name whole, else its middle names as initials, never clipped), `largest_name_size`, `fit_cells`, `fit::balanced`, `paginate` (a card that does not hold at the floor is cut in halves; `crates/artosia-slides/src/warta/fit.rs`, `mod.rs`).
- **One accent at most, and never alone.** A highlight is a shape plus a heavier weight plus a colour, and a special schedule row also carries the word `Khusus`. Reading the slide in greyscale loses nothing.
- **Numerals.** Every figure is tabular so amounts, times and days align in columns. The renderer already asks for it (`TextStyle::features("'tnum' 1")`, `repo: crates/artosia-slides/src/notice.rs` line 174, `audience.rs` line 354). Faces whose default digits are already tabular need no feature (verified from the font files: Source Serif 4, Hanken Grotesk, IBM Plex Mono and Plex Sans Condensed each have one digit advance; Figtree has `tnum` and the wireframe bakes it in).
- **Money.** `Rp` set small beside the digits, dot thousands, no suffix, the style in `shared.json` (`money.prefix "Rp "`, `thousand "."`). The amount column is measured from the widest amount first; labels wrap or shrink, an amount never does.
- **Date range.** The page's week is written once, in the title row, as `15 sampai 21 November 2026`; a highlighted range is named by its own days (`8 sampai 14 November`) and never by "this week" (commit 4c44e0b). The mock writes the word `sampai` where `Words::range` writes an en dash (`repo: crates/artosia-slides/src/warta/data.rs` line 93), because `docs/design-questions.md` section 8 bans a dash in a mock string; the builder keeps `Words::range` as it is.
- **QR code.** Always on a light plate with a quiet zone, in every colourway, because a dark ground makes it unscannable (the mock draws the plate in `card`, `#FFFFFF`).
- **Continuation.** A page that does not hold at the floor becomes `1/2` and `2/2`, each half drawn at a larger size in two columns, the foot reading `1/2` and the range that half holds (`range_of`: `12 Okt sampai 4 Nov`). A highlight that continues over the cut is drawn on both slides.
- **Month crossing.** A page whose names cross a month writes each day with its month (`Data::day_of`); that costs 65 to 75 px per row (the day cell is 109 to 126 px at page 1/2 against 42 to 53 px for one month, table in section 4 to 7), so at 30 names the longest name no longer fits three columns and the page continues. The stress table in section 7 shows it for all four.

## 4. Candidate A: Serif Italic

**Idea.** One serif in roman and italic on warm off-white; dates are italic numerals; data figures are tabular lining; the only colour is one soft wash rising from the bottom edge and a vermilion that marks the highlight.

**References.** `galleries/sg-grad-s00.jpg`, `sg-grad-s02.jpg`, `sg-grad-s12.jpg` (roman plus italic of one serif is the whole system, italic numerals, one gradient); `galleries/sg-bizslides-s18.jpg` (serif numerals about a quarter of the height, warm off-white); `products/powerpoint/appstore-ipad-03.png` (thin top rule, tracked caps label); `products/beautifulai/area_chart-2.jpg` (a light serif figure under a tracked-caps label for the total); `galleries/canva-amber-p00.jpg`, `canva-amber-p03.jpg` (the tinted colourway: one amber field, cream light serif). `notice-designs\README.md` language L2 and `notice-designs\slides\51-canva-anniversary-programme-order-of-service.jpg` (the time as a real column).

**Type.** Source Serif 4 (SIL OFL, `github.com/google/fonts` `ofl/sourceserif4`), cut static from the variable font: Display (optical size 60) 600 and 400 and italic 400 for the title, figures and total; Text (optical size 24) 400, 600 and italic 400 for rows; Display italic with `'onum' 1` baked for the date line only. Figures are tabular lining by default in every cut (digit advance 470 to 517 of 1000 within a cut, equal across digits, verified), old-style only in that date line.

{{type:a}}

**Grid.** Margins 112 px each side (5.8% of the width). A 2 px ink rule at y 106 under a tracked-caps kicker at baseline 84 (left the word `Warta jemaat`, right the church). Title baseline 262, left. The week, the month and the highlight range share that baseline: the week at the right in oldstyle italic for the schedule; for birthdays the month follows the title and the range sits at the right in accent italic. The body is 332 to 952; the wash is the bottom 220 px (y 860 to 1080), a vertical gradient from transparent to the wash colour, so no text sits on more than its faint upper part.

{{grid:a}}

Colour roles: `paper` ground, `ink` text, `mute` second tone, `rule` hairlines, `tint` highlight band, `accent` the highlight bar and the range, `wash` the bottom colour, `card` the QR plate.

{{palette:a}}

{{contrast:a}}

**The six kinds.**

| Kind | Layout |
|---|---|
| Schedule | Rows grouped by day. At the left an italic date numeral (96 px) over the weekday in tracked caps; then the time as a tabular semibold column; then the event. A 1.5 px hairline between days. The special row has a tint band across the page, a 6 px accent bar at its left edge, semibold event text and the word `Khusus` in accent italic at the right. |
| Birthdays | Three columns by count (1 up to 8 names, 2 up to 20, 3 up to 30). Each row: the day as a tabular numeral in `mute`, then the name; with a page of two months the month follows the day in italic 32 px. A highlight is a tint band across the contiguous highlighted rows of a column with a 6 px bar, the name in semibold and the day in `ink`; it continues in the next column when the run crosses the break. The range sits in the title row. |
| Offerings | Italic numerals 01 to 06 in `mute`, the fund label, `Rp` in small italic and the amount right-aligned. A 2 px ink rule closes the list, then the total: the label in tracked caps at the left and the sum in Display 400 at 144 px with an italic `Rp`, the whole block reserved before the rows are laid out. |
| Account | Bank in tracked caps, the number in Display 400 up to 200 px, a rule, `atas nama` in italic, the holder in Display 600, the notes at 40 px. With a QR it takes the right 480 px on a white plate with `Pindai dengan QRIS` in caps under it; without, the stack takes the full width and the number grows. |
| Verse | Kicker `Ayat Pekan Ini` in accent caps, the reference in accent Display italic 72 px, then the verse in Text 400 with raised accent verse numbers (60% size, rise 0.34). The size is searched from 64 px down to a 44 px floor against the room; 60 words settle where the table says. |
| Notice | Kicker in accent caps, the title in Display 600 up to 144 px (two lines at most), a 96 px accent rule, paragraphs at 48 px. With a picture the picture takes the right 600 px and the text the left; without, the text takes 1500 px and stays left. |

{{fit:a}}

**How it reads.** The highlight is a tint band plus the accent bar plus semibold plus (on the schedule) the word. The total is the only large figure on its slide and sits under a rule. The continuation foot is `1/2   12 Okt sampai 4 Nov` in 32 px semibold ink at the bottom left, over the faint wash. The date range is one italic line at the right of the title, in oldstyle figures; the highlighted range is the same line in accent. Dark colourway: ink `#F4ECDF` on `#181510`, a deep amber wash. Tinted colourway "Ambar": cream ink on one flat amber `#7A3F0A` with a darker wash, the Canva amber field.

**Never.** No frame, no icon, no second accent, no gradient except the one wash, and text on it is judged against the wash's darkest colour (the pair `ink` on `wash`), no all-caps title, no drop shadow, no stock picture.

## 5. Candidate B: Swiss Numerals

**Idea.** A flush-left grid where the date is the biggest thing: day numerals in a light neo-grotesk sit in the one dark column, the title is two tones in two weights, rows sit on hairlines, and one red marks the highlight and the total.

**References.** `products/pitch/kpi-report.jpg` (a huge light neo-grotesk, two tones, a left margin of about 6%, a hairline strip at the foot holding the small meta); `galleries/sg-bizslides-s21.jpg` (a schedule whose only dark element is the first column, times in cream); `galleries/canva-sched-07.jpg`, `canva-sched-01.jpg` (big date numerals over a day abbreviation, hairline row rules); `galleries/sg-planner-s23.jpg` (one huge numeral, scale contrast alone); `products/gamma/quiet-deck.jpg` (two weights in one title); the sage ground of `kpi-report.jpg` is the tinted colourway.

**Type.** Hanken Grotesk (SIL OFL, `ofl/hankengrotesk`), static cuts 300, 500, 700 from the variable font. Chosen over Archivo, Inter Tight and Instrument Sans after a side-by-side render because its light weight keeps the kpi-report character at 100 px and above, and its default digits are tabular (digit advance 560 in every cut), so no feature is needed. Title: the first word in 300 and `mute`, the rest in 700 and `ink`.

{{type:b}}

**Grid.** Margins 96 px. The strip is at the foot: a 2 px ink rule at y 1000 with the kicker (left `Warta jemaat`, right the church) at baseline 1050, as in `kpi-report.jpg`. Title baseline 188. Body 256 to 968. The schedule has a 288 px dark block per day at the left (the date numeral in 300 at 112 px, the weekday in tracked caps), then time at x 432 and event at x 600.

{{grid:b}}

{{palette:b}}

{{contrast:b}}

**The six kinds.**

| Kind | Layout |
|---|---|
| Schedule | One dark block per day group holding the date numeral and the weekday; rows to its right on 1.5 px hairlines; the special row is a solid accent band from the block to the right margin with paper-coloured text, semibold, and `Khusus` in caps at its right end. |
| Birthdays | Three columns, a hairline under every name, the day tabular bold in `mute`. A highlight is a solid accent rectangle behind the contiguous highlighted names of a column, text in `paper`, semibold, crossing the column break as two rectangles. |
| Offerings | Label left, amount right with a small `Rp`, a hairline per row. Below a 2 px rule the total is the hero: the label in tracked caps, then the sum in Hanken 300 at 192 px in the accent, `Rp` in 72 px. |
| Account | Bank in tracked caps, the number in 300 up to 208 px, a rule, holder in 700, notes; QR on the right 480 px plate. |
| Verse | Kicker, the reference in 300 at 96 px, the verse in 500 with bold accent numbers. |
| Notice | Title in 300 up to 152 px with its last line in 700, paragraphs below, picture right (600 by 680) or none. |

{{fit:b}}

**How it reads.** The highlight is a solid accent shape with paper text in the heavier weight. The total is a 192 px light numeral in the accent. The continuation foot is centred in the bottom strip. The date range is one line in `mute` at the right of the title; the highlighted range is the same line in bold accent. Dark colourway inverts the block (light block, dark numerals) and moves the accent to `#FF7447`; tinted colourway is the sage of `kpi-report.jpg`.

**Never.** No second accent, no rounded shape, no icon, no photograph behind rows, no italic, no drop shadow; the dark block is the only dark mass on a light slide.

## 6. Candidate C: Quiet Tint

**Idea.** One sans in two weights and three type steps (title, row, caption) on a tinted ground; tone and weight make the hierarchy; the only accent is a solid pill under the special row, the highlighted names, the total and the verse reference.

**References.** `products/gamma/quiet-deck.jpg` (two weights and two tones in one title, a hairline across the page); `galleries/canva-amber-p00.jpg`, `canva-amber-p03.jpg`, `canva-amber-p08.jpg` (two sizes of one face on one flat field); `galleries/canva-sched-05.jpg` (a solid pill marks the special row among outlined rows); `galleries/canva-borcelle-p01.jpg` (the reference in a pill); `products/pitch/scale-up.jpg` (pale lavender with a dark navy light grotesk).

**Type.** Figtree (SIL OFL, `ofl/figtree`), static 500 and 700 only, with `tnum` baked in. Chosen over Plus Jakarta Sans, Manrope and Atkinson Hyperlegible Next after a side-by-side render: calm shapes, a clear `1` and `l`, and a `tnum` feature (Atkinson's bold zero is slashed, which reads technical in a time such as `07.00`; DM Sans has no `tnum`). The title is the first word in 500 and `mute`, the rest in 700 and `ink`.

{{type:c}}

**Grid.** Margins 104 px. A 1.5 px full-bleed hairline at y 268 under the title row (the Gamma line crossing the page). Kicker baseline 96 in sentence case, title baseline 232, body 316 to 984. Rows have no separators; the gap between days and the pill do the grouping.

{{grid:c}}

{{palette:c}}

{{contrast:c}}

**The six kinds.**

| Kind | Layout |
|---|---|
| Schedule | Weekday in 700 and the date in `mute`, then the time in 700, then the event. No rules. The special row is a full-width solid pill (radius half the row) with `pill_ink` text and `Khusus` at its right. |
| Birthdays | Three columns, no rules. A highlight is one rounded rectangle (radius 22 px) per contiguous run in a column, names in 700 `pill_ink`, the day in the same colour. |
| Offerings | Label left, amount right, no rules, then a pill bar across the page holding `Jumlah persembahan` at the left and the sum in 700 at 112 px. |
| Account | Bank, the number in 700 up to 152 px, holder, notes; the QR on a white plate with 32 px corners. |
| Verse | Kicker, the reference in a pill, the verse in 500 with indigo bold numbers. |
| Notice | Title in 700 up to 128 px in two lines, paragraphs, picture as a rounded 600 by 640 block or none. |

{{fit:c}}

**How it reads.** The highlight is a solid pill (shape), bold (weight) and the pill colour; on the schedule the word `Khusus` too. The total is the pill bar, the only filled shape on its slide. The continuation foot is `1/2   12 Okt sampai 4 Nov` at the bottom left. The date range is `mute` at the right of the title; the highlighted range is bold in the pill colour. Dark colourway "Senja" flips the pill to lilac with dark text; tinted "Lila" is the lilac ground with a deep indigo pill.

**Never.** No more than two weights, no shadow, no gradient, no icon, no sharp-cornered shape beside a pill, no second hue. Purple is the tinted ground's hue, not a rule: other tints are a token change.

## 7. Candidate D: Civic Board

**Idea.** A departure board: a black header band carries the title in mono caps, every figure is mono and so tabular by nature, words are a condensed grotesque, rows sit on hairlines, and the highlight is an inverted row with the accent inside it.

**References.** `products/googleslides/appstore-ipad-01.png` (a heavy mono caps wordmark on warm beige with a small mono caption); `products/googleslides/appstore-ipad-03.png` (mono rows divided by dark hairlines); `galleries/sg-gather-s24.jpg` (a black header band with caps over cream cells), `sg-gather-s30.jpg` (the special day is an inverted cell), `sg-gather-s31.jpg` (rows inverted to black with cream text); `products/gamma/daktilo-dispatch.jpg` (mono caption lines).

**Type.** IBM Plex Mono (SIL OFL, `ofl/ibmplexmono`) 400, 500, 700 for figures, labels and the title; IBM Plex Sans Condensed 500 and 700 (both already in `assets/warta/fonts`) for words. A mono face for the names would take about 24 px a character and fail the 30-name page, so mono is reserved for what must align (times, days, amounts, the account number) and the condensed face carries names, events, funds, the verse and notes. Digit advance is 600 in the mono and 540 in the condensed face, in every cut (verified).

{{type:d}}

**Grid.** Margins 96 px. The header band is the top 200 px, full bleed, `band`: kicker at baseline 72 (left `Warta jemaat`, right the church), title at baseline 168 in mono caps 80 px, the week or the highlight range at the right in the accent. The body is 252 to 1008. On the birthday page the month is a caps caption at the bottom left.

{{grid:d}}

{{palette:d}}

{{contrast:d}}

**The six kinds.**

| Kind | Layout |
|---|---|
| Schedule | Weekday abbreviation and date in mono caps at the left (first row of a day only), the time in mono bold, the event in condensed 500 at up to 56 px; 1.5 px hairlines. The special row is inverted: a `band` rectangle, text in `band_ink`, the time in the accent, `KHUSUS` in accent mono caps at its right. |
| Birthdays | Three columns, hairlines under names, day in mono bold. A highlight is an inverted rectangle behind the run, names in `band_ink`, days in the accent; it crosses the column break as two rectangles. |
| Offerings | Label in condensed, amount in mono bold right-aligned with a small `Rp`; below, an inverted total bar holding `JUMLAH PERSEMBAHAN` and the sum in mono bold 104 px in the accent. |
| Account | Bank in mono caps, the number in mono bold up to 176 px, a 3 px rule, holder in condensed bold, notes; QR on the right. |
| Verse | The band holds `AYAT PEKAN INI`; the reference in mono caps below it; the verse in condensed 500 up to 68 px with bold mono verse numbers. |
| Notice | The band holds `PENGUMUMAN`; the title in mono bold caps up to 112 px in two lines, a 6 px rule, paragraphs in condensed 52 px; picture right or none. |

{{fit:d}}

**How it reads.** The highlight is an inverted rectangle (shape), bold (weight) and an accent that exists only inside it (colour); it is the strongest of the four from the back of a hall. The total is an inverted bar with a yellow figure. The continuation foot is `1/2   12 OKT SAMPAI 4 NOV` at the bottom right. The date range is in the band in the accent. Dark colourway makes the band light (`#E9E6DA`) and the accent a dark amber inside it; tinted colourway "Biru" is a blue-grey ground.

**Never.** No second accent, no gradient, no rounded corner, no icon, no photograph in the band, no mono for names; the accent never sits on the paper, only inside a band.

## 8. Capacity and cost

What each candidate does with data worse than the artboards' (the same layouts, run on a modified copy of the data):

{{stress}}

Two readings that matter. A one-month page of 30 names holds in three columns for every candidate, including three names of 30 to 32 characters, because the ladder turns middle names into initials. A 44-character name, a 77-character event in an 8-row week and a two-month page of long names each continue on a second slide, which is the behaviour `paginate` already has; the mock draws page 1/2 of the two-month case in the third column of the contact sheet. The widest name cell is about 485 to 495 px for all four, so the longest name that fits three columns at 40 px is about 24 characters after the ladder; that number is the capacity to tell a church.

Fonts to add. The repo embeds each face as `include_bytes!` in `crates/artosia-slides/src/fonts.rs` (`FACES`, an array whose length is typed) and a licence in `assets/warta/licences/`. Latin subsets are measured with `fontTools.subset` on Basic Latin, Latin-1 and common punctuation; whether the subset drops a glyph a church needs (the ten languages) is not tested here.

{{fonts}}

## 9. Build notes: what the renderer has and lacks

| Need | Present today | Gap |
|---|---|---|
| Tokens in the shape of a direction | `assets/warta/directions/dN.json`: `colourways`, `swatch`, `pairs`, `fonts`, `type`, `grid`, `s01` to `s06` | none; `tokens/a.json` to `d.json` follow it. The colourway test asserts at least 4 colourways per family (`mod.rs`, test `every_colourway_of_every_direction_reads_on_every_pair_its_layout_draws`); the brief asks for 3, so the builder adds a fourth or the test floor is lowered by the owner |
| Contrast gate | `pairs` of `[fg, bg, text or large or mark]`, 4.5 and 3.0 | the spec lists every pair each layout draws; all 72 pass in every colourway |
| Tabular figures | `features("'tnum' 1")` in `notice.rs` | a role needs a `features` field in its token; Figtree (C) is the only face that needs it |
| A wash | a named vertical gradient with alpha stops, `grad:name`, as `d4.json` `shade` | A's wash is `paper` at alpha 0 to `wash` at alpha 1, top to bottom over 220 px; its stops name palette keys here, literal hex in `d4.json`, so the builder resolves them per colourway. A corner radial (the Slidesgo look) is not available and not needed |
| Rounded shapes | `R::rx`, `R::pill` | none (C) |
| Two-tone title | two `text` calls and `Sheet::measure` | a title role pair `title_a` and `title` (B, C) |
| Fit and continuation | `fit::ladder`, `fit_cells`, `paginate` | none; the layouts here call the same rules (`kit.py` mirrors `fit.rs`) |
| A light plate behind the QR in a dark colourway | `sl.image(Pic::Qris, ...)` | one `card` rectangle behind it, padding 24 px |

The wireframes are hand-written SVG drawn from `spec.json` `resolved`, so the geometry of every element (x, y, size, tracking, fill key, face) is data. `build/kit.py` repeats the arithmetic of `fit.rs` in Python so the numbers can be measured with the real faces before any Rust exists; it is a measuring aid, not shipped.

## 10. The card to build the chosen default

Mock: `a-schedule.svg`, `a-birthdays.svg`, `a-birthdays-two-months-1of2.svg` and `other-kinds/a-*.svg` for A (the same file names with `b-`, `c-`, `d-` for the others). Proof: the builder's renders of the six kinds at 1920x1080 and the phone and tablet screenshots of the preview that draws the same sheet scaled (`repo: .claude/rules/output.md`: previews are the 1920x1080 view scaled down), each compared against the matching mock, with the table below filled from measured values. The wireframes draw no text field, so keyboard behaviour on iPhone and Android is n/a here: text is edited in the sheets that already ride the keyboard (`defaultstyle1.md`, `noticeedit1.md`, `phoneedit1.md`).

Mock values for the recommended candidate (A), to be matched by the built page:

{{mock:a}}

Acceptance criteria, each checkable by a tester before the builder starts:

1. Every colourway of the new family passes every pair in its `pairs` list in the existing colourway test, and the family has the test's minimum number of colourways.
2. 30 names in one month with 7 highlighted draw as one slide in three columns at 40 px or more; no name is clipped or ends in an ellipsis; a name longer than its cell is shown with its middle names as initials; each highlighted name sits on a shape and in the heavier weight, including when the run crosses the column break.
3. A card of 30 names across two months, with names the ladder cannot fit, becomes slides `1/2` and `2/2` of 15 names each; every row writes its day with its month; the foot names the part and the range it holds; the highlighted range is written on the slide that holds a highlighted name.
4. A week of 8 rows over 4 days with one special row draws as one slide at 50 px or more; the day is written once per group; the special row carries a shape, a heavier weight and the word `Khusus`.
5. An event of 77 characters in that week continues on a second slide rather than shrinking below 40 px.
6. Offerings: 6 funds with a 12-digit amount and a total draw as one slide; the amount column is measured first, no amount wraps or is clipped, digits align right edge to right edge, and a 14-digit amount shrinks the total instead of cutting a digit.
7. The total is at least as large as a row, sits apart from the rows, and is never typed over (it is the sum of the funds).
8. Account: with a QR the number and the code are both on the slide and the code is on a light plate in every colourway; without a QR the number takes the full width and is larger; the account number is tabular.
9. A window in another language draws its own months, weekdays and the fixed words (`SLIDEWORDS1`); no candidate string contains a dash or a middot (the `strings.rs` test).
10. The slide drawn at 1920x1080, at 960x540 and on a 150% zoomed second screen differs only by scale (every size is a fraction of the height).
11. A list is drawn at 0, 1, 40 and 400 rows: 0 rows draws the title and kicker only; 1 row uses the sparse size cap; 40 names become two slides; 400 rows continue without a size under the floor.
12. A card saved with another family keeps drawing in it; an unknown colourway id draws the family's first colourway.
13. A new card made without choosing a style gets the new default through the one function that names the default, whether made by hand, by import or by phone; saved cards keep their stored style.
14. Every list that names styles (gallery tiles with live worst-case data, the style rail, saved looks, the phone editor) shows the new family and its colourways in the same order.
15. The six kinds each have a render test of the worst-case data in the first colourway and in the dark colourway, read by what is drawn (positions and sizes), not by a formula.

## 11. The check script, run after the last edit

`ARTOSIA_FONTS=<dir made by make_fonts.py> python -I build/check.py` prints any number in a wireframe that `spec.json` lacks, any colour outside the palette, and the layout rules:

```
{{check}}
```

`python -I build/check.py --selftest` plants a font size, a colour, a time and a collision in a copy and shows each flagged:

```
{{selftest}}
```
