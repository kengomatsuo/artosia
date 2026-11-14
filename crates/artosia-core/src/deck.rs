//! Decks: the slides of one thing that can be shown (a song, a passage, an announcement).

use serde::{Deserialize, Serialize};

use crate::bible::{Bible, Reference};
use crate::calendar::{Date, Event, occurrences};
use crate::giving::{Giving, offering_text, transfer_text};
use crate::song::{SectionKind, Song, Tag};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlideKind {
    #[default]
    Lyrics,
    Scripture,
    Announcement,
}

/// What an announcement is about, which decides how its lines are laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoticeKind {
    #[default]
    Plain,
    Schedule,
    Birthdays,
    Money,
    Account,
}

/// How an announcement slide is built: its structure, not only its colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NoticeStyle {
    /// Dark scrim over the background, a rule under a small label, a heavy title, bare rows.
    #[default]
    Hairline,
    /// A side tab, and the rows on an opaque ruled table over the background.
    Ledger,
    /// A flat band for the title over a flat charcoal body.
    Band,
}

impl NoticeStyle {
    pub const ALL: [NoticeStyle; 3] = [NoticeStyle::Hairline, NoticeStyle::Ledger, NoticeStyle::Band];

    /// The word stored in the library.
    pub fn code(self) -> &'static str {
        match self {
            NoticeStyle::Hairline => "hairline",
            NoticeStyle::Ledger => "ledger",
            NoticeStyle::Band => "band",
        }
    }

    /// None for a card that chose no style and follows the look.
    pub fn from_code(code: &str) -> Option<NoticeStyle> {
        NoticeStyle::ALL.into_iter().find(|style| style.code() == code)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slide {
    pub kind: SlideKind,
    /// What the announcement is about; `Plain` draws the words as they are.
    #[serde(default)]
    pub notice: NoticeKind,
    /// The structure its card chose; None follows the look.
    #[serde(default)]
    pub style: Option<NoticeStyle>,
    /// The title colour its card chose; None takes the style's own.
    #[serde(default)]
    pub title_colour: Option<[u8; 3]>,
    /// "Verse 1", "Chorus", "John 3:16".
    pub label: String,
    /// "V1", "C", "16".
    pub short: String,
    /// The section colour; None for scripture and announcements.
    pub tag: Option<Tag>,
    /// The heading drawn above the text on announcement slides.
    pub heading: String,
    pub text: String,
    /// The same lines in a second translation of a Bible passage, drawn small under the text;
    /// empty when there is none. A song's further languages are in `more`.
    pub second: String,
    /// Small print under the text: the reference and translation, or the song credit.
    pub caption: String,
    /// A picture shown with the words, or alone when there are none, as a path; empty for none.
    #[serde(default)]
    pub image: String,
    /// The language code of `text` on a song's slide; empty when unknown or not a song.
    #[serde(default)]
    pub lang: String,
    /// Where `text`'s language stands in the list of languages the slide was built for, which
    /// picks its colour so a language keeps its colour from song to song.
    #[serde(default)]
    pub slot: u8,
    /// The song's further languages on this slide, in the order of the list, each the same
    /// lines as `text` in step by line.
    #[serde(default)]
    pub more: Vec<Voice>,
    /// Which piece of a slide that had to be split to stay readable this is, (1, 2) for "1/2".
    #[serde(default)]
    pub part: Option<(u16, u16)>,
}

/// One further language on a slide.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Voice {
    pub lang: String,
    pub slot: u8,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckSource {
    Song(i64),
    Passage { reference: Reference, bible: String, second: Option<String> },
    Announcement(i64),
    Announcements,
    /// A rundown video, sound or scene, by `PlanEntry::media_key`.
    Media(String),
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
    /// The calendar's events in the seven days from today, one line each.
    ScheduleWeek,
    /// The funds and what each received on the newest date a gift was kept, with their total.
    Offerings,
    /// The church's transfer account: bank, number, holder and note.
    Transfer,
}

impl Fill {
    /// The word stored in the library.
    pub fn code(self) -> &'static str {
        match self {
            Fill::Nothing => "",
            Fill::BirthdaysWeek => "birthdays-week",
            Fill::BirthdaysMonth => "birthdays-month",
            Fill::Passage => "passage",
            Fill::ScheduleWeek => "schedule-week",
            Fill::Offerings => "offerings",
            Fill::Transfer => "transfer",
        }
    }

    pub fn from_code(code: &str) -> Fill {
        match code {
            "birthdays-week" => Fill::BirthdaysWeek,
            "birthdays-month" => Fill::BirthdaysMonth,
            "passage" => Fill::Passage,
            "schedule-week" => Fill::ScheduleWeek,
            "offerings" => Fill::Offerings,
            "transfer" => Fill::Transfer,
            _ => Fill::Nothing,
        }
    }
}

/// What a card is: where its words come from. Only `Plain` holds words typed by a person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    Schedule,
    Birthdays,
    Offerings,
    Transfer,
    Passage,
    Plain,
}

impl Template {
    pub const ALL: [Template; 6] = [Template::Schedule, Template::Birthdays, Template::Offerings, Template::Transfer, Template::Passage, Template::Plain];

    /// The data source a new card of this template starts with.
    pub fn fill(self) -> Fill {
        match self {
            Template::Schedule => Fill::ScheduleWeek,
            Template::Birthdays => Fill::BirthdaysWeek,
            Template::Offerings => Fill::Offerings,
            Template::Transfer => Fill::Transfer,
            Template::Passage => Fill::Passage,
            Template::Plain => Fill::Nothing,
        }
    }
}

