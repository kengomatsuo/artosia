# The Microsoft Store edition (2026-11-12, card STORE1)

The same programs as the direct download, packaged as MSIX by `tools/store-package.sh`. One build serves both
editions: the program finds out at run time whether it has a package identity. Research is in
`~/Shared/research/artosia-store/` (`docs/` Microsoft's pages, `policies/` the Store's, `open-source/` seven programs
that build their MSIX by script).

## What differs when packaged

| decision | why | where |
|---|---|---|
| `store::packaged()` is `GetCurrentPackageFullName` answering `ERROR_INSUFFICIENT_BUFFER` (identity) rather than `APPMODEL_ERROR_NO_PACKAGE`, cached; `ARTOSIA_EDITION=store\|direct` overrides it; false off Windows | Microsoft's detect-identity page (`docs/detect-identity.txt`, `getcurpkg.txt`) | `crates/artosia/src/store.rs` |
| The updater checks, downloads and applies nothing and never runs the setup's `--refresh` | The Store updates the program and the package folder is read-only. `install_dir()` answers None, so every path below it stops | `update.rs` |
| The library is `Documents\Artosia` | A packaged program's new AppData files are a private copy that Windows deletes on uninstall (`docs/behind-scenes.txt`); a church's songs must outlive that. Documents is not redirected. `--data`, `ARTOSIA_DATA`, headless and `--practice` keep their meaning | `data.rs` `library`, `packaged_library` |
| If `Documents\Artosia` does not exist and `%APPDATA%\artosia` does, its files are copied over once (into a sibling, renamed when whole) | A church that moves to the Store keeps its songs; the old folder stays | `data.rs` `copy_library` |
| The recordings folder asks `data::folder()` like everything else | It built its own AppData path | `actions.rs` |
| Artosia never runs its own elevated firewall step; the manifest declares inbound TCP 4780 for `artosia.exe` on every profile (`desktop2:FirewallRules`, needs `runFullTrust`) | `docs/fw-rules.txt`, `ext-firewall-example.txt`; the background is [remote.md](remote.md) "Why Windows Firewall kept asking" | `firewall.rs` `applies_in`, `tools/store/AppxManifest.xml` |
| Package version `a.b.c` is `(a+1).b.c.0` (0.1.16 is 1.1.16.0) | The first part cannot be 0 and the fourth is reserved (`docs/pkg-reqs.txt`) | `store::package_version`; the script computes the same in shell |
| `artosia.exe` is the one `Application`; the other exes sit in the package folder | Tried, see below | manifest |
| Identity comes from `tools/store/identity.json` | Partner Center gives `Name`, `Publisher` and `PublisherDisplayName` once the name is reserved. The file ships with placeholders and the script prints in capitals that the package is not for upload while they stand | `tools/store-package.sh` |
| Languages: en-US, id, zh-Hans, zh-Hant, ko, ja, es, pt-BR, fr, vi | The program's own tables | manifest |
| Capabilities: `runFullTrust`, `microphone`, `webcam` | Full trust is required for a desktop exe; the two device capabilities make Windows list Artosia in its privacy settings (`docs/capability-decl.txt`) | manifest |

## Build the package

```bash
tools/store-package.sh 0.1.15      # needs dist/0.1.15/programs; makeappx from Windows SDK 10.0.22621.0
```

Makes `dist/<version>/store/` (the loose folder: the release's `programs` as it is, `AppxManifest.xml`, `Assets/`) and
`dist/<version>/Artosia-<version>-x64.msix`. Pictures come from `assets/brand/artosia-icon-512.png` through
`tools/store/make-images.ps1`: StoreLogo 50, Square44x44Logo 44, Square150x150Logo 150, no `makepri` (ChurchPresenter and
mitmproxy ship the same three plain files). The package is unsigned: the Store signs it on publication.

To try it on this PC without a certificate (Developer Mode): `Add-AppxPackage -Register dist\<v>\store\AppxManifest.xml`,
remove with `Get-AppxPackage Artosia.Unreserved | Remove-AppxPackage`. Check packaged behaviour without opening the
window: `artosia.exe --edition [file]` prints the edition, version, data folder and the folder it would copy from, and
exits before any window or library; run it with `Invoke-CommandInDesktopPackage`.

## What was tried (2026-11-12, debug builds of `artosia.exe` and `artosia-output.exe` in a loose package)

- `--edition` inside the package printed `edition=store`, `version=1.1.15.0`, `data=...\Documents\Artosia`, `copy_from=...\AppData\Roaming\artosia`; outside it, `edition=direct` and the AppData folder.
- `artosia-output.exe`, started by a packaged `cmd.exe` with no `Application` entry of its own, started, kept the package identity (`GetPackageFullName` on its process returned the package), ran headless and wrote its snapshot. So the helper exes need no entries. `artosia-stream.exe` and `artosia-output-plain.exe` start the same way but were not run (GStreamer).
- `makeappx pack` of the real `dist/0.1.15/programs`: 69 MB (the folder is 163 MB).

## Not tried

- Install from the Store, or from a signed package (the Store folder is read-only under `WindowsApps`; a loose registration runs from a writable folder).
- Whether Windows shows a firewall box on first listen when packaged, and whether Store certification accepts the rule (GUESS: it does, with `runFullTrust` approved).
- The one-time library copy inside a package (unit tests only, so no real library was touched).
- `makepri`, other tile sizes and per-language manifest strings: the Store listing language list is separate from the package's.
- The manifest's rule has no remote-address limit, unlike the direct edition's `remoteip=localsubnet` (GUESS: the schema offers none; the PIN still guards the remote).

## Left for the other sessions

Packaged, `update::check` returns at once and reports nothing. Settings and the command "Check for updates" must hide when `store::packaged()`; they are in `ui/**` and `palette.rs`.

## The first submission (2026-11-12)

The product is `9NTBWLQ5PBHN` in the owner's Partner Center, publisher matsuokengo; its
identity is in `tools/store/identity.json`. What was done, in order:

| Part of the submission | How it was filled |
|---|---|
| Pricing and availability | Free, every market, by hand |
| Properties | Category Productivity, the privacy page `https://artosia.matsuokengo.com/privacy/`, website and support address, by hand |
| Packages | `tools/store-package.sh 0.1.16` made `dist/0.1.16/Artosia-0.1.16-x64.msix` (72 MB); Partner Center validated it and warned only that `runFullTrust` needs approval |
| Store listings | `tools/store/listing-csv.py` fills the template Partner Center exports with the ten languages of `tools/store/listing/`; the file is imported with Import listings. Screenshots come from `tools/store-shots.sh` and are uploaded on each listing's own page, six a language |
| Submission options | The `runFullTrust` justification and the notes for the testers, both in `tools/store/submission.md` |
| Age ratings | The owner's: the questionnaire takes the owner's e-mail address |

The package was tried before it was uploaded: its loose folder was registered on this PC,
the release's own `artosia.exe --edition` answered `edition=store`, version `1.1.16.0` and
`Documents\Artosia`, and the package was removed again.

Partner Center's page takes a package only through its own file picker or a drop. A session
that cannot use the picker hands the file to the page's own uploader in parts no larger than
9 MB, joined in the page and checked against the file's SHA-256 before the uploader gets it.

Not done: the Windows App Certification Kit was not run, and nothing was installed from the
Store itself, so the firewall rule and the read-only install folder are proven only by the
loose registration.

