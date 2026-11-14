//! The lessons, one per feature, and the words of each step. A step's words live here and not in
//! the engine, so the engine stays free of the window's strings.

use super::window::Action;
use super::{Goal, Lesson, Step};
use crate::palette::Command;
use crate::state::Tab;
use crate::strings as t;

const fn go(id: &'static str, action: Action, done: &'static str) -> Step<Action> {
    Step { id, goal: Goal::Run(action), needs: None, done: Some(done), restore: None }
}

/// A step that needs the tab of the step before it open, and goes back there when it is left.
const fn on_tab(id: &'static str, action: Action, needs: &'static str, done: &'static str, back: &'static str) -> Step<Action> {
    Step { id, goal: Goal::Run(action), needs: Some(needs), done: Some(done), restore: Some(back) }
}

/// An explanation of a part of the window, read and passed with Next. It stands on the tab or
/// dialog the step before it opened, and goes back to that step when it is closed.
const fn point(id: &'static str, key: &'static str, needs: &'static str, back: &'static str) -> Step<Action> {
    Step { id, goal: Goal::Point(key), needs: Some(needs), done: None, restore: Some(back) }
}

const SCREENS_STEPS: [Step<Action>; 3] = [
    go("screens-open", Action::Run(Command::Screens), "screens-open"),
    Step { done: Some("no-projector"), ..point("screens-switch", "projector-switch", "screens-open", "screens-open") },
    Step { done: Some("no-monitor-list"), ..point("screens-jobs", "screen-jobs", "screens-open", "screens-open") },
];
const PHONE_STEPS: [Step<Action>; 2] = [go("phone-open", Action::Run(Command::Remote), "phone-open"), point("phone-switch", "remote-switch", "phone-open", "phone-open")];
const LISTEN_STEPS: [Step<Action>; 2] = [go("listen-open", Action::Run(Command::Listen), "listen-open"), point("listen-mics", "microphones", "listen-open", "listen-open")];
const STREAM_STEPS: [Step<Action>; 3] = [
    go("stream-open", Action::Run(Command::Stream), "stream-open"),
    Step { done: Some("no-stream"), ..point("stream-fields", "stream-fields", "stream-open", "stream-open") },
    Step { done: Some("no-stream"), ..point("stream-start", "stream-start", "stream-open", "stream-open") },
];
const WEB_STEPS: [Step<Action>; 4] = [
    go("web-open", Action::Run(Command::WebSongs), "web-open"),
    point("web-find", "web-field", "web-open", "web-open"),
    point("web-browser", "web-browser", "web-open", "web-open"),
    point("web-import", "web-go", "web-open", "web-open"),
];
const PPTX_STEPS: [Step<Action>; 3] = [
    go("pptx-open", Action::Run(Command::ImportPptx), "pptx-open"),
    point("pptx-way", "pptx-ways", "pptx-open", "pptx-open"),
    point("pptx-import", "pptx-go", "pptx-open", "pptx-open"),
];
const NOTICES_STEPS: [Step<Action>; 2] = [
    go("notices-tab", Action::Tab(Tab::Notices), "notices-tab"),
    on_tab("notices-new", Action::Press("notice-add"), "notices-tab", "notice-draft", "notices-tab"),
];
const LOOK_STEPS: [Step<Action>; 2] = [go("look-tab", Action::Tab(Tab::Look), "look-tab"), point("look-for", "look-for", "look-tab", "look-tab")];
const PLAN_STEPS: [Step<Action>; 3] = [
    go("plan-tab", Action::Tab(Tab::Plan), "plan-tab"),
    point("plan-explain", "plan-add", "plan-tab", "plan-tab"),
    on_tab("plan-add", Action::Press("plan-add-button"), "plan-tab", "add-menu-open", "plan-tab"),
];
const CALENDAR_STEPS: [Step<Action>; 2] = [
    go("calendar-tab", Action::Tab(Tab::Calendar), "calendar-tab"),
    on_tab("calendar-new", Action::Press("cal-new"), "calendar-tab", "event-form-open", "calendar-tab"),
];
const PEOPLE_STEPS: [Step<Action>; 2] = [
    go("people-tab", Action::Tab(Tab::People), "people-tab"),
    on_tab("people-add", Action::Press("people-add"), "people-tab", "person-form-open", "people-tab"),
];
const GIVING_STEPS: [Step<Action>; 2] = [
    go("giving-tab", Action::Tab(Tab::Giving), "giving-tab"),
    on_tab("giving-add", Action::Press("giving-add"), "giving-tab", "gift-form-open", "giving-tab"),
];
const TEMPLATES_STEPS: [Step<Action>; 2] = [
    go("templates-tab", Action::Tab(Tab::Templates), "templates-tab"),
    on_tab("templates-use", Action::Press("tpl-use"), "templates-tab", "notices-tab", "templates-tab"),
];

