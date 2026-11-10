# EasiSlides 5.2 data format (read from a real install, 2026-11-10)

EasiSlides keeps everything under `Documents\EasyslidesData\`. A copy of the `Admin` folder from this PC is in `fixtures/easislides/` (gitignored: the song database may hold copyrighted lyrics). The app is C# WinForms on .NET Framework 4.8 using `System.Data.SQLite`.

| File | Format | What it holds |
|---|---|---|
| `Admin/Databases/<name>.db` (here `English.db`, 961 songs) | SQLite 3 | table `items`, one row per song |
| `Admin/HolyBibles/*.SQLite3` (asv, cuvs, cuvt, kjv, rsv) | SQLite 3, MyBible layout | `books(book_number, short_name, long_name, book_color, sorting_order)`, `verses(book_number, chapter, verse, text)`, `info(name, value)` |
| `Admin/Schedules/*.esw` | XML (UTF-8 BOM), root `<Easyslides>`, `<ListItem>` entries with `<FormatData>` | a service schedule with per-item formatting |
| `Admin/Booklets/*.esp` | XML | booklet generator project |
| `Admin/Settings/DatabasesAdmin.db` | SQLite | per-database settings: section headings, fonts |
| `Admin/Settings/Usages.db` | SQLite | CCLI usage log: date, song title, author, CCLI number |
| `Admin/UserProfiles/<machine>-<user>.xml` | XML | per-user settings |

## `items` (songs)

Primary key `(title1, author)`. Columns that matter for import: `title1`, `title2` (second-language title), `songnumber`, `contents`, `sequence`, `author`, `copyright`, `capo`, `songkey`, `timing`, `notations`, `category`, `bookreference`, `userreference`, `cclisongnumber`, `formatdata`, `lastmodified`, `foldername`. The `*normalized` columns are EasiSlides' own search copies.

`contents` is plain text with numbered section markers on their own line, e.g.:

```
[1]
Amazing grace! How sweet the sound
That saved a wretch like me;
...
[2]
'Twas grace that taught my heart to fear,
```

Bible book numbers follow MyBible (`500` = John), so `500,3,16` is John 3:16.

Next for the importer: read the section-marker set beyond `[1]`..`[n]` (chorus, bridge, second-language regions) across all 961 rows, and the `sequence` and `formatdata` grammar.

## The owner's church export (`Export_2026-10-09.xml`, 1,481 songs)

Kept in `fixtures/church/` (gitignored: real church data with copyrighted lyrics). This is EasiSlides' **Export** format, different from its database: root `<EasiSlides>`, one `<Item>` per song with `Title1`, `Title2`, `Folder`, `SongNumber`, `Contents`, `Notations`, `Sequence`, `Writer`, `Copyright`, `Category`, `Timing`, `MusicKey`, `Capo`, `LicenceAdmin1/2`, `BookReference`, `UserReference`, `FormatData`, `Settings`.

What the real data looks like (counted 2026-11-10):

| Finding | Count | What it means for Artosia |
|---|---|---|
| Songs with Indonesian lyrics (Tuhan, Yesus, kasih, Engkau, hatiku) | 1,196 of 1,481 | Search, typo tolerance and stemming must work for Indonesian first; English second |
| Slides split only by blank lines, no `[markers]` | 819 | The importer treats a blank line as a slide break |
| `[chorus]` markers | 558 (+1 `[Chorus]`) | Marker match is case-insensitive |
| Other markers | `[1]` 9, `[region 2]` 6, `[bridge]` 2, `[2]` 2 | `[region 2]` starts the second text region (here a sub-caption, not a translation) |
| `Sequence`, `Notations`, `SongNumber`, `Writer` filled | 0, 0, 0, 1 | The church never used verse order, chords or numbers; search by title and lyric text is everything |
| `Title2` filled | 10 | Second titles are rare |
| Duplicate titles after normalising | 75 | Import needs a merge/duplicate review step |
| Lines per slide | mostly 1–2 (3,400 and 3,513 slides), long tail to 10+ | Short slides; the output must look right with one line |
| Slides per song | median 6, max 24 | |
| Per-song `FormatData` (34 `key=value>` pairs) | on 1,435 songs | Fonts (`43`: Segoe UI Black 930, Segoe UI 295, Maiandra GD 111), backgrounds (`61`: absolute paths like `C:\EasiSlides\Images\Scenery\sky1.jpg`), transitions (`72`: None 1,255, Fade 77, Rectangle 69) are set per song. The importer maps the common ones to a handful of themes instead of 1,435 copies. |
| Folders | Folder 1: 1,042; 2: 203; 3: 168; 6: 43; 4: 15; 5: 10 | Folder names are generic; categories come from the folder number only |
