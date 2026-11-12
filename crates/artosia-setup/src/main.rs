//! Artosia-setup.exe. No arguments installs; `--uninstall` removes; `--refresh [version]` rewrites
//! the installed-apps entry silently and does nothing when there is none. `--silent` shows
//! nothing, and `--no-start` does not start Artosia afterwards. Under a test root
//! (`ARTOSIA_SETUP_ROOT`) Artosia is never started. A failure's technical cause goes to
//! `Artosia-setup.log` in the temporary folder, because a program with no console has nowhere
//! else to say it.
//!
//! What a person sees is the setup's own window (`window.rs`), run as a child of the same exe
//! (`--show-window`) so that a window that cannot open (no usable graphics adapter, the UI library
//! failing, an abort) leaves the parent to run the task dialogs (`dialog.rs`) instead and an old
//! PC still installs. `--dialogs` goes straight to the dialogs, `--lang id` or `--lang en` picks
//! the language the way the Windows display language otherwise does, and `--window-fail` makes
//! the window process give up at once, to prove that fallback.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("Artosia-setup is for Windows.");
    std::process::exit(2);
}

#[cfg(windows)]
mod dialog;
#[cfg(windows)]
mod window;

#[cfg(windows)]
fn main() {
    std::process::exit(launch::run());
}

#[cfg(windows)]
mod launch {
    use std::path::{Path, PathBuf};

    use artosia_setup::install::{self, Failed, Step};
    use artosia_setup::places::Places;
    use artosia_setup::strings::{Key, Language, Strings};
    use artosia_update::TrustedKeys;
    use windows::Win32::Globalization::GetUserDefaultUILanguage;

    use crate::{dialog, window};
    use artosia_setup::flow::{Mode, run_front};

    const FAILED: i32 = 1;
    const BROKEN: i32 = 2;
    const IN_USE: i32 = 5;

    struct Run {
        places: Places,
        strings: Strings,
        silent: bool,
        self_exe: PathBuf,
    }

    pub fn run() -> i32 {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let has = |flag: &str| args.iter().any(|arg| arg == flag);
        let places = match Places::resolve() {
            Ok(places) => places,
            Err(error) => return log(&format!("no folders: {error}"), BROKEN),
        };
        let Ok(self_exe) = std::env::current_exe() else { return log("no path for this exe", BROKEN) };
        // SAFETY: a plain query with no arguments.
        let language = match args.iter().position(|arg| arg == "--lang").and_then(|at| args.get(at + 1)).map(String::as_str) {
            Some("id") => Language::Indonesian,
            Some("en") => Language::English,
            _ => Language::from_lang_id(unsafe { GetUserDefaultUILanguage() }),
        };
        let run = Run { places, strings: Strings::new(language), silent: has("--silent"), self_exe };
        if let Some(at) = args.iter().position(|arg| arg == "--refresh") {
            // Silent: the app runs this once after the first confirmed start of a new version.
            let version = args.get(at + 1).filter(|version| !version.starts_with("--")).map(String::as_str);
            return match install::refresh_entry(&run.places, version) {
                Ok(_) => 0,
                Err(failed) => log(&failed.detail, FAILED),
            };
        }
        let uninstalling = has("--uninstall");
        // The window process itself, or a headless render of it (`--snap`): no fallback from here.
        if has("--show-window") || rune::is_headless() {
            return in_window(&run, uninstalling, has("--no-start"), &args);
        }
        let dialogs = |run: &Run| {
            if uninstalling {
                return match uninstall(run) {
                    Ok(code) => code,
                    Err(error) => log(&format!("dialog: {error}"), BROKEN),
                };
            }
            match install(run) {
                Ok(Ok(())) if !has("--no-start") && !run.places.test_root => start(&run.places.app_exe()),
                Ok(Ok(())) => 0,
                Ok(Err(code)) => code,
                Err(error) => log(&format!("dialog: {error}"), BROKEN),
            }
        };
        if run.silent || has("--dialogs") {
            return dialogs(&run);
        }
        // The window first, as a child of this exe; its end tells whether it ever worked.
        run_front(
            || std::process::Command::new(&run.self_exe).args(&args).arg("--show-window").status().ok().and_then(|status| status.code()),
            || dialogs(&run),
        )
    }

