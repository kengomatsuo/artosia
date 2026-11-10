//! eBible's catalogue of translations: what may be offered, grouped by language.
//!
//! eBible.org lists its translations in one CSV (`https://ebible.org/Scriptures/translations.csv`,
//! 33 columns, read on 2026-11-11). Only rows it marks `Redistributable` and `downloadable` are
//! offered, and the `Copyright` column is shown before a download because "redistributable"
//! also covers non-commercial terms (docs/bibles.md). This module reads and arranges the list;
//! the network is the window's business.

use crate::text::normalize;

pub const CATALOGUE_URL: &str = "https://ebible.org/Scriptures/translations.csv";

/// The zip of USFX files for one translation.
pub fn download_url(id: &str) -> String {
    format!("https://ebible.org/Scriptures/{id}_usfx.zip")
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Translation {
    pub id: String,
    /// eBible's language code; its first three letters are the ISO 639-3 code.
    pub language_code: String,
    /// The language in its own words.
    pub language: String,
    pub language_english: String,
    pub title: String,
    pub description: String,
    pub copyright: String,
    pub old_books: u16,
    pub new_books: u16,
}

impl Translation {
    pub fn whole_bible(&self) -> bool {
        self.old_books >= 39 && self.new_books >= 27
    }

    /// 0 whole Bible, 1 New Testament, 2 Old Testament, 3 some books.
    pub fn scope(&self) -> u8 {
        match (self.old_books >= 39, self.new_books >= 27) {
            (true, true) => 0,
            (false, true) => 1,
            (true, false) => 2,
            _ => 3,
        }
    }
}

/// Splits CSV text into rows of fields: quoted fields may hold commas, doubled quotes and line breaks.
fn rows(text: &str) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => row.push(std::mem::take(&mut field)),
            ('\r', false) => {}
            ('\n', false) => {
                row.push(std::mem::take(&mut field));
                if row.iter().any(|value| !value.is_empty()) {
                    out.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        out.push(row);
    }
    out
}

fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The translations eBible says anyone may redistribute and download, in the file's order.
/// A file without the columns this needs gives an empty list.
pub fn parse_catalogue(csv: &str) -> Vec<Translation> {
    let mut rows = rows(csv).into_iter();
    let Some(header) = rows.next() else { return Vec::new() };
    let column = |name: &str| header.iter().position(|title| title == name);
    let (Some(id), Some(free)) = (column("translationId"), column("Redistributable")) else { return Vec::new() };
    let get = |row: &[String], at: Option<usize>| at.and_then(|at| row.get(at)).map(|value| tidy(value)).unwrap_or_default();
    let (code, local, english) = (column("languageCode"), column("languageName"), column("languageNameInEnglish"));
    let (title, description, copyright) = (column("title"), column("description"), column("Copyright"));
    let (old, new, downloadable) = (column("OTbooks"), column("NTbooks"), column("downloadable"));
    let mut out = Vec::new();
    for row in rows {
        let flag = |at: Option<usize>| get(&row, at).eq_ignore_ascii_case("true");
        let translation_id = get(&row, Some(id));
        // A row with no `downloadable` column is taken as downloadable; one that says False is not.
        let can_download = downloadable.is_none() || flag(downloadable);
        if translation_id.is_empty() || !flag(Some(free)) || !can_download {
            continue;
        }
        let books = |at: Option<usize>| get(&row, at).parse::<u16>().unwrap_or(0);
        out.push(Translation {
            id: translation_id,
            language_code: get(&row, code),
            language: get(&row, local),
            language_english: get(&row, english),
            title: get(&row, title),
            description: get(&row, description),
            copyright: get(&row, copyright),
            old_books: books(old),
            new_books: books(new),
        });
    }
    out
}

/// ISO 639-3 codes of the languages of Indonesia that eBible might list, Indonesian first.
/// A church in Indonesia looks here before the alphabet. The code is the first three letters of
/// eBible's language code (`zlmKSZI` is Malay).
const INDONESIA: [&str; 31] = [
    "ind", "zlm", "zsm", "msa", "mkn", "pmy", "abs", "jav", "jvn", "sun", "ban", "min", "bug", "mak", "bjn", "ace", "bbc", "btd", "btx", "bts", "mad", "ljp",
    "sas", "mqy", "nij", "kje", "bew", "mui", "kvr", "tet", "pse",
];

/// Where a language ranks among the languages of Indonesia, None for any other language.
pub fn indonesia_rank(language_code: &str) -> Option<usize> {
    let code: String = language_code.chars().take(3).collect::<String>().to_lowercase();
    INDONESIA.iter().position(|known| *known == code)
}

/// The translations of one language.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub language: String,
    pub local: String,
    pub of_indonesia: bool,
    /// Indexes into the list that was grouped.
    pub items: Vec<usize>,
}

