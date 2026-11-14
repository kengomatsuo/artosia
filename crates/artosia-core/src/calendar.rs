//! The church's calendar: named, coloured groups of events, and the one place where the dates of
//! an event are worked out. A rule has the shapes of RFC 5545 a church uses: once, weekly on some
//! days (BYDAY), monthly on the nth or last weekday (BYDAY with a position), each every N weeks or
//! months (INTERVAL, weeks Sunday-first as WKST=SU, counted from the week or month of the first
//! date), with an end (UNTIL) and single dates cancelled or moved (EXDATE, and RECURRENCE-ID keyed
//! by the original date).

/// A civil date. Ordered by year, month, day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to the month's length.
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Date> {
        ((1..=12).contains(&month) && day >= 1 && day <= Date::month_days(year, month)).then_some(Date { year, month, day })
    }

    pub fn leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }

    /// The days in a month, February's leap day included.
    pub fn month_days(year: i32, month: u8) -> u8 {
        match month {
            2 if Date::leap(year) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        }
    }

    /// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
    pub fn days(self) -> i64 {
        let year = i64::from(self.year) - i64::from(self.month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let shifted_month = (i64::from(self.month) + 9) % 12;
        let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        era * 146_097 + day_of_era - 719_468
    }

    pub fn from_days(days: i64) -> Date {
        let shifted = days + 719_468;
        let era = shifted.div_euclid(146_097);
        let day_of_era = shifted - era * 146_097;
        let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u8;
        let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 } as u8;
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Date { year: year as i32, month, day }
    }

    /// 0 for Sunday to 6 for Saturday, as the window's weekday names are listed.
    pub fn weekday(self) -> u8 {
        // 1970-01-01 was a Thursday.
        (self.days() + 4).rem_euclid(7) as u8
    }

    pub fn plus_days(self, days: i64) -> Date {
        Date::from_days(self.days() + days)
    }

    /// The same day some months on, or the month's last day when it has no such day.
    pub fn plus_months(self, months: i32) -> Date {
        let at = self.year * 12 + i32::from(self.month) - 1 + months;
        let (year, month) = (at.div_euclid(12), (at.rem_euclid(12) + 1) as u8);
        Date { year, month, day: self.day.min(Date::month_days(year, month)) }
    }

    pub fn first_of_month(self) -> Date {
        Date { day: 1, ..self }
    }

    /// `2026-11-14`, the form a library keeps.
    pub fn code(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn parse(code: &str) -> Option<Date> {
        let mut parts = code.splitn(3, '-');
        Date::new(parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?)
    }
}

/// Which weekday of a month a monthly event falls on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// The first to the fourth, as 1 to 4.
    Nth(u8),
    Last,
}

impl Position {
    pub const ALL: [Position; 5] = [Position::Nth(1), Position::Nth(2), Position::Nth(3), Position::Nth(4), Position::Last];
}

