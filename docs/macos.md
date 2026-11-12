# Artosia on a Mac

The Mac edition is one app in a disk image, for Macs with Apple silicon: `Artosia-macos-aarch64.dmg`. It costs nothing to make and nothing is paid to Apple, so macOS refuses its first open until the person presses Open Anyway. It was built and checked on one Mac over SSH, with no window opened on that Mac's screen for anyone to look at: read "Not checked" before trusting it.

## What a person gets

| In the disk image | What it is |
|---|---|
| `Artosia.app` | The operator (`Contents/MacOS/artosia`), the projector program built with video (`Contents/MacOS/artosia-output`) and the part of GStreamer they need (`Contents/Resources/gstreamer`, 63 MB). |
| `Applications` | A shortcut to the Applications folder, to drag the app onto. |
| `Read me first.txt` | The first open, where the library is kept, how updates work, how to remove. |

The image is 55 MB and the app 123 MB in 85 files. The library (songs, Bibles, settings) is in `~/Library/Application Support`, where Rune's `AppDirs` puts it, and stays when the app is deleted.

## Why this shape

| Choice | Taken or left | Why |
|---|---|---|
| One `.app` with GStreamer inside | Taken | A Mac has no GStreamer, and Homebrew's must not be mixed with the official one (GStreamer's page on building for macOS). The official runtime ran from a folder of our own with Apple's hardware decoder (`vtdec`), so a trimmed copy travels in the app. |
| GStreamer under `Contents/Resources/gstreamer`, laid out as its framework is (`lib`, `lib/gstreamer-1.0`, `libexec/gstreamer-1.0`) | Taken | Every library and plugin finds the others by the run paths it was built with (`@loader_path/..`, `@loader_path/../lib`), so no library's name is rewritten. The projector program gets one run path more: `@executable_path/../Resources/gstreamer/lib`. Measured on macOS 27: `codesign --verify --deep --strict` accepts code in folders below `Resources`. |
| Ad-hoc signature, no notarization | Taken | Apple silicon runs no unsigned code, and an ad-hoc signature is enough for that. Notarization needs a Developer ID, which needs the paid programme ($99 a year); the owner pays for none. The cost is the first open: refused, then System Settings, Privacy & Security, Open Anyway. |
| A `.dmg` made by `hdiutil` | Taken | It is on every Mac, and it is what Zed, Lapce, Alacritty and FreeShow ship. |
| Artosia's own updater, file by file | Taken | See "Updates" below: measured, an app keeps starting while its files are swapped and verifies again as the new release. Zed and Lapce replace the whole app; ours downloads only what changed, and GStreamer's 60 MB does not change between releases. |
| A second projector program without video | Left | Windows and Linux carry one for a computer with no GStreamer. The Mac app has its own, so the program with video always starts. |
| The streaming program | Left | It takes the picture, the sound and the encoder from Windows' own parts of GStreamer; see "Not built". |
| Intel Macs | Left | One architecture halves the app, and the owner has no Intel Mac to try one on. The GStreamer packages hold both, so `--thin x86_64` and a second build would make one. |
| Homebrew cask | Left | The official tap takes no app that fails Gatekeeper. |

The research behind the table is in `~/Shared/research/artosia-mac-linux/README.md` (2026-11-12), outside the repository.

## What had to change in the code

Nothing to make it compile: the first `cargo build` on macOS ended with no error, because every Windows-only path was already behind `cfg(windows)` and Linux had gone the same way the day before.

