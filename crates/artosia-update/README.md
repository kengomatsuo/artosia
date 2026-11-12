# artosia-update

Self-update for Artosia: check a feed, download with resume, verify, swap the files in on the next start, roll back if the new build does not start. No UI. The feed is one URL.

## Decision

1. A plain static folder of files plus a signed `manifest.json` fits every host (GitHub Releases, Cloudflare R2 or Pages, any web server). No server code, no accounts, no packaging tool.
2. An Ed25519 signature over the manifest, checked against a public key compiled into the app, plus a SHA-256 for every file, means a hijacked host cannot push a build. This is also the part no surveyed library gave us for a folder of files.
3. The swap renames the running exes aside (Windows allows renaming a running exe, refuses overwriting it) and keeps a journal, so a power cut or a bad build is undone. About 1,100 lines of Rust, release tool included, on `ureq`, `ring`, `serde_json` and `semver`, all pure Rust or already in the tree.

## What the surveyed options do (read 2026-11-10)

| Option | Host it needs | Folder with two running exes | Signatures | Delta | Licence |
|---|---|---|---|---|---|
| [`self_update` 1.3.3](https://github.com/jaemk/self_update) (README and the source in `~/.cargo/registry`) | GitHub, GitLab, Gitea, S3, a `manifest.json` on any static host, or a custom source | Built for one exe (`self_replace`); `MoveAll` swaps several files with rollback, but runs at update time, from a zip or tar.gz | `signatures` feature: zipsign over the archive; `checksums` feature: SHA-256 | Not mentioned | MIT |
| [Velopack](https://docs.velopack.io/) (README, `src/lib-rust` on GitHub; the Rust reference page returned 404) | Any static file host or GitHub Releases | Yes, through its own installer and `Update.exe` that waits for the app to exit | Not covered in the pages read | Yes, automatic | MIT |
| [`axoupdater`](https://github.com/axodotdev/axoupdater) | GitHub Releases or Axo Releases only | Updates the app that a cargo-dist installer wrote an install receipt for | Not mentioned | Not mentioned | Apache-2.0 or MIT |

Why not each: `self_update` is a good fit for a single exe and ships no resume or manifest signature, so most of this crate would be written around it anyway. Velopack is the strongest, but it needs its `vpk` packaging tool and an installer, and it moves the app to its own install location; that is a bigger decision than the owner has made. `axoupdater` is tied to cargo-dist installers and GitHub. Revisit Velopack if the owner wants a real installer and deltas inside files.

Pages that could not be read: `docs.velopack.io/reference/rust` (404). The Velopack, `self_update` and `axoupdater` pages were read through a summarising fetch tool, so every claim above that rests on "not mentioned" means the summary did not find it, not that it is absent.

## SmartScreen

- [Microsoft's SmartScreen page](https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/): reputation is checked for "a URL, a file, an app, or a certificate", and "if there's no reputation, the item is marked as a higher risk and presents a warning". It says SmartScreen checks files an app tries to download and run.
- Unverified: whether a file this crate writes to disk (no Mark of the Web, because it is not a browser or mail client) triggers a prompt when first run. Microsoft's pages read here do not say. Test it on a clean PC before relying on it.
- Unverified: whether a replaced unsigned exe loses reputation. Reputation attaches to the file or certificate, and an unsigned exe has only its file hash, so each new build is a new file. [Velopack's signing page](https://docs.velopack.io/packaging/signing) states that unsigned apps "are frequently flagged by SmartScreen".

## Manifest format

A feed is a folder (the "feed URL") holding a signed list and each file, flat. One folder holds every platform (2026-11-12, for the Linux and macOS editions): Windows, published first, keeps the plain names every installed copy and the setup ask for (`manifest.json`, `manifest.json.sig`, `artosia.exe`), and every other platform's list and files carry its name (`manifest-linux-x86_64.json`, `manifest-linux-x86_64.json.sig`, `linux-x86_64__artosia`; `manifest::remote_manifest_name`, `remote_signature_name`, `remote_file_name`). A copy asks for its own platform's list, the manifest names its platform, and `check` refuses another. In the staging folder of an installed copy the list is always `manifest.json`.

```json
{
  "format": 1,
  "version": "0.2.0",
  "platform": "windows-x86_64",
  "notes": "What changed, one or two lines",
  "released": "2026-11-10",
  "files": [
    { "path": "artosia.exe", "name": "artosia.exe", "size": 31457280, "sha256": "<64 hex>", "exec": true },
    { "path": "lib/data.bin", "name": "lib__data.bin", "size": 120000, "sha256": "<64 hex>" }
  ]
}
```

- `path` is where the file goes in the install folder; `name` is its flat name on the host. Paths cannot start with `update`, contain `..`, a drive or a backslash.
- `manifest.json.sig` is the 64-byte Ed25519 signature over the exact bytes of `manifest.json`, as 128 hex digits.
- Only a version strictly newer than the running build counts, so replaying an old signed manifest does nothing.
- Files whose installed copy already has the right hash are not downloaded (the future GStreamer DLLs cost nothing on most releases).

## Making a release

Releases are GitHub Releases of `kengomatsuo/artosia` (owner's call, 2026-11-10). An installed copy asks `https://github.com/kengomatsuo/artosia/releases/latest/download/manifest.json`, which GitHub redirects to the newest published release, so the address never changes.

```
tools/publish.sh 0.1.1 "What changed"
```

It builds the three programs with video, builds the projector a second time without video as `artosia-output-plain.exe`, signs the manifest, zips the programs for a first install by hand, tags, and uploads everything as a draft that is published last. It needs the version already set in `crates/artosia/Cargo.toml` and committed, and the GitHub CLI signed in.

The signing key was made on 2026-11-10 with `artosia-release keygen` and lives outside the repository at `%USERPROFILE%\.artosia-keys\update-signing.key`. Its public half is the line in `update-public-key.txt`. **Losing the file means every installed copy needs a USB stick once more, so it wants a backup.** Several public lines are allowed, so a key can be rotated by shipping a build that trusts both.

`tools/release.ps1` still makes an upload-ready folder for any other host; `ARTOSIA_UPDATE_FEED`, or one line in `update-feed.txt` beside the program, points a copy at it.

### Speech recognition comes in the release when the tools exist

`tools/publish.sh` sources `tools/speech-env.sh`; with CMake and libclang found it builds the operator with `--features speech` and prints "WITH speech recognition", otherwise "WITHOUT". whisper.cpp links statically, so no file is added to the release; `artosia.exe` grows to about 38 MB. The 57 MB model is not in the release.

### GStreamer comes in the release

The projector built with video does not start at all on a computer without GStreamer: Windows cannot load it (`gio-2.0-0.dll: cannot open`, tried on 2026-11-10 with GStreamer off the PATH). So a release carries the part of GStreamer the programs need in a `gstreamer` folder: 31 plugins and the 40 libraries they import, 65 MB of the installer's 306 MB, with the installer's licence files in `licenses.zip`. `tools/gstreamer-subset.py` names each plugin with the reason Artosia needs it and follows every file's import table for the rest, so the list is derived, never kept by hand.

The operator looks in its own folder first and hands the programs it starts that GStreamer alone: its plugin folder in `GST_PLUGIN_SYSTEM_PATH_1_0` and a plugin cache of its own in `GST_REGISTRY_1_0` (`crates/artosia/src/link.rs`, `with_gstreamer`). An installed GStreamer is used only when the folder is missing.

Tried on 2026-11-11: a projector started with nothing but the bundle on its PATH played `fixtures/bg30.mp4` on the second screen through `d3d11h264dec`, 853 frames and one full loop, and all 30 of its GStreamer modules were loaded from the bundle. The streaming program, started the same way, listed 17 sound inputs, 2 screens and 14 windows. Not tried on the bundle alone: the software decoder on a graphics chip without one, a camera, and a stream actually sent.

`artosia-output-plain.exe`, the projector built without video, stays in the release for a copy whose `gstreamer` folder is missing: the operator starts it when it finds no GStreamer at all (`output_program`), and that computer still shows the words.

The bundle includes x264 (GPL) and FFmpeg's libraries (LGPL) beside GStreamer's own (LGPL); Artosia is GPL-3.0-or-later, which all of them allow.

## API

```rust
check(feed, current) -> Result<Option<Release>, UpdateError>      // blocking, 8 s total, worker thread
download(&Release, into, &dyn Fn(done, total)) -> Result<Staged, UpdateError>
apply_on_start(install_dir) -> Result<Applied, UpdateError>
confirm_started(install_dir) -> Result<(), UpdateError>
roll_back(install_dir) -> Result<Applied, UpdateError>
relaunch(path) -> io::Result<()>
staging_dir(install_dir) -> PathBuf                                // pass this as `into`
```

Each also has a `_with` form taking `&TrustedKeys`, which the tests use with a throwaway key. `UpdateError::is_quiet()` is true for offline, slow, nothing published, not set up, rejected; an automatic check should swallow those and a manual "check now" button should show them. `detail()` is the technical cause for the log.

## How the swap works

`apply_on_start` runs first in the operator's `main`.

1. Nothing staged: returns at once (one directory check).
2. A staged update: re-verifies the signature and every staged file's hash, writes a journal, renames each file being replaced to `<name>.artosia-old`, renames the staged file into place. A failure part way puts everything back.
3. It returns `Applied::Updated { relaunch }`. This process still runs the old code, so the caller starts `relaunch` (the install folder path of the running exe, taken before any rename) and exits.
4. The new build calls `apply_on_start` again (counts one start), and `confirm_started` once its window is up. That deletes the `.artosia-old` files. Any file Windows still holds locked (the old projector exe still running) is retried on every later start.
5. If the new build starts twice without confirming, its third start rolls back. If the power failed during the swap, the next start undoes it. A rolled-back version is remembered, so `download` returns `Rejected` for it and the app does not loop.

## What was proven (tests, this PC, no internet)

`tests/end_to_end.rs` serves a fake feed from 127.0.0.1 and updates a fake install folder of real, runnable dummy programs, two of them running during the swap: files change, running programs survive, old files are gone after the next start; a tampered download, a tampered staged file, a wrong signature, an edited manifest and a missing key are all refused; a cut-off download resumes with a `Range` request, and a server that ignores `Range` restarts the file; rollback, never-confirmed rollback and crash recovery restore the old files; offline is a quiet error; the release tool's output is accepted.

## Proven over the internet (2026-11-11)

A copy built to call itself 0.0.9 was started from a scratch folder on a scratch library with the projector off. It found release 0.1.0 at the GitHub address, accepted its signature with the built-in key, and downloaded only `artosia.exe` (36 MB); the three programs it already had at the right hash were left alone. Its next start swapped the file in, started 0.1.0 with the same arguments and left. The installed file then matched the release's hash, and the copy of the old file was gone once 0.1.0 had its window up.

The same copy, then at 0.1.0, updated to 0.1.1 the same way: 77 files, the `gstreamer` folder among them, each matching the release's hash afterwards. From 0.1.1 to 0.1.2 it fetched only the three programs that changed and left the `gstreamer` folder alone; all 77 files matched again.

The released 0.1.16, copied into a scratch folder and started minimised on a scratch library with the projector off (2026-11-12), staged 0.1.17 beside itself in 18 seconds, three files; its next start came back as 0.1.17 with the same arguments and left nothing in the `update` folder.

Not tried: a start that fails after the swap on a real release (the rollback is covered by the local tests), and a computer that has never run Artosia, where SmartScreen may ask about the unsigned programs.

## What is still the owner's

- **A backup of the signing key.** See above.
- **No certificate (owner's decision, 2026-11-12).** The direct download stays unsigned, and the Microsoft Store edition is the download Windows does not warn about ([docs/store.md](../../docs/store.md)).
- **Install folder.** The app must be able to write to its own folder, so not `Program Files` unless it runs as administrator.
- **Limit.** If the new exe cannot start at all (a missing DLL, before `main`), nothing can roll it back; a tiny separate launcher would be needed for that. Not built.
