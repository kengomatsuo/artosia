//! The Calendar window's state and what its buttons do (OVCAL1). The dates themselves are worked
//! out in `artosia_core::calendar`; this holds which month is shown, the event being written and
//! the calendars dialog, and writes the library.

use artosia_core::calendar::{Calendar, Date, Event, Position, Repeat, hour_minute, upcoming};
use rune::TextEdit;

use crate::state::{App, Undo};
use crate::strings as t;

/// The place of the template whose title a schedule card starts with, among `ANNOUNCEMENT_TEMPLATES`, in every language.
pub const SCHEDULE_TEMPLATE: usize = 2;
/// Minutes between two entries of a time list.
pub const STEP: u16 = 15;
/// The last entry of a time list, 23.45.
const LAST_TIME: u16 = 24 * 60 - STEP;
/// How many dates the dialog lists under its sentence.
pub const LISTED: usize = 6;

/// The part of the event dialog that has a list or a month open under it. One at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    None,
    Calendar,
    Start,
    End,
    Which,
    Weekday,
    First,
    Until,
}

/// How an event repeats, as the dialog's three-way switch has it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Once,
    Week,
    Month,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Once, Kind::Week, Kind::Month];
}

/// The event in the dialog: a new one (`id` 0) or a saved one being changed.
pub struct EventForm {
    pub id: i64,
    pub name: TextEdit,
    pub calendar: i64,
    pub kind: Kind,
    /// Bit 0 is Sunday.
    pub days: u8,
    pub position: Position,
    pub weekday: u8,
    pub start: u16,
    pub end: u16,
    pub date: Date,
    pub until: Option<Date>,
    pub skipped: Vec<Date>,
    pub open: Part,
    /// The month the open month picker shows.
    pub shown: Date,
}

impl EventForm {
    /// The rule as an event, for the sentence and the dates under it.
    pub fn event(&self) -> Event {
        let repeat = match self.kind {
            Kind::Once => Repeat::Once,
            Kind::Week => Repeat::Weekly(self.days),
            Kind::Month => Repeat::Monthly(self.position, self.weekday),
        };
        let until = if self.kind == Kind::Once { None } else { self.until };
        Event { id: self.id, calendar: self.calendar, name: self.name.text.trim().to_string(), date: self.date, start: self.start, end: self.end, repeat, until, skipped: self.skipped.clone() }
    }

    /// The weekday and week of the month a date falls on, as a monthly rule would name them.
    fn place_of(date: Date) -> (Position, u8) {
        let position = if date.day + 7 > Date::month_days(date.year, date.month) { Position::Last } else { Position::Nth((date.day - 1) / 7 + 1) };
        (position, date.weekday())
    }

    /// Whether the form can be saved: a name, a calendar and, for a weekly event, a day.
    pub fn ready(&self) -> bool {
        !self.name.text.trim().is_empty() && self.calendar != 0 && !(self.kind == Kind::Week && self.days == 0)
    }
}

/// The Calendar tab and what its dialogs hold.
pub struct Window {
    /// A day inside the month or week on show.
    pub anchor: Date,
    pub week: bool,
    /// Calendars whose events are not drawn. Not saved: the window opens showing everything.
    pub hidden: Vec<i64>,
    pub form: Option<EventForm>,
    /// The calendars dialog is open, on this calendar.
    pub manage: Option<i64>,
    /// The name of the calendar picked in that dialog.
    pub rename: TextEdit,
    /// A calendar's colour moved and is not written to the library yet.
    pub dirty: bool,
}

impl Default for Window {
    fn default() -> Window {
        Window { anchor: Date { year: 1970, month: 1, day: 1 }, week: false, hidden: Vec::new(), form: None, manage: None, rename: TextEdit::new(""), dirty: false }
    }
}

