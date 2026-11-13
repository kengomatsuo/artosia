//! Proposed section marks for lyrics that have none.
//!
//! A song's words are plain text with marker lines (`[1]`, `[chorus]`). Words pasted from a
//! song sheet or fetched from the web have no markers, so the whole song is one run of verses
//! and nothing can jump to the chorus. [`propose`] reads the text and says what every stanza
//! would be, and [`Proposal::changes`] turns that into the marker lines to write. Nothing is
//! written by this module: a person presses a button first.
//!
//! A stanza is a run of lines parted from the next by a blank line. The rules, in this order:
//!
//! 1. **A name the writer put in the text is read.** A line of its own such as `Reff`,
//!    `Bait 2`, `Chorus:`, `Bridge` or `[chorus]` marks the stanza under it, and so does a name
//!    before the first line (`Reff: words`). These are the words `heading` knows, which include
//!    the Indonesian ones a volunteer types (`reff`, `bait`, `ayat`, `jembatan`, `pembuka`,
//!    `penutup`). The proposal says so with [`Basis::Named`], and a name before the first line
//!    is the only one it rewrites.
//! 2. **A stanza that comes back word for word is the chorus.** Case, punctuation and spacing
//!    are ignored. When several stanzas repeat, the one sung most often is the chorus, the
//!    earliest on a tie. A stanza that repeats a stanza marked by hand takes that stanza's mark.
//! 3. **A short stanza that comes once late in the song is a bridge**, when the song has a
//!    chorus and at least four stanzas: it has fewer lines than the median of the stanzas sung once
//!    and starts in the last third. Only the first such stanza is a bridge.
//! 4. **The rest are verses, numbered in the order they come.** A verse that repeats an earlier
//!    verse keeps its number. A number a mark in the text already uses is skipped, so the
//!    proposal fits around it.
//!
//! A stanza that is already under a mark keeps it. After a mark, a stanza with none is one of
//! that mark's slides (that is how `parse_lyrics` reads it), unless fewer than half of the
//! stanzas are marked: then the text is mostly unmarked, and each unmarked stanza is proposed
//! for. A stanza before the first mark is always proposed for.
//!
//! What it cannot tell: a pre-chorus from a verse (neither repeats on its own), an intro or an
//! ending from a bridge or a verse, a chorus sung only once, a chorus whose second singing is
//! worded differently, and a repeated verse from a chorus when the verse is sung more often.
//! Those come out as verses, and the person picks the right kind for the stanza.

use std::collections::{HashMap, HashSet};

use crate::song::{SectionKind, Tag, heading};

/// Why a stanza has its tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    /// A marker line already in the text, such as `[chorus]`.
    Marked,
    /// A name the writer put in the text: `Reff`, `Bait 2`, `Chorus:`.
    Named,
    /// The stanza is sung again word for word.
    Repeat,
    /// A short stanza that comes once, late in the song.
    Bridge,
    /// The next verse in order.
    Order,
}

/// One stanza and the tag it has or would get.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stanza {
    pub tag: Tag,
    pub basis: Basis,
    /// Byte offset of the stanza's first line, which is its marker line when it has one.
    pub start: usize,
    /// Byte offset just after the stanza's last line, before its line break.
    pub end: usize,
    /// How many lyric lines it has.
    pub lines: usize,
    /// A written name before the first line, in bytes up to and including the colon and the
    /// spaces after it: the part the marker replaces.
    prefix: Option<usize>,
    key: String,
}

impl Stanza {
    /// The stanza has no marker line yet: writing the proposal adds one.
    pub fn is_pending(&self) -> bool {
        matches!(self.basis, Basis::Repeat | Basis::Bridge | Basis::Order) || self.prefix.is_some()
    }

    /// The stanza has no mark of any kind, so the person may choose one for it.
    pub fn is_unmarked(&self) -> bool {
        matches!(self.basis, Basis::Repeat | Basis::Bridge | Basis::Order)
    }
}

