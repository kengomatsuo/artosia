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
