//! The search behind the search box for everything that is not a song's lyrics or a Bible
//! verse: plan items, announcements, commands, looks, translations.
//!
//! The shape follows the owner's house-kit `core/palette` (itself after Algolia's documented
//! ranking): each part of the app contributes documents, and hits are ordered by a CASCADE of
//! single criteria, where a criterion only speaks when every one above it tied. A blended
//! score cannot be explained; a cascade says exactly which rule put a row first.
//!
//! The order: preferred, fewer typos, whole words before prefixes, prefixes before a match
//! inside a word, the heavier field, words closer together, the match nearer the start.

use std::cmp::Ordering;

use crate::text::{WordMatch, match_word, words};

/// How much a field counts: the name first, then where it sits, then a detail line.
pub const LABEL: f32 = 3.0;
pub const SECTION: f32 = 2.0;
pub const DETAIL: f32 = 1.0;
/// Other words that find a document without outranking its name.
const ALIAS: f32 = 0.5;

/// One thing the search can find.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    /// Unique across every source, such as "command:black" or "plan:3".
    pub key: String,
    pub group: &'static str,
    /// What the row shows.
    pub display: String,
    pub detail: String,
    /// Weighted texts. A document is indexed under every language it has, so an English
    /// word finds an Indonesian label and the row still shows the reader's language.
    pub fields: Vec<(f32, String)>,
    pub aliases: Vec<String>,
}

