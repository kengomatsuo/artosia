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

# SETUPUI1: the setup's own window (plan, written before the code, 2026-11-12)

The owner picked "Split panel and one button", after Opera's web installer, from four directions drawn in `crates/artosia-setup/examples/looks.rs` (research: `~/Shared/inspo/artosia-installer/`, `README.md`, `directions.md`, `contact-sheet.png`, `renders/`). The task dialogs above stay as the fallback. What this section says replaces the sections above wherever they differ.

SOURCES: `products/e-browsers-tools/24-opera-gfg-install.png` and `25-opera-gfg-extracting.png` (S01 and S02 copied); Mobbin `s03-glassdoor-didnt-upload-try-again.webp`, `s04-air-launching-app.webp`, `s05-babbel-this-will-this-will-not.webp`, `s02-base-processing-do-not-close.webp` (S03 to S06 restyled, Opera having no screen for them); Rune at the pinned rev (`windows.rs`, `app.rs`, `el/tx.rs`, `window/frame.rs`, `tooling/args.rs`) for the window's options, the poll hook, `Tx::close_window`, `App::on_ready`, headless scenes. The code to port is the `opera` function of `looks.rs` with its palette, sizes and copy.

BASE: nothing general. The state of the window is a pure machine (`src/flow.rs`, in the library, no Rune and no Windows), named for what it is: the screens of one setup run. Two users: the install flow and the uninstall flow. The window (`src/window.rs`, in the exe only) draws a `Model` and feeds events to it.

BOUNDARY: the exe is the composition root. `main.rs` chooses a front: none (`--silent`), the task dialogs (`dialog.rs`, unchanged), or the window. The library owns the work (`install`, `uninstall`, `running_programs`, `Step`, `Failed`) and the machine; it never knows Rune or the dialogs. The window owns a `Shared` cell the worker thread writes (step, result) and the worker wakes the window with Rune's `Waker`. No cycle: `window` and `dialog` import the library and never each other.