/// The colours new calendars take in turn: OKLCH at one lightness and strength, six hues.
pub fn next_colour(taken: usize) -> [u8; 3] {
    let hues = [255.0, 150.0, 55.0, 300.0, 0.0, 200.0];
    crate::ui::colour::oklch_to_rgb(0.66, 0.15, hues[taken % hues.len()])
}

/// The name of a weekday by its number, 0 for Sunday; the window lists its names from Monday.
pub fn weekday_name(weekday: u8) -> &'static str {
    t::WEEKDAYS()[usize::from((weekday + 6) % 7)]
}

pub fn position_name(position: Position) -> &'static str {
    match position {
        Position::Nth(1) => t::CAL_POS_FIRST(),
        Position::Nth(2) => t::CAL_POS_SECOND(),
        Position::Nth(3) => t::CAL_POS_THIRD(),
        Position::Nth(_) => t::CAL_POS_FOURTH(),
        Position::Last => t::CAL_POS_LAST(),
    }
}

pub fn time_words(minutes: u16) -> String {
    let (hour, minute) = hour_minute(minutes);
    t::time_of_day(hour, minute)
}

pub fn date_words(date: Date) -> String {
    t::date_long(date.day, t::MONTHS()[usize::from(date.month - 1)], date.year)
}

/// The rule said back as a sentence.
pub fn rule_words(event: &Event) -> String {
    let (start, end, from) = (time_words(event.start), time_words(event.end), date_words(event.date));
    let until = event.until.map(date_words);
    match event.repeat {
        Repeat::Once => t::rule_once(&from, &start, &end),
        Repeat::Weekly(mask) => {
            let days: Vec<&str> = (1..=7).map(|n| n % 7).filter(|day| Repeat::on_weekday(mask, *day)).map(weekday_name).collect();
            t::rule_week(&t::weekday_list(&days), &start, &end, &from, until.as_deref())
        }
        Repeat::Monthly(position, weekday) => t::rule_month(position_name(position), weekday_name(weekday), &start, &end, &from, until.as_deref()),
    }
}

impl App {
    /// Today on this computer's clock.
    pub fn civil_today(&self) -> Date {
        let time = rune::LocalTime::at((self.now * 1000.0) as i64);
        Date::new(time.year, time.month.clamp(1, 12) as u8, 1).map(|first| Date { day: (time.day as u8).clamp(1, Date::month_days(first.year, first.month)), ..first }).unwrap_or(Date { year: 1970, month: 1, day: 1 })
    }

    /// The calendars and events as the library holds them, and a first calendar for a library with none.
    pub fn load_calendar(&mut self) {
        if self.store.calendars().map(|calendars| calendars.is_empty()).unwrap_or(false) {
            let mut services = Calendar { id: 0, name: t::CAL_SERVICES().to_string(), colour: next_colour(0) };
            let _ = self.store.save_calendar(&mut services);
        }
        self.calendars = self.store.calendars().unwrap_or_default();
        self.events = self.store.events().unwrap_or_default();
        self.calendar.anchor = self.civil_today();
    }

    pub fn calendar_of(&self, id: i64) -> Option<&Calendar> {
        self.calendars.iter().find(|calendar| calendar.id == id)
    }

    /// The month or week `by` places on.
    pub fn calendar_move(&mut self, by: i32) {
        let anchor = self.calendar.anchor;
        self.calendar.anchor = if self.calendar.week { anchor.plus_days(7 * i64::from(by)) } else { anchor.first_of_month().plus_months(by) };
    }

    pub fn calendar_today(&mut self) {
        self.calendar.anchor = self.civil_today();
    }

    pub fn calendar_show_week(&mut self, week: bool) {
        self.calendar.week = week;
    }

    /// A tick of the rail: the calendar's events are drawn or not.
    pub fn calendar_toggle(&mut self, id: i64) {
        match self.calendar.hidden.iter().position(|hidden| *hidden == id) {
            Some(at) => drop(self.calendar.hidden.remove(at)),
            None => self.calendar.hidden.push(id),
        }
    }

