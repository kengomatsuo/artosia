//! The Calendar window's state and what its buttons do (OVCAL1, CALREPEAT1, CALEXCEPT1, CALDRAG1).
//! The dates themselves are worked out in `artosia_core::calendar`; this holds which month is
//! shown, the event being written and the calendars dialog, and writes the library.

use artosia_core::calendar::{Calendar, Change, Date, Event, Position, Repeat, hour_minute, place_of, rule_dates, upcoming};
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
/// The most weeks or months a custom repeat counts.
pub const MOST_EVERY: u8 = 12;
/// Points a chip is dragged down in the week for one step of 15 minutes.
pub const QUARTER: f32 = 12.0;
/// Points a chip travels before a press on it becomes a drag.
pub const SLOP: f32 = 4.0;

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
    /// The list of how the event repeats.
    Repeat,
    /// The list of how many weeks or months a custom repeat counts.
    Every,
}

/// How an event repeats: once, or by the week or the month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Once,
    Week,
    Month,
}

/// Which of the dates of a repeating event a change is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    This,
    Following,
    All,
}

/// The event in the dialog: a new one (`id` 0) or a saved one being changed.
pub struct EventForm {
    pub id: i64,
    pub name: TextEdit,
    pub calendar: i64,
    pub kind: Kind,
    /// Every this many weeks or months, from 1.
    pub every: u8,
    /// The repeat list says Custom, whatever `every` is.
    pub custom: bool,
    /// Bit 0 is Sunday.
    pub days: u8,
    pub position: Position,
    pub weekday: u8,
    pub start: u16,
    pub end: u16,
    /// The date being edited: an event's own date, or the date of the series the dialog was opened on.
    pub date: Date,
    /// A series' first date, which only All events moves.
    pub first: Date,
    /// The date the rule gave for the date the dialog was opened on; None for a new or one-time event.
    pub target: Option<Date>,
    pub until: Option<Date>,
    pub overrides: Vec<artosia_core::calendar::Override>,
    /// The question "This event, This and following, All events" is on screen.
    pub asking: bool,
    pub open: Part,
    /// The month the open month picker shows.
    pub shown: Date,
}

impl EventForm {
    /// The rule as an event, for the sentence and the dates under it.
    pub fn event(&self) -> Event {
        let every = self.every.max(1);
        let repeat = match self.kind {
            Kind::Once => Repeat::Once,
            Kind::Week => Repeat::Weekly(self.days, every),
            Kind::Month => Repeat::Monthly(self.position, self.weekday, every),
        };
        let until = if self.kind == Kind::Once { None } else { self.until };
        let date = if self.target.is_some() { self.first } else { self.date };
        Event { id: self.id, calendar: self.calendar, name: self.name.text.trim().to_string(), date, start: self.start, end: self.end, repeat, until, overrides: self.overrides.clone() }
    }

    /// The weekday and week of the month a date falls on, as a monthly rule would name them.
    fn place_of(date: Date) -> (Position, u8) {
        place_of(date)
    }

    /// Whether the form can be saved: a name, a calendar and, for a weekly event, a day.
    pub fn ready(&self) -> bool {
        !self.name.text.trim().is_empty() && self.calendar != 0 && !(self.kind == Kind::Week && self.days == 0)
    }

    /// The place in the repeat list: Once, Every week, Every 2 weeks, Every month, Every 2 months, Custom.
    pub fn choice(&self) -> usize {
        match (self.kind, self.every, self.custom) {
            (Kind::Once, ..) => 0,
            (_, _, true) => 5,
            (Kind::Week, 1, _) => 1,
            (Kind::Week, 2, _) => 2,
            (Kind::Month, 1, _) => 3,
            (Kind::Month, 2, _) => 4,
            _ => 5,
        }
    }
}

/// A chip carried to another day (and, in the week, another time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalDrag {
    pub id: i64,
    pub original: Date,
    /// The day it was picked up on and the minute it started then.
    pub from: Date,
    pub start0: u16,
    pub to: Date,
    pub start: u16,
}

/// The menu a right press on a chip opened, at that point of the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChipMenu {
    pub id: i64,
    pub original: Date,
    pub at: [f32; 2],
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
    pub drag: Option<CalDrag>,
    /// A chip was just dropped, so the press that ends the same gesture opens nothing.
    pub dropped: bool,
    pub menu: Option<ChipMenu>,
}

impl Default for Window {
    fn default() -> Window {
        Window { anchor: Date { year: 1970, month: 1, day: 1 }, week: false, hidden: Vec::new(), form: None, manage: None, rename: TextEdit::new(""), dirty: false, drag: None, dropped: false, menu: None }
    }
}