1. **The Mac says its own languages and country** (`crates/artosia-locale`). A program opened from the Finder gets none of `LANG`, `LANGUAGE`, `LC_ALL` or `LC_MESSAGES` (`launchctl getenv LANG` is empty), so the first run would have opened in English with no country. `defaults read -g AppleLanguages` prints the person's list and `AppleLocale` their formats, with `@rg=idzzzz` when the region they chose is another country than the format's: that region is the one offered.
2. **The updater treats `Artosia.app/Contents` as the install folder** (`crates/artosia-update/src/apply.rs`, `crates/artosia/src/update.rs`). See "Updates".
3. **The operator finds the app's GStreamer** (`crates/artosia/src/link.rs` `gstreamer_root`): `Contents/Resources/gstreamer`, told by its `lib/gstreamer-1.0` folder, and the projector program is handed the same plugin folder and plugin cache variables as on Windows. Whether the program with video can start is asked of the program itself (`--probe`), as on Linux.
4. **Key caps are drawn as a Mac draws them** (`crates/artosia/src/ui/widgets.rs` `printed_key`). Rune reports Command as the primary modifier wherever a shortcut says Ctrl, so the cap is ⌘; with Tab it is ⌃, because Command and Tab belongs to macOS. Option, Shift, Return and Delete are ⌥ ⇧ ⏎ ⌫, the marks of Apple's own list of keys (support.apple.com/en-us/102650, read 2026-11-13, which also gives Command-Tab as the switch between programs and Control-Up Arrow as Mission Control), and a Windows keyboard's Delete is drawn Fn ⌫: that page gives Fn-Delete as forward delete on a keyboard with no key for it.
5. **The GStreamer subset tool knows a Mac** (`tools/gstreamer-subset.py`): its file names, `otool -L` for what a file loads, `--thin arm64`, and a table of libraries a plugin opens by name at run time (the HTTP source opens `libsoup-3.0.0.dylib` itself, which its list of imports does not show).
6. **The sentences macOS shows when it asks for the camera, the microphone or the local network** are in `tools/macos/Info.plist` in English and in `tools/macos/strings/<code>.json` for the nine other languages, each written from `brief.json` by its own writer; `tools/macos/info-strings.py` puts them into the app as `<code>.lproj/InfoPlist.strings`.
7. **The package and the release step**: `tools/package-macos.sh`, `tools/macos/`, and a step in `tools/publish.sh`.

## The Mac's keys for the four switches

No text, Black, Logo and Live are on F1, F2, F3 and F6. By default the top row of an Apple keyboard controls other things, and a function key needs Fn held or the setting "Use F1, F2, etc. keys as standard function keys" (support.apple.com/en-us/102439, read 2026-11-13). The owner's direction (2026-11-13): use the keys that are usual on a Mac. Each program's own shortcut page, read that day:

| Program | Its keys for these switches on a Mac | And on Windows |
|---|---|---|
| Proclaim (support.proclaim.logos.com, "Keyboard Shortcuts") | Blank ⌘B, Logo ⌘L, No Text ⌘T | F10, F11, F8 |
| ProPresenter 7 (support.renewedvision.com, "Keyboard Shortcuts in ProPresenter") | Clear All F1 to Clear Announcements F7, as on Windows; output screens on and off ⌘1 | The same F keys; Control 1 |
| FreeShow (freeshow.app/docs/shortcuts) | Clear keys F1 to F4, as on Windows; output screen on and off ⌘O | The same; Control O |
| PowerPoint for Mac (support.microsoft.com) | Black: B or the full stop | The same |
| Keynote (support.apple.com/guide/keynote) | Black: B | |
| OpenLP (`openlp/core/common/settings.py`) | Blank: the full stop | The same |

So a Mac gets Command keys beside the F keys, which still work there: **⌘T No text, ⌘B Black, ⌘L Logo** as Proclaim has them for the same three switches, and **⌘O Live** as FreeShow switches its output (ProPresenter's ⌘1 is taken here: Command and a digit puts Preview's ring on a section). The tiles print the Mac's key and the keys sheet prints both (`ui/widgets.rs` `switch_key`, `main.rs` `mac_switch`). Windows and Linux keep the F keys alone, as Proclaim does on Windows. Checked on the Mac by a scripted headless run: Command B lights Black, Command O switches Live off, Command T lights No text. A real keyboard was not pressed.

## Updates

A Mac app's signature seals every file inside it. Measured on macOS 27 with a small faceless test app, ad-hoc signed and not quarantined (`target/port/mac-8-bundle-facts.sh` in the working tree of 2026-11-13):

| What was done to the app | `codesign --verify --deep --strict` | Does it still start |
|---|---|---|
| Nothing | passes | yes |
| An empty folder added inside `Contents` | passes | yes |
| A file added inside `Contents` | "a sealed resource is missing or invalid" | yes |
| A resource swapped for another by rename | fails | yes |
| The main program swapped by rename, the old one kept beside it | fails | yes, as the new program |
| Every file of the next release swapped in, leftovers removed | passes | yes, and it is that release file for file |