/// The date of the `position` `weekday` (0 is Sunday) of a month.
pub fn nth_weekday(year: i32, month: u8, position: Position, weekday: u8) -> Date {
    let weekday = weekday % 7;
    match position {
        Position::Nth(n) => {
            let first = Date { year, month, day: 1 };
            let day = 1 + (weekday + 7 - first.weekday()) % 7 + 7 * (n.clamp(1, 4) - 1);
            Date { year, month, day }
        }
        Position::Last => {
            let last = Date { year, month, day: Date::month_days(year, month) };
            last.plus_days(-i64::from((last.weekday() + 7 - weekday) % 7))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeat {
    Once,
    /// Every `every` weeks (1 or more) on the days in the mask: bit 0 is Sunday, bit 6 Saturday.
    Weekly(u8, u8),
    /// Every `every` months on the position's weekday (0 is Sunday).
    Monthly(Position, u8, u8),
}

impl Repeat {
    /// The word a library keeps: `once`, `w65`, `w65/2` (every 2 weeks), `m1-0`, `m5-3/2` (5 is the last).
    pub fn code(self) -> String {
        let every = |every: u8| if every <= 1 { String::new() } else { format!("/{every}") };
        match self {
            Repeat::Once => "once".to_string(),
            Repeat::Weekly(mask, n) => format!("w{mask}{}", every(n)),
            Repeat::Monthly(position, weekday, n) => format!("m{}-{weekday}{}", match position { Position::Nth(n) => n, Position::Last => 5 }, every(n)),
        }
    }

    pub fn from_code(code: &str) -> Repeat {
        let (code, every) = match code.split_once('/') {
            Some((code, every)) => (code, every.parse::<u8>().unwrap_or(1).max(1)),
            None => (code, 1),
        };
        if let Some(mask) = code.strip_prefix('w').and_then(|mask| mask.parse().ok()) {
            return Repeat::Weekly(mask, every);
        }
        let monthly = code.strip_prefix('m').and_then(|rest| rest.split_once('-')).and_then(|(position, weekday)| Some((position.parse::<u8>().ok()?, weekday.parse::<u8>().ok()?)));
        match monthly {
            Some((5, weekday)) => Repeat::Monthly(Position::Last, weekday % 7, every),
            Some((n @ 1..=4, weekday)) => Repeat::Monthly(Position::Nth(n), weekday % 7, every),
            _ => Repeat::Once,
        }
    }

    /// Whether the weekly mask holds a weekday.
    pub fn on_weekday(mask: u8, weekday: u8) -> bool {
        mask & (1 << (weekday % 7)) != 0
    }
}

/// The position and weekday a date has in its month, as a monthly rule would name them.
pub fn place_of(date: Date) -> (Position, u8) {
    let position = if date.day + 7 > Date::month_days(date.year, date.month) { Position::Last } else { Position::Nth((date.day - 1) / 7 + 1) };
    (position, date.weekday())
}

/// A named, coloured group of events. The colour belongs to the calendar, never to one event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub id: i64,
    pub name: String,
    pub colour: [u8; 3],
}

/// What one date of a series does differently, keyed by the date the rule gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Cancelled,
    /// Held on another date or hour, under its own name when it has one.
    Moved { date: Date, start: u16, end: u16, name: Option<String> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Override {
    /// The date the rule gives, never the date it moved to.
    pub original: Date,
    pub change: Change,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: i64,
    pub calendar: i64,
    pub name: String,
    /// The first date; a repeating event never happens before it.
    pub date: Date,
    /// Minutes after midnight.
    pub start: u16,
    pub end: u16,
    pub repeat: Repeat,
    /// The last date a repeating event may happen on.
    pub until: Option<Date>,
    /// Dates of the rule that are cancelled or moved, at most one per original date.
    pub overrides: Vec<Override>,
}

/// One happening of an event on a date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    /// The date the rule gave it; the key of its override.
    pub original: Date,
    pub date: Date,
    pub start: u16,
    pub end: u16,
    pub name: String,
    pub moved: bool,
}

impl Event {
    pub fn change_of(&self, original: Date) -> Option<&Change> {
        self.overrides.iter().find(|item| item.original == original).map(|item| &item.change)
    }

    /// Sets what happens on an original date, replacing an earlier change of it; None puts it back.
    pub fn set_change(&mut self, original: Date, change: Option<Change>) {
        self.overrides.retain(|item| item.original != original);
        if let Some(change) = change {
            self.overrides.push(Override { original, change });
            self.overrides.sort_by_key(|item| item.original);
        }
    }

    /// Whether the rule gives this date.
    pub fn gives(&self, original: Date) -> bool {
        !rule_dates(self, original, original).is_empty()
    }

    /// Drops the changes whose original date the rule no longer gives, and says how many went.
    pub fn keep_fitting(&mut self) -> usize {
        let before = self.overrides.len();
        let kept: Vec<Override> = self.overrides.iter().filter(|item| self.gives(item.original)).cloned().collect();
        self.overrides = kept;
        before - self.overrides.len()
    }

    /// How many changes `keep_fitting` would drop from this event.
    pub fn misfits(&self) -> usize {
        self.overrides.iter().filter(|item| !self.gives(item.original)).count()
    }

    /// The series cut at `at`, a date after its first: the old one ends the day before and keeps
    /// the earlier changes, the new one (id 0) starts on `at` with the same rule and takes the rest.
    pub fn split_at(&self, at: Date) -> (Event, Event) {
        let mut old = self.clone();
        old.until = Some(at.plus_days(-1));
        old.overrides.retain(|item| item.original < at);
        let mut new = self.clone();
        new.id = 0;
        new.date = at;
        new.overrides.retain(|item| item.original >= at);
        (old, new)
    }

    /// The whole series moved as its date `original` moved to `to`: the first and last dates by the
    /// same days, the weekdays turned with them, a monthly rule following the new place.
    pub fn shifted(&self, original: Date, to: Date) -> Event {
        let delta = to.days() - original.days();
        let mut moved = self.clone();
        moved.date = self.date.plus_days(delta);
        moved.until = self.until.map(|until| until.plus_days(delta));
        match self.repeat {
            Repeat::Once => moved.date = to,
            Repeat::Weekly(mask, every) => {
                let turn = delta.rem_euclid(7) as u32;
                moved.repeat = Repeat::Weekly((((u16::from(mask) << turn) | (u16::from(mask) >> (7 - turn))) & 0x7f) as u8, every);
            }
            Repeat::Monthly(_, _, every) => {
                let (position, weekday) = place_of(to);
                moved.repeat = Repeat::Monthly(position, weekday, every);
            }
        }
        moved.keep_fitting();
        moved
    }
}

/// The dates the rule gives from `from` to `to`, both included, with nothing left out.
pub fn rule_dates(event: &Event, from: Date, to: Date) -> Vec<Date> {
    let from = from.max(event.date);
    let to = event.until.map_or(to, |until| to.min(until));
    if from > to {
        return Vec::new();
    }
    match event.repeat {
        Repeat::Once => vec![event.date].into_iter().filter(|date| *date >= from && *date <= to).collect(),
        Repeat::Weekly(mask, every) => {
            // Weeks start on Sunday; the first date's week is week 0.
            let anchor = event.date.days() - i64::from(event.date.weekday());
            (from.days()..=to.days()).filter(|day| (day - anchor).div_euclid(7) % i64::from(every.max(1)) == 0).map(Date::from_days).filter(|date| Repeat::on_weekday(mask, date.weekday())).collect()
        }
        Repeat::Monthly(position, weekday, every) => {
            let months = (to.year - from.year) * 12 + i32::from(to.month) - i32::from(from.month);
            let anchor = event.date.year * 12 + i32::from(event.date.month);
            (0..=months)
                .map(|step| from.first_of_month().plus_months(step))
                .filter(|month| (month.year * 12 + i32::from(month.month) - anchor) % i32::from(every.max(1)) == 0)
                .map(|month| nth_weekday(month.year, month.month, position, weekday))
                .filter(|date| *date >= from && *date <= to)
                .collect()
        }
    }
}

/// What happens from `from` to `to`, both included, in date and time order: the rule's dates less
/// the cancelled and moved ones, and the moved ones that land in the range wherever the rule gave them.
pub fn occurrences(event: &Event, from: Date, to: Date) -> Vec<Occurrence> {
    let mut found: Vec<Occurrence> = rule_dates(event, from, to)
        .into_iter()
        .filter(|original| event.change_of(*original).is_none())
        .map(|original| Occurrence { original, date: original, start: event.start, end: event.end, name: event.name.clone(), moved: false })
        .collect();
    for item in &event.overrides {
        if let Change::Moved { date, start, end, name } = &item.change
            && *date >= from
            && *date <= to
            && event.gives(item.original)
        {
            found.push(Occurrence { original: item.original, date: *date, start: *start, end: *end, name: name.clone().unwrap_or_else(|| event.name.clone()), moved: true });
        }
    }
    found.sort_by(|a, b| (a.date, a.start, &a.name).cmp(&(b.date, b.start, &b.name)));
    found
}

/// The dates the rule gives from `from` to `to` that were cancelled, for a view that shows them struck.
pub fn cancelled(event: &Event, from: Date, to: Date) -> Vec<Date> {
    rule_dates(event, from, to).into_iter().filter(|date| event.change_of(*date) == Some(&Change::Cancelled)).collect()
}

/// The first `count` dates the rule gives from `from` on, cancelled and moved ones included, looking
/// at most three years ahead.
pub fn upcoming(event: &Event, from: Date, count: usize) -> Vec<Date> {
    let mut dates = rule_dates(event, from, from.plus_days(3 * 366));
    dates.truncate(count);
    dates
}

/// Minutes after midnight as a clock's hour and minute.
pub fn hour_minute(minutes: u16) -> (u8, u8) {
    ((minutes / 60 % 24) as u8, (minutes % 60) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap_or_else(|| panic!("{year}-{month}-{day}"))
    }

    fn event(repeat: Repeat, first: Date) -> Event {
        Event { id: 0, calendar: 1, name: "Service".into(), date: first, start: 9 * 60, end: 10 * 60, repeat, until: None, overrides: Vec::new() }
    }

    /// The dates something happens on, from `from` to `to`.
    fn happen(event: &Event, from: Date, to: Date) -> Vec<Date> {
        occurrences(event, from, to).into_iter().map(|found| found.date).collect()
    }

    fn cancel(event: &mut Event, original: Date) {
        event.set_change(original, Some(Change::Cancelled));
    }

    fn moved(event: &mut Event, original: Date, date: Date, start: u16) {
        event.set_change(original, Some(Change::Moved { date, start, end: start + 60, name: None }));
    }

    const SUNDAY: u8 = 0;
    const WEDNESDAY: u8 = 3;

    #[test]
    fn a_date_knows_its_weekday_and_the_days_of_its_month() {
        assert_eq!(date(2026, 11, 14).weekday(), 6, "a Saturday");
        assert_eq!(date(1970, 1, 1).weekday(), 4);
        assert_eq!(date(2000, 2, 29).weekday(), 2);
        assert_eq!(Date::month_days(2024, 2), 29);
        assert_eq!(Date::month_days(2100, 2), 28, "a century is no leap year");
        assert_eq!(Date::month_days(2000, 2), 29, "except every fourth");
        assert_eq!(Date::month_days(2026, 4), 30);
        assert_eq!(Date::new(2026, 2, 29), None);
        assert_eq!(Date::new(2026, 13, 1), None);
        for days in [-800_000, -1, 0, 1, 11_000, 20_406, 900_000] {
            assert_eq!(Date::from_days(days).days(), days);
        }
        assert_eq!(date(2026, 12, 31).plus_days(1), date(2027, 1, 1));
        assert_eq!(date(2024, 1, 31).plus_months(1), date(2024, 2, 29), "a month with no such day ends on its last");
        assert_eq!(date(2026, 11, 14).plus_months(-11), date(2025, 12, 14));
        assert_eq!(Date::parse("2026-11-14"), Some(date(2026, 11, 14)));
        assert_eq!(Date::parse(&date(2026, 1, 5).code()), Some(date(2026, 1, 5)));
        assert_eq!(Date::parse("2026-02-30"), None);
        assert_eq!(Date::parse("nonsense"), None);
    }

    #[test]
    fn a_weekly_event_runs_across_a_month_end_and_new_year() {
        let service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 12, 6));
        let found = happen(&service, date(2026, 12, 20), date(2027, 1, 10));
        assert_eq!(found, vec![date(2026, 12, 20), date(2026, 12, 27), date(2027, 1, 3), date(2027, 1, 10)]);
        // Two days a week, across the end of a 30-day month.
        let two = event(Repeat::Weekly(1 << SUNDAY | 1 << WEDNESDAY, 1), date(2026, 1, 1));
        let found = happen(&two, date(2026, 11, 25), date(2026, 12, 3));
        assert_eq!(found, vec![date(2026, 11, 25), date(2026, 11, 29), date(2026, 12, 2)]);
        // Never before its first date, and a first date that is no listed weekday starts on the next one.
        let late = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 11, 14));
        assert_eq!(happen(&late, date(2026, 11, 1), date(2026, 11, 22)), vec![date(2026, 11, 15), date(2026, 11, 22)]);
        assert!(happen(&event(Repeat::Weekly(0, 1), date(2026, 1, 1)), date(2026, 1, 1), date(2026, 12, 31)).is_empty(), "no day, no event");
    }

    #[test]
    fn a_monthly_event_takes_the_first_and_last_weekday_in_months_of_every_length() {
        // 2026 February has 28 days, 2024 February 29, April 30, May 31.
        let first_sunday = |year, month| nth_weekday(year, month, Position::Nth(1), SUNDAY);
        let last_sunday = |year, month| nth_weekday(year, month, Position::Last, SUNDAY);
        assert_eq!(first_sunday(2026, 2), date(2026, 2, 1));
        assert_eq!(last_sunday(2026, 2), date(2026, 2, 22));
        assert_eq!(last_sunday(2024, 2), date(2024, 2, 25));
        assert_eq!(nth_weekday(2024, 2, Position::Last, 4), date(2024, 2, 29), "the leap day, a Thursday");
        assert_eq!(last_sunday(2026, 4), date(2026, 4, 26));
        assert_eq!(nth_weekday(2026, 4, Position::Last, 4), date(2026, 4, 30));
        assert_eq!(last_sunday(2026, 5), date(2026, 5, 31), "the 31st itself");
        assert_eq!(first_sunday(2026, 11), date(2026, 11, 1));
        assert_eq!(nth_weekday(2026, 11, Position::Nth(4), 6), date(2026, 11, 28));
        // Every month of a year holds its position: the right weekday, inside the right week.
        for month in 1..=12 {
            for weekday in 0..7 {
                for n in 1..=4 {
                    let at = nth_weekday(2026, month, Position::Nth(n), weekday);
                    assert_eq!(at.weekday(), weekday);
                    assert_eq!((at.day - 1) / 7 + 1, n);
                }
                let last = nth_weekday(2026, month, Position::Last, weekday);
                assert_eq!(last.weekday(), weekday);
                assert!(last.day + 7 > Date::month_days(2026, month));
            }
        }
        let monthly = event(Repeat::Monthly(Position::Last, SUNDAY, 1), date(2026, 1, 1));
        assert_eq!(happen(&monthly, date(2026, 2, 1), date(2026, 5, 31)), vec![date(2026, 2, 22), date(2026, 3, 29), date(2026, 4, 26), date(2026, 5, 31)]);
        // A year boundary, and a first date inside the month cuts the month's own date off.
        let first = event(Repeat::Monthly(Position::Nth(1), SUNDAY, 1), date(2026, 12, 7));
        assert_eq!(happen(&first, date(2026, 12, 1), date(2027, 2, 28)), vec![date(2027, 1, 3), date(2027, 2, 7)]);
    }

    #[test]
    fn until_ends_a_rule_and_a_cancelled_date_is_left_out() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 11, 1));
        service.until = Some(date(2026, 11, 22));
        assert_eq!(happen(&service, date(2026, 11, 1), date(2027, 1, 1)), vec![date(2026, 11, 1), date(2026, 11, 8), date(2026, 11, 15), date(2026, 11, 22)], "the last day itself is in");
        cancel(&mut service, date(2026, 11, 8));
        cancel(&mut service, date(2026, 12, 6));
        assert_eq!(happen(&service, date(2026, 11, 1), date(2027, 1, 1)), vec![date(2026, 11, 1), date(2026, 11, 15), date(2026, 11, 22)]);
        // The rule still lists the cancelled date, so the dialog can offer it back, and a view can strike it.
        assert!(rule_dates(&service, date(2026, 11, 1), date(2026, 11, 30)).contains(&date(2026, 11, 8)));
        assert_eq!(cancelled(&service, date(2026, 11, 1), date(2027, 1, 1)), vec![date(2026, 11, 8)], "a date past the end was never the rule's");
        let mut once = event(Repeat::Once, date(2026, 11, 14));
        cancel(&mut once, date(2026, 11, 14));
        assert!(happen(&once, date(2026, 11, 1), date(2026, 11, 30)).is_empty());
        let once = event(Repeat::Once, date(2026, 11, 14));
        assert_eq!(happen(&once, date(2026, 11, 14), date(2026, 11, 14)), vec![date(2026, 11, 14)]);
        assert!(happen(&once, date(2026, 11, 15), date(2026, 12, 31)).is_empty());
    }

    #[test]
    fn every_two_weeks_counts_sunday_first_weeks_from_the_first_date() {
        // RFC 5545, FREQ=WEEKLY;INTERVAL=2;COUNT=4;BYDAY=TU,SU: WKST=SU gives Aug 5, 17, 19, 31 (1997).
        let rfc = event(Repeat::Weekly(1 << 2 | 1 << SUNDAY, 2), date(1997, 8, 5));
        assert_eq!(upcoming(&rfc, date(1997, 8, 1), 4), vec![date(1997, 8, 5), date(1997, 8, 17), date(1997, 8, 19), date(1997, 8, 31)]);
        // Across a month end and New Year.
        let sundays = event(Repeat::Weekly(1 << SUNDAY, 2), date(2026, 12, 6));
        assert_eq!(happen(&sundays, date(2026, 12, 1), date(2027, 1, 20)), vec![date(2026, 12, 6), date(2026, 12, 20), date(2027, 1, 3), date(2027, 1, 17)]);
        // Over the leap day of 2028.
        let tuesdays = event(Repeat::Weekly(1 << 2, 2), date(2028, 2, 15));
        assert_eq!(happen(&tuesdays, date(2028, 2, 1), date(2028, 3, 20)), vec![date(2028, 2, 15), date(2028, 2, 29), date(2028, 3, 14)]);
        // A first date that is no listed weekday: its own week is week 0, so the next listed day is two weeks on.
        let late = event(Repeat::Weekly(1 << SUNDAY, 2), date(2026, 11, 11));
        assert_eq!(happen(&late, date(2026, 11, 1), date(2026, 12, 31)), vec![date(2026, 11, 22), date(2026, 12, 6), date(2026, 12, 20)]);
        // The same range asked in pieces gives the same dates.
        let whole = happen(&sundays, date(2026, 12, 1), date(2027, 3, 1));
        let mut pieces = happen(&sundays, date(2026, 12, 1), date(2026, 12, 31));
        pieces.extend(happen(&sundays, date(2027, 1, 1), date(2027, 3, 1)));
        assert_eq!(whole, pieces);
    }

    #[test]
    fn every_two_months_counts_from_the_first_dates_month() {
        let first = event(Repeat::Monthly(Position::Nth(1), SUNDAY, 2), date(2026, 11, 1));
        assert_eq!(happen(&first, date(2026, 11, 1), date(2027, 6, 1)), vec![date(2026, 11, 1), date(2027, 1, 3), date(2027, 3, 7), date(2027, 5, 2)], "over New Year");
        // The last Sunday over February of a leap year.
        let last = event(Repeat::Monthly(Position::Last, SUNDAY, 2), date(2027, 12, 1));
        assert_eq!(happen(&last, date(2027, 12, 1), date(2028, 4, 30)), vec![date(2027, 12, 26), date(2028, 2, 27), date(2028, 4, 30)]);
        let common = event(Repeat::Monthly(Position::Last, SUNDAY, 2), date(2026, 12, 1));
        assert_eq!(happen(&common, date(2026, 12, 1), date(2027, 2, 28)), vec![date(2026, 12, 27), date(2027, 2, 28)], "the 28th is the common February's last Sunday");
        // A window that starts in a skipped month still finds the next one.
        assert_eq!(happen(&first, date(2026, 12, 1), date(2027, 1, 31)), vec![date(2027, 1, 3)]);
    }

    #[test]
    fn a_moved_date_is_found_in_the_range_it_lands_in_and_not_the_one_it_left() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 11, 1));
        moved(&mut service, date(2026, 11, 15), date(2026, 11, 17), 19 * 60);
        // The week after Sunday the 15th holds the Tuesday it moved to, though the Sunday is outside it.
        let found = occurrences(&service, date(2026, 11, 16), date(2026, 11, 22));
        assert_eq!(found.iter().map(|found| (found.date, found.original, found.start, found.moved)).collect::<Vec<_>>(), vec![(date(2026, 11, 17), date(2026, 11, 15), 19 * 60, true), (date(2026, 11, 22), date(2026, 11, 22), 9 * 60, false)]);
        // The week that held the Sunday does not.
        assert_eq!(happen(&service, date(2026, 11, 8), date(2026, 11, 15)), vec![date(2026, 11, 8)]);
        // A later date moved back into an earlier range, and a rename travelling with it.
        service.set_change(date(2026, 11, 22), Some(Change::Moved { date: date(2026, 11, 10), start: 600, end: 660, name: Some("Prayer".into()) }));
        let found = occurrences(&service, date(2026, 11, 9), date(2026, 11, 12));
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].original), ("Prayer", date(2026, 11, 22)));
        // A second move of the same original date replaces the first.
        moved(&mut service, date(2026, 11, 15), date(2026, 11, 18), 8 * 60);
        assert_eq!(service.overrides.len(), 2);
        assert_eq!(happen(&service, date(2026, 11, 16), date(2026, 11, 20)), vec![date(2026, 11, 18)]);
        // A move whose original the rule no longer gives shows nowhere.
        service.until = Some(date(2026, 11, 8));
        assert!(happen(&service, date(2026, 11, 1), date(2026, 12, 31)).iter().all(|day| *day <= date(2026, 11, 8)));
        // A moved date is not struck as cancelled.
        assert!(cancelled(&service, date(2026, 11, 1), date(2026, 12, 31)).is_empty());
    }

    #[test]
    fn this_and_following_splits_the_series_and_hands_the_later_changes_on() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 11, 1));
        cancel(&mut service, date(2026, 11, 8));
        cancel(&mut service, date(2026, 11, 22));
        moved(&mut service, date(2026, 12, 6), date(2026, 12, 8), 18 * 60);
        let (old, new) = service.split_at(date(2026, 11, 15));
        assert_eq!(old.until, Some(date(2026, 11, 14)));
        assert_eq!(old.overrides.iter().map(|item| item.original).collect::<Vec<_>>(), vec![date(2026, 11, 8)]);
        assert_eq!((new.id, new.date, new.until), (0, date(2026, 11, 15), None));
        assert_eq!(new.overrides.iter().map(|item| item.original).collect::<Vec<_>>(), vec![date(2026, 11, 22), date(2026, 12, 6)]);
        // Together they say what the one series said.
        let mut both = occurrences(&old, date(2026, 11, 1), date(2026, 12, 31));
        both.extend(occurrences(&new, date(2026, 11, 1), date(2026, 12, 31)));
        both.sort_by_key(|found| (found.date, found.start));
        assert_eq!(both, occurrences(&service, date(2026, 11, 1), date(2026, 12, 31)));
        // A changed rule on the new half leaves the old half alone.
        let mut changed = new.clone();
        changed.repeat = Repeat::Weekly(1 << SUNDAY, 2);
        assert_eq!(happen(&old, date(2026, 11, 15), date(2026, 12, 31)), Vec::<Date>::new());
        assert_eq!(happen(&changed, date(2026, 11, 15), date(2026, 12, 13)), vec![date(2026, 11, 15), date(2026, 11, 29), date(2026, 12, 13)]);
    }

    #[test]
    fn all_events_keeps_the_changes_the_new_rule_still_gives_and_counts_the_rest() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 11, 1));
        cancel(&mut service, date(2026, 11, 8));
        cancel(&mut service, date(2026, 11, 15));
        moved(&mut service, date(2026, 11, 22), date(2026, 11, 24), 600);
        service.repeat = Repeat::Weekly(1 << SUNDAY, 2);
        // Every second Sunday from the 1st: the 15th and 29th stay, the 8th and 22nd are gone.
        assert_eq!(service.misfits(), 2);
        assert_eq!(service.keep_fitting(), 2);
        assert_eq!(service.overrides.iter().map(|item| item.original).collect::<Vec<_>>(), vec![date(2026, 11, 15)]);
        assert_eq!(service.misfits(), 0);
        assert_eq!(service.keep_fitting(), 0, "nothing more to drop");
        // A change after the series' new last day goes too.
        cancel(&mut service, date(2026, 11, 29));
        service.until = Some(date(2026, 11, 20));
        assert_eq!(service.keep_fitting(), 1);
    }

    #[test]
    fn a_whole_series_moves_by_the_days_a_date_moved() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY | 1 << 3, 1), date(2026, 11, 1));
        service.until = Some(date(2026, 12, 31));
        // The Sunday the 15th dropped on Tuesday the 17th: Sunday and Wednesday become Tuesday and Friday.
        let all = service.shifted(date(2026, 11, 15), date(2026, 11, 17));
        assert_eq!((all.date, all.until), (date(2026, 11, 3), Some(date(2027, 1, 2))));
        assert_eq!(all.repeat, Repeat::Weekly(1 << 2 | 1 << 5, 1));
        assert_eq!(happen(&all, date(2026, 11, 1), date(2026, 11, 10)), vec![date(2026, 11, 3), date(2026, 11, 6), date(2026, 11, 10)]);
        // A monthly rule follows the place its date took.
        let monthly = event(Repeat::Monthly(Position::Nth(1), SUNDAY, 1), date(2026, 11, 1));
        let next = monthly.shifted(date(2026, 12, 6), date(2026, 12, 9));
        assert_eq!(next.repeat, Repeat::Monthly(Position::Nth(2), 3, 1));
        // A one-time event simply lands on the date.
        assert_eq!(event(Repeat::Once, date(2026, 11, 14)).shifted(date(2026, 11, 14), date(2026, 11, 20)).date, date(2026, 11, 20));
        // A week turned right round comes back to itself.
        assert_eq!(service.shifted(date(2026, 11, 15), date(2026, 11, 22)).repeat, service.repeat);
    }

    #[test]
    fn upcoming_lists_the_next_dates_with_the_cancelled_ones() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY, 1), date(2026, 1, 4));
        cancel(&mut service, date(2026, 11, 22));
        let next = upcoming(&service, date(2026, 11, 14), 3);
        assert_eq!(next, vec![date(2026, 11, 15), date(2026, 11, 22), date(2026, 11, 29)]);
        service.until = Some(date(2026, 11, 16));
        assert_eq!(upcoming(&service, date(2026, 11, 14), 3), vec![date(2026, 11, 15)]);
        let yearly = event(Repeat::Monthly(Position::Nth(1), SUNDAY, 1), date(2026, 1, 1));
        assert_eq!(upcoming(&yearly, date(2026, 11, 14), 2), vec![date(2026, 12, 6), date(2027, 1, 3)]);
    }

    #[test]
    fn a_rule_is_kept_as_a_word_and_read_back() {
        for repeat in [Repeat::Weekly(65, 2), Repeat::Monthly(Position::Last, 3, 6), Repeat::Once, Repeat::Weekly(65, 1), Repeat::Weekly(127, 1), Repeat::Monthly(Position::Nth(1), 0, 1), Repeat::Monthly(Position::Nth(4), 6, 1), Repeat::Monthly(Position::Last, 3, 1)] {
            assert_eq!(Repeat::from_code(&repeat.code()), repeat);
        }
        assert_eq!(Repeat::from_code("nonsense"), Repeat::Once);
        assert_eq!(Repeat::from_code("m9-1"), Repeat::Once);
        assert_eq!(Repeat::from_code("w65"), Repeat::Weekly(65, 1), "a word kept before the interval existed");
        assert_eq!(Repeat::from_code("w65/0"), Repeat::Weekly(65, 1), "an interval is never below 1");
        assert_eq!(Repeat::Weekly(65, 1).code(), "w65");
        assert_eq!(Repeat::Weekly(65, 2).code(), "w65/2");
        assert_eq!(hour_minute(6 * 60 + 30), (6, 30));
    }
}
