//! Songs: lyrics kept as plain text with section markers, split into sections and slides.
//!
//! The marker text is what a volunteer edits. A line holding only `[1]`, `[chorus]`,
//! `[bridge 2]`, `Verse 1` or `Chorus:` starts a section; this is the EasiSlides convention,
//! widened to the headings lyric websites use. Inside a marked section a blank line starts
//! a new slide; with no markers at all a blank line starts a verse.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionKind {
    Verse,
    PreChorus,
    Chorus,
    Bridge,
    Intro,
    Ending,
    Other,
}

/// One section tag: a kind and its number (Verse 2, Chorus 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Tag {
    pub kind: SectionKind,
    pub number: u8,
}

impl Tag {
    pub const fn new(kind: SectionKind, number: u8) -> Tag {
        Tag { kind, number }
    }

    /// The label a volunteer reads: "Verse 2", "Chorus", "Chorus 2".
    pub fn label(&self) -> String {
        let name = match self.kind {
            SectionKind::Verse => "Verse",
            SectionKind::PreChorus => "Pre-chorus",
            SectionKind::Chorus => "Chorus",
            SectionKind::Bridge => "Bridge",
            SectionKind::Intro => "Intro",
            SectionKind::Ending => "Ending",
            SectionKind::Other => "Part",
        };
        let numbered = self.kind == SectionKind::Verse || self.number > 1;
        if numbered { format!("{name} {}", self.number) } else { name.to_string() }
    }

    /// The chip text for arrangement strips: "V2", "C", "B".
    pub fn short(&self) -> String {
        let letter = match self.kind {
            SectionKind::Verse => "V",
            SectionKind::PreChorus => "P",
            SectionKind::Chorus => "C",
            SectionKind::Bridge => "B",
            SectionKind::Intro => "I",
            SectionKind::Ending => "E",
            SectionKind::Other => "X",
        };
        let numbered = self.kind == SectionKind::Verse || self.number > 1;
        if numbered { format!("{letter}{}", self.number) } else { letter.to_string() }
    }

