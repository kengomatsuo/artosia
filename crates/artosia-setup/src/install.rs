//! Installing and uninstalling, with no window: the caller passes a progress callback and draws.
//!
//! Install fetches the signed release with `artosia-update` (`check`, `download`,
//! `apply_on_start`), keeps a copy of this exe beside the program for uninstalling, writes the
//! shortcuts, and writes the uninstall entry last. Order and rollback after Velopack's
//! `install.rs` (MIT).
//!
//! Licence: the order of steps and the delayed self delete follow Velopack's `install.rs` and
//! `self_delete.rs` (MIT, (c) 2021 Caelan Sayler, 2024 Velopack Ltd.); the permission notice is in
//! THIRD-PARTY-LICENSES.md.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use artosia_update::{Applied, TrustedKeys, UpdateError, apply_on_start_with, check_with, download_with, staging_dir};

use crate::entry;
use crate::places::Places;
use crate::shortcut;
use crate::strings::{Key, Strings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Checking,
    Downloading { done: u64, total: u64 },
    SettingUp,
    Removing,
}

impl Step {
    /// How far a download is, in whole percent; None when there is no size to say it by.
    pub fn percent(self) -> Option<u8> {
        match self {
            Step::Downloading { done, total } if total > 0 => Some((done.min(total) * 100 / total) as u8),
            _ => None,
        }
    }

    /// The status line for this step, in the person's language.
    pub fn text(self, strings: &Strings) -> String {
        match self {
            Step::Checking => strings.get(Key::StepChecking).to_string(),
            Step::Downloading { done, total } => {
                let megabytes = |bytes: u64| (bytes / 1_048_576).to_string();
                strings.fill(Key::StepDownloading, &[("done", &megabytes(done)), ("total", &megabytes(total))])
            }
            Step::SettingUp => strings.get(Key::StepSettingUp).to_string(),
            Step::Removing => strings.get(Key::StepRemoving).to_string(),
        }
    }
}

/// What went wrong, in the few kinds the person can act on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Network,
    Damaged,
    Disk,
    Shortcuts,
    Other,
}

impl Kind {
    /// The text that tells the person what happened and what to do.
    pub fn message(self) -> Key {
        match self {
            Kind::Network => Key::FailedOffline,
            Kind::Damaged => Key::FailedDamaged,
            Kind::Disk => Key::FailedDisk,
            Kind::Shortcuts => Key::FailedShortcuts,
            Kind::Other => Key::FailedOther,
        }
    }
}

#[derive(Debug)]
pub struct Failed {
    pub kind: Kind,
    /// The technical cause, for the log and for silent runs; never for the dialog.
    pub detail: String,
}

impl Failed {
    fn new(kind: Kind, detail: impl ToString) -> Failed {
        Failed { kind, detail: detail.to_string() }
    }
}

impl From<UpdateError> for Failed {
    fn from(error: UpdateError) -> Failed {
        let kind = match error {
            UpdateError::Offline | UpdateError::Slow | UpdateError::NotPublished | UpdateError::Server(_) => Kind::Network,
            UpdateError::BadList(_) | UpdateError::BadSignature | UpdateError::WrongPlatform | UpdateError::BadFile(_) | UpdateError::BadVersion(_) => {
                Kind::Damaged
            }
            UpdateError::Disk(_) | UpdateError::InstallFailed(_) | UpdateError::RestoreFailed(_) => Kind::Disk,
            UpdateError::NotSetUp | UpdateError::NothingStaged | UpdateError::StaleStaged | UpdateError::Rejected => Kind::Other,
        };
        let detail = match error.detail() {
            Some(cause) => format!("{error}: {cause}"),
            None => error.to_string(),
        };
        Failed { kind, detail }
    }
}

#[derive(Debug)]
pub struct Installed {
    pub version: String,
    /// False when the program was already in the folder and nothing was downloaded.
    pub downloaded: bool,
}

