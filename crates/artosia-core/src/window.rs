//! The dates a card reads: a range (a week, a month, a quarter, the days before or after today, or
//! two dates), resolved from today and the church's first weekday, and a highlighted part of it.
//! The schedule card, the birthday card, the pill and the panel all read one `Window::resolve`.

use serde::{Deserialize, Serialize};

use crate::calendar::Date;

/// The longest a custom range runs; a longer one is cut here and said to be.
pub const LONGEST: i64 = 366;

/// A stretch of days.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Range {
    /// The week that holds today, moved by this many weeks.
    Week(i32),
    /// The calendar month that holds today, moved by this many months.
    Month(i32),
    /// The calendar quarter that holds today, moved by this many quarters.
    Quarter(i32),
    /// Today and the days after it, this many in all.
    Coming(u16),
    /// The days before today, this many, ending yesterday.
    Past(u16),
    /// Two dates, both included, in either order.
    Custom(Date, Date),
}

/// What a card reads and what stands out in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Window {
    pub range: Range,
    /// A part of the range drawn to stand out; one that falls outside it is cut to fit.
    pub highlight: Option<Range>,
}

impl Default for Window {
    /// The seven days from today, which is what a card read before windows existed.
    fn default() -> Window {
        Window { range: Range::Coming(7), highlight: None }
    }
}

/// A window's dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shown {
    pub from: Date,
    pub to: Date,
    /// The highlighted days inside `from` to `to`; None for no highlight or one wholly outside.
    pub highlight: Option<(Date, Date)>,
    /// The highlight, or the range itself, had to be cut to fit.
    pub clamped: bool,
}

impl Range {
    /// The first and last day, both included. `first` is the church's first weekday, 0 for Sunday.
    pub fn resolve(self, today: Date, first: u8) -> (Date, Date) {
        match self {
            Range::Week(by) => {
                let start = today.plus_days(-i64::from((today.weekday() + 7 - first % 7) % 7) + 7 * i64::from(by));
                (start, start.plus_days(6))
            }
            Range::Month(by) => {
                let start = today.first_of_month().plus_months(by);
                (start, start.plus_months(1).plus_days(-1))
            }
            Range::Quarter(by) => {
                let start = Date { month: (today.month - 1) / 3 * 3 + 1, ..today.first_of_month() }.plus_months(3 * by);
                (start, start.plus_months(3).plus_days(-1))
            }
            Range::Coming(days) => (today, today.plus_days(i64::from(days.max(1)) - 1)),
            Range::Past(days) => (today.plus_days(-i64::from(days.max(1))), today.plus_days(-1)),
            Range::Custom(a, b) => {
                let (from, to) = (a.min(b), a.max(b));
                (from, to.min(from.plus_days(LONGEST - 1)))
            }
        }
    }

    /// The word a library keeps.
    pub fn code(self) -> String {
        match self {
            Range::Week(by) => format!("week:{by}"),
            Range::Month(by) => format!("month:{by}"),
            Range::Quarter(by) => format!("quarter:{by}"),
            Range::Coming(days) => format!("coming:{days}"),
            Range::Past(days) => format!("past:{days}"),
            Range::Custom(from, to) => format!("custom:{}..{}", from.code(), to.code()),
        }
    }

    pub fn from_code(code: &str) -> Option<Range> {
        let (kind, value) = code.split_once(':')?;
        Some(match kind {
            "week" => Range::Week(value.parse().ok()?),
            "month" => Range::Month(value.parse().ok()?),
            "quarter" => Range::Quarter(value.parse().ok()?),
            "coming" => Range::Coming(value.parse().ok()?),
            "past" => Range::Past(value.parse().ok()?),
            "custom" => {
                let (from, to) = value.split_once("..")?;
                Range::Custom(Date::parse(from)?, Date::parse(to)?)
            }
            _ => return None,
        })
    }
}

impl Window {
    /// The days the window covers today and the days that stand out, the second cut to the first.
    pub fn resolve(&self, today: Date, first: u8) -> Shown {
        let (from, to) = self.range.resolve(today, first);
        let cut = matches!(self.range, Range::Custom(a, b) if a.min(b) != from || a.max(b) != to);
        let Some(wanted) = self.highlight else { return Shown { from, to, highlight: None, clamped: cut } };
        let (start, end) = wanted.resolve(today, first);
        let (start, end) = (start.max(from), end.min(to));
        let inside = start <= end;
        let (full_start, full_end) = wanted.resolve(today, first);
        Shown { from, to, highlight: inside.then_some((start, end)), clamped: cut || !inside || (full_start, full_end) != (start, end) }
    }