    /// The marker line written back into lyrics text.
    pub fn marker(&self) -> String {
        match self.kind {
            SectionKind::Verse => format!("[{}]", self.number),
            SectionKind::PreChorus if self.number > 1 => format!("[prechorus {}]", self.number),
            SectionKind::PreChorus => "[prechorus]".to_string(),
            SectionKind::Chorus if self.number > 1 => format!("[chorus {}]", self.number),
            SectionKind::Chorus => "[chorus]".to_string(),
            SectionKind::Bridge if self.number > 1 => format!("[bridge {}]", self.number),
            SectionKind::Bridge => "[bridge]".to_string(),
            SectionKind::Intro => "[intro]".to_string(),
            SectionKind::Ending => "[ending]".to_string(),
            SectionKind::Other => format!("[part {}]", self.number),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub tag: Tag,
    pub lines: Vec<String>,
    /// Indexes into `lines` where the writer started a new slide.
    pub breaks: Vec<usize>,
}

impl Section {
    /// The section's lines as the writer grouped them into slides.
    pub fn groups(&self) -> Vec<&[String]> {
        let mut out = Vec::with_capacity(self.breaks.len() + 1);
        let mut from = 0;
        for &at in &self.breaks {
            if at > from && at < self.lines.len() {
                out.push(&self.lines[from..at]);
                from = at;
            }
        }
        out.push(&self.lines[from..]);
        out
    }

    /// The slides of this section: the writer's groups, each split evenly when longer than
    /// `max_lines`.
    pub fn slides(&self, max_lines: usize) -> Vec<Vec<String>> {
        self.groups().into_iter().flat_map(|group| split_lines(group, max_lines)).collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Song {
    pub id: i64,
    pub title: String,
    pub alt_title: String,
    pub author: String,
    pub copyright: String,
    pub ccli: String,
    pub key: String,
    pub number: u32,
    /// Lyrics with section markers.
    pub lyrics: String,
    /// The same sections in a second language, with the same markers; empty when there is none.
    pub second: String,
    /// Play order as short tags separated by spaces or commas ("V1 C V2 C B C"); empty plays
    /// the sections as written.
    pub order: String,
    /// The id the song has in the library it was imported from, such as an OpenLyrics file
    /// name; empty for songs written here. A later import with the same id updates the song.
    #[serde(default)]
    pub source: String,
    /// The song was changed in Artosia after it was imported, so a later import leaves it alone.
    #[serde(default)]
    pub edited: bool,
}

fn kind_from_word(word: &str) -> Option<SectionKind> {
    Some(match word {
        "verse" | "v" | "vers" | "stanza" | "bait" | "ayat" => SectionKind::Verse,
        "chorus" | "c" | "refrain" | "ref" | "reff" | "koor" | "chor" => SectionKind::Chorus,
        "prechorus" | "pre" | "p" => SectionKind::PreChorus,
        "bridge" | "b" | "jembatan" => SectionKind::Bridge,
        "intro" | "i" | "pembuka" => SectionKind::Intro,
        "ending" | "end" | "outro" | "tag" | "coda" | "penutup" | "akhir" | "e" => SectionKind::Ending,
        "part" | "x" | "o" | "other" => SectionKind::Other,
        _ => return None,
    })
}

/// Reads a tag from marker text such as "1", "chorus 2", "Verse 3", "pre-chorus".
pub fn parse_tag(text: &str) -> Option<Tag> {
    let cleaned: String = text
        .trim()
        .trim_end_matches(':')
        .chars()
        .filter(|c| *c != '-' && *c != '.')
        .collect::<String>()
        .to_lowercase();
    let mut parts = cleaned.split_whitespace();
    let first = parts.next()?;
    let second = parts.next();
    if parts.next().is_some() {
        return None;
    }
    if let Ok(number) = first.parse::<u8>() {
        return second.is_none().then_some(Tag::new(SectionKind::Verse, number.max(1)));
    }
    // "chorus2" and "v1" carry their number without a space.
    let split = first.find(|c: char| c.is_ascii_digit()).unwrap_or(first.len());
    let (word, digits) = first.split_at(split);
    let kind = kind_from_word(word)?;
    let number = match (digits.is_empty(), second) {
        (false, None) => digits.parse::<u8>().ok()?,
        (true, Some(n)) => n.parse::<u8>().ok()?,
        (true, None) => 1,
        (false, Some(_)) => return None,
    };
    Some(Tag::new(kind, number.max(1)))
}

/// A line that is only a section heading, bracketed or as lyric sites write it.
pub fn heading(line: &str) -> Option<Tag> {
    let line = line.trim();
    if let Some(inner) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        return parse_tag(inner);
    }
    if let Some(inner) = line.strip_prefix('(').and_then(|rest| rest.strip_suffix(')')) {
        return parse_tag(inner).filter(|tag| tag.kind != SectionKind::Verse || inner.to_lowercase().contains('v'));
    }
    // A bare heading must name its kind in full, so a lyric line such as "I" or "End" stays a lyric.
    let lower = line.trim_end_matches(':').to_lowercase();
    let word = lower.split(|c: char| c.is_whitespace() || c.is_ascii_digit()).next().unwrap_or("");
    let full = matches!(
        word,
        "verse" | "chorus" | "refrain" | "bridge" | "intro" | "outro" | "ending" | "pre-chorus" | "prechorus" | "tag" | "reff" | "bait" | "koor" | "jembatan" | "pembuka" | "penutup"
    );
    if full && line.len() <= 16 { parse_tag(line) } else { None }
}

/// Splits marker text into sections. Blank lines inside a marked section are dropped.
pub fn parse_lyrics(lyrics: &str) -> Vec<Section> {
    let marked = lyrics.lines().any(|line| heading(line).is_some());
    let mut sections: Vec<Section> = Vec::new();
    let mut verse = 0u8;
    let mut open = false;
    for line in lyrics.lines() {
        let trimmed = line.trim();
        if let Some(tag) = heading(trimmed) {
            sections.push(Section { tag, lines: Vec::new(), breaks: Vec::new() });
            open = true;
            continue;
        }
        if trimmed.is_empty() {
            if !marked {
                open = false;
            } else if let Some(section) = sections.last_mut().filter(|section| open && !section.lines.is_empty()) {
                // A blank line inside a marked section is where the next slide starts.
                if section.breaks.last() != Some(&section.lines.len()) {
                    section.breaks.push(section.lines.len());
                }
            }
            continue;
        }
        if !open {
            verse = verse.saturating_add(1);
            sections.push(Section { tag: Tag::new(SectionKind::Verse, verse), lines: Vec::new(), breaks: Vec::new() });
            open = true;
        }
        if let Some(section) = sections.last_mut() {
            section.lines.push(trimmed.to_string());
        }
    }
    sections.retain(|section| !section.lines.is_empty());
    for section in &mut sections {
        // A break after the last line starts nothing.
        let count = section.lines.len();
        section.breaks.retain(|at| *at < count);
    }
    sections
}

/// Writes sections back as marker text.
pub fn write_lyrics(sections: &[Section]) -> String {
    let mut out = String::new();
    for section in sections {
        out.push_str(&section.tag.marker());
        out.push('\n');
        for (at, line) in section.lines.iter().enumerate() {
            if section.breaks.contains(&at) {
                out.push('\n');
            }
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Reads a play order. Accepts "V1 C V2" and EasiSlides' "1,c,2,c,b,t,e". In the EasiSlides
/// database read on 2026-11-10, `t` appears only in songs with a `[chorus 2]` and `e` only in
/// songs with an `[ending]`; no other letters occur there, so none are guessed.
///
/// A star repeats: "C*2" sings the chorus twice, and a lone "*3" sings the section before it
/// three times in all. That is Artosia's own marker; EasiSlides has none.
pub fn parse_order(order: &str) -> Vec<Tag> {
    let mut out: Vec<Tag> = Vec::new();
    for token in order.split(|c: char| c == ',' || c.is_whitespace()).filter(|token| !token.is_empty()) {
        let lower = token.to_lowercase();
        let (name, times) = match lower.split_once(['*', '\u{d7}']) {
            Some((name, times)) => (name, times.parse::<usize>().unwrap_or(1).clamp(1, 9)),
            None => (lower.as_str(), 1),
        };
        let tag = match name {
            "" => out.last().copied(),
            "t" => Some(Tag::new(SectionKind::Chorus, 2)),
            other => parse_tag(other),
        };
        let Some(tag) = tag else { continue };
        // A lone "*2" already has its first singing in the list.
        let more = if name.is_empty() { times - 1 } else { times };
        out.extend(std::iter::repeat_n(tag, more));
    }
    out
}

/// Writes a play order as text, "V1 C*2 V2", folding a section sung several times in a row.
pub fn write_order(tags: &[Tag]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut at = 0;
    while at < tags.len() {
        let run = tags[at..].iter().take_while(|tag| **tag == tags[at]).count();
        out.push(if run > 1 { format!("{}*{run}", tags[at].short()) } else { tags[at].short() });
        at += run;
    }
    out.join(" ")
}

/// Whether a line holds only chord names, as a chord sheet writes above each lyric line.
fn is_chord_line(line: &str) -> bool {
    let is_chord = |token: &str| {
        let mut chars = token.chars();
        if !chars.next().is_some_and(|c| ('A'..='G').contains(&c)) {
            return false;
        }
        let rest: String = chars.collect();
        let rest = rest.trim_start_matches(['#', 'b']);
        let (quality, bass) = rest.split_once('/').unwrap_or((rest, ""));
        let known = ["", "m", "maj", "min", "sus", "dim", "aug", "add", "M"];
        let quality = quality.trim_end_matches(|c: char| c.is_ascii_digit());
        let bass_ok = bass.is_empty() || (bass.len() <= 2 && bass.starts_with(|c: char| ('A'..='G').contains(&c)));
        known.contains(&quality) && bass_ok
    };
    let tokens: Vec<&str> = line.split_whitespace().filter(|token| *token != "|" && *token != "-").collect();
    tokens.len() >= 2 && tokens.iter().all(|token| is_chord(token))
}

/// Cleans words pasted from anywhere: headings such as "Verse 1" or "Reff:" become markers,
/// chord lines go, stray spaces go, and unmarked verses are numbered.
///
/// Words with no heading at all keep their stanzas unmarked: `marks::propose` offers them
/// marks, and numbering them here would call a chorus sung three times "Verse 2, 4 and 6".
pub fn tidy_lyrics(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let kept: Vec<&str> = text.lines().filter(|line| !is_chord_line(line.trim())).collect();
    if !kept.iter().any(|line| heading(line).is_some()) {
        let mut out = String::new();
        let mut gap = false;
        for line in kept.iter().map(|line| line.trim()) {
            if line.is_empty() {
                gap = !out.is_empty();
                continue;
            }
            if gap {
                out.push('\n');
                gap = false;
            }
            out.push_str(line);
            out.push('\n');
        }
        return out;
    }
    write_lyrics(&parse_lyrics(&kept.join("\n")))
}

impl Song {
    pub fn sections(&self) -> Vec<Section> {
        parse_lyrics(&self.lyrics)
    }

    /// Sections in the order they are sung: the written order, or `order` when it names
    /// sections that exist. Tags the song lacks are skipped.
    pub fn arranged(&self) -> Vec<Section> {
        let sections = self.sections();
        let order = parse_order(&self.order);
        if order.is_empty() {
            return sections;
        }
        let arranged: Vec<Section> =
            order.iter().filter_map(|tag| sections.iter().find(|section| section.tag == *tag).cloned()).collect();
        if arranged.is_empty() { sections } else { arranged }
    }

    /// The order with the chorus sung after every verse, for hymns written verse, chorus, verse.
    pub fn order_with_repeated_chorus(&self) -> Option<String> {
        let sections = self.sections();
        let chorus = sections.iter().find(|section| section.tag.kind == SectionKind::Chorus)?.tag;
        let mut order: Vec<String> = Vec::new();
        for section in &sections {
            if section.tag.kind == SectionKind::Chorus {
                continue;
            }
            order.push(section.tag.short());
            if section.tag.kind == SectionKind::Verse {
                order.push(chorus.short());
            }
        }
        Some(order.join(" "))
    }

    /// The first lyric line, a fallback title for pasted text.
    pub fn first_line(&self) -> Option<String> {
        self.sections().first().and_then(|section| section.lines.first().cloned())
    }
}

/// Splits a section's lines into slides of at most `max_lines`, keeping the pieces even:
/// six lines at a limit of four become three and three, never four and two.
pub fn split_lines(lines: &[String], max_lines: usize) -> Vec<Vec<String>> {
    if max_lines == 0 || lines.len() <= max_lines {
        return vec![lines.to_vec()];
    }
    let pieces = lines.len().div_ceil(max_lines);
    let base = lines.len() / pieces;
    let extra = lines.len() % pieces;
    let mut out = Vec::with_capacity(pieces);
    let mut at = 0;
    for piece in 0..pieces {
        let take = base + usize::from(piece < extra);
        out.push(lines[at..at + take].to_vec());
        at += take;
    }
    out
}

/// Builds a song from pasted text: the first line is the title when a blank line follows it.
pub fn song_from_paste(text: &str) -> Song {
    let text = text.replace("\r\n", "\n");
    let mut lines = text.lines().skip_while(|line| line.trim().is_empty());
    let first = lines.next().unwrap_or("").trim().to_string();
    let rest: Vec<&str> = lines.collect();
    let titled = rest.first().is_some_and(|line| line.trim().is_empty()) && heading(&first).is_none();
    let (title, body) = if titled {
        (first, rest.join("\n"))
    } else {
        let body = std::iter::once(first.as_str()).chain(rest.iter().copied()).collect::<Vec<_>>().join("\n");
        (String::new(), body)
    };
    let mut song = Song { title, lyrics: tidy_lyrics(&body), ..Song::default() };
    if song.title.is_empty() {
        song.title = song.first_line().unwrap_or_default();
    }
    song
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The words a church writes above a stanza: the lyrics box shows `[Bait 1]` and `[Reff]`
    /// as its example in Indonesian, so both must mean what `[1]` and `[chorus]` mean.
    #[test]
    fn indonesian_marker_words_are_read() {
        let song = Song { lyrics: "[Bait 1]\nsatu\n\n[Reff]\ndua\n\n[bait 2]\ntiga\n\n[REFF]\nempat\n".into(), ..Song::default() };
        let tags: Vec<Tag> = song.sections().iter().map(|section| section.tag).collect();
        assert_eq!(tags, [Tag::new(SectionKind::Verse, 1), Tag::new(SectionKind::Chorus, 1), Tag::new(SectionKind::Verse, 2), Tag::new(SectionKind::Chorus, 1)]);
    }

    const MARKED: &str = "[1]\nFirst light on the hill\nSecond line here\n[chorus]\nSing it out\nSing it loud\n[2]\nAnother verse\n";

    #[test]
    fn a_star_in_the_order_repeats_a_section() {
        let chorus = Tag::new(SectionKind::Chorus, 1);
        let verse = Tag::new(SectionKind::Verse, 1);
        assert_eq!(parse_order("V1 C*2"), [verse, chorus, chorus]);
        assert_eq!(parse_order("V1 C *3"), [verse, chorus, chorus, chorus]);
        assert_eq!(write_order(&[verse, chorus, chorus, verse]), "V1 C*2 V1");
        assert_eq!(parse_order(&write_order(&[verse, chorus, chorus])), [verse, chorus, chorus]);
    }

    #[test]
    fn tidying_makes_markers_and_drops_chords() {
        let pasted = "Verse 1\nG   D/F#  Em\nWalk the long road   \nA single line\n\n\nReff:\nC G\nSing it once more\n";
        assert_eq!(tidy_lyrics(pasted), "[1]\nWalk the long road\nA single line\n[chorus]\nSing it once more\n");
        // A line of words that happen to start with chord letters stays.
        assert_eq!(tidy_lyrics("[1]\nA Friend Goes Before\n"), "[1]\nA Friend Goes Before\n");
    }

    #[test]
    fn tidying_words_with_no_heading_keeps_the_stanzas_unmarked() {
        let pasted = "G   D\nFirst line  \nSecond line\n\n\n\nThird line\n\n";
        assert_eq!(tidy_lyrics(pasted), "First line\nSecond line\n\nThird line\n");
        assert_eq!(song_from_paste("Title\n\nOne\nTwo\n\nThree").lyrics, "One\nTwo\n\nThree\n");
    }

    #[test]
    fn bracket_markers_split_sections() {
        let sections = parse_lyrics(MARKED);
        let labels: Vec<String> = sections.iter().map(|s| s.tag.label()).collect();
        assert_eq!(labels, ["Verse 1", "Chorus", "Verse 2"]);
        assert_eq!(sections[1].lines, ["Sing it out", "Sing it loud"]);
    }

    #[test]
    fn web_headings_and_blank_lines_work() {
        let sections = parse_lyrics("Verse 1\nOne\nTwo\n\nChorus:\nThree\n\nBridge\nFour");
        let labels: Vec<String> = sections.iter().map(|s| s.tag.label()).collect();
        assert_eq!(labels, ["Verse 1", "Chorus", "Bridge"]);
    }

    #[test]
    fn unmarked_text_splits_on_blank_lines() {
        let sections = parse_lyrics("One\nTwo\n\nThree\nFour\n\n\nFive");
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[2].tag, Tag::new(SectionKind::Verse, 3));
    }

    #[test]
    fn a_lyric_line_that_looks_like_a_letter_stays_a_lyric() {
        let sections = parse_lyrics("[1]\nI\nEnd of the road\n");
        assert_eq!(sections[0].lines, ["I", "End of the road"]);
    }

    #[test]
    fn order_follows_easislides_letters() {
        let song = Song { lyrics: format!("{MARKED}[chorus 2]\nLast time\n"), order: "1,c,2,t".into(), ..Song::default() };
        let labels: Vec<String> = song.arranged().iter().map(|s| s.tag.label()).collect();
        assert_eq!(labels, ["Verse 1", "Chorus", "Verse 2", "Chorus 2"]);
    }

    #[test]
    fn an_order_naming_missing_sections_falls_back() {
        let song = Song { lyrics: MARKED.into(), order: "B B".into(), ..Song::default() };
        assert_eq!(song.arranged().len(), 3);
    }

    #[test]
    fn chorus_repeats_after_each_verse() {
        let song = Song { lyrics: MARKED.into(), ..Song::default() };
        assert_eq!(song.order_with_repeated_chorus().as_deref(), Some("V1 C V2 C"));
    }

    #[test]
    fn long_sections_split_evenly() {
        let lines: Vec<String> = (1..=6).map(|n| n.to_string()).collect();
        let slides = split_lines(&lines, 4);
        assert_eq!(slides.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3]);
        assert_eq!(split_lines(&lines, 0).len(), 1);
    }

    #[test]
    fn markers_round_trip() {
        assert_eq!(write_lyrics(&parse_lyrics(MARKED)), MARKED);
    }

    #[test]
    fn a_blank_line_inside_a_section_starts_a_slide() {
        let text = "[1]\none\ntwo\n\nthree\n[chorus]\nfour\n\n\nfive\n\n";
        let sections = parse_lyrics(text);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].slides(4), [vec!["one".to_string(), "two".to_string()], vec!["three".to_string()]]);
        assert_eq!(sections[1].slides(4).len(), 2);
        assert_eq!(write_lyrics(&sections), "[1]\none\ntwo\n\nthree\n[chorus]\nfour\n\nfive\n");
    }

    #[test]
    fn pasted_text_takes_its_title_from_the_first_line() {
        let song = song_from_paste("Hill Song\n\nVerse 1\nFirst light\n\nChorus\nSing it out");
        assert_eq!(song.title, "Hill Song");
        assert_eq!(song.sections().len(), 2);
        let untitled = song_from_paste("First light\nSecond line\n\nThird");
        assert_eq!(untitled.title, "First light");
    }
}
