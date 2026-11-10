//! LRCLIB (<https://lrclib.net>): a free, keyless, community lyrics database.
//!
//! `GET /api/search?q=` returns up to 20 records with the words included; `GET /api/get/{id}`
//! returns one. Fields used: `id`, `trackName`, `artistName`, `instrumental`, `plainLyrics`
//! and `syncedLyrics`.

use serde::Deserialize;

use crate::text::{self, norm};
use crate::{Fetched, Found, LyricsError, http};

const SOURCE: &str = "LRCLIB";
const BASE: &str = "https://lrclib.net/api";
/// Suggestions shown at most; the service sends up to 20 and many are the same song.
const MAX_FOUND: usize = 12;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    id: u64,
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    instrumental: bool,
    #[serde(default)]
    plain_lyrics: Option<String>,
    #[serde(default)]
    synced_lyrics: Option<String>,
}

impl Record {
    fn has_words(&self) -> bool {
        !self.instrumental && (non_blank(&self.plain_lyrics) || non_blank(&self.synced_lyrics))
    }

    /// Plain words, falling back to the synced text with its timestamps stripped.
    fn words(&self) -> Option<String> {
        let raw = match (&self.plain_lyrics, &self.synced_lyrics) {
            (Some(plain), _) if !plain.trim().is_empty() => text::tidy(plain),
            (_, Some(synced)) if !synced.trim().is_empty() => text::tidy(&text::strip_timestamps(synced)),
            _ => return None,
        };
        (!raw.is_empty()).then_some(raw)
    }

    fn url(&self) -> String {
        format!("{BASE}/get/{}", self.id)
    }
}

fn non_blank(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|text| !text.trim().is_empty())
}

pub(crate) fn search(query: &str) -> Result<Vec<Found>, LyricsError> {
    let body = http::get(&format!("{BASE}/search"), &[("q", query)])?;
    found_from_json(&body, query)
}

pub(crate) fn fetch(found: &Found) -> Result<Fetched, LyricsError> {
    if found.id.is_empty() || !found.id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(LyricsError::BadAnswer("the song has no valid id".to_string()));
    }
    let body = http::get(&format!("{BASE}/get/{}", found.id), &[])?;
    fetched_from_json(&body)
}

/// Keeps records that have words, puts the closest title first and merges repeats.
pub(crate) fn found_from_json(body: &str, query: &str) -> Result<Vec<Found>, LyricsError> {
    let records: Vec<Record> =
        serde_json::from_str(body).map_err(|why| LyricsError::BadAnswer(why.to_string()))?;
    let wanted = norm(query);
    let mut ranked: Vec<(u8, usize, Record)> = records
        .into_iter()
        .filter(Record::has_words)
        .enumerate()
        .map(|(position, record)| (rank(&norm(&record.track_name), &wanted), position, record))
        .collect();
    // Closest title first; the service's own order breaks ties.
    ranked.sort_by_key(|(rank, position, _)| (*rank, *position));
    let mut seen = std::collections::HashSet::new();
    let found = ranked
        .into_iter()
        .filter(|(_, _, record)| seen.insert((norm(&record.track_name), norm(&record.artist_name))))
        .take(MAX_FOUND)
        .map(|(_, _, record)| Found {
            id: record.id.to_string(),
            title: record.track_name.trim().to_string(),
            artist: record.artist_name.trim().to_string(),
            source: SOURCE.to_string(),
            url: record.url(),
        })
        .collect();
    Ok(found)
}

pub(crate) fn fetched_from_json(body: &str) -> Result<Fetched, LyricsError> {
    let record: Record = serde_json::from_str(body).map_err(|why| LyricsError::BadAnswer(why.to_string()))?;
    let text = record.words().ok_or(LyricsError::NotFound)?;
    Ok(Fetched {
        title: record.track_name.trim().to_string(),
        artist: record.artist_name.trim().to_string(),
        text,
        source: SOURCE.to_string(),
        url: record.url(),
    })
}

