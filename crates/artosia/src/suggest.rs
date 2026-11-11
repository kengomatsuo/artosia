//! What Artosia offers to show next, in one place. Three moments of panic in a service ask for
//! it: a song joins another without anyone telling the operator, a priest starts a song after
//! the sermon, a volunteer preparing wants to know what usually comes next.
//!
//! The offers, in the order of trust (`suggestions`):
//! 1. What the listener heard, when it is not what is live: another song, with the section
//!    where the heard words are, a Bible passage, or a far part of the live song.
//! 2. One quiet line that the words being sung are not on the screen, when the listener has
//!    proved it can read this room and then twice in a row finds nothing.
//! 3. What usually follows the live song in this church.
//!
//! An offer only ever goes into Preview (`App::take_offer`). A misheard word never
//! reaches the projector. The strip that draws them is `ui/suggest.rs`.

use artosia_core::bible::Reference;
use artosia_core::deck::DeckSource;
use artosia_core::text::words as split_words;
use artosia_core::song::Tag;

use crate::palette::Scope;
use crate::state::{App, Sheet};

/// Most offers at once.
pub const MOST: usize = 3;
/// A heard phrase is offered for this many seconds, then it is stale.
const FRESH: f64 = 60.0;
/// The "not on the screen" line needs a phrase this recent, so it goes when the singing stops.
const LOST_FRESH: f64 = 30.0;
/// Phrases of fewer words say too little to match or to miss.
const MIN_WORDS: usize = 4;
/// A heard phrase is read in chunks of this many words, each moving on by `STEP`.
const CHUNK: usize = 6;
const STEP: usize = 3;
/// A chunk of a phrase points at a song only when it has this many words and every one of them
/// sits in one line. Three words of four did so for five songs in a hundred seconds of a real
/// sermon.
const CHUNK_LEAST: usize = 5;
/// Misses in a row before the "not on the screen" line shows.
const MISSES: usize = 2;
/// The listener must have matched something this many seconds before the first miss, which
/// shows that it can read this room's singing at all.
const PROOF: f64 = 240.0;
/// The misses must all lie within this many seconds.
const MISS_SPAN: f64 = 90.0;
/// Slides away from the live one before a section of the live song counts as "far".
const FAR: usize = 2;
const KEEP: usize = 12;
/// Singing rarely reads well enough to match a line outright: on real recordings the model
/// heard five to nine words in twelve seconds, and two to four of them sat together in a line
/// of the right song. So the three nearest songs of every phrase are kept, and a song that
/// keeps coming back within this many seconds is offered.
const SUNG_SPAN: f64 = 75.0;
/// Words together in its lines, summed over the phrases, before such a song is offered.
const SUNG_WORDS: u32 = 5;
/// Words together in one phrase before that phrase counts for a song at all: one word is in
/// every song.
const SUNG_LEAST: u32 = 2;
/// And it must be among the nearest songs of one phrase in this many: preaching makes three
/// times the phrases singing does, and two of six landing near one song was chance in a real
/// sermon.
const SUNG_SHARE: u32 = 2;

/// Where a heard phrase was found: a song, and the line that matched best.
#[derive(Clone, Debug, PartialEq)]
struct Match {
    song: usize,
    /// The section of the matching line, as the song writes its sections; None for a title.
    section: Option<usize>,
    tag: Option<Tag>,
    chunks: u32,
    together: u32,
}

#[derive(Clone, Debug)]
struct Final {
    at: f64,
    words: usize,
    songs: Vec<Match>,
    /// The nearest songs for the whole phrase, however loosely: what `sung` counts.
    near: Vec<Match>,
    passage: Option<(Reference, String)>,
}

impl Final {
    fn matched(&self) -> bool {
        !self.songs.is_empty() || self.passage.is_some()
    }
    /// Long enough to say something, matched or not.
    fn informative(&self) -> bool {
        self.words >= MIN_WORDS || self.passage.is_some()
    }
}

