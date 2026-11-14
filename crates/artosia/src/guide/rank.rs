//! Which control a step rings when several fire its command: the one a person would think of
//! first. No shipped tour ranks by goal (research README, property 1); the order copies
//! Appcues' ranked selectors and what the Artosia tour learnt by hand (ring what is on screen).

/// Where a control sits, in the order a person looks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// Inside the panel or dialog the person is working in.
    Panel,
    /// The window's own bars: the top bar, the tabs, the Live pane's switches.
    Bar,
    /// A row's menu or a drop-down list: one press to open, one to choose.
    Menu,
    /// The search: always there, never the first thing to point at.
    Search,
}

/// One control that fires a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    /// The element's key, so the window can ask its rectangle.
    pub key: String,
    pub place: Place,
    /// Drawn on screen now with a rectangle of some size.
    pub visible: bool,
    /// Can be pressed now.
    pub enabled: bool,
    /// Presses from where the person is to the control, the control's own press included.
    pub presses: u8,
}

/// The control to ring: visible and enabled first, then by place, then by fewest presses; ties
/// keep the order the window listed them in. `None` when no control can be pressed at all.
pub fn best_anchor(mut anchors: Vec<Anchor>) -> Option<Anchor> {
    anchors.retain(|anchor| anchor.enabled);
    anchors.sort_by_key(|anchor| (!anchor.visible, anchor.place, anchor.presses));
    anchors.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(key: &str, place: Place, visible: bool, enabled: bool, presses: u8) -> Anchor {
        Anchor { key: key.to_string(), place, visible, enabled, presses }
    }

    #[test]
    fn on_screen_beats_a_better_place_that_is_hidden() {
        let picked = best_anchor(vec![a("panel-hidden", Place::Panel, false, true, 1), a("bar", Place::Bar, true, true, 1)]);
        assert_eq!(picked.map(|x| x.key), Some("bar".to_string()));
    }

    #[test]
    fn the_panel_beats_the_bar_and_a_menu_row_beats_the_search() {
        let picked = best_anchor(vec![a("bar", Place::Bar, true, true, 1), a("panel", Place::Panel, true, true, 1)]);
        assert_eq!(picked.map(|x| x.key), Some("panel".to_string()));
        let picked = best_anchor(vec![a("search", Place::Search, true, true, 2), a("menu", Place::Menu, true, true, 2)]);
        assert_eq!(picked.map(|x| x.key), Some("menu".to_string()));
    }

    #[test]
    fn fewer_presses_win_a_tie_and_a_disabled_control_is_never_picked() {
        let picked = best_anchor(vec![a("far", Place::Menu, true, true, 3), a("near", Place::Menu, true, true, 2)]);
        assert_eq!(picked.map(|x| x.key), Some("near".to_string()));
        assert_eq!(best_anchor(vec![a("off", Place::Bar, true, false, 1)]), None);
    }
}
