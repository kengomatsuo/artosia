//! What the search learns from use: a log of what was picked and what went live, and the
//! counts read back from it.
//!
//! Nothing here scores. Every read is a plain count over a window of the log, so a wrong
//! number can always be recomputed and a volunteer can be told the reason in words
//! (`docs/palette-suggestions.md`). Every function takes the time it should treat as now, so
//! tests run on a made-up clock.

use std::collections::HashMap;

use rusqlite::params;
use rusqlite::types::Value;

use crate::store::{Store, StoreError};

type Result<T> = std::result::Result<T, StoreError>;

const DAY: i64 = 86_400;
/// A pick counts toward a habit for this long: about one season of weekly services.
pub const HABIT_DAYS: i64 = 84;
/// One pick is an accident; this many make a habit.
pub const MIN_PICKS: u32 = 2;
/// A habit is worth at most this much, so a new favourite can still catch up.
pub const HABIT_CAP: u32 = 5;
/// Longest typed text kept with a pick.
pub const TYPED_MAX: usize = 40;
/// How far back "what followed this song" reads.
const FOLLOW_DAYS: i64 = 26 * 7;
/// How many weeks the "this day of the week" group reads.
const WEEKDAY_WEEKS: i64 = 8;
/// The most services one song can count for in the weekday group.
const WEEKDAY_CAP: u32 = 5;
const RECENT_DAYS: i64 = 21;
/// A song started away from its first section at least this share of the time, and at least
/// [`START_MIN`] times, is "usually started at" that section.
const START_DAYS: i64 = 26 * 7;
const START_MIN: u32 = 3;
const START_SHARE: u32 = 60;
/// Rows older than this are deleted when the library opens.
const KEEP_DAYS: i64 = 730;

/// A moment on the local clock: the second, the local date and the weekday (0 is Monday).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moment {
    pub at: i64,
    pub date: String,
    pub weekday: u8,
}

impl Moment {
    /// `at` is unix seconds; `offset` is seconds east of UTC at that instant.
    pub fn new(at: i64, offset: i32) -> Moment {
        let days = (at + i64::from(offset)).div_euclid(DAY);
        let (year, month, day) = civil(days);
        // 1970-01-01 was a Thursday.
        Moment { at, date: format!("{year:04}-{month:02}-{day:02}"), weekday: (days + 3).rem_euclid(7) as u8 }
    }

    /// The same clock `days` earlier or later, with the same offset.
    pub fn plus_days(&self, days: i64, offset: i32) -> Moment {
        Moment::new(self.at + days * DAY, offset)
    }
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// The text typed at a pick as it is kept: folded the way search folds it, at most 40 characters.
pub fn fold_typed(query: &str) -> String {
    crate::text::normalize(query).chars().take(TYPED_MAX).collect()
}

/// What the volunteer picked in the palette.
pub struct PickLog<'a> {
    pub plan_id: Option<i64>,
    /// "song", "passage", "announcement", "look" or "command".
    pub kind: &'a str,
    /// "song:123", "command:Black".
    pub target: &'a str,
    pub typed: &'a str,
    pub rank: Option<usize>,
    /// "open", "show_now" or "add_to_plan".
    pub action: &'a str,
}

/// How often a song was picked for what was typed, inside the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Habit {
    pub picks: u32,
    pub last: i64,
}

impl Habit {
    /// What the cascade reads: 0 when too few picks, else the count up to the cap.
    pub fn weight(&self) -> u8 {
        if self.picks < MIN_PICKS { 0 } else { self.picks.min(HABIT_CAP) as u8 }
    }
}

/// A song that came after another in past services.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Follower {
    pub song: i64,
    pub services: u32,
    pub last: i64,
}

/// A song sung on this weekday: in how many of the last services of that weekday.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekdaySong {
    pub song: i64,
    pub services: u32,
    pub of: u32,
}

/// A song picked from the list with nothing typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecentPick {
    pub song: i64,
    pub picks: u32,
    pub last: i64,
}

/// A song that usually goes live at a section other than its first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartHabit {
    /// [`Tag::short`](crate::song::Tag::short) of the section.
    pub tag: String,
    pub times: u32,
    pub of: u32,
}