    /// The window's own run. 70 (the "software" exit of sysexits) when the window could not start.
    fn in_window(run: &Run, uninstalling: bool, no_start: bool, args: &[String]) -> i32 {
        const WINDOW_FAILED: i32 = 70;
        if args.iter().any(|arg| arg == "--window-fail") {
            return log("the window was forced to fail", WINDOW_FAILED);
        }
        let shown_failure = match args.iter().position(|arg| arg == "--fail").and_then(|at| args.get(at + 1)).map(String::as_str) {
            Some("damaged") => artosia_setup::install::Kind::Damaged,
            Some("disk") => artosia_setup::install::Kind::Disk,
            _ => artosia_setup::install::Kind::Network,
        };
        // A headless render (`--snap`) only draws; whatever it is made to do goes to a folder of its own, never the real ones.
        let headless = rune::is_headless();
        let config = window::Config {
            mode: if uninstalling { Mode::Uninstall } else { Mode::Install },
            language: run.strings.language(),
            places: if headless { Places::under(&std::env::temp_dir().join("artosia-setup-headless")) } else { run.places.clone() },
            self_exe: run.self_exe.clone(),
            feed: artosia_update::feed_address(None).unwrap_or_default(),
            start: !no_start && !run.places.test_root && !headless,
            shown_failure,
        };
        match window::run(config) {
            Ok(code) => code,
            Err(error) => log(&format!("window: {error}"), WINDOW_FAILED),
        }
    }

    pub fn log_cause(text: &str) {
        let _ = std::fs::write(std::env::temp_dir().join("Artosia-setup.log"), text);
    }

    /// Records a cause for whoever helps, and returns the exit code.
    fn log(text: &str, code: i32) -> i32 {
        log_cause(text);
        code
    }

    pub fn start(app: &Path) -> i32 {
        let started = std::process::Command::new(app).current_dir(app.parent().unwrap_or(Path::new("."))).spawn();
        if started.is_ok() { 0 } else { log("could not start Artosia", FAILED) }
    }

    /// Ok(Ok) when installed; Ok(Err(code)) when it stopped, with the exit code to leave by.
    fn install(run: &Run) -> std::io::Result<Result<(), i32>> {
        loop {
            let places = run.places.clone();
            let (self_exe, feed) = (run.self_exe.clone(), artosia_update::feed_address(None).unwrap_or_default());
            let work = move |report: &dyn Fn(Step)| install::install(&places, &feed, &TrustedKeys::compiled(), &self_exe, report);
            let outcome: Option<Result<_, Failed>> = if run.silent {
                Some(work(&|_| {}))
            } else {
                dialog::progress(&run.strings, Key::InstallHeading, Step::Checking, move |shared| work(&|step| shared.set(step)))?
            };
            let failed = match outcome {
                None => return Ok(Err(0)),
                Some(Ok(_)) => return Ok(Ok(())),
                Some(Err(failed)) => failed,
            };
            log(&failed.detail, FAILED);
            if run.silent || !dialog::failure(&run.strings, Key::FailedHeading, failed.kind.message())? {
                return Ok(Err(FAILED));
            }
        }
    }

    fn uninstall(run: &Run) -> std::io::Result<i32> {
        if !run.silent && !dialog::confirm_remove(&run.strings)? {
            return Ok(0);
        }
        let busy = || !install::running_programs(&run.places, &run.self_exe).is_empty();
        if busy() {
            if run.silent {
                return Ok(IN_USE);
            }
            if !dialog::wait_until_closed(&run.strings, &busy)? {
                return Ok(0);
            }
        }
        loop {
            let (places, self_exe) = (run.places.clone(), run.self_exe.clone());
            let work = move || install::uninstall(&places, &self_exe);
            let outcome = if run.silent {
                Some(work())
            } else {
                dialog::progress(&run.strings, Key::RemovingHeading, Step::Removing, move |_| work())?
            };
            match outcome {
                None => return Ok(0),
                Some(Ok(())) => {
                    let _ = install::self_delete(&run.places, &run.self_exe);
                    return Ok(0);
                }
                Some(Err(failed)) => {
                    log(&failed.detail, FAILED);
                    if run.silent || !dialog::failure(&run.strings, Key::RemoveFailedHeading, Key::RemoveFailedBody)? {
                        return Ok(FAILED);
                    }
                }
            }
        }
    }
}
