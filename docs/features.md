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

## Where each stands (2026-11-10)

| # | Feature | State |
|---|---|---|
| 1 | Live camera and streaming | Built. A camera can be the background (Design, Camera); when it drops out the words stay on black and it comes back by itself. `artosia-stream` sends the projector picture and the room's sound to RTMP, SRT or a file (Stream in the top bar). Proven against a local ingest and a recording; no real service was reached. [output.md](output.md), `crates/artosia-stream/README.md` |
| 2 | Song search that forgives typos | Built, ordered by a cascade after the owner's house-kit palette: songs in today's plan first, the title before the lyrics, `diberkati` found by `berkat`. [operator.md](operator.md) |
| 3 | Any second screen or projector | Built. One screen shows no projector window; a second screen is taken full-screen at once and released when it goes; the first frame is laid out for the real window. Proven on a 1920x1080 screen at 150%. Unplugging was simulated, never done by hand. [output.md](output.md) |
| 4 | Warp and colour | Built: four corners nudged from the Screens dialog, brightness, contrast and colour, drawn by `artosia-warp` over the whole projector picture. A bent picture was checked in the warp crate's own example, and through the projector only unbent |
| 5 | Listening to audio | Half built. The microphone, the level meter and the phrase cutting work (Listen in the top bar). Speech recognition (whisper.cpp) builds with `--features speech` once CMake and libclang are found by `tools/speech-env.sh`, and `tools/publish.sh` then ships it: a synthesised English sentence was heard word for word in 0.3 s using 133 MB; Indonesian is unheard (no voice on this PC. F4 puts what it recognised in Preview. Melody recognition is not built. The room's tempo is found from the microphone and shown in the Listen dialog, and a switch (off) lets the Live label pulse on the beat; what the beat could tell about a slide's last line is written down, not built. `crates/artosia-listen/README.md` |
| 6 | Lyrics from the web | Built: a new song's title offers songs from LRCLIB and fills the editor on Tab; paste still works. LRCLIB found 2 of 20 sampled Indonesian titles. [operator.md](operator.md) |
| 7 | Next slide | Built: Next runs through the item and on to what waits in Preview, which Artosia fills with the next plan item; Shift and an arrow arm another starting slide for a medley; number keys and the letters C, V, B and R jump to sections; announcements loop on a timer. Nothing advances a song by itself |
| 8 | Phone control | Built: live slide, sections, plan, search, messages and countdown from a web page on the same Wi-Fi. Not tried from a real phone in this session. A phone cannot be the camera or microphone from that page: browsers give those only to secure pages, and a LAN address is not one (`crates/artosia-remote`, sources in the builder's report, 2026-11-10) |
| 10 | A plan for each service | Built 2026-11-11 (owner: "Where's the multiple-plan support?"): plans kept in the library, named, dated, duplicated, deleted with Undo, switched from the plan column's title or the search; a switch never touches the projector. A plan may have its own look (camera for the morning, none for Sunday school), chosen at the top of the style controls and marked in the list; built 2026-11-11, not tried in a real window. [operator.md](operator.md) |
| 9 | Quality of life | Built so far: a strip over a song's words that proposes a section mark for every stanza (a chorus sung again, a written Reff or Bait 2, a late short bridge, verses in order) and writes them on one press, with the kinds to pick for the stanza at the caret ([operator.md](operator.md)); a five-stop tour of the window on first start, which Skip ends and the keys sheet, the search and the first screen start again ([operator.md](operator.md)); one search for everything; every action from the keyboard; light and dark; English and Indonesian; PowerPoint and birthday-list import; self-filling birthday slides that say which days they cover and how many names they hold; a verse of the week read from the installed Bible; Show now on any announcement card; one Artosia per library; updates from GitHub Releases without a USB stick; a picker for free Bible translations and a church's own Bible file; a search that learns from what was picked; room for more interface languages; the projector survives an operator crash; the words' colour, font and outline are set in the Look dialog and saved with a look ([output.md](output.md)) |
| 12 | Ready backdrops | Built 2026-11-11 (owner: "We need at least 250"): a gallery of 318 photos and looping videos from Wikimedia Commons, all CC0 or public domain, opened from Design's background row or the search. Nothing ships but a list of addresses; a file is fetched on a press into the library's media folder and then works offline. A VP9 video from the catalogue looped in the video crate's example with hardware and with software decoding; it was not watched on the projector window. [backdrops.md](backdrops.md) |
| 11 | What to show next | Built 2026-11-11 (owner: a slide setter forgets a medley song and panics; a priest starts a song unannounced; a volunteer wants to be told what usually comes next). One strip of up to three rows at the top of the Preview pane: what the listener heard, a far part of the live song, a line that the words being sung are in no song, and what usually follows the live song. Each brings itself into Preview on one press, F4 for the first, and never onto the projector. Not placed in the real window until the shell takes `ui/suggest.rs`; rendered through the `suggest-*` scenes. Not tried with a real model or a real microphone. [palette-suggestions.md](palette-suggestions.md) |

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

On macOS and Linux (GStreamer plugin docs, each page fetched 2026-11-10): cameras `avfvideosrc` (macOS) and `v4l2src` (Linux); audio input `osxaudiosrc` and `pulsesrc`; hardware decode `vtdec` and `vah264dec`; hardware encode `vtenc_h264` and `vah264enc`.

Speech-to-text and pitch-to-song models (feature 5) sit outside GStreamer; it supplies the audio input and `spectrum` analysis.

## libVLC 3.0.24 for comparison (plugins in the `VideoLAN.LibVLC.Windows` NuGet build, listed 2026-11-10)

Has: screen capture (`libscreen`), RTSP (`liblive555`), SRT in and out, WASAPI audio input, Intel QSV and x264 encoders, `transform` and `adjust` filters.
Missing from this build: a USB camera input (no `dshow` access plugin), RTMP output, WebRTC in or out, NDI, a perspective/corner-pin warp. Its frame callbacks also decoded in software in our test (0.27 cores against GStreamer's 0.05).
