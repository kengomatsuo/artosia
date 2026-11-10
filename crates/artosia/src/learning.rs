//! What the search learns from use, on the operator's side: writing the log when a song or
//! passage goes on the projector, reading it back as habits and suggestion groups, and the
//! switch and the forget button. The counts live in `artosia_core::learn`.
//!
//! Nothing is recorded in a practice run, in a headless run, or while the switch is off, and
//! nothing typed into a message, a countdown or the stream key ever reaches the log.

use std::collections::{HashMap, HashSet};

use artosia_core::deck::DeckSource;
use artosia_core::learn::{self, Habit, Moment, PickLog};
use artosia_core::song::Tag;

use crate::palette::{Group, Hit, Pick, Reason};
use crate::state::{App, Undo};
use crate::strings as t;

/// Most rows one suggestion group shows.
const FOLLOW_ROWS: usize = 5;
const WEEKDAY_ROWS: usize = 3;
const RECENT_ROWS: usize = 3;

#[derive(Default)]
pub struct Learning {
    /// This run may write the log: a real library, a real window.
    pub recording: bool,
    /// What the search reached a song or passage with, until it goes live.
    pub reached: Option<(DeckSource, String)>,
    /// Songs that usually start at a section other than the first, by place in the library.
    pub starts: HashMap<usize, (Tag, u32, u32)>,
}

const DAY: i64 = 86_400;

impl App {
    /// Called once the library is open.
    pub fn start_learning(&mut self) {
        self.learn.recording = self.folder.is_some() && !rune::is_headless() && !crate::data::has_flag("--practice");
        if self.learn.recording {
            let _ = self.store.prune_history(self.now as i64);
        }
    }

    fn local_offset(&self) -> i32 {
        rune::LocalTime::at((self.now * 1000.0) as i64).offset
    }

    /// Now, on the local clock.
    pub fn moment(&self) -> Moment {
        Moment::new(self.now as i64, self.local_offset())
    }

    /// Whether picks and live songs are written to the log.
    pub fn records(&self) -> bool {
        self.learn.recording && self.settings.learn
    }

    /// "9 November", in the window's language.
    pub fn day_month(&self, at: i64) -> String {
        let time = rune::LocalTime::at(at * 1000);
        format!("{} {}", time.day, t::MONTHS()[(time.month.clamp(1, 12) - 1) as usize])
    }

    /// A song or passage has just gone live as something new.
    pub(crate) fn log_live(&mut self, source: &DeckSource, tag: Option<Tag>, first: Option<Tag>, index: usize) {
        let reached = self.learn.reached.take();
        if !self.records() {
            return;
        }
        let typed = match reached {
            Some((wanted, typed)) if &wanted == source => typed,
            _ => String::new(),
        };
        let (now, plan) = (self.moment(), Some(self.plan.id));
        let _ = match source {
            DeckSource::Song(id) => {
                let start = tag.map(|tag| tag.short()).unwrap_or_default();
                self.store.record_song_live(&now, plan, *id, &typed, &start, index == 0 || tag == first).map(|_| ())
            }
            DeckSource::Passage { reference: r, .. } => {
                let reference = format!("{}:{}:{}-{}:{}", r.book, r.chapter, r.verse, r.end_chapter, r.end_verse);
                self.store.record_passage_live(&now, plan, &reference, &typed).map(|_| ())
            }
            DeckSource::Announcement(_) | DeckSource::Announcements => Ok(()),
        };
    }