/// Installs into `places.install`. `keys` is `TrustedKeys::compiled()` outside tests.
pub fn install(places: &Places, feed: &str, keys: &TrustedKeys, self_exe: &Path, report: &dyn Fn(Step)) -> Result<Installed, Failed> {
    let (version, bytes, downloaded) = if places.app_exe().is_file() {
        let version = crate::installed_version(places).unwrap_or_default();
        (version, dir_size(&places.install), false)
    } else {
        report(Step::Checking);
        let release = check_with(feed, "0.0.0", keys)?.ok_or_else(|| Failed::new(Kind::Other, "the feed has no release"))?;
        download_with(&release, &staging_dir(&places.install), keys, &|done, total| report(Step::Downloading { done, total }))?;
        report(Step::SettingUp);
        match apply_on_start_with(&places.install, keys)? {
            Applied::Updated { .. } => {}
            other => return Err(Failed::new(Kind::Other, format!("the files were not put in place: {other:?}"))),
        }
        (release.version, release.size, true)
    };
    keep_setup_copy(places, self_exe)?;
    if let Err(failed) = write_shortcuts_and_entry(places, &version, bytes) {
        // A failed install leaves no shortcut and no entry; the files stay so a second try is quick.
        let _ = shortcut::remove(&places.start_lnk());
        let _ = shortcut::remove(&places.desktop_lnk());
        let _ = entry::remove(places);
        return Err(failed);
    }
    Ok(Installed { version, downloaded })
}

/// Rewrites the uninstall entry from what is installed now, so the installed-apps list follows
/// a self-update. `version` is what the running program says it is; without it the version
/// resource of `artosia.exe` is read. Does nothing, and says so with `false`, when there is no
/// entry (a copy unzipped by hand, or an uninstall that already ran) or no program.
pub fn refresh_entry(places: &Places, version: Option<&str>) -> Result<bool, Failed> {
    if entry::read(places, "DisplayName").is_none() || !places.app_exe().is_file() {
        return Ok(false);
    }
    let version = version
        .map(str::to_string)
        .or_else(|| crate::installed_version(places))
        .ok_or_else(|| Failed::new(Kind::Other, "no version to write"))?;
    entry::write(places, &entry::uninstall_values(places, &version, dir_size(&places.install))).map_err(|e| Failed::new(Kind::Disk, e))?;
    Ok(true)
}

fn write_shortcuts_and_entry(places: &Places, version: &str, bytes: u64) -> Result<(), Failed> {
    let (app, folder) = (places.app_exe(), &places.install);
    shortcut::create(&places.start_lnk(), &app, folder).map_err(|e| Failed::new(Kind::Shortcuts, e))?;
    shortcut::create(&places.desktop_lnk(), &app, folder).map_err(|e| Failed::new(Kind::Shortcuts, e))?;
    entry::write(places, &entry::uninstall_values(places, version, bytes)).map_err(|e| Failed::new(Kind::Disk, e))
}