/// One edit of the text: replace `remove` bytes at `at` with `insert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub at: usize,
    pub remove: usize,
    pub insert: String,
}

/// What every stanza of a text would be.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Proposal {
    pub stanzas: Vec<Stanza>,
}

struct Raw {
    start: usize,
    end: usize,
    mark: Option<(Tag, Basis)>,
    prefix: Option<usize>,
    body: Vec<String>,
    /// Only a marker line so far; lyric lines after a blank line still belong to it.
    heading_only: bool,
}

/// `Reff: words` read as a name and the words after it.
fn name_before_words(line: &str) -> Option<(Tag, usize)> {
    let colon = line.find(':')?;
    let rest = line[colon + 1..].trim_start();
    if rest.is_empty() || line[..colon].trim().is_empty() {
        return None;
    }
    let tag = heading(&line[..colon])?;
    Some((tag, line.len() - rest.len()))
}

/// The words of a stanza as one lowercase run, so two singings compare equal.
fn key_of(body: &[String]) -> String {
    let mut words: Vec<String> = Vec::new();
    for line in body {
        for word in line.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()) {
            words.push(word.to_lowercase());
        }
    }
    words.join(" ")
}

fn split_stanzas(text: &str) -> Vec<Raw> {
    let mut raws: Vec<Raw> = Vec::new();
    let mut gap = false;
    let mut at = 0;
    for full in text.split_inclusive('\n') {
        let start = at;
        at += full.len();
        let line = full.trim_end_matches(['\n', '\r']);
        let end = start + line.len();
        if line.trim().is_empty() {
            gap = true;
            continue;
        }
        if let Some(tag) = heading(line) {
            let basis = if line.trim_start().starts_with(['[', '(']) { Basis::Marked } else { Basis::Named };
            raws.push(Raw { start, end, mark: Some((tag, basis)), prefix: None, body: Vec::new(), heading_only: true });
            gap = false;
            continue;
        }
        let continues = match raws.last() {
            Some(last) => !gap || last.heading_only,
            None => false,
        };
        if let (true, Some(last)) = (continues, raws.last_mut()) {
            last.body.push(line.trim().to_string());
            last.end = end;
            last.heading_only = false;
        } else {
            let mut raw = Raw { start, end, mark: None, prefix: None, body: vec![line.trim().to_string()], heading_only: false };
            if let Some((tag, prefix)) = name_before_words(line) {
                raw.mark = Some((tag, Basis::Named));
                raw.prefix = Some(prefix);
                raw.body = vec![line[prefix..].trim().to_string()];
            }
            raws.push(raw);
        }
        gap = false;
    }
    raws.retain(|raw| !raw.body.is_empty());
    raws
}

/// The next verse number after `previous` that no mark in the text uses.
fn next_verse(previous: u8, taken: &HashSet<u8>) -> u8 {
    let mut number = previous.saturating_add(1);
    while taken.contains(&number) && number < u8::MAX {
        number += 1;
    }
    number
}

