//! Small Windows helpers shared by the other modules: wide strings, known folders, the file
//! version of a program, and a COM apartment for the shell link.
//!
//! Licence: `known_folder` follows Velopack's `known_path.rs` (MIT, (c) 2021 Caelan Sayler, 2024
//! Velopack Ltd.); the permission notice is in THIRD-PARTY-LICENSES.md.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoTaskMemFree, CoUninitialize};
use windows::Win32::UI::Shell::{KNOWN_FOLDER_FLAG, SHGetKnownFolderPath};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, HICON, IMAGE_ICON, LR_SHARED, LoadImageW, SM_CXICON, SM_CXSMICON};
use windows::core::{GUID, PCWSTR};

/// A null-ended UTF-16 copy. Paths always go through this and never through an ANSI call, so a
/// user name with accents or another script works.
pub fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

/// A shell folder by its id, as Windows says it is now: a Desktop moved into OneDrive is found.
/// After Velopack's `known_path.rs` (MIT).
pub fn known_folder(id: &GUID) -> io::Result<PathBuf> {
    // SAFETY: the returned buffer is read once and freed with CoTaskMemFree, as the API requires.
    unsafe {
        let text = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).map_err(io::Error::other)?;
        let path = PathBuf::from(OsString::from_wide(text.as_wide()));
        CoTaskMemFree(Some(text.0 as *const _));
        Ok(path)
    }
}

/// `major.minor.patch` from a program's version resource.
pub fn file_version(path: &Path) -> Option<String> {
    let name = wide(path.as_os_str());
    // SAFETY: the buffer is as long as the size the API asked for, and VerQueryValueW points
    // into it for as long as it lives.
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(name.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buffer = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(name.as_ptr()), None, size, buffer.as_mut_ptr().cast()).ok()?;
        let mut info: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut length = 0u32;
        let root = wide(OsStr::new("\\"));
        if !VerQueryValueW(buffer.as_ptr().cast(), PCWSTR(root.as_ptr()), &mut info, &mut length).as_bool()
            || (length as usize) < size_of::<VS_FIXEDFILEINFO>()
        {
            return None;
        }
        let fixed = std::ptr::read_unaligned(info.cast::<VS_FIXEDFILEINFO>());
        Some(format!("{}.{}.{}", fixed.dwFileVersionMS >> 16, fixed.dwFileVersionMS & 0xffff, fixed.dwFileVersionLS >> 16))
    }
}

/// Holds COM open on this thread for the shell link; the shell needs an apartment.
pub struct Com;

impl Com {
    pub fn start() -> io::Result<Com> {
        // SAFETY: balanced by CoUninitialize in Drop, on the same thread.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE).ok().map_err(io::Error::other)? };
        Ok(Com)
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        // SAFETY: pairs with the successful CoInitializeEx in `start`.
        unsafe { CoUninitialize() };
    }
}

/// Artosia's mark from this exe's own icon resource (id 1, put there by `build.rs`), at the
/// size the title bar (small) or the dialog and taskbar (big) asks for. A shared handle: it is
/// never destroyed. A null handle, if the resource is missing, simply shows no icon.
pub fn app_icon(big: bool) -> HICON {
    let size = unsafe { GetSystemMetrics(if big { SM_CXICON } else { SM_CXSMICON }) };
    // SAFETY: resource id 1 is passed as MAKEINTRESOURCE; LR_SHARED handles are owned by the system.
    unsafe {
        let module = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
        LoadImageW(Some(module.into()), PCWSTR(std::ptr::without_provenance(1)), IMAGE_ICON, size, size, LR_SHARED).map_or(HICON::default(), |handle| HICON(handle.0))
    }
}