    /// A new event on `date`, in the first calendar that is drawn.
    pub fn new_event(&mut self, date: Date) {
        let calendar = self.calendars.iter().find(|calendar| !self.calendar.hidden.contains(&calendar.id)).or(self.calendars.first()).map_or(0, |calendar| calendar.id);
        let (position, weekday) = EventForm::place_of(date);
        let mut name = TextEdit::new("");
        name.focused = true;
        self.calendar.form = Some(EventForm { id: 0, name, calendar, kind: Kind::Once, days: 1 << date.weekday(), position, weekday, start: 9 * 60, end: 10 * 60, date, until: None, skipped: Vec::new(), open: Part::None, shown: date });
        self.close_palette();
    }

    pub fn edit_event(&mut self, id: i64) {
        let Some(event) = self.events.iter().find(|event| event.id == id) else { return };
        let (kind, days, position, weekday) = match event.repeat {
            Repeat::Once => (Kind::Once, 1 << event.date.weekday(), EventForm::place_of(event.date).0, event.date.weekday()),
            Repeat::Weekly(mask) => (Kind::Week, mask, EventForm::place_of(event.date).0, event.date.weekday()),
            Repeat::Monthly(position, weekday) => (Kind::Month, 1 << weekday, position, weekday),
        };
        let mut name = TextEdit::new(event.name.clone());
        name.focused = true;
        self.calendar.form = Some(EventForm { id, name, calendar: event.calendar, kind, days, position, weekday, start: event.start, end: event.end, date: event.date, until: event.until, skipped: event.skipped.clone(), open: Part::None, shown: event.date });
    }

    pub fn close_event(&mut self) {
        self.calendar.form = None;
    }

    /// Opens or closes one of the dialog's lists; the month pickers open on the month of their date.
    pub fn event_open(&mut self, part: Part) {
        let Some(form) = self.calendar.form.as_mut() else { return };
        form.open = if form.open == part { Part::None } else { part };
        match form.open {
            Part::First => form.shown = form.date,
            Part::Until => form.shown = form.until.unwrap_or(form.date),
            _ => {}
        }
    }

