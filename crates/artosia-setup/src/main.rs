//! Artosia-setup.exe. No arguments installs; `--uninstall` removes; `--refresh [version]` rewrites
//! the installed-apps entry silently and does nothing when there is none. `--silent` shows no
//! dialog, and `--no-start` does not start Artosia afterwards. Under a test root (`ARTOSIA_SETUP_ROOT`)
//! Artosia is never started. A failure's technical cause goes to `Artosia-setup.log` in the
//! temporary folder, because a window-less program has nowhere else to say it.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("Artosia-setup is for Windows.");
    std::process::exit(2);
}

#[cfg(windows)]
mod dialog;

#[cfg(windows)]
fn main() {
    std::process::exit(flow::run());
}

#[cfg(windows)]
mod flow {
    use std::path::{Path, PathBuf};

    use artosia_setup::install::{self, Failed, Step};
    use artosia_setup::places::Places;
    use artosia_setup::strings::{Key, Language, Strings};
    use artosia_update::TrustedKeys;
    use windows::Win32::Globalization::GetUserDefaultUILanguage;

    use crate::dialog;

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
        let language = Language::from_lang_id(unsafe { GetUserDefaultUILanguage() });
        let run = Run { places, strings: Strings::new(language), silent: has("--silent"), self_exe };
        if let Some(at) = args.iter().position(|arg| arg == "--refresh") {
            // Silent: the app runs this once after the first confirmed start of a new version.
            let version = args.get(at + 1).filter(|version| !version.starts_with("--")).map(String::as_str);
            return match install::refresh_entry(&run.places, version) {
                Ok(_) => 0,
                Err(failed) => log(&failed.detail, FAILED),
            };
        }
        if has("--uninstall") {
            return match uninstall(&run) {
                Ok(code) => code,
                Err(error) => log(&format!("dialog: {error}"), BROKEN),
            };
        }
        match install(&run) {
            Ok(Ok(())) if !has("--no-start") && !run.places.test_root => start(&run.places.app_exe()),
            Ok(Ok(())) => 0,
            Ok(Err(code)) => code,
            Err(error) => log(&format!("dialog: {error}"), BROKEN),
        }
    }

    /// Records a cause for whoever helps, and returns the exit code.
    fn log(text: &str, code: i32) -> i32 {
        let _ = std::fs::write(std::env::temp_dir().join("Artosia-setup.log"), text);
        code
    }

    fn start(app: &Path) -> i32 {
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
