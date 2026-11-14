//! The controls of this frame that fire a command, declared by the controls themselves while the
//! window is drawn (`Btn::anchor`), so no hand-kept table goes stale when a screen changes. The
//! guide reads them after the window's body is built, with the rectangles of the last layout.

use std::cell::RefCell;

use super::rank::{Anchor, Place};
use crate::palette::Command;

struct Marked {
    command: Command,
    key: String,
    place: Place,
    enabled: bool,
    presses: u8,
}

thread_local! {
    static FRAME: RefCell<Vec<Marked>> = const { RefCell::new(Vec::new()) };
}

/// Starts a frame's list; the window's view calls it first.
pub fn begin_frame() {
    FRAME.with(|frame| frame.borrow_mut().clear());
}

/// Records a control and returns the key it must carry, the same every frame for the same
/// command at the same place, so the last layout's rectangle belongs to it.
pub fn mark(command: Command, place: Place, enabled: bool, presses: u8) -> String {
    FRAME.with(|frame| {
        let mut frame = frame.borrow_mut();
        let nth = frame.iter().filter(|m| m.command == command && m.place == place).count();
        let key = format!("anchor-{command:?}-{place:?}-{nth}");
        frame.push(Marked { command, key: key.clone(), place, enabled, presses });
        key
    })
}

/// Every control of this frame for `command`, plus the search, which reaches every command in
/// two presses. A control counts as visible when the last layout gave it an area.
pub fn for_command(command: Command, layout: &dyn Fn(&str) -> Option<[f32; 4]>) -> Vec<Anchor> {
    let shown = |key: &str| layout(key).is_some_and(|[_, _, w, h]| w > 0.0 && h > 0.0);
    let mut found: Vec<Anchor> = FRAME.with(|frame| {
        frame
            .borrow()
            .iter()
            .filter(|m| m.command == command)
            .map(|m| Anchor { key: m.key.clone(), place: m.place, visible: shown(&m.key), enabled: m.enabled, presses: m.presses })
            .collect()
    });
    found.push(Anchor { key: "search-button".to_string(), place: Place::Search, visible: shown("search-button"), enabled: true, presses: 2 });
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marked_control_keeps_its_key_across_frames_and_the_search_is_always_offered() {
        begin_frame();
        let first = mark(Command::Screens, Place::Bar, true, 1);
        begin_frame();
        assert_eq!(mark(Command::Screens, Place::Bar, true, 1), first);
        let layout = |key: &str| (key == first.as_str()).then_some([0.0, 0.0, 80.0, 32.0]);
        let found = for_command(Command::Screens, &layout);
        assert_eq!(found.len(), 2);
        assert!(found[0].visible && found[0].key == first);
        assert_eq!(found[1].place, Place::Search);
        assert!(!found[1].visible);
    }
}