/// Groups the translations that hold every word of `query` (in a language name, a title or an
/// id), the languages of Indonesia first, then A to Z. Within a language: whole Bibles first.
pub fn grouped(list: &[Translation], query: &str) -> Vec<Group> {
    let words: Vec<String> = normalize(query).split_whitespace().map(str::to_string).collect();
    let mut groups: Vec<(Option<usize>, Group)> = Vec::new();
    for (at, item) in list.iter().enumerate() {
        let hay = normalize(&format!("{} {} {} {} {} {}", item.language_english, item.language, item.title, item.id, item.description, item.language_code));
        if !words.iter().all(|word| hay.contains(word.as_str())) {
            continue;
        }
        let name = if item.language_english.is_empty() { item.language.clone() } else { item.language_english.clone() };
        // One language may reach the list under several codes; the shown name joins them.
        match groups.iter_mut().find(|(_, group)| group.language == name) {
            Some((_, group)) => group.items.push(at),
            None => {
                let rank = indonesia_rank(&item.language_code);
                groups.push((rank, Group { language: name, local: item.language.clone(), of_indonesia: rank.is_some(), items: vec![at] }));
            }
        }
    }
    for (_, group) in &mut groups {
        group.items.sort_by(|a, b| {
            let (a, b) = (&list[*a], &list[*b]);
            a.scope().cmp(&b.scope()).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())).then_with(|| a.id.cmp(&b.id))
        });
    }
    groups.sort_by(|(a_rank, a), (b_rank, b)| match (a_rank, b_rank) {
        (Some(x), Some(y)) => x.cmp(y).then_with(|| a.language.cmp(&b.language)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.language.to_lowercase().cmp(&b.language.to_lowercase()),
    });
    groups.into_iter().map(|(_, group)| group).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Made-up rows in the real file's shape, with a comma and a line break inside quotes.
    const CSV: &str = "\u{feff}\"languageCode\",\"translationId\",\"languageName\",\"languageNameInEnglish\",\"title\",\"description\",\"Redistributable\",\"Copyright\",\"OTbooks\",\"NTbooks\",\"downloadable\"\r\n\
        \"xyz\",\"xyz1\",\"Zed\",\"Zedish\",\"The Zed Bible\",\"A made up Bible, whole\",\"True\",\"Made up, 2000\nline two\",\"39\",\"27\",\"True\"\r\n\
        \"ind\",\"indmade\",\"Bahasa Buatan\",\"Indonesian\",\"Alkitab Buatan\",\"Made up\",\"True\",\"Copyright \"\"made up\"\"\",\"0\",\"27\",\"True\"\r\n\
        \"ind\",\"indwhole\",\"Bahasa Buatan\",\"Indonesian\",\"Zeta Buatan\",\"Made up\",\"True\",\"Public domain\",\"39\",\"27\",\"True\"\r\n\
        \"abc\",\"abcnot\",\"Abc\",\"Abcish\",\"Closed\",\"x\",\"False\",\"All rights reserved\",\"39\",\"27\",\"True\"\r\n\
        \"abc\",\"abcgone\",\"Abc\",\"Abcish\",\"Gone\",\"x\",\"True\",\"x\",\"39\",\"27\",\"False\"\r\n\
        \"zlmKSZI\",\"zlmmade\",\"Melayu\",\"Malay\",\"Malay made up\",\"x\",\"True\",\"x\",\"0\",\"27\",\"True\"\r\n\
        \"aaa\",\"aaa1\",\"Aaa\",\"Aaaish\",\"First alphabetically\",\"x\",\"True\",\"x\",\"0\",\"4\",\"True\"\r\n";

    #[test]
    fn only_redistributable_downloadable_rows_are_kept() {
        let list = parse_catalogue(CSV);
        let ids: Vec<&str> = list.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["xyz1", "indmade", "indwhole", "zlmmade", "aaa1"]);
        assert_eq!(list[0].copyright, "Made up, 2000 line two");
        assert_eq!(list[1].copyright, "Copyright \"made up\"");
        assert!(list[0].whole_bible() && !list[1].whole_bible());
        assert_eq!(list[4].scope(), 3);
    }

    #[test]
    fn a_file_without_the_columns_gives_nothing() {
        assert!(parse_catalogue("").is_empty());
        assert!(parse_catalogue("a,b\n1,2\n").is_empty());
    }

    #[test]
    fn the_languages_of_indonesia_come_first() {
        let list = parse_catalogue(CSV);
        let names: Vec<String> = grouped(&list, "").into_iter().map(|group| group.language).collect();
        assert_eq!(names, ["Indonesian", "Malay", "Aaaish", "Zedish"]);
        let groups = grouped(&list, "");
        assert!(groups[0].of_indonesia && groups[1].of_indonesia && !groups[2].of_indonesia);
        // A whole Bible leads its language, whatever the titles say.
        assert_eq!(list[groups[0].items[0]].id, "indwhole");
    }

    #[test]
    fn a_search_reads_language_or_name() {
        let list = parse_catalogue(CSV);
        assert_eq!(grouped(&list, "indonesian").len(), 1);
        assert_eq!(grouped(&list, "BAHASA buatan").len(), 1);
        assert_eq!(grouped(&list, "zed bible").len(), 1);
        assert_eq!(grouped(&list, "xyz1").len(), 1);
        assert!(grouped(&list, "nothing like it").is_empty());
    }

    #[test]
    fn the_download_address_is_the_usfx_zip() {
        assert_eq!(download_url("indags"), "https://ebible.org/Scriptures/indags_usfx.zip");
    }

    #[test]
    fn the_real_catalogue_has_indonesian_first_when_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ebible/translations.csv");
        let Ok(csv) = std::fs::read_to_string(path) else { return };
        let list = parse_catalogue(&csv);
        assert!(list.len() > 1000);
        let groups = grouped(&list, "");
        assert_eq!(groups[0].language, "Indonesian");
        assert!(list[groups[0].items[0]].id.starts_with("ind"));
    }
}
