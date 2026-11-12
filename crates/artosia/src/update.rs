//! Keeping Artosia up to date without a USB stick: a check in the background, a download
//! beside the program, and the swap on the next start. `artosia-update` does the work and
//! checks every file against a signature; this module decides when.
//!
//! New versions come from the latest release on GitHub, and count only when signed by the
//! key built into `artosia-update`. The `ARTOSIA_UPDATE_FEED` variable, or one line in
//! `update-feed.txt` beside the program, points a copy at another folder of releases.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use artosia_update::{Applied, UpdateError};
use rune::Waker;

use crate::state::Arrived;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Left beside the program by a start that swapped new files in, for the start after it.
const MARKER: &str = "just-updated";

/// The folder the program runs from. None in a development build or a headless run, which
/// must never rewrite themselves, and when packaged: the Store updates the program and its
/// folder is read-only. With no folder nothing below downloads, swaps or runs the setup.
pub fn install_dir() -> Option<PathBuf> {
    updatable_dir(cfg!(debug_assertions) || rune::is_headless(), crate::store::packaged(), std::env::current_exe().ok())
}

fn updatable_dir(development: bool, packaged: bool, exe: Option<PathBuf>) -> Option<PathBuf> {
    if development || packaged {
        return None;
    }
    exe?.parent().map(PathBuf::from)
}

/// Where new versions are published.
pub fn feed() -> Option<String> {
    artosia_update::feed_address(install_dir().as_deref())
}

/// Swaps in an update that was downloaded earlier. True when the new program has been
/// started and this one must leave at once, before it opens a window.
pub fn apply_before_window() -> bool {
    let Some(folder) = install_dir() else { return false };
    let relaunch = match artosia_update::apply_on_start(&folder) {
        Ok(Applied::Updated { relaunch, .. }) | Ok(Applied::RolledBack { relaunch, .. }) => relaunch,
        Ok(Applied::Nothing) | Err(_) => return false,
    };
    let Some(program) = relaunch else { return false };
    let _ = std::fs::write(folder.join(MARKER), VERSION);
    artosia_update::relaunch(&program).is_ok()
}

/// Set by `just_updated`, read once by `confirm`: this start is the first on a new version.
static FIRST_START_OF_NEW_VERSION: AtomicBool = AtomicBool::new(false);

/// Whether this start is the first one on new files. The projector left running by the old
/// version must then be replaced too.
pub fn just_updated() -> bool {
    let Some(marker) = install_dir().map(|folder| folder.join(MARKER)) else { return false };
    let first = marker.is_file() && std::fs::remove_file(marker).is_ok();
    if first {
        FIRST_START_OF_NEW_VERSION.store(true, Ordering::SeqCst);
    }
    first
}

/// Tells the updater this version started well, so it is kept. The first time a new version
/// does, the setup that installed Artosia is asked to put the version in Windows' list of
/// installed apps.
pub fn confirm() {
    if let Some(folder) = install_dir() {
        let _ = artosia_update::confirm_started(&folder);
        let refreshing = if FIRST_START_OF_NEW_VERSION.swap(false, Ordering::SeqCst) { refresh_installed_apps(&folder) } else { None };
        // The shortcuts show this version's icon: Windows keeps drawing the one it first saw.
        crate::shortcut_icon::refresh(&folder, refreshing);
    }
}

/// Runs the kept setup copy's `--refresh`, one process that nothing here waits for. A copy
/// unzipped by hand has no kept setup, and then nothing happens.
fn refresh_installed_apps(folder: &std::path::Path) -> Option<std::process::Child> {
    let setup = folder.join(artosia_update::SETUP_FILE);
    if !setup.is_file() {
        return None;
    }
    let mut command = std::process::Command::new(setup);
    command.arg("--refresh").arg(VERSION).current_dir(folder);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    command.spawn().ok()
}

/// Looks for a newer version on a worker and downloads it. `asked` is true when a person
/// pressed the button, and then every outcome is reported, also "nothing new".
pub fn check(inbox: Arc<Mutex<Vec<Arrived>>>, waker: Option<Waker>, asked: bool) {
    if crate::store::packaged() {
        return;
    }
    std::thread::spawn(move || {
        let outcome = fetch();
        let quiet = matches!(&outcome, Err(error) if error.is_quiet()) || matches!(outcome, Ok(None));
        if asked || !quiet {
            let report = outcome.map_err(|error| error.to_string());
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Update(report));
            }
            if let Some(wake) = waker {
                wake();
            }
        }
    });
}

/// The version now waiting beside the program, or None when this one is the newest.
fn fetch() -> Result<Option<String>, UpdateError> {
    let (Some(folder), Some(feed)) = (install_dir(), feed()) else { return Err(UpdateError::NotSetUp) };
    let Some(release) = artosia_update::check(&feed, VERSION)? else { return Ok(None) };
    let staged = artosia_update::download(&release, &artosia_update::staging_dir(&folder), &|_, _| {})?;
    Ok(Some(staged.version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_packaged_program_has_nothing_to_update() {
        let exe = Some(PathBuf::from("apps/x/artosia.exe"));
        assert_eq!(updatable_dir(false, true, exe.clone()), None);
        assert_eq!(updatable_dir(true, false, exe.clone()), None);
        assert_eq!(updatable_dir(false, false, exe), Some(PathBuf::from("apps/x")));
    }
}
