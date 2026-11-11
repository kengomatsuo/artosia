//! Where everything goes. One value holds the install folder, both shortcut folders, the
//! registry key and the folders that are the church's own. A test root moves all of them.

use std::path::{Path, PathBuf};

pub const APP_EXE: &str = "artosia.exe";
pub use artosia_update::SETUP_FILE as SETUP_EXE;
pub const LNK_NAME: &str = "Artosia.lnk";
/// The variable that moves every folder and the registry key under a test root.
pub const ROOT_ENV: &str = "ARTOSIA_SETUP_ROOT";
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

#[derive(Clone, Debug)]
pub struct Places {
    pub install: PathBuf,
    /// The folder the Start menu shortcut goes in (`Programs`).
    pub start_menu: PathBuf,
    pub desktop: PathBuf,
    /// The last part of the uninstall key: `Artosia`, or `Artosia-test-<n>` under a test root.
    pub key_name: String,
    /// Folders that hold the church's library, settings and recordings. Uninstall never enters them.
    pub data: Vec<PathBuf>,
    pub test_root: bool,
}

impl Places {
    /// Everything under `root`, with a registry key named after it.
    pub fn under(root: &Path) -> Places {
        // FNV-1a over the lower-cased path: the same root always gives the same key, so a later
        // `--uninstall` process finds what the install wrote.
        let hash = root
            .to_string_lossy()
            .to_lowercase()
            .bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
        Places {
            install: root.join("Programs").join("Artosia"),
            start_menu: root.join("StartMenu").join("Programs"),
            desktop: root.join("Desktop"),
            key_name: format!("Artosia-test-{:08x}", hash as u32),
            data: vec![root.join("Data")],
            test_root: true,
        }
    }

    /// The test root when `ARTOSIA_SETUP_ROOT` is set, else the user's real folders.
    #[cfg(windows)]
    pub fn resolve() -> std::io::Result<Places> {
        match std::env::var_os(ROOT_ENV).filter(|root| !root.is_empty()) {
            Some(root) => Ok(Places::under(Path::new(&root))),
            None => Places::real(),
        }
    }

    #[cfg(windows)]
    fn real() -> std::io::Result<Places> {
        use windows::Win32::UI::Shell::{FOLDERID_Desktop, FOLDERID_LocalAppData, FOLDERID_Programs, FOLDERID_RoamingAppData};
        let mut data = vec![crate::win::known_folder(&FOLDERID_RoamingAppData)?.join("artosia")];
        // The app reads APPDATA and ARTOSIA_DATA itself (`rune::dirs`, `data::folder`).
        data.extend(std::env::var_os("APPDATA").map(|folder| PathBuf::from(folder).join("artosia")));
        data.extend(std::env::var_os("ARTOSIA_DATA").map(PathBuf::from));
        Ok(Places {
            // `Programs\Artosia`, electron-builder's per-user place. Not `Artosia` beside it: rune keeps
            // its state and log folders in `%LOCALAPPDATA%rtosia`, the same folder on Windows, and
            // an uninstall empties the install folder.
            install: crate::win::known_folder(&FOLDERID_LocalAppData)?.join("Programs").join("Artosia"),
            start_menu: crate::win::known_folder(&FOLDERID_Programs)?,
            desktop: crate::win::known_folder(&FOLDERID_Desktop)?,
            key_name: "Artosia".to_string(),
            data,
            test_root: false,
        })
    }

    pub fn app_exe(&self) -> PathBuf {
        self.install.join(APP_EXE)
    }

    pub fn setup_exe(&self) -> PathBuf {
        self.install.join(SETUP_EXE)
    }

    pub fn start_lnk(&self) -> PathBuf {
        self.start_menu.join(LNK_NAME)
    }

    pub fn desktop_lnk(&self) -> PathBuf {
        self.desktop.join(LNK_NAME)
    }

    /// The key under `HKEY_CURRENT_USER`.
    pub fn registry_path(&self) -> String {
        format!(r"{UNINSTALL}\{}", self.key_name)
    }
}
