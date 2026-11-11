//! Where a song of the plan begins: a medley goes from one song straight into the chorus of
//! the next, so a volunteer marks that while preparing and the song starts there by itself.
//! The mark lives on the plan item (`PlanEntry::Song::start`, a section's tag); this is how
//! the window sets it, shows it and starts from it.

use artosia_core::deck::Deck;
use artosia_core::plan::{PlanEntry, start_slide};
use artosia_core::song::Tag;

use crate::state::App;

/// What the Start here button says and does for the slide the Preview ring is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartState {
    /// The ring is on a section the song does not start at: pressing makes it the start.
    Offer,
    /// The song starts at the ring's section: pressing clears it.
    Marked,
    /// The ring is on the beginning and nothing is marked: there is nothing to set.
    Beginning,
}

impl App {
    /// The section a plan entry begins at, while its song still has that section. A section cut
    /// from the song leaves the plan item beginning at the top.
    pub fn entry_start(&self, entry: &PlanEntry) -> Option<Tag> {
        let tag = entry.start()?;
        let PlanEntry::Song { song_id, .. } = entry else { return None };
        let song = &self.songs[self.song_position(*song_id)?];
        song.sections().iter().any(|section| section.tag == tag).then_some(tag)
    }

    /// The slide a plan item opens on in `deck`: its start, else the first slide.
    pub fn start_cursor(&self, item: u64, deck: &Deck) -> usize {
        let start = self.plan.items.iter().find(|candidate| candidate.id == item).and_then(|candidate| candidate.entry.start());
        start_slide(deck, start)
    }

    /// The slide of the Preview deck that its plan item starts at, when a start was set.
    pub fn marked_start(&self) -> Option<usize> {
        let open = self.open.as_ref()?;
        let item = self.plan.items.iter().find(|item| Some(item.id) == open.item)?;
        let tag = self.entry_start(&item.entry)?;
        open.deck.slides.iter().position(|slide| slide.tag == Some(tag))
    }

    /// The plan item in Preview, the section under the ring, and what the button should say.
    /// None when Preview holds nothing that has sections to start at.
    pub fn start_control(&self) -> Option<(u64, Tag, StartState)> {
        let open = self.open.as_ref()?;
        let id = open.item?;
        let item = self.plan.items.iter().find(|item| item.id == id)?;
        if !matches!(item.entry, PlanEntry::Song { .. }) {
            return None;
        }
        let tag = open.deck.slides.get(open.cursor)?.tag?;
        let start = self.entry_start(&item.entry);
        let first = open.deck.slides.first().and_then(|slide| slide.tag);
        let state = if start == Some(tag) {
            StartState::Marked
        } else if start.is_none() && first == Some(tag) {
            StartState::Beginning
        } else {
            StartState::Offer
        };
        Some((id, tag, state))
    }

    /// Sets or clears the section a plan item begins at. A song in Preview whose ring nobody
    /// moved follows it, and so does the song Artosia queued there.
    pub fn set_start(&mut self, id: u64, tag: Option<Tag>) {
        let Some(item) = self.plan.items.iter_mut().find(|item| item.id == id) else { return };
        let PlanEntry::Song { start, .. } = &mut item.entry else { return };
        if *start == tag {
            return;
        }
        *start = tag;
        self.touch_plan();
        let at = self.open.as_ref().filter(|open| open.item == Some(id) && !open.armed).map(|open| start_slide(&open.deck, tag));
        if let (Some(at), Some(open)) = (at, self.open.as_mut()) {
            open.cursor = at;
        }
        self.requeue();
    }

    /// The Start here button and its key: the ring's section becomes where the song begins, or
    /// stops being it.
    pub fn toggle_start_here(&mut self) {
        let Some((id, tag, state)) = self.start_control() else { return };
        match state {
            StartState::Beginning => {}
            StartState::Marked => self.set_start(id, None),
            StartState::Offer => {
                // The first section is where a song begins anyway, so marking it clears the mark.
                let first = self.open.as_ref().and_then(|open| open.deck.slides.first().and_then(|slide| slide.tag));
                self.set_start(id, if first == Some(tag) { None } else { Some(tag) });
                // A section sung twice is marked at its first slide; the ring goes there so the
                // list shows the mark where the ring is.
                if let Some(open) = self.open.as_mut().filter(|open| open.item == Some(id)) {
                    open.cursor = start_slide(&open.deck, Some(tag));
                }
            }
        }
    }

    /// Puts a plan item's start on the projector: the item is already in Preview.
    pub fn show_start_of(&mut self, id: u64) {
        let Some(open) = self.open.as_ref().filter(|open| open.item == Some(id)) else { return };
        let at = self.start_cursor(id, &open.deck);
        self.show(at);
    }

