//! The lessons, one per feature, and the words of each step. A step's words live here and not in
//! the engine, so the engine stays free of the window's strings.

use super::window::Action;
use super::{Goal, Lesson, Step};
use crate::palette::Command;
use crate::strings as t;

/// Opening the Screens dialog: the first lesson whose step the person does themselves.
const SCREENS_STEPS: [Step<Action>; 1] =
    [Step { id: "screens-open", goal: Goal::Run(Action::Run(Command::Screens)), needs: None, done: Some("screens-open"), restore: None }];
pub const SCREENS: Lesson<Action> = Lesson { id: "screens", steps: &SCREENS_STEPS };

/// Every lesson, for the lesson list and the test that runs each one.
pub const ALL: [Lesson<Action>; 1] = [SCREENS];

/// The headline and the line under it for a step.
pub fn words(step: &str) -> (&'static str, &'static str) {
    match step {
        "screens-open" => (t::GUIDE_SCREENS_OPEN(), t::GUIDE_SCREENS_OPEN_MORE()),
        _ => ("", ""),
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
