# artosia-stream

Sends the projector picture and the room's sound to a streaming service (RTMP, for YouTube Live and Facebook Live), to a local network (SRT), or to an MP4 file, with several destinations at once. The operator starts it as a child process, the way it starts the projector. Built and measured 2026-11-10 on Windows 11, GStreamer 1.28.8, Intel Iris Xe plus an NVIDIA RTX 4060.

## Build and run

```powershell
$gst = "C:\Program Files\gstreamer\1.0\msvc_x86_64"
$env:PKG_CONFIG = "$gst\bin\pkg-config.exe"; $env:PKG_CONFIG_PATH = "$gst\lib\pkgconfig"
cargo build --release -p artosia-stream --target-dir target/agent-stream
$env:PATH = "$gst\bin;$env:PATH"      # the exe needs GStreamer's DLLs on PATH
artosia-stream --window "Artosia projector" --audio "CABLE Output" --record show.mp4 --rtmp rtmp://a.rtmp.youtube.com/live2/<key>
```

The crate is a workspace member but not a default one, like `artosia-video`. Its `gstreamer` and `gstreamer-app` dependencies turn on the `v1_20` feature (needed for `leaky-type` and `max-buffers` on `appsrc`).

## The pipeline

```text
capture pipeline (restarted when the window goes and comes back)
  d3d11screencapturesrc (WGC, one window, or a monitor)
    ! caps framerate ! d3d11convert add-borders ! NV12 1280x720 (D3D11 memory) ! [d3d11download] ! appsink
        |  newest frame kept in Latest
        v
bridge thread, 30 ticks a second: sends the newest frame, or the previous one again
        v
encode pipeline (lives for the whole run)
  appsrc ! encoder ! h264parse ! appsink  ----\
  wasapi2src | audiotestsrc ! audioconvert ! audioresample ! voaacenc ! aacparse ! appsink ---+--> Hub
        v
one small pipeline per destination, each fed from its own queue
  appsrc ! h264parse ! flvmux ! rtmp2sink          (--rtmp)
  appsrc ! h264parse ! mpegtsmux ! srtsink         (--srt)
  appsrc ! h264parse ! mp4mux ! filesink           (--record)
  (audio: appsrc ! aacparse ! the same muxer)
```

Three pipelines instead of one, because each fails on its own. A lost connection rebuilds only that destination. A closed window rebuilds only the capture. The encoder never restarts, so the other destinations never notice. The cost is two hand-offs of encoded data (a few hundred KB a second) and one of GPU frames, which stay on the GPU.

## Decisions and their sources

**Capture: `d3d11screencapturesrc` on the window.** Its documentation page (fetched 2026-11-10) lists `window-handle`, `window-capture-mode`, `monitor-handle`, `monitor-index`, `show-cursor`, and an output of `video/x-raw(memory:D3D11Memory)` in BGRA. `gst-inspect-1.0` on this PC shows the same, plus `capture-api` with `dxgi` (default) and `wgc`. The page does not say which API takes a window. The program always sets `wgc` for a window and that works here. `dxgi` with a window handle was not tried. A monitor uses the default `dxgi` and `monitor-handle`. Window mode captures the client area, so a title bar never shows. The cursor is off.

The alternative that costs more is a GStreamer tee inside the projector. It would give the encoder the exact frame with no capture at all, but it makes `artosia-output` carry encoders. Capture keeps the projector untouched and works for any window.

**The picture stays on the GPU.** `d3d11convert` scales to the stream size, adds black borders to keep the aspect ratio, and converts BGRA to NV12 on the GPU. `qsvh264enc` and `mfh264enc` accept `video/x-raw(memory:D3D11Memory)` NV12 (`gst-inspect-1.0`, and their documentation pages for qsv). `nvh264enc` and `x264enc` do not accept D3D11 memory (`could not link d3d11convert0 to nvh264enc0` when tried), so they get a `d3d11download` first.

**A tick, not the capture's own timing.** Windows Graphics Capture delivers a frame when the window changes. A still window gives no frames, and a vanished window gives none at all. The bridge thread sends the newest frame 30 times a second, and sends the same one again when nothing is new (`frames_repeated`). The repeated frame shares its GPU texture, so it costs no copy. The bridge skips a tick, and counts it in `frames_dropped`, when the encoder already holds three frames.

**The last frame is held when the window disappears.** Black would be a flash on air. When the capture ends, the stored frame is copied (`copy_deep`) so it no longer depends on the dead pipeline's buffer pool. Without that copy, QSV encoded 10 frames a second and dropped the rest during the outage (measured). With it, the rate stayed at 30.

