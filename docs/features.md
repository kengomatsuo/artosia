# Key features (owner, 2026-11-10)

1. Live camera + lyrics, streamed: any video input, USB or wireless, from anywhere.
2. Song search that forgives typos and is instant; results show the title and the matching lyric lines highlighted, and look good.
3. Works with any second screen or projector.
4. Warp and adjust the output video freely (keystone, corner pin, colour).
5. Listen to an audio input (3.5 mm or any device): hear sung lyrics to name the verse and suggest it; hear notes and suggest the song.
6. Add song lyrics from the web quickly.
7. Suggest the next slide / auto-advance.
8. Phone control and phone streaming on the same Wi-Fi, including across routers of one network.
9. Many quality-of-life features the legacy apps lack.

## What the installed GStreamer 1.28.8 covers (checked with `gst-inspect-1.0`, 2026-11-10)

| Need | Elements present on this PC |
|---|---|
| USB cameras and capture cards | `mfvideosrc`, `ksvideosrc`, `dshowvideosrc` |
| Wireless / network cameras and phones | `rtspsrc`, `srtsrc`, `whepsrc` (WebRTC), `ndisrc` (needs the NDI runtime) |
| Screen capture | `d3d11screencapturesrc` |
| Audio input (3.5 mm, USB, any device) | `wasapi2src`, `wasapisrc`; `webrtcdsp` for echo/noise cleanup; `spectrum` for pitch analysis |
| Streaming out | `rtmp2sink`, `srtsink`, `whipclientsink` (WebRTC), `hlssink2`, `ndisink` |
| Hardware encode | `nvh264enc` (NVIDIA), `qsvh264enc` (Intel), `mfh264enc` (Media Foundation); `x264enc` software |
| One source to several outputs | `tee`, `compositor`, `d3d11compositor` |
| Warp and colour | `perspective`, `d3d11convert`, `glcolorbalance` |

Not checked yet: the same elements on macOS and Linux builds; speech-to-text and pitch-to-song models (feature 5) sit outside GStreamer.
