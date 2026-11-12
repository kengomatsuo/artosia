---
paths:
  - "crates/artosia-setup/**"
  - "tools/publish.sh"
---

# The setup program (2026-11-12)

- **`Artosia-setup.exe` IS THE SAME FILE IN EVERY RELEASE UNLESS ITS OWN CODE CHANGED: WINDOWS TRUSTS AN UNSIGNED FILE BY THAT EXACT FILE'S DOWNLOADS, AND A NEW FILE STARTS FROM NONE (`tools/publish.sh`, owner's choice to stay unsigned, 2026-11-12).** Bake nothing of a release into it; `ARTOSIA_NEW_SETUP=1` is the only way a new one ships while it is unsigned. Once the owner's Certum certificate is on this PC (`ARTOSIA_SIGN_SHA1`), every release signs its programs and a fresh setup. Full account: [docs/setup.md](../../docs/setup.md)
- **UNINSTALL NEVER TOUCHES THE CHURCH'S LIBRARY, AND A TEST RUN NEVER TOUCHES THE REAL FOLDERS (`ARTOSIA_SETUP_ROOT`, `--no-start`).** The program lives in `%LOCALAPPDATA%\Programs\Artosia`, apart from the UI library's own `%LOCALAPPDATA%\artosia`. Full account: [docs/setup.md](../../docs/setup.md)
