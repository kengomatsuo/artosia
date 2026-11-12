//! The Microsoft Store edition: the same programs, started from an MSIX package. One build serves
//! both editions, so the program asks at run time whether it has a package identity
//! (`docs/store.md`). Packaged, the Store updates it, the manifest declares its firewall rule and
//! its library lives in Documents, because Windows deletes a packaged program's AppData on
//! uninstall.

use std::path::PathBuf;
use std::sync::OnceLock;

/// Whether this process runs from a package. Cached: the answer cannot change while it runs.
/// `ARTOSIA_EDITION=store|direct` overrides it, for tests and for trying one edition by hand.
pub fn packaged() -> bool {
    static ANSWER: OnceLock<bool> = OnceLock::new();
    *ANSWER.get_or_init(|| edition(std::env::var("ARTOSIA_EDITION").ok().as_deref(), detect))
}

/// The override when it names an edition, else what `detect` finds.
fn edition(over: Option<&str>, detect: impl FnOnce() -> bool) -> bool {
    match over.map(str::trim) {
        Some(word) if word.eq_ignore_ascii_case("store") => true,
        Some(word) if word.eq_ignore_ascii_case("direct") => false,
        _ => detect(),
    }
}

/// `GetCurrentPackageFullName` answers ERROR_INSUFFICIENT_BUFFER for a zero-length buffer when the
/// process has an identity, and APPMODEL_ERROR_NO_PACKAGE when it has none.
#[cfg(windows)]
fn detect() -> bool {
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    let mut length = 0u32;
    // SAFETY: a null buffer with length 0 is the documented way to ask for the needed length.
    unsafe { GetCurrentPackageFullName(&mut length, None) == ERROR_INSUFFICIENT_BUFFER }
}

#[cfg(not(windows))]
fn detect() -> bool {
    false
}

/// The Documents known folder, which Windows does not redirect for a packaged program.
#[cfg(windows)]
pub fn documents_folder() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    // SAFETY: the returned string is read once and then freed as the API requires.
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_Documents, KF_FLAG_DEFAULT, None).ok()?;
        let folder = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0 as *const _));
        folder
    }
}

#[cfg(not(windows))]
pub fn documents_folder() -> Option<PathBuf> {
    None
}

/// The package version of a program version: Microsoft forbids a first part of 0 and reserves the
/// fourth, so `a.b.c` becomes `(a+1).b.c.0`. None for anything that is not three numbers.
/// `tools/store-package.sh` computes the same in shell, and the `--edition` report prints this
/// one so the two can be compared.
pub fn package_version(program: &str) -> Option<String> {
    let parts: Vec<u16> = program.split('.').map(|part| part.parse().ok()).collect::<Option<_>>()?;
    let [major, minor, patch] = parts[..] else { return None };
    Some(format!("{}.{minor}.{patch}.0", major.checked_add(1)?))
}

/// `artosia.exe --edition [file]`: says which edition this is, which library folder it would use,
/// and whether that folder would be filled from a direct download. Opens no window and no library,
/// so it can run inside a package on a machine that has a real one. The release exe has no
/// console, so a file may be named. True when the flag was given.
pub fn report_if_asked() -> bool {
    if !crate::data::has_flag("--edition") {
        return false;
    }
    let library = crate::data::library();
    let line = |label: &str, value: String| format!("{label}={value}\n");
    let mut text = line("edition", if packaged() { "store" } else { "direct" }.into());
    text += &line("version", package_version(crate::update::VERSION).unwrap_or_default());
    text += &line("exe", std::env::current_exe().map(|exe| exe.display().to_string()).unwrap_or_default());
    text += &line("data", library.as_ref().map(|library| library.folder.display().to_string()).unwrap_or_else(|| "none".into()));
    text += &line("copy_from", library.and_then(|library| library.copy_from).map(|from| from.display().to_string()).unwrap_or_else(|| "none".into()));
    match crate::data::flag_value("--edition").filter(|value| !value.starts_with("--")) {
        Some(file) => {
            let _ = std::fs::write(file, &text);
        }
        None => print!("{text}"),
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_override_wins_over_detection() {
        assert!(edition(Some("store"), || false));
        assert!(edition(Some(" STORE "), || false));
        assert!(!edition(Some("direct"), || true));
    }

    #[test]
    fn an_unknown_or_missing_override_asks_the_system() {
        assert!(edition(None, || true));
        assert!(!edition(None, || false));
        assert!(edition(Some("beta"), || true));
        assert!(!edition(Some(""), || false));
    }

    #[test]
    fn the_package_version_never_starts_with_zero_and_ends_with_zero() {
        assert_eq!(package_version("0.1.16").as_deref(), Some("1.1.16.0"));
        assert_eq!(package_version("2.0.0").as_deref(), Some("3.0.0.0"));
        assert_eq!(package_version("0.1.15-rc1"), None);
        assert_eq!(package_version("0.1"), None);
        assert_eq!(package_version("0.1.2.3"), None);
        assert_eq!(package_version("65535.0.0"), None);
    }

    #[test]
    fn this_build_is_not_packaged() {
        // Tests run from target/, which has no package identity (ARTOSIA_EDITION unset in CI).
        if std::env::var_os("ARTOSIA_EDITION").is_none() {
            assert!(!packaged());
        }
    }
}