/// 0 exact title, 1 starts with the query, 2 contains it, 3 anything else.
fn rank(title: &str, wanted: &str) -> u8 {
    if title == wanted {
        0
    } else if title.starts_with(wanted) {
        1
    } else if title.contains(wanted) {
        2
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recorded by hand in the service's shape, with made-up words.
    const SEARCH: &str = r#"[
      {"id": 11, "name": "Lantern Hill", "trackName": "Lantern Hill", "artistName": "Placeholder Choir",
       "albumName": "A", "duration": 200, "instrumental": false,
       "plainLyrics": "Alpha beta gamma\nDelta epsilon\n\nZeta eta theta", "syncedLyrics": null},
      {"id": 12, "trackName": "Lantern Hill (Live)", "artistName": "Placeholder Choir",
       "instrumental": false, "plainLyrics": "Alpha beta gamma", "syncedLyrics": null},
      {"id": 13, "trackName": "Lantern Hill", "artistName": "placeholder choir",
       "instrumental": false, "plainLyrics": "Alpha beta gamma", "syncedLyrics": "[00:01.00] Alpha beta gamma"},
      {"id": 14, "trackName": "Lantern Hill", "artistName": "Nobody",
       "instrumental": true, "plainLyrics": null, "syncedLyrics": null},
      {"id": 15, "trackName": "Hill of Lanterns", "artistName": "Other Band",
       "instrumental": false, "plainLyrics": null, "syncedLyrics": "[00:01.00] Iota kappa"},
      {"id": 16, "trackName": "Quiet Song", "artistName": "Nobody",
       "instrumental": false, "plainLyrics": "", "syncedLyrics": null},
      {"id": 17, "trackName": "The Lantern Hill Song", "artistName": "Third",
       "instrumental": false, "plainLyrics": "Mu nu xi", "syncedLyrics": null}
    ]"#;

    #[test]
    fn search_ranks_filters_and_merges() {
        let found = found_from_json(SEARCH, "lantern hill").unwrap();
        let ids: Vec<&str> = found.iter().map(|f| f.id.as_str()).collect();
        // 13 repeats 11 (same title and artist apart from case); 14 is instrumental; 16 is empty.
        assert_eq!(ids, ["11", "12", "17", "15"]);
        assert_eq!(found[0].source, "LRCLIB");
        assert_eq!(found[0].url, "https://lrclib.net/api/get/11");
    }

    #[test]
    fn empty_and_broken_answers_are_handled() {
        assert!(found_from_json("[]", "x").unwrap().is_empty());
        assert!(matches!(found_from_json("<html>", "x"), Err(LyricsError::BadAnswer(_))));
        assert!(matches!(found_from_json(r#"{"message":"bad"}"#, "x"), Err(LyricsError::BadAnswer(_))));
    }

    #[test]
    fn fetched_keeps_stanzas_and_prefers_plain() {
        let record = r#"{"id": 11, "trackName": " Lantern Hill ", "artistName": "Placeholder Choir",
          "instrumental": false, "plainLyrics": "Alpha beta\r\nGamma\r\n\r\n\r\nDelta\n",
          "syncedLyrics": "[00:01.00] Different"}"#;
        let fetched = fetched_from_json(record).unwrap();
        assert_eq!(fetched.title, "Lantern Hill");
        assert_eq!(fetched.text, "Alpha beta\nGamma\n\nDelta");
    }

    #[test]
    fn fetched_falls_back_to_synced_text() {
        let record = r#"{"id": 15, "trackName": "T", "artistName": "A", "instrumental": false,
          "plainLyrics": null,
          "syncedLyrics": "[00:01.00] Iota kappa\n[00:03.50] Lambda\n[00:05.00] \n[00:06.00] Mu nu"}"#;
        assert_eq!(fetched_from_json(record).unwrap().text, "Iota kappa\nLambda\n\nMu nu");
    }

    #[test]
    fn instrumental_or_empty_record_is_not_found() {
        let record = r#"{"id": 1, "trackName": "T", "artistName": "A", "instrumental": true,
          "plainLyrics": null, "syncedLyrics": null}"#;
        assert_eq!(fetched_from_json(record), Err(LyricsError::NotFound));
    }

    #[test]
    fn fetch_refuses_a_bad_id_without_the_network() {
        let found = Found {
            id: "../x".into(),
            title: String::new(),
            artist: String::new(),
            source: String::new(),
            url: String::new(),
        };
        assert!(matches!(fetch(&found), Err(LyricsError::BadAnswer(_))));
    }

    /// Manual check against the real service: `cargo test -p artosia-lyrics -- --ignored`.
    #[test]
    #[ignore = "uses the network"]
    fn live_search_and_fetch() {
        let found = crate::search("hosanna").unwrap();
        assert!(!found.is_empty(), "no match for a common title");
        let fetched = crate::fetch(&found[0]).unwrap();
        assert!(!fetched.text.is_empty());
    }
}
