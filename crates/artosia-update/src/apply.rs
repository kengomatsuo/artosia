//! Swapping a downloaded update into the install folder on the next start, and undoing it.
//!
//! Windows refuses to overwrite or delete a running exe but allows renaming it, so each file
//! being replaced is first renamed to `<name>.artosia-old` and the staged file renamed into
//! its place. A journal (`update/state.json`) is written before the first rename, so a power
//! cut half way through is rolled back on the next start.

use crate::error::UpdateError;
use crate::fetch::{installed_matches, write_atomic};
use crate::manifest::{
    self, FileEntry, MANIFEST_NAME, Manifest, SIGNATURE_NAME, TrustedKeys, sha256_hex_of_file,
    this_platform,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const OLD: &str = ".artosia-old";
const NEW: &str = ".artosia-new";
/// Starts of a new build that never reached `confirm_started` before it is rolled back.
const MAX_UNCONFIRMED_STARTS: u32 = 2;

/// Where downloads and the journal live: a folder inside the install folder, so renames stay
/// on one volume. A Mac app is the exception: its signature seals every file inside it, and
/// one file more makes `codesign` say "a sealed resource is missing or invalid" (measured on
/// macOS 27, 2026-11-13). Its folder is a hidden one beside the app, on the same volume, so
/// the app holds a release's files and nothing else.
pub fn staging_dir(install_dir: &Path) -> PathBuf {
    match mac_app(install_dir) {
        Some(app) => {
            let name = app.file_stem().unwrap_or_default().to_string_lossy();
            app.with_file_name(format!(".{name}-update"))
        }
        None => install_dir.join("update"),
    }
}

/// The install folder a staging folder belongs to, the other way round from `staging_dir`: the
/// folder it is in, or, for the hidden folder beside a Mac app, that app's `Contents`. A
/// download asks it which files are installed already, so only the changed ones are fetched;
/// asked of the app's own folder, a Mac fetched all of a release every time.
pub(crate) fn install_dir_of(stage: &Path) -> PathBuf {
    let parent = stage.parent().unwrap_or(stage);
    let name = stage.file_name().unwrap_or_default().to_string_lossy();
    if let Some(app) = name.strip_prefix('.').and_then(|rest| rest.strip_suffix("-update")) {
        let contents = parent.join(format!("{app}.app")).join("Contents");
        if contents.is_dir() {
            return contents;
        }
    }
    parent.to_path_buf()
}

/// Where a small file about this copy goes that belongs to no release, such as the operator's
/// mark that it has just been updated: the install folder, or a Mac app's staging folder.
pub fn own_files_dir(install_dir: &Path) -> PathBuf {
    if mac_app(install_dir).is_some() { staging_dir(install_dir) } else { install_dir.to_path_buf() }
}

/// The app an install folder belongs to when it is the `Contents` of a Mac app.
fn mac_app(install_dir: &Path) -> Option<&Path> {
    let app = install_dir.parent()?;
    (install_dir.file_name()? == "Contents" && app.extension()? == "app").then_some(app)
}

/// Where a file waits under another name (`lib/data.bin.artosia-old`) until the update is kept
/// or undone: beside itself, or in the staging folder when the install folder is a Mac app.
fn aside(install_dir: &Path, name: &str) -> PathBuf {
    match mac_app(install_dir) {
        Some(_) => staging_dir(install_dir).join("aside").join(name),
        None => install_dir.join(name),
    }
}

/// Moves a file to its waiting place, making the folders on the way.
fn set_aside(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)?;
    }
    rename_patiently(from, to)
}