/// What the listener's phrases matched lately.
#[derive(Default)]
pub struct Hearing {
    finals: Vec<Final>,
    /// A person acted on a suggestion at this moment; misses before it are not counted again.
    quiet_at: f64,
    /// The window's own clock check: which of the two ages had passed at the last poll.
    aged: (bool, bool),
    /// A scene or a test lets learning be suggested although a practice run records nothing.
    pub demo: bool,
}

impl Hearing {
    /// The listener stopped: what it heard goes with it.
    pub fn clear(&mut self) {
        self.finals.clear();
        self.aged = (false, false);
    }
}

/// Each word once, in the order first said.
fn distinct(words: &[String]) -> Vec<String> {
    let mut once: Vec<String> = Vec::new();
    for word in words {
        if !once.contains(word) {
            once.push(word.clone());
        }
    }
    once
}

/// What an offer is, and so why it is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The listener heard it: "Heard just now".
    Heard,
    /// The listener heard a part of the live song far from the live slide.
    Here,
    /// The words being sung are nowhere in the library.
    Lost,
    /// It usually follows the live song.
    Follows,
}

/// What pressing the offer does.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A song into Preview at a section. With `slide` set the song is the live one and its
    /// ring goes to that slide.
    Song { at: usize, section: Option<usize>, slide: Option<usize> },
    Passage(Reference),
    /// Opens the search.
    Search,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion {
    pub why: Why,
    pub target: Target,
    /// A song's title or a passage's reference; empty for `Why::Lost`.
    pub title: String,
    /// The section the heard words are in.
    pub tag: Option<Tag>,
}

impl App {
    /// Reads a phrase the listener finished and keeps what it matched.
    pub(crate) fn note_heard(&mut self, text: &str) {
        let said: Vec<String> = split_words(text).into_iter().map(|word| word.norm).collect();
        // Each word once, in order: singing repeats its words and the model loops on them, and
        // a word heard five times is still one word of evidence. A hymn's repeated line once
        // put eight "words together" in a song that was not being sung.
        let words = distinct(&said);
        let count = words.len();
        let (songs, near) = if count >= MIN_WORDS { (self.read_songs(&said, &words), self.near_songs(&words.join(" "))) } else { (Vec::new(), Vec::new()) };
        let passage = self.bible().and_then(|bible| bible.guess(text).first().map(|guess| (guess.reference, bible.format(&guess.reference))));
        let at = self.now;
        self.hearing.finals.push(Final { at, words: count, songs, near, passage });
        let extra = self.hearing.finals.len().saturating_sub(KEEP);
        self.hearing.finals.drain(..extra);
    }

    /// What Artosia makes of a phrase, for the recordings: the titles of its nearest songs with
    /// the words together in a line of each, the titles it names by itself, and a passage.
    pub(crate) fn attempt_of(&self, text: &str) -> (Vec<(String, u32)>, Vec<String>, Option<String>) {
        let said: Vec<String> = split_words(text).into_iter().map(|word| word.norm).collect();
        let words = distinct(&said);
        let title = |found: &Match| self.songs.get(found.song).map(|song| song.title.clone()).unwrap_or_default();
        let (near, named) = match words.len() >= MIN_WORDS {
            true => (self.near_songs(&words.join(" ")).iter().map(|found| (title(found), found.together)).collect(), self.read_songs(&said, &words).iter().map(title).collect()),
            false => (Vec::new(), Vec::new()),
        };
        (near, named, self.bible().and_then(|bible| bible.guess(text).first().map(|guess| bible.format(&guess.reference))))
    }

    /// The three songs nearest to the whole phrase, with how many of its words sit together in
    /// one line of each.
    fn near_songs(&self, phrase: &str) -> Vec<Match> {
        self.index
            .search(phrase, MOST)
            .into_iter()
            .filter(|hit| u32::from(hit.together) >= SUNG_LEAST)
            .map(|hit| {
                let (section, tag) = match &hit.line {
                    Some(line) => (Some(line.section), Some(line.tag)),
                    None => (None, None),
                };
                Match { song: hit.song, section, tag, chunks: 1, together: u32::from(hit.together) }
            })
            .collect()
    }

