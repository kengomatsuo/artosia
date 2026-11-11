---
paths:
  - "crates/artosia-output/**"
  - "crates/artosia-slides/**"
  - "crates/artosia-video/**"
  - "crates/artosia/src/link.rs"
---

# The output process and what the screens draw (2026-11-10)

- **A CAMERA WHOSE NAME IS AN ADDRESS IS OPENED OVER THE NETWORK, AND THE PROJECTOR PROGRAM SENDS A TROUBLE AS A KIND THAT THE OPERATOR WORDS (`camera.rs` `open_address`, `video.rs` `words_for_start_failure`, `crates/artosia/src/why.rs` `media_problem`, 2026-11-11).** Its English sentences reached an Indonesian status bar. Full account: [docs/output.md](../../docs/output.md)
- **THE PROJECTOR IS ITS OWN PROCESS AND KEEPS ITS LAST STATE WHEN THE OPERATOR DIES (`artosia-output`, port 47823, 2026-11-10).** Only a `Quit` message closes it. Full account: [docs/output.md](../../docs/output.md)
- **A VIDEO BACKGROUND COSTS 0.25 TO 0.45 CORES AND 300 MB IN THE OUTPUT, MEASURED DRAWN, AND MISSES THE TARGET (2026-11-10).** Decode and download are 0.07 cores; Rune redrawing the window 20 to 110 times a second while a surface is mounted is the rest; the d3d11 decoder saves 60 MB over d3d12. Re-measure with `tools/ram/measure-cpu.ps1` after any change to this path. Full account: [docs/output.md](../../docs/output.md)
- **EVERY SIZE ON A SCREEN IS A FRACTION OF ITS HEIGHT, AND PREVIEWS ARE THE 1920x1080 VIEW SCALED DOWN (`artosia-slides`, 2026-11-10).** A preview must break lines where the projector does. Full account: [docs/output.md](../../docs/output.md)
- **ONLY `artosia-video` LINKS GSTREAMER, BEHIND `artosia-output --features video` (2026-11-10).** The operator must build and run on a machine without it. Full account: [docs/output.md](../../docs/output.md)
- **HEADLESS `--snap` CANNOT SHOW VIDEO; CAPTURE THE REAL WINDOW WITH `tools/grab-window.ps1` (2026-11-10).** Full account: [docs/output.md](../../docs/output.md)
- **THE OUTPUT SHOWS NO PROJECTOR WINDOW ON ONE SCREEN, GOES FULL-SCREEN ON A SECOND, AND NEVER STARTS GSTREAMER'S DEVICE MONITOR UNTIL A CAMERA IS USED (2026-11-10).** Opening a window while video plays holds the video back (wgpu `GpuWaitTimeout` panic otherwise); the camera list costs 50 MB if GStreamer makes it. Full account: [docs/output.md](../../docs/output.md)
- **A PROJECTOR CHANGE IS CHECKED ON A REAL WINDOW ON A ZOOMED SECOND SCREEN (`tools/send-state.py`, `tools/grab-window.ps1`, 2026-11-10, owner-reported).** Windows opened it at the screen's 150% zoom, 2304x1296 on 1920x1080, and the first slide was clipped; every headless render was right. Full account: [docs/output.md](../../docs/output.md)
- **THE PROJECTOR AND THE STAGE VIEW NEVER COVER THE FIRST SCREEN, WHERE ARTOSIA'S OWN WINDOW IS (`artosia-output/src/displays.rs` `plan`, 2026-11-11, owner-reported).** A saved pin on screen 1 covered the owner's main screen at start, on top of everything. Full account: [docs/output.md](../../docs/output.md)
- **THE OPERATOR'S TWO PICTURES DRAW THE PROJECTOR'S OWN VIDEO OR CAMERA FRAME, SENT 4 TIMES A SECOND AS A 256-PIXEL `Preview` (`artosia-video/src/tap.rs`, `artosia-slides/src/preview.rs`, `link.rs` `LivePicture`, 2026-11-11, owner-reported).** The output drops it with `PreviewEnd` when the background stops or changes, and the operator shows it only for the look the projector plays. Full account: [docs/output.md](../../docs/output.md)
