---
paths:
  - "crates/artosia-core/**"
---

# Songs, Bibles and search (2026-11-10)

- **SEARCH MATCHES WORDS THROUGH A VOCABULARY INDEX, NEVER BY SCANNING LINES (`search.rs`, `TextIndex`, 2026-11-10).** A test builds 3,000 songs and fails if a query gets slow. Full account: [docs/architecture.md](../../docs/architecture.md)
- **INDONESIAN COMES FIRST: AN ELIDED `s'gala` IS INDEXED AS `sgala` AND `segala` (`text.rs`, `Word::full`, 2026-11-10).** 1,232 of the church's songs are Indonesian only. Full account: [docs/song-library.md](../../docs/song-library.md)
- **A BLANK LINE INSIDE A MARKED SECTION STARTS A SLIDE; WITH NO MARKERS IT STARTS A VERSE (`song.rs`, 2026-11-10).** OpenLyrics `<lines>` and the church's hand-made slide breaks depend on it. Full account: [docs/operator.md](../../docs/operator.md)
- **AN IMPORT NEVER OVERWRITES A SONG EDITED IN ARTOSIA (`store.rs`, `import_songs`, `Song::edited`, 2026-11-10).** It matches on the library's stable id, updates what changed, and counts the edited songs it kept. Full account: [docs/operator.md](../../docs/operator.md)
- **NO COPYRIGHTED BIBLE TEXT IS IN THE REPOSITORY, A TEST OR THE RELEASE; A BIBLE ARRIVES BY A CHURCH'S OWN FILE, eBIBLE'S REDISTRIBLE LIST OR THE BEBLIA REPOSITORY, WHOSE FILES SHOW THEIR OWN COPYRIGHT LINE BEFORE A PRESS OF DOWNLOAD (`beblia.rs`, `bible_file.rs`, `ebible.rs`, `crates/artosia/src/bibles.rs`, 2026-11-11, owner-directed).** The TB is LAI's and needs LAI's key; its API was left unbuilt because its documentation answers 404. Full account: [docs/bibles.md](../../docs/bibles.md)
- **A BACKDROP IS LISTED ONLY WHEN WIKIMEDIA COMMONS' OWN RECORD FOR THAT FILE SAYS CC0 OR PUBLIC DOMAIN, WITH AN AUTHOR AND NO RESTRICTION, AND IT IS FETCHED FROM COMMONS ON A PRESS, NEVER SHIPPED (`tools/backdrops.py`, `crates/artosia-core/assets/backdrops.json`, `backdrops.rs`, 2026-11-11, owner-asked).** Pexels, Pixabay and Unsplash forbid a competing collection or need a key, so they are not in it. Full account: [docs/backdrops.md](../../docs/backdrops.md)
- **AN INSTALLED CRATE'S SOURCE IS THE API, NOT THE PREVIOUS VERSION'S (quick-xml 0.42, 2026-11-10).** Its events became strings since 0.41 and code written from the 0.41 source did not compile. Full account: [docs/architecture.md](../../docs/architecture.md)
