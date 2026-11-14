//! The guide's engine: lessons whose steps are goals, after JetBrains' lesson engine (prepare,
//! wait for the person's own action, check the state, step back when they wander off, and a
//! scripted test per lesson) and Driver's and Shepherd's element checks. Plan:
//! `docs/plans/artosia/guide.md`; research: `~/Shared/research/artosia-guide-framework/`.
//!
//! The engine has no window. A `World` answers what is on screen and what is true; the window
//! implements it from the layout's rectangles and `App`, a test implements it from a table.

pub mod anchors;
pub mod rank;
pub mod window;

pub use rank::{Anchor, Place, best_anchor};

/// What a step asks of the person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal<C> {
    /// Run this command, by whichever control the person would think of first.
    Run(C),
    /// Reach a state no single command names (a song open, a slide live); `World::holds` decides.
    Reach(&'static str),
}

/// One step of a lesson.
#[derive(Clone, Copy, Debug)]
pub struct Step<C: 'static> {
    /// Names the step for `restore` and for its recorded frames.
    pub id: &'static str,
    pub goal: Goal<C>,
    /// A state the step needs before it can be done (a dialog open, a song in the plan); `None`
    /// when an anchor for the goal is all it needs.
    pub needs: Option<&'static str>,
    /// The state that means the step is done; `None` means the goal's own command ran.
    pub done: Option<&'static str>,
    /// The step to go back to when `needs` stops holding mid-lesson (the person closed the dialog).
    pub restore: Option<&'static str>,
}

/// A lesson: a feature's steps, in order.
#[derive(Clone, Copy, Debug)]
pub struct Lesson<C: 'static> {
    pub id: &'static str,
    pub steps: &'static [Step<C>],
}

/// What the window can tell the engine.
pub trait World<C> {
    /// Every control that fires `command` now, with where it sits and whether it can be pressed.
    fn anchors(&self, command: C) -> Vec<Anchor>;
    /// Whether a named state holds (`"calendar-open"`, `"song-live"`).
    fn holds(&self, state: &str) -> bool;
    /// Whether `command` ran since the step was shown.
    fn ran(&self, command: C) -> bool;
}

/// What the window draws for the current step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Show {
    /// Ring this control and put the step's bubble beside it.
    At { step: usize, key: String },
    /// No control is on screen for the goal: a centred bubble that offers the search.
    Search { step: usize },
    /// The lesson is finished.
    Finished,
}

/// Plays one lesson against a world, a step at a time.
#[derive(Clone, Debug)]
pub struct Runner<C: 'static> {
    lesson: Lesson<C>,
    at: usize,
}

impl<C: Copy + PartialEq + 'static> Runner<C> {
    pub fn new(lesson: Lesson<C>) -> Self {
        Runner { lesson, at: 0 }
    }

    pub fn step(&self) -> usize {
        self.at
    }

    /// Looks at the world and moves the lesson: past every step already done, back to a step's
    /// restore point when what it needs is gone, and on to what to draw now. Called after every
    /// change the person makes, never on a timer.
    pub fn advance(&mut self, world: &impl World<C>) -> Show {
        loop {
            let Some(step) = self.lesson.steps.get(self.at).copied() else { return Show::Finished };
            if is_done(&step, world) {
                self.at += 1;
                continue;
            }
            if let Some(need) = step.needs.filter(|need| !world.holds(need)) {
                // What this step stands on is gone: go back to the step that brings it, if any.
                if let Some(back) = step.restore.and_then(|id| self.index_of(id)).filter(|back| *back < self.at) {
                    self.at = back;
                    continue;
                }
                let _ = need;
                return Show::Search { step: self.at };
            }
            return match step.goal {
                Goal::Run(command) => match best_anchor(world.anchors(command)) {
                    Some(anchor) => Show::At { step: self.at, key: anchor.key },
                    None => Show::Search { step: self.at },
                },
                Goal::Reach(_) => Show::Search { step: self.at },
            };
        }
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.lesson.steps.iter().position(|step| step.id == id)
    }
}