impl Fill {
    pub fn template(self) -> Template {
        match self {
            Fill::Nothing => Template::Plain,
            Fill::BirthdaysWeek | Fill::BirthdaysMonth => Template::Birthdays,
            Fill::Passage => Template::Passage,
            Fill::ScheduleWeek => Template::Schedule,
            Fill::Offerings => Template::Offerings,
            Fill::Transfer => Template::Transfer,
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
    /// The structure the card is drawn in; None reads as the look's, which is how cards made
    /// before styles existed keep their look.
    #[serde(default)]
    pub style: Option<NoticeStyle>,
    /// The colour of the title; None takes the style's own.
    #[serde(default)]
    pub title_colour: Option<[u8; 3]>,
    /// The calendars a schedule card reads, by id; empty reads all of them.
    #[serde(default)]
    pub calendars: Vec<i64>,
    /// A schedule card reads the whole month instead of the seven days from today.
    #[serde(default)]
    pub month: bool,
}

/// Someone whose birthday the church keeps. The year, when known, is for the Members window's age
/// and group only; no slide ever carries it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    /// 1 to 31.
    pub day: u8,
    /// 1 to 12.
    pub month: u8,
    /// The year of birth; 0 when it is not known.
    #[serde(default)]
    pub year: u16,
}

/// The day an announcement is built on, with the month names to write dates in.
#[derive(Clone, Copy, Debug)]
pub struct Today<'a> {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub months: &'a [&'a str; 12],
    /// The names of the weekdays, Monday first, as the window lists them.
    pub weekdays: &'a [&'a str; 7],
    /// How an hour and a minute read as a time of day in the window's language.
    pub clock: fn(u8, u8) -> String,
    /// How a day and a month's name read as a date in the window's language.
    pub date: fn(u8, &str) -> String,
    /// The word that names a card's total, as the window writes it.
    pub total: &'a str,
}

/// The most names on one birthday slide, three columns of ten; more go on a second slide.
pub const BIRTHDAYS_PER_SLIDE: usize = 30;
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
        Fill::Nothing | Fill::Passage | Fill::ScheduleWeek | Fill::Offerings | Fill::Transfer => None,
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

impl Today<'_> {
    /// Today as a date; a day the month does not have is taken as its last.
    pub fn date_of(&self) -> Date {
        let month = self.month.clamp(1, 12);
        Date { year: self.year, month, day: self.day.clamp(1, Date::month_days(self.year, month)) }
    }
}

/// The events of the seven days from today, in date and time order: date, start minute, name.
pub fn week_events(events: &[Event], today: &Today) -> Vec<(Date, u16, String)> {
    window_events(events, today, false, &[])
}

/// The events a schedule card reads: the seven days from today, or the whole month, of the
/// calendars it names (all when it names none), in date and time order.
pub fn window_events(events: &[Event], today: &Today, month: bool, calendars: &[i64]) -> Vec<(Date, u16, String)> {
    let day = today.date_of();
    let (from, to) = if month { (day.first_of_month(), day.first_of_month().plus_months(1).plus_days(-1)) } else { (day, day.plus_days(6)) };
    let mut found: Vec<(Date, u16, String)> = events
        .iter()
        .filter(|event| calendars.is_empty() || calendars.contains(&event.calendar))
        .flat_map(|event| occurrences(event, from, to).into_iter().map(|date| (date, event.start, event.name.clone())))
        .collect();
    found.sort();
    found
}

/// How many things a card reads from its source today: events, names, gifts of the newest date,
/// 1 or 0 for the account and for a passage; None for a card that holds typed words.
pub fn source_count(item: &Announcement, people: &[Person], events: &[Event], giving: &Giving, today: &Today, bible: Option<&Bible>) -> Option<usize> {
    match item.fill {
        Fill::Nothing => None,
        Fill::Passage => Some(usize::from(matches!(passage_state(item, bible), PassageState::Ready(_)))),
        Fill::ScheduleWeek => Some(window_events(events, today, item.month, &item.calendars).len()),
        Fill::BirthdaysWeek | Fill::BirthdaysMonth => Some(birthdays(people, item.fill, today).len()),
        Fill::Offerings => Some(crate::giving::latest(giving.gifts).map_or(0, |day| giving.gifts.iter().filter(|gift| gift.date == day).count())),
        Fill::Transfer => Some(usize::from(giving.account.ready())),
    }
}

/// The lines of a schedule card: the weekday, the time and the name, with the weekday left empty
/// on a line of the same day as the line above, so it is written once.
fn schedule_lines(found: &[(Date, u16, String)], today: &Today) -> Vec<String> {
    let mut before = None;
    found
        .iter()
        .map(|(date, start, name)| {
            let day = if before == Some(*date) { "" } else { today.weekdays[usize::from((date.weekday() + 6) % 7)] };
            before = Some(*date);
            let (hour, minute) = crate::calendar::hour_minute(*start);
            format!("{day}	{}	{name}", (today.clock)(hour, minute))
        })
        .collect()
}

/// The people an announcement fills itself with, in date order from today.
pub fn birthdays<'a>(people: &'a [Person], fill: Fill, today: &Today) -> Vec<&'a Person> {
    let key = |month: u8, day: u8| u16::from(month) * 100 + u16::from(day);
    let from = key(today.month, today.day);
    let mut found: Vec<&Person> = match fill {
        Fill::Nothing | Fill::Passage | Fill::ScheduleWeek | Fill::Offerings | Fill::Transfer => Vec::new(),
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

/// Which of a song's languages its slides draw: a list of codes whose order is the order on
/// the slide, and a code that narrows the slide to one language without changing the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Languages {
    pub shown: Vec<String>,
    pub solo: Option<String>,
}

