# The output process

`artosia-output` draws the projector window and the stage window. It is a separate program so that an operator crash leaves the room looking at the last slide. Built and measured 2026-11-10.

## How the two programs talk

| Step | What happens |
|---|---|
| Start | The operator (`crates/artosia/src/link.rs`) connects to `127.0.0.1:47823`. If nothing answers it starts `artosia-output` from its own folder, with GStreamer's `bin` put first on the child's `PATH`. |
| Hello | The output sends its screens (name, size, scale), which screen each window covers, the cameras and whether it can play video. It is sent the moment the socket connects, built from the system's screen list before any window exists, and again whenever a screen or camera comes or goes. |
| State | The operator sends the whole screen state as one line of JSON whenever it changes (`OutputState` in `crates/artosia-slides/src/frame.rs`). A burst of key presses sends one line: only the newest state is written. |
| Place | `ToOutput::Place { audience, stage, stage_window, here, off }` moves, shows or hides the projector and the stage view without restarting the process. |
| Hidden | `FromOutput::ProjectorHidden` says the person at the single-screen projector window pressed Escape or clicked it. |
| Operator dies | The output keeps its windows and its last state, and waits for the next connection. A restarted operator finds it by the port. |
| Operator closes | It sends `Quit` and the output exits. |

A second copy of `artosia-output` exits at once: the port is taken, so the first copy owns the screens.

## Screens, like a slide show

The output starts with a window-less anchor (`App::windowless`), so it can sit with no window and no GPU at all (141 MB with nothing showing). Windows open and close on demand with `rune::windows::open_window`.

- With a second screen the projector goes full-screen on it at once. With one screen there is no projector window; the operator's Live preview is the picture. `Place { here: true }` shows a window on the single screen, and Escape or a click on it hides it and sends `ProjectorHidden`.
- Neither window has a taskbar button or an Alt-Tab entry. Rune's `WindowOptions` at the pinned rev has no tool-window or skip-taskbar option, so `displays::hide_from_taskbar` sets `WS_EX_TOOLWINDOW` through `WindowHandle::with_raw_handle` while the window is still hidden (a window shows only after it is fitted and off the taskbar). Windows only. A Rune option would replace it.
- Screens are found again with `WM_DISPLAYCHANGE` on a hidden top-level window of `displays.rs`, with a two-second poll behind it. Plugging in a screen moves the projector onto it; pulling out the projector's screen closes its window. A screen pinned by the operator (`Place`) or by `--audience <n>` is used while it exists, remembered by device name, not by number. macOS would need `CGDisplayRegisterReconfigurationCallback`, Linux the XRandR change events (X11) or `wl_output` globals (Wayland); winit 0.30 reports none of them and Rune lists monitors only at start, so other systems keep the start list.
- A window never changes between bordered and borderless, so a change opens the right window first and closes the old one when the new one has drawn. Measured with a screen made to appear and vanish (`ARTOSIA_HIDE_DISPLAYS_FILE`, below): projector onto screen 2 with video playing, 570 ms from the change to the old window gone, 860 ms with the first video frame back; with no video about 450 ms. Screen unplugged: window closed 30 ms after it is noticed. The picture on the other screen never goes dark during a move.
- Windows puts a new window on its screen at that screen's scale, which first made the projector 2304x1296 on a 1920x1080 screen at 150% (text off the right edge). Windows start hidden, are put exactly on their screen (`set_position`, `resize_instant`) and shown after that. The view draws the background only until Rune has laid the window out, and text is always laid out for the window's own size and scale (`Cx::layout_size`; a guessed 1920x1080 on the first frame was what showed twice too large). `ARTOSIA_TRACE=1` prints `size check: window is 1280x720 points at scale 1.50, the view is laid out for 1280x720 at 1.5: same`.
- With video playing, opening a window holds the video back for about half a second. Rune configures a window's surface by waiting for an idle GPU, and wgpu panics (`GpuWaitTimeout`) if a video frame completes meanwhile; that panic was seen. Any other resize or scale change while video plays can hit it, so a panic on the main thread starts a fresh copy that takes over the port (`--takeover`, three restarts at most, tested with `--demo-panic <seconds>`: the new copy was bound 0.3 s later and its window shown 0.6 s after that; the operator reconnects and sends its state again).
- The projector draws through `artosia-warp`'s compositor (corner pin and colour); video and text draw through it unchanged.

