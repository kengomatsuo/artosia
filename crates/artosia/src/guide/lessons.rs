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

const SCREENS_STEPS: [Step<Action>; 1] = [go("screens-open", Action::Run(Command::Screens), "screens-open")];
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

/// Every lesson, for the lesson list and the test that runs each one.
pub const ALL: [Lesson<Action>; 5] = [SCREENS, CALENDAR, PEOPLE, GIVING, TEMPLATES];

/// A feature with a help mark: its name, what it is for, and the lesson that shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Calendar,
    People,
    Giving,
    Templates,
}

impl Feature {
    pub fn key(self) -> &'static str {
        match self {
            Feature::Calendar => "help-calendar",
            Feature::People => "help-people",
            Feature::Giving => "help-giving",
            Feature::Templates => "help-templates",
        }
    }

    pub fn tab(self) -> Tab {
        match self {
            Feature::Calendar => Tab::Calendar,
            Feature::People => Tab::People,
            Feature::Giving => Tab::Giving,
            Feature::Templates => Tab::Templates,
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
        }
    }

    pub fn lesson(self) -> Lesson<Action> {
        match self {
            Feature::Calendar => CALENDAR,
            Feature::People => PEOPLE,
            Feature::Giving => GIVING,
            Feature::Templates => TEMPLATES,
        }
    }
}

/// The headline and the line under it for a step.
pub fn words(step: &str) -> (String, &'static str) {
    let tab = |feature: Feature| (t::guide_open_tab(crate::ui::tabs::name(feature.tab())), feature.about());
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
