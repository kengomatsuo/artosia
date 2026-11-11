//! The two shortcuts: target, working folder and icon, saved through `IShellLinkW` and
//! `IPersistFile`, after Velopack's `shortcuts.rs` (MIT). No AppUserModelID is set: Artosia's
//! operator never sets one itself (`rune_notify::register` is not called), and a shortcut
//! carrying an id the program does not use would split its taskbar button from its window.
//!
//! Licence: follows Velopack's `shortcuts.rs` (MIT, (c) 2021 Caelan Sayler, 2024 Velopack Ltd.); the
//! permission notice is in THIRD-PARTY-LICENSES.md.

use std::io;
use std::path::{Path, PathBuf};

use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, IPersistFile, STGM};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::core::{Interface, PCWSTR};

use crate::win::{Com, wide};

/// The longest target a shell link holds, with its null.
const MAX_TARGET: usize = 260;

/// Writes `lnk` (and its folder, which a fresh Start menu may lack) pointing at `target`.
pub fn create(lnk: &Path, target: &Path, work_dir: &Path) -> io::Result<()> {
    if target.as_os_str().len() >= MAX_TARGET || work_dir.as_os_str().len() >= MAX_TARGET {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the install folder path is too long for a shortcut"));
    }
    if let Some(folder) = lnk.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let _com = Com::start()?;
    let (target_w, work_w, lnk_w) = (wide(target.as_os_str()), wide(work_dir.as_os_str()), wide(lnk.as_os_str()));
    // SAFETY: every wide buffer lives past the calls that read it.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL).map_err(io::Error::other)?;
        link.SetPath(PCWSTR(target_w.as_ptr())).map_err(io::Error::other)?;
        link.SetWorkingDirectory(PCWSTR(work_w.as_ptr())).map_err(io::Error::other)?;
        link.SetIconLocation(PCWSTR(target_w.as_ptr()), 0).map_err(io::Error::other)?;
        let file: IPersistFile = link.cast().map_err(io::Error::other)?;
        file.Save(PCWSTR(lnk_w.as_ptr()), true).map_err(io::Error::other)
    }
}

/// Deletes a shortcut. One that is not there is not an error.
pub fn remove(lnk: &Path) -> io::Result<()> {
    match std::fs::remove_file(lnk) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// What a saved shortcut points at: target, working folder and icon file. Read back by tests.
pub fn read(lnk: &Path) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let _com = Com::start()?;
    let lnk_w = wide(lnk.as_os_str());
    let text = |units: &[u16]| PathBuf::from(OsString::from_wide(&units[..units.iter().position(|u| *u == 0).unwrap_or(units.len())]));
    // SAFETY: the buffers are as long as the lengths passed.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL).map_err(io::Error::other)?;
        let file: IPersistFile = link.cast().map_err(io::Error::other)?;
        file.Load(PCWSTR(lnk_w.as_ptr()), STGM(0)).map_err(io::Error::other)?;
        let (mut target, mut work, mut icon) = ([0u16; 1024], [0u16; 1024], [0u16; 1024]);
        let mut index = 0;
        link.GetPath(&mut target, std::ptr::null_mut(), 0).map_err(io::Error::other)?;
        link.GetWorkingDirectory(&mut work).map_err(io::Error::other)?;
        link.GetIconLocation(&mut icon, &mut index).map_err(io::Error::other)?;
        Ok((text(&target), text(&work), text(&icon)))
    }
}
