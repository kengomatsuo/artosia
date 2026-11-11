//! The uninstall entry in Settings > Installed apps. Which values, after Velopack's
//! `registry.rs`, Squirrel's `InstallHelpers.cs` and electron-builder's `installer.nsh` (all MIT).
//!
//! Licence: the value set follows Velopack (MIT, (c) 2021 Caelan Sayler, 2024 Velopack Ltd.), Squirrel.Windows
//! (MIT, (c) 2012 GitHub, Inc.) and electron-builder (MIT, (c) 2015 Loopline Systems); the permission
//! notice is in THIRD-PARTY-LICENSES.md.

use crate::places::Places;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Text(String),
    /// Windows reads `EstimatedSize`, `NoModify` and `NoRepair` as a DWORD: a QWORD is ignored.
    Dword(u32),
}

/// The entry's values. `version` and `size_bytes` are those of the release just installed.
pub fn uninstall_values(places: &Places, version: &str, size_bytes: u64) -> Vec<(&'static str, Value)> {
    let setup = places.setup_exe();
    let text = Value::Text;
    vec![
        ("DisplayName", text("Artosia".to_string())),
        ("DisplayVersion", text(version.to_string())),
        ("Publisher", text("Artosia".to_string())),
        ("DisplayIcon", text(places.app_exe().display().to_string())),
        ("InstallLocation", text(places.install.display().to_string())),
        ("UninstallString", text(format!("\"{}\" --uninstall", setup.display()))),
        ("QuietUninstallString", text(format!("\"{}\" --uninstall --silent", setup.display()))),
        ("NoModify", Value::Dword(1)),
        ("NoRepair", Value::Dword(1)),
        // In kilobytes, saturating: a u32 of KB covers four terabytes.
        ("EstimatedSize", Value::Dword(u32::try_from(size_bytes / 1024).unwrap_or(u32::MAX))),
    ]
}

#[cfg(windows)]
pub use registry::{read, remove, write};

#[cfg(windows)]
mod registry {
    use super::{Places, Value};
    use crate::win::wide;
    use std::ffi::OsStr;
    use std::io;
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE, RRF_RT_REG_DWORD,
        RRF_RT_REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
    };
    use windows::core::PCWSTR;

    fn check(code: WIN32_ERROR) -> io::Result<()> {
        if code == ERROR_SUCCESS { Ok(()) } else { Err(io::Error::from_raw_os_error(code.0 as i32)) }
    }

    /// Creates the key and sets every value.
    pub fn write(places: &Places, values: &[(&str, Value)]) -> io::Result<()> {
        let path = wide(OsStr::new(&places.registry_path()));
        let mut key = HKEY::default();
        // SAFETY: `key` is closed below; every pointer outlives its call.
        unsafe {
            check(RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut key,
                None,
            ))?;
            let mut result = Ok(());
            for (name, value) in values {
                let name = wide(OsStr::new(name));
                let (kind, bytes) = match value {
                    Value::Text(text) => {
                        let units = wide(OsStr::new(text));
                        (REG_SZ, units.iter().flat_map(|unit| unit.to_le_bytes()).collect::<Vec<u8>>())
                    }
                    Value::Dword(number) => (REG_DWORD, number.to_le_bytes().to_vec()),
                };
                result = check(RegSetValueExW(key, PCWSTR(name.as_ptr()), None, kind, Some(&bytes)));
                if result.is_err() {
                    break;
                }
            }
            let _ = RegCloseKey(key);
            result
        }
    }

    /// Deletes the key and everything under it. A key that is not there is not an error.
    pub fn remove(places: &Places) -> io::Result<()> {
        let path = wide(OsStr::new(&places.registry_path()));
        // SAFETY: plain call with a null-ended string.
        match unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr())) } {
            code if code == ERROR_FILE_NOT_FOUND => Ok(()),
            code => check(code),
        }
    }

    /// One value of the key, or None when the key or the value is not there.
    pub fn read(places: &Places, name: &str) -> Option<Value> {
        let path = wide(OsStr::new(&places.registry_path()));
        let name = wide(OsStr::new(name));
        let mut key = HKEY::default();
        // SAFETY: `key` is closed before returning; the buffer length is passed and re-read.
        unsafe {
            check(RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), None, KEY_READ, &mut key)).ok()?;
            let mut buffer = vec![0u8; 4096];
            let mut length = buffer.len() as u32;
            let mut kind = REG_VALUE_TYPE::default();
            let code = RegGetValueW(
                key,
                PCWSTR::null(),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ | RRF_RT_REG_DWORD,
                Some(&mut kind),
                Some(buffer.as_mut_ptr().cast()),
                Some(&mut length),
            );
            let _ = RegCloseKey(key);
            check(code).ok()?;
            buffer.truncate(length as usize);
            if kind == REG_DWORD {
                return Some(Value::Dword(u32::from_le_bytes(buffer.get(..4)?.try_into().ok()?)));
            }
            let units: Vec<u16> = buffer.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).take_while(|unit| *unit != 0).collect();
            Some(Value::Text(String::from_utf16_lossy(&units)))
        }
    }
}
