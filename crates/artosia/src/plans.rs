//! Many plans, one for each service: kept in the library and switched between.
//!
//! A plan is a service the church holds every week: a day of the week and a starting time,
//! picked from chips and never typed (owner, 2026-11-11: "why cant like every sunday hh:mm
//! you love textfields too much"). A name is something a plan may also have.
//!
//! Switching never reaches the projector. What is live stays live, and Next goes on in the
//! plan that was opened (docs/operator.md, "Several plans").

use artosia_core::deck::Deck;
use artosia_core::plan::{Plan, weekday};
use artosia_slides::look::Look;
use rune::TextEdit;

use crate::state::{App, Undo, entry_matches};
use crate::strings as t;

/// One line of the list of plans.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanRow {
    pub id: i64,
    /// As stored; `label` is what it reads as.
    pub name: String,
    pub date: String,
    /// The day of the week of the service, 0 for Monday, and its start in minutes after midnight.
    pub day: Option<u8>,
    pub time: Option<u16>,
    pub items: usize,
    /// The plan has a look of its own.
    pub own_look: bool,
}

/// What the box of day, time and name is making.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Editing {
    /// A plan that does not exist until Save.
    New,
    /// A plan with the items of this one, for another service.
    Copy(i64),
    /// This plan's own day, time and name.
    Existing(i64),
}

/// The box that sets a plan's day, time and name. Every part but the name is a chip to press.
pub struct PlanEdit {
    pub what: Editing,
    pub day: u8,
    /// None until an hour is pressed: the box never guesses when a service starts.
    pub hour: Option<u8>,
    pub minute: u8,
    pub name: TextEdit,
    /// The digits typed so far, for a time typed as it is said: "0730", "1830", "7".
    pub typed: String,
    /// The letters typed so far, for a day typed by the start of its name.
    pub typed_day: String,
}

impl PlanEdit {
    /// The starting time in minutes after midnight, once an hour is chosen.
    pub fn time(&self) -> Option<u16> {
        self.hour.map(|hour| u16::from(hour) * 60 + u16::from(self.minute))
    }
}

#[derive(Default)]
pub struct Switcher {
    /// The church's plans in the order the list under the plan shows them.
    pub rows: Vec<PlanRow>,
    pub editing: Option<PlanEdit>,
    /// The list that drops from the plan's title is open.
    pub open: bool,
    /// The plan of the list whose menu is open.
    pub menu: Option<i64>,
    /// Another plan was opened since anything from a plan went live, so Next starts it from the top.
    pub fresh: bool,
}

/// Whether a plan has no name of its own: empty, or one of the names the program gave.
pub fn is_unnamed(name: &str) -> bool {
    matches!(name.trim(), "Today" | "Acara hari ini" | "New plan" | "")
}

/// "Sunday 07:00" for a weekly service, in the window's language.
pub fn slot_words(day: Option<u8>, time: Option<u16>) -> Option<String> {
    let day = t::WEEKDAYS().get(usize::from(day?)).copied()?;
    Some(match time {
        Some(time) => t::service_slot(day, &clock_words(time)),
        None => day.to_string(),
    })
}

/// "07:00", as the window's language writes a time.
pub fn clock_words(time: u16) -> String {
    t::time_of_day((time / 60) as u8, (time % 60) as u8)
}

/// What a plan reads as: its own name, else its day and time, else the name for a plan with neither.
pub fn plan_words(name: &str, day: Option<u8>, time: Option<u16>) -> String {
    if !is_unnamed(name) {
        return name.trim().to_string();
    }
    // With neither, it is a plan and no more: it used to read "Today", under a heading that
    // may itself read Today.
    slot_words(day, time).unwrap_or_else(|| t::KIND_PLAN().to_string())
}

impl PlanRow {
    pub fn label(&self) -> String {
        plan_words(&self.name, self.day, self.time)
    }

    /// The name alone, when the plan has one of its own beside its day and time.
    pub fn own_name(&self) -> Option<String> {
        (!is_unnamed(&self.name)).then(|| self.name.trim().to_string())
    }
}

impl App {
    /// Today's day of the week on this computer's clock, 0 for Monday, and the minutes after midnight.
    pub fn weekday_now(&self) -> (u8, u16) {
        let time = rune::LocalTime::at((self.now * 1000.0) as i64);
        (weekday(time.year, time.month, time.day), (time.hour * 60 + time.minute) as u16)
    }

