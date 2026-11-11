# Artosia

A free worship presentation app (songs, Bible verses, media backgrounds) replacing EasiSlides,
for Windows, macOS and Linux. **THE TARGET IS A BUDGET OLD WINDOWS PC, SO RAM WITH A VIDEO
BACKGROUND PLAYING IS THE FIRST CONSTRAINT**: measure it with `tools/ram/measure.ps1` before
claiming anything is light. Other worship apps (FreeShow, OpenLP, ProPresenter) are siblings,
not competitors, and their GPL code is usable here.

**Read [docs/architecture.md](docs/architecture.md) before structural decisions.**
It is authoritative; update it (and this file's headlines) whenever a decision changes.

## Stack (decided — do not relitigate without updating docs/architecture.md)

- Rust, edition 2024; toolchain pinned in `rust-toolchain.toml` to match Rune.
- UI: [Rune](https://github.com/anomalyco/rune), a git dependency pinned to a `dev` rev in the
  root `Cargo.toml`. Its own docs are the reference: `crates/rune/README.md` and `examples/`
  at that rev. The `house-design-language` skill applies to every screen.
- Output video: GStreamer hardware decode to NV12 into a Rune `NativeSurface` on every OS; projector and stage run in a separate renderer process (after Lerret).
- Library: one SQLite file (`rusqlite`, SQLite built in); songs as marker text, Bibles as MyBible modules; EasiSlides and OpenLyrics import.
- The church this is built for sings in Indonesian, so search works for Indonesian first ([docs/song-library.md](docs/song-library.md)). The interface is English and Indonesian, neither translated from the other ([docs/i18n/README.md](docs/i18n/README.md)).

## Structure

- `crates/artosia` — the operator window; `crates/artosia-output` — the projector and stage windows, a separate process
- `crates/artosia-slides` — what the screens draw, shared with previews; `crates/artosia-core` — songs, Bibles, search, plan, importers (no UI)
- `crates/artosia-remote` — the phone remote; `crates/artosia-video` — looping video and cameras; `crates/artosia-warp` — corner pin and colour
- `crates/artosia-stream` — the streaming program (needs GStreamer, like `artosia-video`); `crates/artosia-listen` — microphone and speech; `crates/artosia-lyrics` — lyrics from the web
- `crates/artosia-office` — reads a church's PowerPoint and Word files; `crates/artosia-update` — self-update and the release tool
- `spikes/` — throwaway experiments with their own `Cargo.toml`; [spikes/SPEC.md](spikes/SPEC.md) is the shared test
- `tools/` — `snap.sh`, `ram/measure.ps1`, `ram/measure-cpu.ps1`, `grab-window.ps1`, `make-fixture.sh`, `make-bible-asset.py`
- `site/` — the homepage at artosia.matsuokengo.com, one template in English and Indonesian ([docs/site.md](docs/site.md))
- `fixtures/` — generated media and the church's own library, gitignored
- `docs/` — full accounts behind every rule: [operator.md](docs/operator.md) (the window and its rules), [output.md](docs/output.md) (the output process and its measured cost), [features.md](docs/features.md) (the owner's features and where each stands), [research.md](docs/research.md) (the stack survey)
- `.claude/rules/` — subsystem conventions, path-scoped

## Hard rules

- **CHECK A SCREEN BY RENDERING IT, NEVER BY READING THE CODE (Rune headless, 2026-11-10, owner call).**
  `cargo run -p artosia -- --snap out.png` renders without a window; Rune's README documents
  `--scene`, element-tree dumps and scripted input.
- **A RAM NUMBER COUNTS ONLY IF THE VIDEO WAS PROVEN TO LOOP (2026-10-06, owner-reported).**
  A WPF test was measured after its video had silently stopped. Log the playback position
  and check it wraps twice before measuring. Full account: [spikes/SPEC.md](spikes/SPEC.md).

## Commands

```bash
cargo build                                   # everything but artosia-video; needs no GStreamer
cargo run -p artosia                          # the operator; it starts artosia-output from the same folder
cargo run -p artosia -- --practice            # practice library in memory, projector off
cargo run -p artosia -- --snap out.png [--scene <name>] [--query "<text>"] [--light|--dark] [--indonesian]
tools/snap.sh <folder>                        # every operator scene to PNG
cargo run -p artosia-output -- --window audience --scene lyrics --snap out.png --size 1920x1080 --scale 1
cargo test                                    # core, slides, remote and operator tests
cargo clippy
tools/make-fixture.sh                         # needs ffmpeg on PATH or FFMPEG=<path>
powershell -File tools/ram/measure-cpu.ps1 -Exe <exe> -ArgLine "<args>" -Label <name>
tools/publish.sh <version> "<notes>"          # builds, signs and publishes a GitHub release
tools/site-shots.sh                           # the homepage's pictures, rendered from the real programs
tools/publish-site.sh                         # builds the homepage and replaces the gh-pages branch
```

With GStreamer's development files installed (variables in `crates/artosia-video/README.md`):

```bash
cargo build --release -p artosia -p artosia-output -p artosia-stream --features artosia-output/video
cargo clippy --workspace --features artosia-output/video
cargo test -p artosia-video                   # needs GStreamer's bin folder on PATH to start
```
