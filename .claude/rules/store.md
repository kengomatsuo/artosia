---
paths:
  - "crates/artosia/src/store.rs"
  - "tools/store/**"
  - "tools/store-package.sh"
---

# The Microsoft Store edition

- **ONE BUILD SERVES BOTH EDITIONS: `store::packaged()` ASKS WINDOWS AT RUN TIME, AND PACKAGED THE UPDATER IS OFF, THE LIBRARY LIVES IN DOCUMENTS\ARTOSIA AND THE MANIFEST, NOT ARTOSIA, DECLARES THE FIREWALL RULE (`crates/artosia/src/store.rs`, `data.rs`, `update.rs`, `firewall.rs`, `tools/store-package.sh`).** The three helper programs need no `Application` of their own; a package built with the placeholder identity is not for upload. Full account: [docs/store.md](../../docs/store.md)
