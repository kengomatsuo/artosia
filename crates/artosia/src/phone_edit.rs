//! What a phone with the Edit level changes on this computer (PHONEEDIT1): the rows the phone is
//! shown, the console's lock on what its own editor holds, and the functions that apply an edit.
//! The remote server has already checked the level, the limits and the revision it was shown;
//! each apply checks the revision again against what is stored, because the computer is the truth.

use artosia_core::calendar::{Change, Date, Event, Repeat, rule_dates};
use artosia_core::deck::{Announcement, DeckSource, Fill, Person};
use artosia_remote::{Level, RemoteCard, RemoteEdit, RemoteEvent, RemotePerson, RemoteState, card_key, event_key, person_key};

use crate::calendar::held;
use crate::state::{App, Undo};
use crate::strings as t;

/// Most birthdays a phone is sent: the soonest ones.
pub const PEOPLE_SHOWN: usize = 60;
/// Days of the schedule a phone is sent, today included.
pub const WEEK: i64 = 7;

/// Why an edit was not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Changed or removed on this computer since the phone opened it.
    Old,
    /// This computer's own editor holds it with changes not saved.
    Busy,
    /// Not something that can be written: no words left, a name already listed, no calendar.
    Invalid,
}

fn kind_of(fill: Fill) -> &'static str {
    match fill {
        Fill::Nothing => "plain",
        Fill::Birthdays => "birthdays",
        Fill::Passage => "passage",
        Fill::Schedule => "schedule",
        Fill::Offerings => "offerings",
        Fill::Transfer => "transfer",
    }
}

fn card_row(card: &Announcement) -> RemoteCard {
    RemoteCard { id: card.id, title: card.title.clone(), body: card.body.clone(), body_editable: card.fill == Fill::Nothing, kind: kind_of(card.fill).to_string() }
}

fn person_row(person: &Person, today: Date) -> RemotePerson {
    RemotePerson { id: person.id, name: person.name.clone(), day: person.day, month: person.month, next: person.days_until(today) }
}

/// One date of an event as the calendar holds it, whether the rule gave it or it was moved; None
/// when the rule gives no such date or it was cancelled.
fn event_row(event: &Event, original: Date) -> Option<RemoteEvent> {
    if !event.gives(original) || event.change_of(original) == Some(&Change::Cancelled) {
        return None;
    }
    let (date, start, end, name) = held(event, original);
    Some(RemoteEvent { id: event.id, original: original.code(), name: name.unwrap_or_else(|| event.name.clone()), date: date.code(), start, end, repeats: event.repeat != Repeat::Once })
}

impl App {
    /// Whether any paired phone has the Edit level: only then are the rows worked out each tick.
    pub fn edit_phones(&self) -> bool {
        self.remote.as_ref().is_some_and(|server| server.phones().iter().any(|phone| phone.level == Level::Edit))
    }

    /// Fills the part of the remote's state that only an Edit phone is sent.
    pub fn remote_rows(&self, state: &mut RemoteState) {
        let today = self.civil_today();
        state.cards = self.announcements.iter().map(card_row).collect();
        let mut people: Vec<RemotePerson> = self.people.iter().map(|person| person_row(person, today)).collect();
        people.sort_by(|a, b| (a.next, &a.name).cmp(&(b.next, &b.name)));
        people.truncate(PEOPLE_SHOWN);
        state.people = people;
        let week_end = today.plus_days(WEEK - 1);
        let mut rows: Vec<(Date, u16, RemoteEvent)> = Vec::new();
        for event in &self.events {
            // A date moved into this week counts where the rule gave it, a month either side.
            for original in rule_dates(event, today.plus_days(-31), week_end.plus_days(31)) {
                let Some(row) = event_row(event, original) else { continue };
                let Some(date) = Date::parse(&row.date).filter(|date| *date >= today && *date <= week_end) else { continue };
                rows.push((date, row.start, row));
            }
        }
        rows.sort_by(|a, b| (a.0, a.1, &a.2.name).cmp(&(b.0, b.1, &b.2.name)));
        state.events = rows.into_iter().map(|(_, _, row)| row).collect();
        state.live_card = self.live.as_ref().and_then(|live| match live.deck.source {
            DeckSource::Announcement(id) => Some(id),
            DeckSource::Announcements => self.card_at_slide(live.index).map(|card| card.id),
            _ => None,
        });
        state.locked = self.console_holds(&state.events);
    }

