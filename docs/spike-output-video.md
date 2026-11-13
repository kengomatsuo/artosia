# Spike: how the output window shows video

**Decided 2026-11-10 (owner): GStreamer to NV12 into a Rune `NativeSurface` on every OS.** The libVLC variants below are kept as the record of what was measured and rejected (0.27 cores against GStreamer's 0.05). The spike now builds that one path and measures it with the window drawing: RAM, CPU, dropped frames, on this PC's Intel Iris Xe and on a Celeron-class machine.


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

## Measured: libVLC frame callbacks decode on the CPU (2026-11-10)

libVLC 3.0.24 (NuGet `VideoLAN.LibVLC.Windows`), `libvlc_video_set_callbacks` + `libvlc_video_set_format("RV32", 1920, 1080, 7680)`, the 1080p30 fixture, 20 s per run, from a Python ctypes harness:

| `--avcodec-hw` | Frames in 20 s | Working set | Private |
|---|---|---|---|
| none | 592 (29.6 fps) | 201 MB | 222 MB |
| any | 593 | 202 MB | 223 MB |
| d3d11va | 585 | 202 MB | 223 MB |
| dxva2 | 596 | 201 MB | 221 MB |

Every run logged `trying format dxva2_vld`, `trying format d3d11va_vld`, then `no hw decoder modules matched`: with callbacks the decode is software. The numbers include the Python interpreter. An NV12 run with `libvlc_video_set_format` hung on stop ("waiting decoder fifos to empty"); NV12 needs `libvlc_video_set_format_callbacks`, which declares both planes' pitches.

This fits the plan: the app must stay smooth with CPU decoding on an Intel Celeron with integrated graphics. Next: measure CPU % here and on a Celeron-class machine, and try 720p backgrounds.

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
