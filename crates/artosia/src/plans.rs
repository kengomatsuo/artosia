//! Many plans, one for each service: kept in the library, named, and switched between.
//!
//! Switching never reaches the projector. What is live stays live, and Next goes on in the
//! plan that was opened (docs/operator.md, "Several plans").

use artosia_core::deck::Deck;
use artosia_core::plan::{Plan, valid_date};
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
    pub items: usize,
    /// The plan has a look of its own.
    pub own_look: bool,
}

/// The rename dialog's two fields.
pub struct Renaming {
    /// The plan was made a moment ago and is being named for the first time.
    pub fresh: bool,
    pub id: i64,
    pub name: TextEdit,
    pub date: TextEdit,
}

#[derive(Default)]
pub struct Switcher {
    /// The list under the title is showing.
    pub open: bool,
    /// The church's plans, the open one first and then the most recently used.
    pub rows: Vec<PlanRow>,
    pub renaming: Option<Renaming>,
    /// Another plan was opened since anything from a plan went live, so Next starts it from the top.
    pub fresh: bool,
}

/// A plan nobody renamed reads in the window's language.
pub fn label(name: &str) -> String {
    if matches!(name, "Today" | "Acara hari ini" | "") { t::PLAN_DEFAULT_NAME().to_string() } else { name.to_string() }
}

/// A day as people write it, as year-month-day. Reads "15 November", "15 Nov 2026",
/// "November 15", "15/11", "15-11-2026" and "2026-11-15", with month names in English, in
/// Indonesian and in the window's language. A day written without its year is the next such
/// day: this year, or next year once it lies more than a month behind `today` (year, month, day).
pub fn read_day(text: &str, today: (i32, u32, u32)) -> Option<String> {
    let text = text.trim().to_lowercase();
    if let Some(date) = valid_date(&text) {
        return Some(date);
    }
    let parts: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|part| !part.is_empty()).collect();
    let numbers: Vec<u32> = parts.iter().filter_map(|part| part.parse().ok()).collect();
    let named: Vec<u32> = parts.iter().filter(|part| part.parse::<u32>().is_err()).filter_map(|part| t::month_number(part)).collect();
    if numbers.len() + named.len() != parts.len() || named.len() > 1 {
        return None;
    }
    let (day, month, year) = match (numbers.as_slice(), named.first()) {
        (&[day], Some(&month)) => (day, month, None),
        (&[a, b], Some(&month)) if a > 31 => (b, month, Some(a)),
        (&[a, b], Some(&month)) => (a, month, Some(b)),
        // Day then month, as Indonesia and most of the world write it; a month over twelve means the other order.
        (&[a, b], None) if b > 12 && a <= 12 => (b, a, None),
        (&[a, b], None) => (a, b, None),
        (&[a, b, year], None) if b > 12 && a <= 12 => (b, a, Some(year)),
        (&[a, b, year], None) => (a, b, Some(year)),
        _ => return None,
    };
    let lengths = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=12).contains(&month) || day == 0 || day > lengths[(month - 1) as usize] {
        return None;
    }
    let year = match year {
        Some(year) if year < 100 => 2000 + year as i32,
        Some(year) if (1900..=9999).contains(&year) => year as i32,
        Some(_) => return None,
        // More than a month ago means the person is planning next year's.
        None if month * 31 + day + 31 < today.1 * 31 + today.2 => today.0 + 1,
        None => today.0,
    };
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

/// "15 November 2026" for the field a plan's day is typed in; nothing for a plan with no day.
pub fn day_words(plan_date: &str) -> String {
    let Some(date) = valid_date(plan_date) else { return String::new() };
    let mut parts = date.split('-').filter_map(|part| part.parse::<u32>().ok());
    match (parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day)) => format!("{day} {} {year}", t::MONTHS()[(month - 1) as usize]),
        _ => String::new(),
    }
}

/// "15 November" for a plan that has a date.
pub fn day_text(plan_date: &str) -> Option<String> {
    let mut plan = Plan::named("");
    plan.date = plan_date.to_string();
    let (month, day) = plan.month_day()?;
    Some(format!("{day} {}", t::MONTHS()[(month - 1) as usize]))
}

impl PlanRow {
    pub fn label(&self) -> String {
        label(&self.name)
    }

    /// The date when the plan has one, else how much it holds.
    pub fn detail(&self) -> String {
        day_text(&self.date).unwrap_or_else(|| t::plan_items(self.items))
    }
}