/// The whole log, taken out so a forgotten history can come back.
pub struct History {
    tables: Vec<(&'static str, Vec<Vec<Value>>)>,
}

const TABLES: [&str; 3] = ["palette_picks", "service_songs", "service_passages"];

fn song_of(target: &str) -> Option<i64> {
    target.strip_prefix("song:")?.parse().ok()
}

impl Store {
    /// Notes a pick in the palette.
    pub fn record_pick(&self, now: &Moment, log: &PickLog) -> Result<()> {
        self.db().execute(
            "insert into palette_picks (picked_at, service_date, weekday, plan_id, kind, target, typed, shown_rank, action)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![now.at, now.date, now.weekday, log.plan_id.filter(|id| *id != 0), log.kind, log.target, fold_typed(log.typed), log.rank.map(|r| r as i64), log.action],
        )?;
        Ok(())
    }

    /// Notes that a song went on the projector, and returns its place in that day's service.
    /// `start` is the section it began at; `from_start` says that was its first one.
    pub fn record_song_live(&self, now: &Moment, plan_id: Option<i64>, song: i64, typed: &str, start: &str, from_start: bool) -> Result<u32> {
        let seq = self.next_seq(&now.date)?;
        self.db().execute(
            "insert into service_songs (service_date, weekday, plan_id, seq, song_id, shown_at, typed, start_tag, from_start)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![now.date, now.weekday, plan_id.filter(|id| *id != 0), seq, song, now.at, fold_typed(typed), start, from_start],
        )?;
        Ok(seq)
    }

    /// Notes that a passage went on the projector; `reference` is its book, chapter and verses.
    pub fn record_passage_live(&self, now: &Moment, plan_id: Option<i64>, reference: &str, typed: &str) -> Result<u32> {
        let seq = self.next_seq(&now.date)?;
        self.db().execute(
            "insert into service_passages (service_date, plan_id, seq, reference, shown_at, typed) values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![now.date, plan_id.filter(|id| *id != 0), seq, reference, now.at, fold_typed(typed)],
        )?;
        Ok(seq)
    }

    fn next_seq(&self, date: &str) -> Result<u32> {
        let count: i64 = self.db().query_row(
            "select (select count(*) from service_songs where service_date = ?1) + (select count(*) from service_passages where service_date = ?1)",
            [date],
            |row| row.get(0),
        )?;
        Ok(count as u32 + 1)
    }

