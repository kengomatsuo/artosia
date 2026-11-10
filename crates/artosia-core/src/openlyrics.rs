//! Reading OpenLyrics, the XML song format worship programs exchange (openlyrics.org).
//!
//! A `<verse name="v1" lang="id">` is a section and each `<lines>` in it is one slide, so the
//! writer's slide breaks survive. Verses that repeat a name in another `lang` are the same
//! section in another language: the first language is the song's words, and every further
//! language is stacked, slide by slide, into its second text, so no lyric is lost. Chords,
//! comments and formatting tags are dropped.

use std::path::Path;

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;

use crate::song::{Song, parse_tag};

struct Verse {
    name: String,
    lang: String,
    /// One entry per `<lines>`: the lines of a slide.
    slides: Vec<Vec<String>>,
}

fn attribute(start: &quick_xml::events::BytesStart<'_>, name: &str) -> String {
    // Verse names and language codes carry no entities, so the raw value is the value.
    start.try_get_attribute(name).ok().flatten().map(|attribute| attribute.value.to_string()).unwrap_or_default()
}

fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The marker text of the verses in one language.
fn marker_text(verses: &[&Verse]) -> String {
    let mut out = String::new();
    for verse in verses {
        let Some(tag) = parse_tag(&verse.name) else { continue };
        out.push_str(&tag.marker());
        out.push('\n');
        for (at, slide) in verse.slides.iter().enumerate() {
            if at > 0 {
                out.push('\n');
            }
            for line in slide {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// Reads one OpenLyrics document. None when it is not XML or holds no lyrics.
pub fn read(xml: &str) -> Option<Song> {
    let mut reader = Reader::from_str(xml);
    let mut path: Vec<String> = Vec::new();
    let mut titles: Vec<String> = Vec::new();
    let mut authors: Vec<String> = Vec::new();
    let mut song = Song::default();
    let mut verses: Vec<Verse> = Vec::new();
    let mut text = String::new();
    let mut lines: Vec<String> = Vec::new();
    // Text inside a comment or a chord is not sung.
    let mut muted = 0usize;
    loop {
        let event = reader.read_event().ok()?;
        match event {
            Event::Start(start) => {
                let name = start.local_name().as_ref().to_string();
                match name.as_str() {
                    "verse" => verses.push(Verse { name: attribute(&start, "name"), lang: attribute(&start, "lang"), slides: Vec::new() }),
                    "lines" => {
                        lines.clear();
                        text.clear();
                    }
                    "comment" | "chord" => muted += 1,
                    "title" | "author" | "copyright" | "ccliNo" | "key" | "verseOrder" => text.clear(),
                    _ => {}
                }
                path.push(name);
            }
            Event::Empty(start) => {
                if start.local_name().as_ref() == "br" && path.iter().any(|name| name == "lines") {
                    lines.push(std::mem::take(&mut text));
                }
            }
            Event::Text(content) => {
                if muted == 0 {
                    text.push_str(&content);
                }
            }
            Event::CData(content) => {
                if muted == 0 {
                    text.push_str(&content);
                }
            }
            Event::GeneralRef(entity) => {
                if muted == 0 {
                    if let Ok(Some(character)) = entity.resolve_char_ref() {
                        text.push(character);
                    } else if let Some(value) = resolve_predefined_entity(&entity) {
                        text.push_str(value);
                    }
                }
            }
            Event::End(_) => {
                let Some(name) = path.pop() else { continue };
                match name.as_str() {
                    "comment" | "chord" => muted = muted.saturating_sub(1),
                    "lines" => {
                        lines.push(std::mem::take(&mut text));
                        let slide: Vec<String> = lines.drain(..).map(|line| tidy(&line)).filter(|line| !line.is_empty()).collect();
                        if let (Some(verse), false) = (verses.last_mut(), slide.is_empty()) {
                            verse.slides.push(slide);
                        }
                    }
                    "title" if !tidy(&text).is_empty() => titles.push(tidy(&text)),
                    "author" if !tidy(&text).is_empty() => authors.push(tidy(&text)),
                    "copyright" => song.copyright = tidy(&text),
                    "ccliNo" => song.ccli = tidy(&text),
                    "key" => song.key = tidy(&text),
                    "verseOrder" => song.order = tidy(&text),
                    _ => {}
                }
                if !path.iter().any(|name| name == "lines") {
                    text.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    verses.retain(|verse| !verse.slides.is_empty());
    let first_lang = verses.first()?.lang.clone();
    let of = |lang: &str| -> Vec<&Verse> { verses.iter().filter(|verse| verse.lang == lang).collect() };
    song.lyrics = marker_text(&of(&first_lang));
    // Every other language, in the order the file introduces them, shares the second text.
    let mut stacked: Vec<Verse> = Vec::new();
    for verse in verses.iter().filter(|verse| verse.lang != first_lang) {
        let at = match stacked.iter().position(|other| other.name == verse.name) {
            Some(at) => at,
            None => {
                stacked.push(Verse { name: verse.name.clone(), lang: String::new(), slides: Vec::new() });
                stacked.len() - 1
            }
        };
        let slides = &mut stacked[at].slides;
        if slides.len() < verse.slides.len() {
            slides.resize(verse.slides.len(), Vec::new());
        }
        for (slide, lines) in slides.iter_mut().zip(&verse.slides) {
            slide.extend(lines.iter().cloned());
        }
    }
    song.second = marker_text(&stacked.iter().collect::<Vec<_>>());
    if song.lyrics.is_empty() {
        return None;
    }
    let mut titles = titles.into_iter();
    song.title = titles.next().unwrap_or_default();
    song.alt_title = titles.collect::<Vec<_>>().join(" / ");
    song.author = authors.join(", ");
    Some(song)
}

/// What a library's `index.json` says about a song beside its XML: its kind and its other
/// titles, as Artosia's cleaned church library writes them. Missing or unreadable means none.
fn index_rows(folder: &Path) -> std::collections::HashMap<String, (String, Vec<String>)> {
    let mut out = std::collections::HashMap::new();
    let found = [folder.join("index.json"), folder.join("..").join("index.json")].into_iter().find_map(|path| std::fs::read_to_string(path).ok());
    let Some(rows) = found.and_then(|text| serde_json::from_str::<Vec<serde_json::Value>>(&text).ok()) else { return out };
    for row in rows {
        let Some(id) = row.get("id").and_then(|id| id.as_str()) else { continue };
        let kind = row.get("kind").and_then(|kind| kind.as_str()).unwrap_or("song").to_string();
        let titles = row
            .get("alt_titles")
            .and_then(|titles| titles.as_array())
            .map(|titles| titles.iter().filter_map(|title| title.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        out.insert(id.to_string(), (kind, titles));
    }
    out
}

/// Every OpenLyrics song in a folder (or its `songs` subfolder), with the count of files
/// that could not be read. Empty placeholders are skipped.
pub fn read_folder(folder: &Path) -> (Vec<Song>, usize) {
    let inner = folder.join("songs");
    let folder = if inner.is_dir() { inner } else { folder.to_path_buf() };
    let extras = index_rows(&folder);
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("xml")))
        .collect();
    files.sort();
    let (mut songs, mut failed) = (Vec::new(), 0);
    for file in files {
        let id = file.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
        let extra = extras.get(&id);
        if extra.is_some_and(|(kind, _)| kind == "empty") {
            continue;
        }
        let Some(mut song) = std::fs::read_to_string(&file).ok().and_then(|xml| read(&xml)) else {
            failed += 1;
            continue;
        };
        // The file name is the song's id in its library and survives a change of title.
        song.source = id.clone();
        if song.title.is_empty() {
            song.title = song.first_line().unwrap_or(id);
        }
        if let Some((_, titles)) = extra {
            let mut all: Vec<String> = song.alt_title.split(" / ").filter(|title| !title.is_empty()).map(str::to_string).collect();
            for title in titles {
                if *title != song.title && !all.contains(title) {
                    all.push(title.clone());
                }
            }
            song.alt_title = all.join(" / ");
        }
        songs.push(song);
    }
    (songs, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version='1.0' encoding='UTF-8'?>
<song xmlns="http://openlyrics.info/namespace/2009/song" version="0.9">
  <properties>
    <titles><title lang="id">Pagi di Bukit</title><title lang="en">Morning on the Hill</title></titles>
    <authors><author>A. Writer</author></authors>
    <verseOrder>v1 c v1 c</verseOrder>
  </properties>
  <lyrics>
    <verse name="v1" lang="id">
      <lines>Terang pagi <chord name="G"/>di lembah<br/>Kami bangun &amp; bernyanyi</lines>
      <lines><comment>repeat</comment>S'gala puji bagi-Mu</lines>
    </verse>
    <verse name="c" lang="id"><lines>Nyanyikanlah<br />bersama</lines></verse>
    <verse name="v1" lang="en"><lines>First light in the valley<br/>We wake and sing</lines><lines>All praise to you</lines></verse>
  </lyrics>
</song>"#;

    #[test]
    fn verses_slides_and_titles_are_read() {
        let song = read(SAMPLE).unwrap_or_default();
        assert_eq!(song.title, "Pagi di Bukit");
        assert_eq!(song.alt_title, "Morning on the Hill");
        assert_eq!(song.author, "A. Writer");
        assert_eq!(song.order, "v1 c v1 c");
        assert_eq!(song.lyrics, "[1]\nTerang pagi di lembah\nKami bangun & bernyanyi\n\nS'gala puji bagi-Mu\n[chorus]\nNyanyikanlah\nbersama\n");
        assert_eq!(song.second, "[1]\nFirst light in the valley\nWe wake and sing\n\nAll praise to you\n");
    }

    #[test]
    fn each_lines_element_is_a_slide_and_the_second_language_follows() {
        let song = read(SAMPLE).unwrap_or_default();
        let deck = crate::deck::song_deck(&song, 4, "");
        let texts: Vec<&str> = deck.slides.iter().map(|slide| slide.text.as_str()).collect();
        assert_eq!(texts.len(), 6);
        assert_eq!(texts[1], "S'gala puji bagi-Mu");
        assert_eq!(deck.slides[1].second, "All praise to you");
        assert_eq!(deck.slides[2].label, "Chorus");
    }

    #[test]
    fn a_third_language_is_stacked_under_the_second() {
        let xml = r#"<song><lyrics>
            <verse name="v1" lang="id"><lines>Satu</lines><lines>Dua</lines></verse>
            <verse name="v1" lang="en"><lines>One</lines><lines>Two</lines></verse>
            <verse name="v1" lang="jv"><lines>Siji</lines><lines>Loro</lines></verse>
        </lyrics></song>"#;
        let song = read(xml).unwrap_or_default();
        assert_eq!(song.lyrics, "[1]
Satu

Dua
");
        assert_eq!(song.second, "[1]
One
Siji

Two
Loro
");
        let deck = crate::deck::song_deck(&song, 4, "");
        assert_eq!(deck.slides[1].second, "Two
Loro");
    }

    #[test]
    fn text_that_is_not_a_song_is_refused() {
        assert!(read("not xml at all").is_none());
        assert!(read("<song><properties/></song>").is_none());
    }
}
