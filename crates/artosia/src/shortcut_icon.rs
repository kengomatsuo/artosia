//! The picture on Artosia's shortcuts after its icon changed.
//!
//! Windows draws a shortcut from a cache keyed by the icon's file and index, and it goes on
//! drawing what it first saw for `artosia.exe,0` after an update swapped the file under that
//! name. On Windows 11 (build 26200, 2026-11-12) a shortcut to a copy updated from 0.1.14 kept
//! the old icon through SHCNE_UPDATEITEM, SHCNE_ASSOCCHANGED, SHUpdateImage, a new Explorer
//! window and even a new shortcut, while a shortcut that named its icon by a path Windows had
//! never seen drew the new one at once (docs/setup.md, "The icon after an update").
//!
//! So the icon also stands in the install folder as a file named after its own bytes, and the
//! shortcuts the setup made name that file, as does the entry in Windows' list of installed
//! programs. A new icon is a new file name, which Windows has not cached. Chromium keeps a shortcut's icon in a file of its own for the same reason
//! (`profile_shortcut_manager_win.cc`); it reuses the name and flashes the desktop with
//! SHCNE_ASSOCCHANGED, which did nothing here.

use std::path::{Path, PathBuf};

/// The icon the program file carries (`build.rs` puts the same file into it).
const ICON: &[u8] = include_bytes!("../../../assets/brand/artosia.ico");

/// The shortcut the setup writes to the Start menu and the desktop
/// (`crates/artosia-setup/src/places.rs`, `LNK_NAME`; a test holds the two together).
const SHORTCUT: &str = "Artosia.lnk";

/// The variable that moves the setup's folders and its entry under a test root
/// (`crates/artosia-setup/src/places.rs`, `ROOT_ENV`): a program started with it looks at the
/// shortcuts and the entry under that root, never at a person's real ones.
const ROOT_ENV: &str = "ARTOSIA_SETUP_ROOT";

/// FNV-1a: it only names things, nothing is protected by it.
fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// The icon file's name: the same bytes always give the same name, a new icon another.
pub fn icon_name(bytes: &[u8]) -> String {
    format!("artosia-{:016x}.ico", fnv(bytes.iter().copied()))
}

/// The name of the setup's entry in Windows' list of installed programs: `Artosia`, or under a
/// test root the name `Places::under` gives it there.
fn entry_name(root: Option<&Path>) -> String {
    match root {
        Some(root) => format!("Artosia-test-{:08x}", fnv(root.to_string_lossy().to_lowercase().bytes()) as u32),
        None => "Artosia".to_string(),
    }
}

/// Puts the icon into `install` under its name, unless it is there already. An icon file of
/// an earlier version stays: a shortcut a person copied elsewhere may still name it.
pub fn place(install: &Path, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let path = install.join(icon_name(bytes));
    if std::fs::read(&path).is_ok_and(|have| have == bytes) {
        return Ok(path);
    }
    // Written beside it and renamed, so Windows never reads half an icon.
    let part = path.with_extension("ico.part");
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, &path)?;
    Ok(path)
}

/// After a start that went well: on a worker, makes the shortcuts of an installed copy, and
/// its entry in Windows' list of installed programs, show this version's icon. `install` is
/// the folder the program runs from (`update::install_dir`), so a development build, a headless
/// run and the Store edition never come here. `refreshing` is the kept setup rewriting that
/// entry after an update: it names `artosia.exe` again, so the entry is written after it ends.
pub fn refresh(install: &Path, refreshing: Option<std::process::Child>) {
    // Only a copy the setup installed has shortcuts of its own; one unzipped by hand has no kept setup.
    if !install.join(artosia_update::SETUP_FILE).is_file() {
        return;
    }
    let install = install.to_path_buf();
    std::thread::spawn(move || {
        let Ok(icon) = place(&install, ICON) else { return };
        let Ok(program) = std::env::current_exe() else { return };
        let root = std::env::var_os(ROOT_ENV).filter(|root| !root.is_empty()).map(PathBuf::from);
        for shortcut in shortcuts(root.as_deref()) {
            let _ = shell::repoint(&shortcut, &program, &icon);
        }
        if let Some(mut setup) = refreshing {
            let _ = setup.wait();
        }
        let _ = shell::entry_icon(UNINSTALL, &entry_name(root.as_deref()), &install, &icon);
    });
}