pub const SCREENS: Lesson<Action> = Lesson { id: "screens", steps: &SCREENS_STEPS };
pub const CALENDAR: Lesson<Action> = Lesson { id: "calendar", steps: &CALENDAR_STEPS };
pub const PEOPLE: Lesson<Action> = Lesson { id: "people", steps: &PEOPLE_STEPS };
pub const GIVING: Lesson<Action> = Lesson { id: "giving", steps: &GIVING_STEPS };
pub const TEMPLATES: Lesson<Action> = Lesson { id: "templates", steps: &TEMPLATES_STEPS };
pub const NOTICES: Lesson<Action> = Lesson { id: "notices", steps: &NOTICES_STEPS };
pub const LOOK: Lesson<Action> = Lesson { id: "look", steps: &LOOK_STEPS };
pub const PLAN: Lesson<Action> = Lesson { id: "plan", steps: &PLAN_STEPS };
pub const PHONE: Lesson<Action> = Lesson { id: "phone", steps: &PHONE_STEPS };
pub const LISTEN: Lesson<Action> = Lesson { id: "listen", steps: &LISTEN_STEPS };
pub const STREAM: Lesson<Action> = Lesson { id: "stream", steps: &STREAM_STEPS };
pub const WEB: Lesson<Action> = Lesson { id: "web", steps: &WEB_STEPS };
pub const PPTX: Lesson<Action> = Lesson { id: "pptx", steps: &PPTX_STEPS };

/// Every lesson, for the lesson list and the test that runs each one.
pub const ALL: [Lesson<Action>; 13] = [SCREENS, CALENDAR, PEOPLE, GIVING, TEMPLATES, NOTICES, LOOK, PLAN, PHONE, LISTEN, STREAM, WEB, PPTX];

/// A feature with a help mark: its name, what it is for, and the lesson that shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Calendar,
    People,
    Giving,
    Templates,
    Notices,
    Look,
    Plan,
    Screens,
    Phone,
    Listen,
    Stream,
    WebSongs,
    Pptx,
}