**Encoders, in this order for `--encoder auto`:** on Windows Quick Sync (`qsvh264enc`), Media Foundation (`mfh264enc`), NVENC (`nvh264enc`), then `x264enc`; on Linux VA-API (`vah264enc`), NVENC, `x264enc`, then `openh264enc`; on a Mac VideoToolbox (`vtenc_h264`), then `x264enc`. Hardware first because the target is an old PC with a weak CPU and an Intel GPU, and Quick Sync is on every Intel chip with graphics. Each is tried on 20 test frames through the same GPU path before the stream starts (about 3 seconds), and the first that works is used. All run constant bitrate, no B-frames, no lookahead, a keyframe every 2 seconds (what YouTube asks for). `x264enc` uses `tune=zerolatency`, a one-second VBV buffer, and `--x264-preset superfast` by default. The settings per encoder are in `src/encoder.rs`. A property one driver lacks is skipped.

An encoder that fails after the stream has started ends the stream with a problem line (exit code 4). The program does not switch to the next encoder mid-stream.

**Audio: `wasapi2src`** (documentation page fetched): `device` takes an endpoint id, `low-latency` is always safe, and `continue-on-error` (since 1.28) gives silence and a warning when a device goes away, instead of an error. All three are set. `loopback` is set for output devices, so "what the PC plays" works as an input. Names come from the `wasapi2` device provider, matched exactly and then by part. AAC comes from `voaacenc`, with `mfaacenc` and `avenc_aac` as fallbacks. `--no-audio`, or a device that is not found, sends a silent AAC track, because services refuse a stream without audio.

**Timestamps.** `GstVideoEncoder` adds 1000 hours to the timestamps it outputs. The audio branch does not. The program takes that shift off the video, so a muxer sees both on one clock. Each destination session then starts on a keyframe and subtracts its own start time, so every new connection starts at zero.

**Destinations.**
- RTMP: `flvmux streamable=true` into `rtmp2sink` (documentation page fetched: it takes `video/x-flv`). A refused or lost connection gives a bus error, the destination is rebuilt after a pause of 1, 2, 4, 8, 16, then 30 seconds. A connection that lasted 20 seconds resets the schedule. The sink counts bytes the server sent back, and the destination is "live" only after the first one.
- SRT: `mpegtsmux` into `srtsink`. The sink retries by itself and raises no error, so the program reads its `stats`: they carry `rtt-ms` only while a receiver is attached. A destination with no receiver stays `starting` or goes to `reconnecting`, and after 15 seconds without an answer a problem line says so. For a listener, pass `srt://:port?mode=listener`.
- Record: `mp4mux` into `filesink`. The file is finished (moov written) when the program stops cleanly. A crash or `taskkill` leaves a file that does not play.

## Flags

```
artosia-stream [--window "Artosia projector" [--window-pid N] | --monitor N]
               [--audio "<device name>" | --no-audio]
               (--rtmp <url-with-key> | --srt <uri> | --record <file.mp4>)...
               [--width 1280] [--height 720] [--fps 30]
               [--video-kbps 2500] [--audio-kbps 128]
               [--encoder auto|qsv|mf|nvenc|x264|va|vt|openh264] [--x264-preset superfast]
               [--stop-on-eof] [--stop-file <path>]
artosia-stream --list
```

- `--window` matches the title exactly, then by part, ignoring case. `--window-pid` limits the match to one process (two copies of the projector can run).
- `--monitor N` is the `index` that `--list` gives.
- No `--audio` and no `--no-audio` means the system's default input.
- Width and height must be even. Destinations may be repeated and mixed.
- A bad argument prints one `problem` line, the usage text to stderr, and exits with code 2.

Exit codes: 0 stopped cleanly, 2 bad arguments or GStreamer missing, 3 no encoder works, 4 the encoder failed, 5 a stop hung and the watchdog ended the process (45 seconds after the stop began).

## Stopping, and what keeps streaming

| Event | Result |
|---|---|
| A line `quit` on stdin | Clean stop: the encoder flushes, every destination closes, files are finished. |
| Ctrl+C or Ctrl+Break | Same clean stop (Ctrl+Break tried: 0.3 s to exit, the file played). |
| `--stop-file <path>` appears | Same clean stop, polled every second. For stopping a copy whose operator died. |
| stdin closes (the operator crashed or exited) | **Keeps streaming.** An operator crash must not end a live stream. |
| stdin closes with `--stop-on-eof` | Clean stop. |

