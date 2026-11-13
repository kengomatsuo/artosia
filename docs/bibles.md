# Bibles: what Artosia has, what may be shipped, and what to build

Researched 2026-11-11. This is a reading of each rights holder's published terms, not legal advice. Every fact below comes from a page or file fetched that day (see Sources). Where something could not be confirmed it is listed under "Not confirmed".

The owner's question: the Indonesian Bibles matter most, and could Artosia package every Bible in the world, and could the interface go fully global too?

## What Artosia has

| Question | Answer | Where |
|---|---|---|
| Formats it reads | One: MyBible modules (SQLite with `info`, `books`, `verses`). Files ending `.sqlite3` or `.mybible` in the library's `bibles` folder. | `crates/artosia-core/src/bible.rs` (`Bible::open`, `describe`), `import.rs` (`bible_files`) |
| What ships inside the program | The King James Version only, 31,102 verses as gzipped tab-separated lines, 1.3 MB. | `crates/artosia-core/assets/kjv.tsv.gz`, `import.rs` (`bundled_bible`), `tools/make-bible-asset.py` |
| Rule for the bundled asset | The script says only public-domain translations may be packed in. | `tools/make-bible-asset.py` |
| How the list is built | `list_bibles` starts with the packed KJV, then adds every module found in `<library>/bibles` whose code is new. `want_bible` loads one translation on a worker thread and keeps it in memory. | `crates/artosia/src/state.rs` |
| How a church adds a Bible today | Copy a MyBible file into `<library>/bibles`, or run the EasiSlides import in `actions.rs`, which copies `Admin/HolyBibles/*.sqlite3` from an EasiSlides install into that folder. There is no download, no picker for a file, and no catalogue. | `crates/artosia/src/actions.rs` (line 502), `data.rs` (line 43) |
| Book names and typing | Reference typing (`gen 1 1`, `jn 3 16`) matches English names and short forms only, whatever the module's own book names are. Indonesian book names ("Yohanes", "Mazmur") are not in the alias table. | `bible.rs` (`BOOKS`, `guess`) |
| Canon | 66 books. A module with the Apocrypha is read for the 66 only. | `bible.rs` (`MYBIBLE_NUMBERS`) |

The church's own EasiSlides copy in `fixtures/easislides/HolyBibles/` (gitignored) holds five files:

| File | Language field in the file |
|---|---|
| `asv.SQLite3` | English |
| `cuvs.SQLite3` | Simplified Chinese |
| `cuvt.SQLite3` | Traditional Chinese |
| `kjv.SQLite3` | English |
| `rsv.SQLite3` | English |

No Indonesian Bible is in that install. A church that imports from EasiSlides therefore gets no Indonesian Bible, and Artosia today has none to offer.

## Indonesian Bibles

"May ship" means: may the Artosia installer or repository carry the file. "May download" means: may the program fetch it for the user from the rights holder's or eBible's own server, with the notice shown. Both columns are my reading of the published wording.