impl App {
    /// Every plan the library holds, the open one as it is in memory, in an order that never
    /// changes by use.
    fn all_plans(&self) -> Vec<Plan> {
        let mut plans: Vec<Plan> = self.store.plans().unwrap_or_default().into_iter().filter(|plan| plan.id != self.plan.id).collect();
        if self.plan.id != 0 || !self.plan.items.is_empty() {
            plans.push(self.plan.clone());
        }
        // A fixed order, so a row is where it was the last time the list opened: plans with a
        // day first, by day, then the rest as they were made. The open one is marked, not moved.
        plans.sort_by(|a, b| a.date.is_empty().cmp(&b.date.is_empty()).then(a.date.cmp(&b.date)).then(Self::made(a).cmp(&Self::made(b))));
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

    pub fn refresh_plan_rows(&mut self) {
        self.switcher.rows = self.all_plans().iter().map(|plan| PlanRow { id: plan.id, name: plan.name.clone(), date: plan.date.clone(), items: plan.items.len(), own_look: plan.look.is_some() }).collect();
    }

    pub fn open_switcher(&mut self) {
        self.save_plan_now();
        self.refresh_plan_rows();
        self.switcher.open = true;
        self.more_menu = false;
        self.plan_menu = None;
    }

    pub fn close_switcher(&mut self) {
        self.switcher.open = false;
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
        self.switcher.open = false;
        if id == self.plan.id {
            return;
        }
        if let Some(plan) = self.stored_plan(id) {
            self.enter_plan(plan, true);
        }
    }

    /// Starts an empty plan beside the open one and asks for its name.
    pub fn create_plan(&mut self) {
        self.switcher.open = false;
        self.enter_plan(Plan::named(t::PLAN_NEW_NAME()), true);
        self.begin_rename(true);
    }

    pub fn duplicate_plan(&mut self) {
        self.switcher.open = false;
        let copy = self.plan.copy_named(t::plan_copy_name(&self.plan_title()));
        self.enter_plan(copy, true);
    }

    /// Opens the rename dialog for the open plan. A new plan starts with an empty name field.
    pub fn begin_rename(&mut self, blank: bool) {
        self.switcher.open = false;
        let mut name = TextEdit::new(if blank { String::new() } else { self.plan_title() });
        name.focused = true;
        name.set_caret(name.text.len());
        self.switcher.renaming = Some(Renaming { fresh: blank, id: self.plan.id, name, date: TextEdit::new(day_words(&self.plan.date)) });
    }

    /// Today as year, month and day on this computer's clock.
    fn this_day(&self) -> (i32, u32, u32) {
        let time = rune::LocalTime::at((self.now * 1000.0) as i64);
        (time.year, time.month, time.day)
    }

    /// The date field holds a day, or nothing.
    pub fn rename_date_ok(&self) -> bool {
        self.switcher.renaming.as_ref().is_none_or(|renaming| renaming.date.text.trim().is_empty() || read_day(&renaming.date.text, self.this_day()).is_some())
    }

    pub fn finish_rename(&mut self, save: bool) {
        let Some(renaming) = self.switcher.renaming.take() else { return };
        if !save || !self.rename_date_ok() {
            return;
        }
        let name = renaming.name.text.trim();
        let date = read_day(&renaming.date.text, self.this_day()).unwrap_or_default();
        if renaming.id == self.plan.id {
            if !name.is_empty() {
                self.plan.name = name.to_string();
            }
            self.plan.date = date;
            self.touch_plan();
        }
        self.refresh_plan_rows();
    }

    /// Deletes the open plan and opens the one used most recently before it, or an empty one.
    pub fn delete_plan(&mut self) {
        self.switcher.open = false;
        self.save_plan_now();
        let gone = self.plan.clone();
        let _ = self.store.delete_plan(gone.id);
        let next = self.all_plans().into_iter().find(|plan| plan.id != gone.id).unwrap_or_else(|| Plan::named(t::PLAN_DEFAULT_NAME()));
        self.enter_plan(next, false);
        self.say(t::plan_deleted(&label(&gone.name)), Some(Undo::Plan(Box::new(gone))));
    }

    /// Two more plans beside the practice one, for the scenes that show the list.
    pub fn demo_plans(&mut self) {
        self.plan.name = "Sunday 7:00".into();
        self.plan.date = "2026-11-15".into();
        let mut second = self.plan.copy_named("Sunday 10:00");
        second.date = "2026-11-15".into();
        second.items.truncate(3);
        second.used = self.now as i64 - 3600;
        let mut third = Plan::named("Youth Saturday");
        third.push(artosia_core::plan::PlanEntry::Announcements);
        third.used = self.now as i64 - 7200;
        for plan in [&mut second, &mut third] {
            let _ = self.store.save_plan(plan);
        }
        self.save_plan_now();
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

    #[test]
    fn the_first_plan_stays_when_another_is_made() {
        let mut app = App::practice();
        let before = app.plan.items.len();
        assert!(before > 0);
        app.create_plan();
        assert_eq!(app.plan.items.len(), 0);
        assert_eq!(app.switcher.rows.len(), 2, "the practice plan and the new one");
        let old = app.switcher.rows.iter().find(|row| row.items == before).map(|row| row.id).expect("the first plan is listed");
        app.switch_plan(old);
        assert_eq!(app.plan.items.len(), before);
        assert_eq!(app.switcher.rows.first().map(|row| row.id), Some(old), "the open plan leads the list");
    }

    #[test]
    fn a_plan_is_named_dated_duplicated_and_deleted_with_undo() {
        let mut app = App::practice();
        app.begin_rename(false);
        if let Some(renaming) = app.switcher.renaming.as_mut() {
            renaming.name.replace_text("Sunday 10:00");
            renaming.date.replace_text("2026-11-15");
        }
        app.finish_rename(true);
        assert_eq!((app.plan.name.as_str(), app.plan.date.as_str()), ("Sunday 10:00", "2026-11-15"));
        let items = app.plan.items.len();
        app.duplicate_plan();
        assert_eq!((app.plan.items.len(), app.plan.date.as_str()), (items, ""), "a copy has the items and no day");
        assert!(app.plan.name.starts_with("Sunday 10:00"));
        let copy = app.plan.id;
        assert_ne!(copy, 0);
        // The day reads back in words, and words are read as the day.
        app.begin_rename(false);
        assert_eq!(app.switcher.renaming.as_ref().map(|renaming| renaming.date.text.clone()), Some(String::new()), "a copy has no day");
        app.finish_rename(false);
        let today = (2026, 11, 11);
        assert_eq!(read_day("15 November", today).as_deref(), Some("2026-11-15"));
        assert_eq!(read_day("15 nov 2027", today).as_deref(), Some("2027-11-15"));
        assert_eq!(read_day("November 15", today).as_deref(), Some("2026-11-15"));
        assert_eq!(read_day("17 Agustus", today).as_deref(), Some("2027-08-17"), "a day long past is next year's");
        assert_eq!(read_day("25 Desember", today).as_deref(), Some("2026-12-25"));
        assert_eq!(read_day("15/11", today).as_deref(), Some("2026-11-15"));
        assert_eq!(read_day("15-11-2026", today).as_deref(), Some("2026-11-15"));
        assert_eq!(read_day("11/15/2026", today).as_deref(), Some("2026-11-15"), "a month over twelve is a day");
        assert_eq!(read_day("2026-11-15", today).as_deref(), Some("2026-11-15"));
        assert_eq!(read_day("31 June", today), None);
        assert_eq!(read_day("November", today), None);
        assert_eq!(day_words("2026-11-15"), "15 November 2026");
        // A date that is not a day is refused and the plan keeps what it had.
        app.begin_rename(false);
        if let Some(renaming) = app.switcher.renaming.as_mut() {
            renaming.date.replace_text("tomorrow");
        }
        assert!(!app.rename_date_ok());
        app.finish_rename(true);
        assert_eq!(app.plan.date, "");
        app.delete_plan();
        assert_ne!(app.plan.id, copy);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        assert!(!app.switcher.rows.iter().any(|row| row.id == copy));
        app.undo();
        assert!(app.switcher.rows.iter().any(|row| row.id == copy), "undo puts it back in the list");
        assert!(app.store.plans().unwrap_or_default().iter().any(|plan| plan.id == copy));
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
        app.create_plan();
        app.finish_rename(false);
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
        app.duplicate_plan();
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
        app.create_plan();
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
        app.create_plan();
        app.finish_rename(false);
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
    fn a_duplicate_keeps_the_look_and_the_plan_stays_loadable() {
        let mut app = App::practice();
        app.use_own_look();
        let own = app.look();
        app.duplicate_plan();
        assert!(app.has_own_look());
        assert_eq!(app.look(), own);
    }
}
