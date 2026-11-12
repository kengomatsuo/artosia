//! Bringing a library over from EasiSlides 5, and the Bible that ships inside Artosia.
//!
//! EasiSlides keeps its data folder in `InitialConfig.xml` under the local app-data folder:
//! songs in `Admin/Databases/*.db` (one `items` table) and Bibles as MyBible modules in
//! `Admin/HolyBibles`. Read on 2026-11-10 from an EasiSlides 5.2 install.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::bible::{Bible, BibleError, Verse};
use crate::song::{Song, parse_lyrics, write_lyrics};

/// The EasiSlides data folder on this computer, if EasiSlides was ever set up here.
pub fn easislides_folder() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let config = Path::new(&local).join("Easyslides").join("InitialConfig.xml");
        if let Ok(xml) = std::fs::read_to_string(config)
            && let Some(location) = between(&xml, "<DataLocation>", "</DataLocation>") {
                candidates.push(PathBuf::from(location.trim()));
            }
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        candidates.push(Path::new(&home).join("Documents").join("EasyslidesData"));
    }
    candidates.into_iter().find(|folder| folder.join("Admin").join("Databases").is_dir())
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(&text[start..end])
}

fn files_with(folder: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| extensions.iter().any(|want| want.eq_ignore_ascii_case(e)))
        })
        .collect();
    found.sort();
    found
}

/// The song databases inside an EasiSlides data folder.
pub fn easislides_song_files(folder: &Path) -> Vec<PathBuf> {
    files_with(&folder.join("Admin").join("Databases"), &["db"])
}

/// How many songs an EasiSlides data folder holds, counted without reading them, so the first
/// run can say "961 songs found" at once. A database that cannot be read counts for none.
pub fn easislides_song_count(folder: &Path) -> usize {
    easislides_song_files(folder)
        .iter()
        .filter_map(|file| {
            let db = rusqlite::Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
            db.query_row("select count(*) from items where trim(coalesce(title1, '')) != ''", [], |row| row.get::<_, i64>(0)).ok()
        })
        .map(|count| usize::try_from(count).unwrap_or(0))
        .sum()
}

/// The MyBible modules inside an EasiSlides data folder.
pub fn easislides_bible_files(folder: &Path) -> Vec<PathBuf> {
    files_with(&folder.join("Admin").join("HolyBibles"), &["sqlite3"])
}

/// Every MyBible module in a folder.
pub fn bible_files(folder: &Path) -> Vec<PathBuf> {
    files_with(folder, &["sqlite3", "mybible"])
}

/// Reads the songs of one EasiSlides database. Lyrics already use `[1]` and `[chorus]`
/// markers; a `[region 2]` marker starts the second language.
pub fn read_easislides_songs(path: &Path) -> Result<Vec<Song>, rusqlite::Error> {
    let db = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut statement = db.prepare(
        "select title1, coalesce(title2, ''), coalesce(songnumber, 0), coalesce(contents, ''), coalesce(sequence, ''),
                coalesce(author, ''), coalesce(copyright, ''), coalesce(songkey, ''), coalesce(cclisongnumber, '')
         from items order by title1 collate nocase",
    )?;
    let rows = statement.query_map([], |row| {
        let contents: String = row.get(3)?;
        let (first, second) = split_regions(&contents);
        Ok(Song {
            id: 0,
            title: row.get::<_, String>(0)?.trim().to_string(),
            alt_title: row.get::<_, String>(1)?.trim().to_string(),
            number: u32::try_from(row.get::<_, i64>(2)?).unwrap_or(0),
            lyrics: first,
            second,
            order: row.get::<_, String>(4)?.trim().to_string(),
            author: row.get::<_, String>(5)?.trim().to_string(),
            copyright: row.get::<_, String>(6)?.trim().to_string(),
            key: row.get::<_, String>(7)?.trim().to_string(),
            ccli: row.get::<_, String>(8)?.trim().to_string(),
            // EasiSlides keys a song by title and author; it has no id that outlives a rename.
            source: String::new(),
            edited: false,
        })
    })?;
    let mut songs = Vec::new();
    for row in rows {
        let song = row?;
        if !song.title.is_empty() {
            songs.push(song);
        }
    }
    Ok(songs)
}

