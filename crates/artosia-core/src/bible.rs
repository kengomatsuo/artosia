//! Bibles: MyBible modules read into memory, references typed loosely, and text search.
//!
//! A MyBible module is a SQLite file with `info`, `books` and `verses` tables; EasiSlides
//! ships its Bibles in this format and thousands more exist. Artosia reads the file once and
//! keeps the active translation in memory (about 5 MB), so a lookup never touches the disk.

use std::ops::Range;
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::search::{TextIndex, tokens};
use crate::text::{edit_distance, highlight_ranges, normalize, typo_budget};

/// MyBible's book numbers for the 66 books, in canonical order.
const MYBIBLE_NUMBERS: [u16; 66] = [
    10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 190, 220, 230, 240, 250, 260, 290, 300, 310,
    330, 340, 350, 360, 370, 380, 390, 400, 410, 420, 430, 440, 450, 460, 470, 480, 490, 500, 510, 520, 530, 540, 550,
    560, 570, 580, 590, 600, 610, 620, 630, 640, 650, 660, 670, 680, 690, 700, 710, 720, 730,
];

/// English names and the short forms people type, spaces removed. The first alias is the
/// full name; a prefix of any alias also matches.
const BOOKS: [(&str, &[&str]); 66] = [
    ("Genesis", &["genesis", "gn"]),
    ("Exodus", &["exodus", "exod"]),
    ("Leviticus", &["leviticus", "lv"]),
    ("Numbers", &["numbers", "nm", "nb"]),
    ("Deuteronomy", &["deuteronomy", "dt"]),
    ("Joshua", &["joshua", "jsh"]),
    ("Judges", &["judges", "jdg", "jg"]),
    ("Ruth", &["ruth", "rth"]),
    ("1 Samuel", &["1samuel", "1sm"]),
    ("2 Samuel", &["2samuel", "2sm"]),
    ("1 Kings", &["1kings", "1kgs"]),
    ("2 Kings", &["2kings", "2kgs"]),
    ("1 Chronicles", &["1chronicles"]),
    ("2 Chronicles", &["2chronicles"]),
    ("Ezra", &["ezra"]),
    ("Nehemiah", &["nehemiah"]),
    ("Esther", &["esther"]),
    ("Job", &["job", "jb"]),
    ("Psalms", &["psalms", "psalm", "pslm", "psm", "pss"]),
    ("Proverbs", &["proverbs", "prv"]),
    ("Ecclesiastes", &["ecclesiastes", "qoheleth"]),
    ("Song of Songs", &["songofsongs", "songofsolomon", "sos", "canticles"]),
    ("Isaiah", &["isaiah"]),
    ("Jeremiah", &["jeremiah", "jr"]),
    ("Lamentations", &["lamentations"]),
    ("Ezekiel", &["ezekiel", "ezk"]),
    ("Daniel", &["daniel", "dn"]),
    ("Hosea", &["hosea"]),
    ("Joel", &["joel", "jl"]),
    ("Amos", &["amos"]),
    ("Obadiah", &["obadiah"]),
    ("Jonah", &["jonah", "jnh"]),
    ("Micah", &["micah"]),
    ("Nahum", &["nahum"]),
    ("Habakkuk", &["habakkuk"]),
    ("Zephaniah", &["zephaniah", "zp"]),
    ("Haggai", &["haggai", "hg"]),
    ("Zechariah", &["zechariah", "zc"]),
    ("Malachi", &["malachi", "ml"]),
    ("Matthew", &["matthew", "mt"]),
    ("Mark", &["mark", "mk", "mrk"]),
    ("Luke", &["luke", "lk"]),
    ("John", &["john", "jn", "jhn"]),
    ("Acts", &["acts"]),
    ("Romans", &["romans", "rm"]),
    ("1 Corinthians", &["1corinthians"]),
    ("2 Corinthians", &["2corinthians"]),
    ("Galatians", &["galatians"]),
    ("Ephesians", &["ephesians"]),
    ("Philippians", &["philippians", "php"]),
    ("Colossians", &["colossians"]),
    ("1 Thessalonians", &["1thessalonians", "1ths"]),
    ("2 Thessalonians", &["2thessalonians", "2ths"]),
    ("1 Timothy", &["1timothy", "1tm"]),
    ("2 Timothy", &["2timothy", "2tm"]),
    ("Titus", &["titus"]),
    ("Philemon", &["philemon", "phm", "phlm"]),
    ("Hebrews", &["hebrews"]),
    ("James", &["james", "jas", "jms"]),
    ("1 Peter", &["1peter", "1pt"]),
    ("2 Peter", &["2peter", "2pt"]),
    ("1 John", &["1john", "1jn"]),
    ("2 John", &["2john", "2jn"]),
    ("3 John", &["3john", "3jn"]),
    ("Jude", &["jude"]),
    ("Revelation", &["revelation", "rv", "apocalypse"]),
];