/// Renames, and on Windows tries again for a little over a second when another program has
/// the file open. A virus scanner reading a program that was just written or started holds it
/// for a moment ("The process cannot access the file because it is being used by another
/// process", met in this crate's own tests on 2026-11-13), and rustup's `utils::rename` waits
/// out the same thing. A file that stays held fails as before.
fn rename_patiently(from: &Path, to: &Path) -> io::Result<()> {
    const SHARING_VIOLATION: i32 = 32;
    let mut wait = 20;
    loop {
        match fs::rename(from, to) {
            Err(e)
                if cfg!(windows)
                    && wait <= 640
                    && (e.raw_os_error() == Some(SHARING_VIOLATION) || e.kind() == io::ErrorKind::PermissionDenied) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(wait));
                wait *= 2;
            }
            done => return done,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RollbackReason {
    /// The power or the program died in the middle of the swap.
    Interrupted,
    /// The new build started several times without confirming it works.
    NeverStarted,
    /// `roll_back` was called.
    Requested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    Nothing,
    /// New files are in place. This process still runs the old code: start `relaunch` and exit.
    Updated {
        version: String,
        relaunch: Option<PathBuf>,
    },
    /// Old files are back. Start `relaunch` and exit.
    RolledBack {
        version: String,
        reason: RollbackReason,
        relaunch: Option<PathBuf>,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Applying,
    Trial,
    Cleanup,
}

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    path: String,
    had_old: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct State {
    phase: Phase,
    version: String,
    starts: u32,
    entries: Vec<Entry>,
    /// Files that were locked when we tried to delete them; retried on every start.
    leftovers: Vec<String>,
}

pub(crate) fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    fs::rename(from, to)
}

fn load_state(stage: &Path) -> Option<State> {
    serde_json::from_slice(&fs::read(stage.join("state.json")).ok()?).ok()
}

fn save_state(stage: &Path, state: &State) -> Result<(), UpdateError> {
    let bytes = serde_json::to_vec_pretty(state).map_err(|e| UpdateError::Disk(e.to_string()))?;
    write_atomic(&stage.join("state.json"), &bytes)
}

fn clear_state(stage: &Path) {
    let _ = fs::remove_file(stage.join("state.json"));
}

pub(crate) fn rejected_versions(stage: &Path) -> Vec<String> {
    fs::read(stage.join("rejected.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn reject(stage: &Path, version: &str) {
    let mut all = rejected_versions(stage);
    if !all.iter().any(|v| v == version) {
        all.push(version.to_string());
    }
    if let Ok(bytes) = serde_json::to_vec(&all) {
        let _ = write_atomic(&stage.join("rejected.json"), &bytes);
    }
}

fn drop_staged(stage: &Path) {
    let _ = fs::remove_file(stage.join(SIGNATURE_NAME));
    let _ = fs::remove_file(stage.join(MANIFEST_NAME));
    let _ = fs::remove_dir_all(stage.join("files"));
}

/// The path of the running program inside the install folder, taken before any file moves
/// (Windows reports a renamed running exe under its new name). The program is in the folder
/// itself, or below it: a Mac app keeps its programs in `Contents/MacOS`.
fn relaunch_path(install_dir: &Path) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let folder = fs::canonicalize(exe.parent()?).ok()?;
    let below = folder.strip_prefix(fs::canonicalize(install_dir).ok()?).ok()?.to_path_buf();
    Some(install_dir.join(below).join(exe.file_name()?))
}

/// Starts `path` with this process's own arguments and returns at once; the caller then exits.
pub fn relaunch(path: &Path) -> io::Result<()> {
    std::process::Command::new(path)
        .args(std::env::args_os().skip(1))
        .spawn()
        .map(|_| ())
}

/// Call first thing in the operator's `main`. Applies a verified staged update, finishes or
/// undoes an interrupted one, and rolls back a build that never confirmed it starts.
pub fn apply_on_start(install_dir: &Path) -> Result<Applied, UpdateError> {
    apply_on_start_with(install_dir, &TrustedKeys::compiled())
}

pub fn apply_on_start_with(install_dir: &Path, keys: &TrustedKeys) -> Result<Applied, UpdateError> {
    on_start(install_dir, keys, true)
}

/// Like `apply_on_start`, but a downloaded update keeps waiting: an interrupted swap is finished
/// or undone, a trial start is counted and a build that never confirmed is rolled back, and
/// nothing new is installed. The operator calls it until the person asks for the update.
pub fn finish_on_start(install_dir: &Path) -> Result<Applied, UpdateError> {
    finish_on_start_with(install_dir, &TrustedKeys::compiled())
}

pub fn finish_on_start_with(install_dir: &Path, keys: &TrustedKeys) -> Result<Applied, UpdateError> {
    on_start(install_dir, keys, false)
}

/// The version of a downloaded, verified update waiting beside the program; None while none
/// waits, while a swap is under way or on trial, and for a version that was rolled back.
pub fn staged_version(install_dir: &Path) -> Option<String> {
    staged_version_with(install_dir, &TrustedKeys::compiled())
}

pub fn staged_version_with(install_dir: &Path, keys: &TrustedKeys) -> Option<String> {
    let stage = staging_dir(install_dir);
    if matches!(load_state(&stage).map(|state| state.phase), Some(Phase::Applying | Phase::Trial)) {
        return None;
    }
    if !stage.join(SIGNATURE_NAME).is_file() {
        return None;
    }
    let manifest = read_staged(&stage, keys).ok()?;
    (!rejected_versions(&stage).contains(&manifest.version)).then_some(manifest.version)
}

fn on_start(install_dir: &Path, keys: &TrustedKeys, install_new: bool) -> Result<Applied, UpdateError> {
    let stage = staging_dir(install_dir);
    if !stage.is_dir() {
        return Ok(Applied::Nothing);
    }
    let relaunch = relaunch_path(install_dir);
    if let Some(mut state) = load_state(&stage) {
        match state.phase {
            Phase::Applying => {
                return roll_back_state(install_dir, state, RollbackReason::Interrupted, relaunch);
            }
            Phase::Trial if state.starts >= MAX_UNCONFIRMED_STARTS => {
                return roll_back_state(install_dir, state, RollbackReason::NeverStarted, relaunch);
            }
            Phase::Trial => {
                state.starts += 1;
                save_state(&stage, &state)?;
                return Ok(Applied::Nothing);
            }
            Phase::Cleanup => {
                state.leftovers = remove_leftovers(install_dir, &state.leftovers);
                if state.leftovers.is_empty() {
                    clear_state(&stage);
                } else {
                    save_state(&stage, &state)?;
                }
            }
        }
    }
    if !install_new || !stage.join(SIGNATURE_NAME).is_file() {
        return Ok(Applied::Nothing);
    }
    match apply_staged(install_dir, &stage, keys) {
        Ok(version) => Ok(Applied::Updated { version, relaunch }),
        Err(e) => {
            if !matches!(e, UpdateError::InstallFailed(_)) {
                drop_staged(&stage);
            }
            Err(e)
        }
    }
}

fn read_staged(stage: &Path, keys: &TrustedKeys) -> Result<Manifest, UpdateError> {
    let bytes = fs::read(stage.join(MANIFEST_NAME))?;
    let sig = fs::read_to_string(stage.join(SIGNATURE_NAME))?;
    let m = manifest::verify(&bytes, sig.trim(), keys)?;
    if m.platform != this_platform() {
        return Err(UpdateError::WrongPlatform);
    }
    Ok(m)
}

fn apply_staged(
    install_dir: &Path,
    stage: &Path,
    keys: &TrustedKeys,
) -> Result<String, UpdateError> {
    let m = read_staged(stage, keys)?;
    if rejected_versions(stage).contains(&m.version) {
        return Err(UpdateError::Rejected);
    }
    let files_dir = stage.join("files");
    let mut swaps: Vec<(&FileEntry, PathBuf)> = Vec::new();
    for f in &m.files {
        let staged = files_dir.join(&f.name);
        if staged.is_file() {
            if sha256_hex_of_file(&staged)? != f.sha256 {
                return Err(UpdateError::BadFile(format!("staged {} changed", f.name)));
            }
            swaps.push((f, staged));
        } else if !installed_matches(install_dir, f) {
            return Err(UpdateError::StaleStaged);
        }
    }
    let entries: Vec<Entry> = swaps
        .iter()
        .map(|(f, _)| Entry {
            path: f.path.clone(),
            had_old: install_dir.join(&f.path).exists(),
        })
        .collect();
    // Leftovers from an earlier update must not be mistaken for this one's originals.
    for e in &entries {
        for suffix in [OLD, NEW] {
            let stale = aside(install_dir, &format!("{}{suffix}", e.path));
            match fs::remove_file(&stale) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(UpdateError::InstallFailed(format!(
                        "{}: {err}",
                        stale.display()
                    )));
                }
            }
        }
    }
    let mut state = State {
        phase: Phase::Applying,
        version: m.version.clone(),
        starts: 0,
        entries: entries.clone(),
        leftovers: Vec::new(),
    };
    save_state(stage, &state)?;
    for (f, staged) in &swaps {
        if let Err(e) = swap_one(install_dir, f, staged) {
            let left = restore(install_dir, &entries).unwrap_or_default();
            state.leftovers = left;
            if state.leftovers.is_empty() {
                clear_state(stage);
            } else {
                state.phase = Phase::Cleanup;
                let _ = save_state(stage, &state);
            }
            return Err(UpdateError::InstallFailed(e.to_string()));
        }
    }
    state.phase = Phase::Trial;
    save_state(stage, &state)?;
    drop_staged(stage);
    Ok(m.version)
}