    /// What this computer's own editors hold with changes not saved, as the keys a phone's save
    /// is refused under. An open event dialog holds all of that event's dates.
    fn console_holds(&self, events: &[RemoteEvent]) -> Vec<String> {
        let mut keys = Vec::new();
        if let Some(form) = self.notices.as_ref().filter(|form| form.id != 0)
            && self.notice_draft().as_ref() != self.announcements.iter().find(|card| card.id == form.id)
        {
            keys.push(card_key(form.id));
        }
        if let Some(form) = self.members.form.as_ref().filter(|form| form.id != 0)
            && let Some(stored) = self.people.iter().find(|person| person.id == form.id)
            && (form.name.text.trim(), form.day, form.month, form.year) != (stored.name.as_str(), stored.day, stored.month, stored.year)
        {
            keys.push(person_key(form.id));
        }
        if let Some(form) = self.calendar.form.as_ref().filter(|form| form.id != 0) {
            keys.extend(events.iter().filter(|row| row.id == form.id).map(|row| event_key(row.id, &row.original)));
        }
        keys
    }

    /// Applies one edit a phone sent. A refusal is said on this computer in a toast; the phone
    /// learns it from the state that follows.
    pub fn apply_remote_edit(&mut self, edit: RemoteEdit) {
        let done = match edit {
            RemoteEdit::Card { id, rev, title, body } => self.apply_card_text(id, rev, title.as_deref(), body.as_deref()),
            RemoteEdit::Person { id, rev, name, day, month } => self.apply_person(id, rev, &name, day, month),
            RemoteEdit::PersonDelete { id, rev } => self.apply_person_delete(id, rev),
            RemoteEdit::Event { id, original, rev, name, date, start, end } => self.apply_event(id, &original, rev, &name, &date, start, end),
            RemoteEdit::EventDelete { id, original, rev } => self.apply_event_delete(id, &original, rev),
        };
        match done {
            Ok(()) => {}
            Err(Refused::Busy) => self.say(t::PHONE_EDIT_BUSY(), None),
            Err(Refused::Old | Refused::Invalid) => self.say(t::PHONE_EDIT_OLD(), None),
        }
    }

    /// The title and body of a card, written as the card's editor writes them: into the library,
    /// then the decks, so the projector follows when the card is live.
    pub fn apply_card_text(&mut self, id: i64, rev: u64, title: Option<&str>, body: Option<&str>) -> Result<(), Refused> {
        let stored = self.announcements.iter().find(|card| card.id == id).ok_or(Refused::Old)?;
        if card_row(stored).rev() != rev {
            return Err(Refused::Old);
        }
        if self.notices.as_ref().is_some_and(|form| form.id == id) && self.notice_draft().as_ref() != Some(stored) {
            return Err(Refused::Busy);
        }
        let mut card = stored.clone();
        if let Some(title) = title {
            card.title = title.to_string();
        }
        if let Some(body) = body {
            // A card whose lines come from another window has no body of its own.
            if card.fill != Fill::Nothing {
                return Err(Refused::Invalid);
            }
            card.body = body.to_string();
        }
        if card.title.is_empty() && card.body.is_empty() && card.image.is_empty() && card.fill == Fill::Nothing {
            return Err(Refused::Invalid);
        }
        self.store.save_announcement(&mut card).map_err(|_| Refused::Invalid)?;
        // An editor open on the card, with nothing unsaved, shows what was written.
        if let Some(form) = self.notices.as_mut().filter(|form| form.id == id) {
            form.title = rune::TextEdit::new(card.title.clone());
            form.body = rune::TextEdit::new(card.body.clone());
        }
        self.reload_announcements();
        Ok(())
    }