Test hooks: `ARTOSIA_HIDE_DISPLAYS_FILE` and `ARTOSIA_HIDE_CAMERAS_FILE` name a file whose lines are screens or cameras to leave out of the lists; `ARTOSIA_TRACE=1` prints every decision to stderr; `--here`, `--no-projector`, `--audience-name <device>`, `--stage-name <device>`.

## What the screens draw

`crates/artosia-slides` holds the two views, used by the output and by every preview in the operator window.

- `audience()` sizes everything as a fraction of the height, 1080 as the reference. `thumbnail()` draws it on a 1920x1080 canvas and scales it down, so a preview wraps its lines where the projector does.
- `stage()` follows Lerret's band layout: the live lines large, the next lines under them, the clock and "Slide 3 of 8" at the side, the arrangement along the bottom, a yellow strip on top for a message to the stage.
- Text shadow is a blurred black copy of the text, drawn on the output only. It costs a blur pass per frame while video plays (below), so a look turns it on only for pictures and video.
- The text style is three fields of `Look` (`look.rs`): `ink` (colour), `font` (family name, empty is Rune's own font) and `outline` (off, thin, medium, thick) with `outline_ink`. A look saved before them loads with the defaults (`#[serde(default)]`), and a preset keeps the person's font and outline (`Look::with_preset`). The stage view draws in its own white style and ignores all three.

## One look for each plan (2026-11-11)

The output still receives one `Look` in every `OutputState`; the operator chooses which. It sends the open plan's own look when the plan has one, else the usual look, so the camera behind the words in the morning service and the plain background in Sunday school are the same code path, and a switch of plan that happens while a slide is live changes the background (video or camera included, through the usual `video::want`) on the next state without touching the slide. Full account: [operator.md](operator.md), "A plan's own look".

## Text colour, font and outline (2026-11-11)

Rune at the pinned rev has no text stroke or outline. Its `TextStyle` carries size, weight, family, line height, tracking, OpenType settings and italic only (`crates/rune/src/tok.rs`); the one text effect is `El::shadow` and `blur` on a whole element.

- **The outline is copies of the text under the text.** `dressed()` in `audience.rs` draws the line as the same text in the outline colour at offsets round a circle, under the fill: 8 copies for thin, 16 for medium and thick. The reach is a fraction of the screen's height (`Outline::share`: 0.28%, 0.52%, 0.85%, so 3, 5.6 and 9 pixels at 1080), like every other size. Because the copies are real text, they wrap exactly as the fill does and need no second layout.
- **What it costs.** Each line of a slide becomes 9 or 17 text elements instead of 1, about 1,560 glyphs for a four-line slide against about 100. Glyph counts come from Rune's frame report (`--snap` prints `glyphs 1564`). Neither the draw cost nor the memory of an outlined slide was measured on the old PC, and the video rows below were not re-measured with an outline on; the copies add draw work to every redraw while video plays, and that path misses the target already. The shadow stays off by default for the same reason. A preview in the operator draws 8 copies whatever the width, and a thumbnail under 300 points wide (`Detail::Tile`) draws none.
- **Font.** `Family::Named(&'static str)` takes an installed family by name and loads its files the first time text asks (`text/system_fonts.rs`, `installed_family_files`, through DirectWrite on Windows, CoreText, fontconfig). A name the computer lacks falls back to the sans slot, so a look saved on another PC still shows its words in Rune's font. `fonts::family` gives Rune its `&'static str` by keeping each name once. Rune has no call that lists installed families, so `fonts::installed()` asks fontique (already linked by Rune) for them, once and only when the font picker opens, since reading the system's fonts takes a moment. The projector process loads the same family by the same name from the look in the state JSON.
- A font that is installed on the operator's PC and not on the projector's cannot happen: they are one computer. A look carried to another PC draws in Rune's font there.
- Checked by rendering: `--scene styled` (yellow Georgia, thick black outline), `outlined` (white, medium black outline over a bright background) and `missing-font` (an unknown family, thin outline; the default font shows).

## Video and camera

`crates/artosia-video` loops a file through GStreamer `playbin3` into an NV12 `appsink` and pushes each frame into a Rune `NativeSurface`. A camera goes through the same surface: `mfvideosrc` (any device GStreamer's device monitor lists) to `videoconvert` to the `appsink`. Only `artosia-output` built with `--features video` links it; the operator never needs GStreamer.

- On Windows the Direct3D 11 decoder comes first (`d3d11h264dec`), then software. A file that fails to start with hardware is retried with a software decoder before giving up; a hardware decoder that fails in the middle of a file restarts it in software; the operator is told in plain words.
- A missing or undecodable file, a camera that is unplugged, taken or stuck for two seconds: the surface is cleared so the look's black shows under the text, the operator gets a `MediaError` holding a kind and what it is about (an empty one means "over"), which the operator words in the window's language, and the output tries again, a camera every two seconds and a file every five to ten. A stall shorter than two seconds keeps the last frame. A frozen picture would pass for live, black with the words plainly does not.
- **A camera on the network (2026-11-11, the owner's list: "any video input, USB or wireless, from anywhere").** A camera whose name is an address (`rtsp://`, `srt://`, `http://`) is opened by `playbin3`, as a file is, and only its picture is taken (`camera.rs` `CameraLoop::open_address`, `is_address`). It gets twelve seconds for its first picture, where a plugged-in camera gets six. A phone is such a camera through an app that sends RTSP or SRT; the phone's own browser cannot be one, because a browser gives a page the camera only over HTTPS. The release ships the plugins for it (`tools/gstreamer-subset.py`: MPEG-TS, RTSP, RTP, UDP, TCP, HTTP and multipart JPEG), 4.9 MB more. Checked: a test picture sent over SRT from this computer to itself is opened by its address in `tests/smoke.rs`, and an address nothing answers is an error after twelve seconds; in the real programs, with only the shipped plugins, the address was typed with real keys in Design, Use lit, and the projector window on the second screen showed the moving picture; with the sender stopped the projector went black and the status bar read "The network camera does not answer. Trying again.", and with the sender back the picture returned and the line left with no key pressed. Not tried: RTSP, HTTP, and a real phone.
- **A trouble is a kind, never a sentence (2026-11-11).** The projector program wrote its troubles as English sentences, and the status bar showed them as they were in an Indonesian window. It now sends `camera-none`, `camera-missing`, `camera-busy`, `camera-stopped`, `camera-far`, `video-missing`, `video-failed`, `video-stopped`, `video-software` or `video-none`, with a tab and the name it is about, and `why.rs` `media_problem` words each in the window's language. The line is also shown in Design under the camera rows, because the status bar is behind that dialog.
- Cameras for the operator are listed from the system (`cameras.rs`, no GStreamer) every two seconds. GStreamer's device monitor costs about 50 MB for as long as it runs (141 MB without, 190 MB with), so it starts only when a camera is first used, about three seconds before the first picture and then 0.5 s.

## Measured (release builds, Windows 11, Intel Iris Xe on a 144 Hz primary, 2026-11-10)

CPU cores over 15 s after a 36 s settle (the 15 s file had wrapped twice: `loops=3` in the trace every time), projector full-screen on the second screen unless noted, no shadow unless noted. Other programs were building on this PC, and the same variant gave 0.23 and 0.42 cores in two runs, so read these as 0.1 wide.

| What ran | CPU cores | Working set | Private | Redraws/s |
|---|---|---|---|---|
| Output alone, window, no video | 0.00 | 142 MB | 130 MB | 0 |
| 1080p30, d3d12 decoder (the earlier default) | 0.40 to 0.47 | 358 MB | 345 MB | 38 to 45 |
| 1080p30, d3d11 decoder (default now) | 0.23 to 0.42 | 300 to 309 MB | 264 to 275 MB | 21 to 45 |
| same, text with shadow | 0.43 to 0.57 | 337 to 382 MB | 302 to 369 MB | 40 to 99 |
| 1080p30, software decoder | 0.83 | 406 MB | 399 MB | 52 |
| 1080p30, decode and download only, nothing pushed | 0.07 | 318 MB | 313 MB | 1 |
| 1080p24 (d3d12) | 0.33 | 357 MB | 348 MB | 48 |
| 720p30, d3d11 | 0.39 | 270 MB | 234 MB | 45 |
| 720p24, d3d11 | 0.27 | 265 MB | 229 MB | 31 |
| 1080p30 in a window on the primary, d3d12 | 0.43 | 347 MB | 336 MB | 65 |
| camera 1280x720, about 10 fps | 0.52 | 266 MB | 238 MB | 112 |

Where it goes: decoding and downloading the frame costs 0.07 cores. The rest, about 0.2 to 0.35, is Rune pushing each frame through `push_bytes` (a 3 MB copy and a new texture upload per frame) and drawing. Rune redraws the whole window 20 to 110 times a second while a native surface is mounted, whatever the frame rate: a 10 fps camera redraws 112 times a second at 144 Hz, and the CPU follows the redraw count more closely than the resolution (720p saves memory, 24 fps saves about a fifth of the CPU). The d3d11 decoder uses 60 to 85 MB less than d3d12 for the same CPU. The shadow blur adds up to 0.1.

On a 60 Hz projector the redraw count is capped lower than here; this PC's 144 Hz primary makes the window-on-primary rows worse than a projector would be.

Copy-free path on Windows: Rune has one, `NativeSurface::push_shared(SharedTexture::named(..))`, which copies a named BGRA8 D3D shared texture on the GPU (`native_surface/windows.rs`, adapter from `rune::native_surface::windows::adapter_luid()`; the earlier note that Rune imports GPU frames on macOS only is wrong). It needs the decoder's output converted to BGRA on that adapter (`d3d11convert` or `d3d12convert`) into a texture made with `MISC_SHARED_NTHANDLE` and named with `CreateSharedHandle`, plus a release fence. `gstreamer-rs` has no safe binding for the D3D11/D3D12 memory of a sample, so it needs raw FFI against `gstd3d11`. Not built: it removes the 3 MB copy and upload but not the redraw count, which is the larger part.

What Rune would need to change (not patched): `Presented::same` in `window/presented.rs` requires `rt.surfaces.is_empty()`, and a mounted native surface keeps the window drawing every display tick. Letting a window rest when every mounted surface's newest frame id equals the one drawn last (the surface's waker already wakes it on a push) would draw 30 times a second for 30 fps video and zero for a paused camera. Also `SurfaceRenderer::new` starts a thread that polls the device every millisecond while any frame is in flight, and `submit` adds a second queue submission per frame for timestamps; with `TIMESTAMP_QUERY` not requested for output windows both would go.

