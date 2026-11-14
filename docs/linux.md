# Artosia on Linux

Decided and built on 2026-11-12, on the owner's order to build the macOS and Linux editions (board card PORT1). Do not relitigate the choices below without new evidence. The research behind them is saved in `~/Shared/research/artosia-mac-linux/` (its `README.md` holds the table of projects and every source).

## What a person gets

One file, `Artosia-linux-x86_64.tar.gz`, that unpacks to a folder `Artosia`:

| In the folder | What it is |
|---|---|
| `artosia` | The operator window. It starts the projector program from this same folder. |
| `artosia-output` | The projector and stage windows, built with video. It links the distribution's GStreamer. |
| `artosia-output-plain` | The same program built without video. The operator starts it when the one with video cannot start. |
| `artosia-stream` | The streaming program: the projector's picture and the room's sound to a streaming service or a file. See "Streaming". |
| `share/applications/artosia.desktop`, `share/icons/hicolor` | The menu entry and the icons (`assets/brand/icons/linux`). |
| `install.sh` | Writes the menu entry into `~/.local/share/applications` with this folder's full paths. Nothing needs an administrator. `--remove` takes it away. |
| `README.txt` | How to start, which GStreamer packages video needs, how updates work, how to remove. |

The library and the settings live apart from the folder, in `~/.local/share/artosia` (Rune's `dirs.rs`, `$XDG_DATA_HOME`).

## The one-line install (2026-11-14)

```bash
curl -fsSL https://artosia.matsuokengo.com/install.sh | bash
```

`tools/linux/get.sh`, served by the homepage as `install.sh`, does what a person would do by hand: it takes the newest release that carries the Linux package (a release built while WSL was busy has none, so `latest` alone can point at nothing), unpacks it into `~/.local/share/artosia/` (`$ARTOSIA_HOME` overrides), and runs that folder's `install.sh` for the menu entry. Nothing needs an administrator. A folder already there that holds no Artosia is refused and left alone. The new folder replaces the old one in two renames, so a running copy keeps its open files. Its shape follows bun's installer: strict mode, curl that fails on an error page, a home-folder install, and the whole script in one function so half a download runs nothing. bun, rustup and Ollama each serve their line from their own domain, and so does Artosia. Checked in WSL on 2026-11-14 with an empty home folder: installed, installed again over itself, and refused a foreign folder.

## Why a folder and not an AppImage, a deb or a Flatpak

| Choice | Decision | Why |
|---|---|---|
| A `.tar.gz` unpacked into the person's own folder | Taken | Artosia updates itself by swapping files in its folder, which must be writable. Zed and Lapce ship exactly this and update the same way. |
| The distribution's GStreamer | Taken | Hardware decode belongs to the system: the `va` plugin and its driver, or NVIDIA's decoder, cannot be carried along in a meaningful way. `gstreamer-vaapi` left GStreamer in 1.28; the `va` plugin replaced it. |
| AppImage | Left | A read-only image that can only be replaced whole, and its GStreamer plugin for linuxdeploy has five stars and no stated licence. |
| A deb or a Flatpak | Not now | Both update outside Artosia, so its own updater would have to be switched off for them. Add one only when asked. |

## What had to change in the code

Nothing to make it compile: every Windows-only path was already behind `cfg(windows)` (`firewall.rs`, `store.rs`, `shortcut_icon.rs`, `artosia-locale`, the whole `artosia-setup` crate), and `artosia-locale` already read `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG` off Windows. The first `cargo build` on Linux ended with no error. Three things were added:

1. **One signed list per system on a release** (`crates/artosia-update/src/manifest.rs`). Windows, published first, keeps `manifest.json`, `manifest.json.sig` and plain file names, because every installed copy and the setup ask for those. Another system's list and files carry its name: `manifest-linux-x86_64.json`, `manifest-linux-x86_64.json.sig`, `linux-x86_64__artosia`. A copy asks for its own system's list. Full account: `crates/artosia-update/README.md`.
2. **The operator asks the projector program whether it can start** (`link.rs` `can_start`, `artosia-output --probe`). On Windows the program with video needs GStreamer's folder. On Linux it links the system's GStreamer, and without those libraries the system's loader ends it before it runs; so the operator starts it once with `--probe`, which returns at once, and falls back to `artosia-output-plain` when that fails.
3. **The package and the release step**: `tools/package-linux.sh`, `tools/linux/`, and a step in `tools/publish.sh`.

## Preparing a machine to build it

Done once on the release PC's WSL (Ubuntu 26.04), as `target/port/wsl-setup.sh` did it:

```bash
sudo apt-get install -y build-essential pkg-config cmake clang libclang-dev curl git \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libgstreamer-plugins-bad1.0-dev \
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav gstreamer1.0-tools \
  libasound2-dev libudev-dev libdbus-1-dev libssl-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev libxcb1-dev \
  libfontconfig1-dev libfreetype-dev libvulkan1 mesa-vulkan-drivers libgtk-3-dev ffmpeg
curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none --profile minimal
gh repo clone kengomatsuo/artosia ~/artosia     # rust-toolchain.toml then fetches 1.99.1
```

The list is a superset: it was written from Rune's and GStreamer's documentation before the first build, and the build passed with it. Which of the windowing packages a build can do without was not narrowed down. `cmake`, `clang` and `libclang-dev` are for speech recognition; `ffmpeg` only makes the test video (`tools/make-fixture.sh`).

## How a release carries it

**Paused since 2026-11-14.** The Linux edition was built in the release PC's WSL, and the owner put WSL off limits that day (it is suspected of dropping the F drive, where its disk lives). Until another Linux builder exists, a release carries no Linux files, the homepage and the one-line install take the newest release that has them (0.1.31), and installed copies keep that version.

Until then, `tools/publish.sh` ran in WSL, from the release's tag:

```bash
cd ~/artosia && git fetch -q origin --tags --force && git checkout -q --detach v<version> && bash tools/package-linux.sh <version> --out <the Windows checkout>/dist/<version>/linux --key-file <the signing key> --notes-file <notes>
```

and uploaded `Artosia-linux-x86_64.tar.gz` and everything in `dist/<version>/linux/feed` to the same release. A release that stopped as a draft is finished with `tools/publish.sh <version> --finish`, which builds only the systems the draft lacks.

## Checked on 2026-11-12

On the release PC's WSL: Ubuntu 26.04.1, kernel 6.18 (WSL2), Rust 1.99.1, GStreamer 1.28.2 from Ubuntu's packages, an NVIDIA RTX 4060 passed through by WSL.

| What | Result |
|---|---|
| `cargo build`, first try | No error. |
| `cargo test` | Operator 227 passed (4 ignored; the two tests fewer than on Windows are Windows-only), core 181, updater 13, the rest green. |
| Build with video (`--features artosia-output/video`) | Passes. |
| Headless render of the operator (`--snap`) | The same picture as on Windows. |
| The operator as a real window | Opens on an X11 screen and takes real key presses (`xdotool`): the first-start tour showed, Enter brought a song into Preview. |
| The projector as a real window over a looping video | Draws the slide over the video. The 15-second test video wrapped six times in 100 seconds; decoder `nvh264dec` (NVIDIA's, hardware). |
| The projector's cost, release build, video wrapped twice first | Peak 358 MB resident, 45 presents a second. CPU read 6.9 cores, which says nothing about a real desktop: the virtual screen has no GPU, so every pixel was drawn on the CPU. |
| The package | `Artosia-linux-x86_64.tar.gz`, 49 MB: `artosia` 46 MB, `artosia-output` 33 MB, `artosia-output-plain` 32 MB, `artosia-stream` 0.8 MB, built with speech recognition. |
| A copy updating itself | Version A, unpacked from its tar.gz into a scratch folder, asked `manifest-linux-x86_64.json` on a local feed, fetched version B in 6 seconds, and the next start came up as B: the program's hash was B's, every program was still executable, the `update` folder was empty and no old file was left. Signed with a key made for the test, in a copy of the repository, so the real key and the real clone were not used. |

Windows after these changes: operator 229, projector 5, updater 13 tests pass.

Pictures are in `target/port/linux-windows/` on the release PC (not in the repository).

## How the windows were pictured

WSL shows a Linux window through a Windows process (`msrdc.exe`) that runs in session 0 on this PC, where nobody sees it and no screenshot reaches it; and the operator's connection to WSL's Wayland display was reset ("Connection reset by peer" in winit's Wayland event loop), which ended it. So the windows were opened on a virtual X11 screen (`Xvfb :99 -screen 0 1920x1080x24`, `WAYLAND_DISPLAY` unset) and pictured from inside Linux with ImageMagick's `import`; `xdotool` sent the keys. The projector did run on WSL's own display earlier in the day and played the video there too.

## The projector under Wayland (2026-11-13)

From 0.1.22 to 0.1.26 the projector and stage windows never appeared on a Wayland desktop, which is what GNOME, KDE and sway run. Every window of the output program starts hidden and is shown once it is fitted to its screen (`placer.rs` `settle`), and the fitting waited for the window's position. Wayland tells a program no position, by design, so Rune's `WindowHandle::placement` is `None` there for good and the window stayed hidden. Nobody saw it: WSL's own display cannot be pictured, and the video's own log kept counting. It was found when the sway test desktop built for streaming showed a black picture.

Two changes, in 0.1.27:

1. **Where no position is told, the window is shown at once** (`displays.rs` `tells_positions`, `placer.rs`). It opens where the desktop puts it.
2. **On a Wayland desktop the output program takes the desktop's X11 server, Xwayland, when it answers within two seconds** (`main.rs` `prefer_x11`, which clears `WAYLAND_DISPLAY` for its own process before the window library reads it; winit takes Wayland whenever it is set). Under Wayland a program cannot put a window on a chosen screen, keep it above others or read where it is; an X11 program can ask for all three, and mutter allows an X11 program its own position by default (`src/x11/window-x11.c`, `allow_position_change = TRUE`). The streaming program then takes the window's picture straight from X11 and the desktop asks nothing. `ARTOSIA_WAYLAND=1` keeps the windows on Wayland. The operator's own window stays on Wayland.

Measured on a sway desktop with no screen in WSL (`tools/linux/wayland-rig.sh`), sway asked what it shows; the rows from "Wayland alone" down were run on the build and again on the published 0.1.27 package, with the same results:

| Case | Result |
|---|---|
| Before the change | The operator's window and a plain GStreamer window appear; the projector's does not, with Vulkan or OpenGL: 0 frames presented. |
| Wayland alone | `Artosia projector` appears as a Wayland window, 1920x1080, 194 frames presented in 14 s. |
| An X11 server that answers beside it (`Xvfb`) | The window goes to the X11 screen. |
| The same with `ARTOSIA_WAYLAND=1` | It stays on Wayland. |
| sway with its own Xwayland (run in a mount namespace, because sway refuses WSL's `/tmp/.X11-unix`) | The projector is an `xwayland` window of 1920x1080 and the streaming program records it straight from X11: 10.3 s of H.264, and a frame from the file shows the lyrics over the video. |
| WSL's own X display, which never answers, beside a Wayland desktop | The projector stops drawing after its video starts. The published 0.1.26 does the same, so it is older than this change and WSL's own; a real desktop refuses a missing X server at once. |

## Streaming (2026-11-13)

The streaming program was Windows' own: it took the picture, the sound and the encoder from Windows' parts of GStreamer. Each system now has its own parts behind the same program, and the Stream dialog offers streaming wherever the program starts (`crates/artosia/src/stream.rs` `available` asks it with `--probe`, as the projector program is asked).

| Part | On Linux | Why |
|---|---|---|
| The windows and monitors | X11 through `x11rb` (`win.rs`), asked on a thread of its own with two seconds to answer | The crate is already in the build for the window library. WSL's own X display takes a connection and never answers, which held the program and its stopping until the question got a time limit. |
| The picture under X11 | `ximagesrc` on the projector window's id, or on a monitor's corners | It is GStreamer's own X11 source. |
| The picture under Wayland | The desktop's portal (`org.freedesktop.portal.ScreenCast` through the `ashpd` crate, `portal.rs`) and `pipewiresrc` | Under Wayland no program may look at another's window. winit takes Wayland whenever `WAYLAND_DISPLAY` is set (its `platform_impl/linux/mod.rs`), so the projector is a Wayland window there and X11 cannot see it; a developer of OBS's portal capture writes that X11 capture under Wayland reaches Xwayland programs only. The portal's page says a start "will typically result the portal presenting a dialog", so the desktop asks the person which screen to share. The token it hands back is kept in `$XDG_STATE_HOME/artosia/screen-share-token` and offered the next time, so a desktop that honours it does not ask again. Since 0.1.27 the projector goes through Xwayland where the desktop has one, so its window is found by X11 and the portal is only what is left for a desktop without Xwayland. |
| The sound | `pulsesrc`, on PulseAudio or PipeWire's PulseAudio service | An input the sound server flags as the default is taken; where it flags none (WSL's does not), the source is opened with no device named, which is the server's own default. |
| The encoder | `vah264enc`, then `nvh264enc`, then `x264enc`, then `openh264enc`: the first that encodes 20 test frames | Hardware first, as on Windows. OpenH264 takes I420 frames only, so the frame's layout follows the encoder (`encoder.rs` `frame_format`). |
| Ctrl+C and a kill | `SIGINT` and `SIGTERM` finish the files, as Windows' console events do | A recording cut off without its end cannot be played. |

Measured in the release PC's WSL (Ubuntu 26.04, GStreamer 1.28.2, an RTX 4060), every stream to a file or to a listener on 127.0.0.1:

| What | Result |
|---|---|
| The real projector window, a video looping under lyrics, on a virtual X11 screen, to a file | 14.6 s of H.264 1280x720 at 30 a second with an AAC track, no frame dropped, encoder `nvh264enc`; a frame taken from the file shows the lyrics over the video. |
| The same with `--encoder openh264` and `--encoder nvenc`, and the whole first monitor | Each a file that plays. |
| SRT and RTMP | 10.7 s arrived at a GStreamer SRT listener and 10.8 s at ffmpeg's RTMP listener, both H.264 and AAC. |
| What the computer plays (the monitor input) with a tone playing | The recording's sound reads -13.5 dB; a recording with no sound reads -91 dB. |
| The stop file, and a kill | Stopped within a second each time, and the file plays. |
| Wayland: a sway desktop with no screen, PipeWire and the portal with its wlroots back end set to share the one output | The program asked the portal, went live, and a frame from the file is the desktop's picture; the token was kept; a second start went live the same way; with WSL's unanswering X display beside it the program waited three seconds and went to the portal. |
| The operator | Rendered without a window, the Stream dialog offers the address, the key, the recording and Start; with the program taken away it says this copy came without it. |
| Start pressed in the published 0.1.25 | The downloaded package's operator in a real window on a virtual X11 screen, its own projector program beside it: the recording switch and Start were clicked, the dialog read "On air for 0:09", the streaming program ran with `--window "Artosia projector" --stop-file … --record …`, Stop ended it, and the recording is 15.8 s of H.264 with AAC. |
| The default microphone on PipeWire, as a current Ubuntu desktop has it (found after 0.1.25, fixed in 0.1.26) | A sound server of our own (PipeWire 1.6 with its PulseAudio service) with an input set as the default and a tone on it. 0.1.25 found no input there and streamed silence: with `gstreamer1.0-pipewire` installed GStreamer lists each microphone a second time through `pipewiresrc`, which has no `device` to read, and asking it for one ended the listing. Now only the inputs `pulsesrc` opens are kept, and the default is the one PipeWire's own list flags: the microphone is listed once as the default and its recording reads -13.6 dB with no frame dropped, from the build and again from the published 0.1.26 package. |
| The published releases | The live feed offered 0.1.25 to a 0.1.24, and then 0.1.26 to a 0.1.25, on Linux, on Windows and on a Mac, each with a good signature. |
| Tests | `cargo test -p artosia-stream`: 33 pass on Linux, on the Mac and on Windows. |

Streaming needs these packages on Debian and Ubuntu, each found with `dpkg -S` on the plugin's file: `gstreamer1.0-plugins-base`, `-good` (`ximagesrc`, `pulsesrc`, `flvmux`, `mp4mux`), `-bad` (`rtmp2sink`, `srtsink`, `mpegtsmux`, `nvh264enc`, `vah264enc`, `openh264enc`, `voaacenc`) and `gstreamer1.0-libav`; `gstreamer1.0-pipewire` under Wayland; `gstreamer1.0-plugins-ugly` adds `x264enc`.

## Not checked

- Streaming from a real desktop: GNOME's and KDE's own sharing dialog (both portal back ends name `org.freedesktop.impl.portal.ScreenCast` in their sources, `xdg-desktop-portal-gnome`'s `data/meson.build` and `xdg-desktop-portal-kde`'s `data/kde.portal`, read 2026-11-13; neither desktop was run), the projector's own window through the portal (on the test desktop, which draws in software only, the projector's window presented no frame, so the shared picture was a wallpaper), a physical microphone (WSL has none that opens; the one recorded was made on a sound server of our own), `vah264enc` on Intel or AMD graphics, `x264enc`, and a real service.
- GNOME and KDE themselves: that mutter and KWin put the projector's X11 window on the second screen as an Xorg desktop does was read in mutter's source and not seen, and neither desktop was run.
- A Linux desktop outside WSL: Wayland with a real compositor, a second screen for the projector, full-screen, the scale of a HiDPI screen.
- Hardware decode through the `va` plugin on Intel or AMD graphics (only NVIDIA's decoder was there), and the software decoder (`avdec_h264`).
- Cameras, the microphone and speech recognition at run time, the phone remote from a real phone, EasiSlides import.
- A computer without GStreamer: `artosia-output --probe` and the fall-back to `artosia-output-plain` were built and compiled, and never run on such a machine.
- `install.sh` and the menu entry on a real desktop.
- Package names on Fedora, Arch and others: the README gives Debian's and Ubuntu's and points at GStreamer's own page.
- The Linux step of `tools/publish.sh` before release 0.1.22, which is its first run.
