//! Artosia-setup: a small per-user setup that fetches the signed release with `artosia-update`.
//! Nothing release-specific is built in: it installs whatever the feed's newest release is, so
//! the same file serves every release. See docs/setup.md.

pub mod entry;
pub mod flow;
pub mod places;
pub mod strings;

#[cfg(windows)]
pub mod install;
#[cfg(windows)]
pub mod shortcut;
#[cfg(windows)]
pub mod win;

/// The version of the program in the install folder: its version resource, else what the
/// uninstall entry says.
#[cfg(windows)]
pub fn installed_version(places: &places::Places) -> Option<String> {
    win::file_version(&places.app_exe()).or_else(|| match entry::read(places, "DisplayVersion") {
        Some(entry::Value::Text(version)) => Some(version),
        _ => None,
    })
}
