//! The church year as dates: Easter, and the season a day falls in (Western churches, Gregorian
//! calendar; the rules and their sources are in `docs/church-year.md`).

use crate::calendar::Date;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Season {
    Advent,
    Christmas,
    /// 6 January.
    Epiphany,
    Lent,
    /// Palm Sunday to Holy Saturday, Good Friday apart.
    HolyWeek,
    GoodFriday,
    /// Easter Sunday to the day before Pentecost, Ascension apart.
    Easter,
    Ascension,
    /// Pentecost and the week after it.
    Pentecost,
    /// The Sunday before 31 October.
    Reformation,
    Ordinary,
}

impl Season {
    /// The name a colourway's `season` tag carries.
    pub fn code(self) -> &'static str {
        match self {
            Season::Advent => "advent",
            Season::Christmas => "christmas",
            Season::Epiphany => "epiphany",
            Season::Lent => "lent",
            Season::HolyWeek => "holy-week",
            Season::GoodFriday => "good-friday",
            Season::Easter => "easter",
            Season::Ascension => "ascension",
            Season::Pentecost => "pentecost",
            Season::Reformation => "reformation",
            Season::Ordinary => "ordinary",
        }
    }
}

/// Easter Sunday of `year` (the Meeus/Jones/Butcher algorithm).
pub fn easter(year: i32) -> Date {
    let (a, b, c) = (year % 19, year / 100, year % 100);
    let (d, e, f) = (b / 4, b % 4, (b + 8) / 25);
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    Date { year, month: month as u8, day: day as u8 }
}

/// The last Sunday before `date`.
fn sunday_before(date: Date) -> Date {
    date.plus_days(-(i64::from(date.weekday()) + if date.weekday() == 0 { 7 } else { 0 }))
}

/// The first Sunday of Advent: four Sundays before the Sunday on or before Christmas Eve.
pub fn advent_start(year: i32) -> Date {
    sunday_before(Date { year, month: 12, day: 25 }).plus_days(-21)
}

/// Ash Wednesday, 46 days before Easter.
pub fn ash_wednesday(year: i32) -> Date {
    easter(year).plus_days(-46)
}

/// The Sunday before 31 October.
pub fn reformation_sunday(year: i32) -> Date {
    sunday_before(Date { year, month: 10, day: 31 })
}

pub fn season_on(date: Date) -> Season {
    let year = date.year;
    if date >= (Date { year, month: 12, day: 24 }) {
        return Season::Christmas;
    }
    if date >= advent_start(year) {
        return Season::Advent;
    }
    if date == reformation_sunday(year) {
        return Season::Reformation;
    }
    let epiphany = Date { year, month: 1, day: 6 };
    if date == epiphany {
        return Season::Epiphany;
    }
    // The Sunday after Epiphany ends Christmastide (the Baptism of the Lord).
    let baptism = epiphany.plus_days(7 - i64::from(epiphany.weekday()));
    if date <= baptism {
        return Season::Christmas;
    }
    match date.days() - easter(year).days() {
        -46..=-8 => Season::Lent,
        -2 => Season::GoodFriday,
        -7..=-1 => Season::HolyWeek,
        39 => Season::Ascension,
        49..=55 => Season::Pentecost,
        0..=48 => Season::Easter,
        _ => Season::Ordinary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap()
    }

    #[test]
    fn easter_falls_on_the_known_dates() {
        for (year, month, day) in [(2024, 3, 31), (2025, 4, 20), (2026, 4, 5), (2027, 3, 28), (2028, 4, 16), (2038, 4, 25)] {
            assert_eq!(easter(year), d(year, month, day), "{year}");
        }
    }

    #[test]
    fn the_movable_days_fall_on_the_known_dates() {
        assert_eq!(advent_start(2026), d(2026, 11, 29));
        assert_eq!(advent_start(2027), d(2027, 11, 28));
        assert_eq!(ash_wednesday(2026), d(2026, 2, 18));
        assert_eq!(easter(2026).plus_days(49), d(2026, 5, 24));
        assert_eq!(reformation_sunday(2026), d(2026, 10, 25));
        // 31 October a Sunday: the Sunday before it, a week earlier.
        assert_eq!(reformation_sunday(2027), d(2027, 10, 24));
    }

    #[test]
    fn a_day_is_in_its_season() {
        let cases = [
            (d(2026, 11, 28), Season::Ordinary),
            (d(2026, 11, 29), Season::Advent),
            (d(2026, 12, 23), Season::Advent),
            (d(2026, 12, 24), Season::Christmas),
            (d(2027, 1, 1), Season::Christmas),
            (d(2027, 1, 6), Season::Epiphany),
            (d(2027, 1, 10), Season::Christmas),
            (d(2027, 1, 11), Season::Ordinary),
            (d(2026, 2, 17), Season::Ordinary),
            (d(2026, 2, 18), Season::Lent),
            (d(2026, 3, 28), Season::Lent),
            (d(2026, 3, 29), Season::HolyWeek),
            (d(2026, 4, 2), Season::HolyWeek),
            (d(2026, 4, 3), Season::GoodFriday),
            (d(2026, 4, 4), Season::HolyWeek),
            (d(2026, 4, 5), Season::Easter),
            (d(2026, 5, 14), Season::Ascension),
            (d(2026, 5, 23), Season::Easter),
            (d(2026, 5, 24), Season::Pentecost),
            (d(2026, 5, 30), Season::Pentecost),
            (d(2026, 5, 31), Season::Ordinary),
            (d(2026, 10, 24), Season::Ordinary),
            (d(2026, 10, 25), Season::Reformation),
        ];
        for (date, season) in cases {
            assert_eq!(season_on(date), season, "{}", date.code());
        }
    }
}
