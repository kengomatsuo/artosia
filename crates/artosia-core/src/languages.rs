//! The languages a church might sing in, named in the window's language. One generated asset
//! (`assets/languages.json`, written by `tools/make-languages.py` from Unicode CLDR) holds the
//! names and the map from the three-letter codes a country's language list uses.
//!
//! Codes are BCP 47 as a song stores them: `id`, `en`, `zh-Hans`, or the three-letter code for a
//! language with no two-letter one. A code CLDR does not name shows as the code itself.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::text::normalize;

#[derive(Deserialize)]
struct Asset {
    names: HashMap<String, HashMap<String, String>>,
    iso3: HashMap<String, String>,
}

fn asset() -> &'static Asset {
    static ASSET: OnceLock<Asset> = OnceLock::new();
    ASSET.get_or_init(|| serde_json::from_str(include_str!("../assets/languages.json")).unwrap_or(Asset { names: HashMap::new(), iso3: HashMap::new() }))
}

fn table(window: &str) -> Option<&'static HashMap<String, String>> {
    let asset = asset();
    asset.names.get(window).or_else(|| asset.names.get("en"))
}

/// The CLDR name of exactly this code in `window`, or in English when the window's table lacks it.
fn named(code: &str, window: &str) -> Option<&'static str> {
    let find = |table: Option<&'static HashMap<String, String>>| table.and_then(|names| names.iter().find(|(have, _)| have.eq_ignore_ascii_case(code)).map(|(_, name)| name.as_str()));
    find(table(window)).or_else(|| find(asset().names.get("en")))
}

/// The name of a language where it stands alone, on a chip or in a row of a list: its first
/// letter is a capital, as a list writes it (CLDR writes Spanish, French and Portuguese names
/// small). A code nobody named stays as it is.
pub fn name(code: &str, window: &str) -> String {
    let name = cldr_name(code, window);
    if name == code.trim() {
        return name;
    }
    let mut letters = name.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => name,
    }
}

/// The name of a language inside a sentence: as CLDR writes it, except that Vietnamese writes
/// the word before a language's name small there.
pub fn name_in_sentence(code: &str, window: &str) -> String {
    let name = cldr_name(code, window);
    match name.strip_prefix("Ti\u{1ebf}ng ") {
        Some(rest) if window.starts_with("vi") => format!("ti\u{1ebf}ng {rest}"),
        _ => name,
    }
}

/// The name of a language in the window's language. A tag that adds only a country to a named
/// language (`en-US`) reads as the language; any other code CLDR does not name reads as itself,
/// so a transliteration (`ja-Latn`) is never mistaken for its language.
fn cldr_name(code: &str, window: &str) -> String {
    let code = code.trim();
    if let Some(name) = named(code, window) {
        return name.to_string();
    }
    let mut parts = code.split(['-', '_']);
    let (first, rest) = (parts.next().unwrap_or_default(), parts.collect::<Vec<_>>());
    let region = |part: &&str| (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic())) || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()));
    if rest.len() == 1
        && rest.iter().all(region)
        && let Some(name) = named(first, window)
    {
        return name.to_string();
    }
    code.to_string()
}

/// Every language CLDR lists as spoken somewhere, as (code, name in `window`), sorted by name.
#[cfg(test)]
fn all(window: &str) -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = table("en").into_iter().flatten().map(|(code, _)| (code.clone(), name(code, window))).collect();
    list.sort_by_cached_key(|(code, name)| (normalize(name), code.clone()));
    list
}

/// A language's code as a song stores it, from the three-letter code a country's list uses
/// (`ind` gives `id`); a code with no two-letter form is its own.
pub fn from_iso3(code: &str) -> String {
    asset().iso3.get(code).cloned().unwrap_or_else(|| code.to_string())
}

/// The languages of a country as song codes, most spoken first, each once. Chinese is
/// Traditional in Taiwan, Hong Kong and Macau.
pub fn of_country(country: &str) -> Vec<String> {
    let traditional = matches!(country, "TW" | "HK" | "MO");
    let mut list: Vec<String> = Vec::new();
    for iso3 in crate::countries::languages_of(country) {
        let mut code = from_iso3(&iso3);
        if traditional && code == "zh-Hans" {
            code = "zh-Hant".into();
        }
        if code != "und" && !list.contains(&code) {
            list.push(code);
        }
    }
    list
}