/// Where Windows keeps a person's installed programs, under `HKEY_CURRENT_USER`.
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Every shortcut that may start this program: the two the setup wrote, and the copies Windows
/// keeps of shortcuts a person pinned to the taskbar (the folder Chromium's `base_paths_win.cc`
/// reads as `DIR_TASKBAR_PINS`). Under a test root they are the folders `Places::under` names,
/// with `TaskBar` standing for the pins.
fn shortcuts(root: Option<&Path>) -> Vec<PathBuf> {
    let (start_menu, desktop, pins) = match root {
        Some(root) => (Some(root.join("StartMenu").join("Programs")), Some(root.join("Desktop")), Some(root.join("TaskBar"))),
        None => (
            shell::start_menu(),
            shell::desktop(),
            shell::roaming().map(|roaming| roaming.join("Microsoft").join("Internet Explorer").join("Quick Launch").join("User Pinned").join("TaskBar")),
        ),
    };
    let mut found: Vec<PathBuf> = [start_menu, desktop].into_iter().flatten().map(|folder| folder.join(SHORTCUT)).collect();
    if let Some(pins) = pins {
        let pinned = std::fs::read_dir(pins).into_iter().flatten().flatten().map(|entry| entry.path());
        found.extend(pinned.filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("lnk"))));
    }
    found
}

