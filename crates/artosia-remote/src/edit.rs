//! What a phone with the Edit level may change: the words of an announcement, a birthday and an
//! event of the coming week. The server only checks (level, limits, revision, the console's lock)
//! and relays; the operator applies. A revision is worked out from the fields a phone edits, so
//! any change, from any route, moves it and nothing has to be stored.

use crate::RemoteState;
use serde_json::{Value, json};

/// What a paired phone may do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    /// Slides and switches only.
    #[default]
    Control,
    /// Control, and the words and lists the operator lets it change.
    Edit,
}

impl Level {
    pub fn code(self) -> &'static str {
        match self {
            Level::Control => "control",
            Level::Edit => "edit",
        }
    }
}

/// Longest title of an announcement, in characters.
pub const MAX_TITLE_CHARS: usize = 120;
/// Longest body of an announcement, in characters.
pub const MAX_BODY_CHARS: usize = 600;
/// Longest name of a person or an event, in characters.
pub const MAX_NAME_CHARS: usize = 80;

/// An announcement as a phone may edit it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteCard {
    pub id: i64,
    pub title: String,
    pub body: String,
    /// False for a card whose lines come from another window: only its title is the phone's.
    pub body_editable: bool,
    /// "plain", "birthdays", "schedule", "passage", "offerings" or "transfer".
    pub kind: String,
}

/// A birthday as a phone may edit it. The year of birth never leaves the computer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemotePerson {
    pub id: i64,
    pub name: String,
    pub day: u8,
    pub month: u8,
    /// Days until the next birthday, 0 today; for the order and the week's mark, not part of the revision.
    pub next: u32,
}

/// One date of an event in the coming week.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteEvent {
    pub id: i64,
    /// The date the event's rule gave, `2026-11-15`: with `id` it names this date of a series.
    pub original: String,
    pub name: String,
    /// The date it is held on, `2026-11-15`.
    pub date: String,
    /// Minutes after midnight.
    pub start: u16,
    pub end: u16,
    pub repeats: bool,
}

/// The same fields in, the same number out; 48 bits so a phone's number type holds it.
pub fn revision(parts: &[&str]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for byte in part.bytes().chain(std::iter::once(0xff)) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash & 0xffff_ffff_ffff
}

impl RemoteCard {
    pub fn rev(&self) -> u64 {
        revision(&[&self.title, &self.body])
    }
}

impl RemotePerson {
    pub fn rev(&self) -> u64 {
        revision(&[&self.name, &self.day.to_string(), &self.month.to_string()])
    }
}

impl RemoteEvent {
    pub fn rev(&self) -> u64 {
        revision(&[&self.name, &self.date, &self.start.to_string(), &self.end.to_string()])
    }
}

/// A change a phone asks for, already checked against level, limits and revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteEdit {
    /// Words of a card; a field left None is not the phone's change.
    Card { id: i64, rev: u64, title: Option<String>, body: Option<String> },
    /// A person; id 0 adds one and its `rev` is not looked at.
    Person { id: i64, rev: u64, name: String, day: u8, month: u8 },
    PersonDelete { id: i64, rev: u64 },
    /// One date of an event; id 0 adds a single event on `date`.
    Event { id: i64, original: String, rev: u64, name: String, date: String, start: u16, end: u16 },
    /// Takes one date out of the calendar.
    EventDelete { id: i64, original: String, rev: u64 },
}

impl RemoteEdit {
    /// The key the console's editor is listed under in `RemoteState::locked`; None for an add.
    pub fn key(&self) -> Option<String> {
        match self {
            RemoteEdit::Card { id, .. } => Some(card_key(*id)),
            RemoteEdit::Person { id, .. } | RemoteEdit::PersonDelete { id, .. } => (*id != 0).then(|| person_key(*id)),
            RemoteEdit::Event { id, original, .. } | RemoteEdit::EventDelete { id, original, .. } => (*id != 0).then(|| event_key(*id, original)),
        }
    }
}

pub fn card_key(id: i64) -> String {
    format!("card:{id}")
}