impl Languages {
    /// The song's languages to draw, as (place in the list, index into `Song::texts`), in the
    /// order of the list. A listed code fits a song's language by `artosia_locale::pick`
    /// (`en` fits `en-US`, `zh-Hant` fits `zh-TW`), and a transliteration (`ja-Latn`) fits no
    /// language but its own. A listed language the song lacks is left out; when none is left, the
    /// first language alone is drawn, so a slide is never empty for want of a match.
    ///
    /// A translation with no code (the second text of a library from 0.1.16 never had one) that
    /// the list does not name follows the others, as 0.1.16 drew `second`, in a place after the
    /// list's own. A solo code the song lacks changes nothing.
    pub fn pick(&self, song: &Song) -> Vec<(u8, usize)> {
        let texts: Vec<(&str, &str)> = song.texts().collect();
        let find = |code: &str| texts.iter().position(|(have, text)| fits(have, code) && !text.trim().is_empty());
        let mut picked: Vec<(u8, usize)> = Vec::new();
        for (slot, code) in self.shown.iter().enumerate() {
            if let Some(at) = find(code).filter(|at| picked.iter().all(|(_, have)| have != at)) {
                picked.push((u8::try_from(slot).unwrap_or(u8::MAX), at));
            }
        }
        if let Some(only) = self.solo.as_deref().and_then(find).and_then(|at| picked.iter().find(|(_, have)| *have == at)) {
            return vec![*only];
        }
        if picked.is_empty() {
            picked.push((0, 0));
        }
        let mut next = (self.shown.len()).max(picked.iter().map(|(slot, _)| usize::from(*slot) + 1).max().unwrap_or(0));
        for (at, (code, text)) in texts.iter().enumerate().skip(1) {
            if code.is_empty() && !text.trim().is_empty() && picked.iter().all(|(_, have)| *have != at) {
                picked.push((u8::try_from(next).unwrap_or(u8::MAX), at));
                next += 1;
            }
        }
        picked
    }
}

/// Whether a song's language `have` is the listed language `listed`: the rule of
/// `artosia-locale`, with a script a tag names kept apart (`ja-Latn` is not `ja`).
pub fn fits(have: &str, listed: &str) -> bool {
    if have.is_empty() || listed.is_empty() {
        return false;
    }
    let script = |tag: &str| tag.split(['-', '_']).skip(1).find(|part| part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic())).map(str::to_ascii_lowercase);
    let chinese = |tag: &str| tag.get(..2).is_some_and(|first| first.eq_ignore_ascii_case("zh"));
    if chinese(have) && chinese(listed) && script(listed).is_some() {
        // Chinese is told apart by script: Taiwan's tag is Traditional and fits no Simplified entry.
        return artosia_locale::pick(have, &["zh-Hans", "zh-Hant"]).is_some_and(|got| script(got) == script(listed));
    }
    artosia_locale::pick(have, &[listed]).is_some() && (script(have).is_none() || script(have) == script(listed))
}

/// Cuts one slide's lines, a list of lines for each language, into the fewest pieces in which
/// every piece fits (`fits` is given each language's lines of the piece). Every language is cut
/// at the same place in its lines, in proportion when the languages differ in length, so the
/// languages stay in step. A piece holds nothing of a language that has run out of lines. When
/// nothing fits even one line to a piece, the slide is left whole: more slides would not help.
pub fn split_in_step(lines: &[Vec<String>], fits: &dyn Fn(&[&[String]]) -> bool) -> Vec<Vec<Vec<String>>> {
    let longest = lines.iter().map(Vec::len).max().unwrap_or(0).max(1);
    for pieces in 1..=longest {
        // The longest language is cut as `split_lines` cuts: even pieces, the first ones longer.
        let (base, extra) = (longest / pieces, longest % pieces);
        let mut cuts = vec![0usize];
        for piece in 0..pieces {
            cuts.push(cuts[piece] + base + usize::from(piece < extra));
        }
        let cut = |language: &Vec<String>, at: usize| (cuts[at] * language.len() + longest / 2) / longest;
        let cut_up: Vec<Vec<Vec<String>>> = (0..pieces)
            .map(|piece| lines.iter().map(|language| language[cut(language, piece)..cut(language, piece + 1)].to_vec()).collect())
            .collect();
        let fit = cut_up.iter().all(|piece| {
            let present: Vec<&[String]> = piece.iter().map(Vec::as_slice).filter(|language| !language.is_empty()).collect();
            fits(&present)
        });
        if fit {
            return cut_up;
        }
    }
    vec![lines.to_vec()]
}

/// A song's slides in sung order; sections longer than `max_lines` are split evenly. Only the
/// first language is drawn.
pub fn song_deck(song: &Song, max_lines: usize, licence: &str) -> Deck {
    song_deck_in(song, max_lines, licence, &Languages::default(), &|_| true)
}

