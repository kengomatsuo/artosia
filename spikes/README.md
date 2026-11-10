# Spikes

Throwaway experiments, each in its own folder with its own `Cargo.toml` (outside the workspace).

- `output-rune-vlc/` — variant A: Rune output window, libVLC frames pushed into a `NativeSurface`
- `output-vlc-native/` — variant B: libVLC owns the output window, libVLC text overlay

Both follow [SPEC.md](SPEC.md). Plan and numbers to beat: [docs/spike-output-video.md](../docs/spike-output-video.md).
