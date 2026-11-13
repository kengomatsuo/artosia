//! Instant, typo-forgiving search over songs.
//!
//! Every word of every title and lyric line goes into a sorted vocabulary with the lines it
//! appears in. A query token is matched against the vocabulary (exact, prefix, or within a
//! typo budget), never against every line, so a keystroke costs a few thousand short string
//! compares whatever the size of the library.

use std::cell::Cell;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::ops::Range;

use crate::song::{Song, Tag};
use crate::text::{INFIX_MIN, WordMatch, highlight_ranges, match_quality, match_word, words};

/// In a table of ranks: this token did not match this entry.
const NO_MATCH: u8 = u8::MAX;

/// An inverted index over numbered text entries.
#[derive(Default)]
pub struct TextIndex {
    vocab: Vec<Box<str>>,
    postings: Vec<Vec<u32>>,
    entries: u32,
}

/// A ranked entry of a [`TextIndex`] query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntryScore {
    pub entry: u32,
    /// How many query tokens the entry matched.
    pub matched: u8,
    /// Bit i is set when token i matched.
    pub mask: u16,
    pub score: f32,
}

/// The most tokens a query contributes; later ones are ignored.
pub const MAX_TOKENS: usize = 12;

/// Splits a query into folded tokens.
pub fn tokens(query: &str) -> Vec<String> {
    words(query).into_iter().map(|w| w.norm).take(MAX_TOKENS).collect()
}

impl TextIndex {
    /// Indexes `texts`; an entry's id is its position in the iterator.
    pub fn build<'a>(texts: impl IntoIterator<Item = &'a str>) -> TextIndex {
        let mut by_word: HashMap<String, Vec<u32>> = HashMap::new();
        let mut entries = 0u32;
        for text in texts {
            for word in words(text) {
                // An elided word is findable by both spellings: `s'gala` and `segala`.
                for form in std::iter::once(word.norm).chain(word.full) {
                    let list = by_word.entry(form).or_default();
                    if list.last() != Some(&entries) {
                        list.push(entries);
                    }
                }
            }
            entries += 1;
        }
        let mut pairs: Vec<(String, Vec<u32>)> = by_word.into_iter().collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let mut vocab = Vec::with_capacity(pairs.len());
        let mut postings = Vec::with_capacity(pairs.len());
        for (word, list) in pairs {
            vocab.push(word.into_boxed_str());
            postings.push(list);
        }
        TextIndex { vocab, postings, entries }
    }

    pub fn len(&self) -> usize {
        self.entries as usize
    }

    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }

    /// Vocabulary words a token may stand for, with how well each matches.
    fn candidates(&self, token: &str, typing: bool) -> Vec<(usize, f32)> {
        let mut out = Vec::new();
        let first = token.chars().next();
        for (i, word) in self.vocab.iter().enumerate() {
            // A typo in the first letter is rare; skipping those words keeps short tokens cheap.
            let same_start = word.chars().next() == first;
            if !same_start && token.chars().count() < 5 {
                continue;
            }
            let quality = match_quality(token, word, typing);
            if quality > 0.0 {
                out.push((i, quality));
            }
        }
        out
    }

    /// Vocabulary words a token may stand for, with the rank of each match. Unlike
    /// [`Self::candidates`] this also finds the token inside a longer word.
    fn word_ranks(&self, token: &str, typing: bool) -> Vec<(usize, u8)> {
        let mut out = Vec::new();
        let first = token.chars().next();
        let chars = token.chars().count();
        for (i, word) in self.vocab.iter().enumerate() {
            let same_start = word.chars().next() == first;
            let found = if same_start || chars >= 5 {
                match_word(token, word, typing)
            } else if chars >= INFIX_MIN && word.contains(token) {
                Some(WordMatch { typos: 0, exact: false, infix: true })
            } else {
                None
            };
            if let Some(found) = found {
                out.push((i, found.rank()));
            }
        }
        out
    }

    /// How each token matched each entry: `table[entry * tokens.len() + token]` is the best
    /// [`WordMatch::rank`] among the entry's words, or `NO_MATCH`.
    fn ranks(&self, tokens: &[String]) -> Vec<u8> {
        let count = tokens.len();
        let mut table = vec![NO_MATCH; self.entries as usize * count];
        for (i, token) in tokens.iter().enumerate() {
            for (word, rank) in self.word_ranks(token, i + 1 == count) {
                for &entry in &self.postings[word] {
                    let slot = &mut table[entry as usize * count + i];
                    *slot = (*slot).min(rank);
                }
            }
        }
        table
    }

    /// Scores every entry that matches at least one token, best first.
    pub fn query(&self, tokens: &[String]) -> Vec<EntryScore> {
        let size = self.entries as usize;
        if tokens.is_empty() || size == 0 {
            return Vec::new();
        }
        let mut score = vec![0f32; size];
        let mut mask = vec![0u16; size];
        let mut best = vec![0f32; size];
        let last = tokens.len() - 1;
        for (i, token) in tokens.iter().enumerate().take(MAX_TOKENS) {
            let candidates = self.candidates(token, i == last);
            if candidates.is_empty() {
                continue;
            }
            best.fill(0.0);
            for (word, quality) in candidates {
                // A word found everywhere says little; a rare word says a lot.
                let rarity = 1.0 + (size as f32 / (self.postings[word].len() as f32 + 1.0)).ln().max(0.0) * 0.15;
                for &entry in &self.postings[word] {
                    let value = quality * rarity;
                    if value > best[entry as usize] {
                        best[entry as usize] = value;
                    }
                }
            }
            for entry in 0..size {
                if best[entry] > 0.0 {
                    score[entry] += best[entry];
                    mask[entry] |= 1 << i;
                }
            }
        }
        let mut out: Vec<EntryScore> = (0..size)
            .filter(|&entry| mask[entry] != 0)
            .map(|entry| EntryScore {
                entry: entry as u32,
                matched: mask[entry].count_ones() as u8,
                mask: mask[entry],
                score: score[entry],
            })
            .collect();
        out.sort_by(|a, b| b.matched.cmp(&a.matched).then(b.score.total_cmp(&a.score)).then(a.entry.cmp(&b.entry)));
        out
    }
}

