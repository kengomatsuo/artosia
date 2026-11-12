//! The countries and regions a church may be in, named in the window's language, and the
//! languages each speaks. One generated asset (`assets/countries.json`, written by
//! `tools/make-countries.py` from Unicode CLDR) holds both.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::text::normalize;

#[derive(Deserialize)]
struct Asset {
    names: HashMap<String, HashMap<String, String>>,
    languages: HashMap<String, Vec<String>>,
}

fn asset() -> &'static Asset {
    static ASSET: OnceLock<Asset> = OnceLock::new();
    ASSET.get_or_init(|| serde_json::from_str(include_str!("../assets/countries.json")).unwrap_or(Asset { names: HashMap::new(), languages: HashMap::new() }))
}

/// The names of `language` (a window language's code, such as `pt-BR`), or English when the
/// asset has none for it.
fn table(language: &str) -> Option<&'static HashMap<String, String>> {
    let asset = asset();
    asset.names.get(language).or_else(|| asset.names.get("en"))
}

/// Every country as (two capital letters, name in `language`), sorted by name.
pub fn names(language: &str) -> Vec<(&'static str, &'static str)> {
    let mut list: Vec<(&'static str, &'static str)> = table(language).into_iter().flatten().map(|(code, name)| (code.as_str(), name.as_str())).collect();
    list.sort_by_cached_key(|(code, name)| (normalize(name), *code));
    list
}

/// The name of one country in `language`; None for a code that is no country.
pub fn name(country: &str, language: &str) -> Option<&'static str> {
    table(language)?.get(country).map(String::as_str)
}

/// The languages of a country as ISO 639-3 codes, most spoken first. Empty for a code that is no country.
pub fn languages_of(country: &str) -> Vec<String> {
    asset().languages.get(country).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_country_has_two_capital_letters_and_no_region_is_listed() {
        let list = names("en");
        assert!(list.len() > 190 && list.len() < 260, "{}", list.len());
        for (code, name) in &list {
            assert!(code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase()), "{code}");
            assert!(!name.is_empty());
        }
        for region in ["001", "EU", "UN", "ZZ", "419", "EZ"] {
            assert!(!list.iter().any(|(code, _)| *code == region), "{region} is no country");
        }
        assert!(list.iter().any(|(code, _)| *code == "XK"), "Kosovo has a two-letter code of its own");
    }

    #[test]
    fn names_are_sorted_by_name_in_the_language_asked() {
        for language in ["en", "id", "es", "pt-BR", "fr", "vi", "ja", "ko", "zh-Hans", "zh-Hant"] {
            let list = names(language);
            let keys: Vec<String> = list.iter().map(|(_, name)| normalize(name)).collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "{language}");
        }
        assert_eq!(name("ID", "id"), Some("Indonesia"));
        assert_eq!(name("DE", "en"), Some("Germany"));
        assert_eq!(name("DE", "id"), Some("Jerman"));
        assert_eq!(name("BR", "pt-BR"), Some("Brasil"));
    }

    #[test]
    fn a_language_without_names_reads_english() {
        assert_eq!(names("xx"), names("en"));
        assert_eq!(name("ID", "xx"), Some("Indonesia"));
        assert_eq!(name("ZZ", "en"), None);
        assert_eq!(name("001", "en"), None);
    }

    #[test]
    fn a_country_lists_its_languages_most_spoken_first() {
        assert_eq!(languages_of("ID")[..3], ["ind", "jav", "sun"]);
        assert_eq!(languages_of("BR")[0], "por");
        assert_eq!(languages_of("KR"), ["kor"]);
        // eBible writes Malay and Mandarin by their members' codes.
        assert!(languages_of("MY").contains(&"zlm".to_string()));
        assert!(languages_of("CN").contains(&"cmn".to_string()));
        assert!(languages_of("ZZ").is_empty());
        for (code, _) in names("en") {
            for language in languages_of(code) {
                assert!(language.len() == 3 && language.chars().all(|c| c.is_ascii_lowercase()), "{code} {language}");
            }
        }
    }
}
