# Spike: how the output window shows video

The first task in this repo. Build both variants against [spikes/SPEC.md](../spikes/SPEC.md),
measure both, then record the winner in [architecture.md](architecture.md).

## What Rune gives us (read from the pinned rev, 2026-11-10)

- `Player` (alias `Video`) with `.video(&clip, SurfaceFit::Fill)`: AVPlayer on macOS. On every
  other platform a clip reports `Failed("video playback isn't supported on this platform yet")`
  and draws a grey placeholder (`crates/rune/src/video/unsupported.rs`).
- `NativeSurface`: a latest-wins frame slot bound with `el().native_surface(&surface, fit)`.
  Off macOS, `push_bytes` takes packed BGRA or two packed NV12 planes
  (`examples/video-tiles/README.md`). NV12 is 3 MB per 1080p frame; BGRA is 8 MB.

## Variant A: Rune draws both windows, libVLC decodes

- macOS: Rune's own `Player`.
- Windows and Linux: libVLC decodes with hardware acceleration and hands frames over through
  its video callbacks; the spike pushes them into a `NativeSurface` as NV12 (or BGRA), and
  the lyrics are ordinary Rune text over it. Preview and output then draw the same way.
- Open questions: the cost of the per-frame CPU copy, and whether libVLC can deliver NV12
  straight from its hardware decoder. Read libVLC's `libvlc_video_set_format_callbacks` docs
  before writing it.

## Variant B: Rune operator window, libVLC owns the output window

- libVLC renders into its own native output window (the lowest-RAM path in the survey).
- Lyrics would have to use libVLC's own overlay (marquee/subtitles), so the projector gets
  no Rune styling or springs. Rune text cannot draw over a native child window.

## Bindings

- `vlc-rs` 0.3.0 on crates.io was last released 2018 and its repo last pushed 2021-12, so
  expect to write a small FFI over `libvlc.h` (`libvlc_new`, media, media player,
  `libvlc_video_set_callbacks`, `libvlc_video_set_format`).
- libVLC 3.0.x DLLs ship in the `VideoLAN.LibVLC.Windows` NuGet package (the survey used
  3.0.24; check for the current version). libVLC is LGPL-2.1+.
- `libmpv2` (crates.io 7.0.1, 2026-11-07, kohsine/libmpv-rs) is the alternative decoder; mpv's
  render API is OpenGL or a slow software path, so frames would also go through a copy.

## Numbers to beat (same loop test, Windows 11, 2026-10-06)

| Stack | Working set | Private |
|---|---|---|
| WPF MediaElement (Windows only) | 182 MB | 247 MB |
| Avalonia + libVLC, D3D11VA hardware decode | 250 MB | 294 MB |
| EasiSlides 5.2 itself (older test file; may not have been looping) | 355 MB | 306 MB |
| Slint + ffmpeg pipe, software decode, RGBA | 394 MB | 472 MB |
| egui / Iced + ffmpeg pipe, RGBA | ~500 MB | 630–860 MB |
