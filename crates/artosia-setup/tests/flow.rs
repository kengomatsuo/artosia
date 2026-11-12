//! The window's state, as pure logic: which screen follows which press or arrival, and what the
//! window must do about it. No window, no Windows, no network.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use artosia_setup::flow::{Command, Event, Mode, Model, Outcome, Screen, CLOSE_AFTER, run_front, window_outcome};
use std::cell::Cell;
use artosia_setup::install::{Kind, Step};
use artosia_setup::strings::Key;

fn installing() -> Model {
    let mut model = Model::new(Mode::Install);
    assert_eq!(model.on(Event::Install), vec![Command::StartWork]);
    model
}

#[test]
fn an_install_run_opens_on_start_and_nothing_starts_until_install_is_pressed() {
    let model = Model::new(Mode::Install);
    assert_eq!(model.screen, Screen::Start);
    assert_eq!(model.step, Step::Checking);
}

#[test]
fn pressing_install_shows_progress_at_once_and_starts_the_work_once() {
    let mut model = Model::new(Mode::Install);
    assert_eq!(model.on(Event::Install), vec![Command::StartWork]);
    assert_eq!(model.screen, Screen::Installing);
    assert_eq!(model.step, Step::Checking);
    // A second press of Install while installing does nothing.
    assert_eq!(model.on(Event::Install), vec![]);
}

#[test]
fn each_step_the_updater_reports_replaces_the_one_before_and_the_percent_follows_the_bytes() {
    let mut model = installing();
    let downloading = |done, total| Event::Progress(Step::Downloading { done, total });
    model.on(downloading(0, 168_820_736));
    assert_eq!(model.step.percent(), Some(0));
    model.on(downloading(28_311_552, 168_820_736));
    assert_eq!(model.step, Step::Downloading { done: 28_311_552, total: 168_820_736 });
    assert_eq!(model.step.percent(), Some(16), "27 of 161 MB is 16 percent, rounded down");
    model.on(Event::Progress(Step::SettingUp));
    assert_eq!(model.step, Step::SettingUp);
    assert_eq!(model.step.percent(), None);
    assert_eq!(model.screen, Screen::Installing);
}

#[test]
fn a_download_that_overshoots_or_has_no_total_never_reads_past_the_end() {
    assert_eq!(Step::Downloading { done: 10, total: 5 }.percent(), Some(100));
    assert_eq!(Step::Downloading { done: 10, total: 0 }.percent(), None);
    assert_eq!(Step::Checking.percent(), None);
}

#[test]
fn a_progress_report_outside_installing_is_ignored() {
    let mut model = Model::new(Mode::Install);
    model.on(Event::Progress(Step::SettingUp));
    assert_eq!(model.screen, Screen::Start);
    assert_eq!(model.step, Step::Checking);
}

#[test]
fn a_finished_install_shows_done_starts_artosia_and_closes_itself_a_moment_later() {
    let mut model = installing();
    assert_eq!(model.on(Event::Done { now: 10.0 }), vec![Command::StartApp]);
    assert_eq!(model.screen, Screen::Done);
    assert_eq!(model.on(Event::Tick { now: 10.0 + CLOSE_AFTER - 0.1 }), vec![]);
    assert_eq!(model.on(Event::Tick { now: 10.0 + CLOSE_AFTER }), vec![Command::Exit(0)]);
}

#[test]
fn on_done_any_way_to_close_closes_at_once() {
    for event in [Event::Escape, Event::Close, Event::Cancel] {
        let mut model = installing();
        model.on(Event::Done { now: 0.0 });
        assert_eq!(model.on(event), vec![Command::Exit(0)], "{event:?}");
    }
}

#[test]
fn a_failed_install_shows_failed_with_the_words_of_its_kind() {
    for (kind, key) in [
        (Kind::Network, Key::FailedOffline),
        (Kind::Damaged, Key::FailedDamaged),
        (Kind::Disk, Key::FailedDisk),
        (Kind::Shortcuts, Key::FailedShortcuts),
        (Kind::Other, Key::FailedOther),
    ] {
        let mut model = installing();
        assert_eq!(model.on(Event::Failed(kind)), vec![]);
        assert_eq!(model.screen, Screen::Failed);
        assert_eq!(model.failure_words(), (Key::FailedHeading, key));
    }
}

#[test]
fn try_again_goes_back_to_progress_and_starts_a_new_worker() {
    let mut model = installing();
    model.on(Event::Progress(Step::Downloading { done: 5, total: 10 }));
    model.on(Event::Failed(Kind::Network));
    assert_eq!(model.on(Event::TryAgain), vec![Command::StartWork]);
    assert_eq!(model.screen, Screen::Installing);
    assert_eq!(model.step, Step::Checking, "a new run starts from its first step");
}

#[test]
fn closing_a_failure_leaves_with_the_failure_code() {
    for event in [Event::Close, Event::Escape, Event::Cancel] {
        let mut model = installing();
        model.on(Event::Failed(Kind::Disk));
        assert_eq!(model.on(event), vec![Command::Exit(1)], "{event:?}");
    }
}

#[test]
fn cancel_escape_and_the_cross_stop_a_download_and_leave_quietly() {
    for event in [Event::Cancel, Event::Escape, Event::Close] {
        let mut model = installing();
        model.on(Event::Progress(Step::Downloading { done: 1, total: 2 }));
        assert_eq!(model.on(event), vec![Command::Exit(0)], "{event:?}");
    }
}

#[test]
fn once_the_swap_has_started_cancel_is_off_and_nothing_stops_it() {
    let mut model = installing();
    model.on(Event::Progress(Step::SettingUp));
    assert!(!model.can_cancel());
    for event in [Event::Cancel, Event::Escape, Event::Close] {
        assert_eq!(model.on(event), vec![], "{event:?}");
        assert_eq!(model.screen, Screen::Installing);
    }
}

