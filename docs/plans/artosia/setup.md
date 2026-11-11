# INSTALL1: Artosia-setup.exe (plan, written before the code)

A small per-user setup for Windows. It carries no program files: it fetches the signed release with `artosia-update` (check, download, apply_on_start) into an empty folder, writes two shortcuts and one uninstall entry, starts Artosia, and exits. The same exe, run with `--uninstall`, removes it all and leaves the church's data alone.

## SOURCES (saved 2026-11-12 in `~/Shared/research/artosia-setup/`, notes.md lists the commits)

| need | read | mechanism taken |
|---|---|---|
| uninstall entry | Velopack `registry.rs`, Squirrel `UpdateManager.InstallHelpers.cs`, electron-builder `installer.nsh`, rustup `windows.rs` | HKCU key, six strings, DWORD NoModify, NoRepair, EstimatedSize in KB (a QWORD is ignored), written last |
| shortcuts | Velopack `shortcuts.rs`, Squirrel `ApplyReleases.cs` | `IShellLinkW` + `IPersistFile` on a COM apartment thread: SetPath, SetWorkingDirectory, SetIconLocation, Save |
| known folders | Velopack `known_path.rs` | `SHGetKnownFolderPath` (follows a OneDrive-redirected Desktop) |
| self delete | Velopack `self_delete.rs`, rustup `windows.rs` (delete-on-close copy), electron-builder `uninstaller.nsh` | Velopack's delayed `cmd` (proved on this PC, see docs/setup.md) |
| order | Velopack `install.rs` 239-250 | files, shortcuts, uninstall key last |
| fetching | Squirrel `UpdateRunner.cpp` and Velopack `setup.rs` (both EMBED the package), electron-builder `webPackage.nsh` (downloads, resumes, cancel = exit) | we download, through the one downloader that exists |
| size | Velopack `Cargo.toml` release profile | opt-level z, lto, codegen-units 1 |

BASE: nothing general is made. The feed address and its override move into `artosia-update` (`RELEASES`, `feed_address`), read by the app and by the setup: one constant.

BOUNDARY: `artosia-setup` (lib) depends on `artosia-update` only. The lib owns `Places` (where things go) and takes a progress callback the caller owns. The bin is the composition root: it builds `Places` from the environment, takes `TrustedKeys::compiled()`, and draws the dialogs. Tests call the lib with test keys. No cycle: the updater never knows the setup.

## FLOW (pseudocode)

```
main(args):
  places = Places::resolve()            # the real folders, or the test root from ARTOSIA_SETUP_ROOT
  flags  = silent, no_start, uninstall
  if uninstall: uninstall_flow else install_flow

install_flow:
  loop:
    result = with_progress_dialog(worker: install(places, feed, keys, report))
    Ok        -> break
    Cancelled -> exit 0                   # downloaded parts stay, the updater resumes them
    Err(f)    -> match failure_dialog(f) { TryAgain -> continue, Close -> exit 1 }
  unless no_start: spawn <install>\artosia.exe

install(places, feed, keys, report):
  if exists(install/artosia.exe):
      version = file version of artosia.exe          # downloads nothing
  else:
      report(Checking)
      release = check(feed, "0.0.0")?                # None cannot happen for 0.0.0: NotPublished
      report(Downloading, 0, total)
      download(release, staging_dir(install), progress)?
      report(SettingUp)                              # Cancel is disabled from here
      apply_on_start(install)?                       # files in place, journal in Trial
      version = release.version
  copy own exe to <install>\Artosia-setup.exe        # unless it is that file already
  shortcuts: Programs\Artosia.lnk (create_dir_all first) and Desktop\Artosia.lnk
  on a shortcut error: remove both, return the error
  write uninstall entry LAST; on error remove shortcuts and entry
```

Where the setup exe lives: a file in the install folder that is not in the release manifest. `apply.rs` swaps only paths the manifest lists (`swaps` come from `m.files`), `drop_unlisted` only prunes the staging `files` folder, `restore` only touches journaled entries. So an unlisted `Artosia-setup.exe` survives any number of self-updates and the `UninstallString` keeps working. A test asserts the real manifest does not list that name.

First start confirms the install: after apply the journal is `Trial` with 0 starts. The app's `apply_on_start` counts a start and `confirm_started` clears the journal and removes `.artosia-old` files. A test runs exactly those two calls after setup and checks `update/state.json` is gone. (Three starts without a confirm roll the files back, and on a fresh install that would delete the program: the app's own `main` confirms, the setup never starts anything else.)

