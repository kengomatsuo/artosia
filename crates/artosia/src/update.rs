//! Keeping Artosia up to date without a USB stick: a check in the background, a download
//! beside the program, and the swap on the next start. `artosia-update` does the work and
//! checks every file against a signature; this module decides when.
//!
//! New versions come from the latest release on GitHub, and count only when signed by the
//! key built into `artosia-update`. The `ARTOSIA_UPDATE_FEED` variable, or one line in
//! `update-feed.txt` beside the program, points a copy at another folder of releases.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use artosia_update::{Applied, UpdateError};
use rune::Waker;

use crate::state::Arrived;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Left beside the program by a start that swapped new files in, for the start after it.
const MARKER: &str = "just-updated";

/// The folder the program runs from. None in a development build or a headless run, which
/// must never rewrite themselves.
pub fn install_dir() -> Option<PathBuf> {
    if cfg!(debug_assertions) || rune::is_headless() {
        return None;
    }
    std::env::current_exe().ok()?.parent().map(PathBuf::from)
}

/// The files of the newest release, wherever it was tagged.
const RELEASES: &str = "https://github.com/kengomatsuo/artosia/releases/latest/download";

/// Where new versions are published.
pub fn feed() -> Option<String> {
    let from_file = || std::fs::read_to_string(install_dir()?.join("update-feed.txt")).ok();
    let feed = std::env::var("ARTOSIA_UPDATE_FEED").ok().or_else(from_file).unwrap_or_else(|| RELEASES.to_string());
    let feed = feed.lines().next().unwrap_or("").trim().to_string();
    (!feed.is_empty()).then_some(feed)
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

/// Whether this start is the first one on new files. The projector left running by the old
/// version must then be replaced too.
pub fn just_updated() -> bool {
    let Some(marker) = install_dir().map(|folder| folder.join(MARKER)) else { return false };
    marker.is_file() && std::fs::remove_file(marker).is_ok()
}

/// Tells the updater this version started well, so it is kept.
pub fn confirm() {
    if let Some(folder) = install_dir() {
        let _ = artosia_update::confirm_started(&folder);
    }
}

/// Looks for a newer version on a worker and downloads it. `asked` is true when a person
/// pressed the button, and then every outcome is reported, also "nothing new".
pub fn check(inbox: Arc<Mutex<Vec<Arrived>>>, waker: Option<Waker>, asked: bool) {
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