    /// The window a card made before windows existed read: its data source's word and its month flag.
    pub fn migrated(fill_code: &str, month: bool) -> Window {
        let range = if fill_code == "birthdays-month" || (month && fill_code != "birthdays-week") { Range::Month(0) } else { Range::Coming(7) };
        Window { range, highlight: None }
    }

    /// What a library keeps: the range and the highlight, the second empty for none.
    pub fn codes(&self) -> (String, String) {
        (self.range.code(), self.highlight.map(Range::code).unwrap_or_default())
    }

    /// The window a library's two words give; None when the range is empty or unreadable.
    pub fn from_codes(range: &str, highlight: &str) -> Option<Window> {
        Some(Window { range: Range::from_code(range)?, highlight: Range::from_code(highlight) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).expect("a real date")
    }

    fn span(range: Range, today: Date, first: u8) -> ((i32, u8, u8), (i32, u8, u8)) {
        let (from, to) = range.resolve(today, first);
        ((from.year, from.month, from.day), (to.year, to.month, to.day))
    }

    #[test]
    fn a_week_starts_on_the_churchs_first_weekday() {
        // 14 November 2026 is a Saturday.
        let saturday = day(2026, 11, 14);
        assert_eq!(span(Range::Week(0), saturday, 0), ((2026, 11, 8), (2026, 11, 14)));
        assert_eq!(span(Range::Week(1), saturday, 0), ((2026, 11, 15), (2026, 11, 21)));
        assert_eq!(span(Range::Week(0), saturday, 1), ((2026, 11, 9), (2026, 11, 15)));
        // On the first weekday itself, the week begins today.
        assert_eq!(span(Range::Week(0), day(2026, 11, 8), 0), ((2026, 11, 8), (2026, 11, 14)));
        assert_eq!(span(Range::Week(-1), day(2026, 11, 8), 0), ((2026, 11, 1), (2026, 11, 7)));
        // A week over a month end and over New Year.
        assert_eq!(span(Range::Week(0), day(2026, 12, 31), 0), ((2026, 12, 27), (2027, 1, 2)));
        assert_eq!(span(Range::Week(1), day(2026, 12, 31), 1), ((2027, 1, 4), (2027, 1, 10)));
    }

    #[test]
    fn a_month_and_a_quarter_end_where_the_calendar_does() {
        assert_eq!(span(Range::Month(0), day(2026, 11, 14), 0), ((2026, 11, 1), (2026, 11, 30)));
        assert_eq!(span(Range::Month(1), day(2026, 12, 31), 0), ((2027, 1, 1), (2027, 1, 31)));
        assert_eq!(span(Range::Month(1), day(2026, 1, 31), 0), ((2026, 2, 1), (2026, 2, 28)));
        // Leap February, and the day it was entered from the 29th.
        assert_eq!(span(Range::Month(0), day(2028, 2, 29), 0), ((2028, 2, 1), (2028, 2, 29)));
        assert_eq!(span(Range::Month(1), day(2028, 1, 31), 0), ((2028, 2, 1), (2028, 2, 29)));
        assert_eq!(span(Range::Month(-1), day(2028, 3, 31), 0), ((2028, 2, 1), (2028, 2, 29)));
        assert_eq!(span(Range::Quarter(0), day(2026, 11, 14), 0), ((2026, 10, 1), (2026, 12, 31)));
        assert_eq!(span(Range::Quarter(1), day(2026, 11, 14), 0), ((2027, 1, 1), (2027, 3, 31)));
        assert_eq!(span(Range::Quarter(1), day(2028, 1, 1), 0), ((2028, 4, 1), (2028, 6, 30)));
        assert_eq!(span(Range::Quarter(0), day(2028, 2, 29), 0), ((2028, 1, 1), (2028, 3, 31)));
        assert_eq!(span(Range::Quarter(-1), day(2026, 2, 3), 0), ((2025, 10, 1), (2025, 12, 31)));
    }

    #[test]
    fn the_days_before_and_after_today_count_today_once() {
        let today = day(2026, 12, 28);
        assert_eq!(span(Range::Coming(7), today, 0), ((2026, 12, 28), (2027, 1, 3)));
        assert_eq!(span(Range::Coming(1), today, 0), ((2026, 12, 28), (2026, 12, 28)));
        assert_eq!(span(Range::Past(7), today, 0), ((2026, 12, 21), (2026, 12, 27)));
        assert_eq!(span(Range::Coming(0), today, 0), span(Range::Coming(1), today, 0));
        assert_eq!(span(Range::Coming(30), day(2028, 2, 1), 0), ((2028, 2, 1), (2028, 3, 1)));
        assert_eq!(span(Range::Past(7), day(2028, 3, 3), 0), ((2028, 2, 25), (2028, 3, 2)));
    }

    #[test]
    fn a_custom_range_takes_either_order_and_stops_at_a_year() {
        let (a, b) = (day(2026, 11, 20), day(2026, 11, 3));
        assert_eq!(span(Range::Custom(a, b), day(2026, 1, 1), 0), ((2026, 11, 3), (2026, 11, 20)));
        let long = Window { range: Range::Custom(day(2026, 1, 1), day(2029, 1, 1)), highlight: None }.resolve(day(2026, 6, 1), 0);
        // 366 days from 1 January 2026 reach 1 January 2027.
        assert_eq!((long.to, long.clamped), (day(2027, 1, 1), true));
        let leap = Window { range: Range::Custom(day(2028, 1, 1), day(2029, 1, 1)), highlight: None }.resolve(day(2028, 6, 1), 0);
        assert_eq!((leap.to, leap.clamped), (day(2028, 12, 31), true));
        let year = Window { range: Range::Custom(day(2027, 1, 1), day(2027, 12, 31)), highlight: None }.resolve(day(2027, 6, 1), 0);
        assert_eq!((year.to, year.clamped), (day(2027, 12, 31), false));
    }

    #[test]
    fn a_highlight_is_cut_to_the_range_and_the_cut_is_said() {
        let today = day(2026, 11, 3);
        let month = |highlight| Window { range: Range::Month(0), highlight }.resolve(today, 0);
        // Inside: kept as it is.
        let inside = Window { range: Range::Month(0), highlight: Some(Range::Coming(7)) }.resolve(today, 0);
        assert_eq!((inside.highlight, inside.clamped), (Some((day(2026, 11, 3), day(2026, 11, 9))), false));
        // The past week of the 3rd reaches into October: only November's part stays.
        let past = month(Some(Range::Past(7)));
        assert_eq!((past.highlight, past.clamped), (Some((day(2026, 11, 1), day(2026, 11, 2))), true));
        // Wholly outside: no highlight at all, and it is said.
        let gone = month(Some(Range::Custom(day(2026, 12, 1), day(2026, 12, 5))));
        assert_eq!((gone.highlight, gone.clamped), (None, true));
        // The whole range is no cut.
        let whole = month(Some(Range::Month(0)));
        assert_eq!((whole.highlight, whole.clamped), (Some((day(2026, 11, 1), day(2026, 11, 30))), false));
        let none = month(None);
        assert_eq!((none.highlight, none.clamped), (None, false));
        // A next-week highlight in this week's range is wholly outside.
        let week = Window { range: Range::Week(0), highlight: Some(Range::Week(1)) }.resolve(today, 0);
        assert_eq!((week.highlight, week.clamped), (None, true));
    }

    #[test]
    fn a_card_made_before_windows_reads_as_it_did() {
        assert_eq!(Window::migrated("birthdays-week", false), Window { range: Range::Coming(7), highlight: None });
        assert_eq!(Window::migrated("birthdays-month", false), Window { range: Range::Month(0), highlight: None });
        assert_eq!(Window::migrated("schedule-week", true).range, Range::Month(0));
        assert_eq!(Window::migrated("schedule-week", false).range, Range::Coming(7));
        assert_eq!(Window::migrated("", false), Window::default());
        // The seven days of the old window were the seven days from today, not the week.
        let today = day(2026, 11, 14);
        assert_eq!(Window::migrated("schedule-week", false).resolve(today, 0).to, day(2026, 11, 20));
    }

    #[test]
    fn a_window_survives_the_library_words() {
        let windows = [
            Window::default(),
            Window { range: Range::Week(1), highlight: None },
            Window { range: Range::Month(0), highlight: Some(Range::Past(7)) },
            Window { range: Range::Quarter(-1), highlight: Some(Range::Custom(day(2026, 1, 2), day(2026, 1, 9))) },
            Window { range: Range::Custom(day(2026, 11, 1), day(2026, 11, 30)), highlight: Some(Range::Coming(1)) },
        ];
        for window in windows {
            let (range, highlight) = window.codes();
            assert_eq!(Window::from_codes(&range, &highlight), Some(window), "{range} {highlight}");
        }
        assert_eq!(Window::from_codes("", ""), None);
        assert_eq!(Window::from_codes("week:x", ""), None);
        assert_eq!(Window::from_codes("custom:2026-02-30..2026-03-01", ""), None);
        assert_eq!(Window::from_codes("coming:7", "nonsense"), Some(Window::default()));
    }
}
