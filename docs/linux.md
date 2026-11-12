# Artosia on Linux

Decided and built on 2026-11-12, on the owner's order to build the macOS and Linux editions (board card PORT1). Do not relitigate the choices below without new evidence. The research behind them is saved in `~/Shared/research/artosia-mac-linux/` (its `README.md` holds the table of projects and every source).

## What a person gets

One file, `Artosia-linux-x86_64.tar.gz`, that unpacks to a folder `Artosia`:

| In the folder | What it is |
|---|---|
| `artosia` | The operator window. It starts the projector program from this same folder. |
| `artosia-output` | The projector and stage windows, built with video. It links the distribution's GStreamer. |
| `artosia-output-plain` | The same program built without video. The operator starts it when the one with video cannot start. |
| `artosia-stream` | The streaming program. It is packed and not offered: see "Not built". |
| `share/applications/artosia.desktop`, `share/icons/hicolor` | The menu entry and the icons (`assets/brand/icons/linux`). |
| `install.sh` | Writes the menu entry into `~/.local/share/applications` with this folder's full paths. Nothing needs an administrator. `--remove` takes it away. |
| `README.txt` | How to start, which GStreamer packages video needs, how updates work, how to remove. |

The library and the settings live apart from the folder, in `~/.local/share/artosia` (Rune's `dirs.rs`, `$XDG_DATA_HOME`).

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
2. **The operator asks the projector program whether it can start** (`link.rs` `video_can_start`, `artosia-output --probe`). On Windows the program with video needs GStreamer's folder. On Linux it links the system's GStreamer, and without those libraries the system's loader ends it before it runs; so the operator starts it once with `--probe`, which returns at once, and falls back to `artosia-output-plain` when that fails.
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

`tools/publish.sh <version> "<notes>"` on the Windows PC, after it has pushed the commit and uploaded the Windows files, runs in WSL:

```bash
cd ~/artosia && git pull --no-rebase -q && bash tools/package-linux.sh <version> --out <the Windows checkout>/dist/<version>/linux --key-file <the signing key> --notes-file <notes>
```

and uploads `Artosia-linux-x86_64.tar.gz` and everything in `dist/<version>/linux/feed` to the same release, then adds the Linux download to the release's notes. The key never leaves the PC: WSL reads the same file under `/mnt/c`. Without WSL, or when the Linux build fails, the release carries Windows only and the script says so. The homepage does not offer the Linux download yet (board card SITEOS1).

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

## Not built

- **Streaming.** The streaming program takes the projector's picture, the room's sound and the hardware
  encoder from Windows' own parts of GStreamer (`d3d11screencapturesrc`, `wasapi2src`, `mfh264enc`),
  so on Linux the Stream dialog says streaming is not available (`crates/artosia/src/stream.rs`
  `available`). It needs a screen capture, a sound source and an encoder of Linux's own before it is offered.

## Not checked

- A Linux desktop outside WSL: Wayland with a real compositor, a second screen for the projector, full-screen, the scale of a HiDPI screen.
- Hardware decode through the `va` plugin on Intel or AMD graphics (only NVIDIA's decoder was there), and the software decoder (`avdec_h264`).
- Cameras, the microphone and speech recognition at run time, the phone remote from a real phone, EasiSlides import.
- A computer without GStreamer: `artosia-output --probe` and the fall-back to `artosia-output-plain` were built and compiled, and never run on such a machine.
- `install.sh` and the menu entry on a real desktop.
- Package names on Fedora, Arch and others: the README gives Debian's and Ubuntu's and points at GStreamer's own page.
- The Linux step of `tools/publish.sh` before release 0.1.22, which is its first run.