pub fn person_key(id: i64) -> String {
    format!("person:{id}")
}

pub fn event_key(id: i64, original: &str) -> String {
    format!("event:{id}@{original}")
}

/// Why an edit was not taken.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// Not an edit this server knows, or a field past its limit.
    Bad,
    /// The phone does not have the Edit level.
    Forbidden,
    /// Someone changed it since the phone opened it; carries what it is now (null when it is gone).
    Stale(Value),
    /// The computer's own editor holds unsaved changes to it.
    Console,
}

/// What the server does with an edit that passed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Taken {
    /// Already so: a Done sent twice lands once.
    Already,
    Relay(RemoteEdit),
}

fn clean(text: &str, max: usize, lines: bool) -> Option<String> {
    if text.chars().count() > max {
        return None;
    }
    let kept: String = text.chars().map(|c| if c == '\n' && lines { c } else if c.is_control() { ' ' } else { c }).collect();
    Some(kept.trim().to_string())
}

fn valid_date(code: &str) -> bool {
    let mut parts = code.splitn(3, '-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else { return false };
    let (Ok(year), Ok(month), Ok(day)) = (year.parse::<i32>(), month.parse::<u8>(), day.parse::<u8>()) else { return false };
    code.len() == 10 && (1970..=2200).contains(&year) && days_in(month, year).is_some_and(|days| (1..=days).contains(&day))
}

fn days_in(month: u8, year: i32) -> Option<u8> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if leap { 29 } else { 28 }),
        _ => None,
    }
}

fn rev_of(v: &Value) -> Option<u64> {
    v.get("rev")?.as_u64()
}

fn minutes(v: &Value, key: &str) -> Option<u16> {
    u16::try_from(v.get(key)?.as_u64()?).ok().filter(|m| *m <= 24 * 60)
}

fn parse(v: &Value) -> Option<RemoteEdit> {
    let id = v.get("id").and_then(Value::as_i64).unwrap_or(0);
    let text = |key: &str, max: usize, lines: bool| -> Option<String> { clean(v.get(key)?.as_str()?, max, lines) };
    let optional = |key: &str, max: usize, lines: bool| -> Option<Option<String>> {
        match v.get(key) {
            None | Some(Value::Null) => Some(None),
            Some(_) => text(key, max, lines).map(Some),
        }
    };
    let original = || -> Option<String> { v.get("original").and_then(Value::as_str).filter(|d| valid_date(d)).map(str::to_string) };
    match v.get("kind")?.as_str()? {
        "card" => {
            let (title, body) = (optional("title", MAX_TITLE_CHARS, false)?, optional("body", MAX_BODY_CHARS, true)?);
            (id != 0 && (title.is_some() || body.is_some())).then_some(RemoteEdit::Card { id, rev: rev_of(v)?, title, body })
        }
        "person" => {
            let name = text("name", MAX_NAME_CHARS, false).filter(|name| !name.is_empty())?;
            let (day, month) = (u8::try_from(v.get("day")?.as_u64()?).ok()?, u8::try_from(v.get("month")?.as_u64()?).ok()?);
            days_in(month, 2024).filter(|days| (1..=*days).contains(&day))?;
            Some(RemoteEdit::Person { id, rev: if id == 0 { 0 } else { rev_of(v)? }, name, day, month })
        }
        "person_delete" => (id != 0).then_some(RemoteEdit::PersonDelete { id, rev: rev_of(v)? }),
        "event" => {
            let name = text("name", MAX_NAME_CHARS, false).filter(|name| !name.is_empty())?;
            let date = v.get("date").and_then(Value::as_str).filter(|d| valid_date(d))?.to_string();
            let (start, end) = (minutes(v, "start")?, minutes(v, "end")?);
            if end <= start {
                return None;
            }
            if id == 0 {
                return Some(RemoteEdit::Event { id, original: date.clone(), rev: 0, name, date, start, end });
            }
            Some(RemoteEdit::Event { id, original: original()?, rev: rev_of(v)?, name, date, start, end })
        }
        "event_delete" => (id != 0).then_some(RemoteEdit::EventDelete { id, original: original()?, rev: rev_of(v)? }),
        _ => None,
    }
}

