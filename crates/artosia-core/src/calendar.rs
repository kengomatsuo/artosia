//! The church's calendar: named, coloured groups of events, and the one place where the dates of
//! an event are worked out. A rule has the shapes of RFC 5545 a church uses: once, weekly on some
//! days (BYDAY), monthly on the nth or last weekday (BYDAY with a position), with an end (UNTIL)
//! and dates left out (EXDATE).

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
    /// Every week on the days in the mask: bit 0 is Sunday, bit 6 Saturday.
    Weekly(u8),
    /// Every month on the position's weekday (0 is Sunday).
    Monthly(Position, u8),
}

impl Repeat {
    /// The word a library keeps: `once`, `w65`, `m1-0`, `m5-3` (5 is the last).
    pub fn code(self) -> String {
        match self {
            Repeat::Once => "once".to_string(),
            Repeat::Weekly(mask) => format!("w{mask}"),
            Repeat::Monthly(position, weekday) => format!("m{}-{weekday}", match position { Position::Nth(n) => n, Position::Last => 5 }),
        }
    }

    pub fn from_code(code: &str) -> Repeat {
        if let Some(mask) = code.strip_prefix('w').and_then(|mask| mask.parse().ok()) {
            return Repeat::Weekly(mask);
        }
        let monthly = code.strip_prefix('m').and_then(|rest| rest.split_once('-')).and_then(|(position, weekday)| Some((position.parse::<u8>().ok()?, weekday.parse::<u8>().ok()?)));
        match monthly {
            Some((5, weekday)) => Repeat::Monthly(Position::Last, weekday % 7),
            Some((n @ 1..=4, weekday)) => Repeat::Monthly(Position::Nth(n), weekday % 7),
            _ => Repeat::Once,
        }
    }

    /// Whether the weekly mask holds a weekday.
    pub fn on_weekday(mask: u8, weekday: u8) -> bool {
        mask & (1 << (weekday % 7)) != 0
    }
}

/// A named, coloured group of events. The colour belongs to the calendar, never to one event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    pub id: i64,
    pub name: String,
    pub colour: [u8; 3],
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
    /// Dates the rule gives that are left out.
    pub skipped: Vec<Date>,
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
        Repeat::Weekly(mask) => (from.days()..=to.days()).map(Date::from_days).filter(|date| Repeat::on_weekday(mask, date.weekday())).collect(),
        Repeat::Monthly(position, weekday) => {
            let months = (to.year - from.year) * 12 + i32::from(to.month) - i32::from(from.month);
            (0..=months)
                .map(|step| from.first_of_month().plus_months(step))
                .map(|month| nth_weekday(month.year, month.month, position, weekday))
                .filter(|date| *date >= from && *date <= to)
                .collect()
        }
    }
}

/// The dates an event happens on from `from` to `to`, both included, less the skipped ones.
pub fn occurrences(event: &Event, from: Date, to: Date) -> Vec<Date> {
    let mut dates = rule_dates(event, from, to);
    dates.retain(|date| !event.skipped.contains(date));
    dates
}