// ---- The lists of languages a church and a plan keep, and what is done to them ----

/// The languages a slide shows: the plan's own list when it has one, else the church's usual.
pub fn shown<'a>(plan: Option<&'a [String]>, church: &'a [String]) -> &'a [String] {
    plan.unwrap_or(church)
}

/// Shows `code` alone, or all again when it is the one already alone.
pub fn toggle_solo(current: Option<&str>, code: &str) -> Option<String> {
    (current != Some(code)).then(|| code.to_string())
}

/// The code to use for the window's own language when picking from `all`: a window code such as
/// `pt-BR` is the list's `pt`, and `zh-Hans` is itself.
pub fn listed_for(window: &str) -> String {
    let codes: Vec<String> = asset().names.get("en").map(|names| names.keys().cloned().collect()).unwrap_or_default();
    let refs: Vec<&str> = codes.iter().map(String::as_str).collect();
    artosia_locale::pick(window, &refs).map_or_else(|| window.to_string(), str::to_string)
}

/// The languages Artosia's own window speaks, after the window's own and English, as a song
/// writes them.
const WINDOW_LANGUAGES: [&str; 9] = ["zh-Hans", "zh-Hant", "ko", "ja", "es", "pt", "fr", "vi", "id"];

/// Whether CLDR names `code` in the window's language itself, not in English or as the code.
pub fn named_in(code: &str, window: &str) -> bool {
    asset().names.get(window).is_some_and(|names| names.keys().any(|have| have.eq_ignore_ascii_case(code)))
}

/// The one order of every list of languages a person picks from, the first run's page and the
/// picker of the editor and of the dialog alike, as (code, name in `window`):
///
/// 1. what is in `taken` (already ticked or already in the song) is left out;
/// 2. the window's own language, then English;
/// 3. the picked country's languages in CLDR's order (none when `country` is empty);
/// 4. the other languages Artosia's window speaks;
/// 5. every other language by its name in the window's language, folded for matching;
/// 6. last, those CLDR does not name in the window's language (shown in English or as the code),
///    among themselves by the name shown.
///
/// `extra` are codes found in a church's songs that CLDR's list lacks.
pub fn ordered(window: &str, country: &str, taken: &[String], extra: &[&str]) -> Vec<(String, String)> {
    let mut codes: Vec<String> = Vec::new();
    let put = |code: String, codes: &mut Vec<String>| {
        if !taken.contains(&code) && !codes.contains(&code) {
            codes.push(code);
        }
    };
    let known: Vec<String> = table("en").into_iter().flatten().map(|(code, _)| code.clone()).collect();
    let in_list = |code: &str| known.iter().any(|have| have == code) || extra.contains(&code);
    for code in [listed_for(window), "en".to_string()].into_iter().chain(of_country(country)).chain(WINDOW_LANGUAGES.iter().map(|code| code.to_string())) {
        if in_list(&code) {
            put(code, &mut codes);
        }
    }
    let mut rest: Vec<String> = known.iter().cloned().chain(extra.iter().map(|code| code.trim().to_string()).filter(|code| !code.is_empty())).collect();
    rest.sort();
    rest.dedup();
    let mut rest: Vec<(bool, String, String)> = rest.into_iter().map(|code| (!named_in(&code, window), normalize(&name(&code, window)), code)).collect();
    rest.sort();
    for (_, _, code) in rest {
        put(code, &mut codes);
    }
    codes.into_iter().map(|code| (name(&code, window), code)).map(|(name, code)| (code, name)).collect()
}

/// What a first run ticks before anyone does: the window's own language.
pub fn first_run_ticks(window: &str) -> Vec<String> {
    vec![listed_for(window)]
}

/// Ticks a language, last, or unticks it; the order of the rest is kept.
pub fn tick(list: &mut Vec<String>, code: &str) {
    match list.iter().position(|have| have == code) {
        Some(at) => {
            list.remove(at);
        }
        None => list.push(code.to_string()),
    }
}

/// Moves the language at `from` to `to`, the others keeping their order.
pub fn move_to(list: &mut Vec<String>, from: usize, to: usize) {
    if from < list.len() {
        let code = list.remove(from);
        list.insert(to.min(list.len()), code);
    }
}

/// A song's languages as an editor holds them: (code, words), the first language first.
pub type Texts = Vec<(String, String)>;

