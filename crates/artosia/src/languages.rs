//! The languages the operator window can speak, and how a language other than English and
//! Indonesian is read. English and Indonesian are written in `strings.rs`, because they are the
//! two that were reviewed. Every other language is a table, `docs/i18n/<code>.json`, with the
//! shape of `docs/i18n/id.json`: a key to `{"text": ...}`. A missing key falls back to English.
//!
//! A table entry's `text` is one of:
//! - a string with named placeholders, `"{count} slides"`;
//! - plural forms, `{"zero": .., "one": .., "other": ..}` (`many` is read as `other`);
//! - named cases, `{"none_lost": .., "some_lost": ..}`, each a string or plural forms.
//!
//! To add a language: follow the steps in `docs/i18n/README.md`, then add one line to
//! `LANGUAGES` below.
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub struct Language {
    /// The tag settings and `--language` use.
    pub code: &'static str,
    /// The language in its own words, as the choice shows it.
    pub name: &'static str,
    /// The table, for a language written outside `strings.rs`.
    pub table: Option<&'static str>,
    /// Other words in the search that find it, so a volunteer may type it in any language.
    pub aliases: &'static [&'static str],
    /// False for a language that only a test or `--language` may select.
    pub listed: bool,
}

/// Positions in `LANGUAGES` that `strings.rs` writes out by hand.
pub const ENGLISH: usize = 0;
pub const INDONESIAN: usize = 1;

pub static LANGUAGES: &[Language] = &[
    Language { code: "en", name: crate::strings::LANGUAGE_ENGLISH, table: None, aliases: &["indonesia", "indonesian", "inggris", "english"], listed: true },
    // Indonesian is a table like any language that comes after it, written from the briefs
    // by a writer who never saw the English (`docs/i18n/README.md`).
    Language { code: "id", name: crate::strings::LANGUAGE_INDONESIAN, table: Some(include_str!("../../../docs/i18n/id.json")), aliases: &["indonesia", "indonesian", "inggris", "english"], listed: true },
    // Written from the briefs like Indonesian, each by its own writer, read back by a second
    // reader on rendered screens, and open for native correction (owner's call, 2026-11-12).
    Language { code: "zh-Hans", name: "\u{7b80}\u{4f53}\u{4e2d}\u{6587}", table: Some(include_str!("../../../docs/i18n/zh-Hans.json")), aliases: &["chinese", "mandarin", "simplified", "tionghoa", "cina", "\u{4e2d}\u{6587}"], listed: true },
    Language { code: "zh-Hant", name: "\u{7e41}\u{9ad4}\u{4e2d}\u{6587}", table: Some(include_str!("../../../docs/i18n/zh-Hant.json")), aliases: &["chinese", "mandarin", "traditional", "tionghoa", "cina", "\u{4e2d}\u{6587}"], listed: true },
    Language { code: "ko", name: "\u{d55c}\u{ad6d}\u{c5b4}", table: Some(include_str!("../../../docs/i18n/ko.json")), aliases: &["korean", "korea"], listed: true },
    Language { code: "ja", name: "\u{65e5}\u{672c}\u{8a9e}", table: Some(include_str!("../../../docs/i18n/ja.json")), aliases: &["japanese", "jepang"], listed: true },
    Language { code: "es", name: "Espa\u{f1}ol", table: Some(include_str!("../../../docs/i18n/es.json")), aliases: &["spanish", "spanyol", "espanol"], listed: true },
    Language { code: "pt-BR", name: "Portugu\u{ea}s (Brasil)", table: Some(include_str!("../../../docs/i18n/pt-BR.json")), aliases: &["portuguese", "portugis", "portugues", "brazil"], listed: true },
    Language { code: "fr", name: "Fran\u{e7}ais", table: Some(include_str!("../../../docs/i18n/fr.json")), aliases: &["french", "prancis", "francais"], listed: true },
    Language { code: "vi", name: "Ti\u{1ebf}ng Vi\u{1ec7}t", table: Some(include_str!("../../../docs/i18n/vi.json")), aliases: &["vietnamese", "vietnam", "tieng viet"], listed: true },
    #[cfg(any(test, debug_assertions))]
    Language { code: "xx", name: "Pseudo", table: Some(include_str!("../testdata/pseudo.json")), aliases: &[], listed: false },
];