fn swap_one(install_dir: &Path, f: &FileEntry, staged: &Path) -> io::Result<()> {
    let dest = install_dir.join(&f.path);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    if f.exec {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(staged, fs::Permissions::from_mode(0o755))?;
    }
    if dest.exists() {
        set_aside(&dest, &aside(install_dir, &format!("{}{OLD}", f.path)))?;
    }
    rename_patiently(staged, &dest)
}

/// Puts every journaled file back as it was. Returns the new files it could not delete.
fn restore(install_dir: &Path, entries: &[Entry]) -> Result<Vec<String>, UpdateError> {
    let mut leftovers = Vec::new();
    for e in entries.iter().rev() {
        let dest = install_dir.join(&e.path);
        let old = aside(install_dir, &format!("{}{OLD}", e.path));
        let new = aside(install_dir, &format!("{}{NEW}", e.path));
        let fail = |err: io::Error| UpdateError::RestoreFailed(format!("{}: {err}", e.path));
        if old.exists() {
            if dest.exists() {
                // The new file may be a running exe, so move it aside rather than delete it.
                let _ = fs::remove_file(&new);
                set_aside(&dest, &new).map_err(fail)?;
                leftovers.push(format!("{}{NEW}", e.path));
            }
            rename_patiently(&old, &dest).map_err(fail)?;
        } else if !e.had_old && dest.exists() && fs::remove_file(&dest).is_err() {
            set_aside(&dest, &new).map_err(fail)?;
            leftovers.push(format!("{}{NEW}", e.path));
        }
    }
    Ok(leftovers)
}

