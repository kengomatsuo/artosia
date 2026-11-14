//! What the Members window works out from a person's date of birth: the next birthday, the age
//! and the group the age gives. Nothing here reaches a slide.

use crate::calendar::Date;
use crate::deck::Person;

/// The group an age gives: a child is under 13, a youth under 25, the rest are adults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Child,
    Youth,
    Adult,
}

impl Group {
    pub fn of_age(age: u32) -> Group {
        match age {
            0..=12 => Group::Child,
            13..=24 => Group::Youth,
            _ => Group::Adult,
        }
    }
}

impl Person {
    /// The birthday in `year`. A 29 February falls on the 28th in a year that has no 29th.
    fn on(&self, year: i32) -> Date {
        let month = self.month.clamp(1, 12);
        Date { year, month, day: self.day.clamp(1, Date::month_days(year, month)) }
    }

    /// The birthday on or after `today`.
    pub fn next_birthday(&self, today: Date) -> Date {
        let this_year = self.on(today.year);
        if this_year >= today { this_year } else { self.on(today.year + 1) }
    }

    /// Days from `today` to the next birthday; 0 on the day itself.
    pub fn days_until(&self, today: Date) -> u32 {
        (self.next_birthday(today).days() - today.days()) as u32
    }

    /// The age in whole years on `today`; None when the year of birth is not known or is still to come.
    pub fn age(&self, today: Date) -> Option<u32> {
        if self.year == 0 || i32::from(self.year) > today.year {
            return None;
        }
        let reached = self.on(today.year) <= today;
        Some((today.year - i32::from(self.year)) as u32 - u32::from(!reached))
    }

    pub fn group(&self, today: Date) -> Option<Group> {
        self.age(today).map(Group::of_age)
    }
}

/// How many people have a birthday in each month, January first.
pub fn month_counts(people: &[Person]) -> [usize; 12] {
    let mut counts = [0; 12];
    for person in people {
        counts[usize::from(person.month.clamp(1, 12)) - 1] += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> Date {
        Date::new(year, month, day).unwrap_or(Date { year, month, day })
    }

    fn person(month: u8, day: u8, year: u16) -> Person {
        Person { name: "Test".into(), day, month, year, ..Person::default() }
    }

    #[test]
    fn the_next_birthday_is_today_or_later_and_wraps_over_new_year() {
        let today = date(2026, 11, 14);
        assert_eq!(person(11, 14, 1990).next_birthday(today), today);
        assert_eq!(person(11, 14, 1990).days_until(today), 0);
        assert_eq!(person(11, 16, 0).days_until(today), 2);
        assert_eq!(person(11, 13, 0).next_birthday(today), date(2027, 11, 13));
        assert_eq!(person(1, 2, 0).days_until(today), 49);
    }

    #[test]
    fn a_leap_day_is_kept_on_the_28th_in_other_years() {
        let born = person(2, 29, 2000);
        assert_eq!(born.next_birthday(date(2027, 1, 10)), date(2027, 2, 28));
        assert_eq!(born.next_birthday(date(2028, 1, 10)), date(2028, 2, 29));
        assert_eq!(born.age(date(2027, 2, 27)), Some(26));
        assert_eq!(born.age(date(2027, 2, 28)), Some(27));
    }

    #[test]
    fn the_age_counts_whole_years_and_is_none_without_a_year() {
        assert_eq!(person(11, 15, 2000).age(date(2026, 11, 14)), Some(25));
        assert_eq!(person(11, 14, 2000).age(date(2026, 11, 14)), Some(26));
        assert_eq!(person(1, 1, 0).age(date(2026, 11, 14)), None);
        assert_eq!(person(1, 1, 2030).age(date(2026, 11, 14)), None);
        assert_eq!(person(1, 1, 0).group(date(2026, 11, 14)), None);
    }

    #[test]
    fn the_group_changes_at_thirteen_and_twenty_five() {
        for (age, group) in [(0, Group::Child), (12, Group::Child), (13, Group::Youth), (24, Group::Youth), (25, Group::Adult), (90, Group::Adult)] {
            assert_eq!(Group::of_age(age), group, "age {age}");
        }
    }

    #[test]
    fn every_person_is_counted_in_their_month() {
        let counts = month_counts(&[person(1, 1, 0), person(1, 9, 0), person(12, 31, 0)]);
        assert_eq!((counts[0], counts[11], counts.iter().sum::<usize>()), (2, 1, 3));
    }
}
