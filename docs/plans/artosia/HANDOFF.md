# Artosia handoff (2026-11-14)

**Now:** the UI overhaul (UIOVER1, 13 cards) and the song browser (WEBSONGS1) are built and in Review, waiting for the trailing reviewer. Next owner-facing step: release them (`bash tools/publish.sh <version> "<notes>"`), then the Mac edition once Tailscale is signed in.

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
| UI overhaul | All in Review: tabs c7ff4e7, colour picker e63fb2c, calendar 30ff5d9, members b300540, offerings 9435328, templates 7318e0d, rundown items ebeb24c with playback 1671469 and thumbnails 24a39ee, PowerPoint 8bd3111 with export f059ebe | [card: UIOVER1] [rule: .claude/rules/operator.md, last lines] |
| Song browser | In Review: page reader 35ad9cd (repeat directions dropped d3018b0), browse layer d2df40f, dialog 9c494ab | [card: WEBSONGS1] |
| Guide | In Review: fifteen lessons and the tour, help on every feature, what's new, all texts in ten languages; a lesson keeps its progress and rings scrolled controls | [card: GUIDE1] [card: HELPNEAR1] [commit 825682f] [rule: docs/plans/artosia/guide.md "Where it stands"] |

## Last checks

- `bun ~/.claude/scripts/limit.ts 1200 cargo test -q -p artosia -p artosia-lyrics`: 303 and 42 passed (1 ignored) [read: run before 9c494ab]
- `cargo test -p artosia-core -p artosia-slides -p artosia-office`: 215 (+2), 22 and 51 passed [read: runs before ebeb24c and 8bd3111]
- `python -I tools/check-brief.py`: 5 problems, all `GUIDE_SCREENS_OPEN*` from the guide session's commit 7859a8f [read: last run]
- `python -I tools/check-lessons.py`: 16 of 16 lessons walk to their end; `python -I tools/check-help.py`: 15 of 15 features show their help; `cargo test -p artosia`: 312 passed [read: runs on a copy of 825682f]
- RAM, `artosia-output` release: 293 MB background alone, 355 MB with a looping 1080p item with sound, 286 MB with the background torn down under it [commit 1671469]

## Next actions, in order

1. Release the overhaul: `bash tools/publish.sh <next version> "<notes>"` [rule: CLAUDE.md Commands]; it was not released in this session.
2. Unverified, to check on a real run: a scene whose background is a video, a media item whose file fails, a Stop item's end in the operator thumbnail [read: OVRUN1 builder report]; the LibreOffice export route (no LibreOffice on this PC) [read: OVPPTX1 office report]; Letras pages live (it refuses programs) [commit 35ad9cd].
3. When the owner has signed in to Tailscale: `bash tools/publish.sh 0.1.31 --finish` adds the Mac edition [commit 211d426], then `bash tools/publish-site.sh`.

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
- A session whose work sits beside another session's unfinished edits builds in a copy of HEAD (`git archive HEAD | tar -x -C <copy>`, `CARGO_TARGET_DIR=target/headcheck`) and commits it with `python -I tools/commit-scratch.py <copy> <the rev it was made from> <message file>`, which merges three ways onto the current HEAD, merges translation tables by key, and pushes. A plain copy of the copy's files over HEAD once reverted two commits another session had landed meanwhile [commit ef940ac, rebuilt on c463a33].
- The other session commits whole files from the shared tree: apply every edit to the shared tree too, or its next commit takes the edit back out [read: 8350452 carried this session's unfinished main.rs and actions.rs hunks].
- Python prints of non-English text fail on this console's cp1252 unless `PYTHONIOENCODING=utf8` is set; a script that writes then prints can stop half way [read: the people_in_days rewrite stopped at zh-Hans].
- A table language with no plural "one" (Indonesian, Vietnamese, Chinese, Japanese, Korean) never reads a "one" form: a word that differs for one item (tomorrow) is a case, never a plural form [commit b300540, `people_in_days`].
- The interface never draws an em dash, an en dash, a middle dot or an exclamation mark, in English too; the table test catches it only in tables [rule: crates/artosia/src/languages.rs test `every_table_answers_every_brief_inside_its_width_with_its_variables`].
- Snapshot tests hold every English and Indonesian text: `ARTOSIA_WRITE_SNAPSHOT=1 cargo test -p artosia strings_snapshot` rewrites them, and the run after it passes [read: strings_snapshot.rs].
- A practice window opens on the five-stop tour, which owns the keys until Esc; the window runs at 125%, so headless 1366x768 coordinates in points match `drive-window.ps1`'s [read: drive-window runs 2026-11-14].
- Translations are compiled in: render after `cargo build`, not after `cargo test`, or the old binary shows English [read: colour round].

## Skills to load

`perfect-build`, `design-research`, `house-design-language`, `blind-translation` (every new string in ten languages), `board`, `prior-art`, `ui-patterns`, `house-commit`.