/// Reads `text` and says what each stanza has or would get. See the module's rules.
pub fn propose(text: &str) -> Proposal {
    let mut raws = split_stanzas(text);
    // After a mark, an unmarked stanza is one of its slides, unless the text is mostly unmarked.
    let marked = raws.iter().filter(|raw| raw.mark.is_some()).count();
    if marked > 0 && marked * 2 >= raws.len() {
        let mut seen_mark = false;
        raws.retain(|raw| {
            seen_mark |= raw.mark.is_some();
            raw.mark.is_some() || !seen_mark
        });
    }
    let keys: Vec<String> = raws.iter().map(|raw| key_of(&raw.body)).collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for key in &keys {
        *counts.entry(key.as_str()).or_default() += 1;
    }
    // Tags the writer chose, by the words they stand over.
    let mut known: HashMap<String, Tag> = HashMap::new();
    let mut taken: HashSet<u8> = HashSet::new();
    let mut taken_bridges = 0u8;
    for (raw, key) in raws.iter().zip(&keys) {
        if let Some((tag, _)) = raw.mark {
            known.entry(key.clone()).or_insert(tag);
            if tag.kind == SectionKind::Verse {
                taken.insert(tag.number);
            }
            if tag.kind == SectionKind::Bridge {
                taken_bridges = taken_bridges.max(tag.number);
            }
        }
    }
    let has_chorus_mark = raws.iter().any(|raw| raw.mark.is_some_and(|(tag, _)| tag.kind == SectionKind::Chorus));
    // The most repeated unmarked stanza is the chorus when the writer named none.
    if !has_chorus_mark {
        let mut best: Option<(&str, usize)> = None;
        for (raw, key) in raws.iter().zip(&keys) {
            let count = counts[key.as_str()];
            if raw.mark.is_none() && count >= 2 && best.is_none_or(|(_, most)| count > most) {
                best = Some((key.as_str(), count));
            }
        }
        if let Some((key, _)) = best {
            known.insert(key.to_string(), Tag::new(SectionKind::Chorus, 1));
        }
    }
    let has_chorus = known.values().any(|tag| tag.kind == SectionKind::Chorus);
    // The first short stanza late in the song that is sung once is the bridge.
    let total = raws.len();
    let mut bridge_at: Option<usize> = None;
    if has_chorus && total >= 4 {
        let mut sizes: Vec<usize> = raws.iter().zip(&keys).filter(|(_, key)| counts[key.as_str()] == 1).map(|(raw, _)| raw.body.len()).collect();
        sizes.sort_unstable();
        let median = sizes.get(sizes.len() / 2).copied().unwrap_or(0);
        bridge_at = raws.iter().enumerate().position(|(at, raw)| {
            raw.mark.is_none() && counts[keys[at].as_str()] == 1 && at * 3 >= 2 * (total - 1) && raw.body.len() < median
        });
    }
    let mut stanzas = Vec::with_capacity(raws.len());
    let mut verse_of: HashMap<&str, u8> = HashMap::new();
    let mut previous_verse = 0u8;
    for (at, raw) in raws.iter().enumerate() {
        let key = keys[at].as_str();
        let (tag, basis) = match raw.mark {
            Some((tag, basis)) => {
                if tag.kind == SectionKind::Verse {
                    previous_verse = tag.number;
                }
                (tag, basis)
            }
            None => match known.get(key) {
                Some(tag) => (*tag, Basis::Repeat),
                None if bridge_at == Some(at) => (Tag::new(SectionKind::Bridge, taken_bridges + 1), Basis::Bridge),
                None => match verse_of.get(key) {
                    Some(number) => (Tag::new(SectionKind::Verse, *number), Basis::Repeat),
                    None => {
                        let number = next_verse(previous_verse, &taken);
                        previous_verse = number;
                        verse_of.insert(key, number);
                        (Tag::new(SectionKind::Verse, number), Basis::Order)
                    }
                },
            },
        };
        stanzas.push(Stanza { tag, basis, start: raw.start, end: raw.end, lines: raw.body.len(), prefix: raw.prefix, key: keys[at].clone() });
    }
    Proposal { stanzas }
}

impl Proposal {
    /// How many stanzas still need a marker line.
    pub fn pending(&self) -> usize {
        self.stanzas.iter().filter(|stanza| stanza.is_pending()).count()
    }

    /// The marker lines to write: one before each stanza that has none.
    pub fn changes(&self) -> Vec<Change> {
        self.stanzas.iter().filter(|stanza| stanza.is_pending()).map(change_of).collect()
    }

    /// The stanza the caret is in, for a caret at `byte`.
    pub fn stanza_at(&self, byte: usize) -> Option<usize> {
        self.stanzas.iter().position(|stanza| stanza.start <= byte && byte <= stanza.end)
    }