    /// Songs picked at least twice in the last 12 weeks for a typed text that starts with
    /// `typed`. An empty text matches nothing: it would match everything.
    pub fn habits_for(&self, typed: &str, now: i64) -> Result<HashMap<i64, Habit>> {
        let typed = fold_typed(typed);
        let mut found = HashMap::new();
        if typed.is_empty() {
            return Ok(found);
        }
        let mut statement = self.db().prepare(
            "select target, count(*), max(picked_at) from palette_picks
             where kind = 'song' and typed <> '' and substr(typed, 1, length(?1)) = ?1 and picked_at >= ?2
             group by target having count(*) >= ?3",
        )?;
        let rows = statement.query_map(params![typed, now - HABIT_DAYS * DAY, MIN_PICKS], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?, row.get::<_, i64>(2)?))
        })?;
        for row in rows {
            let (target, picks, last) = row?;
            if let Some(song) = song_of(&target) {
                found.insert(song, Habit { picks, last });
            }
        }
        Ok(found)
    }

    /// Songs that came right after `song` in at least two services of the last 26 weeks,
    /// most often first.
    pub fn followers_of(&self, song: i64, now: i64) -> Result<Vec<Follower>> {
        let mut statement = self.db().prepare(
            "with ordered as (
                 select service_date, song_id,
                        lead(song_id) over (partition by service_date order by seq, id) as next_song,
                        lead(shown_at) over (partition by service_date order by seq, id) as next_at
                 from service_songs where shown_at >= ?2
             )
             select next_song, count(distinct service_date) as services, max(next_at) from ordered
             where song_id = ?1 and next_song is not null and next_song <> ?1
             group by next_song having services >= ?3
             order by services desc, max(next_at) desc, next_song limit 8",
        )?;
        let rows = statement.query_map(params![song, now - FOLLOW_DAYS * DAY, MIN_PICKS], |row| {
            Ok(Follower { song: row.get(0)?, services: row.get(1)?, last: row.get(2)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Songs sung on this weekday in at least two of the last eight weeks, not counting today.
    pub fn weekday_songs(&self, now: &Moment) -> Result<Vec<WeekdaySong>> {
        let since = now.at - WEEKDAY_WEEKS * 7 * DAY;
        let of: u32 = self.db().query_row(
            "select count(distinct service_date) from service_songs where weekday = ?1 and shown_at >= ?2 and service_date < ?3",
            params![now.weekday, since, now.date],
            |row| row.get(0),
        )?;
        let mut statement = self.db().prepare(
            "select song_id, min(count(distinct service_date), ?4) as services from service_songs
             where weekday = ?1 and shown_at >= ?2 and service_date < ?3
             group by song_id having services >= ?5 order by services desc, max(shown_at) desc, song_id limit 12",
        )?;
        let rows = statement.query_map(params![now.weekday, since, now.date, WEEKDAY_CAP, MIN_PICKS], |row| {
            Ok(WeekdaySong { song: row.get(0)?, services: row.get(1)?, of })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Songs picked from the list with nothing typed in the last three weeks.
    pub fn recent_picks(&self, now: i64) -> Result<Vec<RecentPick>> {
        let mut statement = self.db().prepare(
            "select target, count(*), max(picked_at) from palette_picks
             where kind = 'song' and typed = '' and picked_at >= ?1
             group by target order by count(*) desc, max(picked_at) desc limit 12",
        )?;
        let rows = statement.query_map([now - RECENT_DAYS * DAY], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?, row.get::<_, i64>(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (target, picks, last) = row?;
            if let Some(song) = song_of(&target) {
                out.push(RecentPick { song, picks, last });
            }
        }
        Ok(out)
    }

    /// Songs that went live at the same section other than their first, three times or more
    /// and at least 60% of the time, in the last 26 weeks.
    pub fn start_habits(&self, now: i64) -> Result<HashMap<i64, StartHabit>> {
        let mut statement = self.db().prepare(
            "select song_id, start_tag, from_start, count(*) from service_songs where shown_at >= ?1 group by song_id, start_tag, from_start",
        )?;
        let rows = statement.query_map([now - START_DAYS * DAY], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?, row.get::<_, u32>(3)?))
        })?;
        // Per song: every start, and the starts away from the first section by section.
        let mut by_song: HashMap<i64, (u32, HashMap<String, u32>)> = HashMap::new();
        for row in rows {
            let (song, tag, from_start, count) = row?;
            let entry = by_song.entry(song).or_default();
            entry.0 += count;
            if !from_start && !tag.is_empty() {
                *entry.1.entry(tag).or_default() += count;
            }
        }
        let mut found = HashMap::new();
        for (song, (total, away)) in by_song {
            let late: u32 = away.values().sum();
            if total < START_MIN || late * 100 < total * START_SHARE {
                continue;
            }
            if let Some((tag, times)) = away.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))) {
                found.insert(song, StartHabit { tag, times, of: total });
            }
        }
        Ok(found)
    }

    /// How many rows the log holds.
    pub fn history_rows(&self) -> usize {
        TABLES
            .iter()
            .map(|table| self.db().query_row(&format!("select count(*) from {table}"), [], |row| row.get::<_, i64>(0)).unwrap_or(0) as usize)
            .sum()
    }

    /// Empties the log and hands it back, so the forgetting can be undone.
    pub fn forget_history(&self) -> Result<History> {
        let mut tables = Vec::new();
        for table in TABLES {
            let mut statement = self.db().prepare(&format!("select * from {table}"))?;
            let width = statement.column_count();
            let rows = statement.query_map([], |row| (0..width).map(|at| row.get::<_, Value>(at)).collect::<rusqlite::Result<Vec<_>>>())?;
            tables.push((table, rows.collect::<rusqlite::Result<Vec<_>>>()?));
        }
        for table in TABLES {
            self.db().execute(&format!("delete from {table}"), [])?;
        }
        Ok(History { tables })
    }

    /// Puts a forgotten log back.
    pub fn restore_history(&self, history: History) -> Result<()> {
        for (table, rows) in history.tables {
            for row in rows {
                let marks: Vec<String> = (1..=row.len()).map(|at| format!("?{at}")).collect();
                self.db().execute(&format!("insert or replace into {table} values ({})", marks.join(", ")), rusqlite::params_from_iter(row))?;
            }
        }
        Ok(())
    }

    /// Deletes what is older than two years.
    pub fn prune_history(&self, now: i64) -> Result<()> {
        let before = now - KEEP_DAYS * DAY;
        self.db().execute("delete from palette_picks where picked_at < ?1", [before])?;
        self.db().execute("delete from service_songs where shown_at < ?1", [before])?;
        self.db().execute("delete from service_passages where shown_at < ?1", [before])?;
        Ok(())
    }
}

/// The section a start tag names, as written by [`Tag::short`](crate::song::Tag::short).
pub fn parse_tag(code: &str) -> Option<crate::song::Tag> {
    use crate::song::{SectionKind, Tag};
    let kind = match code.chars().next()? {
        'V' => SectionKind::Verse,
        'P' => SectionKind::PreChorus,
        'C' => SectionKind::Chorus,
        'B' => SectionKind::Bridge,
        'I' => SectionKind::Intro,
        'E' => SectionKind::Ending,
        'X' => SectionKind::Other,
        _ => return None,
    };
    let number = code[1..].parse().unwrap_or(1);
    Some(Tag::new(kind, number))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFSET: i32 = 7 * 3600;
    /// Sunday 8 November 2026, 10:00 local.
    const NOW: i64 = 1_794_106_800;

    fn store() -> Store {
        Store::in_memory().unwrap_or_else(|error| panic!("{error}"))
    }

    fn moment(days_ago: i64) -> Moment {
        Moment::new(NOW - days_ago * DAY, OFFSET)
    }

    fn pick(store: &Store, when: &Moment, song: i64, typed: &str) {
        let target = format!("song:{song}");
        let log = PickLog { plan_id: None, kind: "song", target: &target, typed, rank: Some(0), action: "open" };
        assert!(store.record_pick(when, &log).is_ok());
    }

    fn live(store: &Store, when: &Moment, song: i64, start: &str, from_start: bool) {
        assert!(store.record_song_live(when, Some(1), song, "", start, from_start).is_ok());
    }

    #[test]
    fn the_local_date_and_weekday_follow_the_offset() {
        // 2026-11-08 was a Sunday; at 21:00 UTC it is already Monday in UTC+7.
        let utc = Moment::new(NOW, 0);
        assert_eq!((utc.date.as_str(), utc.weekday), ("2026-11-08", 6));
        let late = Moment::new(NOW + 18 * 3600, OFFSET);
        assert_eq!((late.date.as_str(), late.weekday), ("2026-11-09", 0));
        assert_eq!(Moment::new(0, 0).date, "1970-01-01");
        assert_eq!(Moment::new(951_782_400, 0).date, "2000-02-29");
    }

    #[test]
    fn two_picks_in_the_window_make_a_habit_and_one_does_not() {
        let store = store();
        pick(&store, &moment(3), 7, "kasih");
        assert!(store.habits_for("ka", NOW).unwrap_or_default().is_empty());
        pick(&store, &moment(9), 7, "ka");
        let found = store.habits_for("ka", NOW).unwrap_or_default();
        assert_eq!(found.get(&7).map(|habit| (habit.picks, habit.weight())), Some((2, 2)));
        // The longer text only finds the pick typed that long.
        assert!(store.habits_for("kasih", NOW).unwrap_or_default().is_empty());
        // An empty text never counts.
        assert!(store.habits_for("", NOW).unwrap_or_default().is_empty());
    }

    #[test]
    fn picks_older_than_twelve_weeks_fade() {
        let store = store();
        for days in [86, 90, 120] {
            pick(&store, &moment(days), 7, "ka");
        }
        assert!(store.habits_for("ka", NOW).unwrap_or_default().is_empty());
        // The same picks were a habit when they were fresh.
        assert_eq!(store.habits_for("ka", NOW - 100 * DAY).unwrap_or_default().get(&7).map(|habit| habit.picks), Some(3));
    }

    #[test]
    fn a_habit_is_capped_and_a_percent_sign_is_only_text() {
        let store = store();
        for days in 1..=9 {
            pick(&store, &moment(days), 3, "ka");
        }
        assert_eq!(store.habits_for("ka", NOW).unwrap_or_default().get(&3).map(Habit::weight), Some(5));
        assert!(store.habits_for("%", NOW).unwrap_or_default().is_empty());
    }

    /// Four Sundays in which song 1 is followed by 2, and one in which it is followed by 3.
    fn services(store: &Store) {
        for week in 1..=5i64 {
            let day = moment(7 * week);
            live(store, &day, 1, "", true);
            live(store, &day.plus_days(0, OFFSET), if week == 5 { 3 } else { 2 }, "", true);
            let _ = store.record_passage_live(&day, Some(1), "43:3:16", "");
        }
    }

    #[test]
    fn what_followed_a_song_is_counted_most_often_first() {
        let store = store();
        services(&store);
        let found = store.followers_of(1, NOW).unwrap_or_default();
        assert_eq!(found.iter().map(|f| (f.song, f.services)).collect::<Vec<_>>(), [(2, 4)]);
        // A pair seen once is not offered.
        assert!(store.followers_of(2, NOW).unwrap_or_default().is_empty());
        // Past 26 weeks nothing is read.
        assert!(store.followers_of(1, NOW + 200 * DAY).unwrap_or_default().is_empty());
    }

    #[test]
    fn a_passage_between_two_songs_takes_a_place_but_breaks_nothing() {
        let store = store();
        for week in 1..=2i64 {
            let day = moment(7 * week);
            live(&store, &day, 1, "", true);
            let _ = store.record_passage_live(&day, None, "43:3:16", "joh 3");
            live(&store, &day, 2, "", true);
        }
        let seqs: Vec<i64> = store.db().prepare("select seq from service_songs where service_date = ?1 order by seq").and_then(|mut s| s.query_map([moment(7).date], |row| row.get(0)).and_then(|rows| rows.collect())).unwrap_or_default();
        assert_eq!(seqs, [1, 3]);
        assert_eq!(store.followers_of(1, NOW).unwrap_or_default().len(), 1);
    }

    #[test]
    fn the_weekday_group_counts_services_of_the_same_day() {
        let store = store();
        // Sundays one to four weeks ago: song 5 every week, song 6 twice, song 7 once.
        for week in 1..=4i64 {
            let day = moment(7 * week);
            live(&store, &day, 5, "", true);
            if week <= 2 {
                live(&store, &day, 6, "", true);
            }
            if week == 3 {
                live(&store, &day, 7, "", true);
            }
        }
        // A Wednesday does not count for a Sunday.
        live(&store, &moment(4), 8, "", true);
        live(&store, &moment(18), 8, "", true);
        let today = moment(0);
        assert_eq!(today.weekday, 6);
        let found = store.weekday_songs(&today).unwrap_or_default();
        assert_eq!(found.iter().map(|w| (w.song, w.services, w.of)).collect::<Vec<_>>(), [(5, 4, 4), (6, 2, 4)]);
    }

    #[test]
    fn today_does_not_count_toward_its_own_weekday() {
        let store = store();
        live(&store, &moment(0), 5, "", true);
        live(&store, &moment(7), 5, "", true);
        assert!(store.weekday_songs(&moment(0)).unwrap_or_default().is_empty());
    }

    #[test]
    fn picks_from_the_empty_list_come_back_for_three_weeks() {
        let store = store();
        pick(&store, &moment(2), 4, "");
        pick(&store, &moment(5), 4, "");
        pick(&store, &moment(30), 9, "");
        pick(&store, &moment(1), 10, "ka");
        let found = store.recent_picks(NOW).unwrap_or_default();
        assert_eq!(found.iter().map(|r| (r.song, r.picks)).collect::<Vec<_>>(), [(4, 2)]);
    }

    #[test]
    fn a_song_started_at_its_chorus_says_so_and_a_normal_one_does_not() {
        let store = store();
        for week in 1..=4i64 {
            live(&store, &moment(7 * week), 1, if week == 4 { "" } else { "C" }, week == 4);
            live(&store, &moment(7 * week + 1), 2, "", true);
        }
        let found = store.start_habits(NOW).unwrap_or_default();
        assert_eq!(found.get(&1), Some(&StartHabit { tag: "C".into(), times: 3, of: 4 }));
        assert!(!found.contains_key(&2));
        assert_eq!(parse_tag("C").map(|tag| tag.label()), Some("Chorus".to_string()));
        assert_eq!(parse_tag("V2").map(|tag| tag.label()), Some("Verse 2".to_string()));
        // Two starts are too few to call a habit.
        let few = self::store();
        live(&few, &moment(7), 1, "C", false);
        live(&few, &moment(14), 1, "C", false);
        assert!(few.start_habits(NOW).unwrap_or_default().is_empty());
    }

    #[test]
    fn forgetting_empties_the_log_and_undo_brings_it_back() {
        let store = store();
        pick(&store, &moment(3), 7, "ka");
        pick(&store, &moment(4), 7, "ka");
        services(&store);
        let rows = store.history_rows();
        assert!(rows > 10);
        let kept = store.forget_history().unwrap_or_else(|_| panic!("forget"));
        assert_eq!(store.history_rows(), 0);
        assert!(store.habits_for("ka", NOW).unwrap_or_default().is_empty());
        assert!(store.restore_history(kept).is_ok());
        assert_eq!(store.history_rows(), rows);
        assert_eq!(store.habits_for("ka", NOW).unwrap_or_default().len(), 1);
    }

    #[test]
    fn rows_older_than_two_years_are_deleted() {
        let store = store();
        pick(&store, &moment(800), 7, "ka");
        pick(&store, &moment(20), 7, "ka");
        live(&store, &moment(800), 1, "", true);
        assert!(store.prune_history(NOW).is_ok());
        assert_eq!(store.history_rows(), 1);
    }

    #[test]
    fn typed_text_is_folded_and_cut() {
        assert_eq!(fold_typed("  Kasih   SETIA "), "kasih setia");
        assert_eq!(fold_typed(&"a".repeat(80)).len(), TYPED_MAX);
    }
}
