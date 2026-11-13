---
paths:
  - "crates/artosia-stream/**"
  - "crates/artosia/src/stream.rs"
---

# The streaming program (2026-11-13)

- **EACH SYSTEM HAS ITS OWN PICTURE SOURCE, SOUND SOURCE AND ENCODERS BEHIND ONE PROGRAM, AND THE OPERATOR OFFERS STREAMING WHEREVER `artosia-stream --probe` ENDS WITH 0 (`capture.rs` `picture_source`, `encode.rs` `device_source`, `encoder.rs` `order`, `crates/artosia/src/stream.rs` `available`, 2026-11-13).** Full account: [crates/artosia-stream/README.md](../../crates/artosia-stream/README.md)
- **UNDER WAYLAND THE PICTURE IS ASKED OF THE DESKTOP'S PORTAL, NEVER OF X11, WHICH SEES XWAYLAND PROGRAMS ONLY (`portal.rs`, `run.rs` `shares_screens`, 2026-11-13).** The desktop asks the person once; its answer stands until the program ends, and the token it returns is kept for the next stream. Full account: [docs/linux.md](../../docs/linux.md)
- **A QUESTION TO X11 RUNS ON ITS OWN THREAD WITH TWO SECONDS TO ANSWER (`win.rs` `ask`, 2026-11-13).** WSL's X display takes a connection and never answers, which held the stream and its stopping. Full account: [docs/linux.md](../../docs/linux.md)
- **ONLY THE SOUND INPUTS THIS PROGRAM'S OWN SOURCE OPENS ARE LISTED, AND A PROPERTY IS LOOKED FOR BEFORE IT IS READ (`list.rs` `audio_devices`, 2026-11-13).** On a PipeWire desktop GStreamer lists each microphone again through `pipewiresrc`, which has no `device`; reading it ended the listing and 0.1.25 streamed silence there. Full account: [docs/linux.md](../../docs/linux.md)
- **A SOUND CHANGE IS RECORDED ON A PIPEWIRE SERVER WITH A DEFAULT INPUT OF OUR OWN, `tools/linux/stream-mic-rig.sh` (2026-11-13).** WSL's own sound server has no microphone that opens, so it showed nothing of what a real desktop does.
- **THE FRAME'S LAYOUT FOLLOWS THE ENCODER: OPENH264 TAKES I420, EVERY OTHER NV12 (`encoder.rs` `frame_format`, 2026-11-13).** With NV12 the software encoder of a plain Linux was refused and no encoder was left. Full account: [docs/linux.md](../../docs/linux.md)
- **ON A MAC A TEST NEVER STARTS THE SCREEN CAPTURE: IT RECORDS THE OWNER'S SCREEN AND MAKES macOS ASK FOR A PERMISSION THAT IS THE OWNER'S (2026-11-13).** The parts before and after the capture are run on a test picture. Full account: [docs/macos.md](../../docs/macos.md)
- **A STREAM IN A TEST GOES TO A FILE OR TO A LISTENER ON 127.0.0.1, NEVER TO A REAL SERVICE (2026-11-13).** Full account: [crates/artosia-stream/README.md](../../crates/artosia-stream/README.md)
