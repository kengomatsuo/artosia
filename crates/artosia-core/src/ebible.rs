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

/// Where a translation is listed and downloaded from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    Ebible,
    /// The Beblia repository of XML Bibles (`beblia.rs`).
    Beblia,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Translation {
    pub source: Source,
    /// The file in the Beblia repository, empty for eBible.
    pub file: String,
    /// The file's size in bytes when the list gives it (Beblia), else 0.
    pub size: u64,
    pub id: String,
    /// eBible's language code; its first three letters are the ISO 639-3 code.
    pub language_code: String,
    /// The language in its own words.
    pub language: String,
    pub language_english: String,
    pub title: String,
    /// The short code people call it by ("TB", "AYT"), when the source gives one; shown beside
    /// the title and matched by the filter.
    pub code: String,
    /// Other names it is found by, for the filter only: a second title, the file's own label.
    pub also: String,
    pub description: String,
    pub copyright: String,
    pub old_books: u16,
    pub new_books: u16,
}

impl Translation {
    pub fn whole_bible(&self) -> bool {
        self.old_books >= 39 && self.new_books >= 27
    }

    /// 0 whole Bible, 1 New Testament, 2 Old Testament, 3 some books, 4 not known before the
    /// download (Beblia's list has file names and sizes, not book counts).
    pub fn scope(&self) -> u8 {
        if self.source == Source::Beblia {
            return 4;
        }
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

/// The short code inside an eBible id: the id is the language's three letters and then the
/// translation's own letters (`indayt` is AYT). Empty when the id is the language alone.
pub fn short_code(id: &str, language_code: &str) -> String {
    let language: String = language_code.chars().take(3).collect();
    let rest = match id.strip_prefix(language.as_str()) {
        Some(rest) if language.len() == 3 => rest,
        _ => return String::new(),
    };
    rest.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_uppercase()
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
    let short_title = column("shortTitle");
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
            id: translation_id.clone(),
            language_code: get(&row, code),
            language: get(&row, local),
            language_english: get(&row, english),
            title: get(&row, title),
            code: short_code(&translation_id, &get(&row, code)),
            also: get(&row, short_title),
            description: get(&row, description),
            copyright: get(&row, copyright),
            old_books: books(old),
            new_books: books(new),
            ..Translation::default()
        });
    }
    out
}

/// ISO 639-3 codes of the languages of Indonesia that eBible might list, Indonesian first.
/// Hand ordered: Indonesia's own order is a church's, not the census's. The code is the first
/// three letters of eBible's language code (`zlmKSZI` is Malay).
const INDONESIA: [&str; 31] = [
    "ind", "zlm", "zsm", "msa", "mkn", "pmy", "abs", "jav", "jvn", "sun", "ban", "min", "bug", "mak", "bjn", "ace", "bbc", "btd", "btx", "bts", "mad", "ljp",
    "sas", "mqy", "nij", "kje", "bew", "mui", "kvr", "tet", "pse",
];

/// The languages whose Bibles a church in `country` (two capital letters) sees first, as ISO 639-3
/// codes, most wanted first: Indonesia keeps its hand-ordered list, any other country the
/// languages Unicode CLDR gives it, most spoken first.
pub fn ranking(country: &str) -> Vec<String> {
    if country == "ID" {
        return INDONESIA.iter().map(|code| (*code).to_string()).collect();
    }
    crate::countries::languages_of(country)
}

/// Where a language ranks in `ranking`, None for a language outside it.
fn rank_of(language_code: &str, ranking: &[String]) -> Option<usize> {
    let code: String = language_code.chars().take(3).collect::<String>().to_lowercase();
    ranking.iter().position(|known| *known == code)
}

/// The translations of one language.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub language: String,
    pub local: String,
    /// Its language is one of the picked country's.
    pub of_country: bool,
    /// Indexes into the list that was grouped.
    pub items: Vec<usize>,
}