/// A song's slides in the languages `languages` picks. With one language the slides are
/// `song_deck`'s. With several, each section's lines are dealt into slides by the first picked
/// language that has the section, the others follow in step by line, and a slide whose blocks
/// `fits` rejects is split into the fewest slides that it accepts, each marked with its place
/// in `part`.
pub fn song_deck_in(song: &Song, max_lines: usize, licence: &str, languages: &Languages, fits: &dyn Fn(&[&[String]]) -> bool) -> Deck {
    let picked = languages.pick(song);
    let texts: Vec<(&str, &str)> = song.texts().collect();
    let parsed: Vec<Vec<crate::song::Section>> = picked.iter().map(|&(_, at)| crate::song::parse_lyrics(texts[at].1)).collect();
    let credit = song_credit(song, licence);
    let mut slides = Vec::new();
    for section in song.arranged() {
        let have: Vec<(usize, &crate::song::Section)> =
            parsed.iter().enumerate().filter_map(|(k, sections)| sections.iter().find(|other| other.tag == section.tag).map(|other| (k, other))).collect();
        let Some(&(lead_voice, reference)) = have.first() else { continue };
        let pieces = reference.slides(max_lines);
        // Another language keeps its own slide breaks when it has as many as the reference;
        // otherwise its lines are dealt evenly across the reference's slides.
        let others: Vec<(usize, Vec<Vec<String>>)> = have[1..]
            .iter()
            .map(|&(k, other)| {
                let own = other.slides(max_lines);
                let dealt = if own.len() == pieces.len() { own } else { crate::song::split_lines(&other.lines, other.lines.len().div_ceil(pieces.len()).max(1)) };
                (k, dealt)
            })
            .collect();
        for (i, lines) in pieces.iter().enumerate() {
            let mut voices: Vec<(usize, Vec<String>)> = vec![(lead_voice, lines.clone())];
            voices.extend(others.iter().filter_map(|(k, dealt)| dealt.get(i).map(|lines| (*k, lines.clone()))));
            let parts = if voices.len() > 1 {
                let aligned: Vec<Vec<String>> = voices.iter().map(|(_, lines)| lines.clone()).collect();
                split_in_step(&aligned, fits)
            } else {
                vec![vec![lines.clone()]]
            };
            let count = parts.len();
            for (at, part) in parts.into_iter().enumerate() {
                let mut shown = voices.iter().zip(part).filter(|(_, lines)| !lines.is_empty());
                let Some(((first, _), first_lines)) = shown.next() else { continue };
                let voice = |k: usize, lines: &[String]| Voice { lang: texts[picked[k].1].0.to_string(), slot: picked[k].0, text: lines.join("
") };
                let lead = voice(*first, &first_lines);
                slides.push(Slide {
                    kind: SlideKind::Lyrics,
                    label: section.tag.label(),
                    short: section.tag.short(),
                    tag: Some(section.tag),
                    text: lead.text,
                    lang: lead.lang,
                    slot: lead.slot,
                    more: shown.map(|((k, _), lines)| voice(*k, &lines)).collect(),
                    caption: if slides.is_empty() { credit.clone() } else { String::new() },
                    part: (count > 1).then(|| (at as u16 + 1, count as u16)),
                    ..Slide::default()
                });
            }
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
            Some(other) => format!("{} / {}", bible.short, other.short),
            None => bible.short.clone(),
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
            ..Slide::default()
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
/// list. One that fills itself with the calendar's week gives a slide per ten events, and none when
/// the week is empty. One that fills itself with the offerings or the transfer account gives one slide,
/// and none while there is no gift or the account lacks its bank, number or holder. One that fills itself with birthdays gives a slide per eight names, and none at all
/// when nobody has one. A passage card gives its verses, `per_slide` to a slide, and none at all
/// while its reference cannot be read from `bible`.
pub fn announcement_pages(announcements: &[Announcement], people: &[Person], events: &[Event], giving: &Giving, today: &Today, bible: Option<&Bible>, per_slide: usize) -> Vec<(usize, Slide)> {
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
        if item.fill == Fill::ScheduleWeek {
            // A week of services fits one slide; a longer one goes on, the weekday written again where a day is cut.
            for group in window_events(events, today, item.month, &item.calendars).chunks(LINES_PER_SLIDE) {
                let mut slide = announcement_slide(&Announcement { body: schedule_lines(group, today).join("
"), ..item.clone() });
                slide.notice = NoticeKind::Schedule;
                pages.push((at, slide));
            }
            continue;
        }
        if matches!(item.fill, Fill::Offerings | Fill::Transfer) {
            let (text, notice) = if item.fill == Fill::Offerings { (offering_text(giving, today.total), NoticeKind::Money) } else { (transfer_text(giving.account), NoticeKind::Account) };
            // A card with no gifts or no account gives no slide, as a birthday card with nobody does.
            if let Some(text) = text {
                let mut slide = announcement_slide(&Announcement { body: text, ..item.clone() });
                slide.notice = notice;
                pages.push((at, slide));
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
                .map(|person| format!("{}\t{}", person.name, (today.date)(person.day, today.months[usize::from(person.month.clamp(1, 12)) - 1])))
                .collect();
            let filled = Announcement { body: lines.join("
"), ..item.clone() };
            let mut slide = announcement_slide(&filled);
            slide.notice = NoticeKind::Birthdays;
            pages.push((at, slide));
        }
    }
    // Every slide of a card is drawn in the card's own style and title colour.
    for (at, slide) in &mut pages {
        slide.style = announcements[*at].style;
        slide.title_colour = announcements[*at].title_colour;
    }
    pages
}

/// Every announcement as one deck, for the loop shown before a service.
pub fn announcements_deck(announcements: &[Announcement], people: &[Person], events: &[Event], giving: &Giving, today: &Today, bible: Option<&Bible>, per_slide: usize) -> Deck {
    Deck {
        title: "Announcements".to_string(),
        subtitle: String::new(),
        source: DeckSource::Announcements,
        slides: announcement_pages(announcements, people, events, giving, today, bible, per_slide).into_iter().map(|(_, slide)| slide).collect(),
    }
}

/// A notice's lines as rows of cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoticeBody {
    /// A tab splits a line into cells. In a schedule an empty first cell means "the same day as
    /// the row above", and stays empty so the day is drawn once.
    pub rows: Vec<Vec<String>>,
    /// The line that started with `== `, drawn apart as the total.
    pub total: Option<Vec<String>>,
}

pub fn notice_body(text: &str) -> NoticeBody {
    let cells = |line: &str| line.split('\t').map(|cell| cell.trim().to_string()).collect::<Vec<_>>();
    let mut body = NoticeBody::default();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match line.strip_prefix("== ") {
            Some(total) => body.total = Some(cells(total)),
            None => body.rows.push(cells(line)),
        }
    }
    body
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
        ..Slide::default()
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
    use crate::giving::Account;
    use crate::song::Translation;

    fn nothing() -> Giving<'static> {
        static ACCOUNT: Account = Account { bank: String::new(), number: String::new(), holder: String::new(), note: String::new() };
        Giving { funds: &[], gifts: &[], account: &ACCOUNT }
    }

    #[test]
    fn the_money_cards_read_the_gifts_and_the_account_and_give_nothing_while_they_have_none() {
        use crate::giving::{Fund, Gift, Method};
        let funds = vec![Fund { id: 1, name: "Tithe".into(), colour: [0; 3] }, Fund { id: 2, name: "Missions".into(), colour: [0; 3] }];
        let gift = |day: u8, fund: i64, amount: i64| Gift { id: 0, date: Date { year: 2026, month: 11, day }, giver: String::new(), fund, method: Method::Cash, amount };
        let gifts = vec![gift(1, 1, 10), gift(8, 1, 1_000), gift(8, 2, 250_000)];
        let account = Account { bank: "Bank Contoh".into(), number: "0123".into(), holder: "Gereja Contoh".into(), note: "Write the fund".into() };
        let cards = vec![Announcement { title: "Offering".into(), fill: Fill::Offerings, ..Announcement::default() }, Announcement { title: "Transfer".into(), fill: Fill::Transfer, ..Announcement::default() }];
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let today = today_on(11, 13, &MONTHS);
        let pages = announcement_pages(&cards, &[], &[], &Giving { funds: &funds, gifts: &gifts, account: &account }, &today, None, 2);
        assert_eq!(pages.len(), 2);
        assert_eq!((pages[0].1.notice, pages[0].1.text.as_str()), (NoticeKind::Money, "Tithe	Rp 1.000
Missions	Rp 250.000
== Total	Rp 251.000"));
        assert_eq!((pages[1].1.notice, pages[1].1.text.as_str()), (NoticeKind::Account, "Bank Contoh
0123
Gereja Contoh
Write the fund"));
        assert_eq!(pages[1].1.heading, "Transfer");
        assert!(announcement_pages(&cards, &[], &[], &nothing(), &today, None, 2).is_empty());
        for fill in [Fill::Offerings, Fill::Transfer] {
            assert_eq!(Fill::from_code(fill.code()), fill);
        }
    }

    #[test]
    fn a_notice_body_splits_cells_and_sets_the_total_line_apart() {
        let body = notice_body("Minggu\t06.30\tIbadah\n\t09.00\tIbadah 2\n\n== Total\tRp 10");
        assert_eq!(body.rows, vec![vec!["Minggu", "06.30", "Ibadah"], vec!["", "09.00", "Ibadah 2"]]);
        assert_eq!(body.total, Some(vec!["Total".to_string(), "Rp 10".to_string()]));
        assert_eq!(notice_body("one line").rows, vec![vec!["one line"]]);
    }

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

    const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

    fn clock_dotted(hour: u8, minute: u8) -> String {
        format!("{hour:02}.{minute:02}")
    }

    /// A day of 2026, with English names, for the cards that read the date.
    fn today_on<'a>(month: u8, day: u8, months: &'a [&'a str; 12]) -> Today<'a> {
        Today { year: 2026, month, day, months, weekdays: &WEEKDAYS, clock: clock_dotted, date: day_first, total: "Total" }
    }

    fn day_first(day: u8, month: &str) -> String {
        format!("{day} {month}")
    }

    #[test]
    fn a_birthday_announcement_fills_itself_and_pages() {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let person = |name: &str, month: u8, day: u8| Person { name: name.into(), day, month, ..Person::default() };
        let mut people = vec![person("Rina", 1, 2), person("Budi", 12, 30), person("Tom", 12, 20), person("Ana", 6, 1)];
        let today = today_on(12, 28, &MONTHS);
        // The week from 28 December runs into January, and 20 December is already past.
        let week: Vec<&str> = birthdays(&people, Fill::BirthdaysWeek, &today).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(week, ["Budi", "Rina"]);
        let month: Vec<&str> = birthdays(&people, Fill::BirthdaysMonth, &today).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(month, ["Tom", "Budi"]);
        let items = vec![
            Announcement { title: "Welcome".into(), body: "Hello".into(), ..Announcement::default() },
            Announcement { title: "Birthdays".into(), fill: Fill::BirthdaysMonth, ..Announcement::default() },
        ];
        let pages = announcement_pages(&items, &people, &[], &nothing(), &today, None, 2);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1].1.text, "Tom\t20 December\nBudi\t30 December");
        // A language that names the month first writes its dates that way.
        let month_first = Today { date: |day, month| format!("{month} {day}"), ..today };
        assert_eq!(announcement_pages(&items, &people, &[], &nothing(), &month_first, None, 2)[1].1.text, "Tom\tDecember 20\nBudi\tDecember 30");
        // Thirty names fill one slide of three columns; the thirty-first goes on a second. An empty month gives none.
        people.extend((1..=28).map(|day| person(&format!("Person {day:02}"), 12, day)));
        let cards = |people: &[Person]| announcement_pages(&items, people, &[], &nothing(), &today, None, 2).into_iter().filter(|(at, _)| *at == 1).collect::<Vec<_>>();
        assert_eq!(people.iter().filter(|p| p.month == 12).count(), 30);
        let thirty = cards(&people);
        assert_eq!((thirty.len(), thirty[0].1.text.lines().count(), thirty[0].1.notice), (1, 30, NoticeKind::Birthdays));
        people.push(person("Person 29", 12, 29));
        assert_eq!(cards(&people).iter().map(|(_, slide)| slide.text.lines().count()).collect::<Vec<_>>(), [30, 1]);
        people.pop();
        let june = today_on(7, 1, &MONTHS);
        assert_eq!(announcement_pages(&items, &people, &[], &nothing(), &june, None, 2).len(), 1);
        // Twenty-five lines of a brought-in table become three slides of the same announcement.
        let long = vec![Announcement { title: "List".into(), body: (1..=25).map(|n| format!("Row {n}	{n}")).collect::<Vec<_>>().join("
"), ..Announcement::default() }];
        let pages = announcement_pages(&long, &[], &[], &nothing(), &today, None, 2);
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
        let today = today_on(5, 1, &MONTHS);
        let bible = made_up_bible();
        let card = Announcement { title: "Verse of the week".into(), fill: Fill::Passage, passage: "mazmur 23:1-3".into(), ..Announcement::default() };
        assert_eq!(passage_state(&card, Some(&bible)), PassageState::Ready(Reference { book: 19, chapter: 23, verse: 1, end_chapter: 23, end_verse: 3 }));
        let pages = announcement_pages(std::slice::from_ref(&card), &[], &[], &nothing(), &today, Some(&bible), 2);
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0].1.heading, "Verse of the week");
        assert_eq!(pages[0].1.text, "1 Made up one. 2 Made up two.");
        assert_eq!(pages[1].1.caption, "Mazmur 23:3 TST");
        // The English name reads too, and the reference is re-read when the Bible changes.
        let english = Announcement { passage: "psalms 23:2".into(), ..card.clone() };
        assert_eq!(announcement_pages(&[english], &[], &[], &nothing(), &today, Some(&bible), 2)[0].1.text, "Made up two.");
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
        let today = today_on(5, 1, &MONTHS);
        assert!(announcement_pages(&[card], &[], &[], &nothing(), &today, None, 2).is_empty());
    }

    #[test]
    fn a_birthday_card_names_the_days_it_covers() {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let today = today_on(12, 28, &MONTHS);
        assert_eq!(birthday_days(Fill::BirthdaysWeek, &today), Some(((12, 28), (1, 3))));
        assert_eq!(birthday_days(Fill::BirthdaysMonth, &today), Some(((12, 1), (12, 31))));
        assert_eq!(birthday_days(Fill::Passage, &today), None);
    }

    fn three_languages() -> Song {
        Song {
            lyrics: "[1]\none\ntwo\n[chorus]\nthree\nfour\n".into(),
            lang: "id".into(),
            translations: vec![
                Translation { lang: "en".into(), text: "[chorus]\nthree-en\nfour-en\n[1]\none-en\ntwo-en\n".into() },
                Translation { lang: "zh-Hans".into(), text: "[1]\none-zh\ntwo-zh\n[chorus]\nthree-zh\nfour-zh\n".into() },
            ],
            ..Song::default()
        }
    }

    fn languages(codes: &[&str]) -> Languages {
        Languages { shown: codes.iter().map(|code| code.to_string()).collect(), solo: None }
    }

    /// The same capacity test the real screen makes, by line count: `room` lines in all.
    fn room(lines: usize) -> impl Fn(&[&[String]]) -> bool {
        move |blocks: &[&[String]]| blocks.iter().map(|block| block.len()).sum::<usize>() <= lines
    }

    #[test]
    fn a_second_language_follows_its_section() {
        let song = three_languages();
        let deck = song_deck_in(&song, 4, "", &languages(&["id", "en"]), &room(99));
        assert_eq!(deck.slides[0].text, "one\ntwo");
        assert_eq!(deck.slides[0].more, [Voice { lang: "en".into(), slot: 1, text: "one-en\ntwo-en".into() }]);
        assert_eq!(deck.slides[1].more[0].text, "three-en\nfour-en");
        assert_eq!(deck.slides.len(), 2);
    }

    #[test]
    fn the_list_sets_the_order_and_what_the_song_lacks_is_left_out() {
        let song = three_languages();
        // The list's order is the slide's order; the first one listed leads the slide.
        let deck = song_deck_in(&song, 4, "", &languages(&["zh-Hans", "id"]), &room(99));
        assert_eq!((deck.slides[0].lang.as_str(), deck.slides[0].slot, deck.slides[0].text.as_str()), ("zh-Hans", 0, "one-zh\ntwo-zh"));
        assert_eq!((deck.slides[0].more[0].lang.as_str(), deck.slides[0].more[0].slot), ("id", 1));
        // A language the song does not have shows the others, and keeps the others' places in the list.
        let deck = song_deck_in(&song, 4, "", &languages(&["fr", "en", "zh-Hans"]), &room(99));
        assert_eq!((deck.slides[0].lang.as_str(), deck.slides[0].slot, deck.slides[0].more[0].slot), ("en", 1, 2));
        // A song's languages that are not listed are not drawn.
        assert!(deck.slides.iter().all(|slide| slide.more.len() == 1 && slide.lang != "id"));
        // Nothing listed matches: the first language alone, never an empty slide.
        let deck = song_deck_in(&song, 4, "", &languages(&["fr", "de"]), &room(99));
        assert_eq!((deck.slides[0].text.as_str(), deck.slides[0].more.len()), ("one\ntwo", 0));
    }

    #[test]
    fn an_empty_list_is_the_first_language_alone_exactly_as_before() {
        let song = three_languages();
        let alone = song_deck_in(&song, 4, "", &Languages::default(), &|_| false);
        let plain = song_deck(&Song { translations: Vec::new(), lang: String::new(), ..song.clone() }, 4, "");
        assert_eq!(alone.slides.len(), plain.slides.len());
        for (a, b) in alone.slides.iter().zip(&plain.slides) {
            assert_eq!((&a.label, &a.text, &a.second, &a.caption, a.more.len(), a.part), (&b.label, &b.text, &b.second, &b.caption, 0, None));
        }
    }

    /// A song as 0.1.16 stored it: no codes, the second text in the one extra box.
    fn old_song() -> Song {
        Song { lyrics: "[1]\none\ntwo\n".into(), translations: vec![Translation { lang: String::new(), text: "[1]\nuno\ndos\n".into() }], ..Song::default() }
    }

    #[test]
    fn a_translation_with_no_code_follows_the_first_language_whatever_the_list_says() {
        let song = old_song();
        // No list, a list that names other languages, and a list of one: the second text stays in view.
        for list in [Languages::default(), languages(&["en", "id"]), languages(&["zh-Hans"]), languages(&["id"])] {
            let deck = song_deck_in(&song, 4, "", &list, &room(99));
            assert_eq!((deck.slides[0].text.as_str(), deck.slides[0].more.len()), ("one\ntwo", 1), "{list:?}");
            assert_eq!(deck.slides[0].more[0].text, "uno\ndos");
        }
        // Its colour is a place after the list's own, so it never takes a listed language's colour.
        let deck = song_deck_in(&song, 4, "", &languages(&["en", "id"]), &room(99));
        assert_eq!(deck.slides[0].more[0].slot, 2);
        // A language the list does name is drawn once, in its listed place, not twice.
        let mut named = song.clone();
        named.translations[0].lang = "es".into();
        let deck = song_deck_in(&named, 4, "", &languages(&["es"]), &room(99));
        assert_eq!((deck.slides[0].lang.as_str(), deck.slides[0].more.len()), ("es", 0));
        // A translation that has a code the list leaves out stays out.
        let deck = song_deck_in(&named, 4, "", &languages(&["id"]), &room(99));
        assert_eq!(deck.slides[0].more.len(), 0);
    }

    #[test]
    fn a_listed_code_fits_a_song_s_language_by_the_locale_rule() {
        assert!(fits("en-US", "en") && fits("en", "en-GB") && fits("zh-TW", "zh-Hant") && fits("zh-Hant", "zh-Hant"));
        assert!(!fits("zh-TW", "zh-Hans") && !fits("zh-CN", "zh-Hant") && fits("zh-CN", "zh-Hans"));
        assert!(!fits("id", "en") && !fits("", "en") && !fits("en", ""));
        // A transliteration is kept apart from the language it writes.
        assert!(!fits("ja-Latn", "ja") && fits("ja-Latn", "ja-Latn") && fits("ja", "ja"));
        let song = Song { lang: "en-US".into(), lyrics: "[1]\na\n".into(), translations: vec![Translation { lang: "zh-TW".into(), text: "[1]\nb\n".into() }], ..Song::default() };
        let deck = song_deck_in(&song, 4, "", &languages(&["zh-Hant", "en"]), &room(99));
        assert_eq!((deck.slides[0].lang.as_str(), deck.slides[0].slot, deck.slides[0].more[0].slot), ("zh-TW", 0, 1));
    }

    #[test]
    fn a_solo_code_narrows_the_slide_without_changing_the_list() {
        let song = three_languages();
        let mut shown = languages(&["id", "en", "zh-Hans"]);
        shown.solo = Some("en".into());
        let deck = song_deck_in(&song, 4, "", &shown, &room(99));
        assert!(deck.slides.iter().all(|slide| slide.more.is_empty() && slide.lang == "en"));
        assert_eq!(deck.slides[0].text, "one-en\ntwo-en");
        assert_eq!(shown.shown.len(), 3, "the list itself is untouched");
        // A solo language the song lacks narrows nothing.
        shown.solo = Some("fr".into());
        assert_eq!(song_deck_in(&song, 4, "", &shown, &room(99)).slides[0].more.len(), 2);
    }

    #[test]
    fn a_slide_that_does_not_fit_splits_at_the_same_line_in_every_language() {
        let song = Song {
            lyrics: "[1]\na1\na2\na3\na4\n".into(),
            lang: "id".into(),
            translations: vec![Translation { lang: "en".into(), text: "[1]\nb1\nb2\nb3\nb4\n".into() }],
            ..Song::default()
        };
        // Eight lines do not fit a room for four: two slides of two lines in each language.
        let deck = song_deck_in(&song, 4, "", &languages(&["id", "en"]), &room(4));
        let shown: Vec<_> = deck.slides.iter().map(|s| (s.text.as_str(), s.more[0].text.as_str(), s.part)).collect();
        assert_eq!(shown, [("a1\na2", "b1\nb2", Some((1, 2))), ("a3\na4", "b3\nb4", Some((2, 2)))]);
        // It fits: one slide and no marker. One language alone is never split by this rule.
        assert_eq!(song_deck_in(&song, 4, "", &languages(&["id", "en"]), &room(8)).slides[0].part, None);
        let alone = song_deck_in(&song, 4, "", &languages(&["id"]), &room(1));
        assert_eq!((alone.slides.len(), alone.slides[0].part), (1, None));
    }

    #[test]
    fn splitting_picks_the_fewest_slides_and_keeps_languages_of_unequal_length_in_step() {
        let lines = |n: usize, tag: &str| (1..=n).map(|i| format!("{tag}{i}")).collect::<Vec<String>>();
        // Counts 6 and 3 with room for five lines: two pieces, 3+2 and 3+1 lines in proportion.
        let pieces = split_in_step(&[lines(6, "a"), lines(3, "b")], &room(5));
        assert_eq!(pieces.iter().map(|piece| piece.iter().map(Vec::len).collect::<Vec<_>>()).collect::<Vec<_>>(), [[3, 2], [3, 1]]);
        // Every piece of a language together is the language, once, in order.
        let joined: Vec<String> = pieces.iter().flat_map(|piece| piece[1].clone()).collect();
        assert_eq!(joined, lines(3, "b"));
        // A room for everything gives one piece; a room for two lines gives pieces of one line each.
        assert_eq!(split_in_step(&[lines(6, "a"), lines(3, "b")], &room(99)).len(), 1);
        assert_eq!(split_in_step(&[lines(4, "a"), lines(4, "b")], &room(2)).len(), 4);
        // Nothing fits even one line: the slide stays whole.
        assert_eq!(split_in_step(&[lines(4, "a"), lines(4, "b")], &room(1)).len(), 1);
    }

    #[test]
    fn a_language_lacking_a_section_leaves_it_out_of_that_slide() {
        let song = Song {
            lyrics: "[1]\none\n[chorus]\nthree\n".into(),
            lang: "id".into(),
            translations: vec![Translation { lang: "en".into(), text: "[chorus]\nthree-en\n".into() }],
            ..Song::default()
        };
        let deck = song_deck_in(&song, 4, "", &languages(&["id", "en"]), &room(99));
        assert_eq!((deck.slides[0].more.len(), deck.slides[1].more.len()), (0, 1));
        // The first language is not listed: the slides follow the first language's sections, and
        // a section the listed language lacks has no slide.
        let deck = song_deck_in(&song, 4, "", &languages(&["en"]), &room(99));
        assert_eq!(deck.slides.iter().map(|slide| slide.text.as_str()).collect::<Vec<_>>(), ["three-en"]);
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

    fn service(name: &str, date: crate::calendar::Date, start: u16, repeat: crate::calendar::Repeat) -> Event {
        Event { id: 0, calendar: 1, name: name.into(), date, start, end: start + 60, repeat, until: None, skipped: Vec::new() }
    }

    #[test]
    fn a_schedule_card_lists_the_next_seven_days_and_writes_each_day_once() {
        use crate::calendar::Repeat;
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let first = Date::new(2026, 1, 4).unwrap();
        let events = vec![
            service("Second service", first, 9 * 60, Repeat::Weekly(1)),
            service("First service", first, 6 * 60 + 30, Repeat::Weekly(1)),
            service("Prayer", first, 18 * 60 + 30, Repeat::Weekly(1 << 3)),
            // Eight days off: outside the week.
            service("Later", Date::new(2026, 11, 22).unwrap(), 10 * 60, Repeat::Once),
        ];
        let card = Announcement { title: "This week".into(), fill: Fill::ScheduleWeek, ..Announcement::default() };
        // Friday 13 November 2026: Sunday the 15th and Wednesday the 18th are in; the next Sunday, the 22nd, is a week and a day off.
        let today = today_on(11, 13, &MONTHS);
        let pages = announcement_pages(std::slice::from_ref(&card), &[], &events, &nothing(), &today, None, 2);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].1.text, "Sunday	06.30	First service
	09.00	Second service
Wednesday	18.30	Prayer");
        assert_eq!(pages[0].1.notice, NoticeKind::Schedule);
        assert_eq!(pages[0].1.heading, "This week");
        // Saturday the 14th: the week now reaches Friday the 20th, and the 22nd is still out.
        let saturday = today_on(11, 14, &MONTHS);
        assert_eq!(week_events(&events, &saturday).len(), 3);
        // A week over New Year reads January's events too.
        let new_year = Today { year: 2026, ..today_on(12, 29, &MONTHS) };
        let found = week_events(&events, &new_year);
        assert_eq!(found.first().map(|f| f.0), Some(Date::new(2026, 12, 30).unwrap()), "Wednesday's prayer first");
        assert_eq!(found.last().map(|f| f.0), Some(Date::new(2027, 1, 3).unwrap()));
        // Nothing in the week: no slide at all.
        let none = vec![service("Once", Date::new(2027, 6, 1).unwrap(), 60, Repeat::Once)];
        assert!(announcement_pages(&[card.clone()], &[], &none, &nothing(), &today, None, 2).is_empty());
        // More than ten lines go on a second slide, which writes its first day again.
        let many: Vec<Event> = (0..12).map(|n| service(&format!("Event {n}"), Date::new(2026, 11, 15).unwrap(), 6 * 60 + n * 15, Repeat::Once)).collect();
        let pages = announcement_pages(&[card], &[], &many, &nothing(), &today, None, 2);
        assert_eq!(pages.len(), 2);
        assert!(pages[1].1.text.starts_with("Sunday	"));
        assert_eq!(pages[1].1.text.lines().count(), 2);
    }
    #[test]
    fn a_schedule_card_reads_its_calendars_and_its_window_and_counts_them() {
        use crate::calendar::Repeat;
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let mut youth = service("Youth", Date::new(2026, 11, 27).unwrap(), 19 * 60, Repeat::Once);
        youth.calendar = 2;
        let events = vec![service("Sunday", Date::new(2026, 11, 15).unwrap(), 9 * 60, Repeat::Once), youth, service("Early", Date::new(2026, 11, 2).unwrap(), 60, Repeat::Once)];
        let today = today_on(11, 13, &MONTHS);
        let count = |card: &Announcement| source_count(card, &[], &events, &nothing(), &today, None);
        let week = Announcement { fill: Fill::ScheduleWeek, ..Announcement::default() };
        assert_eq!(count(&week), Some(1));
        // The month is the whole of November, the days gone included.
        assert_eq!(count(&Announcement { month: true, ..week.clone() }), Some(3));
        assert_eq!(count(&Announcement { month: true, calendars: vec![2], ..week.clone() }), Some(1));
        assert_eq!(count(&Announcement { month: true, calendars: vec![9], ..week.clone() }), Some(0));
        // Typed words read nothing.
        assert_eq!(count(&Announcement::default()), None);
        assert_eq!(Fill::from_code(Template::Offerings.fill().code()).template(), Template::Offerings);
    }

    #[test]
    fn every_slide_of_a_card_is_drawn_in_its_style_and_title_colour() {
        let plain = Announcement { title: "Hello".into(), body: "Words".into(), style: Some(NoticeStyle::Ledger), title_colour: Some([9, 8, 7]), ..Announcement::default() };
        let old = Announcement { title: "Old".into(), ..Announcement::default() };
        let today = today_on(11, 13, &["January"; 12]);
        let pages = announcement_pages(&[plain, old], &[], &[], &nothing(), &today, None, 2);
        assert_eq!((pages[0].1.style, pages[0].1.title_colour), (Some(NoticeStyle::Ledger), Some([9, 8, 7])));
        assert_eq!((pages[1].1.style, pages[1].1.title_colour), (None, None));
        assert_eq!(NoticeStyle::from_code(NoticeStyle::Band.code()), Some(NoticeStyle::Band));
        assert_eq!(NoticeStyle::from_code(""), None);
    }
}