/// Splits EasiSlides contents into the first and second language and tidies both.
fn split_regions(contents: &str) -> (String, String) {
    let text = contents.replace("\r\n", "\n").replace('\r', "\n");
    let lower = text.to_lowercase();
    let (first, second) = match lower.find("[region 2]") {
        Some(at) => (&text[..at], &text[at + "[region 2]".len()..]),
        None => (text.as_str(), ""),
    };
    let tidy = |part: &str| if part.trim().is_empty() { String::new() } else { write_lyrics(&parse_lyrics(part)) };
    (tidy(first), tidy(second))
}

/// The King James Version (public domain), packed into the program as gzipped lines of
/// `book<TAB>chapter<TAB>verse<TAB>text`. Made by `tools/make-bible-asset.py`.
const KJV: &[u8] = include_bytes!("../assets/kjv.tsv.gz");

/// The Bible every install starts with.
pub fn bundled_bible() -> Result<Bible, BibleError> {
    let mut text = String::new();
    if flate2::read::GzDecoder::new(KJV).read_to_string(&mut text).is_err() {
        return Err(BibleError::Empty);
    }
    let mut verses = Vec::with_capacity(31_200);
    for line in text.lines() {
        let mut fields = line.splitn(4, '\t');
        let (Some(book), Some(chapter), Some(verse), Some(body)) = (fields.next(), fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let (Ok(book), Ok(chapter), Ok(verse)) = (book.parse::<u8>(), chapter.parse::<u16>(), verse.parse::<u16>()) else {
            continue;
        };
        verses.push(Verse { book, chapter, verse, text: body.into() });
    }
    Bible::from_parts("KJV".to_string(), Some("King James Version".to_string()), "English".to_string(), Vec::new(), verses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_bible_is_whole() {
        let bible = bundled_bible().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(bible.books().len(), 66);
        assert_eq!(bible.all_verses().len(), 31_102);
        let guess = &bible.guess("gen 1 1")[0];
        assert!(bible.verses(&guess.reference)[0].text.starts_with("In the beginning"));
        assert_eq!(bible.format(&bible.guess("ps 117")[0].reference), "Psalms 117");
    }

    #[test]
    fn the_bundled_bible_is_searchable() {
        let bible = bundled_bible().unwrap_or_else(|error| panic!("{error}"));
        let hits = bible.search("in the begining god created", 3);
        let first = &bible.all_verses()[hits[0].index];
        assert_eq!((first.book, first.chapter, first.verse), (1, 1, 1));
    }

    #[test]
    fn the_songs_of_an_easislides_folder_are_counted_without_reading_them() {
        let folder = std::env::temp_dir().join(format!("artosia-easi-count-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let databases = folder.join("Admin").join("Databases");
        std::fs::create_dir_all(&databases).unwrap_or_else(|error| panic!("{error}"));
        for (name, titles) in [("a.db", &["One", "Two", ""][..]), ("b.db", &["Three"][..])] {
            let db = rusqlite::Connection::open(databases.join(name)).unwrap_or_else(|error| panic!("{error}"));
            db.execute("create table items (title1 text)", []).unwrap_or_else(|error| panic!("{error}"));
            for title in titles {
                db.execute("insert into items (title1) values (?1)", [title]).unwrap_or_else(|error| panic!("{error}"));
            }
        }
        std::fs::write(databases.join("broken.db"), b"not a database").unwrap_or_else(|error| panic!("{error}"));
        // Three titled songs; the untitled row is skipped as the reader skips it, and the broken file counts none.
        assert_eq!(easislides_song_count(&folder), 3);
        assert_eq!(easislides_song_count(&folder.join("missing")), 0);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn region_two_becomes_the_second_language() {
        let (first, second) = split_regions("[1]\r\nline one\r\n[region 2]\r\n[1]\r\nbaris satu\r\n");
        assert_eq!(first, "[1]\nline one\n");
        assert_eq!(second, "[1]\nbaris satu\n");
    }
}