So Artosia's updater works as it is, a file at a time by rename, which is also what Apple's page "Updating Mac Software" asks for (a new file, never a change in place). Three things differ for a Mac app:

- The install folder is `Contents`, not the folder the program is in, and a release lists every file of it, the signature's own `_CodeSignature/CodeResources` and `Info.plist` among them. A copy that has swapped in a release's files is that release, seal and all.
- Downloads, the journal and the replaced files wait in a hidden folder beside the app (`.Artosia-update`, on the same volume so renames work), never inside it. The app holds a release's files and nothing else before, during and after an update; two tests hold it to that (`a_mac_app_holds_a_releases_files_and_nothing_else_through_an_update`, `a_mac_app_rolled_back_is_the_old_release_again_with_nothing_added`).
- A copy that cannot make that folder says nothing and does not update: the app opened from its disk image, or from Downloads before it was moved (macOS then runs it from a read-only place).

A download asks the app's own `Contents` which files it already has (`install_dir_of`), so only the changed ones travel: between two builds of the same GStreamer that is the operator program and `Info.plist`, because signing the same library twice gives the same bytes.

A file a later release no longer ships stays in the app, as it does on Windows; the seal then names it as added. No release has dropped a file yet. With the projector switched off the mark of a fresh update (`just-updated`) stays in the folder beside the app until a start that uses the projector, as it stays beside the program on Windows.

## Preparing a Mac to build it

Done once, as `target/port/mac-3-gstreamer.sh` did it on 2026-11-13. Nothing asks for a password and nothing is installed into `/Library`.

1. Xcode's command line tools, and Rust through rustup; the repository's `rust-toolchain.toml` picks the version.
2. GStreamer's official packages of the version the Windows edition ships (1.28.8), from `https://gstreamer.freedesktop.org/data/pkg/osx/1.28.8/`: `gstreamer-1.0-1.28.8-universal.pkg` (157 MB) and `gstreamer-1.0-devel-1.28.8-universal.pkg` (809 MB), each checked against the `.sha256sum` file beside it.
3. Unpack both with `pkgutil --expand-full` and join every `Payload` folder into one tree with `ditto`: `~/artosia-gst/root`, 4.0 GB, or any folder named by `ARTOSIA_GSTREAMER_ROOT`. Its `.pc` files find their own folder (`prefix=${pcfiledir}/../..`), and its own `bin/pkg-config` answers with that folder's paths.

The host gave each connection about 170 kB a second and dropped one HTTP/2 stream a quarter of the way in; the script resumes over HTTP/1.1 and can fetch the rest as several ranges side by side.

## How a release carries it

`tools/publish.sh` reaches the Mac through WSL's `ssh mac`, as `tools/publish-site.sh` does:

1. On the Mac: `git pull`, then `tools/package-macos.sh <version>`, which builds the two programs, puts the app together, signs it inside out (every library, plugin and helper, the projector program, then the app), verifies the seal and makes the disk image.
2. The app's `Contents` and the image come back to WSL, and `artosia-release build --platform macos-aarch64` makes and signs the update list there. The signing key stays on the release PC.
3. The image, `manifest-macos-aarch64.json`, its signature and the files (`macos-aarch64__...`) are uploaded beside the other systems' ([linux.md](linux.md) has the naming).

When the Mac cannot be reached or its build fails, the release carries no Mac files and says so, and the homepage, which shows a system only when the newest release holds its file, shows no Mac row ([site.md](site.md)).

## Checked on 2026-11-13

On a MacBook with Apple silicon, macOS 27.0.1, 16 GB, busy with other work throughout (load 9 to 130), Rust 1.99.1, GStreamer 1.28.8. Every check ran over SSH.

