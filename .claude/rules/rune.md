---
paths:
  - "Cargo.toml"
  - "crates/**"
  - "spikes/**"
---

# Rune (UI framework) — pinned pre-release dependency (2026-11-10)

- **RUNE MOVES DAILY, SO IT IS PINNED AND BUMPED ON PURPOSE (root `Cargo.toml`, 2026-11-10).** Rune is pre-release and its `dev` branch is never rewritten; bump the `rev` in its own commit, update `rust-toolchain.toml` to match the new rev's, rebuild, and re-render the snapshots. Full account: [docs/architecture.md](../../docs/architecture.md)
- **RUNE'S `Player` PLAYS VIDEO ON macOS ONLY (`crates/rune/src/video/unsupported.rs` at the pinned rev, 2026-11-10).** Elsewhere a clip reports `Failed` and draws grey; Windows and Linux push decoded BGRA or NV12 bytes into a `NativeSurface`. Full account: [docs/spike-output-video.md](../../docs/spike-output-video.md)
- **READ THE PINNED REV'S DOCS, NOT MEMORY (2026-11-10).** The API is in `crates/rune/README.md` and `examples/` of that rev; Cargo's git checkout under `~/.cargo/git/checkouts/rune-*` holds the exact source.
- **RETURN IN A FIELD OF SEVERAL LINES SUBMITS AND ONLY SHIFT RETURN BREAKS THE LINE, AND RETURN ON A FOCUSED BUTTON GOES TO THE DEFAULT BUTTON; EVERY `TextArea` CALLS `enter_breaks_line` AND EVERY FOCUSABLE ELEMENT HANGS `enter_key` (`edit/input.rs`, `runtime/input/focus.rs` at the pinned rev, 2026-11-11).** Full account: [docs/operator.md](../../docs/operator.md)
- **`App::on_modifiers` IS NEVER CALLED IN A SINGLE-WINDOW APP; USE `App::modifier_keys()` AND `Key::Modifier` (`src/window.rs` at the pinned rev, 2026-11-11).** Headless runs do call it, so only a real window shows the gap. Full account: [docs/operator.md](../../docs/operator.md)