pub fn index_of(code: &str) -> Option<usize> {
    LANGUAGES.iter().position(|language| language.code == code)
}

/// The languages a person may choose, with their positions.
pub fn listed() -> impl Iterator<Item = (usize, &'static Language)> {
    LANGUAGES.iter().enumerate().filter(|(_, language)| language.listed)
}

/// The language a settings file means: its code if known, else the old yes or no for Indonesian.
pub fn from_settings(code: &str, legacy_indonesian: bool) -> &'static str {
    match index_of(code) {
        Some(at) => LANGUAGES[at].code,
        None if legacy_indonesian => "id",
        None => "en",
    }
}

type Table = HashMap<String, Value>;

fn tables() -> &'static Vec<Table> {
    static TABLES: OnceLock<Vec<Table>> = OnceLock::new();
    TABLES.get_or_init(|| {
        LANGUAGES
            .iter()
            .map(|language| {
                let parsed: Value = language.table.and_then(|json| serde_json::from_str(json).ok()).unwrap_or(Value::Null);
                let mut table = Table::new();
                for (key, entry) in parsed.as_object().into_iter().flatten() {
                    if let Some(text) = entry.get("text") {
                        table.insert(key.clone(), text.clone());
                    }
                }
                table
            })
            .collect()
    })
}

/// How many forms a count has in a language: the one small place for plural rules. A language
/// not listed counts like English: "one" for 1 and "other" for the rest.
pub fn plural_category(code: &str, count: usize) -> &'static str {
    // The rule belongs to the language, whatever script or country its code names.
    match code.split('-').next().unwrap_or(code) {
        // No plural: one form for every count.
        "id" | "ms" | "ja" | "ko" | "zh" | "vi" | "th" => "other",
        // Zero counts as singular.
        "fr" | "pt" => if count <= 1 { "one" } else { "other" },
        _ => if count == 1 { "one" } else { "other" },
    }
}

fn pick(lang: usize, node: &'static Value, count: Option<usize>) -> Option<&'static str> {
    match node {
        Value::String(text) => Some(text),
        Value::Object(forms) => {
            let count = count.unwrap_or(usize::MAX);
            let wanted = if count == 0 && forms.contains_key("zero") { "zero" } else { plural_category(LANGUAGES[lang].code, count) };
            forms.get(wanted).or_else(|| forms.get("other")).or_else(|| forms.get("many")).and_then(Value::as_str)
        }
        _ => None,
    }
}

/// A word with a number in front of it: which noun, and how many.
pub enum Arg {
    Text(String),
    Counted(&'static str, usize),
}

pub fn text_arg(value: impl ToString) -> Arg {
    Arg::Text(value.to_string())
}

pub fn counted(noun: &'static str, count: usize) -> Arg {
    Arg::Counted(noun, count)
}

fn fill(template: &str, values: &[(&str, String)]) -> String {
    // One pass, so a value that itself contains a placeholder is left as it is.
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}').and_then(|close| values.iter().find(|(name, _)| *name == &after[..close]).map(|(_, value)| (close, value))) {
            Some((close, value)) => {
                out.push_str(value);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The sentence of a table language for `key`, or None when `lang` has no such entry.
/// `case` picks a named case, `count` picks the plural form, and `args` fill the placeholders.
/// A counted noun is the table's own phrase (`noun.songs`: "{count} songs") with its number.
pub fn sentence(lang: usize, key: &str, case: Option<&str>, count: Option<usize>, args: &[(&str, Arg)]) -> Option<String> {
    let mut node = tables().get(lang)?.get(key)?;
    if let Some(case) = case {
        node = node.as_object()?.get(case)?;
    }
    let template = pick(lang, node, count)?;
    let mut values: Vec<(&str, String)> = Vec::new();
    if let Some(count) = count {
        values.push(("count", count.to_string()));
    }
    for (name, arg) in args {
        let value = match arg {
            Arg::Text(text) => text.clone(),
            Arg::Counted(noun, count) => match tables().get(lang).and_then(|table| table.get(&format!("noun.{noun}"))).and_then(|node| pick(lang, node, Some(*count))) {
                Some(phrase) => fill(phrase, &[("count", count.to_string())]),
                None => count.to_string(),
            },
        };
        values.push((name, value));
    }
    Some(fill(template, &values))
}

/// Teaches the song reader every table's names for the kinds of section, once, so a marker
/// written while the window spoke one language is read while it speaks another.
pub fn teach_markers() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        for lang in 0..LANGUAGES.len() {
            if !has_table(lang) {
                continue;
            }
            let words = ["verse", "pre_chorus", "chorus", "bridge", "intro", "ending", "other"].map(|kind| text(lang, &format!("section.{kind}")).unwrap_or_default().to_string());
            artosia_core::song::learn_marker_words(&words);
        }
    });
}

