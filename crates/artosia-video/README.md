# artosia-video

Loops a video file with GStreamer and pushes every frame, as NV12, into a Rune `NativeSurface`.
Hardware decode first (Direct3D 12/11 on Windows, VideoToolbox on macOS, VA on Linux), software
fallback. Muted by default. Only the renderer process links this crate; the operator binary
(`crates/artosia`) needs no GStreamer.

Pipeline: `playbin3 (flags=video) -> appsink (video/x-raw,format=NV12, max-buffers=1 drop=true sync=true)`.
Looping is a segment seek (`FLUSH|SEGMENT` first, then a non-flushing `SEGMENT` seek on every
`SEGMENT_DONE`), as GStreamer's seeking design doc recommends for seamless loops.

## Build on Windows

GStreamer 1.28 MSVC x86_64 with development files, default install path:

```powershell
$gst = "C:\Program Files\gstreamer\1.0\msvc_x86_64"
$env:PKG_CONFIG      = "$gst\bin\pkg-config.exe"
$env:PKG_CONFIG_PATH = "$gst\lib\pkgconfig"
cargo build --release -p artosia-video --example loop
```

The installer normally sets `GSTREAMER_1_0_ROOT_MSVC_X86_64`; those two variables are enough
without it. `cargo build -p artosia` needs none of this (checked: it builds with no GStreamer).
`cargo build --workspace` fails in `glib-sys` without them.

## Run

The GStreamer DLLs must be on `PATH` (the exe fails to start with a missing-DLL error otherwise):

```powershell
$env:PATH = "$gst\bin;$env:PATH"
target\release\examples\loop.exe [file] [--seconds N] [--software]
```

It prints `pos=<s> loops=<n> decoder=<factory>` once a second. Default file: `fixtures/bg30.mp4`.

## Sound and items

`VideoOptions { muted: false, volume, looping }` plays a video with its sound; `looping: false` ends at the end of the file (`VideoLoop::ended()`, last frame stays) instead of seeking back. `VideoLoop::start_audio(path, options)` plays a sound file alone (playbin3, `flags=audio`), and `set_volume` changes the level while it plays. The output process's `items.rs` uses these for the rundown's video, audio and music items.

## Camera

`CameraLoop::open_address(address, &surface)` opens a camera on the network by its address (`rtsp://`, `srt://`, `http://`), with no device monitor; `Cameras::open` calls it for a name that `is_address`. `Cameras::start()` (about 2 s, about 50 MB for as long as it lives) watches GStreamer's device monitor; `Cameras::open(name, &surface)` starts the camera called `name` (empty: the first) and blocks until a picture arrives, up to 6 s. `CameraLoop::{since_frame, error, frames, name}` say whether it still works; dropping it stops the camera. `VideoError::NoCamera` is a missing one. `ARTOSIA_HIDE_CAMERAS_FILE` names a file of camera names to hide, for tests.

## Decoder choice

`VideoLoop::start` uses the Direct3D 11 decoder first on Windows, then retries with a software decoder when the file will not start (`VideoLoop::fallback()` says why). `VideoOptions::only_decoder` picks a decoder family (`d3d11`, `d3d12`), `discard_frames` decodes without pushing, for measuring. `VideoLoop::since_frame()` and `error()` let the owner notice a stuck or failing loop.

## Using it

- `VideoLoop::start` blocks until the first frame is decoded (10 s limit): call it from a worker thread.
- Call `surface.set_waker(waker)` before `start` (the waker comes from `AppSpec::setup`). Surface clones share the slot.
- Frames arrive on a GStreamer streaming thread and are pushed with `push_bytes`, which is thread-safe. Dropping the `VideoLoop` stops the pipeline; the last frame stays on the surface until you call `surface.clear()`.
- `NativeSurface::push_bytes` takes owned tightly packed planes, so each frame is copied once (3 MB at 1080p), stride removed.
- Errors: a missing file gives `VideoError::NotFound`; a missing `playbin3` gives `MissingElement`; pipeline failures give `Pipeline(message)`. Missing GStreamer DLLs are not a `VideoError`: the process cannot start.
- `prefer_hardware: false` demotes hardware decoder ranks process-wide; `true` restores them.

## Loaded at run time (1080p H.264 file, Windows, hardware path)

DLLs in `bin`: `gstreamer-1.0-0`, `gstbase-1.0-0`, `gstvideo-1.0-0`, `gstaudio-1.0-0`, `gstapp-1.0-0`,
`gstpbutils-1.0-0`, `gsttag-1.0-0`, `gstriff-1.0-0`, `gstrtp-1.0-0`, `gstcodecparsers-1.0-0`,
`gstcodecs-1.0-0`, `gstd3d11-1.0-0`, `gstd3d12-1.0-0`, `gstd3dshader-1.0-0`, `gstdxva-1.0-0`,
`glib-2.0-0`, `gobject-2.0-0`, `gmodule-2.0-0`, `gio-2.0-0`, `intl-8`, `ffi-7`, `pcre2-8-0`, `z-1`, `orc-0.4-0`.

Plugins in `lib\gstreamer-1.0`: `gstcoreelements`, `gsttypefindfunctions`, `gstplayback`,
`gstisomp4`, `gstvideoparsersbad`, `gstvideoconvertscale`, `gstvideofilter`, `gstd3d12` (decoder
picked here: `d3d12h264dec`).

Software fallback also loads `gstlibav` with `avcodec-61`, `avformat-61`, `avutil-59`,
`avfilter-10`, `swscale-8`, `swresample-5` (decoder `avdec_h264`).

Other codecs (HEVC, VP9, AV1, audio) and other OSes load other plugins; list them with the
same method before trimming a bundle.

## Known limits

- Rotation tags and HDR are ignored; colour is mapped to Rune's BT.601/709, video/full range.
- Odd frame sizes are rejected by Rune (`NV12 requires even dimensions`) and reported in `error()`.
- Two loops with different `prefer_hardware` values at once share the same process-wide ranks.
- Audio (`muted: false`, `start_audio`) uses playbin3's default audio sink; played on Windows with the 440 Hz and 220 Hz fixtures (`tools/make-fixture.sh`), not checked on macOS or Linux.