    /// The songs a phrase names by itself, best first. Each chunk of what was said is searched
    /// alone, because a sung phrase runs over several lines and a library of thousands of lines
    /// holds every common word somewhere. One whole chunk is not enough: a sentence of a sermon
    /// held a line that three songs share, and all three were offered. A song is named when two
    /// chunks sit whole in its lines, or one does and the song holds every word of the phrase.
    fn read_songs(&self, said: &[String], words: &[String]) -> Vec<Match> {
        let count = said.len();
        let whole: Vec<usize> = self.index.search(&words.join(" "), 8).into_iter().filter(|hit| hit.complete).map(|hit| hit.song).collect();
        let mut found: Vec<Match> = Vec::new();
        let mut start = 0;
        loop {
            let end = (start + CHUNK).min(count);
            // Six words, and when those are not whole in a line, the first five: a line of five
            // words is followed by the first word of the next.
            let mut hits = Vec::new();
            for reach in [end, (start + CHUNK_LEAST).min(count)] {
                let chunk = distinct(&said[start..reach]);
                if chunk.len() < CHUNK_LEAST {
                    continue;
                }
                hits = self.index.search(&chunk.join(" "), 2);
                hits.retain(|hit| usize::from(hit.together) >= chunk.len());
                if !hits.is_empty() {
                    break;
                }
            }
            for hit in hits {
                let together = u32::from(hit.together);
                let (section, tag) = match &hit.line {
                    Some(line) => (Some(line.section), Some(line.tag)),
                    None => (None, None),
                };
                match found.iter_mut().find(|known| known.song == hit.song) {
                    Some(known) => {
                        known.chunks += 1;
                        if together > known.together && section.is_some() {
                            (known.section, known.tag, known.together) = (section, tag, together);
                        }
                    }
                    None => found.push(Match { song: hit.song, section, tag, chunks: 1, together }),
                }
            }
            if end == count {
                break;
            }
            start += STEP;
        }
        found.retain(|known| known.chunks >= 2 || whole.contains(&known.song));
        found.sort_by(|a, b| b.chunks.cmp(&a.chunks).then(b.together.cmp(&a.together)));
        found.truncate(MOST);
        found
    }

    /// Poll: true when a heard phrase has just grown too old to offer, so the strip is redrawn.
    pub(crate) fn hearing_aged(&mut self) -> bool {
        let age = |limit: f64| self.hearing.finals.iter().any(|heard| self.now - heard.at <= limit);
        let now = (age(FRESH), age(LOST_FRESH));
        let changed = now != self.hearing.aged;
        self.hearing.aged = now;
        changed
    }

    /// Puts an offer in the Preview pane. The projector is never touched.
    pub fn take_offer(&mut self, offer: &Suggestion) {
        self.recording_taken(&format!("{:?}", offer.why), &offer.title);
        match &offer.target {
            Target::Song { at, slide: Some(slide), .. } => {
                // The live song, with the ring on the part the singing has reached.
                if self.songs.get(*at).is_some() {
                    self.open_live();
                }
                if let Some(open) = self.open.as_mut() {
                    open.cursor = (*slide).min(open.deck.slides.len().saturating_sub(1));
                    open.armed = true;
                    open.queued = false;
                }
            }
            Target::Song { at, section, slide: None } => self.open_song(*at, *section),
            Target::Passage(reference) => self.open_passage(reference),
            Target::Search => self.open_palette(Scope::All, ""),
        }
        self.hearing.quiet_at = self.now;
        if offer.target != Target::Search {
            self.sheet = Sheet::None;
        }
    }

    /// The song on the projector, by its place in the library.
    fn live_song(&self) -> Option<usize> {
        match self.live.as_ref().map(|live| &live.deck.source) {
            Some(DeckSource::Song(id)) => self.song_position(*id),
            _ => None,
        }
    }

