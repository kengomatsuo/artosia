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
    let folder = exe?.parent()?.to_path_buf();
    // A Mac app keeps its programs in `Contents/MacOS`, and a release is every file of
    // `Contents`: the signature's own list and the bundled GStreamer are part of it.
    Some(match folder.parent() {
        Some(contents) if folder.ends_with("Contents/MacOS") => contents.to_path_buf(),
        _ => folder,
    })
}

/// Where the mark of a start that swapped new files in is left: beside the program, or, for a
/// Mac app, outside it, because a file added to a signed app breaks its seal.
fn marker() -> Option<PathBuf> {
    install_dir().map(|folder| artosia_update::own_files_dir(&folder).join(MARKER))
}

/// Where new versions are published.
pub fn feed() -> Option<String> {
    artosia_update::feed_address(install_dir().as_deref())
}

/// Before the window opens: finishes or undoes an interrupted update and counts a trial start.
/// A downloaded update is installed only on the start Restart now asked for (`--apply-update`);
/// any other start leaves it waiting, and the window offers it (UPDATEASK1). True when the new
/// program has been started and this one must leave at once, before it opens a window.
pub fn apply_before_window() -> bool {
    let Some(folder) = install_dir() else { return false };
    let asked = crate::data::has_flag("--apply-update");
    if asked {
        wait_for_old_copy();
    }
    let outcome = if asked { artosia_update::apply_on_start(&folder) } else { artosia_update::finish_on_start(&folder) };
    let relaunch = match outcome {
        Ok(Applied::Updated { relaunch, .. }) | Ok(Applied::RolledBack { relaunch, .. }) => relaunch,
        Ok(Applied::Nothing) | Err(_) => return false,
    };
    let Some(program) = relaunch else { return false };
    if let Some(marker) = marker() {
        let _ = std::fs::write(marker, VERSION);
    }
    artosia_update::relaunch(&program).is_ok()
}

/// The copy that pressed Restart now is still closing: wait, at most fifteen seconds, until it
/// lets go of the port that keeps one Artosia per library, so the updated program does not
/// find it still running and step aside.
fn wait_for_old_copy() {
    for _ in 0..150 {
        if std::net::TcpListener::bind(("127.0.0.1", crate::INSTANCE_PORT)).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// The version of a downloaded update waiting to be installed, if any.
pub fn waiting() -> Option<String> {
    install_dir().and_then(|folder| artosia_update::staged_version(&folder))
}

/// Restart now: starts a copy that installs the waiting update before its window opens. The
/// caller then quits; the copy waits for it to close first.
pub fn restart_to_update() -> bool {
    let Ok(program) = std::env::current_exe() else { return false };
    std::process::Command::new(program).arg("--apply-update").spawn().is_ok()
}

/// Set by `just_updated`, read once by `confirm`: this start is the first on a new version.
static FIRST_START_OF_NEW_VERSION: AtomicBool = AtomicBool::new(false);

/// Whether this start is the first one on new files. The projector left running by the old
/// version must then be replaced too.
pub fn just_updated() -> bool {
    let Some(marker) = marker() else { return false };
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
    let stage = artosia_update::staging_dir(&folder);
    // A Mac app opened from its disk image, or from Downloads before it was moved, runs from
    // a place nothing can be written to. Such a copy cannot update itself, and says nothing.
    if cfg!(target_os = "macos") && std::fs::create_dir_all(&stage).is_err() {
        return Err(UpdateError::NotSetUp);
    }
    let staged = artosia_update::download(&release, &stage, &|_, _| {})?;
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

    #[test]
    fn a_mac_app_is_updated_as_its_whole_contents_folder() {
        let exe = Some(PathBuf::from("/Applications/Artosia.app/Contents/MacOS/artosia"));
        let contents = PathBuf::from("/Applications/Artosia.app/Contents");
        assert_eq!(updatable_dir(false, false, exe), Some(contents.clone()));
        // Its mark and its downloads wait beside the app, never inside it.
        assert_eq!(artosia_update::own_files_dir(&contents), PathBuf::from("/Applications/.Artosia-update"));
        assert_eq!(artosia_update::own_files_dir(&PathBuf::from("apps/x")), PathBuf::from("apps/x"));
        // A program that merely sits in a folder named MacOS is where it is.
        let loose = Some(PathBuf::from("tools/MacOS/artosia"));
        assert_eq!(updatable_dir(false, false, loose), Some(PathBuf::from("tools/MacOS")));
    }
}