/// Deletes what it can; returns what is still locked.
fn remove_leftovers(install_dir: &Path, list: &[String]) -> Vec<String> {
    list.iter()
        .filter(|p| {
            let path = aside(install_dir, p);
            path.exists() && fs::remove_file(&path).is_err()
        })
        .cloned()
        .collect()
}

fn roll_back_state(
    install_dir: &Path,
    mut state: State,
    reason: RollbackReason,
    relaunch: Option<PathBuf>,
) -> Result<Applied, UpdateError> {
    let stage = staging_dir(install_dir);
    let mut leftovers = restore(install_dir, &state.entries)?;
    leftovers.append(&mut state.leftovers);
    reject(&stage, &state.version);
    drop_staged(&stage);
    let leftovers = remove_leftovers(install_dir, &leftovers);
    if leftovers.is_empty() {
        clear_state(&stage);
    } else {
        state.phase = Phase::Cleanup;
        state.leftovers = leftovers;
        save_state(&stage, &state)?;
    }
    Ok(Applied::RolledBack {
        version: state.version,
        reason,
        relaunch,
    })
}

/// Call once the new build is up (its window is showing). It makes the update permanent and
/// deletes the old files; until then a rollback is possible.
pub fn confirm_started(install_dir: &Path) -> Result<(), UpdateError> {
    let stage = staging_dir(install_dir);
    let Some(mut state) = load_state(&stage) else {
        return Ok(());
    };
    if state.phase != Phase::Trial {
        return Ok(());
    }
    let olds: Vec<String> = state
        .entries
        .iter()
        .map(|e| format!("{}{OLD}", e.path))
        .collect();
    state.leftovers = remove_leftovers(install_dir, &olds);
    drop_staged(&stage);
    if state.leftovers.is_empty() {
        clear_state(&stage);
    } else {
        state.phase = Phase::Cleanup;
        save_state(&stage, &state)?;
    }
    Ok(())
}

/// Undoes the last update while it is still unconfirmed. Start the returned `relaunch` and exit.
pub fn roll_back(install_dir: &Path) -> Result<Applied, UpdateError> {
    let stage = staging_dir(install_dir);
    match load_state(&stage) {
        Some(state) if state.phase == Phase::Trial => roll_back_state(
            install_dir,
            state,
            RollbackReason::Requested,
            relaunch_path(install_dir),
        ),
        _ => Err(UpdateError::NothingStaged),
    }
}