    /// The slide of the live deck a heard section lies at: the first one after the live slide
    /// with that tag, else the last one before it.
    fn slide_of_section(&self, tag: Tag) -> Option<usize> {
        let live = self.live.as_ref()?;
        let at: Vec<usize> = live.deck.slides.iter().enumerate().filter(|(_, slide)| slide.tag == Some(tag)).map(|(index, _)| index).collect();
        at.iter().copied().find(|index| *index > live.index).or_else(|| at.last().copied())
    }
}

/// What to offer now, at most three, in the order of trust.
pub fn suggestions(s: &App) -> Vec<Suggestion> {
    let mut out = heard(s);
    if out.is_empty() {
        out.extend(lost(s));
    }
    let room = MOST.saturating_sub(out.len());
    let offered = |offer: &Suggestion, out: &[Suggestion]| out.iter().any(|known| matches!((&known.target, &offer.target), (Target::Song { at: a, .. }, Target::Song { at: b, .. }) if a == b));
    let more: Vec<Suggestion> = follows(s).into_iter().filter(|offer| !offered(offer, &out)).take(room).collect();
    out.extend(more);
    out.truncate(MOST);
    out
}

/// The song that keeps coming back among the nearest songs of the last phrases: in two phrases
/// or more and half of them (`SUNG_SHARE`), with `SUNG_WORDS` words together in all, half as
/// many again as any other song, and its newest phrase still fresh. `chunks` is the phrases and
/// `together` the words.
fn sung(s: &App) -> Option<Match> {
    let mut tally: Vec<(Match, f64)> = Vec::new();
    let mut phrases = 0u32;
    for heard in s.hearing.finals.iter().filter(|heard| s.now - heard.at <= SUNG_SPAN) {
        phrases += u32::from(heard.informative());
        for near in &heard.near {
            match tally.iter_mut().find(|(known, _)| known.song == near.song) {
                Some((known, newest)) => {
                    known.chunks += 1;
                    known.together += near.together;
                    *newest = heard.at;
                    if near.section.is_some() {
                        (known.section, known.tag) = (near.section, near.tag);
                    }
                }
                None => tally.push((near.clone(), heard.at)),
            }
        }
    }
    tally.sort_by_key(|(found, _)| std::cmp::Reverse(found.together));
    let (best, newest) = tally.first()?;
    let next = tally.get(1).map_or(0, |(other, _)| other.together);
    (best.chunks >= 2 && best.chunks * SUNG_SHARE >= phrases && best.together >= SUNG_WORDS && best.together * 2 >= next * 3 && s.now - newest <= FRESH).then(|| best.clone())
}

/// What the newest phrase that matched something named, unless that is already in hand; else
/// the song that keeps coming back in what is sung.
fn heard(s: &App) -> Vec<Suggestion> {
    let Some(last) = s.hearing.finals.iter().rev().find(|heard| s.now - heard.at <= FRESH && heard.matched()) else { return sung_offer(s).into_iter().collect() };
    let mut out = Vec::new();
    let open = s.open.as_ref();
    if let Some((reference, title)) = &last.passage
        && !open.is_some_and(|open| matches!(&open.deck.source, DeckSource::Passage { reference: held, .. } if held == reference))
    {
        out.push(Suggestion { why: Why::Heard, target: Target::Passage(*reference), title: title.clone(), tag: None });
    }
    let live = s.live_song();
    for found in &last.songs {
        let Some(song) = s.songs.get(found.song) else { continue };
        if Some(found.song) == live {
            // The live song heard near the live slide says nothing. Far from it, the singing
            // has gone somewhere the operator may not have followed.
            let Some(tag) = found.tag else { continue };
            let Some(slide) = s.slide_of_section(tag) else { continue };
            let from = s.live.as_ref().map(|live| live.index).unwrap_or(0);
            let ringed = s.open_is_live() && open.is_some_and(|open| open.cursor == slide);
            if slide.abs_diff(from) < FAR || ringed {
                continue;
            }
            out.push(Suggestion { why: Why::Here, target: Target::Song { at: found.song, section: found.section, slide: Some(slide) }, title: song.title.clone(), tag: Some(tag) });
            continue;
        }
        // Already in Preview at that part: nothing left to bring.
        let there = open.is_some_and(|open| {
            matches!(open.deck.source, DeckSource::Song(id) if id == song.id)
                && found.tag.is_none_or(|tag| open.deck.slides.get(open.cursor).is_some_and(|slide| slide.tag == Some(tag)))
        });
        if !there {
            out.push(Suggestion { why: Why::Heard, target: Target::Song { at: found.song, section: found.section, slide: None }, title: song.title.clone(), tag: found.tag });
        }
    }
    out
}