impl Feature {
    pub fn key(self) -> &'static str {
        match self {
            Feature::Calendar => "help-calendar",
            Feature::People => "help-people",
            Feature::Giving => "help-giving",
            Feature::Templates => "help-templates",
            Feature::Notices => "help-notices",
            Feature::Look => "help-look",
            Feature::Plan => "help-plan",
            Feature::Screens => "help-screens",
            Feature::Phone => "help-phone",
            Feature::Listen => "help-listen",
            Feature::Stream => "help-stream",
            Feature::WebSongs => "help-web",
            Feature::Pptx => "help-pptx",
        }
    }

    /// The feature's name, as its tab or its dialog is named.
    pub fn name(self) -> &'static str {
        match self {
            Feature::Screens => t::BTN_SCREENS(),
            Feature::Phone => t::BTN_PHONE(),
            Feature::Listen => t::BTN_LISTEN(),
            Feature::Stream => t::BTN_STREAM(),
            Feature::WebSongs => t::CMD_WEB_SONGS(),
            Feature::Pptx => t::CMD_IMPORT_PPTX(),
            tabbed => crate::ui::tabs::name(tabbed.tab()),
        }
    }

    /// The tab a feature lives in; a dialog's feature lives over the rundown.
    pub fn tab(self) -> Tab {
        match self {
            Feature::Calendar => Tab::Calendar,
            Feature::People => Tab::People,
            Feature::Giving => Tab::Giving,
            Feature::Templates => Tab::Templates,
            Feature::Notices => Tab::Notices,
            Feature::Look => Tab::Look,
            // The PowerPoint import adds announcements, and its button is on their tab.
            Feature::Pptx => Tab::Notices,
            Feature::Plan | Feature::Screens | Feature::Phone | Feature::Listen | Feature::Stream | Feature::WebSongs => Tab::Plan,
        }
    }

    /// One or two sentences on what the feature is for, shown by its help mark and under the
    /// lesson's first step.
    pub fn about(self) -> &'static str {
        match self {
            Feature::Calendar => t::HELP_CALENDAR(),
            Feature::People => t::HELP_PEOPLE(),
            Feature::Giving => t::HELP_GIVING(),
            Feature::Templates => t::HELP_TEMPLATES(),
            Feature::Notices => t::HELP_NOTICES(),
            Feature::Look => t::HELP_LOOK(),
            Feature::Plan => t::HELP_PLAN(),
            Feature::Screens => t::HELP_SCREENS(),
            Feature::Phone => t::HELP_PHONE(),
            Feature::Listen => t::HELP_LISTEN(),
            Feature::Stream => t::HELP_STREAM(),
            Feature::WebSongs => t::HELP_WEB(),
            Feature::Pptx => t::HELP_PPTX(),
        }
    }

    /// The clip that shows the feature best: its window's own button being pressed.
    pub fn clip_step(self) -> &'static str {
        match self {
            Feature::Calendar => "calendar-new",
            Feature::People => "people-add",
            Feature::Giving => "giving-add",
            Feature::Templates => "templates-use",
            Feature::Notices => "notices-new",
            Feature::Look => "look-tab",
            Feature::Plan => "plan-add",
            Feature::Screens => "screens-open",
            Feature::Phone => "phone-open",
            Feature::Listen => "listen-open",
            Feature::Stream => "stream-open",
            Feature::WebSongs => "web-open",
            Feature::Pptx => "pptx-open",
        }
    }

    /// The feature's lesson, which Show me starts.
    pub fn lesson(self) -> Lesson<Action> {
        match self {
            Feature::Calendar => CALENDAR,
            Feature::People => PEOPLE,
            Feature::Giving => GIVING,
            Feature::Templates => TEMPLATES,
            Feature::Notices => NOTICES,
            Feature::Look => LOOK,
            Feature::Plan => PLAN,
            Feature::Screens => SCREENS,
            Feature::Phone => PHONE,
            Feature::Listen => LISTEN,
            Feature::Stream => STREAM,
            Feature::WebSongs => WEB,
            Feature::Pptx => PPTX,
        }
    }
}

/// The recorded clip of a step: the real window, the pointer moving to the control and pressing
/// it, made by `tools/record-guide.py` as an animated PNG that the window plays by itself.
pub fn clip(step: &str) -> Option<&'static [u8]> {
    Some(match step {
        "screens-open" => include_bytes!("../../../../assets/guide/screens-open.png"),
        "calendar-tab" => include_bytes!("../../../../assets/guide/calendar-tab.png"),
        "calendar-new" => include_bytes!("../../../../assets/guide/calendar-new.png"),
        "people-tab" => include_bytes!("../../../../assets/guide/people-tab.png"),
        "people-add" => include_bytes!("../../../../assets/guide/people-add.png"),
        "giving-tab" => include_bytes!("../../../../assets/guide/giving-tab.png"),
        "giving-add" => include_bytes!("../../../../assets/guide/giving-add.png"),
        "templates-tab" => include_bytes!("../../../../assets/guide/templates-tab.png"),
        "templates-use" => include_bytes!("../../../../assets/guide/templates-use.png"),
        "notices-tab" => include_bytes!("../../../../assets/guide/notices-tab.png"),
        "notices-new" => include_bytes!("../../../../assets/guide/notices-new.png"),
        "look-tab" => include_bytes!("../../../../assets/guide/look-tab.png"),
        "plan-tab" => include_bytes!("../../../../assets/guide/plan-tab.png"),
        "plan-add" => include_bytes!("../../../../assets/guide/plan-add.png"),
        "phone-open" => include_bytes!("../../../../assets/guide/phone-open.png"),
        "listen-open" => include_bytes!("../../../../assets/guide/listen-open.png"),
        "stream-open" => include_bytes!("../../../../assets/guide/stream-open.png"),
        "web-open" => include_bytes!("../../../../assets/guide/web-open.png"),
        "pptx-open" => include_bytes!("../../../../assets/guide/pptx-open.png"),
        _ => return None,
    })
}

