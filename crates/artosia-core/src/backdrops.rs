//! The catalogue of ready backdrops: pictures and looping videos a volunteer can put behind the
//! words. The list is a JSON file built by `tools/backdrops.py` from Wikimedia Commons files
//! whose own record says CC0 or public domain (docs/backdrops.md). It holds addresses, never
//! media: a file is fetched from its source only when a volunteer presses the button for it.
//! This module reads and filters the list; the network is the window's business.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::text::normalize;

/// The catalogue shipped inside the program.
pub const BUILT_IN: &str = include_str!("../assets/backdrops.json");

/// The categories in the order the browser shows them. Each entry names one of these.
pub const CATEGORIES: [&str; 10] = ["sky", "mountains", "water", "nature", "light", "night", "candles", "church", "people", "texture"];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Picture,
    Video,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub kind: Kind,
    pub category: String,
    /// The file on the source's own server.
    pub url: String,
    /// A small preview on the same server.
    pub preview: String,
    pub width: u32,
    pub height: u32,
    /// Seconds, for a video.
    #[serde(default)]
    pub duration: Option<f32>,
    pub author: String,
    pub licence: String,
    /// The licence's own text.
    pub licence_url: String,
    /// The page on the source that states the licence for this file.
    pub page: String,
    pub source: String,
    /// Mean brightness of the preview, 0 to 100.
    #[serde(default)]
    pub brightness: Option<u8>,
    /// How much detail the preview holds, 0 to 100.
    #[serde(default)]
    pub busy: Option<u8>,
    /// What a video weighs, in megabytes, as far as the source says.
    #[serde(default)]
    pub megabytes: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Catalogue {
    #[serde(default)]
    pub built: String,
    #[serde(default)]
    pub source: String,
    pub entries: Vec<Entry>,
}

pub fn parse(text: &str) -> Result<Catalogue, String> {
    serde_json::from_str(text).map_err(|error| error.to_string())
}

/// The shipped catalogue; empty if the file were ever unreadable (a test pins that it is not).
pub fn built_in() -> Catalogue {
    parse(BUILT_IN).unwrap_or_default()
}

impl Entry {
    /// `jpg`, `png`, `webm` and so on, from the address; by kind when it names none.
    pub fn extension(&self) -> String {
        let path = self.url.split(['?', '#']).next().unwrap_or("");
        let last = path.rsplit('/').next().unwrap_or("");
        match last.rsplit_once('.') {
            Some((_, ext)) if (2..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric()) => ext.to_lowercase(),
            _ => match self.kind {
                Kind::Picture => "jpg".to_string(),
                Kind::Video => "webm".to_string(),
            },
        }
    }

    /// The name the downloaded file gets in the library: stable, so a second press finds it.
    pub fn file_name(&self) -> String {
        let id: String = self.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
        format!("{id}.{}", self.extension())
    }

    /// The backdrop is bright or busy enough that the look's dimming helps the words: about the
    /// brightest tenth of the catalogue, and the busiest tenth (docs/backdrops.md).
    pub fn wants_dimming(&self) -> bool {
        self.brightness.is_some_and(|value| value >= 58) || self.busy.is_some_and(|value| value >= 85)
    }

    fn haystack(&self) -> String {
        normalize(&format!("{} {} {}", self.title, self.category, self.author))
    }
}

/// Where downloaded backdrops live: a folder of their own inside the library's media folder.
pub fn folder(media: &Path) -> PathBuf {
    media.join("backdrops")
}

/// Where a downloaded backdrop's small preview is kept, so the browser draws without the network.
pub fn preview_path(media: &Path, entry: &Entry) -> PathBuf {
    folder(media).join("previews").join(format!("{}.jpg", entry.file_name().rsplit_once('.').map_or("", |(stem, _)| stem)))
}

pub fn local_path(media: &Path, entry: &Entry) -> PathBuf {
    folder(media).join(entry.file_name())
}

/// A file counts as downloaded only when it exists and is not empty.
pub fn is_downloaded(media: &Path, entry: &Entry) -> bool {
    std::fs::metadata(local_path(media, entry)).is_ok_and(|meta| meta.is_file() && meta.len() > 0)
}