fn is_done<C: Copy>(step: &Step<C>, world: &impl World<C>) -> bool {
    match (step.done, step.goal) {
        (Some(state), _) => world.holds(state),
        (None, Goal::Run(command)) => world.ran(command),
        (None, Goal::Reach(state)) => world.holds(state),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum Cmd {
        OpenCalendar,
        AddEvent,
    }

    #[derive(Default)]
    struct Table {
        anchors: Vec<(Cmd, Anchor)>,
        states: RefCell<HashSet<&'static str>>,
        ran: RefCell<HashSet<Cmd>>,
    }

    impl World<Cmd> for Table {
        fn anchors(&self, command: Cmd) -> Vec<Anchor> {
            self.anchors.iter().filter(|(c, _)| *c == command).map(|(_, a)| a.clone()).collect()
        }
        fn holds(&self, state: &str) -> bool {
            self.states.borrow().contains(state)
        }
        fn ran(&self, command: Cmd) -> bool {
            self.ran.borrow().contains(&command)
        }
    }

    const STEPS: &[Step<Cmd>] = &[
        Step { id: "open", goal: Goal::Run(Cmd::OpenCalendar), needs: None, done: Some("calendar-open"), restore: None },
        Step { id: "add", goal: Goal::Run(Cmd::AddEvent), needs: Some("calendar-open"), done: Some("event-added"), restore: Some("open") },
    ];
    const LESSON: Lesson<Cmd> = Lesson { id: "calendar", steps: STEPS };

    fn anchor(key: &str, place: Place, visible: bool) -> Anchor {
        Anchor { key: key.to_string(), place, visible, enabled: true, presses: 1 }
    }

    #[test]
    fn a_step_rings_the_control_a_person_would_think_of_first() {
        let world = Table {
            anchors: vec![
                (Cmd::OpenCalendar, anchor("menu-calendar", Place::Menu, false)),
                (Cmd::OpenCalendar, anchor("tab-calendar", Place::Bar, true)),
            ],
            ..Table::default()
        };
        assert_eq!(Runner::new(LESSON).advance(&world), Show::At { step: 0, key: "tab-calendar".to_string() });
    }

    #[test]
    fn with_no_control_on_screen_the_step_offers_the_search() {
        let world = Table::default();
        assert_eq!(Runner::new(LESSON).advance(&world), Show::Search { step: 0 });
    }

    #[test]
    fn a_done_step_is_passed_and_a_lost_need_steps_back() {
        let world = Table {
            anchors: vec![
                (Cmd::OpenCalendar, anchor("tab-calendar", Place::Bar, true)),
                (Cmd::AddEvent, anchor("calendar-add", Place::Panel, true)),
            ],
            ..Table::default()
        };
        let mut runner = Runner::new(LESSON);
        assert_eq!(runner.advance(&world), Show::At { step: 0, key: "tab-calendar".to_string() });
        // The person opens the calendar: the lesson moves to its second step by itself.
        world.states.borrow_mut().insert("calendar-open");
        assert_eq!(runner.advance(&world), Show::At { step: 1, key: "calendar-add".to_string() });
        // They close it again: the step it needs is gone, so the lesson goes back to opening it.
        world.states.borrow_mut().remove("calendar-open");
        assert_eq!(runner.advance(&world), Show::At { step: 0, key: "tab-calendar".to_string() });
        world.states.borrow_mut().extend(["calendar-open", "event-added"]);
        assert_eq!(runner.advance(&world), Show::Finished);
    }

    #[test]
    fn a_lesson_started_part_way_skips_what_is_already_true() {
        let world = Table { anchors: vec![(Cmd::AddEvent, anchor("calendar-add", Place::Panel, true))], ..Table::default() };
        world.states.borrow_mut().insert("calendar-open");
        assert_eq!(Runner::new(LESSON).advance(&world), Show::At { step: 1, key: "calendar-add".to_string() });
    }
}
