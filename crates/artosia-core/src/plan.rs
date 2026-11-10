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
    pub items: Vec<PlanItem>,
    next_id: u64,
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
        Plan { id: 0, name: name.into(), items: Vec::new(), next_id: 1 }
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
