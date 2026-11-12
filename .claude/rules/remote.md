---
paths:
  - "crates/artosia-remote/**"
---

# The phone remote (2026-11-11)

- **THE PHONE PAGE COPIES PROCLAIM REMOTE, THE OWNER'S PICK OF FOUR CANDIDATES BUILT FROM SAVED REFERENCES, AND MEANS THE SAME BY ITS COLOURS: RED IS WHAT THE ROOM SEES, YELLOW IS HIDDEN OR A WARNING, A CHOSEN STATE IS A FILL, AND NO WORD IS CUT AT 320 POINTS IN ENGLISH OR INDONESIAN (`src/page.html`, 2026-11-12, owner-reported 2026-11-11: "it looks so ugly").** Full account: [docs/remote.md](../../docs/remote.md)
- **THE PHONE PAGE'S LANGUAGE IS CHOSEN BY THE SAME RULE AS THE SETUP'S, AND ITS PICKER AND SECTION NAMES COME FROM THE TABLES (`crates/artosia-locale`, `fit` and `labelRules` in `src/page.html`, 2026-11-12).** A language is added as a table in `EXTRA_LANGUAGES`, never as code in the page. Full account: [docs/remote.md](../../docs/remote.md)
- **A TEST OR AN EXAMPLE OF THE REMOTE LISTENS ON THIS COMPUTER ONLY (`RemoteConfig::local_only`, `examples/serve.rs` `--lan`, 2026-11-11).** The example listened on the network, Windows Firewall asked whether to allow it, and two such questions sat on the owner's screen for hours taking every real key and click. A local server also gives only this computer's address (`RemoteServer::urls`): finding the network's address opens a socket on every network, which may be asked about too (not proven). Windows remembers its answer per program file, so a rebuilt test or a copy of the program in a new folder is asked about again: its log held 13 such questions in two days (2026-11-12, owner: "so many administrator requests for almost every action"). Full account: [docs/remote.md](../../docs/remote.md)
