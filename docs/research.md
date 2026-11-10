# Desktop app stacks (Windows, macOS, Linux)

Researched 2026-10-06 for the EasiSlides replacement (a free worship presentation app):
operator UI plus fullscreen output windows with text over looping video. Sources are the
projects' repos (`gh api`), official docs and Microsoft/Apple references. Re-check
versions before relying on a row.

## Contents

- Frameworks
- Hard constraints
- Media and output layer
- Worship-app siblings and their stacks

## Frameworks

| Stack | Render | Multi-monitor fullscreen | Video | Verdict |
|---|---|---|---|---|
| Tauri 2 (v2.12.1, Apache/MIT) | system webview: WebView2 on Windows, WKWebView on macOS | `availableMonitors()`, `setFullscreen()`, Rust `set_fullscreen_on_monitor()` ([window.ts](https://github.com/tauri-apps/tauri/blob/dev/packages/api/src/window.ts)) | HTML `<video>`; codecs follow each OS webview | smallest install, same RAM as Electron on Windows (measured below); two engines to test |
| Electron (v44, MIT) | bundled Chromium | `screen.getAllDisplays()`, `BrowserWindow` `x/y/fullscreen` | Chromium `<video>`, identical on both OSes | one engine on both OSes, offscreen rendering to a shared GPU texture feeds NDI/Spout/Syphon ([docs](https://www.electronjs.org/docs/latest/tutorial/offscreen-rendering)); FreeShow uses it; heaviest install |
| Qt 6 QML (LGPL/GPL or paid) | Qt Quick scene graph (RHI) | `QGuiApplication::screens()`, `QWindow::setScreen()`, `showFullScreen()` | Qt Multimedia, FFmpeg backend by default | lower working set than Electron, higher private memory (measured below); OpenLP 4 moves here; Rust via cxx-qt |
| Flutter desktop | own renderer | multi-window API still experimental and `@internal` ([_window.dart](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/_window.dart)) | `video_player` lacks Windows; media_kit (libmpv) | ruled out while multi-window is experimental |
| Avalonia 12 (MIT) | Skia | `TopLevel.Screens`, `WindowState.FullScreen` | no built-in; LibVLCSharp `VideoView` hosts a native window, and text over it works through `VideoView.Content` (measured below) | lightest cross-platform stack measured |
| .NET MAUI | native controls | n/a | n/a | macOS only through Mac Catalyst: ruled out |
| GPUI (Zed) | own GPU renderer | unverified | video `Surface` is macOS-only ([surface.rs](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/surface.rs)) | ruled out for Windows video; pre-1.0 |
| Slint (GPLv3 free / paid) | Skia, software | unverified | GStreamer example, nothing built in | possible for a GPL app; video is DIY |
| Iced, egui, Xilem, Makepad, Blitz | wgpu / own | egui has `ViewportBuilder` | none | experimental; you build video and text yourself |
| Compose Multiplatform desktop | Skia | `WindowPlacement.Fullscreen`, no per-monitor API | vlcj in a SwingPanel cannot have Compose text over it; vlcj's `CallbackVideoSurface` drawn on a Canvas can, with software decode | ChurchPresenter uses it |
| Rune (anomalyco, MIT, pre-release) | own GPU renderer (wgpu; Metal on macOS) | yes | `Player` is AVPlayer on macOS only; elsewhere push decoded BGRA/NV12 bytes into a `NativeSurface` | every app runs headless (`--snap` PNG, scripted input); pin a `dev` rev |
| Wails v3 (Go) | system webview | `Fullscreen()`, `SetScreen()` | webview `<video>` | v3 still beta |
| WinUI 3 + SwiftUI/AppKit | native | native | `MediaPlayerElement`; `AVPlayerLooper` | best media, two codebases |

## Measured RAM (2026-10-06, Windows 11, 32 GB PC, RTX 4060 laptop)

A control window plus a 1920x1080 output window looping a 1080p30 H.264 file (8 Mb/s, faststart) under fading lyrics, measured 40 s in (during the second loop), twice, summed over the process tree. Each app's position log was checked to confirm it looped. The artosia repo keeps the measuring script (`tools/ram/measure.ps1`) and the test video generator.

| Stack | Platforms | Video path | Working set | Private |
|---|---|---|---|---|
| WPF MediaElement (.NET 9) | Windows | Media Foundation | 182 MB | 247 MB |
| WinUI 3 MediaPlayerElement | Windows | Media Foundation | 236 MB | 193 MB |
| Avalonia 12 + LibVLCSharp | all | libVLC, D3D11VA hardware | 250 MB | 294 MB |
| EasiSlides 5.2 (WinForms, Media Player control) | Windows | WMP ActiveX | 355 MB | 306 MB |
| Qt 6.10 QML via `qml.exe` (C++, no Python) | all | Qt Multimedia, FFmpeg | 355 MB | 427 MB |
| Qt 6.11 QML via PySide6 | all | Qt Multimedia, FFmpeg | 372 MB | 386 MB |
| Slint 1.18 (femtovg) + ffmpeg pipe | all | software, RGBA copy | 394 MB | 472 MB |
| egui 0.36 + ffmpeg pipe | all | software, RGBA texture each frame | 496 MB | 740 MB |
| Iced 0.14 + ffmpeg pipe | all | software, new image handle each frame | 506 MB | 630–860 MB |
| Tauri 2.12 | all | WebView2 `<video>` | 548 MB | 324 MB |
| Electron 44.5 | all | Chromium `<video>` | 551 MB | 349 MB |
| JavaFX 26 / Compose 1.12 (vlcj callbacks) | all | Media Foundation / libVLC software | 2.5–3.9 GB | 3.2–7.2 GB |

- EasiSlides was measured with an earlier test file that had no duration in its header, so it may not have been looping. Chromium's MediaRecorder writes such MP4s, and WPF cannot loop them.
- The JVM rows are on a 32 GB PC, where the default max heap is 25% of RAM (`MaxRAMPercentage=25`, 8 GB here). A rerun capped at 1 GB, which is what a 4 GB PC gets by default, was cut off before it reported.
- Flutter was not measured: the SDK bootstrap took the whole session.
- Tauri saves no RAM over Electron on Windows. The ffmpeg-pipe rows show the cost of copying full RGBA frames on the CPU, not of the toolkits themselves.

## Hard constraints

- libmpv's render API is OpenGL only ([render.h](https://raw.githubusercontent.com/mpv-player/mpv/master/include/mpv/render.h)); a wgpu (DX12/Metal) renderer needs GL interop or its own FFmpeg decode.
- WebView2 codecs ([Microsoft](https://learn.microsoft.com/en-us/troubleshoot/microsoft-edge/development/video-playback-issues)): H.264, VP8/VP9 and AV1 supported; HEVC only with the HEVC Video Extension. Ship backgrounds as H.264 or VP9.
- WKWebView autoplay without a gesture: set `mediaTypesRequiringUserActionForPlayback` to none (macOS 10.12+).
- HEVC-with-alpha plays natively on macOS 10.15+ only; there is no proven alpha-video format for both webviews.
- Open Tauri video bugs: #7355 (`convertFileSrc` large videos on macOS), #10426, #5605. Serve video through a custom protocol or a local HTTP server and test early.
- Windows: declare Per-Monitor V2 DPI and handle `WM_DPICHANGED`; Microsoft recommends borderless flip-model windows over exclusive fullscreen.
- macOS: native fullscreen opens a new Space; a projector window is a borderless window sized to the `NSScreen` frame.

## Media and output layer

- NDI SDK: royalty-free; bundle the runtime, ship `Processing.NDI.Lib.Licenses.txt`, link ndi.video, and "NDI" in a product name needs permission ([licensing](https://docs.ndi.video/all/developing-with-ndi/sdk/licensing)). Rust: `grafton-ndi` (Apache-2.0, v1.0.0 2026-06). Node: `ChurchApps/grandiose`.
- OBS: obs-websocket v5 ships with OBS 28+. Syphon (macOS, BSD) and Spout2 (Windows, BSD-2) share GPU textures, which a webview cannot hand out directly.
- CCLI SongSelect: no API for new partners (FreeShow FAQ); import files only.
- Lyrics format: OpenLyrics XML 0.8. Bible text: API.Bible free tier allows 3 licensed Bibles for non-commercial use; public-domain translations can ship offline.
- Rust text: cosmic-text + glyphon on wgpu, or parley; shaping by HarfRust.

## Worship-app siblings and their stacks

| App | Stack |
|---|---|
| FreeShow | Electron + Svelte, NDI via grandiose, GPL-3.0 |
| OpenLP | Python + PyQt; 4.0 alphas move to Qt 6 media, drop VLC |
| Quelea | Java + JavaFX, GPL-3.0 |
| Holyrics | Java, closed freeware |
| ProPresenter | C#/WPF on Windows, Swift/ObjC on macOS |
| ChurchPresenter | Kotlin Compose, VLC + ffmpeg, NDI, DeckLink, GPL-3.0 |
| OpenAdoration | C# WPF + FFME, MIT |
| lyric-display-app, bible-song-pro-studio | Electron + React |
| EasiSlides (Easyslides 5.2) | C# WinForms on .NET Framework 4.8 (read from the exe); video through the Windows Media Player ActiveX control, DirectShowLib, SQLite, Office interop for PowerPoint; freeware, unsigned |