    pub fn event_close_part(&mut self) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.open = Part::None;
        }
    }

    pub fn event_kind(&mut self, kind: Kind) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.kind = kind;
            form.open = Part::None;
            match kind {
                Kind::Week if form.days == 0 => form.days = 1 << form.date.weekday(),
                Kind::Month => (form.position, form.weekday) = EventForm::place_of(form.date),
                _ => {}
            }
        }
    }

    pub fn event_toggle_day(&mut self, weekday: u8) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.days ^= 1 << weekday;
        }
    }

    /// The start moves, and the end goes with it when it would come before.
    pub fn event_start(&mut self, minutes: u16) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.start = minutes.min(LAST_TIME);
            if form.end < form.start {
                form.end = (form.start + 60).min(LAST_TIME);
            }
            form.open = Part::None;
        }
    }

    pub fn event_end(&mut self, minutes: u16) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.end = minutes.min(LAST_TIME).max(form.start);
            form.open = Part::None;
        }
    }

    pub fn event_calendar(&mut self, id: i64) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.calendar = id;
            form.open = Part::None;
        }
    }

    /// The month picker on show goes `by` months on.
    pub fn event_month(&mut self, by: i32) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.shown = form.shown.first_of_month().plus_months(by);
        }
    }

    /// A day of the month picker. The first date moves a weekly or monthly rule's own day with it
    /// when the rule was only the day the date had.
    pub fn event_pick_date(&mut self, date: Date) {
        let Some(form) = self.calendar.form.as_mut() else { return };
        if form.open == Part::Until {
            form.until = Some(date.max(form.date));
        } else {
            if form.kind == Kind::Week && form.days == 1 << form.date.weekday() {
                form.days = 1 << date.weekday();
            }
            if form.kind == Kind::Month && (form.position, form.weekday) == EventForm::place_of(form.date) {
                (form.position, form.weekday) = EventForm::place_of(date);
            }
            form.date = date;
            if form.until.is_some_and(|until| until < date) {
                form.until = Some(date);
            }
        }
        form.open = Part::None;
    }

    pub fn event_no_end(&mut self) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.until = None;
            form.open = Part::None;
        }
    }

    pub fn event_position(&mut self, position: Position) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.position = position;
            form.open = Part::None;
        }
    }

    pub fn event_weekday(&mut self, weekday: u8) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.weekday = weekday % 7;
            form.open = Part::None;
        }
    }

    /// Leaves a date out of the rule, or brings it back.
    pub fn event_skip(&mut self, date: Date) {
        if let Some(form) = self.calendar.form.as_mut() {
            match form.skipped.iter().position(|skipped| *skipped == date) {
                Some(at) => drop(form.skipped.remove(at)),
                None => form.skipped.push(date),
            }
        }
    }

    /// The dates the dialog lists: the next ones the rule gives from today, or from the first date.
    pub fn event_dates(&self) -> Vec<Date> {
        let Some(form) = self.calendar.form.as_ref() else { return Vec::new() };
        upcoming(&form.event(), self.civil_today().max(form.date), LISTED)
    }

    pub fn save_event(&mut self) {
        let Some(form) = self.calendar.form.as_ref().filter(|form| form.ready()) else { return };
        let mut event = form.event();
        // A skipped date the rule no longer gives is dropped, so the list does not grow for ever.
        let given = artosia_core::calendar::rule_dates(&event, event.date, event.date.plus_days(5 * 366));
        event.skipped.retain(|date| given.contains(date));
        if self.store.save_event(&mut event).is_ok() {
            self.events = self.store.events().unwrap_or_default();
            self.calendar.form = None;
            self.refresh_decks();
        }
    }

    pub fn delete_event(&mut self) {
        let Some(id) = self.calendar.form.as_ref().map(|form| form.id) else { return };
        self.calendar.form = None;
        let Some(event) = self.events.iter().find(|event| event.id == id).cloned() else { return };
        if self.store.delete_event(id).is_ok() {
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
            self.say(t::event_deleted(&event.name), Some(Undo::Event(event)));
        }
    }

    pub fn restore_event(&mut self, mut event: Event) {
        event.id = 0;
        if self.store.save_event(&mut event).is_ok() {
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
        }
    }

    // ---- Calendars ----

    pub fn open_manage(&mut self) {
        let id = self.calendars.first().map_or(0, |calendar| calendar.id);
        self.pick_calendar(id);
    }

    pub fn pick_calendar(&mut self, id: i64) {
        self.finish_colour(true);
        self.calendar.rename = TextEdit::new(self.calendar_of(id).map(|calendar| calendar.name.clone()).unwrap_or_default());
        self.calendar.rename.focused = true;
        self.calendar.manage = Some(id);
    }

    pub fn close_manage(&mut self) {
        self.finish_colour(true);
        self.calendar.manage = None;
    }

    /// Esc or the dialog's own close: an open colour picker is cancelled first, as it is everywhere,
    /// and the dialog closes on the next press.
    pub fn escape_manage(&mut self) {
        if self.colour.open.is_some() {
            self.finish_colour(false);
        } else {
            self.close_manage();
        }
    }

    pub fn add_calendar(&mut self) {
        let mut calendar = Calendar { id: 0, name: t::CAL_NEW_CALENDAR().to_string(), colour: next_colour(self.calendars.len()) };
        if self.store.save_calendar(&mut calendar).is_ok() {
            self.calendars = self.store.calendars().unwrap_or_default();
            let id = calendar.id;
            self.pick_calendar(id);
            let end = self.calendar.rename.text.len();
            self.calendar.rename.set_selection(0, end);
        }
    }

    /// The name typed in the calendars dialog, kept as it is typed; an empty name waits for a letter.
    pub fn rename_calendar(&mut self) {
        let Some(id) = self.calendar.manage else { return };
        let name = self.calendar.rename.text.trim().to_string();
        if let Some(mut calendar) = self.calendar_of(id).cloned().filter(|calendar| !name.is_empty() && calendar.name != name) {
            calendar.name = name;
            if self.store.save_calendar(&mut calendar).is_ok() {
                self.calendars = self.store.calendars().unwrap_or_default();
            }
        }
    }

    /// Takes a calendar out with its events; Undo puts both back.
    pub fn delete_calendar(&mut self) {
        let Some(id) = self.calendar.manage else { return };
        self.finish_colour(true);
        let Some(calendar) = self.calendar_of(id).cloned() else { return };
        let events: Vec<Event> = self.events.iter().filter(|event| event.calendar == id).cloned().collect();
        if self.store.delete_calendar(id).is_ok() {
            self.calendars = self.store.calendars().unwrap_or_default();
            self.events = self.store.events().unwrap_or_default();
            self.calendar.hidden.retain(|hidden| *hidden != id);
            match self.calendars.first().map(|first| first.id) {
                Some(next) => self.pick_calendar(next),
                None => self.calendar.manage = None,
            }
            self.refresh_decks();
            self.say(t::calendar_deleted(&calendar.name), Some(Undo::Calendar(calendar, events)));
        }
    }

    pub fn restore_calendar(&mut self, mut calendar: Calendar, events: Vec<Event>) {
        calendar.id = 0;
        if self.store.save_calendar(&mut calendar).is_ok() {
            for mut event in events {
                event.id = 0;
                event.calendar = calendar.id;
                let _ = self.store.save_event(&mut event);
            }
            self.calendars = self.store.calendars().unwrap_or_default();
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
        }
    }

    /// Writes a calendar whose colour moved in the picker, once a frame at most.
    pub fn save_calendar_colours(&mut self) {
        self.calendar.dirty = false;
        for mut calendar in self.calendars.clone() {
            let _ = self.store.save_calendar(&mut calendar);
        }
    }

    /// Ctrl N on the Calendar tab: a new event on the day shown, or today.
    pub fn new_event_here(&mut self) {
        let today = self.civil_today();
        let anchor = self.calendar.anchor;
        let date = if anchor.year == today.year && anchor.month == today.month || self.calendar.week { anchor } else { anchor.first_of_month() };
        self.new_event(date);
    }

    /// The first day of the grid of the month on show: the Sunday on or before the 1st.
    pub fn month_start(&self) -> Date {
        let first = self.calendar.anchor.first_of_month();
        first.plus_days(-i64::from(first.weekday()))
    }

    /// The Sunday that starts the week on show.
    pub fn week_start(&self) -> Date {
        self.calendar.anchor.plus_days(-i64::from(self.calendar.anchor.weekday()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::colour::ColourFor;
    use artosia_core::calendar::occurrences;
    use artosia_core::deck::{Announcement, Fill};

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).expect("a real date")
    }

    #[test]
    fn the_practice_calendar_holds_the_week_a_church_keeps() {
        let app = App::practice();
        assert_eq!(app.calendars.iter().map(|calendar| calendar.name.as_str()).collect::<Vec<_>>(), ["Ibadah umum", "Sekolah Minggu", "Pemuda", "Doa", "Musik"]);
        let on = |month: u8, day: u8| -> Vec<String> {
            let mut found: Vec<(u16, String)> = app.events.iter().filter(|event| !occurrences(event, date(2026, month, day), date(2026, month, day)).is_empty()).map(|event| (event.start, event.name.clone())).collect();
            found.sort();
            found.into_iter().map(|(_, name)| name).collect()
        };
        // Sunday 1 November: both services, Sunday school and the first Sunday's communion.
        assert_eq!(on(11, 1), ["Ibadah Raya 1", "Ibadah Raya 2", "Sekolah Minggu", "Perjamuan kudus"]);
        assert_eq!(on(11, 8), ["Ibadah Raya 1", "Ibadah Raya 2", "Sekolah Minggu"]);
        assert_eq!(on(11, 4), ["Ibadah doa"]);
        assert_eq!(on(11, 6), ["Persekutuan pemuda"]);
        assert_eq!(on(11, 7), ["Latihan pujian"]);
        assert!(on(11, 9).is_empty());
    }

    #[test]
    fn the_event_dialog_creates_edits_skips_and_deletes() {
        let mut app = App::practice();
        let before = app.events.len();
        app.new_event(date(2026, 11, 20));
        assert!(!app.calendar.form.as_ref().is_some_and(EventForm::ready), "no name yet");
        app.save_event();
        assert!(app.calendar.form.is_some() && app.events.len() == before, "a form with no name is not saved");
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Choir night");
        }
        app.event_kind(Kind::Week);
        app.event_start(18 * 60);
        assert_eq!(app.calendar.form.as_ref().map(|form| form.end), Some(19 * 60), "the end follows a later start");
        app.save_event();
        assert!(app.calendar.form.is_none());
        let id = app.events.iter().find(|event| event.name == "Choir night").map(|event| event.id).expect("saved");
        assert_eq!(app.events.last().map(|event| event.repeat), Some(Repeat::Weekly(1 << 5)), "Friday, the day the date has");
        // Edit: skip the first date the dialog lists, and save.
        app.edit_event(id);
        let first = app.event_dates()[0];
        app.event_skip(first);
        app.save_event();
        let saved = app.events.iter().find(|event| event.id == id).expect("kept");
        assert_eq!(saved.skipped, vec![first]);
        assert!(occurrences(saved, first, first).is_empty());
        // Bringing it back in the dialog drops it from the event.
        app.edit_event(id);
        app.event_skip(first);
        app.save_event();
        assert!(app.events.iter().find(|event| event.id == id).is_some_and(|event| event.skipped.is_empty()));
        // Delete, and Undo brings the event back.
        app.edit_event(id);
        app.delete_event();
        assert_eq!(app.events.len(), before);
        app.undo();
        assert!(app.events.iter().any(|event| event.name == "Choir night"));
    }

    #[test]
    fn a_weekly_event_needs_a_day_and_the_last_date_never_comes_before_the_first() {
        let mut app = App::practice();
        app.new_event(date(2026, 11, 14));
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Walk");
        }
        app.event_kind(Kind::Week);
        app.event_toggle_day(date(2026, 11, 14).weekday());
        assert!(app.calendar.form.as_ref().is_some_and(|form| form.days == 0 && !form.ready()), "a weekly event with no day cannot be saved");
        app.event_toggle_day(2);
        assert!(app.calendar.form.as_ref().is_some_and(EventForm::ready));
        app.event_open(Part::Until);
        app.event_pick_date(date(2026, 11, 1));
        assert_eq!(app.calendar.form.as_ref().and_then(|form| form.until), Some(date(2026, 11, 14)));
        // Moving the first date past the last drags the last along.
        app.event_open(Part::First);
        app.event_pick_date(date(2026, 12, 2));
        assert_eq!(app.calendar.form.as_ref().and_then(|form| form.until), Some(date(2026, 12, 2)));
    }

    #[test]
    fn a_calendar_takes_its_colour_from_the_one_picker_and_goes_with_its_events() {
        let mut app = App::practice();
        let music = app.calendars.iter().find(|calendar| calendar.name == "Musik").map(|calendar| (calendar.id, calendar.colour)).expect("practice calendar");
        app.toggle_colour(ColourFor::Calendar(music.0));
        app.pick_colour([1, 2, 3]);
        assert_eq!(app.calendar_of(music.0).map(|calendar| calendar.colour), Some([1, 2, 3]));
        assert!(app.calendar.dirty);
        app.finish_colour(false);
        assert_eq!(app.calendar_of(music.0).map(|calendar| calendar.colour), Some(music.1), "Esc puts the colour back");
        app.toggle_colour(ColourFor::Calendar(music.0));
        app.pick_colour([4, 5, 6]);
        app.finish_colour(true);
        app.save_calendar_colours();
        assert_eq!(app.store.calendars().unwrap_or_default().iter().find(|calendar| calendar.id == music.0).map(|calendar| calendar.colour), Some([4, 5, 6]));
        // Deleting a calendar takes its events; Undo puts both back.
        app.open_manage();
        app.pick_calendar(music.0);
        let rehearsals = app.events.iter().filter(|event| event.calendar == music.0).count();
        assert!(rehearsals > 0);
        app.delete_calendar();
        assert!(app.calendar_of(music.0).is_none() && app.events.iter().all(|event| event.calendar != music.0));
        app.undo();
        let back = app.calendars.iter().find(|calendar| calendar.name == "Musik").expect("restored");
        assert_eq!(app.events.iter().filter(|event| event.calendar == back.id).count(), rehearsals);
    }

    #[test]
    fn a_library_with_no_calendar_gets_one_named_for_services() {
        let mut app = App::practice();
        for calendar in app.calendars.clone() {
            let _ = app.store.delete_calendar(calendar.id);
        }
        app.load_calendar();
        assert_eq!(app.calendars.iter().map(|calendar| calendar.name.as_str()).collect::<Vec<_>>(), [t::CAL_SERVICES()]);
        assert!(app.events.is_empty());
    }

    #[test]
    fn a_schedule_card_reads_the_calendar_with_nothing_typed() {
        let mut app = App::practice();
        let card = Announcement { title: "This week".into(), fill: Fill::ScheduleWeek, ..Announcement::default() };
        let slides = app.card_slides(&card);
        assert_eq!(slides.len(), 1);
        let expected = artosia_core::deck::week_events(&app.events, &app.today()).len();
        assert_eq!(slides[0].text.lines().count(), expected);
        assert!(expected >= 6, "a week of the practice calendar holds the services, prayer, youth and rehearsal");
        // Changing an event changes the slide; hiding a calendar in the window does not.
        let event = app.events.iter().find(|event| event.name == "Ibadah doa").map(|event| event.id).expect("practice event");
        app.edit_event(event);
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Doa malam");
        }
        app.save_event();
        assert!(app.card_slides(&card)[0].text.contains("Doa malam"));
        let hidden = app.calendars[0].id;
        app.calendar_toggle(hidden);
        assert_eq!(app.card_slides(&card)[0].text.lines().count(), expected);
    }

    #[test]
    fn a_week_that_crosses_a_month_or_new_year_is_titled_by_both_ends() {
        assert_eq!(t::week_title((29, "November", 2026), (5, "December", 2026)), "29 November to 5 December 2026");
        assert_eq!(t::week_title((28, "December", 2026), (3, "January", 2027)), "28 December 2026 to 3 January 2027");
    }

    #[test]
    fn the_month_and_week_page_by_their_own_unit() {
        let mut app = App::practice();
        app.calendar.anchor = date(2026, 12, 31);
        app.calendar_move(1);
        assert_eq!(app.calendar.anchor, date(2027, 1, 1), "a month on starts at the first");
        app.calendar_show_week(true);
        app.calendar_move(-1);
        assert_eq!(app.calendar.anchor, date(2026, 12, 25));
        assert_eq!(app.week_start(), date(2026, 12, 20), "Sunday first");
        app.calendar_show_week(false);
        app.calendar.anchor = date(2027, 2, 10);
        assert_eq!(app.month_start(), date(2027, 1, 31), "February 2027 starts on a Monday, so the grid opens on the Sunday before");
    }
}
