# Artosia-setup.exe

A small per-user setup for Windows. It carries no program files: it fetches the signed release with `artosia-update`, so it is a download of about 1.7 MB and the same file works for every release. Plan and sources: [plans/artosia/setup.md](plans/artosia/setup.md). Code: `crates/artosia-setup`.

## What it does

`Artosia-setup.exe` with no argument installs:

1. If `artosia.exe` is already in the install folder, nothing is downloaded: the shortcuts and the uninstall entry are written again, then Artosia starts.
2. Otherwise it asks the feed for the newest release (`artosia_update::check(feed, "0.0.0")`), downloads into `<install>\update` with a progress bar (`download`, which resumes a cut-off download and skips files already there), swaps the files in (`apply_on_start`), keeps a copy of itself as `<install>\Artosia-setup.exe`, writes `Artosia.lnk` in the Start menu and on the desktop, writes the uninstall entry last, starts `artosia.exe` and exits.

Flags: `--silent` (no dialog), `--no-start`, `--uninstall`. The feed is `artosia_update::RELEASES` unless `ARTOSIA_UPDATE_FEED` says otherwise; the app reads the same constant (`artosia_update::feed_address`).

## Where things go

| what | where |
|---|---|
| program files | `%LOCALAPPDATA%\Programs\Artosia` (`FOLDERID_LocalAppData`, so a moved profile is followed; electron-builder's per-user place, `eb_multiUser.nsh`). User-writable, because Artosia swaps its own files there. Not `%LOCALAPPDATA%\Artosia`: rune keeps its state and log folders in `%LOCALAPPDATA%\artosia`, the same folder on Windows, and an uninstall empties the install folder |
| setup copy | `%LOCALAPPDATA%\Programs\Artosia\Artosia-setup.exe`, not part of any release |
| Start menu | `FOLDERID_Programs\Artosia.lnk` (the folder is created if missing) |
| desktop | `FOLDERID_Desktop\Artosia.lnk` (follows OneDrive) |
| uninstall entry | `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Artosia`: strings `DisplayName`, `DisplayVersion`, `Publisher`, `DisplayIcon`, `InstallLocation`, `UninstallString` (`"<setup>" --uninstall`), `QuietUninstallString` (same plus `--silent`); DWORDs `NoModify` 1, `NoRepair` 1, `EstimatedSize` in KB |
| the church's data | `%APPDATA%\artosia` (library, bibles, recordings, settings), or `ARTOSIA_DATA`. Never touched |
| failure cause | `%TEMP%\Artosia-setup.log` (one line, overwritten) |

The shortcuts have target, working folder and icon (the icon of `artosia.exe`). No AppUserModelID: the operator never sets one itself (`rune_notify::register` is not called in `crates/artosia`), and a shortcut carrying an id the program does not use would split its taskbar button.

`DisplayVersion` and `EstimatedSize` come from the release just installed (its manifest), never from the setup's own version.

### The version follows a self-update

One owner for the registry code: the setup. `Artosia-setup.exe --refresh [version]` is silent. It rewrites every value of the entry from what is installed now (the version given, else the version resource of `artosia.exe`; the size by walking the folder) and does nothing, exit 0, when there is no entry or no program. `artosia.exe` runs `<install>\Artosia-setup.exe --refresh <its version>` once, from `update::confirm()` (`crates/artosia/src/update.rs`), on the first confirmed start of a new version: `just_updated()` (the marker the swap leaves) sets a flag, and `confirm()` reads it once. No process is waited for. A copy unzipped by hand has no `Artosia-setup.exe`, so nothing runs and nothing changes; a development build has no install folder at all. The file name is `artosia_update::SETUP_FILE`, shared by both. The self-update test installs 2.0.0, applies 3.0.0, runs the real exe with `--refresh 3.0.0` in the test root and checks the entry says 3.0.0; another test checks that with no entry it creates nothing.

## The test root

`ARTOSIA_SETUP_ROOT=<folder>` moves the install folder (`<folder>\Programs\Artosia`), both shortcut folders (`StartMenu\Programs`, `Desktop`), the data folder (`Data`) and the registry key (`Artosia-test-<hash of the folder>`). Under a test root Artosia is never started, whatever the flags say. Every run on a PC that holds a real library uses it. The end-to-end tests (`cargo test -p artosia-setup`) use one per test under the temp folder, with a non-ASCII name, and delete the folder and the key when they end.

## Why the setup copy survives updates

`apply.rs` swaps only paths a manifest lists (`swaps` come from `m.files`); `drop_unlisted` prunes only the staging `files` folder; `restore` only touches journaled paths. An unlisted `Artosia-setup.exe` in the install folder is therefore left alone by every update and every rollback, and the uninstall entry keeps working after many updates. It is a file of the install folder, not of a release. A test asserts the newest real manifest does not list that name; a release must never add it.

First start confirms the install: after `apply_on_start` the journal is in `Trial` with 0 starts. The app's own `apply_on_start` counts the start and `confirm_started` clears the journal. The end-to-end test runs exactly those two calls and checks `update/state.json` is gone. (Three starts without a confirm roll the files back, which on a fresh install would delete the program; the app confirms on every start, so only a program that cannot open a window at all is rolled back.)

## How uninstall works

`--uninstall` (or the entry in Settings > Installed apps):

1. A confirmation with focus on Cancel.
2. If any exe in the install folder is running (it cannot be opened for writing: sharing violation), a dialog asks the person to close Artosia and its projector window and waits, checking each second. Nothing is ever killed. `--silent` with a program running exits with code 5.
3. Both shortcuts, then every file under the install folder except the running setup exe and anything that is, holds or sits inside a data folder (`%APPDATA%\artosia`, `ARTOSIA_DATA`). A refused delete is tried again four times 200 ms apart (antivirus holds a file just after scanning it; Velopack #1129). If any file stays, the entry stays and the person can run it again.
4. The registry key, last.
5. The setup exe cannot delete itself while running, so a hidden `cmd /C ping -n 4 127.0.0.1 >nul & del /f /q "<exe>" & rmdir "<install>"` does it after exit (Velopack's method, `self_delete.rs`). `rmdir` has no `/s`, so a folder still holding data is never removed. Proved on this PC with the release build: the install folder was gone about six seconds after exit. rustup's delete-on-close copy was not tried.

Uninstalling leaves the Windows Firewall rule `Artosia phone remote` behind, and `artosia-setup` is not changed to remove it: Artosia adds that rule the first time the phone remote is switched on (`crates/artosia/src/firewall.rs`), the rule names the full path of `artosia.exe`, and the setup keeps that path the same file across releases, so a later install finds the rule still valid. A person can delete it in Windows Defender Firewall, and switching Phone off and on adds it again.

The tests uninstall with a byte-for-byte comparison of the data folder, with a file held open (uninstall stops, entry kept, second run finishes), and with a data folder inside the install folder.

The install folder is not rune's state and log folder (`%LOCALAPPDATA%\artosia`): a test on the real places checks that neither is the other or inside the other, so an uninstall can never empty what the UI library writes.

## What a person sees

Only the task dialog (`TaskDialogIndirect`), no window of ours: progress (bar, Cancel), failure (Try again, Close), confirmation (Remove, Cancel), close-first (Cancel). The exe carries `assets/brand/artosia.ico` as its icon resource (id 1, by `build.rs`), so the file in Downloads shows it. Every dialog's title bar and taskbar button get it (`WM_SETICON` on creation), and the progress, confirmation and close-first dialogs use it as their main icon (`TDF_USE_HICON_MAIN`). A failure dialog keeps the system's error icon as its main icon, because there the icon says what the dialog is. The manifest (`crates/artosia-setup/setup.manifest`) asks for common controls 6 (without it `TaskDialogIndirect` is not found when the file starts), per-monitor DPI, long paths and UTF-8. Strings: `src/strings.rs` (English), `strings-id.json` (Indonesian, empty: falls back to English), briefs in `strings-brief.json`; the Windows display language picks (`GetUserDefaultUILanguage`, primary language Indonesian = 0x21). A test holds both tables, the briefs and the house rules (no dash, middle dot, emoji or exclamation mark; sentence case) together; it passes with an empty Indonesian table and fails once a table is partial.

Cancel while downloading closes the dialog and exits; downloaded parts stay in `<install>\update` and the next run resumes them (the updater has no cancel call, so the worker is simply left to the process exit). Cancel is greyed out once the swap starts.

## The traps from the research

| trap | what is done |
|---|---|
| antivirus flags small unsigned setups (Squirrel #1204, rustup #4238) | Not solved. Artosia stays unsigned. The same file is kept across releases so Windows' trust by download count carries over (see "One file across releases"). No embedded program, no packer, a plain Rust exe with its manifest |
| antivirus slows folder deletes (Velopack #1129) | each delete is retried four times; the entry stays when a file stays |
| Start menu folder may not exist (Velopack #328) | `create_dir_all` before saving the link; tested with a root that has no Start menu folder |
| paths over 260 characters (Squirrel #304) | std puts the verbatim prefix on long paths; the manifest is `longPathAware`; a test installs under a root where files pass 260 characters. A shortcut target is limited to 259 by the shell, so an install folder that long fails with "could not add its shortcuts" and leaves no entry (tested) |
| non-ASCII user name (rustup #5035) | every path is `OsStr` and passed as UTF-16, never ANSI; every end-to-end root has `é` and `日` in its name |
| OneDrive-redirected Desktop | known folder API, not `%USERPROFILE%\Desktop` (not tried on a redirected desktop) |

## One file across releases

Nothing release-specific is built into the setup: no Artosia version, build date or file list. It installs whatever the feed's newest release is. Its own version is its crate version (`crates/artosia-setup/Cargo.toml`), changed only when its code changes.

The bytes of `Artosia-setup.exe` depend on these inputs and no others:

- `crates/artosia-setup/**` (source, `build.rs`, `setup.manifest`, `strings-id.json`, `assets/brand/artosia.ico`);
- `crates/artosia-update/**` and the crates it pulls in (`Cargo.lock`), including the public key in `update-public-key.txt` compiled into the updater;
- `rust-toolchain.toml` (the compiler);
- the `[profile.setup]` and `[profile.release.package.artosia-setup]` sections of the root `Cargo.toml`;
- the path of the checkout (Rust embeds source paths in the binary).

A release that changed none of them can reuse the previous `Artosia-setup.exe`. The link is deterministic (`/Brepro`): building twice from the same tree gives the same SHA-256 (checked again with the icon resource and the dialog icons in: the icon is a fixed resource compiled from `artosia.ico`, so it changes the bytes only when `artosia.ico` changes, and it adds `assets/brand/artosia.ico` to the list of inputs above). Changing the signing key or the feed address changes the bytes, so the file is then a new file.

## Size

`cargo build --profile setup -p artosia-setup` is the build to ship. Measured on this PC (toolchain 1.99.1):

| build | bytes |
|---|---|
| `--release` (with the package override below) | 2,894,848 |
| `--profile setup` (everything built for size, lto, abort) | 1,681,920 |

Cargo allows a per-package override of `opt-level`, `codegen-units`, `strip`, `debug`, `debug-assertions`, `overflow-checks`, `incremental` and `split-debuginfo`, and refuses `lto`, `panic` and `rpath` there ("may not be specified in a `package` profile", checked by trial). Those three belong to the whole link, and a `[profile.release]` change would hit every other program, so the size build is its own profile, `setup`, which inherits `release`. The package override in `[profile.release.package.artosia-setup]` only shrinks the setup's own few hundred lines; the dependencies (ureq, rustls, ring, windows) stay at opt-level 3 there, which is why the `setup` profile is the real saving.

## Copied from whom

All four are MIT (rustup MIT or Apache-2.0); this project is GPL-3.0-or-later. Saved with permalinks in `~/Shared/research/artosia-setup/`. Nothing was pasted whole: what is taken is a shape, written again in this code, and each file says so.

- Velopack (`registry.rs`, `shortcuts.rs`, `known_path.rs`, `self_delete.rs`, `install.rs`, `Cargo.toml`): MIT, Copyright (c) Velopack Ltd and Caelan Sayler. The value set and the DWORD note, `IShellLinkW` plus `IPersistFile`, the known-folder calls, the delayed `cmd` delete, the install order, the size profile.
- Squirrel.Windows (`InstallHelpers.cs`, `ApplyReleases.cs`): MIT, Copyright (c) 2012 GitHub, Inc. The value names, and the Start menu and desktop targets.
- rustup (`self_update/windows.rs`): MIT OR Apache-2.0. Which values a minimal entry needs. Its delete-on-close copy was read and not used.
- electron-builder (`installer.nsh`, `uninstaller.nsh`, `multiUser.nsh`, `webPackage.nsh`): MIT, Copyright (c) 2015 Loopline Systems. The `QuietUninstallString` pair, the per-user folder, "download into a temporary place and leave the install folder untouched until it is complete".

## Never tried

- Windows 10, 32-bit or ARM Windows; only Windows 11 on one PC.
- A real install on the real folders, the real desktop and the real Start menu: every run used a test root. The first real run is the owner's.
- A OneDrive-redirected Desktop, a profile with non-ASCII characters in the real `%LOCALAPPDATA%` (the roots in tests are non-ASCII, the real folders are not).
- Microsoft Defender SmartScreen and antivirus reactions to the downloaded file (mark of the web); no firewall prompt appeared on the loopback runs.
- Indonesian display language (no Indonesian text exists yet), a display language other than English.
- Starting `artosia.exe` after install (forbidden here; the first start is simulated by its two updater calls).
- High-DPI scaling (the manifest asks for per-monitor DPI; only 100 % was looked at).
- The updater's own cancel: there is none, see above.
- Timing of the dialog's timer path: five ticks a second, each a few string builds and up to three window messages to its own thread; not measured, it has no per-frame work.

## Why it is unsigned (owner, 2026-11-12)

Windows calls the setup an unknown publisher. The routes to a known one were read from Microsoft's
and the sellers' own pages on 2026-11-12 (saved in `~/Shared/research/artosia-code-signing/`):

| Route | Why not |
|---|---|
| Microsoft's own signing service | Individuals must live in the United States or Canada |
| An OV or EV certificate | $150 to $400 and more a year, and Windows still warns until trust builds; EV no longer skips the warning |
| Certum's open-source certificate | €69; the owner's name would show, and Windows still warns until trust builds |
| SignPath Foundation | Free, but every release is built on GitHub and approved by hand, and the publisher shown is SignPath |
| Microsoft Store | No warning at all, but the Store does the updating in place of Artosia's own updater |

The owner asked how ArtCraft (`storytold/artcraft`) does it. It does not: its Windows workflow
passes no certificate and the setup file of its release 0.41.1 has an empty certificate table.
The owner chose the same for Artosia.

Microsoft ties the trust of an unsigned file to that exact file: "When a file is not signed,
SmartScreen reputation must build for each new version of your files, starting with zero
reputation." So the setup is never rebuilt for a release that did not change it.
`tools/publish.sh` fetches the last release's `Artosia-setup.exe` and uploads it again
unchanged; it also builds a fresh one and says whether the bytes differ. `ARTOSIA_NEW_SETUP=1`
ships the fresh one, and that file starts from no trust.

## The download warning: what others ship (research, 2026-11-12)

The owner downloaded the setup with Edge and was told it "isn't commonly downloaded". The
signer of each program's newest Windows installer was read from the file itself (saved in
`~/Shared/research/artosia-download-warning/`):

| Program | Installer |
|---|---|
| OpenLP 3.1.7, Quelea 2024.0.1, ArtCraft 0.41.1 | Unsigned |
| HandBrake 1.11.2 | Certum's open-source certificate, in one person's name |
| FreeShow 1.6.7, Zed 1.28.1, Obsidian 1.15.0 | Microsoft's Artifact Signing (closed to Indonesia) |
| OBS 33.0.1, VLC 3.0.24, Krita, RustDesk, LibreOffice, Audacity | A company's certificate |

Microsoft's page on this warning says a signed file is still warned about "until reputation
accumulates", that the prompt stops "once the file hash has sufficient download history",
that this "can take several weeks and hundreds of clean installs", that there is no way to
send a file in for review, and that only a Store-installed app is never warned about.

ArtCraft is the case the owner pointed at: it stops warning although it is unsigned. Its
setup of version 0.41.0 was downloaded 819,197 times and that of 0.41.1 15,684 times in five
days (GitHub's own counts), which is the download history Microsoft's page describes. Its
tracker still holds the open issue 962, "Windows Defender blocks install: Investigate how we
can gain reputation / trust with Windows".

What follows for Artosia: a person is warned at the first download only, because every later
version arrives through the program's own updater and never through a browser. The warning
on that first download lasts until the one setup file has been downloaded often enough,
which is why the file is kept unchanged. The Microsoft Store is the channel with no warning;
the owner wants a Store edition once the developer account exists (2026-11-12).

