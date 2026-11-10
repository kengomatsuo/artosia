# artosia-lyrics

Finds a song's words on the web for the "New song" screen. Researched and built 2026-11-10; every claim below comes from a page or an answer fetched that day.

```rust
pub fn search(query: &str) -> Result<Vec<Found>, LyricsError>;   // suggestions, best first
pub fn fetch(found: &Found) -> Result<Fetched, LyricsError>;     // plain stanzas, blank-line separated
```

`cargo run -p artosia-lyrics --example find -- "<query>" --fetch N` prints matches and, for match N, stanza and line counts only.

## Sources judged

Coverage was tested with 30 titles from `fixtures/church/library/index.json`: 10 English and 20 Indonesian songs picked at random (seed 11, `languages` = `en` or `id` only). "Found" means a result whose title equals the typed title after lowercasing and dropping punctuation, and that carries words. Counts and titles only; no lyric text was kept.

| Source | Access | Terms read | Found, of 30 (EN of 10 / ID of 20) | Verdict |
|---|---|---|---|---|
| LRCLIB (`lrclib.net`) | Keyless HTTPS JSON. `GET /api/search?q=` (up to 20 records, words included, no paging), `GET /api/get/{id}`. Plain and time-synced text | Docs at `/docs.md`: key optional, identify the app in `User-Agent`; about 45 calls/min per IP unidentified, about 90 identified; `429` with `Retry-After`, honour it; 200-500 ms between calls. No terms for the data itself are published on the pages fetched | 9 / 30 (7 / 10 EN, 2 / 20 ID) | **Used.** Only source that is documented, keyless and open |
| GEMA SABDA (`gema.sabda.org`, Yayasan Lembaga SABDA) | Web pages only, no API, no export. `robots.txt` disallows `/search/`, so only a 38-page alphabetical index (about 1,900 pages) can be walked | Footer: "All Rights Reserved", no reuse terms | 8 / 30 by title in the index (2 / 10 EN, 6 / 20 ID) | **Not built.** Best Indonesian coverage seen, but scraping a rights-reserved site without permission is not something Artosia can call lawful, and 38 requests per lookup is unusable. Ask YLSA for permission or an export; until then link-out only |
| lyrics.ovh | `GET /v1/{artist}/{title}` needs the artist; `/suggest` proxies Deezer | README: MIT licence for the code. It queries LRCLIB first, then up to nine other sources (it names NetEase, Genius, AZLyrics) and says lyrics sites often block datacenter IPs | not run (no title-only lookup) | **Rejected.** A scraper of third-party sites, so it launders rights instead of clearing them, and it needs an artist the volunteer rarely types |
| Musixmatch Pro API | Key and paid plan | Pricing page: Basic is $49/month, 500 lyrics calls/day, static lyrics, caching not included | not run | Rejected: paid, key required |
| Genius API | Token | `docs.genius.com` answered with a Cloudflare human check, which was not bypassed | not read | Not evaluated; needs a token in any case |
| CCLI SongSelect | Partner API only | See below | not applicable | Closed to us |
| hymnary.org | `hymnary.org/help/api` returned 404 | none read | not run | Skipped; public-domain English hymns are in LRCLIB anyway |

### CCLI SongSelect

- ccli.com/us/en/songselect (fetched): SongSelect holds 260,000+ worship songs. "All Church Copyright License holders enjoy free access to lyrics on SongSelect", and the page says SongSelect connects to presentation software through "3rd-party integration". Without a licence only public-domain songs are open.
- CCLI's own pages carry no developer sign-up. `ccli.com/us/en/songselect-api` is a 404, `developer.ccli.com` does not resolve, and `status.ccli.com` calls itself a page "for CCLI Partners" and lists `api.ccli.com` as the partner API.
- The only statement that new partners are refused is an unattributed, undated note in a third-party repository ("CCLI has retired the SongSelect API Partner Program and is no longer accepting new API partners"). **I could not confirm this on a CCLI page.** OnSong's manual (via search) says the v1 API was withdrawn on 2023-06-26. Treat SongSelect as unavailable to Artosia unless CCLI answers a direct request.
- `ccli.com/id/en` is a 404, so no Indonesian CCLI site was found in this session. The licence terms there are not known; the volunteer must check with the licence body that covers the church.

### Indonesian equivalents

No Indonesian church lyric database with an API or export turned up. News and parenting sites print lyrics in articles; SABDA runs GEMA (above) and the Kidung Jemaat and NKB hymn pages, with no reuse terms. LRCLIB has almost no Indonesian worship songs (2 of 20), which is the plain result for this church: most of the library will not be found.

## What copyright and licensing require

- Lyrics are copyrighted. CCLI's own page says the Church Copyright License covers projecting lyrics from a computer, printing them and translating songs. A church projects these songs legally only under such a licence (or for public-domain songs).
- LRCLIB is community-published and does not clear rights. Its docs offer a `/api/flag` route for copyright concerns and state no terms for using the data.
- Artosia **never bundles lyrics**. Words are fetched on request, copied into the church's own library by the volunteer, and not redistributed.

## What the app must show

1. In the suggestion list and next to the filled-in song: the source name and link ("LRCLIB", `Found.url`).
2. A fixed line near the Add button, once per lookup: "Lyrics come from a community database. Your church needs a licence, such as CCLI, to project them."
3. A reminder to check the words against the church's own copy, because versions and spellings differ.
4. The error text from `LyricsError`'s `Display`; it is written for volunteers.

## Recommendation

1. Query LRCLIB. It is the only source used.
2. Order: exact title first, then title starts with the query, then contains it, then the rest; repeats of the same title and artist are merged.
3. Add GEMA SABDA only after YLSA agrees in writing; add SongSelect only if CCLI opens partner access. `lrclib.rs` is the single place a source lives, so a second one means a second module and a merge in `search`.
4. Call `search` after a 400 ms pause in typing, one call at a time, and `fetch` only on a pick. The crate also spaces calls at least 250 ms apart and refuses calls until a `Retry-After` has passed.

## Dependency

`ureq` 3.5.1 (crates.io, updated 2026-11-02, about 261 million downloads): blocking, no async runtime, `unsafe` forbidden, `timeout_global` covers the whole call including the body. Default TLS is rustls with the `ring` provider and bundled `webpki-roots`, so it builds on Windows with MSVC and needs no OpenSSL; verified here with a clean build. `serde` and `serde_json` read the answers. Alternatives seen: `attohttpc` 0.32 (fewer users) and `minreq` 3.0.

## What a volunteer will hit

- **Indonesian songs mostly miss.** 18 of 20 sampled Indonesian titles had no match; hand typing or paste remains the main path.
- **Same title, other song.** LRCLIB matches titles, not lyrics. "Hosanna", "Joy to the World" and "Allah Peduli" each return several different songs and artists, and the first may not be the church's version. The list shows artist so the volunteer can choose, and `fetch` returns what the source holds; LRCLIB does not index words, so a first-line search mostly finds nothing (only 5 of 20 Indonesian first lines returned anything; I did not check whether those were relevant).
- **No stanza breaks.** Some records arrive as one block (one test song gave 46 lines, 1 stanza). `song_from_paste` then makes a single section; the volunteer splits it.
- **Rate limit.** About 90 calls a minute; a debounced field never gets near it. A `429` shows "The lyrics service is busy" and the crate stops calling until the wait ends.
- **Offline or slow.** After 5 seconds the call fails with a readable network message.
- **Time-synced only records** are converted to plain text with timestamps removed.
