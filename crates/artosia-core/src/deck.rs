//! Decks: the slides of one thing that can be shown (a song, a passage, an announcement).

use serde::{Deserialize, Serialize};

use crate::bible::{Bible, Reference};
use crate::song::{SectionKind, Song, Tag};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlideKind {
    #[default]
    Lyrics,
    Scripture,
    Announcement,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slide {
    pub kind: SlideKind,
    /// "Verse 1", "Chorus", "John 3:16".
    pub label: String,
    /// "V1", "C", "16".
    pub short: String,
    /// The section colour; None for scripture and announcements.
    pub tag: Option<Tag>,
    /// The heading drawn above the text on announcement slides.
    pub heading: String,
    pub text: String,
    /// The same lines in a second language or translation; empty when there is none.
    pub second: String,
    /// Small print under the text: the reference and translation, or the song credit.
    pub caption: String,
    /// A picture shown with the words, or alone when there are none, as a path; empty for none.
    #[serde(default)]
    pub image: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckSource {
    Song(i64),
    Passage { reference: Reference, bible: String, second: Option<String> },
    Announcement(i64),
    Announcements,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deck {
    pub title: String,
    /// Author, translation name or nothing.
    pub subtitle: String,
    pub source: DeckSource,
    pub slides: Vec<Slide>,
}

/// What an announcement writes into its own body each time it is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fill {
    #[default]
    Nothing,
    /// The people whose birthday falls in the seven days from today.
    BirthdaysWeek,
    /// The people whose birthday falls in this month.
    BirthdaysMonth,
    /// The words of a Bible passage, read from the installed Bible each time the card is shown.
    Passage,
}

impl Fill {
    /// The word stored in the library.
    pub fn code(self) -> &'static str {
        match self {
            Fill::Nothing => "",
            Fill::BirthdaysWeek => "birthdays-week",
            Fill::BirthdaysMonth => "birthdays-month",
            Fill::Passage => "passage",
        }
    }

    pub fn from_code(code: &str) -> Fill {
        match code {
            "birthdays-week" => Fill::BirthdaysWeek,
            "birthdays-month" => Fill::BirthdaysMonth,
            "passage" => Fill::Passage,
            _ => Fill::Nothing,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Announcement {
    pub id: i64,
    pub title: String,
    /// Lines of words. A tab inside a line splits it into columns, as a table row.
    pub body: String,
    /// Seconds each announcement stays up in the loop.
    pub seconds: u32,
    /// A picture shown with the words, as a path; empty when there is none.
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub fill: Fill,
    /// The reference of a passage card as the volunteer typed it ("Mazmur 23:1-3"); read with
    /// the installed Bible's own parser. Empty on every other card.
    #[serde(default)]
    pub passage: String,
}

/// Someone whose birthday the church keeps. The year is never stored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    /// 1 to 31.
    pub day: u8,
    /// 1 to 12.
    pub month: u8,
}

/// The day an announcement is built on, with the month names to write dates in.
#[derive(Clone, Copy, Debug)]
pub struct Today<'a> {
    pub month: u8,
    pub day: u8,
    pub months: &'a [&'a str; 12],
}

/// The most names on one birthday slide; more go on a second slide.
pub const BIRTHDAYS_PER_SLIDE: usize = 8;
/// The most lines of an announcement on one slide.
pub const LINES_PER_SLIDE: usize = 10;

const MONTH_DAYS: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// The date `days` after a day of the year, wrapping over New Year.
fn days_after(month: u8, day: u8, days: u8) -> (u8, u8) {
    let (mut month, mut day) = (month.clamp(1, 12), day.max(1) + days);
    while day > MONTH_DAYS[usize::from(month) - 1] {
        day -= MONTH_DAYS[usize::from(month) - 1];
        month = month % 12 + 1;
    }
    (month, day)
}

/// The first and last day (month, day) a birthday card covers today; None for a card that
/// holds no birthdays.
pub fn birthday_days(fill: Fill, today: &Today) -> Option<((u8, u8), (u8, u8))> {
    match fill {
        Fill::BirthdaysWeek => Some(((today.month, today.day), days_after(today.month, today.day, 6))),
        Fill::BirthdaysMonth => Some(((today.month, 1), (today.month, MONTH_DAYS[usize::from(today.month.clamp(1, 12)) - 1]))),
        Fill::Nothing | Fill::Passage => None,
    }
}

/// What a passage card can show right now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PassageState {
    /// The card is not a passage card.
    NotAPassage,
    /// No Bible is loaded to read the words from.
    NoBible,
    /// The Bible has nothing at the reference typed: empty, an unknown book or a missing verse.
    Unreadable,
    Ready(Reference),
}