    /// Every plan the library holds, the open one as it is in memory, in an order that never
    /// changes by use: the weekly services from today's day of the week on, each day by its
    /// time, then the plans with no day as they were made.
    fn all_plans(&self) -> Vec<Plan> {
        let mut plans: Vec<Plan> = self.store.plans().unwrap_or_default().into_iter().filter(|plan| plan.id != self.plan.id).collect();
        plans.push(self.plan.clone());
        let today = self.weekday_now().0;
        let place = |plan: &Plan| (plan.day.is_none(), plan.day.map(|day| (day + 7 - today) % 7), plan.time, Self::made(plan));
        plans.sort_by_key(place);
        plans
    }

    /// When a plan was made, as an order: its number in the library, and last when not saved yet.
    fn made(plan: &Plan) -> i64 {
        if plan.id == 0 { i64::MAX } else { plan.id }
    }

    /// The look the screens use now: the open plan's own, else the usual one.
    pub fn look(&self) -> Look {
        self.plan_look().unwrap_or_else(|| self.settings.look.clone())
    }

    /// The open plan's own look, when it has one.
    pub fn plan_look(&self) -> Option<Look> {
        serde_json::from_value(self.plan.look.clone()?).ok()
    }

    pub fn has_own_look(&self) -> bool {
        self.plan_look().is_some()
    }

    /// Gives the open plan a look of its own, starting from the usual one.
    pub fn use_own_look(&mut self) {
        if self.has_own_look() {
            return;
        }
        self.plan.look = serde_json::to_value(&self.settings.look).ok();
        self.touch_plan();
    }

    /// Drops the plan's own look. The toast offers it back.
    pub fn use_usual_look(&mut self) {
        let Some(look) = self.plan.look.take() else { return };
        self.touch_plan();
        self.look_edit.sync(&self.look());
        self.refresh_decks();
        self.say(t::look_own_dropped(&self.plan_title()), Some(Undo::PlanLook { plan: self.plan.id, look }));
    }

    /// Undo of `use_usual_look`, when the plan is still the open one.
    pub fn restore_plan_look(&mut self, plan: i64, look: serde_json::Value) {
        if self.plan.id != plan {
            return;
        }
        self.plan.look = Some(look);
        self.touch_plan();
        self.look_edit.sync(&self.look());
        self.refresh_decks();
    }

    /// Opens the list that drops from the plan's title, with the plans as they are now.
    pub fn open_switcher(&mut self) {
        self.refresh_plan_rows();
        self.switcher.menu = None;
        self.switcher.open = true;
    }

    pub fn close_switcher(&mut self) {
        self.switcher.open = false;
    }

    /// Where the open plan stands among the entries of the list that drops from the title,
    /// counting each day's heading, so the list can open with it in view.
    pub fn switcher_entry(&self) -> Option<usize> {
        let mut at = 0;
        for (_, plans) in self.plan_groups() {
            at += 1;
            for plan in &plans {
                if plan.id == self.plan.id {
                    return Some(at);
                }
                at += 1;
            }
        }
        None
    }

    pub fn refresh_plan_rows(&mut self) {
        self.switcher.rows = self.all_plans().iter().map(|plan| PlanRow { id: plan.id, name: plan.name.clone(), date: plan.date.clone(), day: plan.day, time: plan.time, items: plan.items.len(), own_look: plan.look.is_some() }).collect();
    }

    /// The list under the plan: every plan, in groups of one day of the week each (today's
    /// first), and last the plans that have no day.
    pub fn plan_groups(&self) -> Vec<(Option<u8>, Vec<PlanRow>)> {
        let mut groups: Vec<(Option<u8>, Vec<PlanRow>)> = Vec::new();
        for row in &self.switcher.rows {
            match groups.last_mut() {
                Some((day, rows)) if *day == row.day => rows.push(row.clone()),
                _ => groups.push((row.day, vec![row.clone()])),
            }
        }
        groups
    }

    /// The service of today that is about to start or has just begun, when the program opens:
    /// the first of today's plans that started less than an hour and a half ago, else the last.
    pub fn plan_for_now(&self) -> Option<i64> {
        let (today, minutes) = self.weekday_now();
        let todays: Vec<&PlanRow> = self.switcher.rows.iter().filter(|row| row.day == Some(today)).collect();
        todays.iter().find(|row| row.time.is_none_or(|time| time + 90 >= minutes)).or(todays.last()).map(|row| row.id)
    }

    /// At the start of the program the plan of the service at hand is the open one.
    pub fn open_plan_for_now(&mut self) {
        if self.live.is_none()
            && let Some(id) = self.plan_for_now()
        {
            self.switch_plan(id);
        }
    }

    fn stored_plan(&self, id: i64) -> Option<Plan> {
        self.store.plans().ok()?.into_iter().find(|plan| plan.id == id)
    }