    /// A birthday: id 0 adds a person, any other changes the name, day and month of that one.
    pub fn apply_person(&mut self, id: i64, rev: u64, name: &str, day: u8, month: u8) -> Result<(), Refused> {
        let today = self.civil_today();
        let mut person = if id == 0 {
            Person { id: 0, name: name.to_string(), day, month, year: 0 }
        } else {
            let stored = self.people.iter().find(|person| person.id == id).ok_or(Refused::Old)?;
            if person_row(stored, today).rev() != rev {
                return Err(Refused::Old);
            }
            if self.person_form_holds(stored) {
                return Err(Refused::Busy);
            }
            Person { name: name.to_string(), day, month, ..stored.clone() }
        };
        if self.people.iter().any(|known| known.id != person.id && crate::people::same(known, &person)) {
            return Err(Refused::Invalid);
        }
        self.store.save_person(&mut person).map_err(|_| Refused::Invalid)?;
        self.reload_people();
        Ok(())
    }

    fn person_form_holds(&self, stored: &Person) -> bool {
        self.members.form.as_ref().is_some_and(|form| form.id == stored.id && (form.name.text.trim(), form.day, form.month, form.year) != (stored.name.as_str(), stored.day, stored.month, stored.year))
    }

    /// Removes a person; the toast offers Undo as the Members tab's removal does.
    pub fn apply_person_delete(&mut self, id: i64, rev: u64) -> Result<(), Refused> {
        let today = self.civil_today();
        let stored = self.people.iter().find(|person| person.id == id).ok_or(Refused::Old)?;
        if person_row(stored, today).rev() != rev {
            return Err(Refused::Old);
        }
        if self.person_form_holds(stored) {
            return Err(Refused::Busy);
        }
        self.delete_person(id);
        Ok(())
    }

