# artosia-office

Reads a church's own office files with no PowerPoint or Word installed, no UI and no C
dependency (`zip` and `quick-xml` only). Both formats are zips of XML.

| Module | Reads | Gives |
| --- | --- | --- |
| `Pptx` | .pptx | slides in presentation order: title, paragraphs, tables, pictures with their place |
| `Docx` | .docx | paragraphs and tables in document order |
| `birthdays` | tables from either | `Vec<Birthday { name, day, month, year }>` (year 0 when unknown), `in_range`, `in_month`, `dedup` |
| `xlsx_table`, `csv_from_bytes` | .xlsx, .csv | the first sheet or the file as rows of cell text; a date held as a number comes out as `1990-01-05` |
| `guess`, `read` | rows of a table | which column is name, date, day, month or year (`Mapping`), and the people under a mapping |
| `to_announcements` | a read `Pptx` | `Vec<Draft { title, body, picture, .. }>` for Artosia's announcements |
| `Picture::drawable` | any picture | PNG or JPEG bytes Artosia can draw |
| `slides_as_pictures` | a .pptx | each visible slide as a PNG (`Exported { tool, files }`), drawn by the installed PowerPoint, else LibreOffice (one `soffice` run per slide, untested here); `Error::NoRenderer` when neither is installed, `Error::Export` on failure or a time limit |
| `redraw`, `classify` | a bulletin's tables | per table `Birthdays(Found)`, `Money { gifts: Vec<Gift>, printed_total }` or `Other`; a money table needs a header naming an amount and a giver or fund column; `parse_rupiah` reads `Rp 2.500.000`, `2.500.000,-`, `Rp2,500,000` |

```bash
cargo run -p artosia-office --example read -- bulletin.pptx birthdays.docx target/out
```

The example prints counts only (slides, rows, people per month), never text, so it is safe on a
real congregation's files.

## What is read

- **Slide order** comes from `ppt/presentation.xml` (`sldIdLst`) and its relationship file, never
  from `slideN.xml` numbering. Media and layouts are found through each slide's `.rels`.
- **Paragraphs**: runs (`a:r`) are joined, so a word split over several runs is one word. A soft
  break (`a:br`) becomes `\n`. Empty paragraphs are dropped. Footer, date and slide-number
  placeholders are dropped.
- **Reading order**: shapes sorted top to bottom in bands of 2% of the slide height, then left to
  right. A placeholder with no box of its own takes its layout's box, then its master's. Shapes
  inside groups are placed through the group's transform. Shapes with no position keep their XML
  order, after the rest.
- **Tables** (`a:tbl`): rows of cell text; a merged cell leaves its continuation cells empty so
  columns stay aligned.
- **Pictures** (`p:pic`, including the preview picture of an embedded object such as a pasted
  spreadsheet): file name, bytes, detected format (by magic bytes), and the box as fractions of
  the slide.
- **docx**: `w:p` and `w:tbl` in the body, through content controls and links; tab is `\t`, break
  is `\n`, tracked deletions are left out.

## Birthdays

A date cell is a day plus a month in Indonesian or English (full and short forms), also `05/01`,
`5-1`, `1990-01-05` and `5 Jan 1990`. A four-digit year of birth is kept (0 when there is none or it is doubtful); no slide carries it. Day comes before month in numbers. A dot
is not a separator, so an amount like `10.5` is never a date. Typos survive when they are one
letter off a long month name and cannot mean another month (`Nopvember`); a five-digit year typo
is accepted and dropped. A row with a name and a date in one cell (`Name 12 Nov`) reads too, and a
bare day under a month heading in a docx takes the heading's month.

`Found` counts what it skipped: header rows (`headers`), rows with content but no name or date
(`unread`). Empty rows and rows holding only a number (unfilled slots) are skipped silently.

`from_*` does not remove repeats. Office lists often hold a person twice; call `dedup`.
`in_range` wraps over New Year and sorts from the start date onward (Dec 30 before Jan 2).

## Announcements

Artosia's announcements hold a title and lines. `to_announcements` makes one `Draft` per slide:

