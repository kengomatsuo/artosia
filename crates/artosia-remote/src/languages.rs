//! The phone page's languages. English and Indonesian are written in `page.html`. Any other
//! language is a table, `docs/i18n/phone-<code>.json`, with the keys of `STR.en` in `page.html`
//! and optionally `labels`, a list of `[pattern, replacement]` that turns the section names the
//! operator sends in English into the language's own. A key a table lacks is English, and a
//! language the page does not know is English too.

/// `(code, the language in its own words, the table's JSON)`. A language joins here once a person
/// who can judge it has reviewed it; see `docs/i18n/README.md`.
pub const EXTRA_LANGUAGES: &[(&str, &str, &str)] = &[];

const MARKER: &str = "/*EXTRA_LANGUAGES*/";

/// The page with the registry's further languages written into it.
pub fn page_with(template: &str, extra: &[(&str, &str, &str)]) -> String {
    let mut script = String::new();
    for (code, name, table) in extra {
        // A table is data in a script block, so `</` is written so it cannot end the block.
        let table = table.replace("</", "<\\/");
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
        assert!(page.contains(r#"addLanguage("xx", "Pseudo", {"live": "<\/script>Live"});"#));
    }

    #[test]
    fn the_page_picks_a_known_language_and_falls_back_to_english() {
        let template = crate::server::PAGE;
        assert!(template.contains("function addLanguage("));
        assert!(template.contains("LANGS.some("), "a saved or browser language is checked against the list");
        assert!(template.contains("STR.en[key]"), "a key a table lacks is English");
    }
}