/// The offer for `sung`: nothing when that song is the live one or already waits in Preview.
fn sung_offer(s: &App) -> Option<Suggestion> {
    let found = sung(s)?;
    let song = s.songs.get(found.song)?;
    if Some(found.song) == s.live_song() || s.open.as_ref().is_some_and(|open| matches!(open.deck.source, DeckSource::Song(id) if id == song.id)) {
        return None;
    }
    Some(Suggestion { why: Why::Heard, target: Target::Song { at: found.song, section: found.section, slide: None }, title: song.title.clone(), tag: found.tag })
}

/// The words being sung are on no screen and in no song. Said only when the listener has read
/// this room before (some phrase matched in the last four minutes) and then, twice in a row,
/// read phrases that match nothing at all, while a song is live.
fn lost(s: &App) -> Option<Suggestion> {
    s.live_song()?;
    let counted: Vec<&Final> = s.hearing.finals.iter().filter(|heard| heard.at > s.hearing.quiet_at && heard.informative()).collect();
    let tail = counted.get(counted.len().checked_sub(MISSES)?..)?;
    let newest = tail.last()?;
    if s.now - newest.at > LOST_FRESH || tail.iter().any(|heard| heard.matched()) || s.now - tail[0].at > MISS_SPAN {
        return None;
    }
    let first = tail[0].at;
    let proof = counted[..counted.len() - MISSES].iter().any(|heard| heard.matched() && first - heard.at <= PROOF);
    proof.then_some(Suggestion { why: Why::Lost, target: Target::Search, title: String::new(), tag: None })
}

