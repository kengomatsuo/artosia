//! The screens of one setup run, as a pure machine: which screen a press or an arrival leads to,
//! and what the window must do about it. No window, no Windows, no network; the window draws a
//! `Model` and turns presses and arrivals into `Event`s.

use crate::install::{Kind, Step};
use crate::strings::Key;

/// Seconds the done screen stays before the window closes by itself.
pub const CLOSE_AFTER: f64 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Install,
    Uninstall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Start,
    Installing,
    Failed,
    Done,
    Remove,
    CloseFirst,
    Removing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Install,
    Remove { busy: bool, now: f64 },
    TryAgain,
    Cancel,
    Close,
    Escape,
    Progress(Step),
    Done { now: f64 },
    Failed(Kind),
    Busy(bool),
    Tick { now: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    StartWork,
    StartApp,
    CheckBusy,
    Exit(i32),
}

/// The exit codes the flows use: ended well or by choice, failed, broken, and in use.
const EXIT_CODES: [i32; 4] = [0, 1, 2, 5];
const FAILED: i32 = 1;
/// Seconds between two looks at whether Artosia has been closed.
const CHECK_EVERY: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub mode: Mode,
    pub screen: Screen,
    pub step: Step,
    pub failure: Kind,
    close_at: f64,
    next_check: f64,
}

impl Model {
    /// An install run opens on the start screen, an uninstall run on the question.
    pub fn new(mode: Mode) -> Model {
        let screen = if mode == Mode::Install { Screen::Start } else { Screen::Remove };
        Model { mode, screen, step: Step::Checking, failure: Kind::Other, close_at: f64::INFINITY, next_check: 0.0 }
    }

    /// Takes one press or arrival; returns what the window must do about it, in order.
    pub fn on(&mut self, event: Event) -> Vec<Command> {
        match (self.screen, event) {
            (Screen::Start, Event::Install) => self.begin(Screen::Installing, Step::Checking),
            (Screen::Installing, Event::Progress(step)) => {
                self.step = step;
                Vec::new()
            }
            (Screen::Installing, Event::Done { now }) => {
                self.screen = Screen::Done;
                self.close_at = now + CLOSE_AFTER;
                vec![Command::StartApp]
            }
            (Screen::Removing, Event::Done { .. }) => vec![Command::Exit(0)],
            (Screen::Installing | Screen::Removing, Event::Failed(kind)) => {
                self.screen = Screen::Failed;
                self.failure = kind;
                Vec::new()
            }
            (Screen::Failed, Event::TryAgain) if self.mode == Mode::Install => self.begin(Screen::Installing, Step::Checking),
            (Screen::Failed, Event::TryAgain) => self.begin(Screen::Removing, Step::Removing),
            (Screen::Remove, Event::Remove { busy: false, .. }) => self.begin(Screen::Removing, Step::Removing),
            (Screen::Remove, Event::Remove { busy: true, now }) => {
                self.screen = Screen::CloseFirst;
                self.next_check = now + CHECK_EVERY;
                Vec::new()
            }
            (Screen::CloseFirst, Event::Tick { now }) if now >= self.next_check => {
                self.next_check = now + CHECK_EVERY;
                vec![Command::CheckBusy]
            }
            (Screen::CloseFirst, Event::Busy(false)) => self.begin(Screen::Removing, Step::Removing),
            (Screen::Done, Event::Tick { now }) if now >= self.close_at => vec![Command::Exit(0)],
            (_, Event::Cancel | Event::Close | Event::Escape) => self.leave(),
            _ => Vec::new(),
        }
    }

    fn begin(&mut self, screen: Screen, step: Step) -> Vec<Command> {
        self.screen = screen;
        self.step = step;
        vec![Command::StartWork]
    }

    /// Cancel, Escape and the window's cross all mean the same thing, and what that is depends on the screen.
    fn leave(&self) -> Vec<Command> {
        match self.screen {
            Screen::Installing if !self.can_cancel() => Vec::new(),
            Screen::Removing => Vec::new(),
            Screen::Failed => vec![Command::Exit(FAILED)],
            _ => vec![Command::Exit(0)],
        }
    }

    /// Whether leaving does anything now. Once the files are being swapped in, stopping would
    /// only be undone; while removing, it was never on offer.
    pub fn can_cancel(&self) -> bool {
        match self.screen {
            Screen::Installing => self.step != Step::SettingUp,
            Screen::Removing => false,
            _ => true,
        }
    }

    /// The question before removing opens with the keyboard on Cancel; every other screen opens
    /// on the filled button.
    pub fn focus_starts_on_cancel(&self) -> bool {
        self.screen == Screen::Remove
    }

    /// The heading and the sentence of a failure: the install's by what went wrong, the removal's own.
    pub fn failure_words(&self) -> (Key, Key) {
        match self.mode {
            Mode::Install => (Key::FailedHeading, self.failure.message()),
            Mode::Uninstall => (Key::RemoveFailedHeading, Key::RemoveFailedBody),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The window ran and the run ended with this exit code.
    Finished(i32),
    /// The window never worked (its own error exit, a crash, an abort): run the task dialogs.
    Dialogs,
}

/// What to do about the window process's end. Only the codes the flows themselves use are an
/// answer; anything else, or no code, is the window failing to run.
pub fn window_outcome(code: Option<i32>) -> Outcome {
    match code {
        Some(code) if EXIT_CODES.contains(&code) => Outcome::Finished(code),
        _ => Outcome::Dialogs,
    }
}

/// The front of a run: the window if it works, else the task dialogs. `window` runs the window
/// and says how it ended; `dialogs` runs the dialog flow and gives its exit code.
pub fn run_front(window: impl FnOnce() -> Option<i32>, dialogs: impl FnOnce() -> i32) -> i32 {
    match window_outcome(window()) {
        Outcome::Finished(code) => code,
        Outcome::Dialogs => dialogs(),
    }
}
