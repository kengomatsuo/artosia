//! The Beblia repository's catalogue: one XML Bible per file in the root of
//! https://github.com/Beblia/Holy-Bible-XML-Format (branch `master`).
//!
//! The repository has no index. Its files are listed with GitHub's git-tree API and every file
//! is named `<Language><Edition>Bible.xml`: `IndonesianAGSBible.xml`, `Afrikaans1983Bible.xml`,
//! `ChineseSimplifiedBible.xml`. The language is the first capitalised word and the rest is the
//! edition. The file itself carries the translation's real name and its copyright line in its
//! `<bible>` tag (`bible_file::beblia_header`). A file name says too little (`IndonesianBible.xml`
//! is the Terjemahan Baru), so the names and short codes of the files read on the day of a
//! release ship in `assets/beblia-names.json` (`tools/make-beblia-names.py`) and a row shows
//! its file's own name from the first frame; a file newer than the release shows its file name
//! until its header is read.
//! Read 2026-11-11: 1,048 XML files, no licence file, a README that says "Use at your own
//! discretion, no need to ask for permission, no warranty's" (docs/bibles.md).

use crate::ebible::{Source, Translation};

pub const REPOSITORY_URL: &str = "https://github.com/Beblia/Holy-Bible-XML-Format";
/// Every path in the repository, in one unauthenticated call (60 a hour per address).
pub const TREE_URL: &str = "https://api.github.com/repos/Beblia/Holy-Bible-XML-Format/git/trees/master?recursive=1";

/// The raw file, from `raw.githubusercontent.com`.
pub fn download_url(file: &str) -> String {
    format!("https://raw.githubusercontent.com/Beblia/Holy-Bible-XML-Format/master/{file}")
}

/// The first bytes of the file, enough to read the `<bible>` tag.
pub const HEADER_BYTES: u64 = 4096;

/// What a file says about itself, read when the release was made.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Known {
    /// The name the file gives its translation: "Indonesian (Alkitab Terjemahan Baru)".
    pub name: String,
    /// The short code people call it by, from the file's own link: "TB".
    #[serde(default)]
    pub code: String,
}

fn names() -> &'static std::collections::HashMap<String, Known> {
    static NAMES: std::sync::OnceLock<std::collections::HashMap<String, Known>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| serde_json::from_str(include_str!("../assets/beblia-names.json")).unwrap_or_default())
}

/// The name and code of a file of the repository, when the release knew the file.
pub fn known(file: &str) -> Option<&'static Known> {
    names().get(file)
}

/// The same for a Bible already downloaded, whose module is named by the file's stem in small
/// letters (`indonesian.sqlite3` came from `IndonesianBible.xml`).
pub fn known_by_module(code: &str) -> Option<&'static Known> {
    names().iter().find(|(file, _)| file.trim_end_matches(".xml").trim_end_matches("Bible").eq_ignore_ascii_case(code)).map(|(_, known)| known)
}

/// ISO 639-3 codes of the languages of Indonesia that have a file here, so they sort with the
/// other languages of Indonesia (`ebible::ranking`). Names are the file names' words.
fn language_code(language: &str) -> &'static str {
    match language {
        "Indonesian" => "ind",
        "Malaysian" => "zsm",
        "Javanese" => "jav",
        "Sundanese" => "sun",
        "Balinese" => "ban",
        "Bugis" => "bug",
        "Aceh" => "ace",
        "Madurese" => "mad",
        "Sasak" => "sas",
        _ => "",
    }
}

/// Spaces between the words of an edition: `Dawro DFBE`, `Tyndale 1537`.
fn spaced(edition: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in edition.chars() {
        if let Some(p) = previous {
            let break_here = (p.is_lowercase() && c.is_uppercase()) || (p.is_alphabetic() && c.is_ascii_digit()) || (p.is_ascii_digit() && c.is_alphabetic());
            if break_here {
                out.push(' ');
            }
        }
        out.push(c);
        previous = Some(c);
    }
    out
}