#[cfg(windows)]
mod shell {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, IPersistFile, STGM};
    use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_Programs, FOLDERID_RoamingAppData, IShellLinkW, KF_FLAG_DEFAULT, SHCNE_UPDATEITEM, SHCNF_FLUSHNOWAIT, SHCNF_PATHW, SHChangeNotify, SHGetKnownFolderPath, ShellLink};
    use windows::core::{GUID, Interface, PCWSTR};

    fn known(id: &GUID) -> Option<PathBuf> {
        // SAFETY: the returned string is read once and then freed as the API requires.
        unsafe {
            let text = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
            let folder = PathBuf::from(OsString::from_wide(text.as_wide()));
            CoTaskMemFree(Some(text.0 as *const _));
            Some(folder)
        }
    }

    pub fn start_menu() -> Option<PathBuf> {
        known(&FOLDERID_Programs)
    }

    pub fn desktop() -> Option<PathBuf> {
        known(&FOLDERID_Desktop)
    }

    pub fn roaming() -> Option<PathBuf> {
        known(&FOLDERID_RoamingAppData)
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    fn text(units: &[u16]) -> PathBuf {
        PathBuf::from(OsString::from_wide(&units[..units.iter().position(|unit| *unit == 0).unwrap_or(units.len())]))
    }

    /// Windows compares file names without regard to case.
    fn same(a: &Path, b: &Path) -> bool {
        a.as_os_str().to_string_lossy().eq_ignore_ascii_case(&b.as_os_str().to_string_lossy())
    }

    /// Makes one shortcut name `icon`, when it starts `program` and names another picture.
    /// True when the shortcut was written. A shortcut that is not there, or that starts
    /// something else, is left alone.
    pub fn repoint(shortcut: &Path, program: &Path, icon: &Path) -> std::io::Result<bool> {
        if !shortcut.is_file() {
            return Ok(false);
        }
        let (shortcut_w, icon_w) = (wide(shortcut), wide(icon));
        // SAFETY: every wide buffer outlives the call that reads it, the output buffers are as
        // long as the lengths the calls are given, and COM is left only when this call entered it.
        unsafe {
            let entered = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
            let outcome = (|| -> windows::core::Result<bool> {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL)?;
                let file: IPersistFile = link.cast()?;
                // STGM_READWRITE: the same file is saved again below.
                file.Load(PCWSTR(shortcut_w.as_ptr()), STGM(2))?;
                let (mut target, mut have, mut index) = ([0u16; 1024], [0u16; 1024], 0);
                link.GetPath(&mut target, std::ptr::null_mut(), 0)?;
                if !same(&text(&target), program) {
                    return Ok(false);
                }
                link.GetIconLocation(&mut have, &mut index)?;
                if index == 0 && same(&text(&have), icon) {
                    return Ok(false);
                }
                link.SetIconLocation(PCWSTR(icon_w.as_ptr()), 0)?;
                file.Save(PCWSTR(shortcut_w.as_ptr()), true)?;
                // Explorer redraws the shortcut now (seen on the desktop size of icon, 2026-11-12).
                SHChangeNotify(SHCNE_UPDATEITEM, SHCNF_PATHW | SHCNF_FLUSHNOWAIT, Some(shortcut_w.as_ptr().cast()), None);
                Ok(true)
            })();
            if entered {
                CoUninitialize();
            }
            outcome.map_err(std::io::Error::other)
        }
    }

    /// Reads one text value of a key under `HKEY_CURRENT_USER`. None when it is not there.
    fn value(key: &str, name: &str) -> Option<String> {
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
        let (key_w, name_w): (Vec<u16>, Vec<u16>) = (key.encode_utf16().chain(std::iter::once(0)).collect(), name.encode_utf16().chain(std::iter::once(0)).collect());
        let mut text = [0u16; 1024];
        let mut bytes = (text.len() * 2) as u32;
        // SAFETY: the buffer is as long as the size given, and both names end in a null.
        let code = unsafe { RegGetValueW(HKEY_CURRENT_USER, PCWSTR(key_w.as_ptr()), PCWSTR(name_w.as_ptr()), RRF_RT_REG_SZ, None, Some(text.as_mut_ptr().cast()), Some(&mut bytes)) };
        code.is_ok().then(|| String::from_utf16_lossy(&text[..text.iter().position(|unit| *unit == 0).unwrap_or(text.len())]))
    }

    /// Makes the entry `name` under `parent` show `icon`, when the entry is this install's
    /// (its `InstallLocation` is `install`) and shows another picture. True when it was written.
    pub fn entry_icon(parent: &str, name: &str, install: &Path, icon: &Path) -> std::io::Result<bool> {
        use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegSetKeyValueW};
        let key = format!(r"{parent}\{name}");
        let installed = value(&key, "InstallLocation").unwrap_or_default();
        if installed.is_empty() || !same(Path::new(installed.trim_end_matches('\\')), install) {
            return Ok(false);
        }
        if value(&key, "DisplayIcon").is_some_and(|have| same(Path::new(&have), icon)) {
            return Ok(false);
        }
        let (key_w, icon_w): (Vec<u16>, Vec<u16>) = (key.encode_utf16().chain(std::iter::once(0)).collect(), wide(icon));
        // SAFETY: the names end in a null and the data is the wide string with its null, counted in bytes.
        let code = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, PCWSTR(key_w.as_ptr()), windows::core::w!("DisplayIcon"), REG_SZ.0, Some(icon_w.as_ptr().cast()), (icon_w.len() * 2) as u32) };
        code.ok().map(|()| true).map_err(std::io::Error::other)
    }

    /// What a shortcut starts and which picture it names. Read back by the tests.
    #[cfg(test)]
    pub fn read(shortcut: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
        let shortcut_w = wide(shortcut);
        // SAFETY: as in `repoint`.
        unsafe {
            let entered = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
            let outcome = (|| -> windows::core::Result<(PathBuf, PathBuf)> {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL)?;
                let file: IPersistFile = link.cast()?;
                file.Load(PCWSTR(shortcut_w.as_ptr()), STGM(0))?;
                let (mut target, mut have, mut index) = ([0u16; 1024], [0u16; 1024], 0);
                link.GetPath(&mut target, std::ptr::null_mut(), 0)?;
                link.GetIconLocation(&mut have, &mut index)?;
                Ok((text(&target), text(&have)))
            })();
            if entered {
                CoUninitialize();
            }
            outcome.map_err(std::io::Error::other)
        }
    }

    /// Writes a shortcut as the setup does: the program as target, working folder and picture.
    #[cfg(test)]
    pub fn write(shortcut: &Path, program: &Path) -> std::io::Result<()> {
        let (shortcut_w, program_w) = (wide(shortcut), wide(program));
        let folder_w = wide(program.parent().unwrap_or(program));
        // SAFETY: as in `repoint`.
        unsafe {
            let entered = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).is_ok();
            let outcome = (|| -> windows::core::Result<()> {
                let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_ALL)?;
                link.SetPath(PCWSTR(program_w.as_ptr()))?;
                link.SetWorkingDirectory(PCWSTR(folder_w.as_ptr()))?;
                link.SetIconLocation(PCWSTR(program_w.as_ptr()), 0)?;
                let file: IPersistFile = link.cast()?;
                file.Save(PCWSTR(shortcut_w.as_ptr()), true)
            })();
            if entered {
                CoUninitialize();
            }
            outcome.map_err(std::io::Error::other)
        }
    }
}