| What | Result |
|---|---|
| `cargo build` | No error on the first try. |
| Tests | Operator 227 and 4 ignored on the first build, core 181, updater 15, slides 18, remote 21, office 26, warp 13, lyrics 11, locale 5; the updater's and the operator's update tests again after each change to them. Two operator tests that hold a time limit failed once under a load of 60 and pass alone. |
| The Mac's language and country | `["en-GB", "id-ID", "ja-ID"]`, region `ID`: what System Settings shows. |
| Headless render | The operator in light and in dark Indonesian, the keys sheet, and the projector's lyrics slide (`target/port/macos-check/`). |
| GStreamer from our own folder | `vtdec`, `vtdec_hw`, `avfvideosrc`, `playbin3`, `qtdemux`, `h264parse`, `avdec_h264`, `matroskademux`, `vp9dec`, `dav1ddec`, `rtspsrc`, `srtsrc` and `souphttpsrc` all load from the thinned subset, through its own plugin scanner. |
| The package | `Artosia-macos-aarch64.dmg`, 55 MB; the app 123 MB in 85 files with no symbolic link: `artosia` 37 MB, `artosia-output` 25 MB, GStreamer 63 MB (27 plugins, 42 libraries, the plugin scanner), nine `.lproj` folders. The first package tried was built without speech recognition; with it the operator is 45 MB and the app gains no file. |
| The seal | `codesign --verify --deep --strict` passes on the app, and on the app inside the mounted image. `spctl --assess` says "rejected", which is the first-open refusal a downloaded copy meets. |
| Nothing from outside the app | With the unpacked GStreamer moved away, `artosia-output --probe` ends with 0 from the app and from the mounted image, and the operator and the projector render from the app. The projector program's only run path is `@executable_path/../Resources/gstreamer/lib`. |
| A video with no window | `cargo test -p artosia-video a_video_loops` on the app's own GStreamer alone: decoder `vtdec_hw` (Apple's hardware decoder), 867 frames, round to its start twice in 28.9 s. This settles what the research left open: `vtdec` loads from a moved, ad-hoc signed copy on macOS 27. |
| Self-update | Version 0.1.22, copied into a scratch folder and opened hidden through LaunchServices on an empty library with the projector off, fetched 0.1.23 from a feed on 127.0.0.1 (2 files, the ones that differ, of 85), kept its seal while the download waited beside it, and at the next start came up as 0.1.23: the app verified and was release 0.1.23 file for file (`diff -r`), and the replaced files were gone once the new version had started. Both versions trusted a key made for the test. |
| The update list made on the release PC | The app's `Contents` brought to WSL by rsync gives the same list as the one made on the Mac: 76 files with the same hashes, sizes and executable marks. |
| Speech recognition | `cargo build --release -p artosia --features speech` builds with Homebrew's CMake and Xcode's libclang, links only the system's frameworks, and the listening crate's tests pass (36; five that need a model are ignored). The package script builds with it when both tools are there. Hearing a real room was not tried. |
| The published 0.1.23 | `cargo run -p artosia-update --example published` on the Mac, on Linux and on Windows reads the live feed as an installed copy does: each is offered 0.1.23 with a good signature. The published image, fetched on the Mac with `curl` (so with no quarantine mark), is 58,460,176 bytes and holds the app, the Applications shortcut and the note; the app is 0.1.23, its seal verifies, and with the unpacked GStreamer moved away its projector program answers `--probe` and the operator renders. The homepage shows the Mac row and a browser that answers as a Mac is offered the image, whose link answers 200. |
| Permission sentences | `plutil -p` reads each language's `InfoPlist.strings`; the dialogs themselves were not seen. |

## Not built

- **Streaming.** As on Linux: the streaming program needs a screen capture, a sound source and an encoder of the Mac's own before the Stream dialog offers it, and the app does not carry the program.
- **A Mac key for F4 and F5** (what the listener heard, and the announcements, into Preview). No program read has a key for either, so they stay F keys, which need Fn held on a Mac as it comes.

## Not checked

- **A window.** No picture of a real window could be taken: a screenshot over SSH needs a screen-recording permission that is the owner's to give, and a window opened there lands on the owner's screen. Keys, the pointer, a second screen for the projector, full screen, the Dock icon and the menu bar are all unseen.
- **The first open of a downloaded copy.** A browser marks the image as quarantined and Gatekeeper then refuses the app; the steps on the homepage and in the README are Apple's own page (support.apple.com/en-us/102445), not a picture of this app on this Mac. Whether an update of an app opened that way asks again is not known.
- A video on a real projector window, cameras, the microphone, the phone remote, EasiSlides import.
- macOS older than 27, an Intel Mac, a Mac of a standard user who cannot write to Applications.