/// Takes the language at `at` out of a song's texts. The only language of a song cannot go.
pub fn remove_text(texts: &mut Texts, at: usize) -> Option<(usize, (String, String))> {
    (texts.len() > 1 && at < texts.len()).then(|| (at, texts.remove(at)))
}

/// Puts back what `remove_text` took, at its place.
pub fn restore_text(texts: &mut Texts, at: usize, entry: (String, String)) {
    texts.insert(at.min(texts.len()), entry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_language_is_named_in_the_window_s_language_and_an_unknown_code_is_itself() {
        assert_eq!(name("id", "en"), "Indonesian");
        assert_eq!(name("id", "id"), "Indonesia");
        assert_eq!(name("zh-Hans", "en"), "Simplified Chinese");
        assert_eq!(name("ja", "ja"), "日本語");
        assert_eq!(name("en-US", "id"), "Inggris");
        assert_eq!(name("ja-Latn", "en"), "ja-Latn");
        assert_eq!(name("qq", "en"), "qq");
        assert_eq!(name("jv", "xx"), name("jv", "en"));
    }

    #[test]
    fn the_list_is_sorted_by_name_in_each_window_language_and_holds_what_a_church_might_sing_in() {
        for window in ["en", "id", "ja", "zh-Hans", "pt-BR", "ko"] {
            let list = all(window);
            let keys: Vec<String> = list.iter().map(|(_, name)| normalize(name)).collect();
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "{window}");
            assert!(list.len() > 400 && list.len() < 700, "{}", list.len());
        }
        let codes: Vec<String> = all("en").into_iter().map(|(code, _)| code).collect();
        for code in ["id", "en", "jv", "su", "ko", "ja", "zh-Hans", "zh-Hant", "es", "pt", "fr", "vi", "ms"] {
            assert!(codes.iter().any(|have| have == code), "{code}");
        }
        assert!(!codes.iter().any(|code| code == "zh" || code == "und"));
        // A code found in a song is listed once, under its own name or as itself.
        let with = ordered("en", "", &[], &["id", "xx-Test", "ja-Latn"]);
        assert_eq!(with.len(), codes.len() + 2);
        assert!(with.iter().any(|(code, name)| code == "xx-Test" && name == "xx-Test"));
    }

    #[test]
    fn a_country_s_languages_are_song_codes_in_order() {
        assert_eq!(of_country("ID")[..3], ["id", "jv", "su"]);
        assert_eq!(of_country("KR"), ["ko"]);
        assert_eq!(of_country("TW")[0], "zh-Hant");
        assert!(of_country("CN").contains(&"zh-Hans".to_string()));
        assert!(of_country("MY").contains(&"ms".to_string()));
        assert!(of_country("ZZ").is_empty());
        assert_eq!(from_iso3("eng"), "en");
        for country in crate::countries::names("en") {
            for code in of_country(country.0) {
                assert!(code.len() == 2 || code.len() == 3 || code.contains('-'), "{} {code}", country.0);
            }
        }
    }

    #[test]
    fn a_plan_s_own_list_wins_over_the_church_s_and_solo_toggles() {
        let church = vec!["id".to_string(), "en".to_string()];
        let own = vec!["en".to_string()];
        assert_eq!(shown(None, &church), ["id", "en"]);
        assert_eq!(shown(Some(&own), &church), ["en"]);
        assert!(shown(Some(&[]), &church).is_empty(), "an own list may be empty: the first language alone");
        assert_eq!(toggle_solo(None, "en"), Some("en".into()));
        assert_eq!(toggle_solo(Some("id"), "en"), Some("en".into()));
        assert_eq!(toggle_solo(Some("en"), "en"), None);
    }

    #[test]
    fn the_first_run_ticks_the_window_s_language() {
        assert_eq!(first_run_ticks("id"), ["id"]);
        assert_eq!(first_run_ticks("pt-BR"), ["pt"]);
        assert_eq!(first_run_ticks("zh-Hant"), ["zh-Hant"]);
    }

    fn codes_of(rows: &[(String, String)]) -> Vec<&str> {
        rows.iter().map(|(code, _)| code.as_str()).collect()
    }

    #[test]
    fn one_order_for_every_list_the_window_then_english_then_the_country_then_the_windows_languages_then_the_rest_by_name() {
        // An Indonesian window in Indonesia: Indonesian, English, the country's others, then the window's.
        let rows = ordered("id", "ID", &[], &[]);
        let codes = codes_of(&rows);
        assert_eq!(codes[..2], ["id", "en"]);
        let country: Vec<String> = of_country("ID").into_iter().filter(|code| code != "id" && code != "en" && codes.contains(&code.as_str())).collect();
        assert_eq!(codes[2..2 + country.len()], country.iter().map(String::as_str).collect::<Vec<_>>()[..]);
        let after = 2 + country.len();
        let spoken = ["zh-Hans", "zh-Hant", "ko", "ja", "es", "pt", "fr", "vi"].into_iter().filter(|code| !country.iter().any(|have| have == code));
        let spoken: Vec<&str> = spoken.collect();
        assert_eq!(codes[after..after + spoken.len()], spoken[..]);
        // Every language once, and the rest follow by name in Indonesian, the ones Indonesian lacks last.
        let rest = &rows[after + spoken.len()..];
        let mut sorted = codes.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), codes.len());
        let first_unnamed = rest.iter().position(|(code, _)| !named_in(code, "id")).expect("some language has no Indonesian name");
        assert!(rest[first_unnamed..].iter().all(|(code, _)| !named_in(code, "id")), "what CLDR does not name in the window's language comes after all that it does");
        let named: Vec<String> = rest[..first_unnamed].iter().map(|(_, name)| normalize(name)).collect();
        assert!(named.windows(2).all(|pair| pair[0] <= pair[1]));
        // A Japanese window with no country set: Japanese, English, then the window's languages, none of a country's.
        let ja = ordered("ja", "", &[], &[]);
        assert_eq!(codes_of(&ja)[..10], ["ja", "en", "zh-Hans", "zh-Hant", "ko", "es", "pt", "fr", "vi", "id"]);
        // What is taken is not offered again, and the order of the others holds.
        let taken = vec!["ja".to_string(), "en".to_string()];
        let without = ordered("ja", "", &taken, &[]);
        assert_eq!(codes_of(&without)[..3], ["zh-Hans", "zh-Hant", "ko"]);
        assert!(!codes_of(&without).iter().any(|code| *code == "ja" || *code == "en"));
        // A code found only in a song is named by itself and sorts last.
        let with = ordered("en", "", &[], &["xx-Test"]);
        assert_eq!(with.last().map(|(code, name)| (code.as_str(), name.as_str())), Some(("xx-Test", "xx-Test")));
        // The country's languages come in CLDR's order and none without a country.
        let brazil = of_country("BR");
        assert_eq!(brazil[0], "pt");
        let next = brazil.iter().find(|code| *code != "pt" && *code != "en").expect("Brazil has another language");
        assert_eq!(codes_of(&ordered("en", "BR", &[], &[]))[..3], ["en", "pt", next.as_str()]);
        assert_eq!(codes_of(&ordered("en", "", &[], &[]))[..3], ["en", "zh-Hans", "zh-Hant"]);
    }

    #[test]
    fn ticking_keeps_the_order_ticked_and_a_language_moves() {
        let mut list = first_run_ticks("id");
        tick(&mut list, "en");
        tick(&mut list, "zh-Hans");
        assert_eq!(list, ["id", "en", "zh-Hans"]);
        tick(&mut list, "id");
        tick(&mut list, "id");
        assert_eq!(list, ["en", "zh-Hans", "id"]);
        move_to(&mut list, 2, 0);
        assert_eq!(list, ["id", "en", "zh-Hans"]);
        move_to(&mut list, 0, 9);
        assert_eq!(list, ["en", "zh-Hans", "id"]);
        move_to(&mut list, 7, 0);
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn a_language_removed_from_a_song_comes_back_where_it_was() {
        let mut texts: Texts = vec![("id".into(), "satu".into()), ("en".into(), "one".into()), ("zh-Hans".into(), "yi".into())];
        let before = texts.clone();
        let taken = remove_text(&mut texts, 1).expect("removed");
        assert_eq!((texts.len(), taken.0), (2, 1));
        restore_text(&mut texts, taken.0, taken.1);
        assert_eq!(texts, before);
        // The first language may go when others remain, and then the next is first.
        let first = remove_text(&mut texts, 0).expect("removed");
        assert_eq!(texts[0].0, "en");
        restore_text(&mut texts, first.0, first.1);
        assert_eq!(texts, before);
        // The only language stays.
        let mut one: Texts = vec![("id".into(), "satu".into())];
        assert!(remove_text(&mut one, 0).is_none() && one.len() == 1);
    }
}
