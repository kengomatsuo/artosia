# Artosia handoff (2026-11-14)

**Now:** build card OVGIVING1 (the Offerings tab), then OVTEMPL1, OVRUN1, OVPPTX1 and WEBDLG1. Tabs, colour picker, calendar, members and the lyric reader are in Review. Each card's plan is its section of [overhaul-build.md](overhaul-build.md); a builder brief starts from [builder-rules.md](builder-rules.md).

## Where things stand

| Area | State | Source |
|---|---|---|
| 0.1.31 | Published with Windows and Linux; no Mac files (the Mac was unreachable) | [read: `gh release view v0.1.31` lists Artosia-linux-x86_64.tar.gz, not a draft] |
| Linux one-line install | Live: `curl -fsSL https://artosia.matsuokengo.com/install.sh \| bash`, homepage row with copy button, ten languages | [commit 211d426] [card: LINUXSH1] |
| Release finish | `tools/publish.sh <version> --finish` builds only what a release lacks, from the tag | [commit 211d426] [card: RELFIN1] |
| WSL | Off limits (owner); scripts carry no WSL call; Linux builds paused | [rule: memory no-wsl.md] [card: NOWSL1] |
| Mac | Reached over Tailscale SSH at 100.120.153.35 (`Host mac` in `C:\Users\kenne\.ssh\config`); Tailscale's check mode wants the owner's Tailscale sign-in, then 12 hours free | [read: ssh mac printed "Tailscale SSH requires an additional check"] |
| PowerPoint import | Works (6 of 6 Warta slides); redraws text only; picture-only card now named | [commit c2f10e6] [card: PPTX1] |
| Announcement styles | Hairline, Ledger, Band drawn for five kinds (`crates/artosia-slides/src/notice.rs`), demo scenes only | [commit 88013b9] |
| UI overhaul | In Review: tabs (c7ff4e7), one colour picker (e63fb2c), calendar with schedule card (30ff5d9), members with import and 30 birthdays a slide (b300540). Claimed, not started: OVGIVING1. Ready: OVTEMPL1, OVRUN1, OVPPTX1 | [card: UIOVER1] [rule: .claude/rules/operator.md, last five lines] |
| Song browser | Page reader rebuilt on scraper with a site table and ChordPro chord tests, 10 real pages (35ad9cd); `browse` and `words` give the dialog one call each (d2df40f); the dialog itself is card WEBDLG1 | [card: WEBSONGS1] [card: LYRICPARSE1] [card: WEBDLG1] |

## Last checks

- `bun ~/.claude/scripts/limit.ts 1200 cargo test -q -p artosia -p artosia-core -p artosia-office -p artosia-slides`: 267, 202 (+2), 37 and 19 passed [read: run after the members tables merged, 2026-11-14]
- `bun ~/.claude/scripts/limit.ts 600 cargo test -q -p artosia-lyrics`: 41 passed, 1 ignored [read: browse layer builder's run, d2df40f]
- `python -I tools/check-brief.py`: 919 entries, 0 problems [read: same run]

## Next actions, in order

1. OVGIVING1: launch a builder with `docs/plans/artosia/builder-rules.md` and the card's section "OVGIVING1: Persembahan" of overhaul-build.md; it is claimed as builder-giving and nothing was started [card: OVGIVING1].
2. When it reports: look at its renders, press its controls in a practice window (`target/debug/artosia --practice`, Esc skips the tour, then `tools/drive-window.ps1`), run a writer round (`docs/i18n/README.md`, last section), merge, test, commit the card's paths, `board.ts move OVGIVING1 review --commit <sha>` [rule: docs/i18n/README.md] [rule: .claude/rules/operator.md, real-window line].
3. Then OVTEMPL1, OVRUN1 (RAM measured with `tools/ram/measure.ps1` only after the video is logged looping twice, CLAUDE.md), OVPPTX1, in that order, the same way [card: UIOVER1].
4. WEBDLG1: the song browser dialog on `artosia_lyrics::browse` and `artosia_lyrics::words` (d2df40f), Gema index kept by `GemaIndex::load`/`save`/`is_stale` in the library folder; Import on a button with its key [card: WEBDLG1].
5. When the owner has signed in to Tailscale: `bash tools/publish.sh 0.1.31 --finish` adds the Mac edition [commit 211d426], then `bash tools/publish-site.sh`.

## Owner decisions in force

- Overhaul: direction B with A's rundown; a value with a shape gets the control of that shape, never a text field [rule: .claude/rules/operator.md].
- Songs from the web: Gema SABDA's page is shown parsed in Artosia's own window with an Import button, so the import is the person's act; web search happens in the person's browser [card: WEBSONGS1].
- The UI is always Artosia's: a page is parsed before it reaches the person; Import, Add or a key brings it in [card: LYRICPARSE1].
- WSL is off limits [rule: memory no-wsl.md].

## Waiting on the owner

- Tailscale sign-in for SSH to the Mac (check mode) [read: ssh mac printed the login link; two 20-minute waits expired].
- Microsoft Store developer account [card: STORE1].

## Gotchas

- DuckDuckGo is blocked in Indonesia (since July 2024), Google answers programs with a JavaScript-only page, and Bing serves a challenge, so no search engine can be read by Artosia itself [read: curl 2026-11-14].
- `bun ~/.claude/scripts/limit.ts` splits quoted arguments on this PC: `--example gema -- "kj 4"` arrives as `kj` [read: gema example listed every KJ title].
- A shell heredoc halves backslashes; write scripts with a file tool [rule: memory shell-eats-backslashes.md].
- The kit `Table` needs a `TableState` in `App`; the mocks drew tables by hand [read: builders' reports].
- Ledger style with the Paper look draws its heading white on cream [read: direction A builder's report].
- Other sessions commit to `crates/artosia` at the same time (a guide engine landed today): commit by path only, read `git status` first, and never sweep a peer's file into a card's commit [read: commits 4e5fa6d, 70433d2 by another session].
- Python prints of non-English text fail on this console's cp1252 unless `PYTHONIOENCODING=utf8` is set; a script that writes then prints can stop half way [read: the people_in_days rewrite stopped at zh-Hans].
- A table language with no plural "one" (Indonesian, Vietnamese, Chinese, Japanese, Korean) never reads a "one" form: a word that differs for one item (tomorrow) is a case, never a plural form [commit b300540, `people_in_days`].
- The interface never draws an em dash, an en dash, a middle dot or an exclamation mark, in English too; the table test catches it only in tables [rule: crates/artosia/src/languages.rs test `every_table_answers_every_brief_inside_its_width_with_its_variables`].
- Snapshot tests hold every English and Indonesian text: `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot` rewrites them, and the run after it passes [read: strings_snapshot.rs].
- A practice window opens on the five-stop tour, which owns the keys until Esc; the window runs at 125%, so headless 1366x768 coordinates in points match `drive-window.ps1`'s [read: drive-window runs 2026-11-14].
- Translations are compiled in: render after `cargo build`, not after `cargo test`, or the old binary shows English [read: colour round].

## Skills to load

`perfect-build`, `design-research`, `house-design-language`, `blind-translation` (every new string in ten languages), `board`, `prior-art`, `ui-patterns`, `house-commit`.