/// Other systems have no shortcut files of this kind.
#[cfg(not(windows))]
mod shell {
    use std::path::{Path, PathBuf};

    pub fn start_menu() -> Option<PathBuf> {
        None
    }

    pub fn desktop() -> Option<PathBuf> {
        None
    }

    pub fn roaming() -> Option<PathBuf> {
        None
    }

    pub fn repoint(_: &Path, _: &Path, _: &Path) -> std::io::Result<bool> {
        Ok(false)
    }

    pub fn entry_icon(_: &str, _: &str, _: &Path, _: &Path) -> std::io::Result<bool> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("artosia-shortcut-icon-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
        folder
    }

    #[test]
    fn the_icon_file_is_named_after_its_bytes() {
        let (one, two) = (icon_name(b"one icon"), icon_name(b"another icon"));
        assert_ne!(one, two);
        assert_eq!(one, icon_name(b"one icon"));
        assert!(one.starts_with("artosia-") && one.ends_with(".ico") && one.len() == "artosia-".len() + 16 + ".ico".len(), "{one}");
        // The shipped icon is a real icon file, and its name differs from a made-up one's.
        assert_eq!(&ICON[..4], [0, 0, 1, 0]);
        assert_ne!(icon_name(ICON), one);
    }

    #[test]
    fn the_icon_is_placed_once_and_an_earlier_one_stays() {
        let folder = scratch("place");
        let old = place(&folder, b"old icon").unwrap_or_else(|e| panic!("{e}"));
        let new = place(&folder, b"new icon").unwrap_or_else(|e| panic!("{e}"));
        assert_ne!(old, new);
        assert_eq!(std::fs::read(&new).unwrap_or_default(), b"new icon");
        assert!(old.is_file(), "a shortcut copied elsewhere may still name the earlier icon");
        // A second start writes nothing.
        let written = std::fs::metadata(&new).and_then(|meta| meta.modified()).ok();
        assert_eq!(place(&folder, b"new icon").unwrap_or_else(|e| panic!("{e}")), new);
        assert_eq!(std::fs::metadata(&new).and_then(|meta| meta.modified()).ok(), written);
        assert!(!folder.join(icon_name(b"new icon")).with_extension("ico.part").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// The setup's own names, read from its source so the two cannot drift: the shortcut, the
    /// test root's variable and folders, and the entry's name there (two names the setup really
    /// gave on 2026-11-12, read from the registry).
    #[test]
    fn the_places_are_the_ones_the_setup_uses() {
        let places = include_str!("../../artosia-setup/src/places.rs");
        assert!(places.contains(&format!("pub const LNK_NAME: &str = \"{SHORTCUT}\";")));
        assert!(places.contains(&format!("pub const ROOT_ENV: &str = \"{ROOT_ENV}\";")));
        assert!(places.contains(r#"start_menu: root.join("StartMenu").join("Programs"),"#) && places.contains(r#"desktop: root.join("Desktop"),"#));
        assert_eq!(entry_name(None), "Artosia");
        assert_eq!(entry_name(Some(Path::new(r"F:\Repositories\artosia\target\setup-test-0117\root"))), "Artosia-test-da03ac61");
        assert_eq!(entry_name(Some(Path::new(r"F:\Repositories\artosia\target\setup-root-0116"))), "Artosia-test-c702d0fc");
        let root = Path::new("some-root");
        assert_eq!(shortcuts(Some(root))[..2], [root.join("StartMenu").join("Programs").join(SHORTCUT), root.join("Desktop").join(SHORTCUT)]);
    }

    /// The entry in the list of installed programs: one that is this install's takes the icon
    /// file once, one that belongs to another folder is left alone. A key of the test's own,
    /// outside the real list, made and removed with `reg.exe`.
    #[cfg(windows)]
    #[test]
    fn the_entry_of_this_install_names_the_icon_file() {
        // One key of its own, so removing it leaves nothing behind.
        let parent = format!(r"Software\Artosia-tests-{}", std::process::id());
        let reg = |arguments: &[&str]| std::process::Command::new("reg.exe").args(arguments).output().is_ok_and(|output| output.status.success());
        let key = |name: &str| format!(r"HKCU\{parent}\{name}");
        let (install, icon) = (Path::new(r"C:\made-up\Programs\Artosia"), Path::new(r"C:\made-up\Programs\Artosia\artosia-0123456789abcdef.ico"));
        assert!(reg(&["add", &key("Mine"), "/v", "InstallLocation", "/d", r"C:\made-up\Programs\Artosia", "/f"]));
        assert!(reg(&["add", &key("Mine"), "/v", "DisplayIcon", "/d", r"C:\made-up\Programs\Artosia\artosia.exe", "/f"]));
        assert!(reg(&["add", &key("Theirs"), "/v", "InstallLocation", "/d", r"C:\another\folder", "/f"]));
        assert!(shell::entry_icon(&parent, "Mine", install, icon).unwrap_or_else(|e| panic!("{e}")));
        assert!(!shell::entry_icon(&parent, "Mine", install, icon).unwrap_or_else(|e| panic!("{e}")), "already right");
        assert!(!shell::entry_icon(&parent, "Theirs", install, icon).unwrap_or_else(|e| panic!("{e}")));
        assert!(!shell::entry_icon(&parent, "Missing", install, icon).unwrap_or_else(|e| panic!("{e}")));
        let shown = std::process::Command::new("reg.exe").args(["query", &key("Mine"), "/v", "DisplayIcon"]).output().map(|output| String::from_utf8_lossy(&output.stdout).into_owned()).unwrap_or_default();
        assert!(shown.contains("artosia-0123456789abcdef.ico"), "{shown}");
        assert!(reg(&["delete", &format!(r"HKCU\{parent}"), "/f"]));
    }

    /// For looking at a real Explorer window (`tools/check-shortcut-icon.ps1`): the folder named
    /// by `ARTOSIA_ICON_TEST` holds `app\artosia.exe` and `links\Artosia.lnk`, and the shortcut
    /// is given the shipped icon exactly as a start of Artosia gives it.
    #[cfg(windows)]
    #[test]
    #[ignore = "works on a folder the check script prepared"]
    fn a_prepared_shortcut_takes_the_shipped_icon() {
        let Some(root) = std::env::var_os("ARTOSIA_ICON_TEST").map(PathBuf::from) else { return };
        let app = root.join("app");
        let icon = place(&app, ICON).unwrap_or_else(|e| panic!("{e}"));
        assert!(shell::repoint(&root.join("links").join(SHORTCUT), &app.join("artosia.exe"), &icon).unwrap_or_else(|e| panic!("{e}")));    }

    /// A real shortcut file: one that starts this program takes the new picture once, one that
    /// starts another program is left alone, and one that is not there is no error.
    #[cfg(windows)]
    #[test]
    fn a_shortcut_to_this_program_names_the_icon_file() {
        let folder = scratch("repoint");
        let (program, other) = (folder.join("artosia.exe"), folder.join("other.exe"));
        std::fs::write(&program, b"program").unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&other, b"other").unwrap_or_else(|e| panic!("{e}"));
        let icon = place(&folder, b"new icon").unwrap_or_else(|e| panic!("{e}"));
        let (mine, theirs) = (folder.join("Artosia.lnk"), folder.join("Other.lnk"));
        shell::write(&mine, &program).unwrap_or_else(|e| panic!("{e}"));
        shell::write(&theirs, &other).unwrap_or_else(|e| panic!("{e}"));

        assert!(shell::repoint(&mine, &program, &icon).unwrap_or_else(|e| panic!("{e}")));
        let (target, picture) = shell::read(&mine).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((target.file_name(), picture.file_name()), (program.file_name(), icon.file_name()));
        // Already right: nothing is written the second time.
        assert!(!shell::repoint(&mine, &program, &icon).unwrap_or_else(|e| panic!("{e}")));

        assert!(!shell::repoint(&theirs, &program, &icon).unwrap_or_else(|e| panic!("{e}")));
        assert_eq!(shell::read(&theirs).unwrap_or_else(|e| panic!("{e}")).1.file_name(), other.file_name());
        assert!(!shell::repoint(&folder.join("Missing.lnk"), &program, &icon).unwrap_or_else(|e| panic!("{e}")));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
