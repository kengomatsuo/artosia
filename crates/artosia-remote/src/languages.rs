//! The phone page's languages. English and Indonesian are written in `page.html`. Any other
//! language is a table, `docs/i18n/phone-<code>.json`, in the shape of the operator's tables:
//! the keys of `STR.en` in `page.html`, each `{"text": ...}`, with `label.verse` and its
//! neighbours naming the parts of a song and `label.numbered` joining a name to its number. A
//! key a table lacks is English, and a language the page does not know is English too.

/// `(code, the language in its own words, the table's JSON)`, in the order of the operator's
/// list; see `docs/i18n/README.md`.
pub const EXTRA_LANGUAGES: &[(&str, &str, &str)] = &[
    ("zh-Hans", "\u{7b80}\u{4f53}\u{4e2d}\u{6587}", include_str!("../../../docs/i18n/phone-zh-Hans.json")),
    ("zh-Hant", "\u{7e41}\u{9ad4}\u{4e2d}\u{6587}", include_str!("../../../docs/i18n/phone-zh-Hant.json")),
    ("ko", "\u{d55c}\u{ad6d}\u{c5b4}", include_str!("../../../docs/i18n/phone-ko.json")),
    ("ja", "\u{65e5}\u{672c}\u{8a9e}", include_str!("../../../docs/i18n/phone-ja.json")),
    ("es", "Espa\u{f1}ol", include_str!("../../../docs/i18n/phone-es.json")),
    ("pt-BR", "Portugu\u{ea}s (Brasil)", include_str!("../../../docs/i18n/phone-pt-BR.json")),
    ("fr", "Fran\u{e7}ais", include_str!("../../../docs/i18n/phone-fr.json")),
    ("vi", "Ti\u{1ebf}ng Vi\u{1ec7}t", include_str!("../../../docs/i18n/phone-vi.json")),
];

const MARKER: &str = "/*EXTRA_LANGUAGES*/";

/// The page with the registry's further languages written into it.
pub fn page_with(template: &str, extra: &[(&str, &str, &str)]) -> String {
    let mut script = String::new();
    for (code, name, table) in extra {
        // A table is data in a script block, so `</` is written so it cannot end the block.
        let table = flat(table).replace("</", "<\\/");
        script.push_str(&format!("addLanguage({}, {}, {table});\n", json_string(code), json_string(name)));
    }
    template.replacen(MARKER, &script, 1)
}

/// The page as served.
pub fn page() -> String {
    page_with(crate::server::PAGE, EXTRA_LANGUAGES)
}

fn json_string(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

/// A table as the page reads it: each key straight to its text. The review's back-translation
/// and reason stay in the file and never reach a phone.
fn flat(table: &str) -> String {
    let Ok(serde_json::Value::Object(entries)) = serde_json::from_str::<serde_json::Value>(table) else { return "{}".to_string() };
    let texts: serde_json::Map<String, serde_json::Value> = entries.into_iter().map(|(key, entry)| (key, entry.get("text").cloned().unwrap_or(entry))).collect();
    serde_json::Value::Object(texts).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_carries_english_and_indonesian_and_names_where_more_go() {
        let page = page();
        assert!(page.contains("const STR = {"));
        assert!(page.contains("[\"id\", \"Bahasa Indonesia\"]"));
        assert!(!page.contains(MARKER), "the marker is replaced");
    }

    #[test]
    fn a_further_language_is_written_into_the_page_and_cannot_end_the_script() {
        let page = page_with(crate::server::PAGE, &[("xx", "Pseudo", r#"{"live": "</script>Live"}"#)]);
        assert!(page.contains(r#"addLanguage("xx", "Pseudo", {"live":"<\/script>Live"});"#));
    }

    #[test]
    fn every_further_language_answers_every_phone_brief_and_reaches_the_page_as_plain_texts() {
        let briefs: serde_json::Value = serde_json::from_str(include_str!("../../../docs/i18n/brief-phone.json")).unwrap_or_default();
        let page = page();
        assert_eq!(EXTRA_LANGUAGES.len(), 8);
        for (code, name, table) in EXTRA_LANGUAGES {
            let texts: serde_json::Value = serde_json::from_str(&flat(table)).unwrap_or_default();
            for brief in briefs.as_array().into_iter().flatten() {
                let key = brief["key"].as_str().unwrap_or_default();
                let text = texts[key].as_str().unwrap_or_default();
                assert!(!text.is_empty(), "{code}: {key} has a brief and no text");
                assert!(!text.contains(['\u{2014}', '\u{2013}', '\u{b7}', '!', '\u{ff01}']), "{code}: {key} carries a mark the page never uses: {text}");
            }
            let numbered = texts["label.numbered"].as_str().unwrap_or_default();
            assert!(numbered.contains("{0}") && numbered.contains("{1}"), "{code}: label.numbered is {numbered:?}");
            assert!(page.contains(&format!("addLanguage({}, {}, {{", json_string(code), json_string(name))), "{code} is not in the page");
        }
        assert!(!page.contains("\"why\""), "the review's notes stay out of the page");
    }

    #[test]
    fn the_page_picks_a_known_language_and_falls_back_to_english() {
        let template = crate::server::PAGE;
        assert!(template.contains("function addLanguage("));
        assert!(template.contains("LANGS.some("), "a saved or browser language is checked against the list");
        assert!(template.contains("STR.en[key]"), "a key a table lacks is English");
    }
}