/// The copy of this exe that the uninstall entry points at. It is not in any release's file
/// list, and `artosia-update` only ever swaps, restores or deletes files a manifest lists, so
/// every later update leaves it alone.
fn keep_setup_copy(places: &Places, self_exe: &Path) -> Result<(), Failed> {
    let target = places.setup_exe();
    if same_file(self_exe, &target) {
        return Ok(());
    }
    fs::create_dir_all(&places.install).map_err(|e| Failed::new(Kind::Disk, e))?;
    match fs::copy(self_exe, &target) {
        Ok(_) => Ok(()),
        // The old copy is running or locked: it still uninstalls, so keep it.
        Err(_) if target.is_file() => Ok(()),
        Err(error) => Err(Failed::new(Kind::Disk, error)),
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((fs::canonicalize(a), fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

fn dir_size(folder: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(folder) else { return 0 };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_size(&entry.path()),
            _ => entry.metadata().map_or(0, |meta| meta.len()),
        })
        .sum()
}

// ---- uninstall ----

/// The programs of the install folder that are running now, by file name. A running exe refuses
/// to be opened for writing (sharing violation); nothing is ever killed.
pub fn running_programs(places: &Places, self_exe: &Path) -> Vec<String> {
    const SHARING_VIOLATION: i32 = 32;
    let Ok(entries) = fs::read_dir(&places.install) else { return Vec::new() };
    entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("exe")) && !same_file(&entry.path(), self_exe))
        .filter(|entry| {
            fs::OpenOptions::new().write(true).open(entry.path()).is_err_and(|error| error.raw_os_error() == Some(SHARING_VIOLATION))
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Removes the program, both shortcuts and the entry. The entry goes last and only when every
/// file went, so a run that stopped half way can be run again. The church's own folders
/// (`places.data`) and this exe are never touched; see `self_delete` for the exe.
pub fn uninstall(places: &Places, self_exe: &Path) -> Result<(), Failed> {
    let mut stuck: Vec<String> = Vec::new();
    for lnk in [places.start_lnk(), places.desktop_lnk()] {
        if let Err(error) = shortcut::remove(&lnk) {
            stuck.push(format!("{}: {error}", lnk.display()));
        }
    }
    stuck.extend(remove_tree(&places.install, &places.data, self_exe).into_iter().map(|(path, error)| format!("{}: {error}", path.display())));
    if !stuck.is_empty() {
        return Err(Failed::new(Kind::Disk, stuck.join("; ")));
    }
    entry::remove(places).map_err(|e| Failed::new(Kind::Disk, e))
}

fn lower(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

/// Deletes what is under `folder` except `keep` and anything that is, holds or sits inside one
/// of `protected`. Returns each path that would not go. Antivirus often holds a file for a
/// moment right after it is scanned, so a refusal is tried again a few times.
fn remove_tree(folder: &Path, protected: &[PathBuf], keep: &Path) -> Vec<(PathBuf, io::Error)> {
    let (keep, protected): (PathBuf, Vec<PathBuf>) = (lower(keep), protected.iter().map(|p| lower(p)).collect());
    let mut stuck = Vec::new();
    remove_inside(folder, &protected, &keep, &mut stuck);
    // The folder itself, when nothing is left in it.
    if stuck.is_empty() {
        let _ = fs::remove_dir(folder);
    }
    stuck
}

fn remove_inside(folder: &Path, protected: &[PathBuf], keep: &Path, stuck: &mut Vec<(PathBuf, io::Error)>) {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => return stuck.push((folder.to_path_buf(), error)),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = lower(&path);
        if name == keep || protected.iter().any(|data| name.starts_with(data)) {
            continue;
        }
        let holds_data = protected.iter().any(|data| data.starts_with(&name));
        let is_real_folder = entry.file_type().is_ok_and(|kind| kind.is_dir() && !kind.is_symlink());
        if is_real_folder {
            remove_inside(&path, protected, keep, stuck);
            if !holds_data
                && !stuck.iter().any(|(p, _)| p.starts_with(&path))
                && let Err(error) = retry(|| fs::remove_dir(&path))
            {
                stuck.push((path, error));
            }
        } else if let Err(error) = retry(|| fs::remove_file(&path).or_else(|_| fs::remove_dir(&path))) {
            stuck.push((path, error));
        }
    }
}

fn retry(mut action: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut result = action();
    for _ in 0..4 {
        match &result {
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                std::thread::sleep(Duration::from_millis(200));
                result = action();
            }
            _ => break,
        }
    }
    match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// A running exe cannot delete itself, so when it sits in the install folder a hidden `cmd`
/// waits a few seconds, deletes it and then the folder (`rmdir` without `/s`, so it can never
/// take a folder that still holds data). After Velopack's `self_delete.rs` (MIT).
#[cfg(windows)]
pub fn self_delete(places: &Places, self_exe: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    if !lower(self_exe).starts_with(lower(&places.install)) {
        return Ok(());
    }
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let script = format!("/C ping -n 4 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{}\"", self_exe.display(), places.install.display());
    std::process::Command::new("cmd.exe")
        .raw_arg(script)
        .current_dir(std::env::temp_dir())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}