/// The language and the edition a file name carries, as shown before the file is read:
/// `IndonesianAGSBible.xml` is ("Indonesian", "AGS"), `IndonesianBible.xml` ("Indonesian", "").
/// None for a name that is not `<Language><Edition>Bible.xml`.
pub fn language_and_edition(file: &str) -> Option<(String, String)> {
    let stem = file.strip_suffix("Bible.xml")?;
    let mut chars = stem.chars();
    let first = chars.next().filter(char::is_ascii_uppercase)?;
    let mut language = first.to_string();
    let mut rest = chars.as_str();
    while let Some(c) = rest.chars().next().filter(char::is_ascii_lowercase) {
        language.push(c);
        rest = &rest[c.len_utf8()..];
    }
    if language.len() < 2 || stem.contains(|c: char| !c.is_ascii_alphanumeric()) {
        return None;
    }
    Some((language, spaced(rest)))
}

/// The Bibles in a git-tree answer: the XML files in the repository's root. A file that is not
/// named the way the repository names its Bibles is left out.
pub fn parse_tree(json: &str) -> Vec<Translation> {
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        size: u64,
    }
    #[derive(serde::Deserialize)]
    struct Tree {
        #[serde(default)]
        tree: Vec<Entry>,
    }
    let Ok(tree) = serde_json::from_str::<Tree>(json) else { return Vec::new() };
    tree.tree
        .into_iter()
        .filter(|entry| entry.kind == "blob" && !entry.path.contains('/'))
        .filter_map(|entry| {
            let (language, edition) = language_and_edition(&entry.path)?;
            let stem = entry.path.trim_end_matches(".xml").trim_end_matches("Bible").to_string();
            // What the file name alone says, kept as another name so a person who knew the row
            // by it still finds it.
            let label = if edition.is_empty() { language.clone() } else { format!("{language} {edition}") };
            let known = known(&entry.path);
            Some(Translation {
                source: Source::Beblia,
                // The id is the file's stem, which cannot meet an eBible id: those are lowercase.
                id: stem.clone(),
                language_code: language_code(&language).to_string(),
                language: language.clone(),
                language_english: language.clone(),
                title: known.map_or_else(|| label.clone(), |known| known.name.clone()),
                code: known.map(|known| known.code.clone()).filter(|code| !code.is_empty()).unwrap_or_else(|| edition.clone()),
                also: format!("{label} {stem}"),
                file: entry.path,
                size: entry.size,
                ..Translation::default()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE: &str = r#"{"sha":"x","truncated":false,"tree":[
        {"path":"README.md","type":"blob","size":300},
        {"path":".gitattributes","type":"blob","size":10},
        {"path":"IndonesianBible.xml","type":"blob","size":5000000},
        {"path":"IndonesianAGSBible.xml","type":"blob","size":1400000},
        {"path":"Afrikaans1983Bible.xml","type":"blob","size":4000000},
        {"path":"ChineseSimplifiedNewBible.xml","type":"blob","size":3000000},
        {"path":"AmharicDawroDFBEBible.xml","type":"blob","size":900000},
        {"path":"tools","type":"tree"},
        {"path":"tools/django_importer/README.md","type":"blob","size":9},
        {"path":"Not a bible.xml","type":"blob","size":9}]}"#;

    #[test]
    fn a_file_name_gives_a_language_and_an_edition() {
        assert_eq!(language_and_edition("IndonesianBible.xml"), Some(("Indonesian".into(), String::new())));
        assert_eq!(language_and_edition("IndonesianAGSBible.xml"), Some(("Indonesian".into(), "AGS".into())));
        assert_eq!(language_and_edition("Afrikaans1983Bible.xml"), Some(("Afrikaans".into(), "1983".into())));
        assert_eq!(language_and_edition("AmharicDawroDFBEBible.xml"), Some(("Amharic".into(), "Dawro DFBE".into())));
        assert_eq!(language_and_edition("EnglishTyndale1537Bible.xml"), Some(("English".into(), "Tyndale 1537".into())));
        assert_eq!(language_and_edition("README.md"), None);
        assert_eq!(language_and_edition("bibleBible.xml"), None);
    }

    #[test]
    fn only_bibles_in_the_root_are_listed() {
        let list = parse_tree(TREE);
        let files: Vec<&str> = list.iter().map(|item| item.file.as_str()).collect();
        assert_eq!(files, ["IndonesianBible.xml", "IndonesianAGSBible.xml", "Afrikaans1983Bible.xml", "ChineseSimplifiedNewBible.xml", "AmharicDawroDFBEBible.xml"]);
        // A file the release read shows the name and the short code the file gives itself.
        assert_eq!((list[0].title.as_str(), list[0].code.as_str()), ("Indonesian (Alkitab Terjemahan Baru)", "TB"));
        assert_eq!((list[1].title.as_str(), list[1].code.as_str()), ("Indonesian AGS (Alkitab Gratis untuk Semua)", "AGS"));
        assert_eq!((list[1].id.as_str(), list[1].size, list[1].language_code.as_str()), ("IndonesianAGS", 1_400_000, "ind"));
        // One that came after the release shows what its file name says.
        let later = parse_tree(r#"{"tree":[{"path":"ZzzmadeNew2099Bible.xml","type":"blob","size":9}]}"#);
        assert_eq!((later[0].title.as_str(), later[0].code.as_str()), ("Zzzmade New 2099", "New 2099"));
        assert!(list.iter().all(|item| item.source == Source::Beblia && item.scope() == 4));
        assert!(parse_tree("not json").is_empty());
    }

    /// The Terjemahan Baru's file is `IndonesianBible.xml`: a person finds it by the name it
    /// gives itself, by the code people call it by, and by what its file name says.
    #[test]
    fn a_bible_is_found_by_its_own_name_its_code_and_its_file_name() {
        use crate::ebible::grouped;
        let list = parse_tree(TREE);
        let found = |query: &str| -> Vec<&str> { grouped(&list, query, &[]).iter().flat_map(|group| group.items.iter().map(|at| list[*at].file.as_str())).collect() };
        assert_eq!(found("terjemahan baru"), ["IndonesianBible.xml"]);
        assert!(found("tb").contains(&"IndonesianBible.xml"));
        assert_eq!(found("gratis"), ["IndonesianAGSBible.xml"]);
        assert_eq!(found("ags"), ["IndonesianAGSBible.xml"]);
        assert_eq!(found("dawro"), ["AmharicDawroDFBEBible.xml"]);
        // A Bible already downloaded is known by its module's name.
        assert_eq!(known_by_module("indonesian").map(|known| known.code.as_str()), Some("TB"));
        assert_eq!(known_by_module("no such module"), None);
        // The names file holds nearly every file of the repository.
        assert!(names().len() > 1000, "{} names", names().len());
    }

    #[test]
    fn the_download_address_is_the_raw_file() {
        assert_eq!(download_url("IndonesianAGSBible.xml"), "https://raw.githubusercontent.com/Beblia/Holy-Bible-XML-Format/master/IndonesianAGSBible.xml");
    }

    #[test]
    fn beblia_joins_the_indonesian_group_of_ebible() {
        use crate::ebible::{grouped, parse_catalogue};
        let csv = "languageCode,translationId,languageName,languageNameInEnglish,title,description,Redistributable,Copyright,OTbooks,NTbooks,downloadable\n\
                   ind,indmade,Bahasa Buatan,Indonesian,Alkitab Buatan,x,True,Made up,0,27,True\n";
        let mut list = parse_catalogue(csv);
        list.extend(parse_tree(TREE));
        let groups = grouped(&list, "", &crate::ebible::ranking("ID"));
        assert_eq!(groups[0].language, "Indonesian");
        let ids: Vec<&str> = groups[0].items.iter().map(|at| list[*at].id.as_str()).collect();
        assert_eq!(ids, ["indmade", "Indonesian", "IndonesianAGS"]);
        assert_eq!(groups.len(), 4);
        // A filter that leaves only the repository's files keeps the language's own name.
        let narrowed = grouped(&list, "ags", &crate::ebible::ranking("ID"));
        assert_eq!((narrowed[0].language.as_str(), narrowed[0].local.as_str()), ("Indonesian", "Bahasa Buatan"));
    }
}