```
uninstall_flow:
  unless silent: confirm dialog, focus on Cancel; Cancel -> exit 0
  while running_programs(install) not empty: dialog "close Artosia" polls each second; Cancel -> exit 0
      # running = an exe in the install folder that cannot be opened for writing (a running image refuses it). Never kills.
  with_progress_dialog(worker):
      remove both shortcuts
      remove files: walk install; keep own exe and any path that is, contains or is inside a protected data folder
      remove the registry key
  if own exe is inside install: spawn hidden `cmd /C ping -n 3 127.0.0.1 >nul & del /f /q "<exe>" & rmdir "<install>"`
      # rmdir without /s: it can never take data
```

Protected folders: `%APPDATA%\artosia` (rune `AppDirs::new("artosia").data()`: library.db, bibles, recordings, settings) and `ARTOSIA_DATA` when set. The install folder is `%LOCALAPPDATA%\Programs\Artosia` (electron-builder's `eb_multiUser.nsh` place), never `%LOCALAPPDATA%\Artosia`: rune's state and logs folder is `%LOCALAPPDATA%\artosia`, the same folder on a case-insensitive disk, and an uninstall empties the install folder. A test checks the real places do not overlap. The protect list also covers an `ARTOSIA_DATA` set inside the install folder.

Icon and refresh (added after the first round): the exe carries `assets/brand/artosia.ico` as resource 1; every dialog sets it on its title bar, and all but the failure dialog use it as main icon. `--refresh [version]` rewrites the uninstall entry silently from what is installed (nothing when there is no entry); `artosia.exe` runs the kept copy with it once after the first confirmed start of a new version (`update::confirm`, flag set by `just_updated`), only when `Artosia-setup.exe` exists beside it.

## TEST ROOT

`ARTOSIA_SETUP_ROOT=<dir>`: install = `<dir>\Programs\Artosia`, Start menu = `<dir>\StartMenu\Programs`, Desktop = `<dir>\Desktop`, data = `<dir>\Data`, registry key name `Artosia-test-<hash of dir>`. `--no-start` never starts the app. `--silent` shows no dialog (the tests use it so no window pops up).

## SCREEN, TREE, KIT, PLACE

Windows `TaskDialogIndirect`, no custom window, no kit part. Four dialogs: progress (bar, Cancel), failure (Try again, Close), remove confirmation (Remove, Cancel; Cancel focused), close-Artosia wait (Cancel). The exe manifest asks for common controls 6 (without it `TaskDialogIndirect` is not found at load), long paths, UTF-8 and asInvoker. Windows only: other platforms get an empty `main`.

## RULES, STATE, DATA

Progress is two atomics (done, total) and a phase atomic, written by the worker and read by the dialog's 100 ms timer on the UI thread: no store, nothing derived is stored. Network and disk go through `artosia-update` only.

## ACTION blocks

- press Cancel while downloading: flag set, dialog closes, process exits; `.part` files stay and resume. Cancel is disabled while applying (a half swap would be rolled back by the journal anyway).
- press Try again: a new worker over the same staging folder; finished files are kept.
- press Remove with the app open: the wait dialog until it is closed.
- incoming data: the progress timer only.

## WORDS

One table per language: `src/strings.rs` (English) and `strings-id.json` (Indonesian, left empty: falls back to English; a writer fills it from `strings-brief.json`). Language from `GetUserDefaultUILanguage`. No em-dash, en-dash, middle dot, emoji or exclamation mark; sentence case; a failure says what happened and what to do in one or two short sentences. A test holds all of it.

## TIMING, SIZE, EDGES

One fixed-size native dialog; the bar is a marquee until the total is known. Edges: offline, slow, 404, bad signature, hash mismatch, disk full or antivirus locking a folder (the updater's Disk and InstallFailed), no Start menu folder (create_dir_all), path over 260 (std uses verbatim paths; a `.lnk` target is checked against 259), non-ASCII user name (every path is `OsStr` and wide, never ANSI; the end-to-end root is non-ASCII), OneDrive Desktop (known folder), re-run on an installed copy (no download), cut-off download (the updater resumes).

The only repeating path is the 100 ms timer: two atomic loads and one `SendMessageW`. It is reported as that, not as a frame budget (a native dialog, no animation we own).

## SIZE

A release profile for the package in the root `Cargo.toml` after Velopack's. What Cargo allows per package and what it does not is checked by building, and written in docs/setup.md with the measured size.