    /// The first item of the open plan with something to show, with its deck.
    pub(crate) fn plan_start(&self) -> Option<(u64, Deck)> {
        self.plan.items.iter().filter(|item| !item.entry.is_heading()).find_map(|item| self.deck_for(&item.entry).filter(|deck| !deck.slides.is_empty()).map(|deck| (item.id, deck)))
    }

    /// Makes `plan` the open one. The projector is not touched: what is live stays live and loses
    /// its place in the old plan, since item numbers belong to one plan. A live or open deck
    /// that the new plan also holds takes its place there, so Next goes on after it.
    fn enter_plan(&mut self, mut plan: Plan, keep_old: bool) {
        if keep_old && (self.plan.id != 0 || !self.plan.items.is_empty()) {
            self.save_plan_now();
        }
        plan.used = self.now as i64;
        let lines = self.look().max_lines;
        self.plan = plan;
        self.save_plan_now();
        self.look_edit.sync(&self.look());
        self.plan_drag = None;
        self.plan_menu = None;
        self.more_menu = false;
        self.resume = None;
        self.switcher.fresh = true;
        let place_of = |app: &App, source: &artosia_core::deck::DeckSource| app.plan.items.iter().find(|item| entry_matches(&item.entry, source)).map(|item| item.id);
        if let Some(source) = self.live.as_ref().map(|live| live.deck.source.clone()) {
            let item = place_of(self, &source);
            if let Some(live) = self.live.as_mut() {
                live.item = item;
            }
        }
        match self.open.as_ref().map(|open| (open.queued, open.deck.source.clone())) {
            // What Artosia queued came from the old plan, so it is queued again from the new one.
            Some((true, _)) => self.open = None,
            Some((false, source)) => match place_of(self, &source) {
                Some(item) => {
                    if let Some(open) = self.open.as_mut() {
                        open.item = Some(item);
                    }
                }
                // What Preview held belongs to the plan that was left. Showing it under the
                // new plan's list made a tester think the switch had failed (2026-11-11).
                None => self.open = None,
            },
            None => {}
        }
        // Another plan may split lyrics into other line counts. The live deck keeps the words it
        // has on the projector; the one in Preview and what is queued follow the new plan.
        if self.look().max_lines != lines {
            let live = self.live.take();
            self.refresh_decks();
            self.live = live;
        }
        self.requeue();
        // With nothing live, Preview opens the plan's first item, as it does at the start.
        if self.live.is_none()
            && self.open.is_none()
            && let Some(first) = self.plan.items.iter().find(|item| !item.entry.is_heading()).map(|item| item.id)
        {
            self.open_plan_item(first);
        }
        self.refresh_plan_rows();
    }

    pub fn switch_plan(&mut self, id: i64) {
        self.switcher.menu = None;
        if id == self.plan.id {
            return;
        }
        if let Some(plan) = self.stored_plan(id) {
            self.enter_plan(plan, true);
        }
    }

    /// The number a plan of the list has in the library. The open plan may not be saved yet
    /// and has none until it is, so it is saved here and its new number is the answer.
    fn settled(&mut self, id: i64) -> i64 {
        if id == self.plan.id {
            self.save_plan_now();
            return self.plan.id;
        }
        id
    }

    /// A plan of the list as it is now: the open one from memory, another from the library.
    fn plan_by_id(&mut self, id: i64) -> Option<Plan> {
        if id == self.plan.id {
            return Some(self.plan.clone());
        }
        self.stored_plan(id)
    }

    /// Opens the box for a new plan, on the open plan's day (another service of that day is
    /// the usual reason) or on today, with no hour chosen: a guessed hour was taken for a
    /// setting by a tester. Nothing exists until Save.
    pub fn begin_new_plan(&mut self) {
        let day = self.plan.day.unwrap_or(self.weekday_now().0);
        self.switcher.menu = None;
        self.switcher.editing = Some(PlanEdit { what: Editing::New, day, hour: None, minute: 0, name: TextEdit::new(""), typed: String::new(), typed_day: String::new() });
    }

    /// Opens the box for a copy of a plan, for another service with the same items.
    pub fn begin_copy(&mut self, id: i64) {
        let id = self.settled(id);
        let Some(source) = self.plan_by_id(id) else { return };
        let day = source.day.unwrap_or(self.weekday_now().0);
        let minute = source.time.map(|time| (time % 60) as u8).unwrap_or(0);
        self.switcher.menu = None;
        self.switcher.editing = Some(PlanEdit { what: Editing::Copy(id), day, hour: None, minute, name: TextEdit::new(""), typed: String::new(), typed_day: String::new() });
    }