The operator should send `quit` when the user ends the stream, and not rely on closing the pipe. After an operator crash the stream goes on until someone stops it: a restarted operator can create the `--stop-file` it passed at the start, or press Ctrl+C in the console. Killing the process works but leaves a recording unfinished.

## Output

stdout carries one JSON object per line. The key `type` is `status`, `problem`, `info` or `list`. Every line passes through the redactor, so a stream key never reaches stdout. stderr carries nothing from this program; GStreamer's log is off unless `GST_DEBUG` is set, and at debug levels it can contain a URL. The operator should discard stderr. One line from GIO about a missing `giolibproxy.dll` appears on stderr on this PC and is harmless.

A status line, once a second:

```json
{"type":"status","state":"live","seconds_live":10,"frames_encoded":326,"frames_dropped":0,"frames_repeated":0,
 "encoder":"qsvh264enc","bitrate_kbps":2788,"picture":"ok",
 "destinations":[{"name":"rtmp://a.rtmp.youtube.com/live2/***","state":"live"},{"name":"file show.mp4","state":"live"}]}
```

- `state` is `starting` before the first destination connects, `live` while any destination is live, `reconnecting` when one was live and none is now, and `stopped` after the final line.
- `seconds_live` counts seconds in the `live` state. `frames_encoded` counts frames the encoder produced. `frames_dropped` counts frames lost: a tick skipped because the encoder was busy or the PC stalled, and frames a slow destination skipped. `frames_repeated` counts ticks that sent the previous picture. `bitrate_kbps` is the encoded video and audio in the last second, not what the network carried.
- `picture` is `ok` or `waiting for window`. `destinations[].state` uses the same four words.

Problems and information:

```json
{"type":"problem","message":"Could not connect to rtmp://a.rtmp.youtube.com/live2/***. Trying again in 4 seconds.","detail":"..."}
{"type":"problem","message":"The picture source went away. Waiting for it. The last picture stays on the stream.","detail":"The window closed."}
{"type":"info","message":"Encoder: qsvh264enc (hardware)."}
```

`message` is a sentence for a volunteer. `detail` is GStreamer's text for the log, redacted. `--list` prints one `list` object with `monitors` (index, x, y, width, height, primary), `windows` (title, handle, pid, minimized) and `audio_inputs` (name, actual_name, default, loopback), then exits.

## What loads at run time (qsv path, RTMP, default audio)

DLLs in `bin`: `gstreamer-1.0-0`, `gstbase`, `gstvideo`, `gstaudio`, `gstapp`, `gstpbutils`, `gsttag`, `gstcodecparsers`, `gstcodecs`, `gstd3d11`, `gstd3dshader`, `gstdxva`, `glib-2.0-0`, `gobject-2.0-0`, `gmodule-2.0-0`, `gio-2.0-0`, `intl-8`, `ffi-7`, `pcre2-8-0`, `z-1`, `orc-0.4-0`, `vo-aacenc-0`, plus `gstd3d12-1.0-0` for qsv.

Plugins in `lib\gstreamer-1.0`: `gstcoreelements`, `gstd3d11` (capture, convert, download, upload), `gstwasapi2`, `gstaudioconvert`, `gstaudioresample`, `gstaudioparsers`, `gstvoaacenc`, `gstvideoparsersbad`, `gstflv`, `gstrtmp2`, `gstvideotestsrc` (the encoder test), and the encoder: `gstqsv`, or `gstmediafoundation` with `gstwinrt-1.0-0`, or `gstnvcodec` with `gstcuda-1.0-0` and `gstgl-1.0-0`, or `gstx264` with `x264-164`. Other destinations add `gstsrt` (SRT), `gstmpegtsmux`, `gstisomp4` (MP4). `gstaudiotestsrc` loads when the audio is silent. Listed from the loaded modules of a running process, once per encoder, plus `gst-inspect-1.0` for the last group.

## Measured

`tools/ram/measure-cpu.ps1`, 15 s settle then 20 s window, 1280x720 at 30 fps, 2500 kbps, default audio input, RTMP to a local ingest (a 150-line Python RTMP server written for this test), subject the projector looping `fixtures/bg30.mp4` with lyrics. The numbers are the `artosia-stream` process only. The projector costs 0.3 to 0.4 cores and 290 MB more ([docs/output.md](../../docs/output.md)). Each run delivered 30 video frames a second to the ingest with none dropped. CPU is cores of this PC (Intel Iris Xe laptop), one run each.

