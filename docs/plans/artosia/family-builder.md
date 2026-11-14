# Rules for a builder of one announcement template family

Read `docs/plans/artosia/builder-rules.md` first; everything there holds. Then:

## Read
- Your family's spec: the section for your candidate in `docs/plans/artosia/templates-default.md` (or `templates-library.md`), its tokens in `docs/plans/artosia/templates-default/tokens/<letter>.json`, `spec.json`, and its wireframes `<letter>-*.svg` and `other-kinds/<letter>-*.svg`. The wireframes are the target: the built slides must match them.
- The model: `docs/templates-is-colourways-and-archetypes.md`. A family is `crates/artosia-slides/assets/warta/directions/<dN>.json` (palette, colourways, swatch, pairs, fonts, type roles, grid, per-kind sections) plus `crates/artosia-slides/src/warta/<dN>.rs` drawing the six kinds; `d1.rs` and `d5.rs` are the models to copy the structure from.
- Words: a family writes no fixed word of its own; every label comes from `Data` (`d.w.*`, `d.week_label`, `d.month`, `d.day_of`, `d.kicker`, `d.church_kicker`). The mock's "sampai" is the en dash `Words::range` already writes.

## Build
1. Your `directions/<dN>.json` from the tokens file, with four colourways (the spec's three plus one more in the same language) that pass the contrast test in `crates/artosia-slides` (it reads `pairs`).
2. Your `<dN>.rs` drawing all six kinds as the wireframes show, using the shared engine (`largest_name_size`, `grown_size`, `name_in`, `fit_cells`, `put_lines`, `verse`, `holds`/`split` for continuation slides).
3. Wiring, each a one-line edit made with the Edit tool only (other builders edit the same files at the same time; never Write an existing file; if an Edit fails because the file changed, read it again and redo only your line): a `NoticeStyle` variant with its code in `crates/artosia-core/src/deck.rs` (`ALL`, `code`, `is_poster`), `FAMILIES` and the `dir_of`/`build`/`foot` arms in `crates/artosia-slides/src/warta/mod.rs`, the `POSTERS` list in `warta/layout.rs`, `style_name` in `crates/artosia/src/ui/templates.rs`, a `text!(STYLE_<NAME>, ...)` line in `crates/artosia/src/strings.rs` after the other `STYLE_` lines with its brief in `docs/i18n/brief.json` (`tools/add-briefs.py`), the sheet name arm in `crates/artosia-output/src/demo.rs`, scenes `notice-style-<code>-<kind>` for schedule, birthdays, money, account, verse, plain, plain-nopic, account-noqris in `crates/artosia-output/src/main.rs`, and your style in the `styles` list of `crates/artosia-output/examples/notice_matrix.rs`.
4. The fonts are already in `crates/artosia-slides/assets/warta/fonts/` with their licences in `assets/warta/licences/`.

## Check
- Render your eight scenes with `cargo run -p artosia-output -- --window audience --scene notice-style-<code>-<kind> --snap <png> --size 1920x1080 --scale 0.5` and hold each against its wireframe; fix every difference you can see.
- Do not run `tools/notice-matrix.sh` (other builders share its folder); the coordinator runs it once all families are in.
- `cargo test -q -p artosia-core -p artosia-slides` pass; `cargo test -q -p artosia` passes except the table test listing your untranslated style name.

## Report
Pass or fail first; files changed; the new string key; the PNG paths of your eight scenes and one contact sheet of them (`python -I tools/contact-sheet.py <png folder> <out folder>`, files named `<code>__<kind>.png`); what still differs from the wireframes and why.