/// Reads a passage card's reference with the Bible's own parser (`Bible::guess`, which knows
/// the book names of its language and English). A bare book name is not enough for a card.
pub fn passage_state(item: &Announcement, bible: Option<&Bible>) -> PassageState {
    if item.fill != Fill::Passage {
        return PassageState::NotAPassage;
    }
    let Some(bible) = bible else { return PassageState::NoBible };
    match bible.guess(&item.passage).into_iter().next() {
        Some(guess) if !guess.book_only && !bible.verses(&guess.reference).is_empty() => PassageState::Ready(guess.reference),
        _ => PassageState::Unreadable,
    }
}

/// The people an announcement fills itself with, in date order from today.
pub fn birthdays<'a>(people: &'a [Person], fill: Fill, today: &Today) -> Vec<&'a Person> {
    let key = |month: u8, day: u8| u16::from(month) * 100 + u16::from(day);
    let from = key(today.month, today.day);
    let mut found: Vec<&Person> = match fill {
        Fill::Nothing | Fill::Passage => Vec::new(),
        Fill::BirthdaysMonth => people.iter().filter(|person| person.month == today.month).collect(),
        Fill::BirthdaysWeek => {
            let (month, day) = days_after(today.month, today.day, 6);
            let to = key(month, day);
            people
                .iter()
                .filter(|person| {
                    let at = key(person.month, person.day);
                    // A week that runs over New Year holds the end of December and the start of January.
                    if from <= to { at >= from && at <= to } else { at >= from || at <= to }
                })
                .collect()
        }
    };
    // Dates before today belong to next year when the week wraps.
    let order = |person: &&Person| {
        let at = key(person.month, person.day);
        (fill == Fill::BirthdaysWeek && at < from, at)
    };
    found.sort_by(|a, b| order(a).cmp(&order(b)).then_with(|| a.name.cmp(&b.name)));
    found
}

/// The credit line shown with a song's first slide.
pub fn song_credit(song: &Song, licence: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !song.author.trim().is_empty() {
        parts.push(song.author.trim().to_string());
    }
    if !song.copyright.trim().is_empty() {
        parts.push(song.copyright.trim().to_string());
    }
    if !licence.trim().is_empty() {
        parts.push(format!("CCLI {}", licence.trim()));
    }
    parts.join(", ")
}

/// A song's slides in sung order; sections longer than `max_lines` are split evenly.
pub fn song_deck(song: &Song, max_lines: usize, licence: &str) -> Deck {
    let second = crate::song::parse_lyrics(&song.second);
    let credit = song_credit(song, licence);
    let mut slides = Vec::new();
    for section in song.arranged() {
        let pieces = section.slides(max_lines);
        // The second language keeps its own slide breaks when it has as many as the first;
        // otherwise its lines are dealt evenly across the first language's slides.
        let other_pieces: Vec<Vec<String>> = match second.iter().find(|s| s.tag == section.tag) {
            Some(other) if other.slides(max_lines).len() == pieces.len() => other.slides(max_lines),
            Some(other) => crate::song::split_lines(&other.lines, other.lines.len().div_ceil(pieces.len()).max(1)),
            None => Vec::new(),
        };
        for (i, lines) in pieces.iter().enumerate() {
            slides.push(Slide {
                kind: SlideKind::Lyrics,
                label: section.tag.label(),
                short: section.tag.short(),
                tag: Some(section.tag),
                heading: String::new(),
                text: lines.join("\n"),
                second: other_pieces.get(i).map(|lines| lines.join("\n")).unwrap_or_default(),
                caption: if slides.is_empty() { credit.clone() } else { String::new() },
                image: String::new(),
            });
        }
    }
    Deck { title: song.title.clone(), subtitle: song.author.clone(), source: DeckSource::Song(song.id), slides }
}

