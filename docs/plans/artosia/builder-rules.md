# Rules for a builder of an Artosia overhaul card

Artosia is a Rust worship presentation program at F:\Repositories\artosia, on Rune (UI framework; its docs and source are the pinned checkout under `~/.cargo/git/checkouts/rune-c9b805e9ed8a3730/70eff81/`, `crates/rune/README.md`, `crates/rune-kit/src/`). The owner picked direction B of the overhaul: the church's data in its own windows, one tab each.

## Read first
- Your card's section in `docs/plans/artosia/overhaul-build.md`. Build exactly that.
- The picked mock page in `crates/artosia/src/ui/mock_b.rs` (or `mock_a.rs` page 7 for the rundown); render it with `cargo run -p artosia -- --snap out.png --scene mock-b-p<N>`.
- `CLAUDE.md`, `.claude/rules/operator.md`, `.claude/rules/rune.md`, `.claude/rules/core.md`.
- Patterns already built for this overhaul: `ui/tabs.rs` (tabs, routed in `ui/mod.rs` `window`), `ui/colour.rs` (the one colour picker; a new colour is a `ColourFor` variant handled in `current` and `apply`), `ui/calendar.rs` once it exists (tables of data, dialogs with pickers, date and time pickers: reuse them, never write a second date picker), `ui/announcements.rs`, `ui/sheets.rs` (`setting`, dialogs, Select drop-downs laid out after the dialog), `ui/widgets.rs` (`btn`, keycaps, `enter_key`).

## Rules
- A value with a shape gets the control of that shape: a day is a toggle, a time comes from a list, a date from a month picker, a colour from the one picker, an amount from a number field; only names and free words are typed.
- Colours on the window go through `theme` (`ink_of`, `tint_of`, `accent`, `warn_ink`); main-window buttons never take keyboard focus unless inside a dialog (`focusable()`); every focusable element hangs `enter_key`.
- Every string a person reads goes in `crates/artosia/src/strings.rs` as `text!(KEY, "English", "English")` (English twice) or a sentence function in that file's style, AND gets a brief in `docs/i18n/brief.json` in the existing shape (screen, moment, role, does, budget, variables, concepts; `does` must not share four words in a row with the English; concepts from `docs/i18n/concepts.md`, add a concept there when none fits). Run `python -I tools/check-brief.py` until it says 0 problems. Write no other language: the table test then lists the missing keys, which is expected.
- Weekday and month names come from `t::WEEKDAYS()` and `t::MONTHS()`; Sunday first.
- The practice library (`--practice` and headless runs) gets made-up data for the card; no real church data, no real song lyrics, and the church's own files (in `fixtures/`, gitignored) never enter the repository.
- Add a scene in `crates/artosia/src/main.rs` `SCENES` for each screen and dialog state the card names.
- Check every screen by rendering it: `cargo run -p artosia -- --snap out.png --scene <name>` plus `--indonesian` and `--light`, at the default 1366x768. Look at each PNG and fix what clips or misaligns.
- Every check runs as `bun ~/.claude/scripts/limit.ts <seconds> <command>` (e.g. `bun ~/.claude/scripts/limit.ts 900 cargo test -q -p artosia`). That wrapper splits quoted arguments on this PC.
- Never run WSL in any form. Never commit. Other sessions edit this tree too: touch only what the card needs, never revert a change you did not make, never `git stash`.
- Comments English and at most a line.

## Report
Lead with pass or fail. Then the files changed, the new string keys, the scenes with the PNG paths you rendered (en dark and id light at least), test counts, and anything left undone with the reason.