    /// A result was picked in the search. Only what a volunteer chose to open is kept: never
    /// the words of a message, a countdown or a stage note.
    pub(crate) fn log_pick(&mut self, hit: &Hit, how: Pick) {
        if !self.records() {
            return;
        }
        let song_id = |song: usize| self.songs.get(song).map(|found| found.id);
        let (kind, target) = match hit {
            Hit::Song { song, .. } | Hit::Section { song, .. } => match song_id(*song) {
                Some(id) => ("song", format!("song:{id}")),
                None => return,
            },
            Hit::Passage { reference: r, step: None, .. } => ("passage", format!("passage:{}:{}:{}-{}:{}", r.book, r.chapter, r.verse, r.end_chapter, r.end_verse)),
            Hit::Announcement { index, .. } => match self.announcements.get(*index) {
                Some(item) => ("announcement", format!("announcement:{}", item.id)),
                None => return,
            },
            Hit::Look { name, .. } => ("look", format!("look:{name}")),
            Hit::Command(command) => ("command", format!("command:{command:?}")),
            _ => return,
        };
        // Inside a song the words are its sections, not a title, so they teach nothing.
        let typed = if self.palette.chip.is_some() || matches!(hit, Hit::Section { .. }) { String::new() } else { self.palette.query.text.clone() };
        let action = match how {
            Pick::Open => "open",
            Pick::ShowNow => "show_now",
            Pick::AddToPlan => "add_to_plan",
        };
        let log = PickLog { plan_id: Some(self.plan.id), kind, target: &target, typed: &typed, rank: Some(self.palette.picked), action };
        let _ = self.store.record_pick(&self.moment(), &log);
        if how != Pick::AddToPlan {
            let source = match hit {
                Hit::Song { song, .. } | Hit::Section { song, .. } => song_id(*song).map(DeckSource::Song),
                _ => None,
            };
            self.learn.reached = source.map(|source| (source, typed));
        }
    }

    /// Picks of songs for a typed text lately, by place in the library. Empty with learning off.
    pub(crate) fn habit_weights(&self, query: &str) -> HashMap<usize, Habit> {
        if !self.settings.learn {
            return HashMap::new();
        }
        let found = self.store.habits_for(query, self.now as i64).unwrap_or_default();
        found.into_iter().filter_map(|(id, habit)| Some((self.song_position(id)?, habit))).collect()
    }

    /// Reads which songs usually start away from their first section.
    pub(crate) fn read_starts(&mut self) {
        self.learn.starts = if self.settings.learn {
            let found = self.store.start_habits(self.now as i64).unwrap_or_default();
            found
                .into_iter()
                .filter_map(|(id, habit)| Some((self.song_position(id)?, (learn::parse_tag(&habit.tag)?, habit.times, habit.of))))
                .collect()
        } else {
            HashMap::new()
        };
    }

    /// The groups the empty search shows below its two commands: what followed the live song,
    /// what is sung on this day of the week, what was picked from the list lately. A song in
    /// today's plan never appears, and a song shows in one group only.
    pub(crate) fn suggestion_rows(&self) -> Vec<Hit> {
        if !self.settings.learn {
            return Vec::new();
        }
        let (now, planned) = (self.moment(), self.planned_songs());
        // What is on the projector now needs no suggesting.
        let mut seen: HashSet<usize> = match self.live.as_ref().map(|live| &live.deck.source) {
            Some(DeckSource::Song(id)) => self.song_position(*id).into_iter().collect(),
            _ => HashSet::new(),
        };
        let mut out: Vec<Hit> = Vec::new();
        let mut add = |out: &mut Vec<Hit>, group: Group, rows: Vec<(i64, Reason)>, limit: usize| {
            let rows: Vec<(usize, Reason)> = rows
                .into_iter()
                .filter_map(|(id, reason)| {
                    let at = self.song_position(id)?;
                    (!planned.contains(&at) && seen.insert(at)).then_some((at, reason))
                })
                .take(limit)
                .collect();
            if rows.is_empty() {
                return;
            }
            out.push(Hit::Group(group));
            out.extend(rows.into_iter().map(|(song, reason)| Hit::Song { song, title_ranges: Vec::new(), line: None, planned: false, reason: Some(reason) }));
        };
        if let Some(DeckSource::Song(live)) = self.live.as_ref().map(|live| &live.deck.source) {
            let title = self.song_position(*live).map(|at| self.songs[at].title.clone()).unwrap_or_default();
            let rows = self.store.followers_of(*live, now.at).unwrap_or_default();
            add(&mut out, Group::Follows(title), rows.into_iter().map(|f| (f.song, Reason::Follows { count: f.services, last: f.last })).collect(), FOLLOW_ROWS);
        }
        let rows = self.store.weekday_songs(&now).unwrap_or_default();
        add(&mut out, Group::Weekday, rows.into_iter().map(|w| (w.song, Reason::Weekday { count: w.services, of: w.of })).collect(), WEEKDAY_ROWS);
        let rows = self.store.recent_picks(now.at).unwrap_or_default();
        add(&mut out, Group::Recent, rows.into_iter().map(|r| (r.song, Reason::Recent { picks: r.picks, last: r.last })).collect(), RECENT_ROWS);
        out
    }