/// A passage's slides, `per_slide` verses on each. With a second translation the same verses
/// are shown under the first.
pub fn passage_deck(bible: &Bible, reference: &Reference, second: Option<&Bible>, per_slide: usize) -> Deck {
    let verses = bible.verses(reference);
    let per_slide = per_slide.max(1);
    let mut slides = Vec::new();
    for group in verses.chunks(per_slide) {
        let (Some(first), Some(last)) = (group.first(), group.last()) else { continue };
        let span = Reference { book: first.book, chapter: first.chapter, verse: first.verse, end_chapter: last.chapter, end_verse: last.verse };
        let numbered = group.len() > 1;
        let join = |bible: &Bible, span: &Reference| -> String {
            bible
                .verses(span)
                .iter()
                .map(|verse| if numbered { format!("{} {}", verse.verse, verse.text) } else { verse.text.to_string() })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let label = span_label(bible, &span);
        let codes = match second {
            Some(other) => format!("{} / {}", bible.code, other.code),
            None => bible.code.clone(),
        };
        slides.push(Slide {
            kind: SlideKind::Scripture,
            label: label.clone(),
            short: if numbered { format!("{}-{}", first.verse, last.verse) } else { first.verse.to_string() },
            tag: None,
            heading: String::new(),
            text: join(bible, &span),
            second: second.map(|other| join(other, &span)).unwrap_or_default(),
            caption: format!("{label} {codes}"),
            image: String::new(),
        });
    }
    Deck {
        title: bible.format(reference),
        subtitle: bible.name.clone(),
        source: DeckSource::Passage { reference: *reference, bible: bible.code.clone(), second: second.map(|b| b.code.clone()) },
        slides,
    }
}

/// A reference for a few verses, never collapsed to its chapter: a slide says "Psalms 117:1-2".
fn span_label(bible: &Bible, span: &Reference) -> String {
    let name = bible.book(span.book).map(|book| book.name.as_str()).unwrap_or_else(|| crate::bible::english_name(span.book));
    if span.end_chapter != span.chapter {
        format!("{name} {}:{}-{}:{}", span.chapter, span.verse, span.end_chapter, span.end_verse)
    } else if span.end_verse != span.verse {
        format!("{name} {}:{}-{}", span.chapter, span.verse, span.end_verse)
    } else {
        format!("{name} {}:{}", span.chapter, span.verse)
    }
}

/// The slides of every announcement in order, each with the place of its announcement in the
/// list. One that fills itself with birthdays gives a slide per eight names, and none at all
/// when nobody has one. A passage card gives its verses, `per_slide` to a slide, and none at all
/// while its reference cannot be read from `bible`.
pub fn announcement_pages(announcements: &[Announcement], people: &[Person], today: &Today, bible: Option<&Bible>, per_slide: usize) -> Vec<(usize, Slide)> {
    let mut pages = Vec::new();
    for (at, item) in announcements.iter().enumerate() {
        if item.fill == Fill::Passage {
            if let (PassageState::Ready(reference), Some(bible)) = (passage_state(item, bible), bible) {
                for mut slide in passage_deck(bible, &reference, None, per_slide).slides {
                    slide.heading = item.title.clone();
                    slide.image = item.image.clone();
                    pages.push((at, slide));
                }
            }
            continue;
        }
        if item.fill == Fill::Nothing {
            // A long list, such as a table brought in from PowerPoint, goes over several slides
            // so the words stay large enough to read from the back.
            let lines: Vec<&str> = item.body.lines().collect();
            if lines.len() <= LINES_PER_SLIDE {
                pages.push((at, announcement_slide(item)));
            } else {
                for group in lines.chunks(LINES_PER_SLIDE) {
                    pages.push((at, announcement_slide(&Announcement { body: group.join("
"), ..item.clone() })));
                }
            }
            continue;
        }
        let found = birthdays(people, item.fill, today);
        for group in found.chunks(BIRTHDAYS_PER_SLIDE) {
            let lines: Vec<String> = group
                .iter()
                .map(|person| format!("{}\t{} {}", person.name, person.day, today.months[usize::from(person.month.clamp(1, 12)) - 1]))
                .collect();
            let filled = Announcement { body: lines.join("\n"), ..item.clone() };
            pages.push((at, announcement_slide(&filled)));
        }
    }
    pages
}

/// Every announcement as one deck, for the loop shown before a service.
pub fn announcements_deck(announcements: &[Announcement], people: &[Person], today: &Today, bible: Option<&Bible>, per_slide: usize) -> Deck {
    Deck {
        title: "Announcements".to_string(),
        subtitle: String::new(),
        source: DeckSource::Announcements,
        slides: announcement_pages(announcements, people, today, bible, per_slide).into_iter().map(|(_, slide)| slide).collect(),
    }
}

pub fn announcement_slide(announcement: &Announcement) -> Slide {
    Slide {
        kind: SlideKind::Announcement,
        label: announcement.title.clone(),
        short: String::new(),
        tag: None,
        heading: announcement.title.clone(),
        text: announcement.body.clone(),
        second: String::new(),
        caption: String::new(),
        image: announcement.image.clone(),
    }
}

/// The colour family of a section, as an index a theme maps to a colour.
pub fn tag_group(tag: Tag) -> usize {
    match tag.kind {
        SectionKind::Verse => 0,
        SectionKind::Chorus => 1,
        SectionKind::Bridge => 2,
        SectionKind::PreChorus => 3,
        SectionKind::Intro | SectionKind::Ending => 4,
        SectionKind::Other => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bible::Verse;

    #[test]
    fn a_song_deck_follows_the_order_and_splits_long_sections() {
        let song = Song {
            id: 7,
            title: "Hill Song".into(),
            author: "A. Writer".into(),
            lyrics: "[1]\na\nb\nc\nd\ne\nf\n[chorus]\nx\ny\n".into(),
            order: "V1 C C".into(),
            ..Song::default()
        };
        let deck = song_deck(&song, 4, "12345");
        let labels: Vec<&str> = deck.slides.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(labels, ["Verse 1", "Verse 1", "Chorus", "Chorus"]);
        assert_eq!(deck.slides[0].text, "a\nb\nc");
        assert_eq!(deck.slides[0].caption, "A. Writer, CCLI 12345");
        assert!(deck.slides[1].caption.is_empty());
    }

    #[test]
    fn a_birthday_announcement_fills_itself_and_pages() {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let person = |name: &str, month: u8, day: u8| Person { id: 0, name: name.into(), day, month };
        let mut people = vec![person("Rina", 1, 2), person("Budi", 12, 30), person("Tom", 12, 20), person("Ana", 6, 1)];
        let today = Today { month: 12, day: 28, months: &MONTHS };
        // The week from 28 December runs into January, and 20 December is already past.
        let week: Vec<&str> = birthdays(&people, Fill::BirthdaysWeek, &today).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(week, ["Budi", "Rina"]);
        let month: Vec<&str> = birthdays(&people, Fill::BirthdaysMonth, &today).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(month, ["Tom", "Budi"]);
        let items = vec![
            Announcement { title: "Welcome".into(), body: "Hello".into(), ..Announcement::default() },
            Announcement { title: "Birthdays".into(), fill: Fill::BirthdaysMonth, ..Announcement::default() },
        ];
        let pages = announcement_pages(&items, &people, &today, None, 2);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1].1.text, "Tom\t20 December\nBudi\t30 December");
        // Ten more December birthdays spill onto a second slide; an empty month gives none.
        people.extend((1..=10).map(|day| person(&format!("Person {day:02}"), 12, day)));
        assert_eq!(announcement_pages(&items, &people, &today, None, 2).iter().filter(|(at, _)| *at == 1).count(), 2);
        let june = Today { month: 7, day: 1, months: &MONTHS };
        assert_eq!(announcement_pages(&items, &people, &june, None, 2).len(), 1);
        // Twenty-five lines of a brought-in table become three slides of the same announcement.
        let long = vec![Announcement { title: "List".into(), body: (1..=25).map(|n| format!("Row {n}	{n}")).collect::<Vec<_>>().join("
"), ..Announcement::default() }];
        let pages = announcement_pages(&long, &[], &today, None, 2);
        assert_eq!(pages.iter().map(|(_, slide)| slide.text.lines().count()).collect::<Vec<_>>(), [10, 10, 5]);
    }

    fn made_up_bible() -> Bible {
        let verse = |v: u16, text: &str| Verse { book: 19, chapter: 23, verse: v, text: text.into() };
        let names = vec![(19, "Mazmur".to_string(), "Mzm".to_string())];
        Bible::from_parts("TST".into(), None, String::new(), names, vec![verse(1, "Made up one."), verse(2, "Made up two."), verse(3, "Made up three.")]).unwrap_or_else(|_| unreachable!())
    }

    #[test]
    fn a_passage_card_fills_from_the_bible() {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let today = Today { month: 5, day: 1, months: &MONTHS };
        let bible = made_up_bible();
        let card = Announcement { title: "Verse of the week".into(), fill: Fill::Passage, passage: "mazmur 23:1-3".into(), ..Announcement::default() };
        assert_eq!(passage_state(&card, Some(&bible)), PassageState::Ready(Reference { book: 19, chapter: 23, verse: 1, end_chapter: 23, end_verse: 3 }));
        let pages = announcement_pages(std::slice::from_ref(&card), &[], &today, Some(&bible), 2);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].1.heading, "Verse of the week");
        assert_eq!(pages[0].1.text, "1 Made up one. 2 Made up two.");
        assert_eq!(pages[1].1.caption, "Mazmur 23:3 TST");
        // The English name reads too, and the reference is re-read when the Bible changes.
        let english = Announcement { passage: "psalms 23:2".into(), ..card.clone() };
        assert_eq!(announcement_pages(&[english], &[], &today, Some(&bible), 2)[0].1.text, "Made up two.");
    }

    #[test]
    fn a_passage_card_says_why_it_has_no_words() {
        let bible = made_up_bible();
        let card = Announcement { fill: Fill::Passage, passage: "mazmur 23:1".into(), ..Announcement::default() };
        assert_eq!(passage_state(&card, None), PassageState::NoBible);
        for typed in ["", "mazmur", "mazmur 23:99", "nonsense words", "mazmur 24:1"] {
            let card = Announcement { passage: typed.into(), ..card.clone() };
            assert_eq!(passage_state(&card, Some(&bible)), PassageState::Unreadable, "{typed:?}");
        }
        let plain = Announcement { title: "Plain".into(), ..Announcement::default() };
        assert_eq!(passage_state(&plain, Some(&bible)), PassageState::NotAPassage);
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let today = Today { month: 5, day: 1, months: &MONTHS };
        assert!(announcement_pages(&[card], &[], &today, None, 2).is_empty());
    }

    #[test]
    fn a_birthday_card_names_the_days_it_covers() {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let today = Today { month: 12, day: 28, months: &MONTHS };
        assert_eq!(birthday_days(Fill::BirthdaysWeek, &today), Some(((12, 28), (1, 3))));
        assert_eq!(birthday_days(Fill::BirthdaysMonth, &today), Some(((12, 1), (12, 31))));
        assert_eq!(birthday_days(Fill::Passage, &today), None);
    }

    #[test]
    fn a_second_language_follows_its_section() {
        let song = Song { lyrics: "[1]\none\ntwo\n[chorus]\nthree\n".into(), second: "[chorus]\ntiga\n[1]\nsatu\ndua\n".into(), ..Song::default() };
        let deck = song_deck(&song, 4, "");
        assert_eq!(deck.slides[0].second, "satu\ndua");
        assert_eq!(deck.slides[1].second, "tiga");
    }

    #[test]
    fn a_passage_becomes_one_slide_per_verse() {
        let verse = |v: u16, text: &str| Verse { book: 43, chapter: 3, verse: v, text: text.into() };
        let bible = Bible::from_parts("TST".into(), None, String::new(), Vec::new(), vec![verse(16, "One."), verse(17, "Two."), verse(18, "Three.")])
            .unwrap_or_else(|_| unreachable!());
        let reference = Reference { book: 43, chapter: 3, verse: 16, end_chapter: 3, end_verse: 18 };
        let deck = passage_deck(&bible, &reference, None, 1);
        assert_eq!(deck.slides.len(), 3);
        assert_eq!(deck.slides[1].caption, "John 3:17 TST");
        let paired = passage_deck(&bible, &reference, Some(&bible), 2);
        assert_eq!(paired.slides.len(), 2);
        assert_eq!(paired.slides[0].text, "16 One. 17 Two.");
        assert_eq!(paired.slides[0].second, "16 One. 17 Two.");
        assert_eq!(paired.slides[0].label, "John 3:16-17");
    }
}