    /// Opens the box on a plan's own day, time and name.
    pub fn begin_edit(&mut self, id: i64) {
        let id = self.settled(id);
        let Some(plan) = self.plan_by_id(id) else { return };
        // The plan's own day and time; a plan that has none yet opens on today with no hour.
        let day = plan.day.unwrap_or(self.weekday_now().0);
        let (hour, minute) = match plan.time {
            Some(time) => (Some((time / 60) as u8), (time % 60) as u8),
            None => (None, 0),
        };
        let name = if is_unnamed(&plan.name) { String::new() } else { plan.name.clone() };
        self.switcher.menu = None;
        self.switcher.editing = Some(PlanEdit { what: Editing::Existing(id), day, hour, minute, name: TextEdit::new(name), typed: String::new(), typed_day: String::new() });
    }

    /// A digit typed in the box while no field has the keys: the time is typed as it is said,
    /// hour then minutes. "7" is seven o'clock, "0730" and "730" half past seven, "1830" half
    /// past six in the evening. A keys-only tester pressed an arrow nine times to get from 16 to 07.
    pub fn plan_box_digit(&mut self, digit: char) {
        let Some(edit) = self.switcher.editing.as_mut() else { return };
        let Some(digit) = digit.to_digit(10) else { return };
        if edit.typed.len() >= 4 {
            edit.typed.clear();
        }
        edit.typed.push(char::from(b'0' + digit as u8));
        let digits: Vec<u8> = edit.typed.bytes().map(|byte| byte - b'0').collect();
        let two = |a: u8, b: u8| a * 10 + b;
        let tens = |digit: u8| if digit <= 5 { digit * 10 } else { 0 };
        (edit.hour, edit.minute) = match digits[..] {
            [hour] => (Some(hour), 0),
            [a, b] if two(a, b) <= 23 => (Some(two(a, b)), 0),
            [hour, minute] => (Some(hour), tens(minute)),
            [a, b, minute] if two(a, b) <= 23 => (Some(two(a, b)), tens(minute)),
            [hour, a, b] => (Some(hour), two(a, b).min(59)),
            [a, b, c, d] if two(a, b) <= 23 => (Some(two(a, b)), two(c, d).min(59)),
            [hour, a, b, _] => (Some(hour), two(a, b).min(59)),
            _ => (edit.hour, edit.minute),
        };
    }

    /// A letter typed in the box while no field has the keys: the day whose name begins with
    /// the letters typed so far, in the window's language or in English ("sun", "min", "sab").
    /// A letter that fits no day starts the word again.
    pub fn plan_box_letter(&mut self, letter: char) {
        let Some(edit) = self.switcher.editing.as_mut() else { return };
        let find = |typed: &str| {
            let english = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
            (0..7u8).find(|day| t::WEEKDAYS()[usize::from(*day)].to_lowercase().starts_with(typed) || english[usize::from(*day)].starts_with(typed))
        };
        edit.typed_day.extend(letter.to_lowercase());
        let found = find(&edit.typed_day).or_else(|| {
            edit.typed_day = letter.to_lowercase().collect();
            find(&edit.typed_day)
        });
        match found {
            Some(day) => edit.day = day,
            None => edit.typed_day.clear(),
        }
    }

    /// Ctrl Tab and Ctrl Shift Tab: the plan after or before the open one in the list, going
    /// round, as the same keys move between the tabs of a Windows program. The projector is
    /// not touched, as with every switch of plan.
    pub fn walk_plans(&mut self, forward: bool) {
        if let Some(id) = self.plan_beside(forward) {
            self.switch_plan(id);
        }
    }

    /// The plan after or before the open one in the list, going round; none with one plan.
    /// The list draws Ctrl Tab on the row of the one after.
    pub fn plan_beside(&self, forward: bool) -> Option<i64> {
        let count = self.switcher.rows.len();
        let at = self.switcher.rows.iter().position(|row| row.id == self.plan.id)?;
        if count < 2 {
            return None;
        }
        let to = if forward { (at + 1) % count } else { (at + count - 1) % count };
        Some(self.switcher.rows[to].id)
    }

