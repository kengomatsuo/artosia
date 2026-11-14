# Artosia handoff (2026-11-14)

**Now:** the UI overhaul (direction B with A's rundown) is half built: the tabs, the colour picker, the calendar and the lyric reader are in Review; the members tab is being built; offerings, templates, the rundown, PowerPoint as made and the song browser dialog remain. Per-card plans: [overhaul-build.md](overhaul-build.md).

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
| UI overhaul | Tabs (c7ff4e7), one colour picker (e63fb2c), calendar with schedule card (30ff5d9) in Review; OVPEOPLE1 in Doing; OVGIVING1, OVTEMPL1, OVRUN1, OVPPTX1 in Ready, each planned in overhaul-build.md | [card: UIOVER1] [rule: .claude/rules/operator.md, last four lines] |
| Song browser | Page reader rebuilt on scraper with a site table and ChordPro chord tests, 10 real pages (35ad9cd); `browse` and `words` give the dialog one call each (d2df40f); the dialog itself is card WEBDLG1 | [card: WEBSONGS1] [card: LYRICPARSE1] [card: WEBDLG1] |

## Last checks

- `cargo test -q -p artosia -p artosia-slides -p artosia-core`: 236, 187, 2 and 19 passed [read: run 2026-11-14 before the mocks]
- `cargo test -q -p artosia-update`: 16 passed [read: run after `artosia-release sign`]
- `cargo test -q -p artosia-lyrics`: 18 passed [read: run after chord handling]
- `bun ~/.claude/skills/design-research/check-directions.ts ~/Shared/inspo/artosia-ui-overhaul`: ok, 7 pages x 3 directions

## Next actions, in order

1. OVPEOPLE1, then OVGIVING1, OVTEMPL1, OVRUN1, OVPPTX1, then WEBDLG1 [card: UIOVER1 children] [card: WEBDLG1]; each card's plan is its section of overhaul-build.md.
2. A card lands as: plan section, build (a builder subagent works well), snaps in English dark and Indonesian light, briefs in `docs/i18n/brief.json` and a blind-writer round for the nine table languages, `ARTOSIA_WRITE_SNAPSHOT=1 cargo test strings_snapshot` when English or Indonesian change, a real-window press with `tools/drive-window.ps1` (a practice window runs at 125%, so headless 1366x768 coordinates map one to one), commit of the card's own paths only (other sessions edit `crates/artosia`), board to Review.
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

## Skills to load

`perfect-build`, `design-research`, `house-design-language`, `blind-translation` (every new string in ten languages), `board`, `prior-art`.
