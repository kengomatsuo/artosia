# Architecture

A free worship presentation app (songs, Bible verses, media) to replace EasiSlides. It runs on
Windows, macOS and Linux, and on the budget old Windows PCs small churches own, so RAM is
the first constraint, with video backgrounds playing. It must run smoothly on an Intel Celeron
with integrated graphics even when video is decoded on the CPU (owner, 2026-11-10).

## Decided

| Area | Decision | Why |
|---|---|---|
| Language | Rust, edition 2024, toolchain pinned in `rust-toolchain.toml` | Owner call, 2026-11-10 |
| UI | [Rune](https://github.com/anomalyco/rune), pinned `dev` rev in the root `Cargo.toml` | Owner call, 2026-11-10. GPU-rendered, MIT, Windows/macOS/Linux; every app runs headless, so an agent can `--snap` a PNG and drive the UI with scripts |
| Licence | GPL-3.0-or-later | The app is free; GPL code (FreeShow, OpenLP) and libraries are usable |
| Output video | GStreamer on Windows, macOS and Linux: hardware decode (Direct3D 11/12, VideoToolbox, VA-API) to NV12, pushed into a Rune `NativeSurface`; software fallback | Owner call, 2026-11-10, on measurements in [research.md](research.md): 0.05 cores and 107 MB for 1080p30 against 0.27 cores and 197 MB for libVLC callbacks |
| Processes | Operator app in one process; projector and stage windows in a separate renderer process | Owner call, 2026-11-10, copied from Lerret so an operator crash cannot black out the screens |
| Frame pacing | Port Lerret's cadence logic (`crates/lerret-render/src/video/cadence.rs`, GPL-3.0) | Owner call, 2026-11-10: stops 30 fps judder on 60/120 Hz screens |
| Volunteer UX | Numbered monitor setup, no-second-screen fallback, phone remote paired by QR + PIN, practice mode, after Lerret | Owner call, 2026-11-10 |
| Storage | One SQLite file per library (`library.db`) through `rusqlite` with SQLite built in: songs, plans, announcements, settings | Built 2026-11-10 (owner: "build as you see fit"). Omni Presenter, VideoPsalm-GPUI, Open Worship and Hearken store the same way ([research.md](research.md)) |
| Song text | Plain text with section markers (`[1]`, `[chorus]`, `Verse 1`); a blank line inside a section starts a slide; a second language repeats the markers | EasiSlides' own convention, so its 961-song database imports unchanged. Full account: [operator.md](operator.md) |
| Song import | EasiSlides 5 databases read in place; OpenLyrics folders (`<lines>` is one slide; a verse name repeated in another `lang` is the second language, and a third or fourth is stacked under it slide by slide) | The church's cleaned library is OpenLyrics ([song-library.md](song-library.md)): 1,333 of its items import |
| Bible | MyBible modules (SQLite) read into memory, one translation at a time, about 5 MB each; the King James Version is packed into the program | EasiSlides ships its Bibles as MyBible modules, so a church's existing Bibles carry over |
| Search | An in-memory word index per library and per Bible; a query token matches a word exactly, as a prefix, or within one typo (two for words of eight letters or more); an elided `s'gala` also answers to `segala` | Instant on 1,333 songs and 31,102 verses; Indonesian first ([song-library.md](song-library.md)) |
| Operator and output | `artosia` and `artosia-output` exchange JSON lines on `127.0.0.1:47823`; the output keeps its last state when the operator dies | Built 2026-11-10. Full account: [output.md](output.md) |
| Phone remote | A small HTTP server in the operator (`crates/artosia-remote`), paired by a 4-digit PIN and a QR code, long-polling for state | Built 2026-11-10; OpenLP, Lerret and FreeShow serve remotes the same way. A store app was weighed and left: the page already controls everything, and a wrapper around it is what Apple's guideline 4.2 refuses |
| Search ranking | A cascade of single rules (`artosia-core/src/palette.rs`, `search.rs`), after the owner's house-kit palette | Owner, 2026-11-10: "Have you taken a look at my housekit's palette integration kit?" A blended score cannot say why a row is first |
| One Artosia per library | The operator holds `127.0.0.1:47824`; a second start asks the first to come forward and leaves | Owner-reported 2026-11-10: two were open |
| Projector placement | The output decides from the screens it sees and the operator's `Place` message; it restarts for nothing. One screen: no window | Owner, 2026-11-10: "Why can't it be like powerpoint". The process stays separate so a crash cannot black out the screen; it has no taskbar button |
| Interface languages | English and Indonesian in `strings.rs`, neither translated from the other; briefs and the reviewed Indonesian in `docs/i18n/` | Owner, 2026-11-10 ("all the open stuff simply solve them"). Written blind from briefs, then rendered and fitted |
| Updates | `artosia-update`: a signed manifest and flat files, published as GitHub Releases of `kengomatsuo/artosia` by `tools/publish.sh`, swapped in at the next start, rolled back if the new build does not start | Owner, 2026-11-10 ("so I don't have to send the executable over and over by usb or whatsapp"; GitHub Releases was the owner's pick). The signing key is outside the repository. `crates/artosia-update/README.md` |
| Church files | `artosia-office` reads .pptx and .docx without Office; EMF pictures are redrawn through Windows | Owner's bulletin and birthday list, 2026-11-10. The files hold personal data and stay out of the repository |
| GStreamer in the release | The 65 MB of GStreamer the programs need travels in the release's `gstreamer` folder, worked out by `tools/gstreamer-subset.py`; an installed one is the fallback | The video projector cannot start on a computer without GStreamer, and the church PC has none. Proven with the bundle alone on 2026-11-11. `crates/artosia-update/README.md` |
| Window arrangement | The plan, then Preview and Live side by side as numbered lists, a dock under both (`crates/artosia/src/ui/shell.rs`) | Owner's pick on 2026-11-10 from four trials drawn after OpenLP, ProPresenter, FreeShow and Lerret; the other three and their switch are deleted. [operator.md](operator.md) |
| Streaming | `artosia-stream`, a third program: Windows Graphics Capture of the projector window, one hardware encoder, one small pipeline per destination | Its own process for the reason the projector is; a dropped connection rebuilds only that destination |

## Why not the others

The stack survey and RAM measurements behind this choice are in [research.md](research.md).

## Layout

| Crate | What it is | Links Rune | Links GStreamer |
|---|---|---|---|
| `crates/artosia` | The operator window | yes | no |
| `crates/artosia-output` | The projector and stage windows, a separate process | yes | only with `--features video` |
| `crates/artosia-slides` | What the screens draw, shared with the operator's previews; the messages between the two programs | yes | no |
| `crates/artosia-core` | Songs, Bibles, search, the plan, the library file, importers. No UI | no | no |
| `crates/artosia-remote` | The phone remote's server and its one web page | no | no |
| `crates/artosia-video` | A looping video or a camera into a Rune `NativeSurface` | yes | yes |
| `crates/artosia-warp` | Corner pin and colour over the whole projector picture, as a Rune compositor | yes | no |
| `crates/artosia-stream` | The streaming and recording program | no | yes |
| `crates/artosia-listen` | Microphone capture, level meter, phrase cutting; speech recognition behind the `whisper` feature | no | no |
| `crates/artosia-lyrics` | Finding a song's words on LRCLIB | no | no |
| `crates/artosia-office` | Reading a church's PowerPoint and Word files | no | no |
| `crates/artosia-update` | Self-update from a signed static folder, and the release tool | no | no |

A bare `cargo build` builds everything but `artosia-video` (`default-members` in the root `Cargo.toml`), so a machine without GStreamer still builds and runs Artosia with colour and picture backgrounds.

- `spikes/` — throwaway experiments, each with its own `Cargo.toml`, outside the workspace
- `tools/` — `snap.sh` (render operator scenes), `ram/measure.ps1` and `ram/measure-cpu.ps1`, `grab-window.ps1` (capture one real window), `make-fixture.sh` (test video), `make-bible-asset.py` (pack a public-domain Bible)
- `fixtures/` — generated media and the church's own data, gitignored

## UI research

Screenshots, source files and product cards for every screen (S01–S13) are in `C:\Users\kenne\Shared\inspo\artosia-presenter-ui\`; start with its `README.md`. No screen is designed from memory: each one copies a pattern named there.

## Open (2026-11-10)

| Area | State |
|---|---|
| RAM and CPU with a video background | Misses the target: 0.25 to 0.45 cores and about 300 MB in the output on an Intel Iris Xe laptop. Decode is 0.07 cores; the rest is Rune redrawing the window while a video surface is mounted, which needs a change in Rune. Numbers and the change: [output.md](output.md) |
| Frame pacing | Lerret's cadence logic needs the app to own `present`, which Rune does not allow. Not ported |
| Speech recognition | Written, never compiled. The 57 MB model is in `%APPDATA%\artosia\models` (downloaded 2026-11-10 with the owner's yes); the build still needs CMake and LLVM, which the owner installs |
| Never tried | A real phone on the remote; a real streaming service; a Celeron; unplugging a screen by hand; a bent picture through the projector |

## Dependency notes

- **quick-xml 0.42 (2026-11-10).** `cargo add` wrote 0.42.0 into the manifest before its source was downloaded, and the registry folder still held 0.41.0. The OpenLyrics reader was first written from 0.41's source and did not compile: in 0.42 `BytesText`, `BytesRef`, `BytesCData` and element names are strings, an attribute's `value` is a `Cow<str>`, and an entity in text arrives as its own `Event::GeneralRef`. Run `cargo fetch` and read the folder whose version matches `Cargo.lock` before writing against a new crate.
- **rune-kit** comes from the same pinned Rune rev as `rune`. Its buttons take keyboard focus on a click, which is why the operator's main window draws its own ([operator.md](operator.md)).