impl RemoteState {
    /// What the phone sees of an item now, for the refusal that tells it what changed.
    fn current(&self, edit: &RemoteEdit) -> Option<(u64, Value)> {
        match edit {
            RemoteEdit::Card { id, .. } => self.cards.iter().find(|c| c.id == *id).map(|c| (c.rev(), json!({"title": c.title, "body": c.body}))),
            RemoteEdit::Person { id, .. } | RemoteEdit::PersonDelete { id, .. } => {
                self.people.iter().find(|p| p.id == *id).map(|p| (p.rev(), json!({"name": p.name, "day": p.day, "month": p.month})))
            }
            RemoteEdit::Event { id, original, .. } | RemoteEdit::EventDelete { id, original, .. } => self
                .events
                .iter()
                .find(|e| e.id == *id && &e.original == original)
                .map(|e| (e.rev(), json!({"name": e.name, "date": e.date, "start": e.start, "end": e.end}))),
        }
    }

    /// Whether the edit leaves the item as it already is.
    fn is_done(&self, edit: &RemoteEdit) -> bool {
        match edit {
            RemoteEdit::Card { id, title, body, .. } => {
                self.cards.iter().find(|c| c.id == *id).is_some_and(|c| title.as_ref().is_none_or(|t| *t == c.title) && body.as_ref().is_none_or(|b| *b == c.body))
            }
            RemoteEdit::Person { id, name, day, month, .. } => self.people.iter().find(|p| p.id == *id).is_some_and(|p| (&p.name, p.day, p.month) == (name, *day, *month)),
            RemoteEdit::Event { id, original, name, date, start, end, .. } => {
                self.events.iter().find(|e| e.id == *id && &e.original == original).is_some_and(|e| (&e.name, &e.date, e.start, e.end) == (name, date, *start, *end))
            }
            RemoteEdit::PersonDelete { id, .. } => !self.people.iter().any(|p| p.id == *id),
            RemoteEdit::EventDelete { id, original, .. } => !self.events.iter().any(|e| e.id == *id && &e.original == original),
        }
    }

    /// Checks an edit command's body for a phone of this level, and says what to do with it.
    pub fn take_edit(&self, level: Level, body: &Value) -> Result<Taken, Refusal> {
        if level != Level::Edit {
            return Err(Refusal::Forbidden);
        }
        let edit = parse(body).ok_or(Refusal::Bad)?;
        let adds = matches!(edit, RemoteEdit::Person { id: 0, .. } | RemoteEdit::Event { id: 0, .. });
        if adds {
            return Ok(Taken::Relay(edit));
        }
        let rev = match &edit {
            RemoteEdit::Card { rev, .. } | RemoteEdit::Person { rev, .. } | RemoteEdit::PersonDelete { rev, .. } | RemoteEdit::Event { rev, .. } | RemoteEdit::EventDelete { rev, .. } => *rev,
        };
        let Some((now, current)) = self.current(&edit) else {
            // Gone from the computer: a delete already happened, anything else is a conflict.
            return if matches!(edit, RemoteEdit::PersonDelete { .. } | RemoteEdit::EventDelete { .. }) { Ok(Taken::Already) } else { Err(Refusal::Stale(Value::Null)) };
        };
        if now != rev {
            if self.is_done(&edit) {
                return Ok(Taken::Already);
            }
            let mut current = current;
            current["rev"] = json!(now);
            return Err(Refusal::Stale(current));
        }
        if edit.key().is_some_and(|key| self.locked.contains(&key)) {
            return Err(Refusal::Console);
        }
        Ok(Taken::Relay(edit))
    }

