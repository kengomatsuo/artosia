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
| Storage, Bible and song formats | Not chosen yet | Research before picking |

## Why not the others

The stack survey and RAM measurements behind this choice are in [research.md](research.md).

## Layout

- `crates/artosia` — the app (operator window placeholder today)
- `spikes/` — throwaway experiments, each with its own `Cargo.toml`, outside the workspace
- `tools/ram/measure.ps1` — RAM of a process tree; `tools/make-fixture.sh` — test video
- `fixtures/` — generated media, gitignored

## UI research

Screenshots, source files and product cards for every screen (S01–S13) are in `C:\Users\kenne\Shared\inspo\artosia-presenter-ui\`; start with its `README.md`. No screen is designed from memory: each one copies a pattern named there.

## Proposed (2026-11-10, waiting for the owner's yes)

| Area | Proposal | Evidence |
|---|---|---|
| Storage | SQLite via `rusqlite` with the `bundled` feature | Used by Omni Presenter, VideoPsalm-GPUI, Open Worship, Hearken, ChyguiSlide (their `Cargo.toml`, [research.md](research.md)) |
| Song exchange | OpenLyrics XML, plus importers for EasiSlides, OpenLP, ProPresenter files | OpenLyrics 0.8 spec; FreeShow and OpenLP import lists (research `languages/freeshow.md`, `languages/openlp.md`) |
| Bible text | Public-domain translations offline; MyBible and Zefania import | EasiSlides' v5 feature page lists both formats; API.Bible's free tier allows 3 licensed Bibles for non-commercial use ([research.md](research.md)) |
| Phone remote transport | HTTP + WebSocket server in the app | OpenLP, Lerret and FreeShow serve remotes this way (research `languages/`) |