impl Doc {
    pub fn new(key: impl Into<String>, group: &'static str, display: impl Into<String>) -> Doc {
        let display = display.into();
        Doc { key: key.into(), group, fields: vec![(LABEL, display.clone())], display, ..Doc::default() }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Doc {
        let detail = detail.into();
        self.fields.push((DETAIL, detail.clone()));
        self.detail = detail;
        self
    }

    /// Words in any language that should also find this document.
    pub fn aliases(mut self, aliases: &[&str]) -> Doc {
        self.aliases.extend(aliases.iter().map(|alias| (*alias).to_string()));
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocHit {
    /// Index into the documents searched.
    pub doc: usize,
    pub preferred: bool,
    /// Edits across the query's words; 0 when none was misspelt.
    pub typos: u8,
    /// How many of the query's words matched a whole word.
    pub exact: u8,
    /// How many matched only inside a longer word.
    pub infix: u8,
    /// The heaviest field any word matched in.
    pub weight: f32,
    /// Words between the matched words; 0 when they sit side by side.
    pub proximity: u32,
    /// How far into its field the first matched word sits.
    pub position: u32,
}

/// THE CASCADE. Read it top to bottom: each line only speaks when every line above it tied.
pub fn compare(a: &DocHit, b: &DocHit) -> Ordering {
    b.preferred
        .cmp(&a.preferred)
        .then(a.typos.cmp(&b.typos))
        .then(b.exact.cmp(&a.exact))
        .then(a.infix.cmp(&b.infix))
        .then(b.weight.total_cmp(&a.weight))
        .then(a.proximity.cmp(&b.proximity))
        .then(a.position.cmp(&b.position))
        .then(a.doc.cmp(&b.doc))
}

/// Documents that hold every word of the query, best first. `preferred` marks documents that
/// go first when everything else ties, such as what is already in today's plan.
pub fn search(docs: &[Doc], query: &str, preferred: &dyn Fn(&Doc) -> bool, limit: usize) -> Vec<DocHit> {
    let tokens = crate::search::tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }
    let last = tokens.len() - 1;
    let mut hits: Vec<DocHit> = Vec::new();
    for (index, doc) in docs.iter().enumerate() {
        let texts = doc.fields.iter().map(|(weight, text)| (*weight, text.as_str())).chain(doc.aliases.iter().map(|alias| (ALIAS, alias.as_str())));
        let fields: Vec<(f32, Vec<String>)> = texts.map(|(weight, text)| (weight, words(text).into_iter().map(|word| word.norm).collect())).collect();
        // For each query word, its best match anywhere in the document.
        let mut found: Vec<(WordMatch, f32, u32)> = Vec::with_capacity(tokens.len());
        for (i, token) in tokens.iter().enumerate() {
            let mut best: Option<(WordMatch, f32, u32)> = None;
            for (weight, field) in &fields {
                for (at, word) in field.iter().enumerate() {
                    let Some(matched) = match_word(token, word, i == last) else { continue };
                    let better = match best {
                        None => true,
                        Some((held, held_weight, _)) => matched.rank() < held.rank() || (matched.rank() == held.rank() && *weight > held_weight),
                    };
                    if better {
                        best = Some((matched, *weight, at as u32));
                    }
                }
            }
            match best {
                Some(best) => found.push(best),
                None => break,
            }
        }
        if found.len() != tokens.len() {
            continue;
        }
        let at: Vec<u32> = found.iter().map(|(_, _, at)| *at).collect();
        let (low, high) = (at.iter().copied().min().unwrap_or(0), at.iter().copied().max().unwrap_or(0));
        hits.push(DocHit {
            doc: index,
            preferred: preferred(doc),
            typos: found.iter().map(|(matched, _, _)| matched.typos).sum(),
            exact: found.iter().filter(|(matched, _, _)| matched.exact).count() as u8,
            infix: found.iter().filter(|(matched, _, _)| matched.infix).count() as u8,
            weight: found.iter().map(|(_, weight, _)| *weight).fold(0.0, f32::max),
            proximity: if found.len() > 1 { (high - low).saturating_sub(found.len() as u32 - 1) } else { 0 },
            position: low,
        });
    }
    hits.sort_by(compare);
    hits.truncate(limit);
    hits
}

/// A number written at the end of a query, with the words before it: "countdown 5" gives
/// ("countdown", 5). For commands that take one number.
pub fn trailing_number(query: &str) -> Option<(String, u32)> {
    let trimmed = query.trim();
    let split = trimmed.rfind(|c: char| !c.is_ascii_digit()).map(|at| at + 1).unwrap_or(0);
    let (head, digits) = trimmed.split_at(split);
    let number = digits.parse().ok()?;
    let head = head.trim();
    (!head.is_empty()).then(|| (head.to_string(), number))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs() -> Vec<Doc> {
        vec![
            Doc::new("command:black", "action", "Black").detail("Turn the projector black").aliases(&["hitam", "gelap", "blank", "screen off"]),
            Doc::new("command:clear", "action", "Clear text").aliases(&["hide words", "sembunyikan teks"]),
            Doc::new("announcement:1", "announcement", "Youth night").detail("Friday at 7 pm").aliases(&["show announcement", "tampilkan pengumuman"]),
            Doc::new("announcement:2", "announcement", "Blackboard sale").aliases(&["show announcement", "tampilkan pengumuman"]),
            Doc::new("look:Forest", "look", "Forest").aliases(&["look", "tampilan"]),
        ]
    }

    fn keys(query: &str) -> Vec<String> {
        let docs = docs();
        search(&docs, query, &|_| false, 10).into_iter().map(|hit| docs[hit.doc].key.clone()).collect()
    }

    #[test]
    fn a_whole_word_beats_a_prefix_and_a_typo() {
        // "Black" is the whole word; "Blackboard" only starts with it.
        assert_eq!(keys("black"), ["command:black", "announcement:2"]);
        // A swapped pair still finds it, and the unfinished longer word after it.
        assert_eq!(keys("blakc"), ["command:black", "announcement:2"]);
    }

    #[test]
    fn another_language_finds_the_same_command() {
        assert_eq!(keys("hitam"), ["command:black"]);
        assert_eq!(keys("gelp"), ["command:black"]);
    }

    #[test]
    fn an_action_takes_what_it_acts_on() {
        assert_eq!(keys("show announcement youth"), ["announcement:1"]);
        assert_eq!(keys("tampilkan pengumuman black"), ["announcement:2"]);
        assert_eq!(keys("look forest"), ["look:Forest"]);
    }

    #[test]
    fn every_word_must_be_found() {
        assert!(keys("youth sale").is_empty());
        assert!(keys("   ").is_empty());
    }

    #[test]
    fn preferred_documents_go_first_on_a_tie_and_never_past_a_better_match() {
        let docs = docs();
        let hits = search(&docs, "show announcement", &|doc| doc.key == "announcement:2", 10);
        assert_eq!(docs[hits[0].doc].key, "announcement:2");
        let hits = search(&docs, "black", &|doc| doc.key == "announcement:2", 10);
        // Preference is the first line of the cascade, as in the kit: it does win here.
        assert_eq!(docs[hits[0].doc].key, "announcement:2");
    }

    #[test]
    fn a_trailing_number_is_split_off() {
        assert_eq!(trailing_number("countdown 5"), Some(("countdown".to_string(), 5)));
        assert_eq!(trailing_number("hitung mundur 10"), Some(("hitung mundur".to_string(), 10)));
        assert_eq!(trailing_number("42"), None);
        assert_eq!(trailing_number("psalm"), None);
    }
}
