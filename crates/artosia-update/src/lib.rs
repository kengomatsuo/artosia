//! Self-update from a static folder of files with a signed manifest. No UI.
//!
//! Flow: `check` on a worker thread, `download` into `staging_dir(install_dir)`, then on the
//! next start `apply_on_start` swaps the files in; `confirm_started` makes it permanent.
//! See README.md for the manifest format and how a release is made.

mod apply;
mod error;
mod fetch;
pub mod manifest;
pub mod publish;

pub use apply::{
    Applied, RollbackReason, apply_on_start, apply_on_start_with, confirm_started, own_files_dir,
    relaunch, roll_back, staging_dir,
};
pub use error::UpdateError;
pub use fetch::{Release, Staged, check, check_with, download, download_with};
pub use manifest::TrustedKeys;

/// The files of the newest release, wherever it was tagged. The app and the setup read this one.
pub const RELEASES: &str = "https://github.com/kengomatsuo/artosia/releases/latest/download";

/// The file name of the setup program kept in the install folder. It is in no release's file
/// list, so no update touches it; the app runs it to refresh the installed-apps entry.
pub const SETUP_FILE: &str = "Artosia-setup.exe";

/// Where new versions are published: the `ARTOSIA_UPDATE_FEED` variable, then the first line of
/// `update-feed.txt` beside the program (`beside`), then `RELEASES`. None when the chosen line is empty.
pub fn feed_address(beside: Option<&std::path::Path>) -> Option<String> {
    let from_file = || std::fs::read_to_string(beside?.join("update-feed.txt")).ok();
    let feed = std::env::var("ARTOSIA_UPDATE_FEED").ok().or_else(from_file).unwrap_or_else(|| RELEASES.to_string());
    let feed = feed.lines().next().unwrap_or("").trim().to_string();
    (!feed.is_empty()).then_some(feed)
}