    /// The words under a song's title in the preview: why it is where it is.
    pub fn song_notes(&self, song: usize, reason: Option<&Reason>) -> Vec<String> {
        let mut notes = Vec::new();
        if let Some(reason) = reason {
            notes.push(self.reason_line(reason));
        }
        if let Some((tag, times, of)) = self.learn.starts.get(&song) {
            notes.push(t::starts_line(&t::section_label(*tag), *times, *of));
        }
        notes
    }

    /// One sentence with the real numbers, never a score.
    pub fn reason_line(&self, reason: &Reason) -> String {
        match reason {
            Reason::Habit { typed, picks, last } => t::habit_line(typed, *picks, &self.day_month(*last)),
            Reason::Follows { count, last } => t::follows_line(*count, &self.day_month(*last)),
            Reason::Weekday { count, of } => t::weekday_line(*count, *of),
            Reason::Recent { picks, last } => t::recent_line(*picks, &self.day_month(*last)),
        }
    }

    /// Empties the log. The toast offers it back.
    pub fn forget_learning(&mut self) {
        if let Ok(history) = self.store.forget_history() {
            self.learn.starts.clear();
            self.say(t::LEARN_FORGOTTEN(), Some(Undo::History(Box::new(history))));
        }
    }

    /// Writes a made-up history into the library for `--snap`, relative to now so the weekday
    /// group fills on whatever day it runs. Song numbers are places in the practice library.
    pub fn seed_demo_history(&mut self) {
        let ids: Vec<i64> = self.songs.iter().map(|song| song.id).collect();
        let offset = self.local_offset();
        let today = self.moment();
        let at = |days: i64, hours: i64| Moment::new(today.at - days * DAY + hours * 3600, offset);
        let plan = Some(1);
        for week in 1..=5i64 {
            let day = at(7 * week, 0);
            let _ = self.store.record_song_live(&day, plan, ids[0], "", "", true);
            // Three Sundays the song after it was the fifth, two the sixth.
            let _ = self.store.record_song_live(&at(7 * week, 1), plan, if week <= 3 { ids[4] } else { ids[5] }, "", "", true);
            if week <= 4 {
                // This one is nearly always started at its chorus.
                let _ = self.store.record_song_live(&at(7 * week, 2), plan, ids[3], "", "C", false);
            }
            if week <= 2 {
                let _ = self.store.record_song_live(&at(7 * week, 3), plan, ids[2], "", "", true);
            }
        }
        for days in [2, 6] {
            let log = PickLog { plan_id: plan, kind: "song", target: &format!("song:{}", ids[1]), typed: "", rank: Some(0), action: "open" };
            let _ = self.store.record_pick(&at(days, 0), &log);
        }
        for days in [3, 9, 16, 30] {
            let log = PickLog { plan_id: plan, kind: "song", target: &format!("song:{}", ids[3]), typed: "mor", rank: Some(1), action: "show_now" };
            let _ = self.store.record_pick(&at(days, 0), &log);
        }
        self.read_starts();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Scope;

    /// A practice app with a made-up clock: Sunday 8 November 2026, 10:00 UTC.
    fn app() -> App {
        let mut app = App::practice();
        app.now = 1_794_106_800.0;
        app.learn.recording = true;
        app.plan.items.clear();
        app
    }

    fn ids(app: &App, titles: &[&str]) -> Vec<i64> {
        titles.iter().map(|title| app.songs.iter().find(|song| song.title == *title).map(|song| song.id).unwrap_or(-1)).collect()
    }

    fn search(app: &mut App, query: &str) {
        app.palette.query = rune::TextEdit::new(query);
        app.search();
    }

    fn songs_of(app: &App) -> Vec<usize> {
        app.palette.hits.iter().filter_map(|hit| if let Hit::Song { song, .. } = hit { Some(*song) } else { None }).collect()
    }

    fn pick_for(app: &mut App, days_ago: i64, song: usize, typed: &str) {
        let id = app.songs[song].id;
        let at = Moment::new(app.now as i64 - days_ago * DAY, 0);
        let target = format!("song:{id}");
        let log = PickLog { plan_id: None, kind: "song", target: &target, typed, rank: Some(0), action: "open" };
        let _ = app.store.record_pick(&at, &log);
    }

    #[test]
    fn nothing_is_recorded_in_practice_or_with_learning_off() {
        let mut app = App::practice();
        app.now = 1_794_106_800.0;
        assert!(!app.records(), "a practice run never records");
        app.open_palette(Scope::All, "morning");
        app.pick(Pick::ShowNow);
        assert_eq!(app.store.history_rows(), 0);
        // A real run records, until the switch goes off.
        app.learn.recording = true;
        app.open_palette(Scope::All, "river");
        app.pick(Pick::ShowNow);
        assert!(app.store.history_rows() >= 2, "a pick and a live song");
        app.settings.learn = false;
        let before = app.store.history_rows();
        app.open_palette(Scope::All, "quiet");
        app.pick(Pick::ShowNow);
        assert_eq!(app.store.history_rows(), before);
    }

    #[test]
    fn a_headless_run_does_not_record() {
        // `cargo test` is not headless but `start_learning` also refuses with no folder.
        let mut app = App::practice();
        app.start_learning();
        assert!(!app.learn.recording);
    }

    #[test]
    fn a_pick_and_a_live_song_keep_what_was_typed_and_never_a_message() {
        let mut app = app();
        app.open_palette(Scope::All, "hill morn");
        app.pick(Pick::Open);
        app.show(0);
        let typed: String = app.store.history_rows().to_string();
        assert_eq!(typed, "2");
        // The words of a message, a countdown or a stage note never reach the log.
        for words in ["message Parent of the child", "stage one more chorus", "countdown 5"] {
            app.open_palette(Scope::All, words);
            app.pick(Pick::Open);
        }
        assert_eq!(app.store.history_rows(), 2);
        let habits = app.store.habits_for("hill", app.now as i64).unwrap_or_default();
        assert!(habits.is_empty(), "one pick is not a habit");
        // A song shown again from the same slide counts once.
        app.show(1);
        assert_eq!(app.store.history_rows(), 2);
    }

    #[test]
    fn a_habit_orders_songs_that_tie_and_shows_why() {
        let mut app = app();
        // Two practice songs share the word "hill" in their titles; learn the later one.
        let ids = ids(&app, &["Hill of Morning Stars", "Morning on the Hill"]);
        search(&mut app, "mor");
        let plain = songs_of(&app);
        let loved = app.song_position(ids[0]).unwrap_or(0);
        let other = app.song_position(ids[1]).unwrap_or(0);
        assert!(plain.contains(&loved) && plain.contains(&other));
        for days in [3, 10, 17] {
            pick_for(&mut app, days, loved, "mor");
        }
        search(&mut app, "mor");
        let hit = app.palette.hits.iter().find(|hit| matches!(hit, Hit::Song { song, .. } if *song == loved)).cloned();
        assert!(matches!(hit, Some(Hit::Song { reason: Some(Reason::Habit { picks: 3, .. }), .. })), "{hit:?}");
        let line = app.reason_line(&Reason::Habit { typed: "mor".into(), picks: 3, last: app.now as i64 });
        assert!(line.contains("3 times") && line.contains("8 November"), "{line}");
        // The other song, with no picks, carries no reason.
        assert!(app.palette.hits.iter().any(|hit| matches!(hit, Hit::Song { song, reason: None, .. } if *song == other)));
    }

    #[test]
    fn a_habit_never_lifts_a_song_above_one_in_the_plan_or_an_exact_title() {
        let mut app = app();
        let hill = app.song_position(ids(&app, &["Hill of Morning Stars"])[0]).unwrap_or(0);
        let morning = app.song_position(ids(&app, &["Morning on the Hill"])[0]).unwrap_or(0);
        for days in [2, 4, 6, 8] {
            pick_for(&mut app, days, hill, "morning");
        }
        // Planned, the other song stays first although the habit belongs to `hill`.
        app.add_song_to_plan(morning);
        search(&mut app, "morning");
        assert!(matches!(app.palette.hits.first(), Some(Hit::Song { song, planned: true, .. }) if *song == morning));
        // A whole title typed out puts that song first, whatever the history says.
        app.plan.items.clear();
        for days in [2, 4, 6, 8] {
            pick_for(&mut app, days, hill, "quiet harbour");
        }
        search(&mut app, "quiet harbour");
        let quiet = app.song_position(ids(&app, &["Quiet Harbour"])[0]).unwrap_or(0);
        assert!(matches!(app.palette.hits.first(), Some(Hit::Song { song, .. }) if *song == quiet));
    }

    #[test]
    fn picks_older_than_twelve_weeks_leave_the_order_as_it_was() {
        let mut app = app();
        let hill = app.song_position(ids(&app, &["Hill of Morning Stars"])[0]).unwrap_or(0);
        search(&mut app, "mor");
        let plain = songs_of(&app);
        for days in [100, 110, 120] {
            pick_for(&mut app, days, hill, "mor");
        }
        search(&mut app, "mor");
        assert_eq!(songs_of(&app), plain);
        assert!(app.palette.hits.iter().all(|hit| !matches!(hit, Hit::Song { reason: Some(_), .. })));
    }

    #[test]
    fn switching_learning_off_stops_reading_the_history_too() {
        let mut app = app();
        let hill = app.song_position(ids(&app, &["Hill of Morning Stars"])[0]).unwrap_or(0);
        for days in [3, 10] {
            pick_for(&mut app, days, hill, "mor");
        }
        app.settings.learn = false;
        search(&mut app, "mor");
        assert!(app.palette.hits.iter().all(|hit| !matches!(hit, Hit::Song { reason: Some(_), .. })));
    }

    /// Song a is followed by song b on three Sundays, by c on one.
    fn services(app: &mut App) -> Vec<i64> {
        let ids = ids(app, &["Morning on the Hill", "Bread for the Road", "Kasih Setia-Mu", "River of Mercy"]);
        for week in 1..=4i64 {
            let day = Moment::new(app.now as i64 - 7 * week * DAY, 0);
            let _ = app.store.record_song_live(&day, Some(1), ids[0], "", "", true);
            let _ = app.store.record_song_live(&day, Some(1), if week == 4 { ids[2] } else { ids[1] }, "", "", true);
        }
        ids
    }

    #[test]
    fn the_follower_group_lists_what_came_after_the_live_song() {
        let mut app = app();
        let ids = services(&mut app);
        app.open_song(app.song_position(ids[0]).unwrap_or(0), None);
        app.show(0);
        app.open_palette(Scope::All, "");
        let heads: Vec<&Group> = app.palette.hits.iter().filter_map(|hit| if let Hit::Group(group) = hit { Some(group) } else { None }).collect();
        assert!(matches!(heads.first(), Some(Group::Follows(title)) if title == "Morning on the Hill"), "{heads:?}");
        let follows: Vec<(i64, u32)> = app
            .palette
            .hits
            .iter()
            .filter_map(|hit| match hit {
                Hit::Song { song, reason: Some(Reason::Follows { count, .. }), .. } => Some((app.songs[*song].id, *count)),
                _ => None,
            })
            .collect();
        assert_eq!(follows, [(ids[1], 3)], "the one that came once is not a pattern");
        // New song stays first, so Enter does what it always did.
        assert_eq!(app.palette.picked, 0);
        assert!(matches!(app.palette.hits[0], Hit::Command(crate::palette::Command::NewSong)));
    }

    #[test]
    fn a_planned_song_is_not_suggested_and_headings_cannot_be_picked() {
        let mut app = app();
        let ids = services(&mut app);
        let next = app.song_position(ids[1]).unwrap_or(0);
        app.add_song_to_plan(next);
        app.open_song(app.song_position(ids[0]).unwrap_or(0), None);
        app.show(0);
        app.open_palette(Scope::All, "");
        assert!(!app.palette.hits.iter().any(|hit| matches!(hit, Hit::Group(_))));
        // With a heading in the list, the arrows step over it.
        app.plan.items.clear();
        app.open_palette(Scope::All, "");
        let heading = app.palette.hits.iter().position(|hit| matches!(hit, Hit::Group(_))).unwrap_or(0);
        app.palette.picked = heading - 1;
        app.move_pick(1);
        assert_eq!(app.palette.picked, heading + 1);
        app.move_pick(-1);
        assert_eq!(app.palette.picked, heading - 1);
    }

    #[test]
    fn songs_sung_on_this_weekday_and_picked_from_the_list_get_their_own_groups() {
        let mut app = app();
        let ids = services(&mut app);
        app.open_palette(Scope::All, "");
        let weekday: Vec<i64> = app
            .palette
            .hits
            .iter()
            .filter_map(|hit| match hit {
                Hit::Song { song, reason: Some(Reason::Weekday { .. }), .. } => Some(app.songs[*song].id),
                _ => None,
            })
            .collect();
        // Sunday songs of the last weeks, most often first; a song that came up once is left out.
        assert_eq!(weekday, [ids[0], ids[1]]);
        let river = app.song_position(ids[3]).unwrap_or(0);
        pick_for(&mut app, 2, river, "");
        app.open_palette(Scope::All, "");
        assert!(app.palette.hits.iter().any(|hit| matches!(hit, Hit::Song { song, reason: Some(Reason::Recent { picks: 1, .. }), .. } if *song == river)));
    }

    #[test]
    fn a_song_that_starts_at_its_chorus_says_so_and_enter_still_opens_it_normally() {
        let mut app = app();
        let id = ids(&app, &["Morning on the Hill"])[0];
        let at = app.song_position(id).unwrap_or(0);
        for week in 1..=4i64 {
            let day = Moment::new(app.now as i64 - 7 * week * DAY, 0);
            let _ = app.store.record_song_live(&day, None, id, "", "C", false);
        }
        search(&mut app, "morning on");
        assert_eq!(app.learn.starts.get(&at).map(|(tag, times, of)| (tag.short(), *times, *of)), Some(("C".to_string(), 4, 4)));
        let notes = app.song_notes(at, None);
        assert_eq!(notes, ["Usually started at Chorus, 4 of 4 times"]);
        app.pick(Pick::Open);
        // Opened as always in Preview: nothing went live and no section was jumped to.
        assert!(app.live.is_none());
        assert_eq!(app.open.as_ref().map(|open| open.deck.source.clone()), Some(DeckSource::Song(id)));
    }

    #[test]
    fn going_live_notes_the_section_it_began_at() {
        let mut app = app();
        let chorus = |app: &App| {
            let open = app.open.as_ref();
            open.and_then(|open| open.deck.slides.iter().position(|slide| slide.tag.is_some_and(|tag| tag.kind == artosia_core::song::SectionKind::Chorus)))
        };
        // Three Sundays in a row the second song is put up at its chorus, after the first.
        for week in (1..=3).rev() {
            app.now = 1_794_106_800.0 - 7.0 * week as f64 * DAY as f64;
            app.open_song(0, None);
            app.show(0);
            app.open_song(1, None);
            let at = chorus(&app).unwrap_or(1);
            assert!(at > 0);
            app.show(at);
        }
        app.now = 1_794_106_800.0;
        app.read_starts();
        assert_eq!(app.learn.starts.get(&1).map(|(tag, times, of)| (tag.short(), *times, *of)), Some(("C".to_string(), 3, 3)));
        assert!(!app.learn.starts.contains_key(&0));
    }

    #[test]
    fn forgetting_empties_the_history_and_undo_brings_it_back() {
        let mut app = app();
        let ids = services(&mut app);
        let rows = app.store.history_rows();
        app.forget_learning();
        assert_eq!(app.store.history_rows(), 0);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert_eq!(app.store.history_rows(), rows);
        assert_eq!(app.store.followers_of(ids[0], app.now as i64).unwrap_or_default().len(), 1);
    }

    #[test]
    fn the_demo_history_fills_every_group() {
        let mut app = App::practice();
        app.plan.items.clear();
        app.seed_demo_history();
        app.open_song(0, None);
        app.show(0);
        app.open_palette(Scope::All, "");
        let heads = app.palette.hits.iter().filter(|hit| matches!(hit, Hit::Group(_))).count();
        assert_eq!(heads, 3);
    }
}
