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
/// on one volume.
pub fn staging_dir(install_dir: &Path) -> PathBuf {
    install_dir.join("update")
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

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
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
/// (Windows reports a renamed running exe under its new name).
fn relaunch_path(install_dir: &Path) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let same_dir = fs::canonicalize(exe.parent()?).ok()? == fs::canonicalize(install_dir).ok()?;
    same_dir.then(|| exe.file_name().map(|n| install_dir.join(n)))?
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
    if !stage.join(SIGNATURE_NAME).is_file() {
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
            let stale = with_suffix(&install_dir.join(&e.path), suffix);
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
        fs::rename(&dest, with_suffix(&dest, OLD))?;
    }
    fs::rename(staged, &dest)
}

/// Puts every journaled file back as it was. Returns the new files it could not delete.
fn restore(install_dir: &Path, entries: &[Entry]) -> Result<Vec<String>, UpdateError> {
    let mut leftovers = Vec::new();
    for e in entries.iter().rev() {
        let dest = install_dir.join(&e.path);
        let old = with_suffix(&dest, OLD);
        let fail = |err: io::Error| UpdateError::RestoreFailed(format!("{}: {err}", e.path));
        if old.exists() {
            if dest.exists() {
                // The new file may be a running exe, so move it aside rather than delete it.
                let aside = with_suffix(&dest, NEW);
                let _ = fs::remove_file(&aside);
                fs::rename(&dest, &aside).map_err(fail)?;
                leftovers.push(format!("{}{NEW}", e.path));
            }
            fs::rename(&old, &dest).map_err(fail)?;
        } else if !e.had_old && dest.exists() && fs::remove_file(&dest).is_err() {
            let aside = with_suffix(&dest, NEW);
            fs::rename(&dest, &aside).map_err(fail)?;
            leftovers.push(format!("{}{NEW}", e.path));
        }
    }
    Ok(leftovers)
}

/// Deletes what it can; returns what is still locked.
fn remove_leftovers(install_dir: &Path, list: &[String]) -> Vec<String> {
    list.iter()
        .filter(|p| {
            let path = install_dir.join(p);
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