/// Whether a language is written in a table. English never is; a language without a table is
/// written in `strings.rs`.
pub fn has_table(lang: usize) -> bool {
    LANGUAGES.get(lang).is_some_and(|language| language.table.is_some())
}

pub fn text(lang: usize, key: &str) -> Option<&'static str> {
    pick(lang, tables().get(lang)?.get(key)?, None)
}

type Rows = &'static [(&'static str, &'static str)];

/// A list of pairs read row by row (`NAME[3].0`, `NAME[3].1`); a row the table lacks is English.
pub fn rows(lang: usize, name: &'static str, english: Rows) -> Rows {
    static CACHE: OnceLock<Mutex<HashMap<(usize, &'static str), Rows>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.entry((lang, name)).or_insert_with(|| {
        let built: Vec<(&'static str, &'static str)> = english
            .iter()
            .enumerate()
            .map(|(i, (first, second))| (text(lang, &format!("{name}[{i}].0")).unwrap_or(first), text(lang, &format!("{name}[{i}].1")).unwrap_or(second)))
            .collect();
        Box::leak(built.into_boxed_slice())
    })
}

/// The twelve month names, each English where the table lacks it.
pub fn months(lang: usize, english: &'static [&'static str; 12]) -> &'static [&'static str; 12] {
    static CACHE: OnceLock<Mutex<HashMap<usize, &'static [&'static str; 12]>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.entry(lang).or_insert_with(|| Box::leak(Box::new(std::array::from_fn(|i| text(lang, &format!("MONTHS[{i}]")).unwrap_or(english[i])))))
}

/// The seven names of the days of the week, Monday first, each English where the table lacks it.
pub fn weekdays(lang: usize, english: &'static [&'static str; 7]) -> &'static [&'static str; 7] {
    static CACHE: OnceLock<Mutex<HashMap<usize, &'static [&'static str; 7]>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    cache.entry(lang).or_insert_with(|| Box::leak(Box::new(std::array::from_fn(|i| text(lang, &format!("WEEKDAYS[{i}]")).unwrap_or(english[i])))))
}