/// The headline and the line under it for a step.
pub fn words(step: &str) -> (String, &'static str) {
    let tab = |feature: Feature| (t::guide_open_tab(feature.name()), feature.about());
    match step {
        "screens-open" => (t::GUIDE_SCREENS_OPEN().to_string(), t::GUIDE_SCREENS_OPEN_MORE()),
        "calendar-tab" => tab(Feature::Calendar),
        "calendar-new" => (t::GUIDE_CAL_NEW().to_string(), t::GUIDE_CAL_NEW_MORE()),
        "people-tab" => tab(Feature::People),
        "people-add" => (t::GUIDE_PEOPLE_ADD().to_string(), t::GUIDE_PEOPLE_ADD_MORE()),
        "giving-tab" => tab(Feature::Giving),
        "giving-add" => (t::GUIDE_GIVING_ADD().to_string(), t::GUIDE_GIVING_ADD_MORE()),
        "templates-tab" => tab(Feature::Templates),
        "templates-use" => (t::GUIDE_TPL_USE().to_string(), t::GUIDE_TPL_USE_MORE()),
        "notices-tab" => tab(Feature::Notices),
        "notices-new" => (t::GUIDE_NOTICE_NEW().to_string(), t::GUIDE_NOTICE_NEW_MORE()),
        "look-tab" => tab(Feature::Look),
        "look-for" => (t::GUIDE_LOOK_FOR().to_string(), t::GUIDE_LOOK_FOR_MORE()),
        "plan-tab" => tab(Feature::Plan),
        "plan-explain" => (t::GUIDE_PLAN_EXPLAIN().to_string(), t::GUIDE_PLAN_EXPLAIN_MORE()),
        "plan-add" => (t::GUIDE_PLAN_ADD().to_string(), t::GUIDE_PLAN_ADD_MORE()),
        "screens-switch" => (t::GUIDE_SCREENS_SWITCH().to_string(), t::GUIDE_SCREENS_SWITCH_MORE()),
        "screens-jobs" => (t::GUIDE_SCREENS_JOBS().to_string(), t::GUIDE_SCREENS_JOBS_MORE()),
        "phone-open" => (t::GUIDE_PHONE_OPEN().to_string(), t::GUIDE_PHONE_OPEN_MORE()),
        "phone-switch" => (t::GUIDE_PHONE_SWITCH().to_string(), t::GUIDE_PHONE_SWITCH_MORE()),
        "listen-open" => (t::GUIDE_LISTEN_OPEN().to_string(), t::GUIDE_LISTEN_OPEN_MORE()),
        "listen-mics" => (t::GUIDE_LISTEN_MICS().to_string(), t::GUIDE_LISTEN_MICS_MORE()),
        "stream-open" => (t::GUIDE_STREAM_OPEN().to_string(), t::GUIDE_STREAM_OPEN_MORE()),
        "stream-fields" => (t::GUIDE_STREAM_FIELDS().to_string(), t::GUIDE_STREAM_FIELDS_MORE()),
        "stream-start" => (t::GUIDE_STREAM_START().to_string(), t::GUIDE_STREAM_START_MORE()),
        "web-open" => (t::GUIDE_WEB_OPEN().to_string(), t::GUIDE_WEB_OPEN_MORE()),
        "web-find" => (t::GUIDE_WEB_FIND().to_string(), t::GUIDE_WEB_FIND_MORE()),
        "web-browser" => (t::GUIDE_WEB_BROWSER().to_string(), t::GUIDE_WEB_BROWSER_MORE()),
        "web-import" => (t::GUIDE_WEB_IMPORT().to_string(), t::GUIDE_WEB_IMPORT_MORE()),
        "pptx-open" => (t::GUIDE_PPTX_OPEN().to_string(), t::GUIDE_PPTX_OPEN_MORE()),
        "pptx-way" => (t::GUIDE_PPTX_WAY().to_string(), t::GUIDE_PPTX_WAY_MORE()),
        "pptx-import" => (t::GUIDE_PPTX_IMPORT().to_string(), t::GUIDE_PPTX_IMPORT_MORE()),
        _ => (String::new(), ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_has_words_and_every_state_it_names_is_known() {
        for lesson in ALL {
            for step in lesson.steps {
                assert!(!words(step.id).0.is_empty(), "{} has no words", step.id);
                for state in [step.needs, step.done].into_iter().flatten() {
                    assert!(crate::guide::window::KNOWN.contains(&state), "{state} is not a state the window knows");
                }
            }
        }
    }
}