    /// The tag a stanza gets when a person picks `kind` for it. A verse takes the next number
    /// after the verse before it; the other kinds take the next number of their kind, and a
    /// stanza worded like one already marked that way takes its tag.
    pub fn tag_as(&self, stanza: usize, kind: SectionKind) -> Tag {
        let Some(this) = self.stanzas.get(stanza) else { return Tag::new(kind, 1) };
        let others = || self.stanzas.iter().enumerate().filter(move |(at, _)| *at != stanza).map(|(_, other)| other);
        if let Some(same) = others().find(|other| other.tag.kind == kind && other.key == this.key) {
            return same.tag;
        }
        if kind == SectionKind::Verse {
            let previous = self.stanzas[..stanza].iter().rev().find(|before| before.tag.kind == SectionKind::Verse).map_or(0, |before| before.tag.number);
            let taken: HashSet<u8> = others().filter(|other| !other.is_unmarked() && other.tag.kind == SectionKind::Verse).map(|other| other.tag.number).collect();
            return Tag::new(SectionKind::Verse, next_verse(previous, &taken));
        }
        let most = others().filter(|other| other.tag.kind == kind).map(|other| other.tag.number).max().unwrap_or(0);
        Tag::new(kind, most + 1)
    }

    /// The edit that gives one stanza `tag`.
    pub fn change_for(&self, stanza: usize, tag: Tag) -> Option<Change> {
        let stanza = self.stanzas.get(stanza)?;
        Some(Change { at: stanza.start, remove: stanza.prefix.unwrap_or(0), insert: format!("{}\n", tag.marker()) })
    }
}

fn change_of(stanza: &Stanza) -> Change {
    Change { at: stanza.start, remove: stanza.prefix.unwrap_or(0), insert: format!("{}\n", stanza.tag.marker()) }
}

/// `text` with the changes made. Changes are in text order and do not overlap.
pub fn apply(text: &str, changes: &[Change]) -> String {
    let mut out = String::with_capacity(text.len() + changes.len() * 10);
    let mut at = 0;
    for change in changes {
        let from = change.at.max(at).min(text.len());
        out.push_str(&text[at..from]);
        out.push_str(&change.insert);
        at = (from + change.remove).min(text.len());
    }
    out.push_str(&text[at..]);
    out
}