/// The indices of the entries to show: those of the chosen category (all when `None`) whose
/// title, category or author holds every word typed, pictures before videos inside the catalogue's own order.
pub fn matching(entries: &[Entry], category: Option<&str>, query: &str) -> Vec<usize> {
    let words: Vec<String> = normalize(query).split_whitespace().map(str::to_string).collect();
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| category.is_none_or(|wanted| entry.category == wanted))
        .filter(|(_, entry)| {
            let hay = entry.haystack();
            words.iter().all(|word| hay.contains(word.as_str()))
        })
        .map(|(at, _)| at)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, title: &str, category: &str, url: &str) -> Entry {
        Entry {
            id: id.into(),
            title: title.into(),
            kind: Kind::Picture,
            category: category.into(),
            url: url.into(),
            preview: "https://example.org/p.jpg".into(),
            width: 1920,
            height: 1080,
            duration: None,
            author: "A. Author".into(),
            licence: "CC0".into(),
            licence_url: "https://example.org/licence".into(),
            page: "https://example.org/page".into(),
            source: "wikimedia".into(),
            brightness: Some(30),
            busy: Some(10),
            megabytes: None,
        }
    }

    #[test]
    fn the_shipped_catalogue_parses_and_every_entry_names_its_author_and_licence() {
        let catalogue = parse(BUILT_IN).expect("the shipped catalogue parses");
        assert!(catalogue.entries.len() >= 250, "the owner asked for at least 250, found {}", catalogue.entries.len());
        let mut seen = std::collections::HashSet::new();
        for entry in &catalogue.entries {
            let who = &entry.id;
            assert!(seen.insert(entry.id.clone()), "{who} is listed twice");
            assert!(!entry.title.trim().is_empty(), "{who} has no title");
            assert!(!entry.author.trim().is_empty(), "{who} has no author");
            assert!(!entry.licence.trim().is_empty(), "{who} has no licence");
            assert!(entry.licence_url.starts_with("https://"), "{who} has no licence address");
            assert!(entry.page.starts_with("https://"), "{who} has no page that states the licence");
            assert!(entry.url.starts_with("https://") && entry.preview.starts_with("https://"), "{who} has no download address");
            assert!(CATEGORIES.contains(&entry.category.as_str()), "{who} is in an unknown category {}", entry.category);
            let licence = entry.licence.to_lowercase();
            assert!(licence.starts_with("cc0") || licence.starts_with("public domain") || licence.starts_with("pd"), "{who} is under {}", entry.licence);
            assert!(entry.width >= 848 && entry.height > 0, "{who} is too small");
            if entry.kind == Kind::Picture {
                assert!(entry.width >= 1920 || entry.height >= 1080, "{who} is a small picture");
            } else {
                assert!(entry.duration.is_some_and(|seconds| seconds > 0.0), "{who} is a video without a length");
                assert!(["webm", "mp4"].contains(&entry.extension().as_str()), "{who} is a video this release may not play: {}", entry.url);
            }
        }
    }

    #[test]
    fn a_file_name_comes_from_the_id_and_the_address() {
        assert_eq!(entry("wc-12", "a", "sky", "https://x.org/a/b/Name.JPG").file_name(), "wc-12.jpg");
        assert_eq!(entry("wc-12", "a", "sky", "https://x.org/a/b/Name.webm?x=1").file_name(), "wc-12.webm");
        assert_eq!(entry("../evil", "a", "sky", "https://x.org/noext").file_name(), "evil.jpg", "an id can never leave the folder");
        assert_eq!(preview_path(Path::new("m"), &entry("wc-12", "a", "sky", "https://x.org/a.png")), Path::new("m").join("backdrops").join("previews").join("wc-12.jpg"));
    }

    #[test]
    fn a_search_needs_every_word_in_the_title_the_category_or_the_author() {
        let list = vec![entry("1", "Sunset over the sea", "water", "https://x.org/a.jpg"), entry("2", "Mountain mist", "mountains", "https://x.org/b.jpg"), entry("3", "Cloud study", "sky", "https://x.org/c.jpg")];
        assert_eq!(matching(&list, None, ""), vec![0, 1, 2]);
        assert_eq!(matching(&list, Some("mountains"), ""), vec![1]);
        assert_eq!(matching(&list, None, "SEA sunset"), vec![0]);
        assert_eq!(matching(&list, None, "author"), vec![0, 1, 2]);
        assert_eq!(matching(&list, Some("sky"), "sea"), Vec::<usize>::new());
    }

    #[test]
    fn a_bright_or_busy_backdrop_asks_for_the_looks_dimming() {
        let mut item = entry("1", "a", "sky", "https://x.org/a.jpg");
        assert!(!item.wants_dimming());
        item.brightness = Some(70);
        assert!(item.wants_dimming());
        item.brightness = Some(20);
        item.busy = Some(90);
        assert!(item.wants_dimming());
    }
}
