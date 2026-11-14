---
paths:
  - "crates/artosia-setup/**"
  - "tools/publish.sh"
---

# The setup program (2026-11-12)

- **`Artosia-setup.exe` IS THE SAME FILE IN EVERY RELEASE UNLESS ITS OWN CODE CHANGED: WINDOWS TRUSTS AN UNSIGNED FILE BY THAT EXACT FILE'S DOWNLOADS, AND A NEW FILE STARTS FROM NONE (`tools/publish.sh`, owner's choice to stay unsigned, 2026-11-12).** Bake nothing of a release into it; `ARTOSIA_NEW_SETUP=1` is the only way a new one ships. Full account: [docs/setup.md](../../docs/setup.md)
- **A SHORTCUT KEEPS THE ICON WINDOWS FIRST DREW FOR `artosia.exe,0`, SO THE PROGRAM KEEPS ITS ICON AS A FILE NAMED AFTER ITS BYTES AND POINTS ITS OWN SHORTCUTS THERE AFTER EVERY GOOD START (`crates/artosia/src/shortcut_icon.rs`, `tools/check-shortcut-icon.ps1`, 2026-11-12, owner-reported).** No refresh call changed the picture on Windows 11; the setup copy on an installed PC is never updated, so the setup cannot do it. Full account: [docs/setup.md](../../docs/setup.md)
- **ONE RELEASE HOLDS A SIGNED LIST PER SYSTEM: WINDOWS KEEPS `manifest.json` AND PLAIN FILE NAMES, BECAUSE EVERY INSTALLED COPY AND THE SETUP ASK FOR THEM, AND ANOTHER SYSTEM'S LIST AND FILES CARRY ITS NAME (`crates/artosia-update/src/manifest.rs` `remote_manifest_name`, `tools/publish.sh`, `tools/package-linux.sh`, 2026-11-12).** The Linux edition is a folder using the distribution's GStreamer; its builds are paused while WSL is off limits (2026-11-14). Full account: [docs/linux.md](../../docs/linux.md)
- **A MAC APP IS UPDATED AS ITS WHOLE `Contents` FOLDER AND NOTHING IS EVER ADDED INSIDE IT: DOWNLOADS, THE JOURNAL AND REPLACED FILES WAIT IN `.Artosia-update` BESIDE THE APP (`crates/artosia-update/src/apply.rs` `mac_app`, `aside`, `crates/artosia/src/update.rs` `updatable_dir`, 2026-11-13).** One file more inside a signed app breaks its seal; a release lists the signature's own files, so a copy that swapped them in is that release. Full account: [docs/macos.md](../../docs/macos.md)
- **THE MAC EDITION IS ONE AD-HOC SIGNED APP IN A DISK IMAGE WITH ITS OWN PART OF GSTREAMER IN `Contents/Resources`, BUILT ON A MAC OVER SSH FROM WINDOWS, ITS UPDATE LIST SIGNED AGAIN ON THE RELEASE PC (`artosia-release sign`) (`tools/package-macos.sh`, `tools/publish.sh`, 2026-11-13).** The update signing key never goes to the Mac. Full account: [docs/macos.md](../../docs/macos.md)
- **UNINSTALL NEVER TOUCHES THE CHURCH'S LIBRARY, AND A TEST RUN NEVER TOUCHES THE REAL FOLDERS (`ARTOSIA_SETUP_ROOT`, `--no-start`).** The program lives in `%LOCALAPPDATA%\Programs\Artosia`, apart from the UI library's own `%LOCALAPPDATA%\artosia`. Full account: [docs/setup.md](../../docs/setup.md)