    /// Closes the box. With `save` the plan is made, copied or changed; without, nothing is.
    /// Save does nothing until an hour is chosen, and the box stays.
    pub fn finish_edit(&mut self, save: bool) {
        if save && self.switcher.editing.as_ref().is_some_and(|edit| edit.hour.is_none()) {
            return;
        }
        let Some(edit) = self.switcher.editing.take() else { return };
        if !save {
            return;
        }
        let (day, time, name) = (Some(edit.day.min(6)), edit.time().map(|time| time.min(24 * 60 - 1)), edit.name.text.trim().to_string());
        match edit.what {
            Editing::New => {
                let mut plan = Plan::named(name);
                (plan.day, plan.time) = (day, time);
                self.enter_plan(plan, true);
            }
            Editing::Copy(id) => {
                let Some(source) = self.plan_by_id(id) else { return };
                let mut copy = source.copy_named(name);
                (copy.day, copy.time) = (day, time);
                self.enter_plan(copy, true);
            }
            Editing::Existing(id) if id == self.plan.id => {
                (self.plan.name, self.plan.day, self.plan.time) = (name, day, time);
                self.touch_plan();
                self.save_plan_now();
            }
            Editing::Existing(id) => {
                if let Some(mut plan) = self.stored_plan(id) {
                    (plan.name, plan.day, plan.time) = (name, day, time);
                    let _ = self.store.save_plan(&mut plan);
                }
            }
        }
        self.refresh_plan_rows();
    }

    /// Deletes a plan; the toast offers it back. Deleting the open one opens the plan before it
    /// in the list, or an empty one.
    pub fn delete_plan_id(&mut self, id: i64) {
        self.switcher.menu = None;
        if id == self.plan.id {
            self.save_plan_now();
            let gone = self.plan.clone();
            let _ = self.store.delete_plan(gone.id);
            let next = self.all_plans().into_iter().find(|plan| plan.id != gone.id).unwrap_or_else(|| Plan::named(""));
            self.enter_plan(next, false);
            self.say(t::plan_deleted(&plan_words(&gone.name, gone.day, gone.time)), Some(Undo::Plan(Box::new(gone))));
        } else if let Some(gone) = self.stored_plan(id) {
            let _ = self.store.delete_plan(id);
            self.refresh_plan_rows();
            self.say(t::plan_deleted(&plan_words(&gone.name, gone.day, gone.time)), Some(Undo::Plan(Box::new(gone))));
        }
    }

    /// Deletes the open plan.
    pub fn delete_plan(&mut self) {
        self.delete_plan_id(self.plan.id);
    }

    /// Two more plans beside the practice one, for the scenes that show the list: two services
    /// on Sunday and the young people's on Saturday.
    pub fn demo_plans(&mut self) {
        (self.plan.name, self.plan.day, self.plan.time) = (String::new(), Some(6), Some(7 * 60));
        // Saved first, so the morning's plan is the oldest, as a church makes them.
        self.save_plan_now();
        let mut second = self.plan.copy_named("");
        second.time = Some(10 * 60);
        second.items.truncate(3);
        let mut third = Plan::named("Youth");
        (third.day, third.time) = (Some(5), Some(18 * 60));
        third.push(artosia_core::plan::PlanEntry::Announcements);
        for plan in [&mut second, &mut third] {
            let _ = self.store.save_plan(plan);
        }
        self.refresh_plan_rows();
    }

