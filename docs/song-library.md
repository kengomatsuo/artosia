# Church song library (cleaned, 2026-11-10)

The owner's church library, cleaned from its EasiSlides export (1,481 items, 2026-10-09) into **1,335 items in OpenLyrics 0.9**: 1,308 songs, 19 announcements, 6 liturgy items, 2 empty placeholders. It is the real data Artosia's import, search and output are built and tested against.

**Location:** `fixtures/church/library/` (gitignored: real church data with copyrighted lyrics). Rebuild source and tools: `C:\Users\kenne\Shared\artosia-songs-staging\` (`in/` raw, `out/` cleaned, `tools/`).

| File | What it is |
|---|---|
| `songs/<id>.xml` | One song, OpenLyrics 0.9: `<verse name="v1|c|b|p|e|o..." lang="id|en|jv|zh...">`, one `<lines>` per slide, `verseOrder` in `<properties>` |
| `songs/<id>.json` | Per-song record: `kind`, `title`, `alt_titles`, `first_line`, `chorus_first_line`, `languages`, `source_tag`, every `changes` entry with before/after and reason, `flags`, `resolved_flags` |
| `index.json` | One row per item for search and the palette: title, alternate titles (including the titles of merged duplicate copies), first line, chorus first line, languages, links (`translation`, `part-of`, `medley`, `shares-chorus`, `related`), `merged_from` |
| `REPORT.md` | Change counts by type and what is still open (nothing) |
| `relations.json` | 206 decisions on duplicates, partial songs, medleys, translations, different songs sharing a title |
| `RULES.md`, `AUDIT.md` | The cleanup rulebook and the independent audit (80 songs, ~3% defect rate, every pattern then fixed library-wide) |

## Decisions baked into the data

- **One global theme.** Songs carry no styling; the 1,435 per-song EasiSlides fonts/backgrounds were dropped (OpenLyrics has no style). Artosia applies one theme to every song.
- **Words for God** follow EYD V rule 7: Tuhan, Allah, Yesus, Roh Kudus, Engkau, Kau, Dia capitalised; **`-Mu`/`-Nya` hyphenated and capitalised only when they refer to God** (kasih-Mu, nama-Nya). Scripture lines were checked against TB: e.g. Mazmur 37:24 "sebab TUHAN menopang tangannya" stays lowercase (the person's hand).
- **Sung elisions kept**, one spelling each (s'gala, t'lah, s'bab, kar'na...), straight apostrophe only; real words anugrah, samudra, takkan, seorangpun, cahya stay plain.
- **Sections** are labelled for keyboard shortcuts (`v1..vN`, `c`/`c1`, `p`, `b`, `e`, `o`); repeated choruses stored once and repeated through `verseOrder`.
- **Slides**: at most 4 lines, sizes within a section differ by at most 1 line, split by musical phrase.
- **Multilingual** songs hold each language as the same verse name with a different `lang`.
- **Titles**: typos fixed; different songs that shared a title carry a distinguishing lyric line in brackets ("Sukacita (Dan kakiku menari)"); attribution tags (TW, GMS, JPCC, Glenn) moved to `source_tag` and kept as an alternate title.
- **`di` + place word** written apart (di dalam, di hadapan), per EYD.

## What the data says for the product

- 1,232 songs are Indonesian only: search, typo tolerance and stemming must work for Indonesian first; the palette should match title, alternate titles, first line and chorus first line, and treat `s'gala`/`segala`/`sgala` as one word.
- Most slides are 1–2 lines; the output must look right with a single short line.
- Nothing is open. Every doubt was settled by research: 122 against a published lyric, Bible verse or KBBI entry (URL in the song's `resolved_flags`), 89 structural, and 5 church-only items where no published copy exists anywhere (searches recorded). All 206 song relations are confirmed by a source or by line-by-line lyrics.

## Re-checking

`python tools/validate.py all` (1,481 cleaned songs, 0 errors) and `python tools/verify_final.py` (final set, links, unique titles, 0 errors), both in the staging folder.