/// Where a caret at `byte` of the old text lands in the text `apply` makes, so the caret stays
/// before the same letter.
pub fn caret_after(changes: &[Change], byte: usize) -> usize {
    let mut shifted = byte as isize;
    for change in changes {
        if change.at + change.remove <= byte {
            shifted += change.insert.len() as isize - change.remove as isize;
        } else if change.at < byte {
            // Inside the replaced name: land after what replaced it.
            shifted += change.insert.len() as isize - (byte - change.at) as isize;
        }
    }
    shifted.max(0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::parse_lyrics;

    fn tags(proposal: &Proposal) -> Vec<String> {
        proposal.stanzas.iter().map(|stanza| stanza.tag.short()).collect()
    }

    fn marked(text: &str) -> String {
        let proposal = propose(text);
        apply(text, &proposal.changes())
    }

    fn labels(text: &str) -> Vec<String> {
        parse_lyrics(text).iter().map(|section| section.tag.label()).collect()
    }

    const THREE_VERSES: &str = "Lamps are lit along the shore\nBoats come home at the end of day\n\nHold the light up high\nSung the same each time\n\nSecond verse is new\nNothing here repeats\n\nHold the light up high\nSung the same each time\n\nThird verse is new\nStill nothing repeats\n\nhold the LIGHT up high!\nsung the same each time\n";

    #[test]
    fn a_chorus_sung_three_times_is_found() {
        let proposal = propose(THREE_VERSES);
        assert_eq!(tags(&proposal), ["V1", "C", "V2", "C", "V3", "C"]);
        assert_eq!(proposal.stanzas[1].basis, Basis::Repeat);
        assert_eq!(proposal.stanzas[0].basis, Basis::Order);
        assert_eq!(proposal.pending(), 6);
        // Writing it makes the song a deck with a chorus to jump to.
        assert_eq!(labels(&marked(THREE_VERSES)), ["Verse 1", "Chorus", "Verse 2", "Chorus", "Verse 3", "Chorus"]);
    }

    #[test]
    fn names_the_writer_put_in_are_read() {
        let text = "Opening lines of the first part\nGoing on a little\n\nBait 2\nSecond part words\nMore of them\n\nReff\nAll together now\nOnce more together\n";
        let proposal = propose(text);
        assert_eq!(tags(&proposal), ["V1", "V2", "C"]);
        assert_eq!(proposal.stanzas[1].basis, Basis::Named);
        assert_eq!(proposal.stanzas[2].basis, Basis::Named);
        // The leading stanza fits around the written Bait 2.
        assert_eq!(proposal.stanzas[0].tag.number, 1);
        assert_eq!(proposal.pending(), 1);
        assert_eq!(labels(&marked(text)), ["Verse 1", "Verse 2", "Chorus"]);
    }

    #[test]
    fn a_name_before_the_first_line_is_read_and_rewritten() {
        let text = "Reff: All together now\nOnce more together\n\nBait 1\nA verse line\nAnother";
        let proposal = propose(text);
        assert_eq!(tags(&proposal), ["C", "V1"]);
        assert_eq!(proposal.pending(), 1);
        assert_eq!(marked(text), "[chorus]\nAll together now\nOnce more together\n\nBait 1\nA verse line\nAnother");
    }

    #[test]
    fn indonesian_names_are_read() {
        let text = "Pembuka\nLa la la\n\nBait 1\nSatu dua\n\nJembatan\nTiga empat\n\nPenutup\nLima enam\n";
        let proposal = propose(text);
        assert_eq!(tags(&proposal), ["I", "V1", "B", "E"]);
        assert_eq!(proposal.pending(), 0);
    }

    #[test]
    fn a_song_already_fully_marked_proposes_nothing() {
        let text = "[1]\none\ntwo\n\nthree\n[chorus]\nfour\nfive\n\n[2]\nsix\n";
        let proposal = propose(text);
        assert_eq!(proposal.pending(), 0);
        assert!(proposal.changes().is_empty());
        assert_eq!(tags(&proposal), ["V1", "C", "V2"]);
        // A slide break after a mark is a slide of that mark, not a stanza to mark.
        assert_eq!(proposal.stanzas.len(), 3);
    }

    #[test]
    fn a_stanza_marked_by_hand_is_respected() {
        let text = "Alpha words\nAlpha again\n\n[chorus]\nBeta words\nBeta again\n\nGamma words\nGamma again\n\nDelta words\nDelta again\n";
        let proposal = propose(text);
        assert_eq!(tags(&proposal), ["V1", "C", "V2", "V3"]);
        assert_eq!(proposal.stanzas[1].basis, Basis::Marked);
        assert_eq!(proposal.pending(), 3);
        assert_eq!(labels(&marked(text)), ["Verse 1", "Chorus", "Verse 2", "Verse 3"]);
    }

    #[test]
    fn a_verse_number_the_text_uses_is_skipped() {
        let text = "Alpha words\nAlpha again\n\n[2]\nBeta words\nBeta again\n\nGamma words\nGamma again\n\nDelta words\nDelta again\n\nEpsilon\nZeta\n";
        assert_eq!(tags(&propose(text)), ["V1", "V2", "V3", "V4", "V5"]);
        let proposal = propose(text);
        assert_eq!(proposal.stanzas.iter().filter(|stanza| stanza.tag.number == 2).count(), 1);
    }

    #[test]
    fn two_stanzas_without_a_repeat_are_two_verses() {
        let proposal = propose("Only the first\nof two\n\nOnly the second\nof two\n");
        assert_eq!(tags(&proposal), ["V1", "V2"]);
        assert!(proposal.stanzas.iter().all(|stanza| stanza.basis == Basis::Order));
    }

    #[test]
    fn a_short_stanza_once_late_is_a_bridge() {
        let text = "First part line a\nFirst part line b\nFirst part line c\nFirst part line d\n\nSing the refrain\nSing it again\n\nSecond part line a\nSecond part line b\nSecond part line c\nSecond part line d\n\nSing the refrain\nSing it again\n\nA turn near the end\nLeading back\n";
        let proposal = propose(text);
        assert_eq!(tags(&proposal), ["V1", "C", "V2", "C", "B"]);
        assert_eq!(proposal.stanzas[4].basis, Basis::Bridge);
        // Without a chorus there is nothing for a bridge to lead back to.
        let plain = propose("a b c d\ne f g h\n\ni j k l\nm n o p\n\nq r s t\nu v w x\n\nshort\nline\n");
        assert!(plain.stanzas.iter().all(|stanza| stanza.tag.kind == SectionKind::Verse));
    }

    #[test]
    fn writing_a_proposal_leaves_the_caret_before_the_same_letter() {
        let text = "Alpha words\nAlpha again\n\nBeta words\nBeta again\n";
        let proposal = propose(text);
        let changes = proposal.changes();
        let written = apply(text, &changes);
        assert_eq!(written, "[1]\nAlpha words\nAlpha again\n\n[2]\nBeta words\nBeta again\n");
        let caret = text.find("Beta").unwrap() + 2;
        let moved = caret_after(&changes, caret);
        assert_eq!(&written[moved..moved + 2], "ta");
        // A caret at the very start of a stanza stays before its first letter, after the new marker.
        let start = text.find("Alpha").unwrap();
        assert!(written[caret_after(&changes, start)..].starts_with("Alpha"));
    }

    #[test]
    fn a_caret_stanza_can_be_given_any_kind() {
        let text = "Alpha words\nAlpha again\n\nBeta words\nBeta again\n\nGamma words\nGamma again\n";
        let proposal = propose(text);
        let at = proposal.stanza_at(text.find("Beta").unwrap() + 1).unwrap();
        assert_eq!(at, 1);
        assert_eq!(proposal.tag_as(at, SectionKind::Verse), Tag::new(SectionKind::Verse, 2));
        assert_eq!(proposal.tag_as(at, SectionKind::Chorus), Tag::new(SectionKind::Chorus, 1));
        assert_eq!(proposal.tag_as(at, SectionKind::Bridge), Tag::new(SectionKind::Bridge, 1));
        let change = proposal.change_for(at, Tag::new(SectionKind::Chorus, 1)).unwrap();
        let written = apply(text, &[change]);
        // The parser reads the stanza after a mark as that mark's second slide.
        assert_eq!(labels(&written), ["Verse 1", "Chorus"]);
        // A second chorus that is worded differently is Chorus 2.
        let again = propose(&written);
        let third = again.stanza_at(written.find("Gamma").unwrap()).unwrap();
        assert_eq!(again.tag_as(third, SectionKind::Chorus), Tag::new(SectionKind::Chorus, 2));
        assert_eq!(again.stanza_at(written.find("[chorus]").unwrap()), Some(1));
    }

    #[test]
    fn a_caret_between_stanzas_is_in_none() {
        let text = "Alpha\nAlpha again\n\nBeta\nBeta again\n";
        let proposal = propose(text);
        assert_eq!(proposal.stanza_at(text.find("\n\n").unwrap() + 1), None);
    }

    #[test]
    fn crlf_text_is_read_like_lf() {
        let lf = "Alpha words\nAlpha again\n\nBeta words\nBeta again\n";
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(tags(&propose(&crlf)), tags(&propose(lf)));
    }
}
