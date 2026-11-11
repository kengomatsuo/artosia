//! The service plan: what will be shown today, in order.

use serde::{Deserialize, Serialize};

use crate::bible::Reference;
use crate::deck::Deck;
use crate::song::Tag;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanEntry {
    /// `title` is kept so a deleted song still reads as itself in the plan. `start` is the
    /// section the song begins at, as a medley goes straight into the next song's chorus. It is
    /// a section's tag and not a slide number, so it survives the song being edited.
    Song {
        song_id: i64,
        title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<Tag>,
    },
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

    /// A song entry with no start set: it begins at its first slide.
    pub fn song(song_id: i64, title: impl Into<String>) -> PlanEntry {
        PlanEntry::Song { song_id, title: title.into(), start: None }
    }

    /// The section a song entry was set to begin at; None for the beginning and for any other entry.
    pub fn start(&self) -> Option<Tag> {
        match self {
            PlanEntry::Song { start, .. } => *start,
            _ => None,
        }
    }
}

/// The slide of a deck that a plan item begins at: the first slide of the section `start` names,
/// and the first slide when there is no start or the song no longer has that section.
pub fn start_slide(deck: &Deck, start: Option<Tag>) -> usize {
    start.and_then(|tag| deck.slides.iter().position(|slide| slide.tag == Some(tag))).unwrap_or(0)
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
        PlanEntry::song(id, format!("Song {id}"))
    }

    fn practice_song(lyrics: &str) -> crate::song::Song {
        crate::song::Song { id: 1, title: "Practice".into(), lyrics: lyrics.into(), ..crate::song::Song::default() }
    }

    const CHORUS: Tag = Tag::new(crate::song::SectionKind::Chorus, 1);

    #[test]
    fn a_plan_saved_before_starts_still_loads_and_begins_at_the_top() {
        let old = r#"{"id":3,"name":"Sunday","items":[{"id":1,"entry":{"Song":{"song_id":7,"title":"Old"}}}],"next_id":2}"#;
        let plan: Plan = serde_json::from_str(old).expect("an old plan loads");
        assert_eq!(plan.items[0].entry.start(), None);
        // A plan with no starts is written as it always was.
        assert!(!serde_json::to_string(&plan).unwrap_or_default().contains("start"));
    }

    #[test]
    fn a_start_is_saved_and_follows_its_section_through_an_edit() {
        let mut plan = Plan::named("Sunday");
        plan.push(PlanEntry::Song { song_id: 1, title: "Practice".into(), start: Some(CHORUS) });
        let json = serde_json::to_string(&plan).unwrap_or_default();
        let loaded: Plan = serde_json::from_str(&json).expect("loads");
        let start = loaded.items[0].entry.start();
        assert_eq!(start, Some(CHORUS));
        let before = practice_song("[1]\nFirst line\n\n[chorus]\nChorus line");
        let deck = crate::deck::song_deck(&before, 4, "");
        assert_eq!(start_slide(&deck, start), 1);
        // A verse is written in front of it: the chorus moved, the start moved with it.
        let after = practice_song("[1]\nFirst line\n\n[2]\nSecond line\n\n[chorus]\nChorus line");
        let deck = crate::deck::song_deck(&after, 4, "");
        assert_eq!(start_slide(&deck, start), 2);
        // The chorus is cut: the song begins at the top again.
        let without = practice_song("[1]\nFirst line\n\n[2]\nSecond line");
        let deck = crate::deck::song_deck(&without, 4, "");
        assert_eq!(start_slide(&deck, start), 0);
        assert_eq!(start_slide(&deck, None), 0);
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