    /// Undo of a deletion: the plan is back in the list, and the open plan stays open.
    pub fn restore_plan(&mut self, plan: Plan) {
        let _ = self.store.restore_plan(&plan);
        self.refresh_plan_rows();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use artosia_core::deck::DeckSource;
    use artosia_core::plan::PlanEntry;

    fn song_ids(app: &App) -> Vec<i64> {
        app.plan.items.iter().filter_map(|item| if let PlanEntry::Song { song_id, .. } = item.entry { Some(song_id) } else { None }).collect()
    }

    fn live_source(app: &App) -> Option<DeckSource> {
        app.live.as_ref().map(|live| live.deck.source.clone())
    }

    /// Makes another plan on a day and time, as the box of chips does on Save.
    fn another_plan(app: &mut App, day: u8, hour: u8) {
        app.begin_new_plan();
        if let Some(edit) = app.switcher.editing.as_mut() {
            (edit.day, edit.hour, edit.minute) = (day, Some(hour), 0);
        }
        app.finish_edit(true);
    }

    fn names(app: &App) -> Vec<String> {
        app.switcher.rows.iter().map(|row| row.label()).collect()
    }

    #[test]
    fn a_plan_is_a_weekly_service_with_a_day_and_a_time() {
        let mut app = App::practice();
        let items = app.plan.items.len();
        // Nothing exists until Save, and Cancel leaves nothing.
        app.begin_new_plan();
        assert_eq!(app.switcher.rows.len(), 1);
        // No hour is guessed, and Save waits for one.
        assert_eq!(app.switcher.editing.as_ref().map(|edit| edit.hour), Some(None));
        app.finish_edit(true);
        assert!(app.switcher.editing.is_some(), "the box stays until an hour is chosen");
        app.finish_edit(false);
        assert_eq!((app.switcher.rows.len(), app.plan.items.len()), (1, items));
        // Sunday at seven, with no name: it reads as its day and time.
        another_plan(&mut app, 6, 7);
        assert_eq!((app.plan.day, app.plan.time, app.plan.items.len()), (Some(6), Some(420), 0));
        assert_eq!(app.plan_title(), "Sunday 07:00");
        // A name is extra, and is what the plan reads as once it has one.
        let open = app.plan.id;
        app.begin_edit(open);
        if let Some(edit) = app.switcher.editing.as_mut() {
            edit.name.replace_text("Sunday school");
            (edit.hour, edit.minute) = (Some(10), 30);
        }
        app.finish_edit(true);
        assert_eq!((app.plan_title().as_str(), app.plan.time), ("Sunday school", Some(630)));
        assert_eq!(slot_words(app.plan.day, app.plan.time).as_deref(), Some("Sunday 10:30"));
    }

    #[test]
    fn a_time_is_typed_as_it_is_said() {
        let mut app = App::practice();
        let typed = |app: &mut App, digits: &str| {
            app.begin_new_plan();
            for digit in digits.chars() {
                app.plan_box_digit(digit);
            }
            let edit = app.switcher.editing.take().expect("the box is open");
            (edit.hour, edit.minute)
        };
        assert_eq!(typed(&mut app, "7"), (Some(7), 0));
        assert_eq!(typed(&mut app, "07"), (Some(7), 0));
        assert_eq!(typed(&mut app, "0730"), (Some(7), 30));
        assert_eq!(typed(&mut app, "730"), (Some(7), 30));
        assert_eq!(typed(&mut app, "1830"), (Some(18), 30));
        assert_eq!(typed(&mut app, "10"), (Some(10), 0));
        assert_eq!(typed(&mut app, "1015"), (Some(10), 15));
        // A day is typed by the start of its name, in English or in the window's language.
        app.begin_new_plan();
        for letter in "sat".chars() {
            app.plan_box_letter(letter);
        }
        assert_eq!(app.switcher.editing.as_ref().map(|edit| edit.day), Some(5));
        for letter in "sunday".chars() {
            app.plan_box_letter(letter);
        }
        assert_eq!(app.switcher.editing.as_ref().map(|edit| edit.day), Some(6), "a new word starts when the letters fit no day");
        app.finish_edit(false);
    }

    #[test]
    fn ctrl_tab_goes_round_the_plans_and_leaves_the_projector() {
        let mut app = App::practice();
        another_plan(&mut app, 6, 7);
        another_plan(&mut app, 6, 10);
        let order: Vec<i64> = app.switcher.rows.iter().map(|row| row.id).collect();
        let first = app.plan.items.iter().find(|item| !item.entry.is_heading()).map(|item| item.id);
        assert!(first.is_none(), "the newest plan is empty");
        let start = order.iter().position(|id| *id == app.plan.id).expect("the open plan is listed");
        app.walk_plans(true);
        assert_eq!(app.plan.id, order[(start + 1) % order.len()]);
        app.walk_plans(false);
        app.walk_plans(false);
        assert_eq!(app.plan.id, order[(start + order.len() - 1) % order.len()]);
        assert!(app.live.is_none());
    }

    #[test]
    fn the_list_that_drops_from_the_title_knows_where_the_open_plan_stands() {
        let mut app = App::practice();
        let (today, _) = app.weekday_now();
        another_plan(&mut app, today, 7);
        another_plan(&mut app, (today + 1) % 7, 9);
        app.open_switcher();
        assert!(app.switcher.open);
        // Entries are a heading for each day and then that day's plans.
        let mut entries: Vec<Option<i64>> = Vec::new();
        for (_, plans) in app.plan_groups() {
            entries.push(None);
            entries.extend(plans.iter().map(|plan| Some(plan.id)));
        }
        let at = app.switcher_entry().expect("the open plan is in the list");
        assert_eq!(entries[at], Some(app.plan.id));
        app.close_switcher();
        assert!(!app.switcher.open);
    }

    #[test]
    fn the_list_groups_plans_by_day_from_today_on_and_orders_a_day_by_time() {
        let mut app = App::practice();
        let (today, _) = app.weekday_now();
        let tomorrow = (today + 1) % 7;
        another_plan(&mut app, tomorrow, 9);
        another_plan(&mut app, today, 17);
        another_plan(&mut app, today, 7);
        let groups = app.plan_groups();
        let days: Vec<Option<u8>> = groups.iter().map(|(day, _)| *day).collect();
        assert_eq!(days, [Some(today), Some(tomorrow), None], "today first, the plan with no day last");
        let times: Vec<Option<u16>> = groups[0].1.iter().map(|row| row.time).collect();
        assert_eq!(times, [Some(420), Some(1020)]);
        // Switching moves no row.
        let before = names(&app);
        let first = groups[0].1[0].id;
        app.switch_plan(first);
        assert_eq!(names(&app), before);
    }

    #[test]
    fn a_copy_is_made_from_any_plan_of_the_list_and_keeps_its_items() {
        let mut app = App::practice();
        app.save_plan_now();
        let (first, items) = (app.plan.id, app.plan.items.len());
        another_plan(&mut app, 6, 7);
        assert_eq!(app.plan.items.len(), 0);
        // Copied from its row while another plan is open.
        app.begin_copy(first);
        if let Some(edit) = app.switcher.editing.as_mut() {
            (edit.day, edit.hour, edit.minute) = (6, Some(17), 0);
        }
        app.finish_edit(true);
        assert_eq!((app.plan.items.len(), app.plan.day, app.plan.time), (items, Some(6), Some(1020)));
        assert_eq!(app.switcher.rows.len(), 3);
    }

    #[test]
    fn a_plan_is_changed_and_deleted_from_its_row_with_undo() {
        let mut app = App::practice();
        app.save_plan_now();
        let first = app.plan.id;
        another_plan(&mut app, 6, 7);
        let second = app.plan.id;
        // The first plan gets a day while the second is the open one.
        app.begin_edit(first);
        if let Some(edit) = app.switcher.editing.as_mut() {
            (edit.day, edit.hour, edit.minute) = (2, Some(19), 0);
        }
        app.finish_edit(true);
        assert_eq!(app.plan.id, second, "the open plan stays open");
        assert!(app.switcher.rows.iter().any(|row| row.id == first && row.day == Some(2) && row.time == Some(1140)));
        // Deleting a plan that is not open leaves the open one, and Undo puts it back.
        app.delete_plan_id(first);
        assert_eq!(app.plan.id, second);
        assert!(!app.switcher.rows.iter().any(|row| row.id == first));
        app.undo();
        assert!(app.switcher.rows.iter().any(|row| row.id == first));
        // Deleting the open one opens another.
        app.delete_plan_id(second);
        assert_ne!(app.plan.id, second);
        app.undo();
        assert!(app.store.plans().unwrap_or_default().iter().any(|plan| plan.id == second));
    }

    #[test]
    fn the_service_at_hand_is_the_open_plan_when_the_program_starts() {
        let mut app = App::practice();
        let (today, minutes) = app.weekday_now();
        let hour = (minutes / 60) as u8;
        // One service of today long over, one about now.
        another_plan(&mut app, today, hour.saturating_sub(4));
        let early = app.plan.id;
        another_plan(&mut app, today, hour);
        let now = app.plan.id;
        app.switch_plan(early);
        assert_eq!(app.plan_for_now(), if hour >= 4 { Some(now) } else { app.plan_for_now() });
        app.open_plan_for_now();
        if hour >= 4 {
            assert_eq!(app.plan.id, now);
        }
        // A plan saved before days were picked reads by its name and sits in the group with no day.
        assert!(app.plan_groups().iter().any(|(day, rows)| day.is_none() && !rows.is_empty()));
    }

    #[test]
    fn the_first_plan_stays_when_another_is_made() {
        let mut app = App::practice();
        let before = app.plan.items.len();
        assert!(before > 0);
        another_plan(&mut app, 6, 7);
        assert_eq!(app.plan.items.len(), 0);
        assert_eq!(app.switcher.rows.len(), 2, "the practice plan and the new one");
        let old = app.switcher.rows.iter().find(|row| row.items == before).map(|row| row.id).expect("the first plan is listed");
        app.switch_plan(old);
        assert_eq!(app.plan.items.len(), before);
    }

    #[test]
    fn deleting_the_only_plan_leaves_an_empty_one() {
        let mut app = App::practice();
        app.delete_plan();
        assert!(app.plan.items.is_empty());
        assert_eq!(app.switcher.rows.len(), 1);
    }

    #[test]
    fn switching_while_something_is_live_leaves_the_projector_alone() {
        let mut app = App::practice();
        let first = app.plan.items.iter().find(|item| matches!(item.entry, PlanEntry::Song { .. })).map(|item| item.id).expect("a song");
        app.open_plan_item(first);
        app.show(1);
        let (source, index) = (live_source(&app), app.live.as_ref().map(|live| live.index));
        assert!(source.is_some());
        // A second plan holding one other song.
        let planned = song_ids(&app);
        let other = app.songs.iter().find(|song| !planned.contains(&song.id)).map(|song| (song.id, song.title.clone())).expect("a song outside the plan");
        another_plan(&mut app, 6, 10);
        app.plan.push(PlanEntry::song(other.0, other.1));
        app.touch_plan();
        assert_eq!(live_source(&app), source, "the live deck did not change");
        assert_eq!(app.live.as_ref().map(|live| live.index), index, "nor its slide");
        assert_eq!(app.live.as_ref().and_then(|live| live.item), None, "it belongs to no item of the new plan");
        // Next stays inside the live song until it ends, then goes on in the new plan.
        let slides = app.live.as_ref().map(|live| live.deck.slides.len()).unwrap_or(0);
        // Started on slide 1, so the last slide is `slides - 2` presses away and one more leaves it.
        for _ in 0..slides - 1 {
            assert_eq!(live_source(&app), source, "Next stayed in the live song");
            app.next();
        }
        assert_eq!(live_source(&app), Some(DeckSource::Song(other.0)), "Next reached the new plan's first song");
        assert_eq!(app.live.as_ref().map(|live| live.index), Some(0));
    }

    #[test]
    fn the_song_that_is_live_takes_its_place_in_the_plan_that_is_opened() {
        let mut app = App::practice();
        let songs: Vec<(i64, String)> = app.plan.items.iter().filter_map(|item| if let PlanEntry::Song { song_id, title, .. } = &item.entry { Some((*song_id, title.clone())) } else { None }).collect();
        assert!(songs.len() >= 2);
        let first = app.plan.items.iter().find(|item| matches!(item.entry, PlanEntry::Song { .. })).map(|item| item.id).expect("a song");
        app.open_plan_item(first);
        app.show(0);
        // A copy holds the same songs, so the live one is found in it and Next follows it.
        app.begin_copy(app.plan.id);
        if let Some(edit) = app.switcher.editing.as_mut() {
            edit.hour = Some(10);
        }
        app.finish_edit(true);
        let at = app.live.as_ref().and_then(|live| live.item);
        assert!(at.is_some_and(|id| app.plan.position(id).is_some()));
        assert_eq!(live_source(&app), Some(DeckSource::Song(songs[0].0)));
        assert!(app.next_item_title().is_some());
    }

    #[test]
    fn what_artosia_queued_is_queued_again_from_the_new_plan() {
        let mut app = App::practice();
        let first = app.plan.items.iter().find(|item| matches!(item.entry, PlanEntry::Song { .. })).map(|item| item.id).expect("a song");
        app.open_plan_item(first);
        app.show(0);
        assert!(app.open.as_ref().is_some_and(|open| open.queued));
        another_plan(&mut app, 6, 10);
        // The new plan is empty: nothing is queued from the old one, and Preview holds the live deck.
        let open = app.open.as_ref().expect("Preview is filled");
        assert_eq!(open.deck.source, app.live.as_ref().expect("still live").deck.source);
        assert_eq!(open.item, None);
    }

    #[test]
    fn a_plan_with_its_own_look_wins_and_falls_back_when_it_is_cleared() {
        use artosia_slides::look::{Background, Look};
        let mut app = App::practice();
        let usual = app.settings.look.clone();
        assert_eq!(app.look(), usual);
        app.use_own_look();
        assert!(app.has_own_look());
        let plain = Look { background: Background::Solid([10, 20, 30]), ..usual.clone() };
        app.set_look(plain.clone());
        assert_eq!(app.look(), plain, "the plan's look is the one in use");
        assert_eq!(app.settings.look, usual, "the usual look was not touched");
        assert_eq!(app.output_state().look, plain, "the projector gets it");
        assert_eq!(app.preview_state(None).look, plain);
        // Another plan without one uses the usual look; coming back finds the own look again.
        app.save_plan_now();
        let first = app.plan.id;
        another_plan(&mut app, 6, 10);
        assert_eq!(app.look(), usual);
        app.switch_plan(first);
        assert_eq!(app.look(), plain);
        assert!(app.switcher.rows.iter().any(|row| row.id == first && row.own_look));
        app.use_usual_look();
        assert_eq!(app.look(), usual);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert_eq!(app.look(), plain, "undo gives the own look back");
        // The usual look still changes the usual look only while the plan has none.
        app.use_usual_look();
        let other = Look { size: 1.2, ..usual.clone() };
        app.set_look(other.clone());
        assert_eq!(app.settings.look, other);
    }

    #[test]
    fn a_copy_keeps_the_look_and_the_plan_stays_loadable() {
        let mut app = App::practice();
        app.use_own_look();
        let own = app.look();
        app.begin_copy(app.plan.id);
        if let Some(edit) = app.switcher.editing.as_mut() {
            edit.hour = Some(10);
        }
        app.finish_edit(true);
        assert!(app.has_own_look());
        assert_eq!(app.look(), own);
    }
}