/// How close together an entry's matched words sit, and where the run starts: (words in
/// between, index of the first). (MAX, MAX) when nothing matched.
fn window(text: &str, tokens: &[String]) -> (u32, u32) {
    let last = tokens.len().saturating_sub(1);
    let masks: Vec<u16> = words(text)
        .iter()
        .map(|word| {
            tokens.iter().enumerate().fold(0u16, |mask, (i, token)| {
                let hit = match_word(token, &word.norm, i == last).is_some() || word.full.as_ref().is_some_and(|full| match_word(token, full, i == last).is_some());
                if hit { mask | 1 << i } else { mask }
            })
        })
        .collect();
    let all = masks.iter().fold(0u16, |all, mask| all | mask);
    if all == 0 {
        return (u32::MAX, u32::MAX);
    }
    let wanted = all.count_ones();
    let mut best = (u32::MAX, u32::MAX);
    for start in 0..masks.len() {
        if masks[start] == 0 {
            continue;
        }
        let mut seen = 0u16;
        for (end, mask) in masks.iter().enumerate().skip(start) {
            seen |= mask;
            if seen == all {
                let between = ((end - start + 1) as u32).saturating_sub(wanted);
                if between < best.0 {
                    best = (between, start as u32);
                }
                break;
            }
        }
    }
    best
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineHit {
    /// Index into the song's written sections.
    pub section: usize,
    pub tag: Tag,
    pub text: String,
    pub ranges: Vec<Range<usize>>,
    /// The language code of the line, empty when the song never named it; the song's first
    /// language when the line is one of its written lyrics.
    pub lang: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SongHit {
    /// Index into the slice the index was built from.
    pub song: usize,
    /// The caller asked for this song to go first, such as one in today's plan.
    pub preferred: bool,
    /// The song holds every word of the query.
    pub complete: bool,
    /// The most query words found in one title or one lyric line, and how many words the query
    /// has: a heard phrase that sits in one line is a much stronger match than one whose words
    /// are scattered over the song.
    pub together: u8,
    pub words: u8,
    /// The words were found in the title, not only in the lyrics.
    pub in_title: bool,
    /// How often the volunteer picked this song for the typed text lately (0 to 5). It only
    /// ever orders songs that tied on everything else.
    pub habit: u8,
    pub title_ranges: Vec<Range<usize>>,
    /// The lyric line that matched best, when the lyrics matched at all.
    pub line: Option<LineHit>,
}

struct Entry {
    song: u32,
    /// None for the title line.
    section: Option<u16>,
    tag: Tag,
    text: Box<str>,
    /// Which of the song's languages the line is in: 0 is the first.
    lang: u8,
}

/// What a song's place in the results rests on. The derived order IS the cascade, after the
/// owner's house-kit palette: each field only speaks when every field above it tied.
/// One line differs from the kit, on purpose: a title outranks a whole-word match, because a
/// word that is common in lyrics would otherwise bury the song named after it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Facts {
    /// The query is this song's number.
    numbered: Reverse<bool>,
    /// Query words found anywhere in the song.
    covered: Reverse<u8>,
    preferred: Reverse<bool>,
    /// The most query words found in one title or one line.
    together: Reverse<u8>,
    typos: u8,
    in_title: Reverse<bool>,
    exact: Reverse<u8>,
    infix: u8,
    /// Picks of this song for the typed text in the last 12 weeks, from the caller. LAST: a
    /// plan song, an exact title and a typed number all stay above a habit.
    habit: Reverse<u8>,
}

struct Found {
    song: u32,
    facts: Facts,
    /// The title or line the facts were read from.
    best: u32,
    title_matched: bool,
    line: Option<u32>,
}

/// Search over a song library.
#[derive(Default)]
pub struct SongIndex {
    index: TextIndex,
    entries: Vec<Entry>,
    numbers: Vec<u32>,
    songs: usize,
    /// Each song's language codes, the first language first.
    codes: Vec<Vec<String>>,
}

impl SongIndex {
    pub fn build(songs: &[Song]) -> SongIndex {
        let mut entries: Vec<Entry> = Vec::new();
        let verse = Tag::new(crate::song::SectionKind::Verse, 1);
        for (i, song) in songs.iter().enumerate() {
            let title = if song.alt_title.is_empty() {
                song.title.clone()
            } else {
                format!("{} {}", song.title, song.alt_title)
            };
            entries.push(Entry { song: i as u32, section: None, tag: verse, text: title.into_boxed_str(), lang: 0 });
            let written = song.sections();
            // Every language's lines are searched; a translation's section is the written
            // section with the same tag, so a hit opens the right place whichever language matched.
            for (language, (_, words)) in song.texts().enumerate() {
                let translated;
                let sections = if language == 0 {
                    &written
                } else {
                    translated = crate::song::parse_lyrics(words);
                    &translated
                };
                for (at, section) in sections.iter().enumerate() {
                    let place = if language == 0 { at } else { written.iter().position(|first| first.tag == section.tag).unwrap_or(0) };
                    for line in &section.lines {
                        entries.push(Entry {
                            song: i as u32,
                            section: Some(place as u16),
                            tag: section.tag,
                            text: line.clone().into_boxed_str(),
                            lang: u8::try_from(language).unwrap_or(u8::MAX),
                        });
                    }
                }
            }
        }
        let index = TextIndex::build(entries.iter().map(|entry| &*entry.text));
        let codes = songs.iter().map(|song| song.texts().map(|(code, _)| code.to_string()).collect()).collect();
        SongIndex { index, entries, numbers: songs.iter().map(|song| song.number).collect(), songs: songs.len(), codes }
    }

    pub fn len(&self) -> usize {
        self.songs
    }

    pub fn is_empty(&self) -> bool {
        self.songs == 0
    }

    /// The best `limit` songs for a query, with no song preferred.
    pub fn search(&self, query: &str, limit: usize) -> Vec<SongHit> {
        self.search_with(query, limit, &|_| false)
    }

    /// [`Self::search_ranked`] with no habits.
    pub fn search_with(&self, query: &str, limit: usize, preferred: &dyn Fn(usize) -> bool) -> Vec<SongHit> {
        self.search_ranked(query, limit, preferred, &|_| 0)
    }

    /// The best `limit` songs for a query. A song that holds every word goes before one that
    /// holds some; among those, a song `preferred` names goes first, then words found together
    /// in one line, fewer typos, the title before the lyrics, whole words, and words side by
    /// side. A number finds the song with that number before anything else. `habit` is read
    /// last, so it only orders songs that tie on every one of those.
    pub fn search_ranked(&self, query: &str, limit: usize, preferred: &dyn Fn(usize) -> bool, habit: &dyn Fn(usize) -> u8) -> Vec<SongHit> {
        let tokens = tokens(query);
        if tokens.is_empty() {
            return Vec::new();
        }
        let count = tokens.len();
        let table = self.index.ranks(&tokens);
        let number: Option<u32> = query.trim().parse().ok().filter(|number| *number != 0);
        let mut found: Vec<Found> = Vec::new();
        let mut at = 0;
        while at < self.entries.len() {
            let song = self.entries[at].song;
            let title = at as u32;
            // Per query word, the best match anywhere in this song.
            let mut per_token = [NO_MATCH; MAX_TOKENS];
            // (words in one entry, is the title, summed ranks, entry): what the song is judged by.
            let mut best: Option<(u8, bool, u16, u32)> = None;
            let mut line: Option<(u8, u16, u32)> = None;
            let mut title_matched = false;
            while at < self.entries.len() && self.entries[at].song == song {
                let row = &table[at * count..(at + 1) * count];
                let held = row.iter().filter(|rank| **rank != NO_MATCH).count() as u8;
                if held > 0 {
                    let cost: u16 = row.iter().filter(|rank| **rank != NO_MATCH).map(|rank| u16::from(*rank)).sum();
                    for (slot, rank) in per_token.iter_mut().zip(row) {
                        *slot = (*slot).min(*rank);
                    }
                    let is_title = self.entries[at].section.is_none();
                    title_matched |= is_title;
                    let better = match best {
                        None => true,
                        Some((most, title, least, _)) => (Reverse(held), Reverse(is_title), cost) < (Reverse(most), Reverse(title), least),
                    };
                    if better {
                        best = Some((held, is_title, cost, at as u32));
                    }
                    if !is_title && line.is_none_or(|(most, least, _)| (Reverse(held), cost) < (Reverse(most), least)) {
                        line = Some((held, cost, at as u32));
                    }
                }
                at += 1;
            }
            let numbered = number.is_some() && number == self.numbers.get(song as usize).copied();
            let Some((together, in_title, _, best)) = best else {
                if numbered {
                    let facts = Facts {
                        numbered: Reverse(true),
                        covered: Reverse(0),
                        preferred: Reverse(preferred(song as usize)),
                        together: Reverse(0),
                        typos: 0,
                        in_title: Reverse(false),
                        exact: Reverse(0),
                        infix: 0,
                        habit: Reverse(habit(song as usize)),
                    };
                    found.push(Found { song, facts, best: title, title_matched: false, line: None });
                }
                continue;
            };
            let matched = per_token[..count].iter().filter(|rank| **rank != NO_MATCH);
            let facts = Facts {
                numbered: Reverse(numbered),
                covered: Reverse(matched.clone().count() as u8),
                preferred: Reverse(preferred(song as usize)),
                together: Reverse(together),
                typos: matched.clone().map(|rank| rank / 4).sum(),
                in_title: Reverse(in_title),
                exact: Reverse(matched.clone().filter(|rank| **rank % 4 == 0).count() as u8),
                infix: matched.filter(|rank| **rank % 2 == 1).count() as u8,
                habit: Reverse(habit(song as usize)),
            };
            found.push(Found { song, facts, best, title_matched, line: line.map(|(_, _, entry)| entry) });
        }
        // Words side by side, then nearest the start, decide what is still tied. Reading that
        // costs a pass over the line, so it is read only when two songs tie on all the rest.
        let windows: Vec<Cell<Option<(u32, u32)>>> = vec![Cell::new(None); found.len()];
        let window_of = |i: usize| match windows[i].get() {
            Some(known) => known,
            None => {
                let read = window(&self.entries[found[i].best as usize].text, &tokens);
                windows[i].set(Some(read));
                read
            }
        };
        let mut order: Vec<usize> = (0..found.len()).collect();
        // The habit is the last line of `Facts`, but the side-by-side and nearest-the-start reads
        // below it still come first: a habit only orders what tied on everything else.
        let without_habit = |facts: Facts| Facts { habit: Reverse(0), ..facts };
        order.sort_by(|&a, &b| {
            without_habit(found[a].facts)
                .cmp(&without_habit(found[b].facts))
                .then_with(|| window_of(a).cmp(&window_of(b)))
                .then(found[a].facts.habit.cmp(&found[b].facts.habit))
                .then(found[a].song.cmp(&found[b].song))
        });
        order
            .into_iter()
            .take(limit)
            .map(|i| {
                let hit = &found[i];
                let line = hit.line.map(|at| {
                    let entry = &self.entries[at as usize];
                    LineHit {
                        section: usize::from(entry.section.unwrap_or(0)),
                        tag: entry.tag,
                        text: entry.text.to_string(),
                        ranges: highlight_ranges(&entry.text, &tokens),
                        lang: self.codes[entry.song as usize].get(usize::from(entry.lang)).cloned().unwrap_or_default(),
                    }
                });
                let title_ranges = if hit.title_matched { highlight_ranges(&self.entries[self.title_entry(hit.song) as usize].text, &tokens) } else { Vec::new() };
                SongHit {
                    song: hit.song as usize,
                    preferred: hit.facts.preferred.0,
                    complete: usize::from(hit.facts.covered.0) == count,
                    together: hit.facts.together.0,
                    words: count as u8,
                    in_title: hit.facts.in_title.0,
                    habit: hit.facts.habit.0,
                    title_ranges,
                    line,
                }
            })
            .collect()
    }

    /// The entry that holds a song's title. Entries are in song order, title first.
    fn title_entry(&self, song: u32) -> u32 {
        self.entries.partition_point(|entry| entry.song < song) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> Vec<Song> {
        let song = |number: u32, title: &str, lyrics: &str| Song {
            number,
            title: title.into(),
            lyrics: lyrics.into(),
            ..Song::default()
        };
        vec![
            song(1, "Morning on the Hill", "[1]\nFirst light breaks over the valley\nWe wake and lift our voices\n[chorus]\nSing it out across the water\nEvery heart together now\n"),
            song(2, "River of Mercy", "[1]\nDown by the river we gather\nMercy flowing like the rain\n[chorus]\nLet the river carry us home\n"),
            song(3, "Hill of Morning Stars", "[1]\nNight is over, stars are fading\n"),
            song(47, "Quiet Harbour", "[1]\nStill waters hold the evening\n[bridge]\nAnchor deep, anchor sure\n"),
        ]
    }

    #[test]
    fn a_line_in_any_language_of_a_song_finds_it_and_the_hit_shows_that_line() {
        let mut songs = library();
        songs.push(Song {
            number: 9,
            title: "Morning of Thanks".into(),
            lyrics: "[1]\nPagi datang kami di sini\n[chorus]\nKami bersyukur\n".into(),
            lang: "id".into(),
            translations: vec![
                crate::song::Translation { lang: "en".into(), text: "[1]\nThe morning is here\n[chorus]\nWe thank you gladly\n".into() },
                crate::song::Translation { lang: "zh-Hans".into(), text: "[chorus]\nwo men gan xie\n".into() },
            ],
            ..Song::default()
        });
        let index = SongIndex::build(&songs);
        let hit = index.search("thank gladly", 1).remove(0);
        assert_eq!(hit.song, 4);
        let line = hit.line.expect("the matching line is shown");
        assert_eq!((line.text.as_str(), line.lang.as_str(), line.tag), ("We thank you gladly", "en", Tag::new(crate::song::SectionKind::Chorus, 1)));
        // The line opens at the written section with the same tag, whichever language matched.
        assert_eq!(line.section, 1);
        let third = index.search("gan xie", 1).remove(0);
        assert_eq!((third.song, third.line.map(|line| line.lang)), (4, Some("zh-Hans".into())));
        // The first language still answers as before.
        assert_eq!(index.search("bersyukur", 1).remove(0).line.map(|line| line.lang), Some("id".into()));
    }

    #[test]
    fn title_prefix_wins() {
        let songs = library();
        let index = SongIndex::build(&songs);
        let hits = index.search("morn", 10);
        assert_eq!(hits[0].song, 0);
        assert_eq!(&songs[0].title[hits[0].title_ranges[0].clone()], "Morning");
    }

    #[test]
    fn typos_are_forgiven() {
        let index = SongIndex::build(&library());
        assert_eq!(index.search("rivr of mercey", 10)[0].song, 1);
        assert_eq!(index.search("quite harbor", 10)[0].song, 3);
    }

    #[test]
    fn a_lyric_line_is_found_and_highlighted() {
        let index = SongIndex::build(&library());
        let hits = index.search("anchor deep", 10);
        assert_eq!(hits[0].song, 3);
        let line = hits[0].line.as_ref().map(|line| (line.tag.label(), line.text.clone(), line.ranges.clone()));
        let (label, text, ranges) = line.unwrap_or_default();
        assert_eq!(label, "Bridge");
        assert_eq!(&text[ranges[0].clone()], "Anchor deep");
    }

    #[test]
    fn a_phrase_beats_scattered_words() {
        let index = SongIndex::build(&library());
        let hits = index.search("the river", 10);
        assert_eq!(hits[0].song, 1);
    }

    #[test]
    fn an_elided_word_is_found_by_either_spelling() {
        let mut songs = library();
        songs.push(Song { number: 9, title: "Pujian".into(), lyrics: "[1]\nS'gala puji bagi-Mu\nKar'na kasih-Mu besar\n".into(), ..Song::default() });
        let index = SongIndex::build(&songs);
        for query in ["segala puji", "sgala puji", "s'gala puji", "karena kasih"] {
            assert_eq!(index.search(query, 5)[0].song, 4, "{query}");
        }
    }

    #[test]
    fn a_song_in_the_plan_goes_first_when_it_holds_every_word() {
        let index = SongIndex::build(&library());
        // "morning" is in two titles; the later one is in the plan.
        assert_eq!(index.search("morning", 10)[0].song, 0);
        let hits = index.search_with("morning", 10, &|song| song == 2);
        assert_eq!((hits[0].song, hits[0].preferred), (2, true));
        // A planned song that holds one word of two does not pass a song that holds both.
        let hits = index.search_with("river mercy", 10, &|song| song == 0);
        assert_eq!(hits[0].song, 1);
    }

    #[test]
    fn the_title_goes_before_the_same_word_in_lyrics() {
        let mut songs = library();
        songs.push(Song { number: 8, title: "Harbour Lights".into(), lyrics: "[1]\nA quiet evening\n".into(), ..Song::default() });
        let index = SongIndex::build(&songs);
        let hits = index.search("quiet", 10);
        assert_eq!(hits[0].song, 3, "the song named Quiet Harbour");
        assert!(hits[0].in_title && !hits[1].in_title);
    }

    #[test]
    fn a_word_inside_a_longer_word_is_found_last() {
        let mut songs = library();
        songs.push(Song { number: 9, title: "Pujian".into(), lyrics: "[1]\nDiberkati selamanya\n".into(), ..Song::default() });
        songs.push(Song { number: 10, title: "Syukur".into(), lyrics: "[1]\nBerkat yang baru\n".into(), ..Song::default() });
        let index = SongIndex::build(&songs);
        let hits: Vec<usize> = index.search("berkat", 10).into_iter().map(|hit| hit.song).collect();
        assert_eq!(hits, [5, 4]);
    }

    #[test]
    fn a_habit_orders_songs_that_tie() {
        let mut songs = library();
        for (number, title) in [(20, "Kasih Besar"), (21, "Kasih Agung"), (22, "Kasih Setia")] {
            songs.push(Song { number, title: title.into(), lyrics: "[1]
Words made up for the test
".into(), ..Song::default() });
        }
        let index = SongIndex::build(&songs);
        // Three titles start with "ka"; nothing but library order separates them.
        let plain: Vec<usize> = index.search("ka", 10).iter().map(|hit| hit.song).collect();
        assert_eq!(plain, [4, 5, 6]);
        let hits = index.search_ranked("ka", 10, &|_| false, &|song| if song == 6 { 3 } else { 0 });
        assert_eq!(hits.iter().map(|hit| (hit.song, hit.habit)).collect::<Vec<_>>(), [(6, 3), (4, 0), (5, 0)]);
    }

    #[test]
    fn a_habit_never_beats_a_plan_song_or_a_typed_number() {
        let index = SongIndex::build(&library());
        let hits = index.search_ranked("morning", 10, &|song| song == 0, &|song| if song == 2 { 5 } else { 0 });
        assert_eq!(hits[0].song, 0, "the plan song stays first");
        let hits = index.search_ranked("47", 10, &|_| false, &|song| if song == 1 { 5 } else { 0 });
        assert_eq!(hits[0].song, 3, "the number stays first");
    }

    #[test]
    fn a_habit_never_beats_an_exact_title_or_a_closer_match() {
        let mut songs = library();
        songs.push(Song { number: 9, title: "Pujian".into(), lyrics: "[1]
Diberkati selamanya
".into(), ..Song::default() });
        songs.push(Song { number: 10, title: "Berkat".into(), lyrics: "[1]
Other words
".into(), ..Song::default() });
        let index = SongIndex::build(&songs);
        // The song named for the word goes first, whatever was picked for it before.
        let hits = index.search_ranked("berkat", 10, &|_| false, &|song| if song == 4 { 5 } else { 0 });
        assert_eq!(hits.iter().map(|hit| hit.song).collect::<Vec<_>>(), [5, 4]);
        // A whole title typed out puts that song first.
        let hits = index.search_ranked("quiet harbour", 10, &|_| false, &|song| if song == 0 { 5 } else { 0 });
        assert_eq!(hits[0].song, 3);
    }

    #[test]
    fn a_habit_comes_after_words_side_by_side() {
        let mut songs = library();
        // Both hold "river" and "home" in a line, one with the words side by side.
        songs.push(Song { number: 11, title: "Alpha".into(), lyrics: "[1]
River bends toward the home
".into(), ..Song::default() });
        songs.push(Song { number: 12, title: "Beta".into(), lyrics: "[1]
River home again
".into(), ..Song::default() });
        let index = SongIndex::build(&songs);
        let plain: Vec<usize> = index.search("river home", 3).iter().map(|hit| hit.song).collect();
        let loved: Vec<usize> = index.search_ranked("river home", 3, &|_| false, &|song| if song == plain[1] { 5 } else { 0 }).iter().map(|hit| hit.song).collect();
        assert_eq!(loved[0], plain[0], "a closer phrase still goes first");
    }

    #[test]
    fn a_number_finds_its_song() {
        let index = SongIndex::build(&library());
        assert_eq!(index.search("47", 10)[0].song, 3);
    }

    #[test]
    fn nothing_typed_finds_nothing() {
        let index = SongIndex::build(&library());
        assert!(index.search("  ", 10).is_empty());
        assert!(index.search("zzzzqq", 10).is_empty());
    }

    #[test]
    fn a_large_library_stays_fast() {
        let mut songs = Vec::new();
        for i in 0..3000u32 {
            let mut lyrics = String::new();
            for verse in 1..=4 {
                lyrics.push_str(&format!("[{verse}]\n"));
                for line in 0..4 {
                    lyrics.push_str(&format!("word{} filler{} common tune{} line{}\n", (i * 7 + line) % 911, i % 577, verse, line));
                }
            }
            songs.push(Song { number: i, title: format!("Title {i} song{}", i % 97), lyrics, ..Song::default() });
        }
        let index = SongIndex::build(&songs);
        let start = std::time::Instant::now();
        let hits = index.search("filler12 comon tune", 20);
        let elapsed = start.elapsed();
        assert!(!hits.is_empty());
        // Generous: debug builds on a slow CI box still pass, a quadratic scan would not.
        assert!(elapsed.as_millis() < 400, "search took {elapsed:?}");
    }
}