- title: the title placeholder; with none, a first text line of 60 characters or fewer; else empty
- body: paragraphs and tables in reading order, one line each; a table row is one line with its
  cells joined by a TAB (trailing empty cells trimmed)
- a slide with no text becomes a draft with its first picture

**Lost on purpose**: fonts, colours, sizes, bold, bullets and list levels, exact positions,
animations, transitions, backgrounds, shapes without text, charts and diagrams, speaker notes. A
picture on a slide that also has text is not carried (counted in `dropped_pictures`). To show a
deck exactly as drawn, use the PowerPoint route below.

## EMF and WMF pictures (decision)

The bulletin's picture is an EMF (the preview of a pasted spreadsheet). Artosia draws PNG and
JPEG, so `Picture::drawable(width)` converts anything else.

**Chosen: GDI+ through Windows PowerShell** (`powershell.exe`, `System.Drawing`), in
`picture.rs`. It draws the metafile onto a white bitmap of the asked width and saves a PNG. It
needs nothing beyond the OS, adds no dependency and no `unsafe`, and uses the very fonts the
file names. Costs: a process start per picture (a few hundred ms, paid once at import) and
Windows only; on other systems it returns `ConvertError::UnsupportedPlatform`. The background is
white, not clear, so a QR code stays scannable. It also converts BMP, GIF and TIFF.

Tried and kept out: **`emf-core` 0.1.1** (MIT, pure Rust, EMF and EMF+ to SVG, falls back to
`wmf-core`) plus **`resvg`**. On the real file it got every line and all 40 text runs right, but
the text came out in a fallback serif because the metafile's font is looked up by name in
whatever fonts the machine has, and one label overflowed its cell. It works as a fallback for a
future macOS or Linux build; it adds a large dependency tree (resvg, fontdb, tiny-skia) and is
young ("some EMF records are not yet fully implemented", its README), so it is not in this
crate. Other pure-Rust names found: `emfsdk`, `betteroffice-metafile`; not tried.

Checked on the real file: the PNG shows a numbered table of budget lines with currency amounts
and a bold total row, on a faint spreadsheet grid.

## PowerPoint on this PC and exact slides through COM

Researched, nothing automated.

- PowerPoint is installed: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\POWERPNT.EXE`
  points to `C:\Program Files\Microsoft Office\Root\Office16\POWERPNT.EXE`, the file exists, and
  the `PowerPoint.Application` COM class is registered. File version 16.0.20527.20150; Click-to-Run
  release `HomeStudent2021Retail`. A church PC may not have it, so the feature must test for it.
- The route: start PowerPoint through COM (`PowerPoint.Application`), open the file, export, close.
  Microsoft's own pages (fetched in this session):
  - [`Presentations.Open`](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentations.open):
    `Open(FileName, ReadOnly, Untitled, WithWindow)`. `ReadOnly:=msoTrue` leaves the file alone;
    `WithWindow:=msoFalse` hides the opened presentation.
  - [`Slide.Export`](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.slide.export):
    `Export(FileName, FilterName, ScaleWidth, ScaleHeight)`; `FilterName` is a registered
    extension such as `"PNG"`; the scale arguments are pixels.
  - [`Presentation.Export`](https://learn.microsoft.com/en-us/office/vba/api/powerpoint.presentation.export):
    `Export(Path, FilterName, ScaleWidth, ScaleHeight)` does every slide into a folder, named by
    PowerPoint itself (typically `Slide1.png`, `Slide2.png`).
  - Exporting does not mark the presentation as changed.
- Example, for the later task (Microsoft's `Slide.Export` shape, in PowerShell):
  `$p = (New-Object -ComObject PowerPoint.Application).Presentations.Open($path, -1, 0, 0)`
  then `$p.Export($folder, 'PNG', 1920, 1080)`, `$p.Close()`.
- Unverified: whether a hidden window (`WithWindow` false) is honoured by this build; that the
  export keeps fonts the deck needs when they are missing; and how long a first start takes.
  PowerPoint also shows a window on first run or when licensing needs attention, so the app must
  time out and fall back to `to_announcements`.