/// Books in the order a short prefix should prefer them: "jo" means John before Joshua.
const POPULAR: [u8; 30] = [
    43, 19, 40, 45, 1, 42, 20, 23, 44, 46, 41, 49, 50, 58, 66, 59, 2, 47, 48, 62, 51, 60, 5, 24, 55, 54, 52, 27, 6, 18,
];

/// The English name of book 1..=66.
pub fn english_name(book: u8) -> &'static str {
    BOOKS.get(usize::from(book).wrapping_sub(1)).map(|entry| entry.0).unwrap_or("")
}

/// A colour group for the book grid: law, history, wisdom, prophets, gospels, letters, vision.
pub fn book_group(book: u8) -> u8 {
    match book {
        1..=5 => 0,
        6..=17 => 1,
        18..=22 => 2,
        23..=39 => 3,
        40..=44 => 4,
        45..=65 => 5,
        _ => 6,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Reference {
    /// 1..=66.
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
    pub end_chapter: u16,
    pub end_verse: u16,
}

impl Reference {
    pub fn single(book: u8, chapter: u16, verse: u16) -> Reference {
        Reference { book, chapter, verse, end_chapter: chapter, end_verse: verse }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Verse {
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
    pub text: Box<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BookInfo {
    pub number: u8,
    pub name: String,
    pub short: String,
    /// Verse count of each chapter; index 0 is chapter 1.
    pub chapters: Vec<u16>,
}

/// A reference read from loose typing, with how sure the reading is.
#[derive(Clone, Debug, PartialEq)]
pub struct Guess {
    pub reference: Reference,
    pub score: f32,
    /// No verse was typed, so the reference covers the chapter.
    pub whole_chapter: bool,
    /// No chapter was typed either.
    pub book_only: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerseHit {
    /// Index into [`Bible::all_verses`].
    pub index: usize,
    pub ranges: Vec<Range<usize>>,
    /// The verse holds every word of the query.
    pub complete: bool,
}

#[derive(Debug)]
pub enum BibleError {
    Sqlite(rusqlite::Error),
    Empty,
}

impl std::fmt::Display for BibleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BibleError::Sqlite(error) => write!(f, "could not read the Bible file: {error}"),
            BibleError::Empty => write!(f, "the file holds no Bible verses"),
        }
    }
}

impl std::error::Error for BibleError {}

impl From<rusqlite::Error> for BibleError {
    fn from(error: rusqlite::Error) -> Self {
        BibleError::Sqlite(error)
    }
}

/// One translation, held in memory.
pub struct Bible {
    /// Short code shown on slides and in pickers: "KJV".
    pub code: String,
    pub name: String,
    pub language: String,
    books: Vec<BookInfo>,
    verses: Vec<Verse>,
    index: OnceLock<TextIndex>,
}

/// Removes MyBible markup and the stray CSV quoting some modules carry.
fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if in_tag => {}
            '\u{b6}' => {}
            _ => out.push(c),
        }
    }
    let trimmed = out.trim();
    let unquoted = match trimmed.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')) {
        Some(inner) if trimmed.len() >= 2 => inner.replace("\"\"", "\""),
        _ => trimmed.to_string(),
    };
    unquoted.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn canonical(mybible_number: u16) -> Option<u8> {
    MYBIBLE_NUMBERS.iter().position(|&n| n == mybible_number).map(|at| at as u8 + 1)
}

/// A translation's code, name and language, read without loading its verses.
pub fn describe(path: &Path) -> Result<(String, String, String), BibleError> {
    let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let info = |key: &str| -> String {
        db.query_row("select value from info where name = ?1", [key], |row| row.get::<_, String>(0)).unwrap_or_default()
    };
    db.query_row("select count(*) from verses limit 1", [], |row| row.get::<_, i64>(0))?;
    let code = path.file_stem().map(|stem| stem.to_string_lossy().to_uppercase()).unwrap_or_default();
    let name = info("description");
    Ok((code.clone(), if name.is_empty() { code } else { name }, info("language")))
}

