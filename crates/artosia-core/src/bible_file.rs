//! Bringing a Bible file in: MyBible, USFX and USFM become one MyBible module in the library.
//!
//! Artosia's reader loads MyBible modules only. A church's own Bible arrives as a MyBible file,
//! as USFX (one XML file, what eBible publishes as `<id>_usfx.zip`) or as a zip of USFM files
//! (Paratext's text format). Reading converts once; the module is written into the library's
//! `bibles` folder and never converted again. Footnotes, cross references and headings are
//! dropped: a slide shows verse text.

use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;

use crate::bible::{BibleError, Verse, describe, mybible_number};

/// The three-letter book codes of USFM and USFX, in canonical order.
const CODES: [&str; 66] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH", "2CH", "EZR", "NEH", "EST", "JOB", "PSA",
    "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK", "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC",
    "MAL", "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH", "2TH", "1TI", "2TI", "TIT", "PHM",
    "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD", "REV",
];

/// The largest file read out of a zip, so a hostile archive cannot fill the memory.
const LIMIT: u64 = 96 * 1024 * 1024;

fn book_of(code: &str) -> Option<u8> {
    let code = code.trim().to_ascii_uppercase();
    // Older files spell three books differently.
    let code = match code.as_str() {
        "SOS" => "SNG",
        "EZE" => "EZK",
        "JOE" => "JOL",
        "NAH" => "NAM",
        other => other,
    };
    CODES.iter().position(|known| *known == code).map(|at| at as u8 + 1)
}

/// Verses and the book names a file carried.
#[derive(Default)]
pub struct Parsed {
    pub verses: Vec<Verse>,
    /// (book 1..=66, long name, short name).
    pub names: Vec<(u8, String, String)>,
    /// What the file says about itself, empty when it says nothing.
    pub name: String,
    pub language: String,
    /// The copyright or status line the file carries, empty when it carries none.
    pub copyright: String,
}