/// Groups the translations that hold every word of `query` (in a language name, a title, a
/// short code, another name or an id), the languages of `ranking` (`ranking(country)`) first in its order, then A to Z. Within a
/// language: whole Bibles first.
pub fn grouped(list: &[Translation], query: &str, ranking: &[String]) -> Vec<Group> {
    let words: Vec<String> = normalize(query).split_whitespace().map(str::to_string).collect();
    let mut groups: Vec<(Option<usize>, Group)> = Vec::new();
    for (at, item) in list.iter().enumerate() {
        let hay = normalize(&format!("{} {} {} {} {} {} {} {}", item.language_english, item.language, item.title, item.code, item.also, item.id, item.description, item.language_code));
        if !words.iter().all(|word| hay.contains(word.as_str())) {
            continue;
        }
        let name = if item.language_english.is_empty() { item.language.clone() } else { item.language_english.clone() };
        // One language may reach the list under several codes; the shown name joins them.
        match groups.iter_mut().find(|(_, group)| group.language == name) {
            Some((_, group)) => group.items.push(at),
            None => {
                let rank = rank_of(&item.language_code, ranking);
                // A file of the repository names its language in English only: the heading takes
                // the language's own name from another translation of it, shown or not, so it
                // reads the same whatever the filter leaves under it.
                let local = if item.language == name {
                    list.iter().find(|other| other.language_english == name && other.language != name && !other.language.is_empty()).map_or_else(|| item.language.clone(), |other| other.language.clone())
                } else {
                    item.language.clone()
                };
                groups.push((rank, Group { language: name, local, of_country: rank.is_some(), items: vec![at] }));
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
    fn the_languages_of_the_picked_country_come_first() {
        let list = parse_catalogue(CSV);
        let indonesia = ranking("ID");
        let names: Vec<String> = grouped(&list, "", &indonesia).into_iter().map(|group| group.language).collect();
        assert_eq!(names, ["Indonesian", "Malay", "Aaaish", "Zedish"]);
        let groups = grouped(&list, "", &indonesia);
        assert!(groups[0].of_country && groups[1].of_country && !groups[2].of_country);
        // A whole Bible leads its language, whatever the titles say.
        assert_eq!(list[groups[0].items[0]].id, "indwhole");
        // Another country ranks its own languages: Malaysia lists Malay before Indonesian, and a
        // country whose languages are not in the list leaves it A to Z.
        let malaysia: Vec<String> = grouped(&list, "", &ranking("MY")).into_iter().map(|group| group.language).collect();
        assert_eq!(malaysia[0], "Malay");
        let none: Vec<String> = grouped(&list, "", &[]).into_iter().map(|group| group.language).collect();
        assert_eq!(none, ["Aaaish", "Indonesian", "Malay", "Zedish"]);
        assert!(grouped(&list, "", &[]).iter().all(|group| !group.of_country));
    }

    #[test]
    fn indonesia_keeps_its_hand_ordered_list_and_others_take_the_census() {
        let indonesia = ranking("ID");
        assert_eq!(indonesia.len(), 31);
        assert_eq!(&indonesia[..3], ["ind", "zlm", "zsm"], "hand order, not CLDR's, which has Javanese second");
        assert_eq!(ranking("BR")[0], "por");
        assert!(ranking("ZZ").is_empty());
    }

    #[test]
    fn a_search_reads_language_or_name() {
        let list = parse_catalogue(CSV);
        assert_eq!(grouped(&list, "indonesian", &[]).len(), 1);
        assert_eq!(grouped(&list, "BAHASA buatan", &[]).len(), 1);
        assert_eq!(grouped(&list, "zed bible", &[]).len(), 1);
        assert_eq!(grouped(&list, "xyz1", &[]).len(), 1);
        assert!(grouped(&list, "nothing like it", &[]).is_empty());
    }

    /// An eBible id is the language's three letters and the translation's own: the rest is the
    /// code people know, and a second title in the file is another name to find it by.
    #[test]
    fn a_short_code_and_a_second_title_are_read_and_found() {
        assert_eq!((short_code("indayt", "ind"), short_code("ind", "ind"), short_code("eng-kjv2006", "eng"), short_code("xyz1", "abc")), ("AYT".to_string(), String::new(), "KJV2006".to_string(), String::new()));
        let csv = "languageCode,translationId,languageName,languageNameInEnglish,title,shortTitle,description,Redistributable,Copyright,OTbooks,NTbooks,downloadable\n\
                   ind,indzed,Bahasa Buatan,Indonesian,Alkitab Buatan,Made Up Short Name,x,True,Made up,39,27,True\n";
        let list = parse_catalogue(csv);
        assert_eq!((list[0].code.as_str(), list[0].also.as_str()), ("ZED", "Made Up Short Name"));
        assert_eq!(grouped(&list, "zed", &[]).len(), 1);
        assert_eq!(grouped(&list, "short name", &[]).len(), 1);
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
        let groups = grouped(&list, "", &ranking("ID"));
        assert_eq!(groups[0].language, "Indonesian");
        assert!(list[groups[0].items[0]].id.starts_with("ind"));
    }
}
