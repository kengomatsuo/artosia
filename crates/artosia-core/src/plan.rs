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
    /// The day of the service as year-month-day, or empty when the plan has no day.
    #[serde(default)]
    pub date: String,
    /// Unix seconds when the plan last became the open one; the list of plans is ordered by it.
    #[serde(default)]
    pub used: i64,
    /// The day of the week the service is held, 0 for Monday to 6 for Sunday, when the plan
    /// is a service held every week. A plan made before days were picked has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<u8>,
    /// When that service starts, as minutes after midnight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<u16>,
    /// This plan's own look, as the operator window stores it (a serialized `artosia_slides::look::Look`,
    /// which this crate cannot name). None means the plan uses the usual look.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<serde_json::Value>,
    /// The languages this plan's songs show together, in order, when it has a choice of its own
    /// (a youth service in one language, a joint service in three). None follows the church's usual.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,
    pub items: Vec<PlanItem>,
    next_id: u64,
}

/// The day of the week of a date, 0 for Monday to 6 for Sunday (Sakamoto's method).
pub fn weekday(year: i32, month: u32, day: u32) -> u8 {
    const SHIFT: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let year = if month < 3 { year - 1 } else { year };
    let sunday_first = (year + year / 4 - year / 100 + year / 400 + SHIFT[(month.clamp(1, 12) - 1) as usize] + day as i32).rem_euclid(7);
    ((sunday_first + 6) % 7) as u8
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
        Plan { id: 0, name: name.into(), date: String::new(), used: 0, day: None, time: None, look: None, languages: None, items: Vec::new(), next_id: 1 }
    }

    /// A new plan holding the same items, for another service. It keeps the day of the week,
    /// has no time yet and is not saved yet. It keeps the look and the languages: a service that sings the same
    /// songs on the same screen looks the same.
    pub fn copy_named(&self, name: impl Into<String>) -> Plan {
        Plan { id: 0, name: name.into(), date: String::new(), used: 0, day: self.day, time: None, look: self.look.clone(), languages: self.languages.clone(), items: self.items.clone(), next_id: self.next_id }
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
        PlanEntry::song(id, format!("Song {id}"))
    }

    #[test]
    fn a_date_names_its_day_of_the_week() {
        // 11 November 2026 is a Wednesday, 15 November a Sunday, 1 January 2024 a Monday.
        assert_eq!((weekday(2026, 11, 11), weekday(2026, 11, 15), weekday(2024, 1, 1), weekday(2024, 2, 29)), (2, 6, 0, 3));
    }

    #[test]
    fn a_plan_saved_before_days_were_picked_loads_with_none() {
        let old = r#"{"id":3,"name":"Sunday","date":"2026-11-15","used":5,"items":[],"next_id":1}"#;
        let plan: Plan = serde_json::from_str(old).expect("an old plan loads");
        assert_eq!((plan.day, plan.time), (None, None));
        let mut weekly = Plan::named("");
        (weekly.day, weekly.time) = (Some(6), Some(7 * 60));
        let back: Plan = serde_json::from_str(&serde_json::to_string(&weekly).expect("a plan is written")).expect("and read back");
        assert_eq!((back.day, back.time), (Some(6), Some(420)));
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

    #[test]
    fn a_plan_may_carry_a_look_and_a_copy_keeps_it() {
        let old = r#"{"id":0,"name":"Today","items":[],"next_id":1}"#;
        let plan: Plan = serde_json::from_str(old).expect("an old plan loads");
        assert!(plan.look.is_none(), "a plan saved before looks has none");
        assert!(!serde_json::to_string(&plan).unwrap_or_default().contains("look"), "nothing is written for no look");
        let mut own = Plan::named("Sunday school");
        own.look = Some(serde_json::json!({"name": "Plain"}));
        let json = serde_json::to_string(&own).unwrap_or_default();
        let loaded: Plan = serde_json::from_str(&json).expect("round trip");
        assert_eq!(loaded.look, own.look);
        assert_eq!(own.copy_named("Copy").look, own.look);
    }

    #[test]
    fn a_plan_may_carry_its_own_languages_and_a_copy_keeps_them() {
        let old = r#"{"id":0,"name":"Today","items":[],"next_id":1}"#;
        let plan: Plan = serde_json::from_str(old).expect("an old plan loads");
        assert!(plan.languages.is_none() && !serde_json::to_string(&plan).unwrap_or_default().contains("languages"));
        let mut own = Plan::named("Youth");
        own.languages = Some(vec!["en".into()]);
        let loaded: Plan = serde_json::from_str(&serde_json::to_string(&own).unwrap_or_default()).expect("round trip");
        assert_eq!(loaded.languages, own.languages);
        assert_eq!(own.copy_named("Copy").languages, own.languages);
    }
}