#[test]
fn start_closes_on_escape_or_the_cross_and_leaves_quietly() {
    for event in [Event::Escape, Event::Close] {
        let mut model = Model::new(Mode::Install);
        assert_eq!(model.on(event), vec![Command::Exit(0)], "{event:?}");
    }
}

#[test]
fn an_uninstall_run_opens_on_the_question() {
    let model = Model::new(Mode::Uninstall);
    assert_eq!(model.screen, Screen::Remove);
}

#[test]
fn cancel_on_the_question_leaves_and_removes_nothing() {
    for event in [Event::Cancel, Event::Escape, Event::Close] {
        let mut model = Model::new(Mode::Uninstall);
        assert_eq!(model.on(event), vec![Command::Exit(0)], "{event:?}");
    }
}

#[test]
fn remove_with_artosia_closed_starts_removing_at_once() {
    let mut model = Model::new(Mode::Uninstall);
    assert_eq!(model.on(Event::Remove { busy: false, now: 3.0 }), vec![Command::StartWork]);
    assert_eq!(model.screen, Screen::Removing);
    assert_eq!(model.step, Step::Removing);
}

#[test]
fn remove_with_artosia_running_waits_on_close_first_and_goes_on_by_itself() {
    let mut model = Model::new(Mode::Uninstall);
    assert_eq!(model.on(Event::Remove { busy: true, now: 3.0 }), vec![]);
    assert_eq!(model.screen, Screen::CloseFirst);
    // The check is made once a second, not on every frame.
    assert_eq!(model.on(Event::Tick { now: 3.4 }), vec![]);
    assert_eq!(model.on(Event::Tick { now: 4.0 }), vec![Command::CheckBusy]);
    assert_eq!(model.on(Event::Tick { now: 4.2 }), vec![]);
    assert_eq!(model.on(Event::Busy(true)), vec![]);
    assert_eq!(model.screen, Screen::CloseFirst);
    assert_eq!(model.on(Event::Tick { now: 5.0 }), vec![Command::CheckBusy]);
    assert_eq!(model.on(Event::Busy(false)), vec![Command::StartWork]);
    assert_eq!(model.screen, Screen::Removing);
}

#[test]
fn cancel_while_waiting_for_artosia_to_close_leaves_and_removes_nothing() {
    for event in [Event::Cancel, Event::Escape, Event::Close] {
        let mut model = Model::new(Mode::Uninstall);
        model.on(Event::Remove { busy: true, now: 0.0 });
        assert_eq!(model.on(event), vec![Command::Exit(0)], "{event:?}");
    }
}

#[test]
fn removing_offers_no_cancel_and_ends_by_closing_the_window() {
    let mut model = Model::new(Mode::Uninstall);
    model.on(Event::Remove { busy: false, now: 0.0 });
    assert!(!model.can_cancel());
    for event in [Event::Cancel, Event::Escape, Event::Close] {
        assert_eq!(model.on(event), vec![], "{event:?}");
    }
    assert_eq!(model.on(Event::Done { now: 1.0 }), vec![Command::Exit(0)]);
}

#[test]
fn a_failed_removal_says_so_in_its_own_words_and_try_again_removes_again() {
    let mut model = Model::new(Mode::Uninstall);
    model.on(Event::Remove { busy: false, now: 0.0 });
    assert_eq!(model.on(Event::Failed(Kind::Disk)), vec![]);
    assert_eq!(model.screen, Screen::Failed);
    assert_eq!(model.failure_words(), (Key::RemoveFailedHeading, Key::RemoveFailedBody));
    assert_eq!(model.on(Event::TryAgain), vec![Command::StartWork]);
    assert_eq!(model.screen, Screen::Removing);
    model.on(Event::Failed(Kind::Disk));
    assert_eq!(model.on(Event::Close), vec![Command::Exit(1)]);
}

#[test]
fn the_filled_button_of_each_screen_is_the_one_that_goes_on() {
    // Start: Install. Failed: Try again. Remove: the focus starts on Cancel, the filled one is Remove.
    assert!(!Model::new(Mode::Install).focus_starts_on_cancel());
    assert!(Model::new(Mode::Uninstall).focus_starts_on_cancel());
}

#[test]
fn the_window_outcome_trusts_the_codes_the_flows_use_and_falls_back_on_any_other_end() {
    for code in [0, 1, 2, 5] {
        assert_eq!(window_outcome(Some(code)), Outcome::Finished(code), "{code}");
    }
    // The UI library's own failure exit, a panic, an abort and a crash status, and a child killed
    // without a code: the window never worked, so the task dialogs run.
    for code in [Some(70), Some(101), Some(-1073740791), Some(3), Some(255), None] {
        assert_eq!(window_outcome(code), Outcome::Dialogs, "{code:?}");
    }
}

#[test]
fn a_window_that_ends_with_one_of_our_codes_is_the_answer_and_the_dialogs_never_open() {
    for code in [0, 1, 5] {
        let opened = Cell::new(false);
        let answer = run_front(|| Some(code), || { opened.set(true); 99 });
        assert_eq!(answer, code);
        assert!(!opened.get(), "the dialogs opened after a window that worked");
    }
}

#[test]
fn a_window_that_cannot_open_hands_over_to_the_task_dialogs_and_their_code_is_the_answer() {
    // The library's error exit, a crash status and a child that left no code at all.
    for ended in [Some(70), Some(-1073740791), None] {
        let opened = Cell::new(false);
        let answer = run_front(|| ended, || { opened.set(true); 42 });
        assert_eq!(answer, 42, "{ended:?}");
        assert!(opened.get(), "{ended:?}: an old PC must still install");
    }
}
