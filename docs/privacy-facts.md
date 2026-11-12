# Privacy page facts (2026-11-12)

The page is `site/privacy.json`, public at artosia.matsuokengo.com/privacy/. Each row is a sentence group of it with the code that proves it. Change the code, change the page and its date.

| Page says | Proof |
|---|---|
| No accounts, analytics, telemetry, advertising or crash reporting; nothing sent to the makers | A search of every `crates/**/src` file for analytics, telemetry, crash, sentry, account, advert found only a string label (`artosia/src/strings.rs:1247`, an offering's bank account name), a doc comment about the owner's wish (`ui/sheets.rs:773`) and a key error (`artosia-update/src/manifest.rs:108`). The only network clients are `ureq` agents in `artosia-update/src/fetch.rs:40`, `artosia-lyrics/src/http.rs:26`, `artosia/src/bibles.rs:153`, `backdrops.rs:131`, `speech_model.rs:215`, plus the stream and camera below |
| Update check each start, direct download only | `artosia/src/main.rs:66-67` (`update::check(..., false)` when a library is open); `update.rs:98-100` returns at once when packaged; `update.rs:26-35` no folder in a development or headless build |
| It asks GitHub for two small files, sends no library data | `artosia-update/src/lib.rs:22` (`RELEASES`, github.com/kengomatsuo/artosia/releases/latest/download); `fetch.rs:86-91` (GET manifest and signature, nothing else) |
| Downloads beside the program, swaps in on next start | `update.rs:117-121`, `apply.rs:26-28` (`staging_dir` is `<install>/update`), `update.rs:44-53` |
| "Check for updates" does the same on a press | `palette.rs:1024` (`check(..., true)`) |
| Store edition never updates itself | `update.rs:98-100`, `store.rs` header, `docs/store.md` row 2 |
| Lyrics search: new song, empty words, title of 4+ letters, 0.4 s after typing stops, sent to lrclib.net | `actions.rs:268` (id 0 and empty lyrics), `actions.rs:287` (4 letters, 0.4 s), `actions.rs:1644-1645`, `actions.rs:298`, `artosia-lyrics/src/lrclib.rs:13,58` (`q` parameter) |
| Picking one fetches that song by its number | `actions.rs:338`, `lrclib.rs:62-68` (digits-only id) |
| Bibles window fetches the lists from ebible.org, api.github.com, raw.githubusercontent.com; lists cached for a week | `bibles.rs:330-335` (`open_bibles`), `bibles.rs:229-244`, `bibles.rs:210-225` (`cached_bytes`, `CATALOGUE_FRESH`), `artosia-core/src/ebible.rs:3,15`, `beblia.rs:16,20` |
| Choosing a Beblia translation reads the start of its file; download on a press | `bibles.rs:382-392` (`pick_bible`, `get_head`), `bibles.rs:417-422` |
| Backdrop list is inside the program; previews from thumb.wikimedia.org for tiles on screen; full file on a press from thumb or upload.wikimedia.org | `artosia-core/assets/backdrops.json` (hosts counted: commons.wikimedia.org, creativecommons.org, thumb.wikimedia.org, upload.wikimedia.org), `artosia/src/backdrops.rs:1-5,326-345`, `docs/backdrops.md` rows 1-4 |
| Speech model from huggingface.co on a press, checked by checksum | `speech_model.rs:1-4,24-37` (URL and SHA-256) |
| Live stream goes only to the address and key typed; stream starts on a press | `actions.rs:1012-1044`, `stream.rs:70-77` |
| A network camera address is opened as typed | `artosia-video/src/camera.rs:20-25,135` |
| Own parts talk over 127.0.0.1 | `artosia-output/src/main.rs:478`, `artosia/src/link.rs:169,291`, `main.rs:732` |
| Phone remote off until switched on; may resume at start if left on | `state.rs:172` (`remote_on: false`), `main.rs:69-70`, `actions.rs:1307` |
| Serves one page on port 4780 to the local network; no internet server | `artosia-remote/src/lib.rs:130` (binds all addresses unless `local_only`), `docs/remote.md` "Where it listens"; `rg` found no outbound call in `artosia-remote/src` and no `http` link in `page.html` |
| Four-digit PIN, five wrong tries lock for one minute | `artosia-remote/src/lib.rs:238-247`, `server.rs:20-21,201-207` |
| Paired phone remembered only while Artosia runs | `server.rs:34` (tokens in memory), `server.rs:164-171,215-220` |
| What the phone sees and does | `server.rs:142-161` (state), `server.rs:272-295` (commands); no command reads files or settings |
| Plain web traffic, not encrypted | `server.rs:1,112-132` (HTTP/1.1 over `std::net`) |
| Direct download's firewall rule allows the local subnet only | `artosia/src/firewall.rs:98` (`remoteip=localsubnet`) |
| Microphone only while Listen is on; Listen off at each start | `actions.rs:1059-1090`; the only caller of `set_listening(true)` from the window is `ui/sheets.rs:861`, none at start in `main.rs` |
| Mic or system sound; level, tempo, words | `docs/recordings.md` row `about.json`, `.claude/rules/listen.md` |
| Words read on the computer, model in the library folder, nothing sent | `artosia-listen/README.md:12-15` (whisper.cpp), `speech_model.rs:71-73` (`<library>/models`); `artosia-listen` has no `ureq` |
| Never shown on the projector by itself | `actions.rs:1057-1058` |
| Keep recordings off until switched on; lit mark; what is saved and not; never sent | `state.rs:184` (`keep_recordings: false`), `recordings.rs:157-161`, `docs/recordings.md` "What is kept, and what is not"; `recordings.rs` has no network call |
| Folder: library `recordings` or a picked one; 1 GB limit, oldest first; pack, move, delete | `recordings.rs:138-143`, `docs/recordings.md` "The limit" and the first table |
| Camera is a background, drawn on screen and in the preview | `artosia-video/src/camera.rs:1-3`, `.claude/rules/output.md` (the 256-pixel preview) |
| Stream recording saves a video in the library folder | `actions.rs:1027-1031` (`<library>/recordings/service-....mp4`) |
| Library locations: direct `%APPDATA%\artosia`, Store `Documents\Artosia` | `data.rs:30-67`, `artosia-setup/src/places.rs:58-60`, `docs/store.md` row 3 |
| Library is one database plus folders | `artosia-core/src/store.rs:90-131` (SQLite), `bibles.rs:337` (`bibles`), `backdrops.rs` media folder, `speech_model.rs:72` |
| Stream address and key saved as plain text in settings | `actions.rs:1017-1019`, `state.rs:146`, `store.rs:90-93` (settings table, text values) |
| Learn from use on by default; what it writes; two years; off switch; forget button | `state.rs:192` (`learn: true`), `artosia-core/src/store.rs:94-131`, `learn.rs:40,346-351` (730 days), `learning.rs:1-6,38-41,54-56`, `ui/sheets.rs:296-307` |
| Uninstall removes program, shortcuts, entry; never the library | `artosia-setup/src/places.rs:21`, `install.rs:218-225`, `.claude/rules/setup.md` |
| Store edition: library in Documents, manifest lists microphone and webcam | `docs/store.md` rows 3 and 12 (capabilities), `data.rs:59-61` |

## Not proven, so not on the page

- What Microsoft collects about a Store install, crash reports or install counts it gives the publisher. The page only points to Microsoft's own statement.
- Whether Windows lets a Store-edition phone remote be reached from outside the local subnet: the manifest rule has no remote-address limit (`docs/store.md` "Not tried"). The page says the firewall limit holds in the direct download only.
- Whether the live stream carries sound. The page says "the picture of the projector" only.
- What a camera at a network address sends back to Artosia beyond the stream it opens. The page says only that Artosia opens a connection to it.
- The name of the buttons in the stream settings (the page says "start the live stream" and "recording in the stream settings").
- That the phone page keeps no token in the phone's own browser. Only the computer's side is stated.
- The `README.md`, `docs/features.md`, `docs/bibles.md`, `docs/palette-suggestions.md` (read the first 80 lines), `docs/setup.md`, `artosia-lyrics`, `artosia-stream` docs were not read in full. The page rests on code searches and on the docs cited above.
