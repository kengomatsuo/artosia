//! The one call behind the "Songs from the internet" dialog: one field, one list, one pick.

use crate::text::{clean_query, norm};
use crate::{Fetched, Found, GemaIndex, LyricsError, gema, lrclib, page};

/// `Found.source` of a result that is the web address the person pasted.
pub const PASTED: &str = "Pasted page";
/// Results shown at most.
const MOST: usize = 30;

/// What the dialog lists. LRCLIB failing leaves Gema's results in `found`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Browse {
    pub found: Vec<Found>,
    /// Why LRCLIB gave nothing, to show under the list; `None` when it answered.
    pub lrclib_problem: Option<LyricsError>,
}

/// Matches for what was typed: Gema's titles first (a hymn number such as "kj 4" works), then
/// LRCLIB's, a song in both kept once, at most 30. A pasted http or https address is the one
/// result, marked `PASTED`. Blocking up to five seconds when it asks LRCLIB.
pub fn browse(query: &str, gema: Option<&GemaIndex>) -> Browse {
    browse_with(query, gema, crate::search)
}

/// `browse` with the LRCLIB call passed in, so tests need no network.
fn browse_with(
    query: &str,
    gema: Option<&GemaIndex>,
    lrclib: impl FnOnce(&str) -> Result<Vec<Found>, LyricsError>,
) -> Browse {
    let query = clean_query(query, 2000);
    if query.is_empty() {
        return Browse::default();
    }
    if is_address(&query) {
        let pasted = Found {
            id: query.clone(),
            title: query.clone(),
            artist: String::new(),
            source: PASTED.to_string(),
            url: query,
        };
        return Browse { found: vec![pasted], lrclib_problem: None };
    }
    let mut found = gema.map(|index| index.search(&query, MOST)).unwrap_or_default();
    let mut lrclib_problem = None;
    match lrclib(&query) {
        Ok(more) => found.extend(more),
        Err(why) => lrclib_problem = Some(why),
    }
    let mut seen = Vec::new();
    found.retain(|song| {
        let key = norm(gema::strip_number(&song.title));
        let new = !seen.contains(&key);
        if new {
            seen.push(key);
        }
        new
    });
    found.truncate(MOST);
    Browse { found, lrclib_problem }
}

fn is_address(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && !query.contains(' ')
        && query.split_once("://").is_some_and(|(_, rest)| !rest.is_empty())
}

/// The words of one pick, parsed: LRCLIB's record through `fetch`, a Gema or pasted page
/// through the page reader. Blocking up to five seconds.
pub fn words(found: &Found) -> Result<Fetched, LyricsError> {
    if found.source == lrclib::SOURCE {
        crate::fetch(found)
    } else {
        page::from_page(&found.url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GemaSong;

    fn index() -> GemaIndex {
        let song = |title: &str, path: &str| GemaSong { title: title.to_string(), path: path.to_string() };
        GemaIndex {
            songs: vec![
                song("KJ 012. Lagu Satu", "/kj_012_lagu_satu"),
                song("KJ 004. Lagu Empat", "/kj_004_lagu_empat"),
                song("Lagu Bebas", "/lagu_bebas"),
            ],
            read_on: "2026-11-14".to_string(),
        }
    }

    fn lrclib_song(title: &str) -> Found {
        Found {
            id: "7".to_string(),
            title: title.to_string(),
            artist: "Penyanyi".to_string(),
            source: lrclib::SOURCE.to_string(),
            url: "https://lrclib.net/api/get/7".to_string(),
        }
    }

    fn titles(result: &Browse) -> Vec<&str> {
        result.found.iter().map(|song| song.title.as_str()).collect()
    }

    #[test]
    fn hymn_number_finds_its_hymn() {
        let index = index();
        let result = browse_with("kj 4", Some(&index), |_| Ok(Vec::new()));
        assert_eq!(titles(&result), ["KJ 004. Lagu Empat"]);
    }

    #[test]
    fn gema_comes_before_lrclib() {
        let index = index();
        let result = browse_with("lagu", Some(&index), |_| Ok(vec![lrclib_song("Lagu Lain")]));
        assert_eq!(titles(&result), ["KJ 012. Lagu Satu", "KJ 004. Lagu Empat", "Lagu Bebas", "Lagu Lain"]);
        assert_eq!(result.lrclib_problem, None);
    }

    #[test]
    fn a_song_in_both_is_kept_once_from_gema() {
        let index = index();
        let result = browse_with("lagu bebas", Some(&index), |_| Ok(vec![lrclib_song("lagu  bebas!")]));
        assert_eq!(titles(&result), ["Lagu Bebas"]);
        assert_eq!(result.found[0].source, gema::SOURCE);
        let numbered = browse_with("lagu empat", Some(&index), |_| Ok(vec![lrclib_song("Lagu Empat")]));
        assert_eq!(titles(&numbered), ["KJ 004. Lagu Empat"]);
    }

    #[test]
    fn a_pasted_address_is_the_only_result_and_skips_lrclib() {
        let index = index();
        let result = browse_with("  https://example.test/lagu/1 ", Some(&index), |_| panic!("no call"));
        assert_eq!(result.found.len(), 1);
        assert_eq!(result.found[0].source, PASTED);
        assert_eq!(result.found[0].url, "https://example.test/lagu/1");
        assert_eq!(browse_with("HTTP://example.test/a", None, |_| panic!("no call")).found[0].source, PASTED);
        // Words that merely mention http are a search, not an address.
        assert!(browse_with("http is a protocol", None, |_| Ok(Vec::new())).found.is_empty());
        assert!(browse_with("https://", None, |_| Ok(Vec::new())).found.is_empty());
    }

    #[test]
    fn lrclib_failing_keeps_gemas_results() {
        let index = index();
        let result = browse_with("lagu bebas", Some(&index), |_| Err(LyricsError::Network("down".to_string())));
        assert_eq!(titles(&result), ["Lagu Bebas"]);
        assert_eq!(result.lrclib_problem, Some(LyricsError::Network("down".to_string())));
        let alone = browse_with("lagu bebas", None, |_| Err(LyricsError::NotFound));
        assert!(alone.found.is_empty());
        assert_eq!(alone.lrclib_problem, Some(LyricsError::NotFound));
    }

    #[test]
    fn no_index_uses_lrclib_alone_and_an_empty_query_asks_nobody() {
        let result = browse_with("lagu", None, |_| Ok(vec![lrclib_song("Lagu Lain")]));
        assert_eq!(titles(&result), ["Lagu Lain"]);
        assert_eq!(browse_with("   ", None, |_| panic!("no call")), Browse::default());
    }

    #[test]
    fn at_most_thirty() {
        let many = (0..40).map(|n| lrclib_song(&format!("Lagu nomor {n}"))).collect();
        assert_eq!(browse_with("lagu", None, |_| Ok(many)).found.len(), MOST);
    }

    #[test]
    fn a_bad_lrclib_pick_is_refused_before_any_call() {
        let mut pick = lrclib_song("Lagu Lain");
        pick.id = "../x".to_string();
        assert!(matches!(words(&pick), Err(LyricsError::BadAnswer(_))));
    }
}
