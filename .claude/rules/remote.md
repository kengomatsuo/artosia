---
paths:
  - "crates/artosia-remote/**"
---

# The phone remote

- **THE PHONE PAGE COPIES PROCLAIM REMOTE AND MEANS THE SAME BY ITS COLOURS: RED IS WHAT THE ROOM SEES, YELLOW IS HIDDEN OR A WARNING, A CHOSEN STATE IS A FILL, AND NO WORD IS CUT AT 320 POINTS IN ENGLISH OR INDONESIAN (`src/page.html`).** Full account: [docs/remote.md](../../docs/remote.md)
- **THE PHONE PAGE'S LANGUAGE IS CHOSEN BY THE SAME RULE AS THE SETUP'S, AND ITS PICKER AND SECTION NAMES COME FROM THE TABLES (`crates/artosia-locale`, `fit` and `labelRules` in `src/page.html`).** A language is added as a table in `EXTRA_LANGUAGES`, never as code in the page. Full account: [docs/remote.md](../../docs/remote.md)
- **A TEST OR AN EXAMPLE OF THE REMOTE LISTENS ON THIS COMPUTER ONLY (`RemoteConfig::local_only`, `examples/serve.rs` `--lan`).** A listener on the network makes Windows Firewall ask whether to allow it, and the question sits on the screen taking every real key and click. A local server also gives only this computer's address (`RemoteServer::urls`): finding the network's address opens a socket on every network, which may be asked about too (not proven). Windows remembers its answer per program file, so a rebuilt test or a copy of the program in a new folder is asked about again. Full account: [docs/remote.md](../../docs/remote.md)