impl Bible {
    /// Reads a MyBible module. `code` falls back to the file stem in capitals.
    pub fn open(path: &Path) -> Result<Bible, BibleError> {
        let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let info = |key: &str| -> String {
            db.query_row("select value from info where name = ?1", [key], |row| row.get::<_, String>(0)).unwrap_or_default()
        };
        let code = path.file_stem().map(|stem| stem.to_string_lossy().to_uppercase()).unwrap_or_default();
        let mut names: Vec<(u8, String, String)> = Vec::new();
        {
            let mut statement = db.prepare("select book_number, short_name, long_name from books order by book_number")?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?;
            for row in rows {
                let (number, short, long) = row?;
                if let Some(book) = canonical(number as u16) {
                    names.push((book, long.trim().to_string(), short.trim().to_string()));
                }
            }
        }
        let mut verses: Vec<Verse> = Vec::new();
        {
            let mut statement = db.prepare("select book_number, chapter, verse, text from verses order by book_number, chapter, verse")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?, row.get::<_, f64>(2)?, row.get::<_, String>(3)?))
            })?;
            for row in rows {
                let (number, chapter, verse, text) = row?;
                let Some(book) = canonical(number as u16) else { continue };
                let text = clean(&text);
                if text.is_empty() || chapter < 1.0 || verse < 1.0 {
                    continue;
                }
                verses.push(Verse { book, chapter: chapter as u16, verse: verse as u16, text: text.into_boxed_str() });
            }
        }
        let name = info("description");
        Bible::from_parts(code, if name.is_empty() { None } else { Some(name) }, info("language"), names, verses)
    }

    /// Builds a translation from verses already in canonical order.
    pub fn from_parts(
        code: String,
        name: Option<String>,
        language: String,
        names: Vec<(u8, String, String)>,
        mut verses: Vec<Verse>,
    ) -> Result<Bible, BibleError> {
        if verses.is_empty() {
            return Err(BibleError::Empty);
        }
        verses.sort_by_key(|verse| (verse.book, verse.chapter, verse.verse));
        let mut books: Vec<BookInfo> = Vec::new();
        for verse in &verses {
            if books.last().map(|book| book.number) != Some(verse.book) {
                let own = names.iter().find(|entry| entry.0 == verse.book);
                books.push(BookInfo {
                    number: verse.book,
                    name: own.map(|entry| entry.1.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| english_name(verse.book).to_string()),
                    short: own.map(|entry| entry.2.clone()).unwrap_or_default(),
                    chapters: Vec::new(),
                });
            }
            if let Some(book) = books.last_mut() {
                let chapter = usize::from(verse.chapter);
                if book.chapters.len() < chapter {
                    book.chapters.resize(chapter, 0);
                }
                book.chapters[chapter - 1] = book.chapters[chapter - 1].max(verse.verse);
            }
        }
        let name = name.unwrap_or_else(|| code.clone());
        Ok(Bible { code, name, language, books, verses, index: OnceLock::new() })
    }

    pub fn books(&self) -> &[BookInfo] {
        &self.books
    }

    pub fn book(&self, number: u8) -> Option<&BookInfo> {
        self.books.iter().find(|book| book.number == number)
    }

    pub fn all_verses(&self) -> &[Verse] {
        &self.verses
    }

    fn position(&self, book: u8, chapter: u16, verse: u16) -> usize {
        self.verses.partition_point(|v| (v.book, v.chapter, v.verse) < (book, chapter, verse))
    }

    /// The verses a reference covers, in order.
    pub fn verses(&self, reference: &Reference) -> &[Verse] {
        let start = self.position(reference.book, reference.chapter, reference.verse);
        let end = self.position(reference.book, reference.end_chapter, reference.end_verse.saturating_add(1));
        if start >= end { &[] } else { &self.verses[start..end] }
    }

    /// The reference with the next verse of the same book added; None at the book's end.
    pub fn extended(&self, reference: &Reference) -> Option<Reference> {
        let after = self.position(reference.book, reference.end_chapter, reference.end_verse.saturating_add(1));
        let next = self.verses.get(after).filter(|verse| verse.book == reference.book)?;
        Some(Reference { end_chapter: next.chapter, end_verse: next.verse, ..*reference })
    }

    /// "John 3:16", "John 3:16-18", "John 3", "John 3:16-4:2", using this translation's book names.
    pub fn format(&self, reference: &Reference) -> String {
        let name = self.book(reference.book).map(|book| book.name.as_str()).unwrap_or_else(|| english_name(reference.book));
        let last = self.book(reference.book).and_then(|book| book.chapters.get(usize::from(reference.chapter).wrapping_sub(1)).copied()).unwrap_or(0);
        if reference.end_chapter != reference.chapter {
            format!("{name} {}:{}-{}:{}", reference.chapter, reference.verse, reference.end_chapter, reference.end_verse)
        } else if reference.verse == 1 && reference.end_verse >= last && last > 1 {
            format!("{name} {}", reference.chapter)
        } else if reference.end_verse != reference.verse {
            format!("{name} {}:{}-{}", reference.chapter, reference.verse, reference.end_verse)
        } else {
            format!("{name} {}:{}", reference.chapter, reference.verse)
        }
    }

    /// Scores every book against a typed name with its spaces removed.
    fn match_books(&self, typed: &str) -> Vec<(u8, f32)> {
        let typed_chars = typed.chars().count();
        let ordinal = typed.chars().next().filter(char::is_ascii_digit);
        let mut out: Vec<(u8, f32)> = Vec::new();
        for info in &self.books {
            let at = usize::from(info.number) - 1;
            let own = [normalize(&info.name).replace(' ', ""), normalize(&info.short).replace(' ', "")];
            let aliases = BOOKS[at].1.iter().map(|alias| (*alias).to_string()).chain(own);
            let mut best = 0f32;
            for alias in aliases {
                if alias.is_empty() || ordinal != alias.chars().next().filter(char::is_ascii_digit) {
                    continue;
                }
                let quality = if alias == typed {
                    1.0
                } else if alias.starts_with(typed) {
                    0.9
                } else if edit_distance(typed, &alias, typo_budget(typed_chars)).is_some_and(|_| typed_chars >= 4) {
                    0.6
                } else if typed_chars >= 4 && alias.chars().count() > typed_chars {
                    let head: String = alias.chars().take(typed_chars).collect();
                    if edit_distance(typed, &head, 1).is_some() { 0.5 } else { 0.0 }
                } else {
                    0.0
                };
                best = best.max(quality);
            }
            if best > 0.0 {
                let rank = POPULAR.iter().position(|&book| book == info.number).unwrap_or(POPULAR.len() + at);
                out.push((info.number, best + 0.08 * (1.0 - rank as f32 / 100.0)));
            }
        }
        out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        out
    }

    /// Reads a reference as it is being typed: "joh 3 16", "1 pet 3:3-7", "ps23", "rom 8,28".
    /// Returns nothing unless the text is a book name followed only by numbers.
    pub fn guess(&self, query: &str) -> Vec<Guess> {
        let mut book = String::new();
        let mut numbers: Vec<u16> = Vec::new();
        let mut pieces: Vec<(bool, String)> = Vec::new();
        let mut joined = false;
        for c in query.trim().chars() {
            let digit = c.is_ascii_digit();
            if !digit && !c.is_alphanumeric() {
                joined = false;
                continue;
            }
            match pieces.last_mut() {
                Some((kind, text)) if joined && *kind == digit => text.push(c),
                _ => pieces.push((digit, c.to_string())),
            }
            joined = true;
        }
        let mut pieces = pieces.into_iter().peekable();
        // A leading 1, 2 or 3 belongs to the book when letters follow it.
        if let Some((true, digits)) = pieces.peek().cloned() {
            if matches!(digits.as_str(), "1" | "2" | "3") {
                pieces.next();
                // "1st", "2nd", "3rd": the suffix is not part of the name.
                if matches!(pieces.peek(), Some((false, suffix)) if matches!(suffix.to_lowercase().as_str(), "st" | "nd" | "rd")) {
                    pieces.next();
                }
                if matches!(pieces.peek(), Some((false, _))) {
                    book.push_str(&digits);
                } else {
                    return Vec::new();
                }
            } else {
                return Vec::new();
            }
        }
        while let Some((false, text)) = pieces.peek().cloned() {
            let folded = normalize(&text).replace(' ', "");
            let word = match (book.is_empty(), folded.as_str()) {
                (true, "first" | "1st") => "1",
                (true, "second" | "2nd") => "2",
                (true, "third" | "3rd") => "3",
                _ => folded.as_str(),
            };
            book.push_str(word);
            pieces.next();
        }
        for (digit, text) in pieces {
            // Letters after the numbers mean this is a search for words, not a reference.
            if !digit {
                return Vec::new();
            }
            numbers.push(text.parse().unwrap_or(u16::MAX));
        }
        if book.is_empty() || book.chars().all(|c| c.is_ascii_digit()) || numbers.len() > 4 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (number, score) in self.match_books(&book).into_iter().take(6) {
            let Some(info) = self.book(number) else { continue };
            let chapters = info.chapters.len() as u16;
            let one_chapter = chapters == 1;
            // "jude 5" is verse 5 of the only chapter.
            let numbers: Vec<u16> = if one_chapter && numbers.len() <= 2 && numbers.first() != Some(&1) {
                std::iter::once(1).chain(numbers.iter().copied()).collect()
            } else {
                numbers.clone()
            };
            let chapter = numbers.first().copied().unwrap_or(1);
            if chapter == 0 || chapter > chapters {
                continue;
            }
            let last = |chapter: u16| info.chapters.get(usize::from(chapter) - 1).copied().unwrap_or(1);
            let reference = match numbers.as_slice() {
                [] | [_] => Reference { book: number, chapter, verse: 1, end_chapter: chapter, end_verse: last(chapter) },
                [_, verse] => {
                    if *verse == 0 || *verse > last(chapter) {
                        continue;
                    }
                    Reference::single(number, chapter, *verse)
                }
                [_, verse, end] => {
                    if *verse == 0 || *verse > last(chapter) {
                        continue;
                    }
                    Reference { book: number, chapter, verse: *verse, end_chapter: chapter, end_verse: (*end).clamp(*verse, last(chapter)) }
                }
                [_, verse, end_chapter, end_verse] => {
                    if *verse == 0 || *verse > last(chapter) || *end_chapter < chapter || *end_chapter > chapters {
                        continue;
                    }
                    Reference { book: number, chapter, verse: *verse, end_chapter: *end_chapter, end_verse: (*end_verse).clamp(1, last(*end_chapter)) }
                }
                _ => continue,
            };
            out.push(Guess { reference, score, whole_chapter: numbers.len() <= 1, book_only: numbers.is_empty() });
        }
        out
    }

    /// Builds the word index now; call from a worker thread so the first search is instant.
    pub fn warm(&self) {
        self.index();
    }

    fn index(&self) -> &TextIndex {
        self.index.get_or_init(|| TextIndex::build(self.verses.iter().map(|verse| &*verse.text)))
    }

    /// Verses holding the typed words, typos forgiven; an exact phrase comes first.
    pub fn search(&self, query: &str, limit: usize) -> Vec<VerseHit> {
        let tokens = tokens(query);
        if tokens.is_empty() {
            return Vec::new();
        }
        let phrase = tokens.join(" ");
        let mut scored: Vec<(f32, usize, bool)> = self
            .index()
            .query(&tokens)
            .into_iter()
            .take_while(|hit| usize::from(hit.matched) * 2 > tokens.len() || tokens.len() == 1)
            .take(400)
            .map(|hit| {
                let text = normalize(&self.verses[hit.entry as usize].text);
                let whole = usize::from(hit.matched) == tokens.len();
                let bonus = if whole && tokens.len() > 1 && text.contains(&phrase) { 3.0 } else { 0.0 };
                (hit.score + f32::from(hit.matched) * 2.0 + bonus, hit.entry as usize, whole)
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        scored
            .into_iter()
            .take(limit)
            .map(|(_, index, complete)| VerseHit { index, ranges: highlight_ranges(&self.verses[index].text, &tokens), complete })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> Bible {
        let verse = |book: u8, chapter: u16, verse: u16, text: &str| Verse { book, chapter, verse, text: text.into() };
        let mut verses = vec![
            verse(1, 1, 1, "Opening line of the first book."),
            verse(1, 1, 2, "Second line of the first book."),
            verse(1, 2, 1, "A new chapter starts here."),
            verse(18, 1, 1, "A patient man lived in the east."),
            verse(18, 1, 2, "He had sons and daughters."),
            verse(19, 23, 1, "The shepherd leads me beside quiet water."),
            verse(19, 23, 2, "He restores what was worn thin."),
            verse(43, 3, 16, "So great a love was shown to the world."),
            verse(43, 3, 17, "Not to judge it but to rescue it."),
            verse(43, 3, 18, "Whoever trusts is not judged."),
            verse(43, 4, 1, "A journey north through the hills."),
            verse(43, 4, 2, "Though the students did the washing."),
            verse(6, 1, 1, "After the leader died a new one rose."),
            verse(6, 1, 2, "Now cross the river with the people."),
            verse(65, 1, 5, "A reminder of what was once known."),
            verse(60, 3, 3, "Let beauty be more than braided hair."),
            verse(62, 4, 8, "Whoever does not love has not known."),
        ];
        for chapter in 1..=22u16 {
            verses.push(verse(19, chapter, 1, "Filler line for an earlier psalm."));
        }
        for v in 1..=15u16 {
            verses.push(verse(43, 3, v, "An earlier line of the third chapter."));
        }
        for v in 1..=4u16 {
            verses.push(verse(65, 1, v, "Opening lines of a short letter."));
        }
        Bible::from_parts("TST".into(), None, "English".into(), Vec::new(), verses).unwrap_or_else(|_| unreachable!())
    }

    fn top(bible: &Bible, query: &str) -> Option<String> {
        bible.guess(query).first().map(|guess| bible.format(&guess.reference))
    }

    #[test]
    fn loose_references_are_read() {
        let bible = tiny();
        assert_eq!(top(&bible, "john 3:16").as_deref(), Some("John 3:16"));
        assert_eq!(top(&bible, "joh 3,16-18").as_deref(), Some("John 3:16-18"));
        assert_eq!(top(&bible, "jn3 16").as_deref(), Some("John 3:16"));
        assert_eq!(top(&bible, "1 pet 3 3").as_deref(), Some("1 Peter 3:3"));
        assert_eq!(top(&bible, "1pet3:3").as_deref(), Some("1 Peter 3:3"));
        assert_eq!(top(&bible, "first john 4 8").as_deref(), Some("1 John 4:8"));
        assert_eq!(top(&bible, "1st john 4:8").as_deref(), Some("1 John 4:8"));
        assert_eq!(top(&bible, "ps 23").as_deref(), Some("Psalms 23"));
        assert_eq!(top(&bible, "john 3:16-4:2").as_deref(), Some("John 3:16-4:2"));
    }

    #[test]
    fn a_short_prefix_prefers_the_book_people_mean() {
        let bible = tiny();
        assert_eq!(top(&bible, "jo 3 16").as_deref(), Some("John 3:16"));
        assert_eq!(top(&bible, "jos").as_deref(), Some("Joshua 1"));
        assert_eq!(top(&bible, "job").as_deref(), Some("Job 1"));
    }

    #[test]
    fn typos_in_book_names_are_forgiven() {
        let bible = tiny();
        assert_eq!(top(&bible, "jhon 3 16").as_deref(), Some("John 3:16"));
        assert_eq!(top(&bible, "pslams 23 1").as_deref(), Some("Psalms 23:1"));
    }

    #[test]
    fn one_chapter_books_take_a_bare_verse() {
        let bible = tiny();
        assert_eq!(top(&bible, "jude 5").as_deref(), Some("Jude 1:5"));
    }

    #[test]
    fn words_after_numbers_are_not_a_reference() {
        let bible = tiny();
        assert!(bible.guess("john 3 love").is_empty());
        assert!(bible.guess("316").is_empty());
        assert!(bible.guess("quiet water").is_empty());
    }

    #[test]
    fn verses_that_do_not_exist_are_dropped() {
        let bible = tiny();
        assert!(bible.guess("john 99").is_empty());
        assert!(bible.guess("john 3 99").is_empty());
        // An end past the chapter is pulled back to its last verse.
        assert_eq!(top(&bible, "john 3 16-99").as_deref(), Some("John 3:16-18"));
    }

    #[test]
    fn a_reference_returns_its_verses() {
        let bible = tiny();
        let guess = &bible.guess("john 3:16-4:1")[0];
        let texts: Vec<u16> = bible.verses(&guess.reference).iter().map(|v| v.verse).collect();
        assert_eq!(texts, [16, 17, 18, 1]);
    }

    #[test]
    fn text_search_forgives_typos_and_prefers_phrases() {
        let bible = tiny();
        let hits = bible.search("quiet watr", 5);
        let first = &bible.all_verses()[hits[0].index];
        assert_eq!((first.book, first.chapter, first.verse), (19, 23, 1));
        let hits = bible.search("first book", 5);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn markup_and_csv_quotes_are_removed() {
        assert_eq!(clean("\"And he said, \"\"Go\"\"; so they went.\""), "And he said, \"Go\"; so they went.");
        assert_eq!(clean("In <i>the</i> beginning<pb/>  God"), "In the beginning God");
    }
}