Frame cadence (Lerret's `FrameClock` choosing frames on the vsync grid) needs the app to own `present`; Lerret does, Rune does not. The Rune change above gives its main saving (no redraws between frames) and Fifo presents then land on the grid; porting the clock itself waits for that change.

## Not built yet

- Streaming out, NDI.

## The operator's pictures show the video (2026-11-11)

The two small pictures in the operator window (under the Preview list and under the Live list) drew the words on plain black over a video or camera, while the projector showed them over the moving picture (owner: "Why the preview or live view not show the same thing on live?"). They now draw the words over what the projector draws them over.

- **Where frames come from.** `artosia-video` has a tap (`tap.rs`): the appsink callback that feeds the surface, for a file and for a camera alike, offers the newest NV12 frame to one registered function at most every 250 ms, and only while an operator is connected. Off, a frame costs one atomic load. The output registers it in `video.rs`: `artosia_slides::preview::shrink` makes a 256-pixel-wide RGB picture straight from the NV12 planes (four samples averaged per pixel, BT.709 or BT.601, limited or full range, shape kept), and a writer thread sends the newest as `FromOutput::Preview { width, height, rgb }` with the RGB as base64 in the JSON line. A slow socket drops pictures and never holds the decoder.
- **Dropping it.** `go_dark` (a look that is no longer moving, a failed or stopped video, a switch to another file) sends `PreviewEnd` under a lock shared with the writer, so a picture in flight cannot land after it. A closed socket also drops it. The operator keeps the newest picture in a `Pixels` (`link.rs`, `LivePicture`); `Pixels::set` wakes only the windows showing it, so the window redraws once per picture and not at all between them. `ui::thumb` hands it to `Media.picture` only for a look that is the moving look the projector plays now, so the looks list and the editor keep their still picture or colour.
- **Transport.** Base64 in the JSON line, chosen by measuring: 256x144 at 4 a second is 147 KB a line, 0.56 MB/s on the local socket, and the writer thread and the operator's reader thread cost too little to separate from the noise below. A second connection of raw bytes would save a third of that and cost a second socket to keep alive; not built.
- **Measured** (release, the 15 s `bg30.mp4` fixture in a window on the primary 144 Hz screen, `tools/ram/measure-cpu.ps1` with 36 s settle and 15 s window, a stand-in operator connected in every run; `ARTOSIA_NO_PREVIEW=1` makes no pictures): without pictures `cores=0.49 workingSetMB=260 privateMB=249` and `cores=0.51 workingSetMB=255 privateMB=245`; with them `cores=0.46 workingSetMB=256 privateMB=245` and `cores=0.42 workingSetMB=259 privateMB=248`. The difference is far inside this PC's run-to-run spread of 0.1 cores (see the table above), so the cost of the pictures is under what can be measured here. 19 pictures went out in each 5 s, and the file wrapped twice (`loops=2`, `video pos` 14.20 then 1.24 then 0.43 in the trace).
- **Not tried.** A real camera (the owner's was open on the projector, so only a file was played; the camera shares the appsink callback, so it gives the same pictures); the pictures with the operator minimised (they are still decoded and sent).
- Test hooks: `cargo run -p artosia -- --snap out.png --scene camera-background` renders both pictures over a made-up gradient handed in as if the projector had sent it.

## Checking the projector on a real screen

`--snap` cannot see what Windows does to a real window. On 2026-11-10 the owner photographed lyrics running off the right edge of the projector: Windows had opened the window at the second screen's 150% zoom, 2304x1296 pixels on a 1920x1080 screen, and the view had laid the first slide out for a size it did not have. Every headless render was correct. The output now opens its windows hidden, fits them to the screen, draws only the background until the size is known, and says so in the trace ("size check ... same").

To check a change: start `artosia-output.exe --port 47932` from a copy of the build, run `python tools/send-state.py 47932 6`, which sends where to show and one made-up slide and then asks it to quit, and capture the window meanwhile with `tools/grab-window.ps1 -Name artosia-output -Out shot.png`. The capture tool is per-monitor DPI aware; before that it returned 1600x900 for this screen and cropped the picture, which looked like the defect it was checking for. Checked this way on 2026-11-10: the slide is centred and whole at 1920x1080, and the output rests at 0.01 cores and 142 MB with nothing moving.

## The first screen is never covered (2026-11-11)

The projector goes full-screen and always on top. On the screen that holds Artosia's own window that locks the volunteer out of the program: nothing of the operator can be reached to move it back. The owner hit it on 2026-11-10 ("cant return back to screen 1 as the app only") and the Screens dialog saved screen 1 as the projector's screen. On 2026-11-11 a fresh start honoured that saved choice and covered the owner's main screen, a 2560x1080 display, over every other program.

`displays::plan` now passes over a pin on the first screen, for the projector and for the stage view: with another screen there it takes that one, and with no other screen it is the one-screen case, where no projector window opens unless "show it here" asks for an ordinary window. The Screens dialog offers no role on that screen's row and says why (`ui/sheets.rs`, `SCREEN_OWN`). Tried on the owner's two screens: asked for screen 1, the projector opened at 2560,0 on screen 2.

"The first screen" is the one Windows calls primary, where Artosia's window opens. A volunteer who drags Artosia's window to another screen and wants the projector on the primary one is not served; the output would need to be told which screen the operator window is on.

