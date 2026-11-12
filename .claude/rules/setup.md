---
paths:
  - "crates/artosia-setup/**"
  - "tools/publish.sh"
---

# The setup program (2026-11-12)

- **`Artosia-setup.exe` IS THE SAME FILE IN EVERY RELEASE UNLESS ITS OWN CODE CHANGED: WINDOWS TRUSTS AN UNSIGNED FILE BY THAT EXACT FILE'S DOWNLOADS, AND A NEW FILE STARTS FROM NONE (`tools/publish.sh`, owner's choice to stay unsigned, 2026-11-12).** Bake nothing of a release into it; `ARTOSIA_NEW_SETUP=1` is the only way a new one ships. Full account: [docs/setup.md](../../docs/setup.md)
- **A SHORTCUT KEEPS THE ICON WINDOWS FIRST DREW FOR `artosia.exe,0`, SO THE PROGRAM KEEPS ITS ICON AS A FILE NAMED AFTER ITS BYTES AND POINTS ITS OWN SHORTCUTS THERE AFTER EVERY GOOD START (`crates/artosia/src/shortcut_icon.rs`, `tools/check-shortcut-icon.ps1`, 2026-11-12, owner-reported).** No refresh call changed the picture on Windows 11; the setup copy on an installed PC is never updated, so the setup cannot do it. Full account: [docs/setup.md](../../docs/setup.md)
- **UNINSTALL NEVER TOUCHES THE CHURCH'S LIBRARY, AND A TEST RUN NEVER TOUCHES THE REAL FOLDERS (`ARTOSIA_SETUP_ROOT`, `--no-start`).** The program lives in `%LOCALAPPDATA%\Programs\Artosia`, apart from the UI library's own `%LOCALAPPDATA%\artosia`. Full account: [docs/setup.md](../../docs/setup.md)