/// What usually follows the live song, when Preview holds nothing a person chose and the plan
/// has nothing after the live item. Nothing in practice, where nothing is recorded.
fn follows(s: &App) -> Vec<Suggestion> {
    if !s.settings.learn || !(s.learn.recording || s.hearing.demo) {
        return Vec::new();
    }
    let Some(DeckSource::Song(live)) = s.live.as_ref().map(|live| live.deck.source.clone()) else { return Vec::new() };
    if s.open.as_ref().is_some_and(|open| !open.queued) || s.next_item_title().is_some() {
        return Vec::new();
    }
    let planned = s.planned_songs();
    let rows = s.store.followers_of(live, s.moment().at).unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| s.song_position(row.song))
        .filter(|at| !planned.contains(at) && Some(*at) != s.live_song())
        .map(|at| Suggestion { why: Why::Follows, target: Target::Song { at, section: None, slide: None }, title: s.songs[at].title.clone(), tag: None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use artosia_core::learn::Moment;

    const MORNING: &str = "Morning on the Hill";
    const RIVER: &str = "River of Mercy";

    /// A song's place in the library, which is ordered by title.
    fn at(app: &App, title: &str) -> usize {
        app.songs.iter().position(|song| song.title == title).expect("a practice song")
    }

    /// A practice app on a made-up clock, with the first song live at its first slide.
    fn app() -> App {
        let mut app = App::practice();
        app.now = 1_794_106_800.0;
        app.plan.items.clear();
        app
    }

    fn live(app: &mut App, title: &str, slide: usize) {
        let song = at(app, title);
        app.open_song(song, None);
        app.show(slide);
    }

    fn hear(app: &mut App, seconds_later: f64, text: &str) {
        app.now += seconds_later;
        app.heard_final(text.to_string());
    }

    const RIVER_WORDS: &str = "down by the river we gather mercy flowing like the rain";
    const BRIDGE_WORDS: &str = "we will not be moved we will not be moved you are with us still";
    const FIRST_VERSE_WORDS: &str = "first light breaks over the valley we wake and lift our voices";
    const GARBAGE: [&str; 3] = ["lantern carpet violin tractor window pencil", "marble ladder compass orchard blanket whistle", "pepper saddle anvil thimble copper feather"];

    #[test]
    fn a_phrase_of_another_song_is_offered_with_its_section() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, RIVER_WORDS);
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].why, Why::Heard);
        assert_eq!(found[0].title, "River of Mercy");
        assert!(matches!(found[0].target, Target::Song { section: Some(0), slide: None, .. }), "{:?}", found[0].target);
        assert_eq!(found[0].tag.map(|tag| tag.short()), Some("V1".to_string()));
        // Taking it brings it into Preview at that section and leaves the projector alone.
        let live_before = app.live.as_ref().map(|live| (live.deck.title.clone(), live.index));
        app.take_offer(&found[0]);
        assert_eq!(app.open.as_ref().map(|open| open.deck.title.clone()), Some("River of Mercy".to_string()));
        assert_eq!(app.live.as_ref().map(|live| (live.deck.title.clone(), live.index)), live_before);
        // Once it is in Preview there is nothing left to offer.
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn a_later_section_of_another_song_opens_at_that_section() {
        let mut app = app();
        live(&mut app, RIVER, 0);
        hear(&mut app, 5.0, "sing it out across the water every heart together now");
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].tag.map(|tag| tag.short()), Some("C".to_string()));
        app.take_offer(&found[0]);
        let open = app.open.as_ref().expect("a deck is in Preview");
        assert_eq!(open.deck.title, "Morning on the Hill");
        assert_eq!(open.deck.slides[open.cursor].short, "C");
    }

    #[test]
    fn the_live_song_heard_near_the_live_slide_offers_nothing() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, FIRST_VERSE_WORDS);
        assert!(suggestions(&app).is_empty(), "{:?}", suggestions(&app));
        // The next slide is the chorus: one Next away, so it is no news either.
        hear(&mut app, 5.0, "sing it out across the water every heart together now");
        assert!(suggestions(&app).is_empty(), "{:?}", suggestions(&app));
    }

    #[test]
    fn the_live_song_heard_far_from_the_live_slide_offers_that_part() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, BRIDGE_WORDS);
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].why, Why::Here);
        assert_eq!(found[0].tag.map(|tag| tag.short()), Some("B".to_string()));
        let Target::Song { slide: Some(slide), .. } = found[0].target else { panic!("{:?}", found[0].target) };
        assert!(slide >= 2);
        app.take_offer(&found[0]);
        let open = app.open.as_ref().expect("the live song is in Preview");
        assert_eq!(open.cursor, slide);
        assert!(app.preview_waits(), "Enter would show the heard part");
        assert_eq!(app.live.as_ref().map(|live| live.index), Some(0), "the projector did not move");
        assert!(suggestions(&app).is_empty(), "the ring already sits there");
    }

    /// What the recogniser makes of singing: a few right words of a line among wrong ones.
    const SUNG_BADLY: [&str; 3] = [
        "lantern down by the carpet violin tractor",
        "window we gather pencil marble ladder compass",
        "orchard mercy flowing blanket whistle pepper",
    ];

    #[test]
    fn a_song_that_keeps_coming_back_in_badly_heard_singing_is_offered() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, SUNG_BADLY[0]);
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard), "one phrase is not enough: {:?}", suggestions(&app));
        hear(&mut app, 12.0, SUNG_BADLY[1]);
        hear(&mut app, 12.0, SUNG_BADLY[2]);
        let found = suggestions(&app);
        assert_eq!(found.first().map(|offer| (offer.why, offer.title.as_str())), Some((Why::Heard, RIVER)), "{found:?}");
        // It goes with the singing: a minute after the last such phrase it is no longer offered.
        app.now += FRESH + 1.0;
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard));
    }

    #[test]
    fn a_sentence_that_only_holds_a_line_of_a_song_names_nothing_by_itself() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        // A preacher's sentence with a song's line inside it: the rest is in no song.
        hear(&mut app, 5.0, "down by the river we gather brothers yesterday umbrellas tomorrow");
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard), "{:?}", suggestions(&app));
    }

    #[test]
    fn a_word_sung_again_and_again_counts_once() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        // Looped words must not pile up into a match: "river" five times is one word.
        hear(&mut app, 5.0, "river river river river river lantern");
        hear(&mut app, 12.0, "river river river mercy mercy mercy carpet");
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard), "{:?}", suggestions(&app));
    }

    #[test]
    fn phrases_that_are_near_a_different_song_each_time_offer_nothing() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, SUNG_BADLY[0]);
        hear(&mut app, 12.0, "saddle bread for the road anvil thimble copper");
        hear(&mut app, 12.0, "feather night is over lantern carpet violin");
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard), "{:?}", suggestions(&app));
    }

    #[test]
    fn two_phrases_near_one_song_among_many_phrases_are_chance() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        // Preaching: a phrase every few seconds, two of eight near one song.
        hear(&mut app, 5.0, SUNG_BADLY[0]);
        for garbage in GARBAGE {
            hear(&mut app, 4.0, garbage);
        }
        hear(&mut app, 4.0, SUNG_BADLY[1]);
        for garbage in GARBAGE {
            hear(&mut app, 4.0, garbage);
        }
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Heard), "{:?}", suggestions(&app));
    }

    #[test]
    fn a_suggestion_goes_stale() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, RIVER_WORDS);
        assert_eq!(suggestions(&app).len(), 1);
        app.now += FRESH + 1.0;
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn words_that_match_nothing_need_repeated_misses_and_proof_the_listener_can_read() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        // Misses with no proof before them say nothing: the listener may simply not read this room.
        hear(&mut app, 5.0, GARBAGE[0]);
        hear(&mut app, 12.0, GARBAGE[1]);
        hear(&mut app, 12.0, GARBAGE[2]);
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Lost), "no proof of reading");
        // The room is read: the live song is matched.
        app.hearing.clear();
        hear(&mut app, 5.0, FIRST_VERSE_WORDS);
        hear(&mut app, 12.0, GARBAGE[0]);
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Lost), "one miss is not a pattern");
        hear(&mut app, 12.0, GARBAGE[1]);
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].why, Why::Lost);
        assert_eq!(found[0].target, Target::Search);
        // A phrase that matches something puts the line away again.
        hear(&mut app, 12.0, RIVER_WORDS);
        assert!(suggestions(&app).iter().all(|offer| offer.why != Why::Lost));
        // A heard song is offered first, and the warning waits until that offer is stale.
        app.now += FRESH;
        // It goes when the singing stops.
        hear(&mut app, 12.0, GARBAGE[0]);
        hear(&mut app, 12.0, GARBAGE[1]);
        assert!(suggestions(&app).iter().any(|offer| offer.why == Why::Lost));
        app.now += LOST_FRESH + 1.0;
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn short_phrases_neither_match_nor_miss() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, FIRST_VERSE_WORDS);
        for _ in 0..4 {
            hear(&mut app, 12.0, "amen amen");
        }
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn pressing_search_on_the_lost_line_quiets_it() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, FIRST_VERSE_WORDS);
        hear(&mut app, 12.0, GARBAGE[0]);
        hear(&mut app, 12.0, GARBAGE[1]);
        let found = suggestions(&app);
        app.take_offer(&found[0]);
        assert!(app.palette.open);
        app.palette.open = false;
        assert!(suggestions(&app).is_empty());
    }

    /// Song a is followed by song b on three Sundays, by c once.
    fn history(app: &mut App) {
        let (morning, river) = (app.songs[at(app, MORNING)].id, app.songs[at(app, RIVER)].id);
        for week in 1..=3i64 {
            let day = Moment::new(app.now as i64 - 7 * week * 86_400, 0);
            let _ = app.store.record_song_live(&day, Some(1), morning, "", "", true);
            let _ = app.store.record_song_live(&day, Some(1), river, "", "", true);
        }
    }

    #[test]
    fn followers_appear_only_when_preview_is_empty_or_holds_artosias_own_queue() {
        let mut app = app();
        app.learn.recording = true;
        history(&mut app);
        live(&mut app, MORNING, 0);
        // With the live song itself queued in Preview, and nothing after it in the plan.
        assert!(app.open.as_ref().is_some_and(|open| open.queued));
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!((found[0].why, found[0].title.as_str()), (Why::Follows, "River of Mercy"));
        // A person opens something: it is no longer Artosia's own queue.
        let other = at(&app, "Quiet Harbour");
        app.open_song(other, None);
        assert!(suggestions(&app).is_empty());
        // Back to nothing: Esc-style, the queue returns.
        app.open = None;
        assert_eq!(suggestions(&app).len(), 1);
        // The plan has an item after the live one: Preview waits with that, not with a guess.
        let (morning, other) = (at(&app, MORNING), at(&app, "Quiet Harbour"));
        app.add_song_to_plan(morning);
        app.add_song_to_plan(other);
        let first = app.plan.items.first().map(|item| item.id).unwrap_or(0);
        app.open_plan_item(first);
        app.show(0);
        assert!(suggestions(&app).is_empty(), "{:?}", suggestions(&app));
    }

    #[test]
    fn nothing_is_suggested_from_learning_in_practice() {
        let mut app = app();
        history(&mut app);
        live(&mut app, MORNING, 0);
        assert!(!app.learn.recording);
        assert!(suggestions(&app).is_empty());
        // A scene may ask for it.
        app.hearing.demo = true;
        assert_eq!(suggestions(&app).len(), 1);
        // And the switch still turns it off.
        app.settings.learn = false;
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn a_song_heard_is_not_offered_again_as_a_follower() {
        let mut app = app();
        app.learn.recording = true;
        history(&mut app);
        live(&mut app, MORNING, 0);
        app.hearing.finals.clear();
        hear(&mut app, 5.0, RIVER_WORDS);
        // Heard comes before what usually follows, and a song heard is not offered twice.
        let found = suggestions(&app);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].why, Why::Heard);
    }

    #[test]
    fn f4_takes_the_first_suggestion_of_any_kind() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, RIVER_WORDS);
        app.open_last_heard();
        assert_eq!(app.open.as_ref().map(|open| open.deck.title.clone()), Some("River of Mercy".to_string()));
        // With nothing to suggest it says so.
        let mut app = App::practice();
        app.open_last_heard();
        assert!(app.toast.is_some());
        // Followers count too.
        let mut app = app_with_followers();
        app.open_last_heard();
        assert_eq!(app.open.as_ref().map(|open| open.deck.title.clone()), Some("River of Mercy".to_string()));
        assert!(app.open.as_ref().is_some_and(|open| !open.queued));
    }

    fn app_with_followers() -> App {
        let mut app = app();
        app.learn.recording = true;
        history(&mut app);
        live(&mut app, MORNING, 0);
        app
    }

    #[test]
    fn the_listener_stopping_forgets_what_it_heard() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, RIVER_WORDS);
        app.hearing.clear();
        assert!(suggestions(&app).is_empty());
    }

    #[test]
    fn a_long_sung_phrase_is_read_to_its_end() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        // Twenty-five words over two songs: nothing past the twelfth word is dropped.
        hear(&mut app, 5.0, &format!("{RIVER_WORDS} {BRIDGE_WORDS}"));
        let found = suggestions(&app);
        let kinds: Vec<Why> = found.iter().map(|offer| offer.why).collect();
        assert!(kinds.contains(&Why::Heard) && kinds.contains(&Why::Here), "{found:?}");
    }

    #[test]
    fn aging_asks_for_a_redraw_once() {
        let mut app = app();
        live(&mut app, MORNING, 0);
        hear(&mut app, 5.0, RIVER_WORDS);
        assert!(app.hearing_aged());
        assert!(!app.hearing_aged());
        app.now += LOST_FRESH + 1.0;
        assert!(app.hearing_aged());
        app.now += FRESH;
        assert!(app.hearing_aged());
        assert!(!app.hearing_aged());
    }
}