#[derive(Debug)]
pub enum ImportError {
    Read(String),
    /// The file is not a MyBible, USFX or USFM Bible.
    Unsupported,
    /// A Bible was found but holds no verses.
    Empty,
    Write(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Read(why) => write!(f, "could not read the file: {why}"),
            ImportError::Unsupported => write!(f, "it is not a MyBible, USFX or USFM Bible"),
            ImportError::Empty => write!(f, "it holds no Bible verses"),
            ImportError::Write(why) => write!(f, "could not save the Bible: {why}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Accumulates the verse being read.
#[derive(Default)]
struct Collector {
    book: Option<u8>,
    chapter: u16,
    verse: u16,
    text: String,
    out: Vec<Verse>,
}

impl Collector {
    fn flush(&mut self) {
        let body = self.text.split_whitespace().collect::<Vec<_>>().join(" ");
        self.text.clear();
        let (Some(book), true) = (self.book, self.verse > 0 && self.chapter > 0) else { return };
        if body.is_empty() {
            return;
        }
        // A bridged verse or a split one ("1a", "1b") lands on one verse.
        if let Some(last) = self.out.last_mut()
            && (last.book, last.chapter, last.verse) == (book, self.chapter, self.verse)
        {
            last.text = format!("{} {body}", last.text).into_boxed_str();
            return;
        }
        self.out.push(Verse { book, chapter: self.chapter, verse: self.verse, text: body.into_boxed_str() });
    }

    fn start_verse(&mut self, id: &str) {
        self.flush();
        let digits: String = id.chars().take_while(char::is_ascii_digit).collect();
        self.verse = digits.parse().unwrap_or(0);
    }

    fn start_chapter(&mut self, id: &str) {
        self.flush();
        self.chapter = id.trim().parse().unwrap_or(0);
        self.verse = 0;
    }

    fn start_book(&mut self, code: &str) {
        self.flush();
        self.book = book_of(code);
        self.chapter = 0;
        self.verse = 0;
    }

    fn push(&mut self, text: &str) {
        if self.verse > 0 && self.book.is_some() {
            self.text.push_str(text);
        }
    }
}

/// Elements whose words are part of the verse and need no space around them.
const INLINE: [&str; 24] = [
    "w", "wj", "add", "nd", "qt", "it", "bd", "bdit", "em", "sc", "k", "tl", "sig", "bk", "ord", "pn", "wg", "wh", "wa", "rq", "dc", "sup",
    "no", "ndx",
];

/// Elements whose content is not verse text.
const SKIPPED_XML: [&str; 15] = ["f", "x", "fe", "fig", "h", "toc", "id", "ide", "s", "r", "rem", "d", "ca", "va", "vp"];

fn attribute(start: &quick_xml::events::BytesStart<'_>, name: &str) -> String {
    start.try_get_attribute(name).ok().flatten().map(|attribute| attribute.value.to_string()).unwrap_or_default()
}

/// Reads one USFX document (https://ebible.org/usfx/), the XML eBible publishes.
pub fn parse_usfx(xml: &str) -> Result<Parsed, ImportError> {
    let xml = xml.trim_start_matches('\u{feff}');
    if !xml.contains("<usfx") {
        return Err(ImportError::Unsupported);
    }
    let mut reader = Reader::from_str(xml);
    let mut collector = Collector::default();
    // Depth inside a skipped element, and the names of the open ones.
    let mut skipping = 0usize;
    let mut open: Vec<String> = Vec::new();
    loop {
        let event = reader.read_event().map_err(|error| ImportError::Read(error.to_string()))?;
        match event {
            Event::Start(start) => {
                let name = start.local_name().as_ref().to_string();
                if skipping > 0 {
                    skipping += 1;
                } else if name == "book" {
                    collector.start_book(&attribute(&start, "id"));
                } else if SKIPPED_XML.contains(&name.as_str()) {
                    skipping = 1;
                } else if !INLINE.contains(&name.as_str()) {
                    collector.push(" ");
                }
                open.push(name);
            }
            Event::Empty(start) => {
                if skipping > 0 {
                    continue;
                }
                match start.local_name().as_ref() {
                    "c" => collector.start_chapter(&attribute(&start, "id")),
                    "v" => collector.start_verse(&attribute(&start, "id")),
                    "ve" => {
                        collector.flush();
                        collector.verse = 0;
                    }
                    "p" | "q" | "b" => collector.push(" "),
                    _ => {}
                }
            }
            Event::Text(content) => {
                if skipping == 0 {
                    collector.push(&content);
                }
            }
            Event::GeneralRef(entity) => {
                if skipping == 0 {
                    if let Ok(Some(character)) = entity.resolve_char_ref() {
                        collector.push(&character.to_string());
                    } else if let Some(value) = resolve_predefined_entity(&entity) {
                        collector.push(value);
                    }
                }
            }
            Event::End(_) => {
                let name = open.pop().unwrap_or_default();
                if skipping > 0 {
                    skipping -= 1;
                } else if name == "book" {
                    collector.flush();
                    collector.book = None;
                } else if !INLINE.contains(&name.as_str()) {
                    collector.push(" ");
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    collector.flush();
    if collector.out.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(Parsed { verses: collector.out, ..Parsed::default() })
}

/// What a Beblia XML file says about itself in its `<bible>` tag: (translation name, status line).
/// Beblia's repository publishes one XML file per Bible (https://github.com/Beblia/Holy-Bible-XML-Format).
/// Works on the first kilobytes of a file, so a copyright line can be read before the download.
/// None when the text is not a Beblia file.
pub fn beblia_header(xml: &str) -> Option<(String, String)> {
    let mut reader = Reader::from_str(xml.trim_start_matches('\u{feff}'));
    loop {
        match reader.read_event().ok()? {
            Event::Start(start) | Event::Empty(start) => {
                if start.local_name().as_ref() != "bible" {
                    return None;
                }
                let read = |name: &str| {
                    let value = start.try_get_attribute(name).ok().flatten().and_then(|attribute| attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|value| value.into_owned()));
                    value.unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ")
                };
                return Some((read("translation"), read("status")));
            }
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// Reads one Beblia XML Bible: `<bible translation status><testament><book number><chapter number>
/// <verse number>text</verse>`. A book's number is its place in the 66-book canon; books past 66
/// are skipped.
pub fn parse_beblia(xml: &str) -> Result<Parsed, ImportError> {
    let xml = xml.trim_start_matches('\u{feff}');
    let Some((name, copyright)) = beblia_header(xml) else { return Err(ImportError::Unsupported) };
    let mut reader = Reader::from_str(xml);
    let (mut book, mut chapter, mut verse) = (0u8, 0u16, 0u16);
    let mut inside = false;
    let mut text = String::new();
    let mut out: Vec<Verse> = Vec::new();
    let number = |start: &quick_xml::events::BytesStart<'_>| attribute(start, "number").trim().parse::<u32>().unwrap_or(0);
    loop {
        let event = reader.read_event().map_err(|error| ImportError::Read(error.to_string()))?;
        match event {
            Event::Start(start) => match start.local_name().as_ref() {
                "book" => book = u8::try_from(number(&start)).ok().filter(|n| (1..=66).contains(n)).unwrap_or(0),
                "chapter" => chapter = u16::try_from(number(&start)).unwrap_or(0),
                "verse" => {
                    verse = u16::try_from(number(&start)).unwrap_or(0);
                    inside = true;
                    text.clear();
                }
                _ => {}
            },
            Event::Text(content) if inside => text.push_str(&content),
            Event::GeneralRef(entity) if inside => {
                if let Ok(Some(character)) = entity.resolve_char_ref() {
                    text.push(character);
                } else if let Some(value) = resolve_predefined_entity(&entity) {
                    text.push_str(value);
                }
            }
            Event::End(end) => match end.local_name().as_ref() {
                "verse" if inside => {
                    inside = false;
                    let body = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if book > 0 && chapter > 0 && verse > 0 && !body.is_empty() {
                        out.push(Verse { book, chapter, verse, text: body.into_boxed_str() });
                    }
                }
                "book" => book = 0,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    if out.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(Parsed { verses: out, names: Vec::new(), name, language: String::new(), copyright })
}

/// The long book names in eBible's `BookNames.xml`, as (book, name).
fn parse_book_names(xml: &str) -> Vec<(u8, String, String)> {
    let mut reader = Reader::from_str(xml.trim_start_matches('\u{feff}'));
    let mut names = Vec::new();
    while let Ok(event) = reader.read_event() {
        match event {
            Event::Empty(start) | Event::Start(start) if start.local_name().as_ref() == "book" => {
                let (code, long) = (attribute(&start, "code"), attribute(&start, "long"));
                if let (Some(book), false) = (book_of(&code), long.trim().is_empty()) {
                    names.push((book, long.trim().to_string(), long.trim().to_string()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    names
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(&text[start..end])
}

/// The name and language a Digital Bible Library `metadata.xml` gives.
fn parse_metadata(xml: &str) -> (String, String) {
    let name = between(xml, "<nameLocal>", "</nameLocal>").or_else(|| between(xml, "<name>", "</name>")).unwrap_or("");
    let language = between(xml, "<language>", "</language>").and_then(|part| between(part, "<name>", "</name>")).unwrap_or("");
    (name.trim().to_string(), language.trim().to_string())
}

/// Markers whose whole line is not verse text: headings, titles, introductions and notes.
fn skips_line(marker: &str) -> bool {
    let base = marker.trim_end_matches(|c: char| c.is_ascii_digit());
    matches!(
        base,
        "ide" | "h" | "toc" | "mt" | "mte" | "ms" | "mr" | "s" | "sr" | "r" | "rem" | "d" | "sp" | "cl" | "cd" | "cp" | "ip" | "ipi" | "im"
            | "imi" | "ipq" | "imq" | "ipr" | "iq" | "ili" | "ib" | "iot" | "io" | "iex" | "imt" | "imte" | "ie" | "is" | "sts" | "restore"
            | "periph"
    )
}

/// Reads USFM text into `parsed`. One file is usually one book, named by its `\id` line.
pub fn parse_usfm(text: &str, parsed: &mut Parsed) {
    let text = text.trim_start_matches('\u{feff}');
    let mut collector = Collector::default();
    let (mut long, mut short) = (String::new(), String::new());
    let mut skip_until: Option<String> = None;
    let mut in_word = false;
    let mut at = 0usize;
    let finish_names = |collector: &Collector, long: &mut String, short: &mut String, parsed: &mut Parsed| {
        if let Some(book) = collector.book
            && !long.is_empty()
            && !parsed.names.iter().any(|entry| entry.0 == book)
        {
            let short_name = if short.is_empty() { long.clone() } else { short.clone() };
            parsed.names.push((book, long.clone(), short_name));
        }
        long.clear();
        short.clear();
    };
    while at < text.len() {
        let rest = &text[at..];
        let Some(slash) = rest.find('\\') else {
            if skip_until.is_none() {
                collector.push(if in_word { rest.split('|').next().unwrap_or("") } else { rest });
            }
            break;
        };
        if skip_until.is_none() {
            let chunk = &rest[..slash];
            collector.push(if in_word { chunk.split('|').next().unwrap_or("") } else { chunk });
        }
        at += slash + 1;
        let after = &text[at..];
        let plus = after.starts_with('+');
        let start = usize::from(plus);
        let length = after[start..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).unwrap_or(after.len() - start);
        let marker = &after[start..start + length];
        at += start + length;
        let closing = text[at..].starts_with('*');
        if closing {
            at += 1;
        }
        if let Some(waiting) = &skip_until {
            if closing && waiting == marker {
                skip_until = None;
            }
            continue;
        }
        if closing {
            if matches!(marker, "w" | "wg" | "wh" | "wa") {
                in_word = false;
            }
            continue;
        }
        // The single space after a marker belongs to the marker.
        let value_start = text[at..].strip_prefix(' ').map_or(at, |_| at + 1);
        let line_end = |from: usize| text[from..].find('\n').map_or(text.len(), |n| from + n);
        match marker {
            "id" => {
                finish_names(&collector, &mut long, &mut short, parsed);
                collector.start_book(text[value_start..line_end(value_start)].split_whitespace().next().unwrap_or(""));
                at = line_end(value_start);
            }
            "c" => {
                let end = line_end(value_start);
                collector.start_chapter(text[value_start..end].split_whitespace().next().unwrap_or(""));
                at = end;
            }
            "v" => {
                let token = text[value_start..].split_whitespace().next().unwrap_or("");
                collector.start_verse(token);
                at = value_start + token.len();
            }
            "toc1" | "toc2" | "h" => {
                let end = line_end(value_start);
                let value = text[value_start..end].trim().to_string();
                match marker {
                    "toc1" => long = value,
                    _ if short.is_empty() => short = value,
                    _ => {}
                }
                at = end;
            }
            "f" | "fe" | "x" | "fig" | "va" | "vp" | "ca" => skip_until = Some(marker.to_string()),
            "w" | "wg" | "wh" | "wa" => in_word = true,
            _ if skips_line(marker) => at = line_end(value_start),
            _ if INLINE.contains(&marker) || plus => {}
            _ => collector.push(" "),
        }
    }
    collector.flush();
    finish_names(&collector, &mut long, &mut short, parsed);
    parsed.verses.append(&mut collector.out);
}

/// What an installed Bible is called and how it reached the library.
#[derive(Clone, Debug, Default)]
pub struct Details {
    /// File name without extension; empty takes it from the source file.
    pub code: String,
    /// Empty takes the name from the file.
    pub name: String,
    pub language: String,
    /// The copyright line, kept in the module so the Look dialog can show it.
    pub copyright: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub code: String,
    pub name: String,
    pub verses: usize,
}

/// A file name safe on every system: letters, digits and underscores, at most 24 characters.
pub fn file_code(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let stem = lower.trim_end_matches("_usfx").trim_end_matches("_usfm");
    stem.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(24).collect()
}

/// Writes verses as a MyBible module: the layout `Bible::open` reads.
pub fn write_mybible(path: &Path, details: &Details, parsed: &Parsed) -> Result<(), BibleError> {
    let _ = std::fs::remove_file(path);
    let mut db = rusqlite::Connection::open(path)?;
    let tx = db.transaction()?;
    tx.execute_batch(
        "create table info (name text, value text);
         create table books (book_number numeric, short_name text, long_name text);
         create table verses (book_number numeric, chapter numeric, verse numeric, text text);
         create index verses_place on verses (book_number, chapter, verse);",
    )?;
    let name = if details.name.is_empty() { &parsed.name } else { &details.name };
    let language = if details.language.is_empty() { &parsed.language } else { &details.language };
    let copyright = if details.copyright.is_empty() { &parsed.copyright } else { &details.copyright };
    for (key, value) in [("description", name.as_str()), ("language", language.as_str()), ("detailed_info", copyright.as_str())] {
        tx.execute("insert into info (name, value) values (?1, ?2)", [key, value])?;
    }
    for (book, long, short) in &parsed.names {
        tx.execute("insert into books (book_number, short_name, long_name) values (?1, ?2, ?3)", rusqlite::params![mybible_number(*book), short, long])?;
    }
    {
        let mut insert = tx.prepare("insert into verses (book_number, chapter, verse, text) values (?1, ?2, ?3, ?4)")?;
        for verse in &parsed.verses {
            insert.execute(rusqlite::params![mybible_number(verse.book), verse.chapter, verse.verse, &*verse.text])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn read_limited(reader: impl Read) -> Result<Vec<u8>, ImportError> {
    let mut bytes = Vec::new();
    reader.take(LIMIT).read_to_end(&mut bytes).map_err(|error| ImportError::Read(error.to_string()))?;
    Ok(bytes)
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// Reads a zip holding USFX or USFM files.
fn parse_archive(bytes: &[u8]) -> Result<Parsed, ImportError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| ImportError::Read(error.to_string()))?;
    let mut usfx: Option<String> = None;
    let mut usfm: Vec<(String, String)> = Vec::new();
    let (mut names_xml, mut meta_xml) = (String::new(), String::new());
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index(index) else { continue };
        let name = entry.name().map(|name| name.into_owned()).unwrap_or_default();
        let lower = name.to_lowercase();
        if entry.is_dir() || lower.contains("__macosx") {
            continue;
        }
        if lower.ends_with(".usfm") || lower.ends_with(".sfm") {
            usfm.push((name, utf8(read_limited(entry)?)));
        } else if lower.ends_with("booknames.xml") {
            names_xml = utf8(read_limited(entry)?);
        } else if lower.ends_with("metadata.xml") {
            meta_xml = utf8(read_limited(entry)?);
        } else if lower.ends_with(".xml") && usfx.is_none() {
            let text = utf8(read_limited(entry)?);
            if text.contains("<usfx") {
                usfx = Some(text);
            }
        }
    }
    let mut parsed = if let Some(text) = usfx {
        parse_usfx(&text)?
    } else if !usfm.is_empty() {
        usfm.sort();
        let mut parsed = Parsed::default();
        for (_, text) in &usfm {
            parse_usfm(text, &mut parsed);
        }
        if parsed.verses.is_empty() {
            return Err(ImportError::Empty);
        }
        parsed
    } else {
        return Err(ImportError::Unsupported);
    };
    if parsed.names.is_empty() {
        parsed.names = parse_book_names(&names_xml);
    }
    (parsed.name, parsed.language) = parse_metadata(&meta_xml);
    Ok(parsed)
}

/// Converts or copies a Bible into `bibles` and returns what was installed. `file_name` says
/// what the bytes are: `.sqlite3` or `.mybible`, `.xml` (USFX) or `.zip` (USFX or USFM files).
pub fn install_bytes(file_name: &str, bytes: &[u8], bibles: &Path, details: &Details) -> Result<Installed, ImportError> {
    let lower = file_name.to_lowercase();
    let stem = Path::new(file_name).file_stem().map(|stem| stem.to_string_lossy().into_owned()).unwrap_or_default();
    let code = if details.code.is_empty() { file_code(&stem) } else { file_code(&details.code) };
    if code.is_empty() {
        return Err(ImportError::Unsupported);
    }
    std::fs::create_dir_all(bibles).map_err(|error| ImportError::Write(error.to_string()))?;
    let target: PathBuf = bibles.join(format!("{code}.sqlite3"));
    let temporary: PathBuf = bibles.join(format!("{code}.partial"));
    let counted = if lower.ends_with(".sqlite3") || lower.ends_with(".mybible") {
        std::fs::write(&temporary, bytes).map_err(|error| ImportError::Write(error.to_string()))?;
        if describe(&temporary).is_err() {
            let _ = std::fs::remove_file(&temporary);
            return Err(ImportError::Unsupported);
        }
        None
    } else {
        let parsed = if lower.ends_with(".zip") {
            parse_archive(bytes)?
        } else if lower.ends_with(".xml") {
            let text = utf8(bytes.to_vec());
            // USFX has a <usfx> root; a Beblia file has a <bible> root with numbered books.
            if text.contains("<usfx") { parse_usfx(&text)? } else { parse_beblia(&text)? }
        } else if lower.ends_with(".usfm") || lower.ends_with(".sfm") {
            let mut parsed = Parsed::default();
            parse_usfm(&utf8(bytes.to_vec()), &mut parsed);
            if parsed.verses.is_empty() {
                return Err(ImportError::Empty);
            }
            parsed
        } else {
            return Err(ImportError::Unsupported);
        };
        let count = parsed.verses.len();
        let mut details = details.clone();
        if details.name.is_empty() && parsed.name.is_empty() {
            details.name = stem.clone();
        }
        if let Err(error) = write_mybible(&temporary, &details, &parsed) {
            let _ = std::fs::remove_file(&temporary);
            return Err(ImportError::Write(error.to_string()));
        }
        Some(count)
    };
    std::fs::rename(&temporary, &target).map_err(|error| ImportError::Write(error.to_string()))?;
    let (shown, name, _) = describe(&target).map_err(|error| ImportError::Write(error.to_string()))?;
    let verses = match counted {
        Some(count) => count,
        None => crate::bible::Bible::open(&target).map(|bible| bible.all_verses().len()).map_err(|error| ImportError::Write(error.to_string()))?,
    };
    Ok(Installed { code: shown, name, verses })
}

/// Installs a Bible file the person chose.
pub fn install_path(source: &Path, bibles: &Path) -> Result<Installed, ImportError> {
    let bytes = std::fs::read(source).map_err(|error| ImportError::Read(error.to_string()))?;
    let name = source.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    install_bytes(&name, &bytes, bibles, &Details::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const USFX: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"utf-8\"?><usfx><languageCode>xx</languageCode>\n\
        <book id=\"FRT\"><p>Front matter words.</p></book>\n\
        <book id=\"GEN\"><id id=\"GEN\">Test Book</id><h>Made Up</h>\n\
        <c id=\"1\" /><p sfm=\"p\"><s>A heading</s>\n<v id=\"1\" bcv=\"GEN.1.1\" />First <w>made</w> up line<f caller=\"+\"><fr>1:1</fr><ft>a note</ft></f>, ended.<ve />\n\
        <v id=\"2\" bcv=\"GEN.1.2\" />Second made up line &amp; more.<ve /></p>\n\
        <c id=\"2\" /><p><v id=\"1-2\" />Bridged made up line.<ve /></p></book>\n\
        <book id=\"3JN\"><c id=\"1\" /><p><v id=\"1\" />Short letter line.<ve /></p></book></usfx>";

    const USFM_ONE: &str = "\\id GEN Test\n\\h Made Up\n\\toc1 The Book Of Made Up\n\\c 1\n\\s1 A heading\n\\p\n\\v 1 First \\w made|lemma=\"x\"\\w* up line\\f + \\fr 1:1 \\ft a note\\f*, ended.\n\\q1\n\\v 2 Second made\n\\q2 up line.\n\\c 2\n\\p\n\\v 1-2 Bridged made up line.\n";
    const USFM_TWO: &str = "\\id REV\n\\c 21\n\\p\n\\v 5 Last made up line.\n";

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("artosia-bible-file-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, body) in files {
            writer.start_file(*name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap_or_else(|e| panic!("{e}"));
            writer.write_all(body.as_bytes()).unwrap_or_else(|e| panic!("{e}"));
        }
        writer.finish().unwrap_or_else(|e| panic!("{e}")).into_inner()
    }

    fn texts(parsed: &Parsed) -> Vec<(u8, u16, u16, String)> {
        parsed.verses.iter().map(|v| (v.book, v.chapter, v.verse, v.text.to_string())).collect()
    }

    #[test]
    fn usfx_keeps_verse_text_and_drops_notes_and_headings() {
        let parsed = parse_usfx(USFX).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            texts(&parsed),
            [
                (1, 1, 1, "First made up line, ended.".to_string()),
                (1, 1, 2, "Second made up line & more.".to_string()),
                (1, 2, 1, "Bridged made up line.".to_string()),
                (64, 1, 1, "Short letter line.".to_string()),
            ]
        );
    }

    #[test]
    fn usfm_keeps_verse_text_and_drops_notes_and_headings() {
        let mut parsed = Parsed::default();
        parse_usfm(USFM_ONE, &mut parsed);
        parse_usfm(USFM_TWO, &mut parsed);
        assert_eq!(
            texts(&parsed),
            [
                (1, 1, 1, "First made up line, ended.".to_string()),
                (1, 1, 2, "Second made up line.".to_string()),
                (1, 2, 1, "Bridged made up line.".to_string()),
                (66, 21, 5, "Last made up line.".to_string()),
            ]
        );
        assert_eq!(parsed.names, [(1, "The Book Of Made Up".to_string(), "Made Up".to_string())]);
    }

    #[test]
    fn a_usfx_zip_installs_as_a_module_the_reader_loads() {
        let bibles = folder("usfx");
        let names = "<BookNames><book code=\"GEN\" long=\"Kitab Buatan\" /></BookNames>";
        let meta = "<DBLMetadata><identification><nameLocal>Alkitab Buatan</nameLocal></identification><language><name>Indonesian</name></language></DBLMetadata>";
        let bytes = zip_of(&[("xx_usfx.xml", USFX), ("BookNames.xml", names), ("xxmetadata.xml", meta)]);
        let details = Details { code: "xx".into(), copyright: "Made up copyright line".into(), ..Details::default() };
        let installed = install_bytes("xx_usfx.zip", &bytes, &bibles, &details).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((installed.code.as_str(), installed.name.as_str(), installed.verses), ("XX", "Alkitab Buatan", 4));
        let bible = crate::bible::Bible::open(&bibles.join("xx.sqlite3")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bible.language, "Indonesian");
        assert_eq!(bible.books()[0].name, "Kitab Buatan");
        let guess = &bible.guess("kej 1 2")[0];
        assert_eq!(bible.verses(&guess.reference)[0].text.as_ref(), "Second made up line & more.");
        let _ = std::fs::remove_dir_all(&bibles);
    }

    #[test]
    fn a_zip_of_usfm_files_installs() {
        let bibles = folder("usfm");
        let bytes = zip_of(&[("02-REV.usfm", USFM_TWO), ("01-GEN.usfm", USFM_ONE), ("readme.txt", "nothing")]);
        let installed = install_bytes("My Church Bible.zip", &bytes, &bibles, &Details::default()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((installed.code.as_str(), installed.verses), ("MYCHURCHBIBLE", 4));
        let bible = crate::bible::Bible::open(&bibles.join("mychurchbible.sqlite3")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bible.name, "My Church Bible");
        assert_eq!(bible.books().len(), 2);
        let _ = std::fs::remove_dir_all(&bibles);
    }

    #[test]
    fn a_mybible_file_is_copied_after_a_check() {
        let (source, bibles) = (folder("mybible-source"), folder("mybible"));
        std::fs::create_dir_all(&source).unwrap_or_else(|e| panic!("{e}"));
        let parsed = parse_usfx(USFX).unwrap_or_else(|e| panic!("{e}"));
        let file = source.join("mine.sqlite3");
        write_mybible(&file, &Details { name: "Mine".into(), ..Details::default() }, &parsed).unwrap_or_else(|e| panic!("{e}"));
        let installed = install_path(&file, &bibles).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((installed.code.as_str(), installed.name.as_str(), installed.verses), ("MINE", "Mine", 4));
        // Something that is not a Bible is refused and leaves nothing behind.
        let junk = source.join("junk.sqlite3");
        std::fs::write(&junk, b"not a database").unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(install_path(&junk, &bibles), Err(ImportError::Unsupported)));
        assert!(!bibles.join("junk.partial").exists());
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&bibles);
    }

    #[test]
    fn files_that_are_not_bibles_are_refused_in_words() {
        let bibles = folder("refuse");
        assert!(matches!(install_bytes("a.txt", b"hello", &bibles, &Details::default()), Err(ImportError::Unsupported)));
        assert!(matches!(install_bytes("a.zip", &zip_of(&[("a.txt", "x")]), &bibles, &Details::default()), Err(ImportError::Unsupported)));
        assert!(matches!(install_bytes("a.xml", b"<usfx></usfx>", &bibles, &Details::default()), Err(ImportError::Empty)));
        let _ = std::fs::remove_dir_all(&bibles);
    }

    /// Reads a real eBible download when one sits in the gitignored `fixtures/ebible` folder.
    #[test]
    fn a_real_ebible_zip_converts_when_present() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ebible/indags_usfx.zip");
        let Ok(bytes) = std::fs::read(source) else { return };
        let bibles = folder("real");
        let installed = install_bytes("indags_usfx.zip", &bytes, &bibles, &Details::default()).unwrap_or_else(|e| panic!("{e}"));
        // The file has 7,959 verse tags; 17 of them are verses a modern text leaves empty.
        assert_eq!(installed.verses, 7942);
        let bible = crate::bible::Bible::open(&bibles.join("indags.sqlite3")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bible.books().len(), 27);
        assert!(!bible.guess("yoh 3 16").is_empty());
        assert!(bible.verses(&bible.guess("yoh 3 16")[0].reference).iter().all(|v| !v.text.contains("1:1")));
        let _ = std::fs::remove_dir_all(&bibles);
    }

    /// Made up, in the shape of a Beblia file: book 99 is past the canon and is skipped.
    const BEBLIA: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <bible translation=\"Made Up (Alkitab Buatan)\" status=\"Made up status &amp; line\" link=\"https://example.invalid/x\">\n\
        \t<testament name=\"Old\">\n\t\t<book number=\"1\">\n\t\t\t<chapter number=\"1\">\n\
        \t\t\t\t<verse number=\"1\">First made   up line &amp; more.</verse>\n\
        \t\t\t\t<verse number=\"2\">Second made up line.</verse>\n\
        \t\t\t\t<verse number=\"3\"></verse>\n\
        \t\t\t</chapter>\n\t\t</book>\n\t</testament>\n\
        \t<testament name=\"New\">\n\t\t<book number=\"66\"><chapter number=\"22\"><verse number=\"21\">Last made up line.</verse></chapter></book>\n\
        \t\t<book number=\"99\"><chapter number=\"1\"><verse number=\"1\">Outside the canon.</verse></chapter></book>\n\t</testament>\n</bible>";

    #[test]
    fn a_beblia_file_keeps_its_verses_name_and_copyright() {
        let parsed = parse_beblia(BEBLIA).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(texts(&parsed), [(1, 1, 1, "First made up line & more.".to_string()), (1, 1, 2, "Second made up line.".to_string()), (66, 22, 21, "Last made up line.".to_string())]);
        assert_eq!((parsed.name.as_str(), parsed.copyright.as_str()), ("Made Up (Alkitab Buatan)", "Made up status & line"));
        // The first kilobyte is enough to read what the file says about itself.
        assert_eq!(beblia_header(&BEBLIA[..300]), Some(("Made Up (Alkitab Buatan)".to_string(), "Made up status & line".to_string())));
        assert_eq!(beblia_header("<usfx></usfx>"), None);
        assert!(matches!(parse_beblia("<usfx></usfx>"), Err(ImportError::Unsupported)));
        assert!(matches!(parse_beblia("<bible translation=\"x\"></bible>"), Err(ImportError::Empty)));
    }

    #[test]
    fn a_beblia_file_installs_with_the_name_and_notice_it_carries() {
        let bibles = folder("beblia");
        let details = Details { language: "Indonesian".into(), ..Details::default() };
        let installed = install_bytes("IndonesianMade.xml", BEBLIA.as_bytes(), &bibles, &details).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((installed.code.as_str(), installed.name.as_str(), installed.verses), ("INDONESIANMADE", "Made Up (Alkitab Buatan)", 3));
        let bible = crate::bible::Bible::open(&bibles.join("indonesianmade.sqlite3")).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(bible.language, "Indonesian");
        let detail: String = rusqlite::Connection::open(bibles.join("indonesianmade.sqlite3"))
            .and_then(|db| db.query_row("select value from info where name = 'detailed_info'", [], |row| row.get(0)))
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(detail, "Made up status & line");
        let _ = std::fs::remove_dir_all(&bibles);
    }

    #[test]
    fn a_file_code_is_safe_on_disk() {
        assert_eq!(file_code("indags_usfx"), "indags");
        assert_eq!(file_code("../Evil Name!"), "evilname");
    }
}