| Encoder | CPU cores | Working set | Private |
|---|---|---|---|
| `qsvh264enc` (Quick Sync) | 0.21 | 180 MB | 148 MB |
| `mfh264enc` (Media Foundation) | 0.22 | 210 MB | 169 MB |
| `nvh264enc` (NVENC) | 0.19 | 260 MB | 350 MB |
| `x264enc` superfast | 0.75 | 163 MB | 187 MB |
| `x264enc` ultrafast | 0.46 | 154 MB | 180 MB |
| `qsvh264enc`, 640x360 at 1000 kbps | 0.14 | 162 MB | 131 MB |

Startup, from launch to the first status line: about 4 seconds (the encoder test takes most of it). Live about a second later.

**What a Celeron should expect (an estimate, no Celeron was available).** The Quick Sync path is the one to count on. Its work is on the GPU, and the 0.2 cores here are capture, copies, muxing and the network. An Intel Celeron with UHD graphics has Quick Sync, so expect about the same share of one of its cores, plus the projector's own 0.3 to 0.4. `x264enc` is the fallback: superfast took 0.75 of a core on this fast CPU, and a Celeron core is several times slower, so it will not hold 720p30 beside the projector. On a Celeron without Quick Sync use `--x264-preset ultrafast`, `--width 854 --height 480`, or `--fps 25`, and watch `frames_dropped`.

## Proven, and what was not

Proven on this PC (commands and results in the session report):
- A 33 s recording of the real projector window (video and lyrics): H.264 High 1280x720 30 fps, AAC 48 kHz stereo, plays, a frame shows the lyrics over the video. 996 frames, none dropped.
- Window gone and back: the held frame stayed in the recording, the rate stayed at 30, capture resumed on its own.
- SRT to a local listener (`gst-launch srtsrc ... mode=listener`): 11.7 s of H.264 and AAC arrived. The listener was killed and restarted: state went `live`, `reconnecting`, `live`, and the new listener received data. A caller with no listener stayed `starting` and printed the 15 s warning.
- RTMP to a local ingest: the full command sequence (connect, releaseStream, FCPublish, createStream, publish) completed and 26.9 s of H.264 and AAC arrived as FLV that `gst-discoverer-1.0` reads. The ingest dropped the connection at 16 s: the program printed a problem, waited, reconnected, and the second connection received 27 s. A closed port gave `Could not connect ... Trying again in 1, 2, 4 seconds`. The key never appeared in stdout or stderr.
- Audio from a named device (`CABLE Output`), a missing device (silent track and a problem line), `--list`, `--monitor 1`, `quit`, Ctrl+Break, `--stop-file`, `--stop-on-eof`, stdin closing while streaming.

Not proven:
- A real service. No YouTube or Facebook Live was reached, so their handshake quirks, `rtmps://` (TLS), stream-key rules and ingest limits are untested. The local ingest accepts what the client sends and checks nothing.
- `rtmps://` and SRT with a passphrase.
- A Celeron, a second screen as a capture source with the projector full-screen, a stream longer than a few minutes, `dxgi` with a window handle, and a recording after a power cut.
- Other GPUs than this laptop's: `qsvh264enc` runs here on the Intel adapter and the capture on the default adapter, and a PC with two GPUs may choose differently. QSV also dropped frames (175 of 255) in one run while other programs used the GPU; a later run with the same build held 30 fps. Re-measure on the target.
- GStreamer 1.28's `continue-on-error` for an input that is unplugged during the stream.

## Linux and the Mac (2026-11-13)

Everything above was measured on Windows. The same program runs on Linux and on a Mac with each system's own source, sound and encoder; `--probe` ends at once with 0, which is how the operator learns the program can start.

| | Windows | Linux | Mac |
|---|---|---|---|
| Picture | `d3d11screencapturesrc`, converted on the GPU | `ximagesrc` under X11; under Wayland the desktop's portal and `pipewiresrc` | `avfvideosrc capture-screen`, the screen the window is on |
| Sound | `wasapi2src` | `pulsesrc` | `osxaudiosrc` |
| Windows and monitors | Win32 by hand | `x11rb` | Core Graphics |
| Stop from outside | console events | `SIGINT`, `SIGTERM` | `SIGINT`, `SIGTERM` |

What was measured on each, and what was not: [docs/linux.md](../../docs/linux.md) and [docs/macos.md](../../docs/macos.md), under "Streaming".

## Source layout

`args` (command line), `backoff`, `redact`, `status` (pure, with the unit tests), `win` (window and monitor lists and Ctrl+C on each system), `portal` (a Wayland desktop's shared screen), `list` (`--list`, audio choice), `capture`, `encoder`, `encode` (encoder pipeline and the bridge tick), `hub`, `dest`, `report`, `run`.