/// How many whole cells of `cell` points a pointer `at` points from a cell's top-left is from it,
/// negative to the left and above.
pub fn cells_from(at: [f32; 2], cell: [f32; 2]) -> (i32, i32) {
    ((at[0] / cell[0].max(1.0)).floor() as i32, (at[1] / cell[1].max(1.0)).floor() as i32)
}

/// The start of an event carried `dy` points down from `start`, in steps of 15 minutes inside the day.
pub fn snapped_start(start: u16, dy: f32) -> u16 {
    let steps = (dy / QUARTER).round() as i32;
    (i32::from(start) + steps * i32::from(STEP)).clamp(0, i32::from(LAST_TIME)) as u16
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
        Repeat::Weekly(mask, every) => {
            let days: Vec<&str> = (1..=7).map(|n| n % 7).filter(|day| Repeat::on_weekday(mask, *day)).map(weekday_name).collect();
            let days = t::weekday_list(&days);
            if every > 1 { t::rule_weeks(every, &days, &start, &end, &from, until.as_deref()) } else { t::rule_week(&days, &start, &end, &from, until.as_deref()) }
        }
        Repeat::Monthly(position, weekday, every) => {
            if every > 1 {
                t::rule_months(every, position_name(position), weekday_name(weekday), &start, &end, &from, until.as_deref())
            } else {
                t::rule_month(position_name(position), weekday_name(weekday), &start, &end, &from, until.as_deref())
            }
        }
    }
}

/// The calendar day a unix time in seconds falls on, on this computer's clock.
pub fn civil_at(seconds: f64) -> Date {
    let time = rune::LocalTime::at((seconds * 1000.0) as i64);
    Date::new(time.year, time.month.clamp(1, 12) as u8, 1).map(|first| Date { day: (time.day as u8).clamp(1, Date::month_days(first.year, first.month)), ..first }).unwrap_or(Date { year: 1970, month: 1, day: 1 })
}

/// The start, end and own name of a date of an event as the calendar shows it.
pub(crate) fn held(event: &Event, original: Date) -> (Date, u16, u16, Option<String>) {
    match event.change_of(original) {
        Some(Change::Moved { date, start, end, name }) => (*date, *start, *end, name.clone()),
        _ => (original, event.start, event.end, None),
    }
}

impl App {
    /// Today on this computer's clock.
    pub fn civil_today(&self) -> Date {
        civil_at(self.now)
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
        self.calendar.form = Some(EventForm { id: 0, name, calendar, kind: Kind::Once, every: 1, custom: false, days: 1 << date.weekday(), position, weekday, start: 9 * 60, end: 10 * 60, date, first: date, target: None, until: None, overrides: Vec::new(), asking: false, open: Part::None, shown: date });
        self.close_palette();
    }

    /// The dialog on an event, on the first date its rule gives.
    pub fn edit_event(&mut self, id: i64) {
        let Some(event) = self.events.iter().find(|event| event.id == id) else { return };
        let first = rule_dates(event, event.date, event.date.plus_days(400)).first().copied().unwrap_or(event.date);
        self.edit_date(id, first);
    }