SCREEN (700 by 440 points, borderless, one window that changes in place, the same size on every screen):
- S01 Start: the panel with four feature lines, the mark and name, the tagline, one sentence, one filled Install button at the foot. Reached by opening the setup with no flag.
- S02 Progress: the name, the status line (the updater's own step and megabytes) with the percent beside it, one bar, "Installing Artosia" and a sentence, the real Artosia window cut off at the lower right, a quiet Cancel. Reached by Install, or Try again.
- S03 Failure: a yellow disc with a cross, "Artosia was not installed", the cause in words, Close and Try again. Reached from S02 when the worker fails (for a failed removal, with the removal's own words).
- S04 Done: a filled tick, "Artosia is installed", "Artosia is opening now." Reached from S02 when the worker succeeds; Artosia is started and the window closes by itself 1.6 s later.
- S05 Remove: "Remove Artosia?", what goes and what stays, Cancel (focused) and Remove. Reached by `--uninstall`.
- S06 Close first: the ring with the mark, "Close Artosia first", the sentence, Cancel. Reached from S05 when a program of the install folder is running; the window checks each second and goes on by itself.
- Removing (not drawn by the direction): S02's layout with "Removing Artosia" and "Removing the program and its shortcuts", a sliding bar, no Cancel (never available here, so absent). A failed removal is S03's layout with the removal's own words.
- Already installed (`artosia.exe` is in the folder): the same S01. Install then shows S02 for the instant it takes to write the shortcuts and the entry (first status line, no download), then S04, and Artosia starts, as today.

TREE (from `looks.rs` `opera`): root `el` (page fill, clipped) > S01: `row` of the left panel (`col`, 280 wide, gradient, four icon rows) and the right `col` (lockup, tagline, sentence, spacer, Install); S02 to S06: blocks placed in the page; plus the close cross (26 by 26, 12 from the top, 38 from the right). Kit parts: none; every leaf is `el()`, `text()`, `icon()`, `canvas()`, `row()` or `col()`. The kit's `Progress` is 8 pt, rounded, in kit colours; the direction's bar is 17 pt square.

KIT: held against the task dialog it replaces (the owner called it "not beautiful nor professional-looking") and against Opera's own window. No kit part is used.

PLACE: Windows only. Borderless, not resizable, `min_size` and `max_size` both 700 by 440, centred by `App::on_ready` and `WindowHandle::center`, movable by `.window_drag()` on the page root (panels and empty areas; buttons take their own presses). Its own close cross. Taskbar icon: `rune::set_app_icon` with `artosia-icon-256.png`; the exe's icon resource stays. Title: the table's `title` as the window title (the accessible name). DPI: the manifest says per-monitor v2 and Rune lays out in points, so 700 by 440 points is 875 by 550 pixels at 125 percent, 1050 by 660 at 150 and 1400 by 880 at 200; proved by headless renders at `--scale 1`, `1.25`, `1.5` and `2`, the real window only at this PC's own scale.

RULES: one screen at a time. The filled button is the window's `default_button` (Enter). Escape cancels or closes: S01 closes, S02 cancels while Cancel is on offer, S03 closes, S04 closes at once, S05 and S06 cancel. Tab visits every control with a focus ring (`tab_visits_controls`, `Cx::focus_ring`). Colours are the direction's (the brand's Crust, Ink and Crumb with warm neutrals; yellow for the failure mark; no red); every text and button pair is held at 4.5 to 1 by a test.

WORDS: the strings the window needs go into `src/strings.rs` with a brief each in `strings-brief.json`. No Indonesian is written: the new keys are left out of `strings-id.json`, fall back to English, and the test that wants a whole Indonesian table fails until a writer has written it. House rules: no em-dash, en-dash, middle dot, emoji or exclamation mark; sentence case. New keys: `start_tagline`, `start_body`, `install`, `feature_songs`, `feature_bible`, `feature_phone`, `feature_projector`, `wait_body`, `wait_remove_body`, `done_heading`, `done_body`.

DATA: progress comes from `artosia_update::download_with`'s own callback through `install::install`'s `report(Step)`, written by the worker into `Shared` (a `Mutex<Step>` and a `Mutex<Option<result>>`), read by the window's `poll`. The percent is `done * 100 / total`. The product picture is one of four embedded images (light and dark, English and Indonesian), chosen by the system theme and the Windows language; nothing is read from disk or the network.

STATE (owner, kind, readers):
- `Model.screen`: the machine, leaf state of the window, read by `view`.
- `Model.mode`: install or uninstall, fixed at launch.
- `Shared.step` and `Shared.result`: written by the worker, drained by `poll` into events, then folded into `Model`.
- `Model.close_at` (a deadline on Rune's clock, set at S04) and `Model.next_check` (the one-second busy check at S06), read by `poll`.
- Nothing stored that can be derived: the percent, the status text and the picture are computed in `view`.

ACTION (each press, key or arrival is one event into the machine and one commit):
- press Install, or Enter on S01: event `Install`; screen becomes S02 with step Checking; command `StartWork`; commit 1: the S02 tree replaces S01; nothing else renders. moves: none (a still change; a spring on the bar's fill is named for a later pass). fails: the worker's failure is its own event.
- incoming progress: the worker writes `Shared` and wakes the window only when the step, the whole megabytes or the percent changed (about 260 wakes for 161 MB: one per megabyte and one per percent); `poll` turns it into `Progress(step)`; commit: the status line, the percent, the bar. At step SettingUp the Cancel link dims and stops answering.
- incoming result: Ok is S04 with the commands `StartApp` and a close deadline 1.6 s on; Err is S03 with the kind's words, the cause going to the log file as today.
- press Try again (S03): event `TryAgain`; back to S02 Checking; a new worker over the same staging folder (finished files are kept).
- press Close, Escape or the cross on S03: exit 1. On S04: close at once, exit 0.
- press Cancel on S02, Escape, or the cross: allowed until the swap starts; exit 0, the worker is left to the exit and the parts stay for the next run to resume.
- S04 deadline: `poll` sees `clock >= close_at` and calls `Tx::close_window`.
- `--uninstall`: S05 opens with focus on Cancel. Press Remove: if a program of the folder is running go to S06, else to Removing with `StartWork`. Cancel, Escape or the cross exit 0. S06: `poll` asks `running_programs` once a second (`wake_after`) and goes on to Removing when none runs; Cancel exits 0. Removing success: start the delayed self delete and close; failure: S03 with the removal words, Try again or Close.
- long press, swipe, back: n/a (a desktop window). keyboard: Enter presses the filled button; Escape as above; Tab and Shift Tab move a focus ring.

ARRIVE: the window opens already on S01 or S05 (the model is built before the window); pictures are decoded in `setup` before the first frame (`Image::wait`), so frame one is whole. Nothing goes over the network until Install is pressed. Offline: the press still answers in the next frame (S02) and the failure is S03.

FLAGS: no flag, role or record setting decides what the window offers. Switches: `--silent` has no window at all; `--no-start` and a test root start no program; `--dialogs` forces the task dialogs; `--show-window` is the window process itself (see FALLBACK); `--refresh` stays silent.

FALLBACK: the default run is a parent that starts the same exe with `--window` and waits. A child that ends with one of the exit codes the flows use (0, 1, 2, 5) is the answer. Any other end (the UI library's own error exit 70, a crash, an abort, which is what a missing or broken graphics adapter gives with `panic = "abort"`) makes the parent run the task-dialog flow, so an old PC still installs. Proved by a test of the decision function (`flow::run_front`) and by a forced run (`--window-fail`).

TIMING: t=0 press: the next frame is S02 (no network wait). Load: S02 shows "Looking for the latest version" at 0, replaced by the megabytes when the first byte count arrives, by the instant the first response is in. Animations: none built; every screen is a still, which is what the owner chose from; when a motion is wanted its property, thread and spring are named then. The hot path is the progress wake (at most about 260 an install: a lock, a compare, a `wake`) and the Rune frame it causes; measured in the report on the real window.

SIZE: every region has its size before data arrives: the bar is always 628 by 17, the status line and percent one line at 14 pt, the picture block 328 by 240 at a fixed place, so nothing moves when the numbers change. 700 by 440 on every screen.

EDGES: empty n/a; loading is S02 before the total is known (the bar shows nothing until the first count, as Opera's does); error S03 for each kind (offline, damaged, disk, shortcuts, other); one row n/a; longest text: Indonesian `failed_damaged` and `remove_body` fit in the direction's renders; scrolling n/a; keyboard focus on every control; rotated n/a; dark and light follow the system; display scaling 100 to 200 percent by renders; no usable graphics adapter or a UI library failure: FALLBACK; already installed: described under SCREEN.

GUIDE: a borderless window has no system menu or snap layouts from a title bar, so the close cross is drawn and Alt+F4 still closes; a screen reader reads the window title and the labelled buttons (`role` and `label` on every control).

CHECKS (written first): pure tests for `flow.rs` (start, installing, each step, done, failed, try again, cancel, remove, close first, removing), the decision function of the fallback and the contrast pairs; the existing end-to-end tests untouched; headless renders of the shipped exe (`--show-window --snap --scene S0n`, light and dark) laid beside the `looks.rs` renders with every difference listed; one real run in a test root against `dist/0.1.14/feed` on 127.0.0.1 with pictures from `tools/grab-window.ps1`.