    /// Puts a taken edit into the server's copy at once, so a second phone holding the old
    /// revision is refused before the computer has even answered. The computer's next state
    /// replaces it either way.
    pub fn apply_edit(&mut self, edit: &RemoteEdit) {
        match edit {
            RemoteEdit::Card { id, title, body, .. } => {
                if let Some(card) = self.cards.iter_mut().find(|c| c.id == *id) {
                    if let Some(title) = title {
                        card.title.clone_from(title);
                    }
                    if let Some(body) = body {
                        card.body.clone_from(body);
                    }
                }
            }
            RemoteEdit::Person { id, name, day, month, .. } => {
                if let Some(person) = self.people.iter_mut().find(|p| p.id == *id) {
                    person.name.clone_from(name);
                    (person.day, person.month) = (*day, *month);
                }
            }
            RemoteEdit::PersonDelete { id, .. } => self.people.retain(|p| p.id != *id),
            RemoteEdit::Event { id, original, name, date, start, end, .. } => {
                if let Some(event) = self.events.iter_mut().find(|e| e.id == *id && &e.original == original) {
                    event.name.clone_from(name);
                    event.date.clone_from(date);
                    (event.start, event.end) = (*start, *end);
                }
            }
            RemoteEdit::EventDelete { id, original, .. } => self.events.retain(|e| !(e.id == *id && &e.original == original)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> RemoteState {
        RemoteState {
            cards: vec![RemoteCard { id: 1, title: "Picnic".into(), body: "Bring food".into(), body_editable: true, kind: "plain".into() }],
            people: vec![RemotePerson { id: 4, name: "Sample Alpha".into(), day: 15, month: 11, next: 0 }],
            events: vec![RemoteEvent { id: 7, original: "2026-11-15".into(), name: "Service".into(), date: "2026-11-15".into(), start: 540, end: 600, repeats: true }],
            ..Default::default()
        }
    }

    fn card_edit(rev: u64, title: &str) -> Value {
        json!({"kind": "card", "id": 1, "rev": rev, "title": title})
    }

    #[test]
    fn a_revision_follows_the_fields_and_fits_a_phones_number() {
        let card = state().cards[0].clone();
        assert_eq!(card.rev(), card.rev());
        assert_ne!(card.rev(), RemoteCard { title: "Picnic!".into(), ..card.clone() }.rev());
        // The same letters split another way are another revision.
        assert_ne!(revision(&["ab", "c"]), revision(&["a", "bc"]));
        assert!(card.rev() < 1 << 48);
    }

    #[test]
    fn a_control_phone_is_refused_every_edit() {
        let s = state();
        let rev = s.cards[0].rev();
        assert_eq!(s.take_edit(Level::Control, &card_edit(rev, "New")), Err(Refusal::Forbidden));
        assert!(matches!(s.take_edit(Level::Edit, &card_edit(rev, "New")), Ok(Taken::Relay(_))));
    }

    #[test]
    fn a_current_revision_is_taken_and_a_stale_one_is_refused_with_what_is_there_now() {
        let mut s = state();
        let opened = s.cards[0].rev();
        s.cards[0].title = "Church picnic".into();
        let Err(Refusal::Stale(current)) = s.take_edit(Level::Edit, &card_edit(opened, "Mine")) else { panic!("a stale edit is refused") };
        assert_eq!(current["title"], "Church picnic");
        assert_eq!(current["body"], "Bring food");
        assert!(matches!(s.take_edit(Level::Edit, &card_edit(s.cards[0].rev(), "Mine")), Ok(Taken::Relay(RemoteEdit::Card { .. }))));
    }

    #[test]
    fn a_done_sent_twice_lands_once() {
        let mut s = state();
        let body = card_edit(s.cards[0].rev(), "Mine");
        let Ok(Taken::Relay(edit)) = s.take_edit(Level::Edit, &body) else { panic!("first is taken") };
        s.apply_edit(&edit);
        assert_eq!(s.take_edit(Level::Edit, &body), Ok(Taken::Already));
    }

    #[test]
    fn the_console_wins_while_its_editor_holds_the_item() {
        let mut s = state();
        s.locked = vec![card_key(1)];
        assert_eq!(s.take_edit(Level::Edit, &card_edit(s.cards[0].rev(), "Mine")), Err(Refusal::Console));
        // Another item is not held.
        let person = json!({"kind": "person", "id": 4, "rev": s.people[0].rev(), "name": "Sample Beta", "day": 3, "month": 2});
        assert!(matches!(s.take_edit(Level::Edit, &person), Ok(Taken::Relay(_))));
    }

    #[test]
    fn limits_are_checked_per_field() {
        let s = state();
        let rev = s.cards[0].rev();
        assert_eq!(s.take_edit(Level::Edit, &card_edit(rev, &"x".repeat(MAX_TITLE_CHARS + 1))), Err(Refusal::Bad));
        assert!(s.take_edit(Level::Edit, &card_edit(rev, &"x".repeat(MAX_TITLE_CHARS))).is_ok());
        let body = |text: &str| json!({"kind": "card", "id": 1, "rev": rev, "body": text});
        assert_eq!(s.take_edit(Level::Edit, &body(&"y".repeat(MAX_BODY_CHARS + 1))), Err(Refusal::Bad));
        // A body keeps its lines, a title is one line, and no control character gets through.
        let Ok(Taken::Relay(RemoteEdit::Card { title, body, .. })) = s.take_edit(Level::Edit, &json!({"kind": "card", "id": 1, "rev": rev, "title": "a\nb\u{7}", "body": "a\nb\u{7}c"})) else { panic!() };
        assert_eq!((title.as_deref(), body.as_deref()), (Some("a b"), Some("a\nb c")));
        // Nothing to change, an unknown kind and a missing revision are all bad.
        assert_eq!(s.take_edit(Level::Edit, &json!({"kind": "card", "id": 1, "rev": rev})), Err(Refusal::Bad));
        assert_eq!(s.take_edit(Level::Edit, &json!({"kind": "slide", "id": 1, "rev": rev})), Err(Refusal::Bad));
        assert_eq!(s.take_edit(Level::Edit, &json!({"kind": "card", "id": 1, "title": "x"})), Err(Refusal::Bad));
    }

    #[test]
    fn a_birthday_and_an_event_check_their_own_shapes() {
        let s = state();
        let person = |day: u64, month: u64, name: &str| json!({"kind": "person", "id": 4, "rev": s.people[0].rev(), "name": name, "day": day, "month": month});
        assert!(s.take_edit(Level::Edit, &person(29, 2, "A")).is_ok(), "a leap day is a birthday");
        for bad in [person(30, 2, "A"), person(0, 1, "A"), person(1, 13, "A"), person(1, 1, "  ")] {
            assert_eq!(s.take_edit(Level::Edit, &bad), Err(Refusal::Bad), "{bad}");
        }
        let rev = s.events[0].rev();
        let event = |date: &str, start: u64, end: u64| json!({"kind": "event", "id": 7, "original": "2026-11-15", "rev": rev, "name": "Service", "date": date, "start": start, "end": end});
        assert!(s.take_edit(Level::Edit, &event("2026-11-16", 600, 660)).is_ok());
        for bad in [event("2026-02-30", 600, 660), event("16 November", 600, 660), event("2026-11-16", 660, 600), event("2026-11-16", 600, 1500)] {
            assert_eq!(s.take_edit(Level::Edit, &bad), Err(Refusal::Bad), "{bad}");
        }
    }

    #[test]
    fn an_add_needs_no_revision_and_a_delete_of_what_is_gone_is_done() {
        let s = state();
        let add = json!({"kind": "person", "name": "Sample Gamma", "day": 1, "month": 1});
        assert!(matches!(s.take_edit(Level::Edit, &add), Ok(Taken::Relay(RemoteEdit::Person { id: 0, .. }))));
        assert_eq!(s.take_edit(Level::Edit, &json!({"kind": "person_delete", "id": 99, "rev": 1})), Ok(Taken::Already));
        assert_eq!(s.take_edit(Level::Edit, &json!({"kind": "card", "id": 99, "rev": 1, "title": "x"})), Err(Refusal::Stale(Value::Null)));
    }
}
