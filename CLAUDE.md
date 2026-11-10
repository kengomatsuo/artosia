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
- Output video: Rune draws it; Rune's `Player` on macOS, libVLC frames into a `NativeSurface` on Windows and Linux. The spike in [docs/spike-output-video.md](docs/spike-output-video.md) confirms the RAM.

## Structure

- `crates/artosia` — the app
- `spikes/` — throwaway experiments with their own `Cargo.toml`; [spikes/SPEC.md](spikes/SPEC.md) is the shared test
- `tools/` — `ram/measure.ps1` (RAM of a process tree), `make-fixture.sh` (test video)
- `fixtures/` — generated media, gitignored
- `docs/` — full accounts behind every rule; [docs/research.md](docs/research.md) is the stack survey
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
cargo build -p artosia
cargo run -p artosia
cargo run -p artosia -- --snap out.png
cargo clippy --workspace
tools/make-fixture.sh            # needs ffmpeg on PATH or FFMPEG=<path>
powershell -File tools/ram/measure.ps1 -Exe <exe> -Wait 40 -Label <name>
```