/// A seconds count as a clock, "1:05", for the sentences that carry one.
pub fn clock(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings as t;
    use artosia_core::song::{SectionKind, Tag};

    fn pseudo() {
        assert!(t::set_language("xx"));
    }

    #[test]
    fn a_third_language_reaches_a_text_key_and_falls_back_to_english_for_a_missing_one() {
        pseudo();
        assert_eq!(t::BTN_SAVE(), "[xx] Save");
        assert_eq!(t::BTN_DELETE(), "Delete");
    }

    #[test]
    fn a_third_language_has_its_own_plural_forms_and_cases() {
        pseudo();
        assert_eq!((t::slides(0), t::slides(1), t::slides(2)), ("[xx] no slides".to_string(), "[xx] 1 slide".to_string(), "[xx] 2 slides".to_string()));
        assert_eq!(t::phones(1), "[xx] 1 phone");
        assert_eq!(t::pick_chapter("Mark"), "[xx] Mark: chapter?");
        assert_eq!(t::import_done(2, 1), "[xx] +2 tunes +1 book");
        assert_eq!(t::import_summary(3, 1, 2), "[xx] added 3 tunes, updated 1 tune; kept 2 tunes");
        assert_eq!(t::import_summary(0, 0, 0), "[xx] nothing new");
        assert_eq!((t::stream_live(65, 0), t::stream_live(65, 1), t::stream_live(65, 4)), ("[xx] live 1:05".to_string(), "[xx] live 1:05, 1 frame lost".to_string(), "[xx] live 1:05, 4 frames lost".to_string()));
        assert_eq!(t::stage_next("{label}"), "[xx] then {label}");
        // A sentence the table lacks is English.
        assert_eq!(t::countdown_for(2), "Countdown, 2 minutes");
    }

    #[test]
    fn a_third_language_reads_array_rows_and_sections_and_falls_back_row_by_row() {
        pseudo();
        assert_eq!(t::KEYS_SHOWING_ROWS()[0], ("Down Space", "[xx] go on"));
        assert_eq!(t::KEYS_SHOWING_ROWS()[1], ("Up", "Previous slide"));
        assert_eq!(t::ANNOUNCEMENT_TEMPLATES()[0], ("[xx] Hello", "We are glad you are here."));
        assert_eq!((t::MONTHS()[0], t::MONTHS()[1]), ("Xanuary", "February"));
        let tag = |kind, number| t::section_label(Tag::new(kind, number));
        assert_eq!((tag(SectionKind::Chorus, 1), tag(SectionKind::Chorus, 2), tag(SectionKind::Verse, 2), tag(SectionKind::Bridge, 1)), ("Refrain".to_string(), "2. Refrain".to_string(), "2. Strophe".to_string(), "Bridge".to_string()));
    }

    #[test]
    fn english_and_indonesian_are_not_changed_by_the_registry() {
        assert!(t::set_language("id"));
        assert_eq!((t::BTN_SAVE(), t::slides(2)), ("Simpan", "2 slide".to_string()));
        assert!(!t::set_language("zz"));
        assert_eq!(t::language_code(), "id");
        assert!(t::set_language("en"));
        assert_eq!(t::BTN_SAVE(), "Save");
    }

    #[test]
    fn the_registry_lists_each_language_once_under_its_own_name() {
        let codes: Vec<&str> = LANGUAGES.iter().map(|language| language.code).collect();
        for (i, code) in codes.iter().enumerate() {
            assert!(!codes[..i].contains(code), "{code} twice");
        }
        assert_eq!((LANGUAGES[ENGLISH].code, LANGUAGES[INDONESIAN].code), ("en", "id"));
        let names: Vec<&str> = listed().map(|(_, language)| language.name).collect();
        assert_eq!(names.len(), 10);
        assert_eq!(names[..2], ["English", "Bahasa Indonesia"]);
        assert_eq!(index_of("pt-BR").map(|at| LANGUAGES[at].name), Some("Portugu\u{ea}s (Brasil)"));
    }

    #[test]
    fn every_table_is_valid_json_in_the_shape_of_the_reviewed_one() {
        for language in LANGUAGES {
            let Some(table) = language.table else { continue };
            let parsed: Value = serde_json::from_str(table).unwrap_or_else(|error| panic!("{}: {error}", language.code));
            for (key, entry) in parsed.as_object().into_iter().flatten() {
                assert!(entry.get("text").is_some(), "{} {key} has no text", language.code);
            }
        }
    }

    #[test]
    fn plural_rules_are_per_language() {
        assert_eq!([plural_category("en", 1), plural_category("en", 0), plural_category("en", 2)], ["one", "other", "other"]);
        assert_eq!([plural_category("id", 1), plural_category("id", 2)], ["other", "other"]);
        assert_eq!([plural_category("fr", 0), plural_category("fr", 1), plural_category("fr", 2)], ["one", "one", "other"]);
        assert_eq!([plural_category("pt-BR", 0), plural_category("pt-BR", 2)], ["one", "other"]);
        assert_eq!([plural_category("zh-Hant", 1), plural_category("zh-Hans", 5)], ["other", "other"]);
    }

    /// The width a text takes, in Latin letters: a Chinese, Japanese or Korean character is two.
    fn width(text: &str) -> usize {
        text.chars().map(|c| if matches!(c as u32, 0x1100..=0x11FF | 0x2E80..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0xFF00..=0xFF60) { 2 } else { 1 }).sum()
    }

    fn texts(entry: &Value) -> Vec<&str> {
        match entry {
            Value::String(text) => vec![text.as_str()],
            Value::Object(forms) => forms.values().flat_map(texts).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn every_table_answers_every_brief_inside_its_width_with_its_variables() {
        let briefs: Value = serde_json::from_str(include_str!("../../../docs/i18n/brief.json")).unwrap_or_default();
        let mut wrong = Vec::new();
        for language in LANGUAGES.iter().filter(|language| language.listed) {
            let Some(table) = language.table else { continue };
            let table: Value = serde_json::from_str(table).unwrap_or_default();
            let code = language.code;
            for brief in briefs.as_array().into_iter().flatten() {
                let key = brief["key"].as_str().unwrap_or_default();
                let found = texts(&table[key]["text"]);
                if found.is_empty() {
                    wrong.push(format!("{code}: {key} has a brief and no text"));
                    continue;
                }
                let budget = brief["budget"].as_u64().unwrap_or(u64::MAX) as usize;
                let names: Vec<&str> = brief["variables"].as_array().into_iter().flatten().filter_map(|variable| variable["name"].as_str()).collect();
                for text in &found {
                    let longest = text.lines().map(width).max().unwrap_or(0);
                    if !text.contains('{') && longest > budget.max(1) {
                        wrong.push(format!("{code}: {key} is {longest} wide and its brief allows {budget}: {text}"));
                    }
                    if text.contains(['\u{2014}', '\u{2013}', '\u{b7}', '!', '\u{ff01}']) {
                        wrong.push(format!("{code}: {key} carries a mark the interface never uses: {text}"));
                    }
                    for hole in text.split('{').skip(1).filter_map(|rest| rest.split('}').next()) {
                        if hole != "count" && !names.contains(&hole) {
                            wrong.push(format!("{code}: {key} asks for {{{hole}}}, which nothing fills: {text}"));
                        }
                    }
                }
                for name in &names {
                    if !found.iter().any(|text| text.contains(&format!("{{{name}}}"))) {
                        wrong.push(format!("{code}: {key} never uses {{{name}}}"));
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{} texts need their writer:\n{}", wrong.len(), wrong.join("\n"));
    }

    #[test]
    fn old_settings_that_said_indonesian_still_mean_it() {
        let old = |json: &str| serde_json::from_str::<crate::state::Settings>(json).unwrap_or_else(|error| panic!("{error}"));
        for (json, code) in [(r#"{"indonesian":true}"#, "id"), (r#"{"indonesian":false}"#, "en"), (r#"{}"#, "en"), (r#"{"language":"xx"}"#, "xx"), (r#"{"language":"zz","indonesian":true}"#, "id")] {
            let settings = old(json);
            assert_eq!(from_settings(&settings.language, settings.legacy_indonesian), code, "{json}");
        }
        assert!(!serde_json::to_string(&crate::state::Settings::default()).unwrap_or_default().contains("indonesian"));
    }
}
