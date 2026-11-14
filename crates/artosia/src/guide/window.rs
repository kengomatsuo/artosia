//! The real window as the guide's `World`: controls from this frame's anchors with the last
//! layout's rectangles, and states read from `App`.

use super::{Anchor, Place, World, anchors};
use crate::palette::Command;
use crate::state::{App, Sheet, Tab};

/// What a lesson step asks the person to do: run a command, or go to a tab of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Run(Command),
    Tab(Tab),
    /// Press the control the window keys this way (a window's own button such as `cal-new`).
    Press(&'static str),
}

pub struct Window<'a> {
    pub app: &'a App,
    pub layout: &'a dyn Fn(&str) -> Option<[f32; 4]>,
}

impl World<Action> for Window<'_> {
    fn anchors(&self, action: Action) -> Vec<Anchor> {
        match action {
            Action::Run(command) => anchors::for_command(command, self.layout),
            // A tab's pill is keyed by the tab bar itself, and the bar is always on screen.
            Action::Tab(tab) => {
                let key = crate::ui::tabs::key(tab);
                let visible = (self.layout)(key).is_some_and(|[_, _, w, h]| w > 0.0 && h > 0.0);
                vec![Anchor { key: key.to_string(), place: Place::Bar, visible, enabled: true, presses: 1 }]
            }
            Action::Press(key) => {
                let visible = (self.layout)(key).is_some_and(|[_, _, w, h]| w > 0.0 && h > 0.0);
                vec![Anchor { key: key.to_string(), place: Place::Panel, visible, enabled: true, presses: 1 }]
            }
        }
    }

    fn holds(&self, state: &str) -> bool {
        holds(self.app, state)
    }

    /// Buttons call their own code rather than a command, so a step finishes on a state, never
    /// on "the command ran"; every lesson step names its `done` state.
    fn ran(&self, _action: Action) -> bool {
        false
    }

    fn shown(&self, key: &str) -> bool {
        (self.layout)(key).is_some_and(|[_, _, w, h]| w > 0.0 && h > 0.0)
    }
}

/// Every state name `holds` knows; the lesson test checks each step against it.
pub const KNOWN: [&str; 27] = [
    "screens-open", "phone-open", "settings-open", "no-dialog", "plan-tab", "calendar-tab", "people-tab", "giving-tab",
    "notices-tab", "templates-tab", "look-tab", "search-open", "event-form-open", "gift-form-open", "person-form-open",
    "notice-draft", "add-menu-open", "listen-open", "no-projector", "no-monitor-list",
    "stream-open", "no-stream", "web-open", "pptx-open", "new-notice",
    "editor-open", "bible-open",
];

/// The states lessons name. A name nobody knows is never true, and a test lists the names every
/// lesson uses against this match.
pub fn holds(app: &App, state: &str) -> bool {
    match state {
        "screens-open" => app.sheet == Sheet::Screens,
        "phone-open" => app.sheet == Sheet::Remote,
        "settings-open" => app.sheet == Sheet::Settings,
        "no-dialog" => app.sheet == Sheet::None,
        "plan-tab" => app.tab == Tab::Plan,
        "calendar-tab" => app.tab == Tab::Calendar,
        "people-tab" => app.tab == Tab::People,
        "giving-tab" => app.tab == Tab::Giving,
        "notices-tab" => app.tab == Tab::Notices,
        "templates-tab" => app.tab == Tab::Templates,
        "look-tab" => app.tab == Tab::Look,
        "search-open" => app.palette.open,
        "event-form-open" => app.calendar.form.is_some(),
        "gift-form-open" => app.giving.form.is_some(),
        "person-form-open" => app.members.form.is_some(),
        "notice-draft" => app.notice_draft().is_some(),
        "add-menu-open" => app.add_menu,
        "listen-open" => app.sheet == Sheet::Listen,
        // Practice and a window started before the projector have no switch to explain.
        "no-projector" => !(app.link.is_started() || app.demo_screens),
        "no-monitor-list" => !app.lists_monitors(),
        "stream-open" => app.sheet == Sheet::Stream,
        // A copy without the streaming program says so in the dialog and has nothing to explain.
        "no-stream" => !crate::stream::available(),
        "web-open" => app.web.is_some(),
        "pptx-open" => app.pptx.is_some(),
        "editor-open" => app.editor.is_some(),
        "bible-open" => app.palette.open && app.palette.scope == crate::palette::Scope::Bible,
        // An announcement opened from the list is not a new one.
        "new-notice" => app.notices.as_ref().is_some_and(|form| !app.announcements.iter().any(|card| card.id == form.id)),
        _ => false,
    }
}