/// The first `count` dates the rule gives from `from` on, skipped ones included, looking at most
/// three years ahead.
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
        Event { id: 0, calendar: 1, name: "Service".into(), date: first, start: 9 * 60, end: 10 * 60, repeat, until: None, skipped: Vec::new() }
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
        let service = event(Repeat::Weekly(1 << SUNDAY), date(2026, 12, 6));
        let found = occurrences(&service, date(2026, 12, 20), date(2027, 1, 10));
        assert_eq!(found, vec![date(2026, 12, 20), date(2026, 12, 27), date(2027, 1, 3), date(2027, 1, 10)]);
        // Two days a week, across the end of a 30-day month.
        let two = event(Repeat::Weekly(1 << SUNDAY | 1 << WEDNESDAY), date(2026, 1, 1));
        let found = occurrences(&two, date(2026, 11, 25), date(2026, 12, 3));
        assert_eq!(found, vec![date(2026, 11, 25), date(2026, 11, 29), date(2026, 12, 2)]);
        // Never before its first date, and a first date that is no listed weekday starts on the next one.
        let late = event(Repeat::Weekly(1 << SUNDAY), date(2026, 11, 14));
        assert_eq!(occurrences(&late, date(2026, 11, 1), date(2026, 11, 22)), vec![date(2026, 11, 15), date(2026, 11, 22)]);
        assert!(occurrences(&event(Repeat::Weekly(0), date(2026, 1, 1)), date(2026, 1, 1), date(2026, 12, 31)).is_empty(), "no day, no event");
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
        let monthly = event(Repeat::Monthly(Position::Last, SUNDAY), date(2026, 1, 1));
        assert_eq!(occurrences(&monthly, date(2026, 2, 1), date(2026, 5, 31)), vec![date(2026, 2, 22), date(2026, 3, 29), date(2026, 4, 26), date(2026, 5, 31)]);
        // A year boundary, and a first date inside the month cuts the month's own date off.
        let first = event(Repeat::Monthly(Position::Nth(1), SUNDAY), date(2026, 12, 7));
        assert_eq!(occurrences(&first, date(2026, 12, 1), date(2027, 2, 28)), vec![date(2027, 1, 3), date(2027, 2, 7)]);
    }

    #[test]
    fn until_ends_a_rule_and_a_skipped_date_is_left_out() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY), date(2026, 11, 1));
        service.until = Some(date(2026, 11, 22));
        assert_eq!(occurrences(&service, date(2026, 11, 1), date(2027, 1, 1)), vec![date(2026, 11, 1), date(2026, 11, 8), date(2026, 11, 15), date(2026, 11, 22)], "the last day itself is in");
        service.skipped = vec![date(2026, 11, 8), date(2026, 12, 6)];
        assert_eq!(occurrences(&service, date(2026, 11, 1), date(2027, 1, 1)), vec![date(2026, 11, 1), date(2026, 11, 15), date(2026, 11, 22)]);
        // The rule still lists the skipped date, so the dialog can offer it back.
        assert!(rule_dates(&service, date(2026, 11, 1), date(2026, 11, 30)).contains(&date(2026, 11, 8)));
        let once = Event { skipped: vec![date(2026, 11, 14)], ..event(Repeat::Once, date(2026, 11, 14)) };
        assert!(occurrences(&once, date(2026, 11, 1), date(2026, 11, 30)).is_empty());
        let once = event(Repeat::Once, date(2026, 11, 14));
        assert_eq!(occurrences(&once, date(2026, 11, 14), date(2026, 11, 14)), vec![date(2026, 11, 14)]);
        assert!(occurrences(&once, date(2026, 11, 15), date(2026, 12, 31)).is_empty());
    }

    #[test]
    fn upcoming_lists_the_next_dates_with_the_skipped_ones() {
        let mut service = event(Repeat::Weekly(1 << SUNDAY), date(2026, 1, 4));
        service.skipped = vec![date(2026, 11, 22)];
        let next = upcoming(&service, date(2026, 11, 14), 3);
        assert_eq!(next, vec![date(2026, 11, 15), date(2026, 11, 22), date(2026, 11, 29)]);
        service.until = Some(date(2026, 11, 16));
        assert_eq!(upcoming(&service, date(2026, 11, 14), 3), vec![date(2026, 11, 15)]);
        let yearly = event(Repeat::Monthly(Position::Nth(1), SUNDAY), date(2026, 1, 1));
        assert_eq!(upcoming(&yearly, date(2026, 11, 14), 2), vec![date(2026, 12, 6), date(2027, 1, 3)]);
    }

    #[test]
    fn a_rule_is_kept_as_a_word_and_read_back() {
        for repeat in [Repeat::Once, Repeat::Weekly(65), Repeat::Weekly(127), Repeat::Monthly(Position::Nth(1), 0), Repeat::Monthly(Position::Nth(4), 6), Repeat::Monthly(Position::Last, 3)] {
            assert_eq!(Repeat::from_code(&repeat.code()), repeat);
        }
        assert_eq!(Repeat::from_code("nonsense"), Repeat::Once);
        assert_eq!(Repeat::from_code("m9-1"), Repeat::Once);
        assert_eq!(hour_minute(6 * 60 + 30), (6, 30));
    }
}