    /// One date of an event, as an override of that date; an event that happens once is
    /// changed itself, and id 0 adds one.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_event(&mut self, id: i64, original: &str, rev: u64, name: &str, date: &str, start: u16, end: u16) -> Result<(), Refused> {
        let date = Date::parse(date).ok_or(Refused::Invalid)?;
        if id == 0 {
            let calendar = self.calendars.first().map(|calendar| calendar.id).ok_or(Refused::Invalid)?;
            let mut event = Event { id: 0, calendar, name: name.to_string(), date, start, end, repeat: Repeat::Once, until: None, overrides: Vec::new() };
            self.store.save_event(&mut event).map_err(|_| Refused::Invalid)?;
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
            return Ok(());
        }
        let original = Date::parse(original).ok_or(Refused::Invalid)?;
        let event = self.events.iter().find(|event| event.id == id).ok_or(Refused::Old)?;
        if event_row(event, original).ok_or(Refused::Old)?.rev() != rev {
            return Err(Refused::Old);
        }
        if self.calendar.form.as_ref().is_some_and(|form| form.id == id) {
            return Err(Refused::Busy);
        }
        if event.repeat == Repeat::Once {
            let mut changed = event.clone();
            (changed.name, changed.date, changed.start, changed.end) = (name.to_string(), date, start, end);
            self.store.save_event(&mut changed).map_err(|_| Refused::Invalid)?;
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
            return Ok(());
        }
        // Back to what the rule gives: no override at all.
        let alike = date == original && start == event.start && end == event.end && name == event.name;
        let own_name = (name != event.name).then(|| name.to_string());
        let change = (!alike).then_some(Change::Moved { date, start, end, name: own_name });
        self.change_date(id, original, change).map(|_| ()).ok_or(Refused::Invalid)
    }

    /// Takes one date out of the calendar (the whole event when it happens once); Undo is offered.
    pub fn apply_event_delete(&mut self, id: i64, original: &str, rev: u64) -> Result<(), Refused> {
        let original = Date::parse(original).ok_or(Refused::Invalid)?;
        let event = self.events.iter().find(|event| event.id == id).ok_or(Refused::Old)?.clone();
        if event_row(&event, original).ok_or(Refused::Old)?.rev() != rev {
            return Err(Refused::Old);
        }
        if self.calendar.form.as_ref().is_some_and(|form| form.id == id) {
            return Err(Refused::Busy);
        }
        if event.repeat != Repeat::Once {
            self.cancel_date(id, original);
            return Ok(());
        }
        self.store.delete_event(id).map_err(|_| Refused::Invalid)?;
        self.events = self.store.events().unwrap_or_default();
        self.refresh_decks();
        self.say(t::event_deleted(&event.name), Some(Undo::Event(event)));
        Ok(())
    }

    /// The Phone dialog's choice for one phone.
    pub fn set_phone_level(&mut self, id: u32, level: Level) {
        if let Some(server) = &self.remote {
            server.set_level(id, level);
        }
    }

    /// The Phone dialog's second PIN switch: a number that is not the first PIN, or none.
    pub fn set_remote_edit_pin(&mut self, on: bool) {
        self.remote_edit_pin = on.then(|| loop {
            let pin = artosia_remote::new_pin();
            if pin != self.remote_pin {
                break pin;
            }
        });
        if let Some(server) = &self.remote {
            server.set_edit_pin(self.remote_edit_pin.clone());
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use artosia_core::calendar::occurrences;

    fn plain_card(app: &mut App) -> Announcement {
        let mut card = Announcement { title: "Picnic".into(), body: "Bring food".into(), seconds: 10, ..Announcement::default() };
        app.store.save_announcement(&mut card).expect("saved");
        app.reload_announcements();
        card
    }

    fn rows(app: &App) -> RemoteState {
        let mut state = RemoteState::default();
        app.remote_rows(&mut state);
        state
    }

    fn card_rev(app: &App, id: i64) -> u64 {
        rows(app).cards.iter().find(|card| card.id == id).expect("row").rev()
    }

    #[test]
    fn a_current_revision_changes_the_card_and_a_stale_one_is_refused() {
        let mut app = App::practice();
        let card = plain_card(&mut app);
        let opened = card_rev(&app, card.id);
        assert_eq!(app.apply_card_text(card.id, opened, Some("Church picnic"), None), Ok(()));
        let stored = app.announcements.iter().find(|own| own.id == card.id).expect("kept");
        assert_eq!((stored.title.as_str(), stored.body.as_str()), ("Church picnic", "Bring food"));
        // A second phone opened the card before that and now holds an old revision.
        assert_eq!(app.apply_card_text(card.id, opened, Some("Mine"), None), Err(Refused::Old));
        assert_eq!(app.announcements.iter().find(|own| own.id == card.id).map(|own| own.title.as_str()), Some("Church picnic"));
        app.apply_remote_edit(RemoteEdit::Card { id: card.id, rev: opened, title: Some("Mine".into()), body: None });
        assert_eq!(app.toast.as_ref().map(|toast| toast.text.as_str()), Some(t::PHONE_EDIT_OLD()));
    }

    #[test]
    fn the_console_wins_while_its_editor_holds_the_card_unsaved() {
        let mut app = App::practice();
        let card = plain_card(&mut app);
        app.notices = Some(App::notices_for(Some(&card)));
        let rev = card_rev(&app, card.id);
        // Open and clean: the phone's words are written, and the editor shows them.
        assert_eq!(app.apply_card_text(card.id, rev, Some("Clean"), None), Ok(()));
        assert_eq!(app.notices.as_ref().map(|form| form.title.text.clone()), Some("Clean".to_string()));
        // Typed and not saved: the phone's save is refused and nothing is written.
        app.notices.as_mut().expect("open").title = rune::TextEdit::new("Typing here");
        assert_eq!(rows(&app).locked, vec![card_key(card.id)]);
        let rev = card_rev(&app, card.id);
        app.apply_remote_edit(RemoteEdit::Card { id: card.id, rev, title: Some("Phone".into()), body: None });
        assert_eq!(app.toast.as_ref().map(|toast| toast.text.as_str()), Some(t::PHONE_EDIT_BUSY()));
        assert_eq!(app.announcements.iter().find(|own| own.id == card.id).map(|own| own.title.as_str()), Some("Clean"));
        assert_eq!(app.notices.as_ref().map(|form| form.title.text.clone()), Some("Typing here".to_string()));
    }

    #[test]
    fn a_card_with_lines_from_another_window_changes_its_title_only() {
        let mut app = App::practice();
        app.demo_notices("birthdays");
        let card = app.announcements.iter().find(|card| card.fill == Fill::Birthdays).cloned().expect("a birthday card");
        let row = rows(&app).cards.into_iter().find(|own| own.id == card.id).expect("row");
        assert!(!row.body_editable);
        assert_eq!(app.apply_card_text(card.id, row.rev(), None, Some("typed")), Err(Refused::Invalid));
        assert_eq!(app.apply_card_text(card.id, row.rev(), Some("Birthdays this week"), None), Ok(()));
    }

    #[test]
    fn a_card_cannot_lose_all_its_words() {
        let mut app = App::practice();
        let card = plain_card(&mut app);
        let rev = card_rev(&app, card.id);
        assert_eq!(app.apply_card_text(card.id, rev, Some(""), Some("")), Err(Refused::Invalid));
        assert!(app.announcements.iter().any(|own| own.id == card.id && own.title == "Picnic"));
    }

    #[test]
    fn a_live_card_edit_reaches_the_slide_the_projector_draws() {
        let mut app = App::practice();
        let card = plain_card(&mut app);
        app.show_notice_now(card.id);
        assert!(app.live.is_some());
        assert_eq!(rows(&app).live_card, Some(card.id));
        let rev = card_rev(&app, card.id);
        assert_eq!(app.apply_card_text(card.id, rev, Some("Changed on the phone"), None), Ok(()));
        let live = app.live.as_ref().expect("still live");
        assert!(live.deck.slides.iter().any(|slide| slide.heading == "Changed on the phone"), "the live deck was rebuilt from the stored card");
    }

    #[test]
    fn a_birthday_edit_keeps_the_year_and_the_list_follows() {
        let mut app = App::practice();
        let mut person = Person { name: "Sample Alpha".into(), day: 10, month: 3, year: 1980, ..Person::default() };
        app.store.save_person(&mut person).expect("saved");
        app.reload_people();
        let row = rows(&app).people.into_iter().find(|own| own.id == person.id).expect("row");
        assert_eq!(app.apply_person(person.id, row.rev(), "Sample Beta", 4, 5), Ok(()));
        let saved = app.people.iter().find(|own| own.id == person.id).expect("kept");
        assert_eq!((saved.name.as_str(), saved.day, saved.month, saved.year), ("Sample Beta", 4, 5, 1980));
        assert_eq!(app.apply_person(person.id, row.rev(), "Again", 1, 1), Err(Refused::Old), "the revision moved");
        // An add needs no revision, and a name listed twice on one day is refused.
        assert_eq!(app.apply_person(0, 0, "Sample Gamma", 9, 9), Ok(()));
        assert_eq!(app.apply_person(0, 0, "sample gamma", 9, 9), Err(Refused::Invalid));
        assert!(app.members.view.iter().any(|at| app.people[*at].name == "Sample Gamma"), "the Members table follows");
    }

    #[test]
    fn a_person_open_in_the_members_dialog_with_changes_is_not_overwritten() {
        let mut app = App::practice();
        let id = app.people[0].id;
        let row = person_row(&app.people[0], app.civil_today());
        app.edit_person(id);
        assert_eq!(app.apply_person(id, row.rev(), "Phone Name", row.day, row.month), Ok(()), "opened and untouched is not held");
        let row = person_row(app.people.iter().find(|own| own.id == id).expect("kept"), app.civil_today());
        app.members.form.as_mut().expect("open").name = rune::TextEdit::new("Typed here");
        assert_eq!(app.apply_person(id, row.rev(), "Phone Again", row.day, row.month), Err(Refused::Busy));
        assert_eq!(rows(&app).locked, vec![person_key(id)]);
    }

    #[test]
    fn deleting_a_person_offers_undo() {
        let mut app = App::practice();
        let (id, before) = (app.people[0].id, app.people.len());
        let row = person_row(&app.people[0], app.civil_today());
        assert_eq!(app.apply_person_delete(id, row.rev()), Ok(()));
        assert_eq!(app.people.len(), before - 1);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
    }

    fn weekly_event(app: &mut App) -> (i64, Date) {
        let today = app.civil_today();
        let calendar = app.calendars[0].id;
        let mask = 1u8 << today.weekday();
        let mut event = Event { id: 0, calendar, name: "Choir".into(), date: today.plus_days(-14), start: 18 * 60, end: 19 * 60, repeat: Repeat::Weekly(mask, 1), until: None, overrides: Vec::new() };
        app.store.save_event(&mut event).expect("saved");
        app.events = app.store.events().unwrap_or_default();
        (event.id, today)
    }

    #[test]
    fn an_event_of_the_week_changes_that_date_only_as_an_override() {
        let mut app = App::practice();
        let (id, today) = weekly_event(&mut app);
        let row = rows(&app).events.into_iter().find(|own| own.id == id).expect("this week's choir");
        assert_eq!((row.original.as_str(), row.repeats), (today.code().as_str(), true));
        let to = today.plus_days(1);
        assert_eq!(app.apply_event(id, &row.original, row.rev(), "Choir, new hall", &to.code(), 19 * 60, 20 * 60), Ok(()));
        let event = app.events.iter().find(|own| own.id == id).expect("kept");
        assert_eq!((event.name.as_str(), event.start, event.date), ("Choir", 18 * 60, today.plus_days(-14)), "the series itself is unchanged");
        let found = occurrences(event, today, today.plus_days(6));
        assert_eq!(found.iter().map(|own| (own.date, own.start, own.name.as_str())).collect::<Vec<_>>(), vec![(to, 19 * 60, "Choir, new hall")]);
        // The row on the phone is the moved date, under the same original, with a new revision.
        let moved = rows(&app).events.into_iter().find(|own| own.id == id).expect("still listed");
        assert_eq!((moved.original.as_str(), moved.date.as_str(), moved.name.as_str()), (row.original.as_str(), to.code().as_str(), "Choir, new hall"));
        assert_eq!(app.apply_event(id, &row.original, row.rev(), "Late", &to.code(), 1, 2), Err(Refused::Old));
        // Putting it back where the rule has it leaves no override.
        assert_eq!(app.apply_event(id, &row.original, moved.rev(), "Choir", &row.original, 18 * 60, 19 * 60), Ok(()));
        assert!(app.events.iter().find(|own| own.id == id).expect("kept").overrides.is_empty());
    }

    #[test]
    fn an_event_that_happens_once_is_changed_itself_and_can_be_added_and_deleted() {
        let mut app = App::practice();
        let today = app.civil_today();
        assert_eq!(app.apply_event(0, &today.code(), 0, "Prayer walk", &today.plus_days(2).code(), 7 * 60, 8 * 60), Ok(()));
        let row = rows(&app).events.into_iter().find(|own| own.name == "Prayer walk").expect("added and in the week");
        assert!(!row.repeats);
        assert_eq!(app.apply_event(row.id, &row.original, row.rev(), "Prayer walk", &today.plus_days(3).code(), 7 * 60, 8 * 60), Ok(()));
        let event = app.events.iter().find(|own| own.id == row.id).expect("kept");
        assert_eq!(event.date, today.plus_days(3));
        let row = rows(&app).events.into_iter().find(|own| own.id == event.id).expect("listed");
        assert_eq!(app.apply_event_delete(row.id, &row.original, row.rev()), Ok(()));
        assert!(!app.events.iter().any(|own| own.name == "Prayer walk"));
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
    }

    #[test]
    fn deleting_a_date_of_a_series_cancels_that_date() {
        let mut app = App::practice();
        let (id, today) = weekly_event(&mut app);
        let row = rows(&app).events.into_iter().find(|own| own.id == id).expect("listed");
        assert_eq!(app.apply_event_delete(id, &row.original, row.rev()), Ok(()));
        let event = app.events.iter().find(|own| own.id == id).expect("the series stays");
        assert_eq!(event.change_of(today), Some(&Change::Cancelled));
        assert!(!rows(&app).events.iter().any(|own| own.id == id && own.original == today.code()));
    }

    #[test]
    fn an_open_event_dialog_holds_its_dates() {
        let mut app = App::practice();
        let (id, _) = weekly_event(&mut app);
        let row = rows(&app).events.into_iter().find(|own| own.id == id).expect("listed");
        app.edit_event(id);
        assert_eq!(rows(&app).locked, vec![event_key(id, &row.original)]);
        assert_eq!(app.apply_event(id, &row.original, row.rev(), "Choir", &row.date, 1, 2), Err(Refused::Busy));
    }

    #[test]
    fn only_the_coming_week_and_the_soonest_birthdays_are_sent() {
        let mut app = App::practice();
        let today = app.civil_today();
        let calendar = app.calendars[0].id;
        for (name, days) in [("Today", 0), ("Week end", 6), ("Next week", 7), ("Yesterday", -1)] {
            let mut event = Event { id: 0, calendar, name: name.into(), date: today.plus_days(days), start: 600, end: 660, repeat: Repeat::Once, until: None, overrides: Vec::new() };
            app.store.save_event(&mut event).expect("saved");
        }
        app.events = app.store.events().unwrap_or_default();
        let names: Vec<String> = rows(&app).events.into_iter().map(|row| row.name).filter(|name| ["Today", "Week end", "Next week", "Yesterday"].contains(&name.as_str())).collect();
        assert_eq!(names, ["Today", "Week end"]);
        let people = rows(&app).people;
        assert!(people.len() <= PEOPLE_SHOWN && people.windows(2).all(|pair| pair[0].next <= pair[1].next));
    }

    #[test]
    fn the_rows_for_a_phone_are_worked_out_in_a_frame() {
        let mut app = App::practice();
        let many: Vec<Person> = (0..600).map(|n| Person { name: format!("Person {n:03}"), day: (n % 28 + 1) as u8, month: (n % 12 + 1) as u8, ..Person::default() }).collect();
        app.store.replace_people(&many).expect("saved");
        app.people = app.store.people().unwrap_or_default();
        let _ = plain_card(&mut app);
        let took = (0..5)
            .map(|_| {
                let started = std::time::Instant::now();
                let _ = rows(&app);
                started.elapsed()
            })
            .min()
            .unwrap_or_default();
        eprintln!("phone edit: rows for 600 people, {} cards and {} events in {took:?}", app.announcements.len(), app.events.len());
        assert!(took.as_millis() < 8, "took {took:?}");
    }

    #[test]
    fn switching_phone_off_forgets_the_second_pin() {
        let mut app = App::practice();
        app.remote_pin = "1234".into();
        app.set_remote_edit_pin(true);
        let pin = app.remote_edit_pin.clone().expect("on");
        assert_eq!(pin.len(), 4);
        assert_ne!(pin, app.remote_pin);
        app.set_remote(false);
        assert_eq!(app.remote_edit_pin, None);
    }
}
