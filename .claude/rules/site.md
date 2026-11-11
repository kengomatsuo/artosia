---
paths:
  - "site/**"
  - "tools/build-site.py"
  - "tools/publish-site.sh"
  - "tools/site-shots.sh"
---

# The homepage (2026-11-11)

- **THE HOMEPAGE WEARS MATSUOKENGO.COM'S LOOK FROM THAT SITE'S OWN FILES, AND EVERY PICTURE ON IT IS A RENDER OF THE REAL PROGRAM (`site/`, `tools/site-shots.sh`).** A screen that changes is rendered again; nothing is drawn by hand. Full account: [docs/site.md](../../docs/site.md)
- **A STRING IS WRITTEN IN `site/strings/`, NEVER IN `page.html`, AND THE INDONESIAN IS COMPOSED FROM `brief.json` BY A WRITER WHO NEVER SEES THE ENGLISH.** The build stops on a missing, unused or unmatched key. Full account: [docs/site.md](../../docs/site.md)
- **THE PAGE NAMES THE VERSION OF THE NEWEST RELEASE, READ FROM GITHUB, NEVER FROM `Cargo.toml` (`tools/publish-site.sh`).** A version bumped but unreleased is one nobody can download. Full account: [docs/site.md](../../docs/site.md)