| Translation | Rights holder | Licence in its own words | May ship? | May download for the user? |
|---|---|---|---|---|
| TB, Terjemahan Baru (1974) | Lembaga Alkitab Indonesia (LAI) | Credit line LAI requires: "Dikutip dari ALKITAB (TB) © LAI 1974" ([policy](https://alkitab.or.id/tentang-kami/penggunaan-teks)). Quoting is allowed up to 1,000 verses and never a whole book for non-commercial use. | No. A whole Bible is far past the limit. | No. Needs LAI's written permission. |
| TB2, Terjemahan Baru Edisi Kedua | LAI | Listed as an LAI edition on [alkitab.or.id](https://www.alkitab.or.id). Same text-usage policy; no separate licence found. | No | No, ask LAI |
| BIMK (Bahasa Indonesia Masa Kini) and BIS (Indonesia Sehari-hari) | LAI | SABDA's notice for BIS: "Released for non-profit scholarly and personal use." ([alkitab.mobi/copyright](https://alkitab.mobi/copyright/)). That is a notice on a third-party site, not an LAI licence. | No | No, ask LAI |
| TL, Terjemahan Lama (1954) | Not named on the page read | No permission text on [alkitab.mobi/copyright](https://alkitab.mobi/copyright/). The copyright status could not be established. | No, until the status is known | No |
| AYT, Alkitab Yang Terbuka | Yayasan Lembaga SABDA (YLSA) | "You do not use this work for commercial purposes." ([ebible.org/indayt/copyright.htm](https://ebible.org/indayt/copyright.htm)). ayt.co says "allow free usage/distribution of the text" ([ayt.co/hakcipta](https://ayt.co/hakcipta)). Credit and source notice required. "AYT" is a trademark. | Only if Artosia stays free of charge, and the notice travels with it. A bundled copy would bind everyone who redistributes the installer to the non-commercial term, so download on demand is the safer route. | Yes, non-commercial, with the copyright notice shown. |
| TSI, Terjemahan Sederhana Indonesia (Albata) | Yayasan Alkitab BahasaKita (Albata) and Pioneer Bible Translators International | "You have permission to share and redistribute this Bible translation in any format" ([ebible.org/ind/copyright.htm](https://ebible.org/ind/copyright.htm)). CC BY-SA 4.0. Attribution required; changes marked. The eBible copy is the New Testament plus 21 Old Testament books. | Yes, with attribution and the same licence on the text file | Yes |
| Alkitab Gratis untuk Semua (Bible for All, `indags`) | Jonathan Gallagher | Same CC BY-SA 4.0 wording ([ebible.org/indags/copyright.htm](https://ebible.org/indags/copyright.htm)). New Testament only. | Yes, same conditions | Yes |
| Shellabear (1912) | British and Foreign Bible Society, per SABDA's page | "The British and Foreign Bible Society, London 1912, 1929, 1949" ([alkitab.mobi/copyright](https://alkitab.mobi/copyright/)). Whether the text is now free is not stated. | No, until confirmed | No |
| Klinkert (1863) | Not named | SABDA's page says "Copyright; Hubungi YLSA." Contact YLSA. | No | No |
| Ende (1970) | Not named | Same SABDA wording, "Hubungi YLSA." | No | No |
| Open Kitab Kehidupan (2025) | Biblica, Inc. | "available under the Creative Commons Attribution-ShareAlike 4.0 International License" ([alkitab.mobi/copyright](https://alkitab.mobi/copyright/)). | Probably yes. Not checked at Biblica's own site or in eBible's list. | Probably yes |
| FAYH (Firman Allah Yang Hidup, 1989) | Living Bibles International | "All rights reserved" ([alkitab.mobi/copyright](https://alkitab.mobi/copyright/)) | No | No |
| AMD (Alkitab Mudah Dibaca, 2014) | Bible League International | "All rights reserved" | No | No |
| VMD (Versi Mudah Dibaca, 2005) | World Bible Translation Center | Quoting over 1,000 verses needs written approval | No | No |
| KSKK (2002) | Bernardo Hurault, Pastoral Bible Foundation | No permission text | No | No |
| MILT (2008) | Yayasan Lentera Bangsa | "Only for non-profit scholarly and personal use." | No | No |

Plain summary:

- **Cannot be shipped or fetched by the program without a written yes:** TB, TB2, BIMK, BIS, TL (status unknown), Shellabear, Klinkert, Ende, FAYH, AMD, VMD, KSKK, MILT.
- **May be shipped:** TSI and Bible for All (CC BY-SA 4.0).
- **May be downloaded for the user, free of charge only:** AYT (non-commercial).
- **LAI offers a developer API** at [bible-api.alkitab.or.id](https://bible-api.alkitab.or.id). It needs registration and a key, and use stays bound by LAI's licence. Using it for the TB means asking LAI first (info@alkitab.or.id, per LAI's own page).
- **The church can still use the TB itself.** LAI's policy allows non-commercial quoting such as slides up to 1,000 verses. The church's own MyBible file of the TB (for example from a purchased app) can sit in the `bibles` folder on that church's computer, as it does for EasiSlides today. The program never carries it.

## A world catalogue

eBible.org publishes `https://ebible.org/Scriptures/translations.csv`, read in full on 2026-11-11.

| Measure | Value |
|---|---|
| Translations listed | 1,556 |
| Languages (distinct `languageCode`) | 1,242 |
| Marked `Redistributable` = True | 1,296 (1,024 languages) |
| Marked False | 260 |
| Public domain (Copyright column reads "public domain") | 112, all redistributable |
| Redistributable and with the whole Old and New Testament | 214 translations in 120 languages |
| `downloadable` = False | 8 |

Exact columns (33): `languageCode, translationId, languageName, languageNameInEnglish, dialect, homeDomain, title, description, Redistributable, Copyright, UpdateDate, publicationURL, OTbooks, OTchapters, OTverses, NTbooks, NTchapters, NTverses, DCbooks, DCchapters, DCverses, FCBHID, Certified, inScript, swordName, rodCode, textDirection, downloadable, font, shortTitle, PODISBN, script, sourceDate`.

Terms, from [ebible.org/find](https://ebible.org/find/): "Listings with a white background are freely redistributable". The same page says the freedom may carry "a noncommercial restriction" and that grey rows may not be redistributed without the owner's permission. The CSV flag says nothing about which restriction applies: AYT is flagged True and is non-commercial only, so the Copyright column and the translation's own `copyright.htm` must be shown to the user. A Bronze licence page exists too: [ebible.org/ccplus.htm](https://ebible.org/ccplus.htm) describes a non-commercial, no-text-changes licence for some translations.

Files per translation, tested by request: `<translationId>_usfx.zip`, `_usfm.zip`, `_vpl.zip` and `_html.zip` exist for AYT. `_osis.zip` returned 404 and `_sqlite.zip` returned 404, so there is no MyBible-ready file and no OSIS file on this host.

| File for `indayt` | Size |
|---|---|
| `indayt_usfx.zip` | 1.7 MB |
| `indayt_usfm.zip` | 1.6 MB |
| `indayt_vpl.zip` | 4.4 MB |

Size of the whole catalogue, measured by a HEAD request on every `_usfx.zip` of the 1,296 redistributable translations (one, `ropk`, returned 404):

| Set | Count | Total zip size | Median |
|---|---|---|---|
| All redistributable | 1,295 | 989 MB | 0.53 MB |
| Of those, whole Bible | 214 | 439 MB | 1.8 MB |
| Of those, New Testament or less | 1,081 | 550 MB | 0.5 MB |

Packing every redistributable translation into the installer is not realistic: about 1 GB of zips (the whole-Bible subset alone is 439 MB), mostly in languages no volunteer at the church can check, and some under non-commercial terms. A download-on-demand picker serves the same goal at a median 0.5 to 1.8 MB per Bible.

Indonesian and Indonesia-region entries in the CSV (all flagged True unless noted):

| Id | Language | Title | Books |
|---|---|---|---|
| `ind` | Indonesian | TSI, second edition | NT, 21 OT books |
| `indags` | Indonesian | Alkitab Gratis untuk Semua | NT |
| `indayt` | Indonesian | Alkitab Yang Terbuka (AYT) | 39 OT, 27 NT, updated 2026-11-10 |
| `mkn` | Kupang Malay | Kupang Malay Bible | NT, 4 OT books |
| `kje` | Kisar | Makromod lalap lirna wawan | NT |
| `zlmKSZI` | Malay (Malaysia) | Kitab Suci Zabur dan Injil | NT |
| `jvnNT` | Javanese, Caribbean (Suriname) | NT | NT |
| `bjn` | Banjar | Bahasa Banjar | partial, False |
| `pmy` | Papuan Malay | Melayu Papua | partial, False |

No Sundanese, Batak, Balinese, Minangkabau, Bugis, Makassarese, Manado Malay or standard Javanese translation was found in the CSV (searched by language name and dialect). Those Bibles exist elsewhere; LAI says it publishes 39 complete Bibles in 39 regional languages ([alkitab.or.id](https://www.alkitab.or.id)), under its own text policy.

Alternatives checked:

| Source | Finding | Usable? |
|---|---|---|
| CrossWire SWORD modules | The module configs (`mods.d.tar.gz`, 428 files, 280 Bible modules, 93 languages) carry a `DistributionLicense` line per module: 142 Public Domain, 65 "Copyrighted; Permission to distribute granted to CrossWire", 21 "Copyrighted; Free non-commercial distribution", plus Creative Commons and GPL ones. No module in Indonesian. Two Indonesian-region modules: Uma New Testament (`uma`, non-commercial) and one in Sama. | Useful as a second world source with a clear licence field. Of no help for Indonesian. Artosia would need a SWORD reader, a second format. |
| MyBible module collections ([mybible.zone](https://mybible.zone/), [ph4.org](https://www.ph4.org/b4_index.php)) | Neither site states a licence or redistribution rights for its modules in the pages read. mybible.zone calls its modules "freely given away" in a work log, which is not a licence. No Indonesian modules seen. | No. Rights are unknown, so Artosia must not fetch or ship from them. A church's own MyBible file stays the church's business. |
| API.Bible through ChurchApps | FreeShow's list at `api.churchapps.org/content/bibles` holds 459 entries in 282 languages (410 from api.bible, 49 from youversion). Indonesian: two TSI entries. | Needs a key and an outside service, so a church on a poor connection loses it. Not wanted. |

## What the siblings do

| App | Formats imported | Downloads in-app | Bundles Bibles | Evidence |
|---|---|---|---|---|
| FreeShow (GPL-3.0) | FreeShow's own `.fsb`, Zefania, OSIS, Beblia, OpenSong. The detector also recognises SoftProjector, Wordproject, BibleQuote, iBible, SQLite and Access files. | Yes. A list and the text come from `https://api.churchapps.org/content/bibles`, which fronts api.bible and YouVersion. | None found in the repository tree | `src/frontend/converters/bible.ts` (`bibleTypes`), `src/electron/data/bibleDetecter.ts`, `src/frontend/components/drawer/bible/scripture.ts` (`SCRIPTURE_API_URL`) |
| OpenLP (GitLab) | CSV, OpenSong, OSIS, SWORD, Wordproject, Zefania | Yes, by reading BibleGateway, BibleServer and Crosswalk web pages | None found. `openlp/plugins/bibles/resources/bibles_resources.sqlite` holds book data. | `openlp/plugins/bibles/lib/importers/` (`csvbible.py`, `opensong.py`, `osis.py`, `sword.py`, `wordproject.py`, `zefania.py`, `http.py`) |
| Quelea (GPL-3.0) | Zefania-style XML files (`.xml`, `.xmm`) read from a Bibles folder | No download code found in the files read | Four: `asv.xml`, `kjv.xml`, `net.xml`, `rsv.xml` | `Quelea/bibles/`, `Quelea/src/main/java/org/quelea/data/bible/BibleManager.java` (`loadBibles`) |

The three do not read MyBible modules in the files read, and none reads USFX. All of them depend on someone else's server or on the user supplying a file. Quelea ships RSV and NET, which are not public domain; Artosia should not copy that choice.

## Recommendation

**Build first: a Bible picker that downloads from eBible.**
Read `translations.csv`, list only `Redistributable` = True, show the copyright line before the download, fetch `<id>_usfx.zip`, convert to a MyBible-compatible module in `<library>/bibles` once. Reasons: one source with a machine-readable list, 1,024 languages, a median zip of 0.5 MB, and it fills the gap the church has today (no Indonesian Bible at all). It needs a USFX reader (USFX is XML) and an Indonesian book-name table so "Yoh 3 16" and "Mazmur 23" resolve; the second is needed even for a TB the church brings.

**Build second: a "use your own file" path.**
A file picker for MyBible, USFM or USFX, and an import of the church's EasiSlides Bibles. Reason: the TB, BIMK and TB2 cannot be shipped or fetched, and the church will want the TB. LAI's policy allows non-commercial slide quoting, and a file the church already holds stays on that computer.

**Ship in the installer: KJV plus, optionally, TSI.**
TSI is CC BY-SA 4.0 and 1.5 MB as a zip. The attribution and licence text must sit beside the file. Reason: a first launch with an Indonesian Bible and no network. Keep AYT as a download, because its non-commercial term would bind everyone who redistributes the installer.

**Ask LAI in writing about the TB.**
A short email to info@alkitab.or.id: a free, GPL, non-commercial presentation app, asking to ship or fetch the TB, TB2 and BIMK. Reason: it is the Bible the church actually reads. No code depends on the answer.

**Never do these:**

- Never bundle all of eBible. About 1 GB, some non-commercial, most languages unchecked.
- Never ship or fetch the TB, TB2, BIMK, BIS, TL, Shellabear, Klinkert, FAYH, AMD, VMD, KSKK or MILT without written permission. The rights holders' pages do not grant it.
- Never scrape alkitab.mobi, SABDA, LAI or BibleGateway pages. SABDA states that it does "not give others permissions" ([copyright.sabda.org](https://copyright.sabda.org/)), and a scrape also breaks whenever the page changes.
- Never trust the `Redistributable` flag alone. Show the copyright line, because True includes non-commercial terms.
- Never add MyBible modules from mybible.zone or ph4.org to a download list. Their pages state no rights.

## Interface languages

The operator window carries any number of languages since 2026-11-11 (`crates/artosia/src/languages.rs`, [docs/i18n/README.md](i18n/README.md)). The table below is what was hard-wired to two before that rework, kept as the record; the Bible book names, the Bible picker and the song search stemmer per language are still to do.

| Where | What is hard-wired to two |
|---|---|
| `crates/artosia/src/strings.rs` | `static INDONESIAN: AtomicBool`, `set_indonesian(bool)`, `indonesian() -> bool`, and the `text!(NAME, "en", "id")` macro, which takes exactly two strings and picks with `if indonesian()`. About 40 functions branch the same way for counted nouns. |
| `crates/artosia/src/state.rs` | `settings.indonesian: bool`, `--indonesian` flag. |
| `crates/artosia/src/palette.rs` | `Hit::Language(bool)` and two entries (`LANGUAGE_ENGLISH`, `LANGUAGE_INDONESIAN`) with English and Indonesian search aliases. |
| `crates/artosia-remote/src/page.html` | `STR.en` and `STR.id` tables, `lang !== "id"` checks, `<html lang="en">`. |
| `docs/i18n/` | `brief.json`, `brief-phone.json`, `concepts.md`, `id.json` and tests that pin Indonesian spellings. |

A shape that carries any number of languages: a `Language` value (a locale tag) in settings instead of the bool; each string a key with one entry per locale (a table or generated file per language) and English as one locale among the others; plural and counter rules chosen per locale; a locale list that the palette and the remote page read instead of two hard-coded rows; and Bible book names, the Bible picker and the song search stemmer per language too. The code for it is not designed here.

What may ship, from the owner's skill `blind-translation`: its section "When not to use it" says "A language nobody on the project can judge" gets English, or a native review that is bought. "Three checkable languages beat fifteen unverifiable ones." Each locale is written from the brief by a writer who never sees the English, checked by back-translation, rendered on the real window, and pinned by a test of known-wrong spellings. So "fully global" means: add a language only when a person who can judge it has agreed to review it. Today that is English and Indonesian. A language whose Bibles exist (Javanese, Sundanese, Chinese from the church's EasiSlides install) is not a reason to ship its interface.

## Built

Built 2026-11-11 on the branch `worktree-agent-a6a81f3f442d350bf`, in four steps; the last was not built.

1. **Indonesian book names in the search** (`crates/artosia-core/src/bible.rs`, `BOOKS_ID`). The 66 full names come from the book list on LAI's own search page (https://www.alkitab.or.id/cari-alkitab/tb/kejadian/1). The short forms (`Kej`, `Mzm`, `Why`, `Flp` and the consonant skeletons) come from the LAI columns of SABDA Labs' table (https://labs.sabda.org/Singkatan). They apply to every loaded Bible, English ones too. Verified by tests: "Yoh 3:16", "Mazmur 23", "1 Kor 13 4-7", "Kej 1 1", "Wahyu 21" and more.
2. **Your own file** (`artosia-core/src/bible_file.rs`, `crates/artosia/src/bibles.rs`). MyBible files are checked and copied; USFX (`.xml`, or a zip with `BookNames.xml` and metadata) and USFM (`.usfm`, or a zip of them) are converted once into `<library>/bibles/<code>.sqlite3`. Footnotes, cross references and headings are dropped. Entry points: a button in the Look dialog, a button in the Bibles dialog, and the commands "Add a Bible from a file" and "Get more Bibles". Verified: unit tests with made-up verses for every format and for refusals; the real `indags_usfx.zip` converts to 7,942 verses (the file has 7,959 verse tags, 17 are empty on purpose).
3. **A Bible picker** (`artosia-core/src/ebible.rs`, `bibles.rs`, `ui/bibles.rs`). The catalogue is fetched on a worker, cached in `<library>/ebible-translations.csv` for a week and served stale when the network fails. Only rows marked `Redistributable` and `downloadable` appear, the languages of Indonesia first (Indonesian, then Malay, Kupang Malay, Javanese and others), then A to Z. A row shows its copyright line before the Download button; nothing downloads before that press. Skeleton, error with retry, empty and list states were rendered in light, dark and Indonesian. Verified: parsing and grouping tests, a cache test, and one live fetch of the real catalogue (`cargo test -p artosia live_ -- --ignored`).
4. **LAI's Bible API: not built.** The API page (https://bible-api.alkitab.or.id) links two Postman documentation pages, `https://documenter.getpostman.com/view/15253393/2sB3WwpwwD` and `.../2sB3Wwpx1W`. Both answered "404 The resource you're looking for can't be found" on 2026-11-11, in the owner's Chrome and from curl, and the Postman gateway API gave 404 too. The API page itself shows only one request, `GET https://bible-api.alkitab.or.id/bible-api/api/v1/bible/tb/kejadian/1/1-5` with `Authorization: Bearer <API_KEY_ANDA>`, a TODO that the base URL is to be confirmed, and a sample whose fields are `book`, `chapter`, `verse`, `text`. Which URLs, response shape, errors and rate limits exist is not documented anywhere readable, so no code was written. To finish it: ask LAI (info@alkitab.or.id) for a working documentation link and a key.

Not verified: a real download through the button (only the same fetch code on the catalogue), the file dialog in a real window, and the Look dialog's two new buttons in a real window (they were rendered, below the fold of the scrolling dialog, and not pressed). Not decided: TSI shipping (see "Not confirmed").

## Beblia, the second source (2026-11-11, owner-directed)

The owner asked for the Indonesian Bibles a church really reads and pointed at https://github.com/Beblia/Holy-Bible-XML-Format. The Bibles dialog lists that repository next to eBible: one list grouped by language with Indonesian first, a small source label on each row, the Beblia rows after eBible's inside a language.

What the repository is, read with `gh api` on 2026-11-11:

- Branch `master`, 1,052 files, 1,048 of them `*.xml` in the root, the rest `README.md`, `.gitattributes` and two files under `tools/`. GitHub reports no licence and the tree has no licence file.
- Its README says: "Use at your own discretion, no need to ask for permission, no warranty's." It speaks for the repository's author. It cannot grant what the translators' own holders keep.
- Eleven files start with `Indonesian`: `IndonesianBible.xml`, `IndonesianAGSBible.xml`, `IndonesianAMD2021Bible.xml`, `IndonesianBISBible.xml`, `IndonesianBPJBible.xml`, `IndonesianFAYHBible.xml`, `IndonesianINOTAZIBible.xml`, `IndonesianPBTB2Bible.xml`, `IndonesianTLBible.xml`, `IndonesianTSIBible.xml`, `IndonesianVMDBible.xml`.
- What each file states about itself in its `<bible status="...">` attribute, summarised: `IndonesianBible.xml` is the Terjemahan Baru and names the Indonesian Bible Society as holder, 1974 and 2018. BIS names the Indonesian Bible Society, 1985. FAYH carries Biblica's all-rights-reserved notice. AMD names Bible League International. VMD names World Bible Translation Center, "used with permission". TSI names Yayasan Alkitab Bahasa Kita and Pioneer Bible Translators. AGS says Creative Commons Attribution-ShareAlike 4.0. TL and PBTB2 name only the Indonesian Bible Society, BPJ names World Bible Translation Center. INOTAZI names LAI 2004.

So most of the Indonesian files are translations whose rights holders this page lists above as "no without written permission". The dialog does not decide that for the church. It shows the file's own statement on the card before the download button, and a download starts only on a press. The owner chose to offer the repository; whether a church may use a given translation stays the church's question, and the LAI email under "Recommendation" is still open. The "Never do these" list above predates this source and applies to what Artosia ships or fetches by itself, which is now the owner's decision for the Beblia list.

The format, from a downloaded Indonesian file (counts only, no text kept): UTF-8 with a byte order mark; root `<bible translation="..." status="..." link="...">`; `<testament name="Old|New">`; `<book number="1..66">` in canon order; `<chapter number>`; `<verse number>text</verse>`. A full Bible has 66 books, 1,189 chapters and 31,102 verses; the smallest Indonesian file (a New Testament) 27 books and 7,959 verse tags. No inline markup, no book names, no language attribute. `bible_file::parse_beblia` reads it, books past 66 are skipped, and `install_bytes` detects it by the `<bible>` root when a `.xml` file has no `<usfx>` root, so "Your own file" takes a Beblia XML too.

How the list is built (`beblia.rs`, `crates/artosia/src/bibles.rs`):

- One call to the unauthenticated git-tree API, `https://api.github.com/repos/Beblia/Holy-Bible-XML-Format/git/trees/master?recursive=1`, cached in `<library>/beblia-files.json` for a week and served stale when the network fails, like eBible's list. GitHub allows 60 such calls an hour per address.
- Only `<Language><Edition>Bible.xml` files in the root count. The language is the first capitalised word (`Indonesian`, `Chinese`, `Amharic`), the edition is the rest with spaces put between words and digits (`AGS`, `Tyndale 1537`, `Dawro DFBE`). A file with no edition is shown as the language alone.
- The list has no book counts, so a Beblia row shows the file size instead of "Whole Bible".
- When a row is picked, the first 4 KB of the file are fetched with a range request. The `<bible>` tag gives the translation's real name, which replaces the derived one in the row and the card, and the status line, which is the card's copyright text. A file with an empty status shows one plain string saying so. Nothing else is downloaded before the button is pressed.
- A download is `https://raw.githubusercontent.com/Beblia/Holy-Bible-XML-Format/master/<file>`, converted once to `<library>/bibles/<file stem in lower case>.sqlite3`; the file's name and status are stored in the module.
- A Beblia language that is also in eBible's list under the same English name joins its group (`Indonesian`). Languages of Indonesia in Beblia (Indonesian, Malaysian, Javanese, Sundanese, Balinese, Bugis, Aceh, Madurese, Sasak) sort with the other languages of Indonesia.
- If one source cannot be reached the other still shows; the dialog fails only when both fail.

Limits: the files carry no book names, so a slide reads "John 3:16" in English even for an Indonesian Beblia Bible; typing "Yoh 3 16" works. A name table per language is the fix and was not built.

## The name people know a Bible by (2026-11-12, owner-reported, board card NAMES1)

The owner looked for the Terjemahan Baru and could not find it. Three things hid it:

| What hid it | Where | What is true now |
|---|---|---|
| A row was named from its file name until it was picked, so `IndonesianBible.xml`, the Terjemahan Baru, was a row called "Indonesian" | the Bibles dialog and the first run | Every row shows the name its own file gives, with its short code beside it |
| The first run's Bible page opened where the country page had been scrolled to, in the middle of the alphabet, with the Indonesian Bibles out of view | the first run, in a real window only | A page with no row to reveal starts at its top |
| The first run's Bible page had no filter, and the dialog's filter did not take the keys when the dialog opened | both | Both have the filter, it takes what is typed at once, and it matches short codes and other names |

**The names file.** `tools/make-beblia-names.py` lists the repository's root with the git-tree API, fetches the first 4,096 bytes of each `*Bible.xml` with a range request and writes `crates/artosia-core/assets/beblia-names.json`, one entry per file: `{"name": ..., "code": ...}`. The name is the `<bible>` tag's `translation` attribute, or `language`, or `name` when the file uses those (162 of the 1,048 files do). The code is the last part of the bible.com address the file gives as its `link` (`.../MAT.4.TB` gives `TB`), so it is the code YouVersion shows for that translation. Run on 2026-11-12: 1,048 files, 1,045 named, 809 with a code, 102 KB. Three files give no name (`EnglishLSBBible.xml`, `Karakalpak2022Bible.xml`, `MakondeShiBible.xml`) and keep the label made from their file name. The file holds names and codes only, no verse and no copyright line; the copyright line is still read from the file itself when a row is picked. `beblia::known(file)` and `beblia::known_by_module(code)` read it. A file added to the repository after a release shows its file-name label until it is picked; run the tool again before a release.

**eBible's rows.** An eBible id is the language's three letters and then the translation's own (`indayt`), so `ebible::short_code` gives `AYT`, and the catalogue's `shortTitle` column is kept as another name.

**What the filter matches** (`ebible::grouped`): every typed word must be found in the language's English name, its own name, the title, the short code, the other names (eBible's short title; for a file of the repository its file-name label and stem, so "Indonesian BIS" still finds what is now shown as the Alkitab dalam Bahasa Indonesia Masa Kini), the id, the description or the language code. A language's heading keeps its own name ("bahasa Indonesia") when the filter leaves only the repository's files under it.

**On a row** the code is drawn in small strong type before the size or the extent (`ui::bibles::code_label`), and left out when the title already holds it ("Indonesian AGS (...)").

**The first run's Bible page** uses the dialog's own filter model (`Desk::query`), cleared whenever the dialog or the first run opens. Typing picks the first Bible the filter shows, so "tb" and Enter is the whole answer; words that match nothing give back the Bible that was picked before the typing began, never one the page no longer shows; a cleared filter keeps a picked Bible in view. The field has the keys when the page opens, as on the country page.

**An installed Bible** is called by one short code wherever it is named: the choices of Bible and Second Bible in Settings, the caption under a verse (`Bible::short`) and the main search. `App::list_bibles` decides it once per module, as `BibleFile::short`, in this order:

| # | Where the code comes from | Example |
|---|---|---|
| 1 | The module's own `short_code` row, written when a Bible is installed from 0.1.18 on | any Bible downloaded since |
| 2 | The names file, for a module that came from the repository before 0.1.18 (`bible::short_code`) | `indonesian.sqlite3` is `TB` |
| 3 | eBible's list as the library last saved it, for an eBible module from before 0.1.18 (`bibles::saved_short_codes`) | `indayt.sqlite3` is `AYT` |
| 4 | The module's code, its file name in capitals | a church's own file |

Two installed Bibles that would be called the same (eBible's and the repository's AGS) each keep their module's code, so the choices stay apart. Settings and plans keep the module's code as before, so nothing saved changes. Before this, Settings offered the Terjemahan Baru as a choice called INDONESIAN and a slide's caption said the same.

**In the main search** an installed Bible is found by that short code, by its module's code and by its file's own name (`palette.rs`).

**Checked** on 2026-11-12: `beblia::tests::a_bible_is_found_by_its_own_name_its_code_and_its_file_name`, `ebible::tests::a_short_code_and_a_second_title_are_read_and_found`, `palette::tests::an_installed_bible_is_found_by_its_short_code_and_its_own_name`, `bibles::tests::an_installed_bible_is_called_by_its_short_code` (made-up modules of each of the four kinds), `ui::onboard::bible_filter_tests`; renders of both screens light, dark and Indonesian and of Settings with several Bibles (scene `settings-bibles`); and real keys on the live catalogue in a window on an empty scratch library: the page opened on the Indonesian group, "tb" picked the Terjemahan Baru, "terjemahan baru" and "bimk" found theirs, "qqqq" showed the empty state with the earlier pick kept, and "tb" and "ags" filtered the Bibles dialog opened from the search. No Bible was downloaded in the check, so the caption on a slide is covered by the tests only.

## Sources

All read on 2026-11-11.
- https://github.com/Beblia/Holy-Bible-XML-Format (README, git tree, the `<bible>` tag of all eleven `Indonesian*` files, `IndonesianAGSBible.xml` and `IndonesianBible.xml` in full for the counts)

- https://ebible.org/Scriptures/translations.csv (full file; counts computed from it)
- https://ebible.org/find/ (redistribution wording, download formats)
- https://ebible.org/ccplus.htm (Bronze licence)
- https://ebible.org/indayt/copyright.htm, https://ebible.org/ind/copyright.htm, https://ebible.org/indags/copyright.htm
- HEAD requests to `https://ebible.org/Scriptures/<id>_usfx.zip` for 1,296 ids; `indayt_usfx.zip`, `_usfm.zip`, `_vpl.zip`, `_html.zip`, `_osis.zip`, `_sqlite.zip`
- https://ayt.co and https://ayt.co/hakcipta
- https://alkitab.or.id/tentang-kami/penggunaan-teks (LAI text policy), https://www.alkitab.or.id, https://bible-api.alkitab.or.id
- https://alkitab.mobi/copyright/ (SABDA's list of Indonesian translations and notices)
- https://copyright.sabda.org/
- https://tsibible.id/ (redirect target of albata.info; returned no readable text)
- https://crosswire.org/ftpmirror/pub/sword/raw/mods.d.tar.gz
- https://mybible.zone/ and https://mybible.zone/modules/, https://www.ph4.org/b4_index.php
- https://api.churchapps.org/content/bibles
- FreeShow, GPL-3.0 per `gh api repos/ChurchApps/FreeShow/license`: `src/frontend/converters/bible.ts`, `src/electron/data/bibleDetecter.ts`, `src/frontend/components/drawer/bible/scripture.ts`
- OpenLP: https://gitlab.com/openlp/openlp/-/tree/master/openlp/plugins/bibles/lib/importers and its `http.py`
- Quelea, GPL-3.0 per `gh api repos/quelea-projection/Quelea/license`: `Quelea/bibles/`, `BibleManager.java`
- In this repository: `crates/artosia-core/src/bible.rs`, `import.rs`, `crates/artosia/src/state.rs`, `actions.rs`, `strings.rs`, `palette.rs`, `crates/artosia-remote/src/page.html`, `docs/easislides-format.md`, `docs/i18n/README.md`, `tools/make-bible-asset.py`, `fixtures/easislides/HolyBibles/` (file names and `info` language only), `C:\Users\kenne\.claude\skills\blind-translation\SKILL.md`

## Not confirmed

- The TSI licence on the rights holder's own site. albata.info redirects to tsibible.id, which returned no readable text in the fetch tool or in Chrome. The CC BY-SA 4.0 wording comes from eBible's copyright page for the translation. Albata's own page (via a search result, not fetched) hints that its downloads are offered for personal use. Confirm with Albata before shipping TSI.
- TB2 and BIMK: LAI's text policy is the only wording found. No separate licence page for either.
- The status of TL (1954), Shellabear (1912) and Klinkert (1863). The pages name no current holder or only point to YLSA. Public-domain status was not established.
- Open Kitab Kehidupan (Biblica) at Biblica's own site, and whether it is downloadable in a usable format.
- Whether the LAI policy's "presentation slides" allowance covers an app that holds the whole TB offline on a church's computer.
- Which of the 1,296 eBible rows are non-commercial only. The CSV has no field for it.
- Whether eBible's list or an eBible mirror carries Sundanese, Batak or Javanese Bibles under another language code. Searched by name and dialect only.
- How FreeShow's `churchapps.org` list gets its Indonesian entries licensed, and its terms of use.
- OpenLP's and the other apps' licences beyond the file headers seen; no GitHub issue was cited.
- MyBible's own policy for its official module registry beyond the pages read.

## Checked after the merge (2026-11-11)

`cargo test -p artosia a_real_download -- --ignored` runs the road a press of Download takes against the real ebible.org: 1,296 translations listed, `indags` downloaded, converted and listed as a Bible in a scratch library. The button itself was not pressed in a real window: the test window could not be brought to the front while the PC was in use.

LAI's Bible API is still unbuilt. Its page links to Postman documentation, and both links answered 404 on 2026-11-11, from a plain fetch and in a browser. It needs a working documentation link and a key, both from LAI (info@alkitab.or.id).

