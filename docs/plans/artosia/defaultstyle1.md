# DEFAULTSTYLE1: a plain, tidy default announcement style

Owner ask (2026-11-15): beside the styled directions, a safe, clean, tidy default template that anyone with no design preference can fall back on.

## What the references show

- Keynote's theme chooser opens on a "Basic" row before every styled theme: Basic White, Basic Black, Classic White, White. Each is a plain ground, one sans, a bold title top left and a small subtitle, nothing else (`~/Shared/inspo/artosia-notice-designs/slides/17-keynote-help-theme-chooser.png`).
- Google Slides starts a blank deck in its Simple Light theme (secondary sources: slidesai.io and aippt.com guides on getting the normal theme back, read 2026-11-15).
- Church Visuals' free lyric templates are plain text on a white ground in Arial or Inter, "for maximum readability" (churchvisuals.com/product/arial-lyric-template, /inter-lyric-template, read 2026-11-15).
- ProPresenter documents no built-in announcement theme; its shipped themes are all styled (`slides/12-propresenter-theme-picker.png`).

So the default is a pair, as Keynote ships it: a light one and a dark one, plain ground, one sans family, no shapes.

## The design

Two `NoticeStyle` variants, `Simple` (light) and `SimpleDark`, drawn by the warta framework as `d5` so they get its fit, continuation slides, one-month pages and on-slide editing boxes for free.

| | Simple (light) | Simple dark |
|---|---|---|
| Ground | white `#FFFFFF` | near black `#121417` |
| Ink | `#1F2328` | `#F1F3F4` |
| Muted | `#5F6368` | `#9AA0A6` |
| Rule | `#DADCE0` | `#3C4043` |
| Accent | blue `#1A73E8` | light blue `#8AB4F8` |
| Highlight ground | `#E8F0FE` | accent at 18% |

GUESS: the colour values are chosen here, not measured from a product; check contrast (WCAG AA 4.5:1 for muted text) before keeping them.

- Type: Instrument Sans only (bundled: 500, 600, 700). Title 700 at about 88 px, top left inside 96 px margins; a small muted kicker above it (the church's name, else nothing); body rows 500, highlighted rows 600.
- Rows sit on hairline rules, never in boxes. A highlighted row gets the highlight ground, a 6 px accent bar at its left and the heavier weight. The highlight's range is written in accent text at the top right, only when the page has a highlighted row (`Data::highlighted`, `week_label`).
- No frames, stickers, shapes, gradients or ornaments. A picture appears only where the card has one (a plain notice's image, an account's QRIS).
- Every kind: schedule (day, time, event; a day written once per group), birthdays (day and name, 1 to 3 columns by count, `Data::day_of` across months), money (fund and amount, a total row under a rule), account (bank, the number large and tabular, holder, QRIS at the right when given), verse (reference small and muted, verse large with raised numbers), plain (title and paragraphs, picture at the right when given).

## Where it shows

- The Templates gallery's curated tiles start with Simple and Simple dark; the style rail lists them first.
- A card made without choosing a style gets Simple. Existing cards and saved looks keep their style.

## Done when

A new card is drawn in Simple without anyone choosing it, the gallery leads with the pair, and all six kinds render in both at projector size with the worst-case data, with nothing clipped.

## Verify

`tools/notice-matrix.sh id` and `en` include both styles; output scenes `notice-style-simple-<kind>` and `notice-style-simpledark-<kind>` render; Templates tab snapped in English dark and Indonesian light.
