//! The service plan: what will be shown today, in order.

use serde::{Deserialize, Serialize};

use crate::bible::Reference;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanEntry {
    /// `title` is kept so a deleted song still reads as itself in the plan.
    Song { song_id: i64, title: String },
    Passage { reference: Reference, bible: String, second: Option<String>, title: String },
    /// Every announcement, looping.
    Announcements,
    /// A divider such as "Worship" or "Word".
    Heading { title: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanItem {
    /// Stable inside one plan, so the list can reorder without losing selection.
    pub id: u64,
    pub entry: PlanEntry,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub id: i64,
    pub name: String,
    /// The day of the service as year-month-day, or empty when the plan has no day.
    #[serde(default)]
    pub date: String,
    /// Unix seconds when the plan last became the open one; the list of plans is ordered by it.
    #[serde(default)]
    pub used: i64,
    pub items: Vec<PlanItem>,
    next_id: u64,
}

/// A day as year-month-day with a real month and day, or None. Accepts `-`, `/` and `.` between
/// the parts and returns it with `-` and two digits.
pub fn valid_date(text: &str) -> Option<String> {
    let parts: Vec<&str> = text.trim().split(['-', '/', '.']).collect();
    let [year, month, day] = parts[..] else { return None };
    if year.len() != 4 {
        return None;
    }
    let (year, month, day): (u32, u32, u32) = (year.parse().ok()?, month.parse().ok()?, day.parse().ok()?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

impl PlanEntry {
    pub fn title(&self) -> &str {
        match self {
            PlanEntry::Song { title, .. } | PlanEntry::Passage { title, .. } | PlanEntry::Heading { title } => title,
            PlanEntry::Announcements => "Announcements",
        }
    }

    pub fn is_heading(&self) -> bool {
        matches!(self, PlanEntry::Heading { .. })
    }
}

impl Plan {
    pub fn named(name: impl Into<String>) -> Plan {
        Plan { id: 0, name: name.into(), date: String::new(), used: 0, items: Vec::new(), next_id: 1 }
    }

    /// A new plan holding the same items, for another service. It has no day and is not saved yet.
    pub fn copy_named(&self, name: impl Into<String>) -> Plan {
        Plan { id: 0, name: name.into(), date: String::new(), used: 0, items: self.items.clone(), next_id: self.next_id }
    }

    /// The month and day of the plan's date, when it has one.
    pub fn month_day(&self) -> Option<(u32, u32)> {
        let date = valid_date(&self.date)?;
        let mut parts = date.split('-').skip(1).filter_map(|part| part.parse().ok());
        Some((parts.next()?, parts.next()?))
    }

    /// Appends an entry and returns its item id.
    pub fn push(&mut self, entry: PlanEntry) -> u64 {
        self.insert(self.items.len(), entry)
    }

    pub fn insert(&mut self, at: usize, entry: PlanEntry) -> u64 {
        let id = self.next_id.max(self.items.iter().map(|item| item.id + 1).max().unwrap_or(1));
        self.next_id = id + 1;
        self.items.insert(at.min(self.items.len()), PlanItem { id, entry });
        id
    }

    pub fn position(&self, id: u64) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<(usize, PlanItem)> {
        let at = self.position(id)?;
        Some((at, self.items.remove(at)))
    }

    /// Moves an item up (`-1`) or down (`1`); false when it is already at that end.
    pub fn shift(&mut self, id: u64, by: isize) -> bool {
        let Some(at) = self.position(id) else { return false };
        let to = at as isize + by;
        if to < 0 || to as usize >= self.items.len() {
            return false;
        }
        self.items.swap(at, to as usize);
        true
    }

    /// Moves an item to a new index, as a drag does.
    pub fn move_to(&mut self, id: u64, to: usize) {
        if let Some(at) = self.position(id) {
            let item = self.items.remove(at);
            self.items.insert(to.min(self.items.len()), item);
        }
    }

    /// The next item after `id` that can be shown, skipping headings.
    pub fn next_showable(&self, id: u64) -> Option<&PlanItem> {
        let at = self.position(id)?;
        self.items[at + 1..].iter().find(|item| !item.entry.is_heading())
    }

    pub fn previous_showable(&self, id: u64) -> Option<&PlanItem> {
        let at = self.position(id)?;
        self.items[..at].iter().rev().find(|item| !item.entry.is_heading())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song(id: i64) -> PlanEntry {
        PlanEntry::Song { song_id: id, title: format!("Song {id}") }
    }

    #[test]
    fn ids_stay_unique_after_removal_and_reload() {
        let mut plan = Plan::named("Sunday");
        let a = plan.push(song(1));
        let b = plan.push(song(2));
        plan.remove(a);
        let json = serde_json::to_string(&plan).unwrap_or_default();
        let mut loaded: Plan = serde_json::from_str(&json).unwrap_or_default();
        let c = loaded.push(song(3));
        assert!(c != b && c != a);
    }

    #[test]
    fn a_plan_saved_before_names_and_dates_still_loads() {
        // The shape the library held until 2026-11-11: no date, no last-used time.
        let old = r#"{"id":0,"name":"Today","items":[{"id":1,"entry":"Announcements"},{"id":2,"entry":{"Heading":{"title":"Word"}}}],"next_id":3}"#;
        let plan: Plan = serde_json::from_str(old).expect("an old plan loads");
        assert_eq!((plan.name.as_str(), plan.date.as_str(), plan.used, plan.items.len()), ("Today", "", 0, 2));
    }

    #[test]
    fn a_copy_keeps_the_items_and_starts_without_a_day() {
        let mut plan = Plan::named("Sunday 7");
        plan.id = 4;
        plan.date = "2026-11-15".into();
        let a = plan.push(song(1));
        let mut copy = plan.copy_named("Sunday 10");
        assert_eq!((copy.id, copy.date.as_str(), copy.items.len()), (0, "", 1));
        // Items added to the copy never reuse an id the original gave out.
        let b = copy.push(song(2));
        assert_ne!(a, b);
    }

    #[test]
    fn dates_are_read_leniently_and_kept_one_way() {
        assert_eq!(valid_date("2026-11-15").as_deref(), Some("2026-11-15"));
        assert_eq!(valid_date(" 2026/1/5 ").as_deref(), Some("2026-01-05"));
        assert_eq!(valid_date("2026-13-01"), None);
        assert_eq!(valid_date("15-11-2026"), None);
        assert_eq!(valid_date(""), None);
        let mut plan = Plan::named("Youth");
        plan.date = "2026-11-14".into();
        assert_eq!(plan.month_day(), Some((11, 14)));
    }

    #[test]
    fn next_skips_headings() {
        let mut plan = Plan::named("Sunday");
        let a = plan.push(song(1));
        plan.push(PlanEntry::Heading { title: "Word".into() });
        let c = plan.push(song(2));
        assert_eq!(plan.next_showable(a).map(|item| item.id), Some(c));
        assert_eq!(plan.previous_showable(c).map(|item| item.id), Some(a));
        assert!(plan.next_showable(c).is_none());
    }

    #[test]
    fn shift_and_move_reorder() {
        let mut plan = Plan::named("Sunday");
        let a = plan.push(song(1));
        let b = plan.push(song(2));
        let c = plan.push(song(3));
        assert!(plan.shift(c, -1));
        assert!(!plan.shift(a, -1));
        plan.move_to(a, 2);
        let order: Vec<u64> = plan.items.iter().map(|item| item.id).collect();
        assert_eq!(order, [c, b, a]);
    }
}
