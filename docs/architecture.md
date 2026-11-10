# Architecture

A free worship presentation app (songs, Bible verses, media) to replace EasiSlides. It runs on
Windows, macOS and Linux, and on the budget old Windows PCs small churches own, so RAM is
the first constraint, with video backgrounds playing.

## Decided

| Area | Decision | Why |
|---|---|---|
| Language | Rust, edition 2024, toolchain pinned in `rust-toolchain.toml` | Owner call, 2026-11-10 |
| UI | [Rune](https://github.com/anomalyco/rune), pinned `dev` rev in the root `Cargo.toml` | Owner call, 2026-11-10. GPU-rendered, MIT, Windows/macOS/Linux; every app runs headless, so an agent can `--snap` a PNG and drive the UI with scripts |
| Licence | GPL-3.0-or-later | The app is free; GPL code (FreeShow, OpenLP) and libraries are usable |
| Output video | **Open: decided by the spike** in [spike-output-video.md](spike-output-video.md) | Rune's `Player` plays video on macOS only |

## Why not the others

The stack survey and RAM measurements behind this choice are in [research.md](research.md).

## Layout

- `crates/lumen` — the app (operator window placeholder today)
- `spikes/` — throwaway experiments, each with its own `Cargo.toml`, outside the workspace
- `tools/ram/measure.ps1` — RAM of a process tree; `tools/make-fixture.sh` — test video
- `fixtures/` — generated media, gitignored