    /// The dialog on one date of an event, `original` being the date its rule gave.
    pub fn edit_date(&mut self, id: i64, original: Date) {
        let Some(event) = self.events.iter().find(|event| event.id == id) else { return };
        let (kind, every, days, position, weekday) = match event.repeat {
            Repeat::Once => (Kind::Once, 1, 1 << event.date.weekday(), EventForm::place_of(event.date).0, event.date.weekday()),
            Repeat::Weekly(mask, every) => (Kind::Week, every, mask, EventForm::place_of(event.date).0, event.date.weekday()),
            Repeat::Monthly(position, weekday, every) => (Kind::Month, every, 1 << weekday, position, weekday),
        };
        let series = kind != Kind::Once;
        let (date, start, end, name) = if series {
            let (date, start, end, own) = held(event, original);
            (date, start, end, own.unwrap_or_else(|| event.name.clone()))
        } else {
            (event.date, event.start, event.end, event.name.clone())
        };
        let mut name = TextEdit::new(name);
        name.focused = true;
        self.calendar.form = Some(EventForm {
            id,
            name,
            calendar: event.calendar,
            kind,
            every,
            custom: !matches!(every, 1 | 2),
            days,
            position,
            weekday,
            start,
            end,
            date,
            first: event.date,
            target: series.then_some(original),
            until: event.until,
            overrides: event.overrides.clone(),
            asking: false,
            open: Part::None,
            shown: date,
        });
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
            let place = form.target.map_or(form.date, |_| form.first);
            match kind {
                Kind::Week if form.days == 0 => form.days = 1 << form.date.weekday(),
                Kind::Month => (form.position, form.weekday) = EventForm::place_of(place),
                _ => {}
            }
        }
    }

    /// A pick in the repeat list: its place as `EventForm::choice` counts them.
    pub fn event_choice(&mut self, choice: usize) {
        let Some(form) = self.calendar.form.as_ref() else { return };
        let (kind, every, custom) = match choice {
            0 => (Kind::Once, 1, false),
            1 => (Kind::Week, 1, false),
            2 => (Kind::Week, 2, false),
            3 => (Kind::Month, 1, false),
            4 => (Kind::Month, 2, false),
            _ => (if form.kind == Kind::Once { Kind::Week } else { form.kind }, if form.every <= 2 { 3 } else { form.every }, true),
        };
        self.event_kind(kind);
        if let Some(form) = self.calendar.form.as_mut() {
            form.every = every;
            form.custom = custom;
        }
    }

    /// The count of a custom repeat, from its list.
    pub fn event_every(&mut self, every: u8) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.every = every.clamp(1, MOST_EVERY);
            form.open = Part::None;
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
            form.until = Some(date.max(form.first.min(form.date)));
        } else {
            if form.target.is_none() {
                if form.kind == Kind::Week && form.days == 1 << form.date.weekday() {
                    form.days = 1 << date.weekday();
                }
                if form.kind == Kind::Month && (form.position, form.weekday) == EventForm::place_of(form.date) {
                    (form.position, form.weekday) = EventForm::place_of(date);
                }
            }
            form.date = date;
            if form.target.is_none() {
                form.first = date;
                if form.until.is_some_and(|until| until < date) {
                    form.until = Some(date);
                }
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

    /// Cancels a date of the rule in the dialog, or brings it back.
    pub fn event_skip(&mut self, date: Date) {
        if let Some(form) = self.calendar.form.as_mut() {
            let cancelled = matches!(form.overrides.iter().find(|item| item.original == date), Some(item) if item.change == Change::Cancelled);
            form.overrides.retain(|item| item.original != date);
            if !cancelled {
                form.overrides.push(artosia_core::calendar::Override { original: date, change: Change::Cancelled });
                form.overrides.sort_by_key(|item| item.original);
            }
        }
    }

    /// The dates the dialog lists: the next ones the rule gives from today, or from the first date.
    pub fn event_dates(&self) -> Vec<Date> {
        let Some(form) = self.calendar.form.as_ref() else { return Vec::new() };
        let event = form.event();
        upcoming(&event, self.civil_today().max(event.date), LISTED)
    }

    // ---- Saving the dialog ----

    /// The saved event the dialog is on.
    pub fn form_base(&self) -> Option<&Event> {
        let id = self.calendar.form.as_ref()?.id;
        self.events.iter().find(|event| event.id == id)
    }

    /// Whether saving asks which dates change: the dialog is on a date of a repeating event.
    pub fn form_is_series(&self) -> bool {
        self.calendar.form.as_ref().is_some_and(|form| form.target.is_some()) && self.form_base().is_some_and(|base| base.repeat != Repeat::Once)
    }

    /// Whether the series has a date before the one the dialog is on, so "This and following" differs from "All events".
    pub fn form_can_split(&self) -> bool {
        let (Some(base), Some(target)) = (self.form_base(), self.calendar.form.as_ref().and_then(|form| form.target)) else { return false };
        !rule_dates(base, base.date, target.plus_days(-1)).is_empty()
    }

    /// What saving the form leaves for a scope: the event as changed and, for a split, the new second half.
    pub fn scoped(&self, scope: Scope) -> Option<(Event, Option<Event>)> {
        let form = self.calendar.form.as_ref()?;
        let base = self.form_base()?;
        let target = form.target?;
        let edited = form.event();
        match scope {
            Scope::This => {
                let mut event = base.clone();
                event.overrides = form.overrides.clone();
                if event.change_of(target) != Some(&Change::Cancelled) {
                    let same = form.date == target && form.start == base.start && form.end == base.end && edited.name == base.name;
                    let name = (edited.name != base.name).then(|| edited.name.clone());
                    event.set_change(target, (!same).then_some(Change::Moved { date: form.date, start: form.start, end: form.end, name }));
                }
                Some((event, None))
            }
            Scope::Following => {
                let (mut old, mut new) = base.split_at(target);
                old.overrides = form.overrides.iter().filter(|item| item.original < target).cloned().collect();
                new.overrides = form.overrides.iter().filter(|item| item.original >= target).cloned().collect();
                new.name = edited.name;
                new.calendar = edited.calendar;
                new.start = form.start;
                new.end = form.end;
                new.repeat = edited.repeat;
                new.until = edited.until;
                new.date = form.date;
                new.set_change(target, None);
                Some((old, Some(new)))
            }
            Scope::All => {
                let mut event = edited;
                event.date = form.first.plus_days(form.date.days() - target.days());
                Some((event, None))
            }
        }
    }

    /// How many changed dates a scope would drop because the rule no longer gives them.
    pub fn scope_drops(&self, scope: Scope) -> usize {
        match self.scoped(scope) {
            Some((_, Some(new))) => new.misfits(),
            Some((event, None)) if scope == Scope::All => event.misfits(),
            _ => 0,
        }
    }

    /// The Save button: a date of a repeating event asks which dates change, anything else saves.
    pub fn save_event(&mut self) {
        if !self.calendar.form.as_ref().is_some_and(EventForm::ready) {
            return;
        }
        if self.form_is_series() {
            if let Some(form) = self.calendar.form.as_mut() {
                form.asking = true;
                form.open = Part::None;
            }
            return;
        }
        let Some(mut event) = self.calendar.form.as_ref().map(EventForm::event) else { return };
        event.keep_fitting();
        self.finish_save(event, None);
    }

    /// The answer to the question: writes the dates it names. A change to the later dates or to all
    /// of them can drop changed dates, so its toast offers one Undo that puts the whole series back.
    pub fn save_scope(&mut self, scope: Scope) {
        let Some((mut event, new)) = self.scoped(scope) else { return };
        let Some(before) = self.form_base().cloned() else { return };
        if scope != Scope::This {
            event.keep_fitting();
        }
        let new = new.map(|mut new| {
            new.keep_fitting();
            new
        });
        let added = self.finish_save(event, new);
        if scope != Scope::This && self.calendar.form.is_none() {
            self.say(t::series_changed(&before.name), Some(Undo::Series { before, added }));
        }
    }

    /// Leaves the question for the dialog.
    pub fn save_back(&mut self) {
        if let Some(form) = self.calendar.form.as_mut() {
            form.asking = false;
        }
    }

    /// Writes the event and, for a split, its new half; the id of that half comes back.
    fn finish_save(&mut self, mut event: Event, new: Option<Event>) -> Option<i64> {
        let mut added = None;
        if self.store.save_event(&mut event).is_ok() {
            if let Some(mut new) = new
                && self.store.save_event(&mut new).is_ok()
            {
                added = Some(new.id);
            }
            self.events = self.store.events().unwrap_or_default();
            self.calendar.form = None;
            self.refresh_decks();
        }
        added
    }

    /// Undo of a change to the later dates or to all of them: the new half goes, the event comes back as it was.
    pub fn restore_series(&mut self, before: Event, added: Option<i64>) {
        if let Some(id) = added {
            let _ = self.store.delete_event(id);
        }
        self.put_back_event(before);
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

    /// Writes an event as it was, under the id it had.
    pub fn put_back_event(&mut self, mut event: Event) {
        if self.store.save_event(&mut event).is_ok() {
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
        }
    }

    // ---- One date of a series ----

    /// Sets what happens on one original date and writes it; the event as it was comes back.
    pub(crate) fn change_date(&mut self, id: i64, original: Date, change: Option<Change>) -> Option<Event> {
        let before = self.events.iter().find(|event| event.id == id)?.clone();
        let mut event = before.clone();
        event.set_change(original, change);
        self.store.save_event(&mut event).ok()?;
        self.events = self.store.events().unwrap_or_default();
        self.refresh_decks();
        Some(before)
    }

    pub fn cancel_date(&mut self, id: i64, original: Date) {
        if let Some(before) = self.change_date(id, original, Some(Change::Cancelled)) {
            self.say(t::date_cancelled(&before.name, &date_words(original)), Some(Undo::EventBack(before)));
        }
    }

    pub fn restore_date(&mut self, id: i64, original: Date) {
        if let Some(before) = self.change_date(id, original, None) {
            self.say(t::date_restored(&before.name, &date_words(original)), Some(Undo::EventBack(before)));
        }
    }

    /// Holds a date on another day and hour; its own name, if it had one, stays.
    pub fn move_date(&mut self, id: i64, original: Date, date: Date, start: u16, end: u16) {
        let name = self.events.iter().find(|event| event.id == id).and_then(|event| held(event, original).3);
        if let Some(before) = self.change_date(id, original, Some(Change::Moved { date, start, end, name })) {
            self.say(t::date_moved(&before.name, &date_words(date)), Some(Undo::EventBack(before)));
        }
    }

    /// The dialog's Cancel this date.
    pub fn form_cancel_date(&mut self) {
        let Some((id, Some(original))) = self.calendar.form.as_ref().map(|form| (form.id, form.target)) else { return };
        self.calendar.form = None;
        self.cancel_date(id, original);
    }

    /// The dialog's Restore.
    pub fn form_restore_date(&mut self) {
        let Some((id, Some(original))) = self.calendar.form.as_ref().map(|form| (form.id, form.target)) else { return };
        self.calendar.form = None;
        self.restore_date(id, original);
    }

    /// The dialog's Move this date: the date and times the dialog shows.
    pub fn form_move_date(&mut self) {
        let Some((id, Some(original), date, start, end)) = self.calendar.form.as_ref().map(|form| (form.id, form.target, form.date, form.start, form.end)) else { return };
        self.calendar.form = None;
        self.move_date(id, original, date, start, end);
    }

    /// Whether the dialog's date and times differ from what the rule says, so Move this date has something to do.
    pub fn form_moves(&self) -> bool {
        let (Some(form), Some(base)) = (self.calendar.form.as_ref(), self.form_base()) else { return false };
        form.target.is_some_and(|target| form.date != target || form.start != base.start || form.end != base.end)
    }

    /// The chip menu's Move this date: the dialog on that date, with its date list open.
    pub fn menu_move_date(&mut self, id: i64, original: Date) {
        self.edit_date(id, original);
        self.event_open(Part::First);
    }

    // ---- Dragging a chip ----

    /// A chip is on its way to `to` at `start`; nothing is written until it drops.
    pub fn drag_chip(&mut self, drag: CalDrag) {
        self.calendar.menu = None;
        self.calendar.drag = Some(drag);
    }

    /// The chip is let go. A drop on another day or hour moves that date only; a repeating event's
    /// toast offers Undo and All dates.
    pub fn drop_chip(&mut self, cancelled: bool) {
        let Some(drag) = self.calendar.drag.take() else { return };
        self.calendar.dropped = true;
        if cancelled || (drag.to == drag.from && drag.start == drag.start0) {
            return;
        }
        let Some(before) = self.events.iter().find(|event| event.id == drag.id).cloned() else { return };
        let mut event = before.clone();
        let (_, start, end, name) = held(&before, drag.original);
        let end = (drag.start + end.saturating_sub(start)).min(24 * 60 - 1);
        if before.repeat == Repeat::Once {
            event.date = drag.to;
            (event.start, event.end) = (drag.start, end);
        } else {
            event.set_change(drag.original, Some(Change::Moved { date: drag.to, start: drag.start, end, name }));
        }
        if self.store.save_event(&mut event).is_err() {
            return;
        }
        self.events = self.store.events().unwrap_or_default();
        self.refresh_decks();
        let words = t::date_moved(&before.name, &date_words(drag.to));
        let undo = if before.repeat == Repeat::Once { Undo::EventBack(before) } else { Undo::Moved { before, drag } };
        self.say(words, Some(undo));
    }

    /// The toast's All dates: the move that was just made applies to the whole series.
    pub fn move_all_dates(&mut self) {
        let Some(toast) = self.toast.take() else { return };
        let Some(Undo::Moved { before, drag }) = toast.undo else { return };
        let mut base = before.clone();
        base.set_change(drag.original, None);
        let mut event = base.shifted(drag.from, drag.to);
        let length = held(&before, drag.original).2.saturating_sub(held(&before, drag.original).1);
        event.start = drag.start;
        event.end = (drag.start + length).min(24 * 60 - 1);
        if self.store.save_event(&mut event).is_ok() {
            self.events = self.store.events().unwrap_or_default();
            self.refresh_decks();
            self.say(t::dates_moved_all(&before.name), Some(Undo::EventBack(before)));
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

    /// The church's first weekday, 0 for Sunday: the one setting the calendar and the cards read.
    pub fn first_weekday(&self) -> u8 {
        self.settings.first_weekday % 7
    }

    /// How many days a date is after the first day of its week.
    pub fn lead(&self, date: Date) -> u8 {
        (date.weekday() + 7 - self.first_weekday()) % 7
    }

    /// The first day of the grid of the month on show: the first weekday on or before the 1st.
    pub fn month_start(&self) -> Date {
        let first = self.calendar.anchor.first_of_month();
        first.plus_days(-i64::from(self.lead(first)))
    }

    /// The first day of the week on show.
    pub fn week_start(&self) -> Date {
        self.calendar.anchor.plus_days(-i64::from(self.lead(self.calendar.anchor)))
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
        assert_eq!(app.events.last().map(|event| event.repeat), Some(Repeat::Weekly(1 << 5, 1)), "Friday, the day the date has");
        // Edit: cancel the first date the dialog lists. A date of a series asks which dates change.
        app.edit_event(id);
        let first = app.event_dates()[0];
        app.event_skip(first);
        app.save_event();
        assert!(app.calendar.form.as_ref().is_some_and(|form| form.asking), "a repeating event asks before it saves");
        app.save_scope(Scope::All);
        let saved = app.events.iter().find(|event| event.id == id).expect("kept");
        assert_eq!(saved.overrides.iter().map(|item| (item.original, item.change.clone())).collect::<Vec<_>>(), vec![(first, Change::Cancelled)]);
        assert!(occurrences(saved, first, first).is_empty());
        // Bringing it back in the dialog drops it from the event.
        app.edit_event(id);
        app.event_skip(first);
        app.save_event();
        app.save_scope(Scope::All);
        assert!(app.events.iter().find(|event| event.id == id).is_some_and(|event| event.overrides.is_empty()));
        // Delete, and Undo brings the event back.
        app.edit_event(id);
        app.delete_event();
        assert_eq!(app.events.len(), before);
        app.undo();
        assert!(app.events.iter().any(|event| event.name == "Choir night"));
    }

    /// A weekly event of our own on Sundays from 1 November 2026, saved.
    fn weekly_sunday(app: &mut App, name: &str) -> i64 {
        app.new_event(date(2026, 11, 1));
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new(name);
        }
        app.event_choice(1);
        app.save_event();
        app.events.iter().find(|event| event.name == name).map(|event| event.id).expect("saved")
    }

    fn found(app: &App, id: i64, from: Date, to: Date) -> Vec<(Date, u16)> {
        let event = app.events.iter().find(|event| event.id == id).expect("event");
        occurrences(event, from, to).into_iter().map(|at| (at.date, at.start)).collect()
    }

    #[test]
    fn the_repeat_list_has_six_choices_and_a_custom_count_comes_from_a_list() {
        let mut app = App::practice();
        app.new_event(date(2026, 11, 20));
        for (choice, kind, every) in [(1, Kind::Week, 1), (2, Kind::Week, 2), (3, Kind::Month, 1), (4, Kind::Month, 2)] {
            app.event_choice(choice);
            let form = app.calendar.form.as_ref().expect("dialog");
            assert_eq!((form.choice(), form.kind, form.every, form.custom), (choice, kind, every, false));
        }
        assert!(matches!(app.calendar.form.as_ref().map(|form| form.event().repeat), Some(Repeat::Monthly(_, 5, 2))), "every 2 months on the Friday the date has");
        // Custom keeps the unit it was on and starts at 3.
        app.event_choice(5);
        assert_eq!(app.calendar.form.as_ref().map(|form| (form.choice(), form.kind, form.every)), Some((5, Kind::Month, 3)));
        app.event_every(5);
        app.event_kind(Kind::Week);
        assert!(matches!(app.calendar.form.as_ref().map(|form| form.event().repeat), Some(Repeat::Weekly(_, 5))));
        assert_eq!(app.calendar.form.as_ref().map(EventForm::choice), Some(5), "5 weeks is custom");
        app.event_every(40);
        assert_eq!(app.calendar.form.as_ref().map(|form| form.every), Some(MOST_EVERY), "a list has no more than twelve");
        app.event_choice(0);
        assert_eq!(app.calendar.form.as_ref().map(|form| form.event().repeat), Some(Repeat::Once));
        // The sentence says the interval.
        app.event_choice(2);
        let words = rule_words(&app.calendar.form.as_ref().expect("dialog").event());
        assert!(words.contains("Every 2 weeks"), "{words}");
        // A saved event with every 3 weeks reopens on Custom.
        app.event_choice(5);
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Cell group");
        }
        app.save_event();
        let id = app.events.iter().find(|event| event.name == "Cell group").map(|event| event.id).expect("saved");
        app.edit_event(id);
        assert_eq!(app.calendar.form.as_ref().map(EventForm::choice), Some(5));
    }

    #[test]
    fn the_three_way_choice_changes_one_date_the_dates_after_it_or_all() {
        let mut app = App::practice();
        let id = weekly_sunday(&mut app, "Choir");
        let (nov15, nov22, tuesday) = (date(2026, 11, 15), date(2026, 11, 22), date(2026, 11, 17));
        // The first date has no "this and following": it would be All events.
        app.edit_date(id, date(2026, 11, 1));
        assert!(app.form_is_series() && !app.form_can_split());
        app.edit_date(id, nov15);
        assert!(app.form_can_split());
        // This event: one date to Tuesday evening; the others stay.
        if let Some(form) = app.calendar.form.as_mut() {
            form.date = tuesday;
        }
        app.event_start(19 * 60);
        app.save_event();
        assert!(app.calendar.form.as_ref().is_some_and(|form| form.asking), "the question comes before anything is written");
        assert_eq!(found(&app, id, nov15, nov15), vec![(nov15, 9 * 60)]);
        app.save_scope(Scope::This);
        assert!(app.calendar.form.is_none());
        assert_eq!(found(&app, id, date(2026, 11, 8), nov22), vec![(date(2026, 11, 8), 9 * 60), (tuesday, 19 * 60), (nov22, 9 * 60)]);
        // The dialog on the moved date shows the moved time.
        app.edit_date(id, nov15);
        assert_eq!(app.calendar.form.as_ref().map(|form| (form.date, form.start)), Some((tuesday, 19 * 60)));
        app.close_event();
        // This and following: from the 22nd on a new name at another hour; the old half ends before it and keeps the move.
        app.edit_date(id, nov22);
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Evening choir");
        }
        app.event_start(18 * 60);
        app.save_event();
        app.save_scope(Scope::Following);
        let old = app.events.iter().find(|event| event.id == id).expect("old half");
        let new = app.events.iter().find(|event| event.name == "Evening choir").expect("new half");
        assert_eq!((old.until, old.name.as_str()), (Some(date(2026, 11, 21)), "Choir"));
        assert_eq!((new.date, new.start, new.until), (nov22, 18 * 60, None));
        assert_eq!(old.overrides.len(), 1, "the move before the split stays with the old half");
        // One Undo takes the split back: the old event as it was, and no second half.
        assert!(matches!(app.toast.as_ref().and_then(|toast| toast.undo.as_ref()), Some(Undo::Series { added: Some(_), .. })));
        app.undo();
        assert!(app.events.iter().all(|event| event.name != "Evening choir"));
        let whole = app.events.iter().find(|event| event.id == id).expect("whole series");
        assert_eq!((whole.until, whole.overrides.len(), whole.name.as_str()), (None, 1, "Choir"));
        app.edit_date(id, nov22);
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Evening choir");
        }
        app.event_start(18 * 60);
        app.save_event();
        app.save_scope(Scope::Following);
        let old = app.events.iter().find(|event| event.id == id).expect("old half");
        let new = app.events.iter().find(|event| event.name == "Evening choir").expect("new half");
        assert_eq!(found(&app, new.id, nov22, date(2026, 12, 6)), vec![(nov22, 18 * 60), (date(2026, 11, 29), 18 * 60), (date(2026, 12, 6), 18 * 60)]);
        // All events: a changed rule drops the changes it no longer gives, and says how many first.
        let id = weekly_sunday(&mut app, "Prayer");
        app.cancel_date(id, date(2026, 11, 8));
        app.cancel_date(id, nov22);
        app.edit_date(id, nov15);
        app.event_choice(2);
        assert_eq!(app.scope_drops(Scope::All), 2);
        assert_eq!(app.scope_drops(Scope::This), 0);
        app.save_event();
        app.save_scope(Scope::All);
        assert!(app.events.iter().find(|event| event.id == id).is_some_and(|event| event.overrides.is_empty() && event.repeat == Repeat::Weekly(1, 2)));
        // The dropped dates come back with one Undo.
        app.undo();
        let back = app.events.iter().find(|event| event.id == id).expect("event");
        assert_eq!((back.repeat, back.overrides.len()), (Repeat::Weekly(1, 1), 2));
    }

    #[test]
    fn a_date_is_cancelled_moved_and_restored_with_undo() {
        let mut app = App::practice();
        let id = weekly_sunday(&mut app, "Choir");
        let (nov8, nov15) = (date(2026, 11, 8), date(2026, 11, 15));
        app.cancel_date(id, nov8);
        assert_eq!(found(&app, id, nov8, nov8), vec![]);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert_eq!(found(&app, id, nov8, nov8), vec![(nov8, 9 * 60)]);
        // The dialog's own buttons: Move this date takes the date and times it shows.
        app.edit_date(id, nov15);
        assert!(!app.form_moves(), "nothing differs yet");
        if let Some(form) = app.calendar.form.as_mut() {
            form.date = date(2026, 11, 16);
        }
        app.event_start(7 * 60);
        assert!(app.form_moves());
        app.form_move_date();
        assert_eq!(found(&app, id, nov15, date(2026, 11, 16)), vec![(date(2026, 11, 16), 7 * 60)]);
        // Restore from the dialog on the moved date.
        app.edit_date(id, nov15);
        app.form_restore_date();
        assert_eq!(found(&app, id, nov15, date(2026, 11, 16)), vec![(nov15, 9 * 60)]);
        // A cancelled date is still listed by the rule and comes back with Restore.
        app.edit_date(id, nov8);
        app.form_cancel_date();
        let event = app.events.iter().find(|event| event.id == id).expect("event");
        assert_eq!(artosia_core::calendar::cancelled(event, nov8, nov15), vec![nov8]);
        app.restore_date(id, nov8);
        assert!(app.events.iter().find(|event| event.id == id).is_some_and(|event| event.overrides.is_empty()));
    }

    #[test]
    fn a_dropped_chip_moves_its_date_only_and_all_dates_takes_the_series_along() {
        let mut app = App::practice();
        let id = weekly_sunday(&mut app, "Choir");
        let (nov15, tuesday) = (date(2026, 11, 15), date(2026, 11, 17));
        let carry = |to: Date, start: u16| CalDrag { id, original: nov15, from: nov15, start0: 9 * 60, to, start };
        // Let go where it was picked up: nothing happens. Escape: nothing either.
        app.drag_chip(carry(nov15, 9 * 60));
        app.drop_chip(false);
        app.drag_chip(carry(tuesday, 9 * 60));
        app.drop_chip(true);
        assert!(app.events.iter().find(|event| event.id == id).is_some_and(|event| event.overrides.is_empty()));
        // Dropped on Tuesday at 19.15: that date alone, and the toast offers Undo and All dates.
        app.drag_chip(carry(tuesday, 19 * 60 + 15));
        assert!(app.calendar.drag.is_some());
        app.drop_chip(false);
        assert!(app.calendar.drag.is_none() && app.calendar.dropped);
        assert_eq!(found(&app, id, nov15, date(2026, 11, 21)), vec![(tuesday, 19 * 60 + 15)]);
        assert!(matches!(app.toast.as_ref().and_then(|toast| toast.undo.as_ref()), Some(Undo::Moved { .. })));
        app.undo();
        assert_eq!(found(&app, id, nov15, date(2026, 11, 21)), vec![(nov15, 9 * 60)]);
        // All dates: the series is on Tuesdays at 19.15 from the first, and Undo puts it back.
        app.drag_chip(carry(tuesday, 19 * 60 + 15));
        app.drop_chip(false);
        app.move_all_dates();
        assert_eq!(found(&app, id, date(2026, 11, 1), date(2026, 11, 24)), vec![(date(2026, 11, 3), 19 * 60 + 15), (date(2026, 11, 10), 19 * 60 + 15), (tuesday, 19 * 60 + 15), (date(2026, 11, 24), 19 * 60 + 15)]);
        app.undo();
        assert_eq!(app.events.iter().find(|event| event.id == id).map(|event| event.repeat), Some(Repeat::Weekly(1, 1)));
        // A one-time event simply moves.
        app.new_event(date(2026, 11, 14));
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Wedding");
        }
        app.save_event();
        let once = app.events.iter().find(|event| event.name == "Wedding").map(|event| event.id).expect("saved");
        app.drag_chip(CalDrag { id: once, original: date(2026, 11, 14), from: date(2026, 11, 14), start0: 9 * 60, to: date(2026, 11, 28), start: 9 * 60 });
        app.drop_chip(false);
        assert_eq!(app.events.iter().find(|event| event.id == once).map(|event| event.date), Some(date(2026, 11, 28)));
        assert!(matches!(app.toast.as_ref().and_then(|toast| toast.undo.as_ref()), Some(Undo::EventBack(_))));
    }

    #[test]
    fn a_pointer_over_a_grid_is_turned_into_cells_and_quarter_hours() {
        // A cell 160 wide and 90 tall: a chip picked up 20 across and 40 down, carried 200 right and 100 down.
        assert_eq!(cells_from([4.0 + 200.0, 40.0 + 100.0], [160.0, 90.0]), (1, 1));
        assert_eq!(cells_from([4.0 - 30.0, 40.0 - 60.0], [160.0, 90.0]), (-1, -1), "left of and above the first cell");
        assert_eq!(cells_from([159.9, 89.9], [160.0, 90.0]), (0, 0));
        assert_eq!(cells_from([5.0, 5.0], [0.0, 0.0]), (5, 5), "a grid not measured yet divides by one");
        assert_eq!(snapped_start(9 * 60, 0.0), 9 * 60);
        assert_eq!(snapped_start(9 * 60, QUARTER * 3.0 + 2.0), 9 * 60 + 45, "snaps to 15 minutes");
        assert_eq!(snapped_start(9 * 60, -QUARTER * 4.0), 8 * 60);
        assert_eq!(snapped_start(10, -400.0), 0, "never before midnight");
        assert_eq!(snapped_start(23 * 60, 400.0), 23 * 60 + 45, "never past the last entry of a time list");
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
        let card = Announcement { title: "This week".into(), fill: Fill::Schedule, ..Announcement::default() };
        let slides = app.card_slides(&card);
        assert_eq!(slides.len(), 1);
        let expected = artosia_core::deck::window_events(&app.events, &app.today(), &artosia_core::window::Window::default(), &[]).len();
        assert_eq!(slides[0].text.lines().count(), expected);
        assert!(expected >= 6, "a week of the practice calendar holds the services, prayer, youth and rehearsal");
        // Changing an event changes the slide; hiding a calendar in the window does not.
        let event = app.events.iter().find(|event| event.name == "Ibadah doa").map(|event| event.id).expect("practice event");
        app.edit_event(event);
        if let Some(form) = app.calendar.form.as_mut() {
            form.name = TextEdit::new("Doa malam");
        }
        app.save_event();
        app.save_scope(Scope::All);
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