    /// What the search learned about where this song is usually begun, for a plan item with no
    /// start set: the section, how often, and out of how many. Offered, never applied.
    pub fn start_suggestion(&self, entry: &PlanEntry) -> Option<(Tag, u32, u32)> {
        let PlanEntry::Song { song_id, start: None, .. } = entry else { return None };
        self.learn.starts.get(&self.song_position(*song_id)?).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn song_item(app: &App, nth: usize) -> u64 {
        app.plan.items.iter().filter(|item| matches!(item.entry, PlanEntry::Song { .. })).nth(nth).map(|item| item.id).expect("the practice plan has songs")
    }

    /// The first slide of a song's chorus, which is not the first slide.
    fn chorus_of(app: &App, id: u64) -> (Tag, usize) {
        let deck = app.deck_for(&app.plan.items.iter().find(|item| item.id == id).expect("item").entry).expect("deck");
        let at = deck.slides.iter().position(|slide| slide.tag.is_some_and(|tag| tag.kind == artosia_core::song::SectionKind::Chorus)).expect("a chorus");
        assert!(at > 0);
        (deck.slides[at].tag.expect("tag"), at)
    }

    #[test]
    fn the_queued_song_starts_at_its_mark_and_at_the_top_once_cleared() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        let (chorus, at) = chorus_of(&app, second);
        app.set_start(second, Some(chorus));
        app.open_plan_item(first);
        app.show(0);
        // What comes next waits in Preview with its ring on the chorus.
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor, open.queued)), Some((Some(second), at, true)));
        // Next at the end of the live song goes into the chorus.
        let slides = app.live.as_ref().map(|live| live.deck.slides.len()).unwrap_or(0);
        for _ in 0..slides {
            app.next();
        }
        assert_eq!(app.live.as_ref().map(|live| (live.item, live.index)), Some((Some(second), at)));
        // Cleared, it begins at the top again.
        app.open_plan_item(first);
        app.show(0);
        app.set_start(second, None);
        assert_eq!(app.open.as_ref().map(|open| open.cursor), Some(0), "the queued ring follows the cleared mark");
        for _ in 0..slides {
            app.next();
        }
        assert_eq!(app.live.as_ref().map(|live| (live.item, live.index)), Some((Some(second), 0)));
    }

    #[test]
    fn opening_walking_and_starting_the_service_begin_at_the_mark() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        let (chorus, at) = chorus_of(&app, first);
        app.set_start(first, Some(chorus));
        app.open_plan_item(first);
        assert_eq!(app.open.as_ref().map(|open| open.cursor), Some(at));
        app.open_plan_item(second);
        app.walk_plan(false);
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(first), at)));
        app.plan.items.retain(|item| !matches!(item.entry, PlanEntry::Announcements));
        app.start_service();
        assert_eq!(app.live.as_ref().map(|live| (live.item, live.index)), Some((Some(first), at)));
    }

    #[test]
    fn the_button_marks_the_ring_unmarks_it_and_is_idle_on_the_beginning() {
        let mut app = App::practice();
        let first = song_item(&app, 0);
        let (chorus, at) = chorus_of(&app, first);
        app.open_plan_item(first);
        assert_eq!(app.start_control().map(|(_, _, state)| state), Some(StartState::Beginning));
        app.toggle_start_here();
        assert_eq!(app.plan.items.iter().find(|item| item.id == first).and_then(|item| item.entry.start()), None);
        // The ring is moved by hand to the chorus: now it can be marked.
        app.select(at as isize);
        assert_eq!(app.start_control().map(|(_, tag, state)| (tag, state)), Some((chorus, StartState::Offer)));
        app.toggle_start_here();
        assert_eq!(app.marked_start(), Some(at));
        assert_eq!(app.start_control().map(|(_, _, state)| state), Some(StartState::Marked));
        assert!(app.plan_dirty);
        // Pressed again it clears, and a ring nobody moved goes back to the top.
        app.toggle_start_here();
        assert_eq!(app.marked_start(), None);
    }

    #[test]
    fn a_section_cut_from_the_song_leaves_the_item_at_the_top_and_unmarked() {
        let mut app = App::practice();
        let first = song_item(&app, 0);
        let (chorus, _) = chorus_of(&app, first);
        app.set_start(first, Some(chorus));
        let id = match app.plan.items.iter().find(|item| item.id == first).map(|item| item.entry.clone()) {
            Some(PlanEntry::Song { song_id, .. }) => song_id,
            _ => unreachable!(),
        };
        let at = app.song_position(id).expect("song");
        app.songs[at].lyrics = "[1]\nOnly a verse is left".to_string();
        assert_eq!(app.entry_start(&app.plan.items.iter().find(|item| item.id == first).expect("item").entry), None);
        app.open_plan_item(first);
        assert_eq!(app.open.as_ref().map(|open| open.cursor), Some(0));
    }
}
