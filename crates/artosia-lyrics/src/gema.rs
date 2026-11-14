//! Gema SABDA's song titles, read from the list pages its robots.txt allows (`/daftar_lirik`
//! and `/daftar_lirik_kidung_jemaat`, never `/search/`), so a title can be matched here and only
//! the page the person picks is opened. Titles and addresses only: no words are kept.

use serde::{Deserialize, Serialize};

use crate::text::norm;
use crate::{Found, LyricsError, http};

const BASE: &str = "https://gema.sabda.org";
pub(crate) const SOURCE: &str = "Gema SABDA";
/// A list page holds about fifty titles; more pages than this means the site changed shape.
const MOST_PAGES: u32 = 120;
/// One list page a second: the site states no limit, so the walk is slow on purpose.
const GAP: std::time::Duration = std::time::Duration::from_secs(1);

/// Every title the lists name, with the page each opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GemaIndex {
    pub songs: Vec<GemaSong>,
    /// When the lists were read, as `YYYY-MM-DD`, so a copy kept on disk is read again weekly.
    pub read_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GemaSong {
    pub title: String,
    /// The page's path on the site, such as `/kj_004_hai_mari_sembah`.
    pub path: String,
}

impl GemaIndex {
    /// Reads the hymnal list and every page of the general list. Blocking, about a minute.
    pub fn read(today: &str) -> Result<GemaIndex, LyricsError> {
        let mut songs = titles_of(&http::get(&format!("{BASE}/daftar_lirik_kidung_jemaat"), &[])?);
        for page in 0..MOST_PAGES {
            std::thread::sleep(GAP);
            let found = titles_of(&http::get(&format!("{BASE}/daftar_lirik"), &[("page", &page.to_string())])?);
            let before = songs.len();
            for song in found {
                if !songs.iter().any(|known| known.path == song.path) {
                    songs.push(song);
                }
            }
            // A page past the end repeats the last one or is empty: nothing new means done.
            if songs.len() == before {
                break;
            }
        }
        Ok(GemaIndex { songs, read_on: today.to_string() })
    }

    /// Titles matching `query`, best first: the same title, then titles starting with it, then
    /// titles holding every typed word. A hymn number works too ("kj 4", "KJ 004").
    pub fn search(&self, query: &str, most: usize) -> Vec<Found> {
        let wanted = norm(&hymn_number(query));
        if wanted.is_empty() {
            return Vec::new();
        }
        let words: Vec<&str> = wanted.split(' ').collect();
        let mut ranked: Vec<(u8, usize, &GemaSong)> = self
            .songs
            .iter()
            .enumerate()
            .filter_map(|(at, song)| {
                let title = norm(&song.title);
                let bare = norm(strip_number(&song.title));
                let rank = if title == wanted || bare == wanted {
                    0
                } else if title.starts_with(&wanted) || bare.starts_with(&wanted) {
                    1
                } else if words.iter().all(|word| title.split(' ').any(|have| have.starts_with(word))) {
                    2
                } else {
                    return None;
                };
                Some((rank, at, song))
            })
            .collect();
        ranked.sort_by_key(|(rank, at, _)| (*rank, *at));
        ranked
            .into_iter()
            .take(most)
            .map(|(_, _, song)| Found {
                id: song.path.clone(),
                title: song.title.clone(),
                artist: String::new(),
                source: SOURCE.to_string(),
                url: format!("{BASE}{}", song.path),
            })
            .collect()
    }
}

/// "kj 4" and "KJ4" become "KJ 004", the way the site writes its hymn numbers.
fn hymn_number(query: &str) -> String {
    let trimmed = query.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("kj") {
        let digits = rest.trim();
        if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(number) = digits.parse::<u32>() {
                return format!("KJ {number:03}");
            }
        }
    }
    trimmed.to_string()
}

/// "KJ 004. Hai Mari Sembah" without its number, so the title alone matches too.
fn strip_number(title: &str) -> &str {
    match title.split_once(". ") {
        Some((head, tail)) if head.starts_with("KJ") && head.len() <= 8 => tail,
        _ => title,
    }
}

/// The song links of a list page: `<a href="/path" hreflang="und">Title</a>`.
pub(crate) fn titles_of(html: &str) -> Vec<GemaSong> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find("hreflang=\"und\"") {
        let before = &rest[..at];
        let path = before.rfind("href=\"").map(|start| &before[start + 6..]).and_then(|tail| tail.split('"').next());
        rest = &rest[at..];
        let title = rest.find('>').and_then(|gt| rest[gt + 1..].split('<').next());
        if let (Some(path), Some(title)) = (path, title) {
            let title = crate::page::decode(title.trim());
            if path.starts_with('/') && !path.contains("://") && !title.is_empty() {
                out.push(GemaSong { title, path: path.to_string() });
            }
        }
        rest = &rest["hreflang=\"und\"".len()..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index() -> GemaIndex {
        let html = r#"<a href="/daftar_lirik_kidung_jemaat" class="x">Daftar</a>
            <a href="/kj_002_suci_suci_suci" hreflang="und">KJ 002. Suci, Suci, Suci</a>
            <a href="/kj_004_hai_mari_sembah" hreflang="und">KJ 004. Hai Mari Sembah </a>
            <a href="/ku_tak_dapat_jalan_sendiri_0" hreflang="und">&#039;Ku Tak Dapat Jalan Sendiri</a>
            <a href="/hai_mari_bernyanyi" hreflang="und">Hai Mari Bernyanyi</a>"#;
        GemaIndex { songs: titles_of(html), read_on: "2026-11-14".to_string() }
    }

    #[test]
    fn list_links_are_read_with_their_paths() {
        let songs = index().songs;
        assert_eq!(songs.len(), 4);
        assert_eq!(songs[1], GemaSong { title: "KJ 004. Hai Mari Sembah".to_string(), path: "/kj_004_hai_mari_sembah".to_string() });
        assert_eq!(songs[2].title, "'Ku Tak Dapat Jalan Sendiri");
    }

    #[test]
    fn a_title_or_a_hymn_number_finds_the_song() {
        let index = index();
        let titles = |query: &str| index.search(query, 5).into_iter().map(|f| f.title).collect::<Vec<_>>();
        assert_eq!(titles("hai mari sembah"), ["KJ 004. Hai Mari Sembah"]);
        assert_eq!(titles("kj 4"), ["KJ 004. Hai Mari Sembah"]);
        assert_eq!(titles("hai mari"), ["KJ 004. Hai Mari Sembah", "Hai Mari Bernyanyi"]);
        assert_eq!(titles("ku tak dapat"), ["'Ku Tak Dapat Jalan Sendiri"]);
        assert!(titles("tidak ada").is_empty());
        assert_eq!(index.search("suci", 5)[0].url, "https://gema.sabda.org/kj_002_suci_suci_suci");
    }
}
