//! The cameras plugged in, listed without GStreamer. Starting GStreamer's device monitor costs
//! about 50 MB of memory for every run, camera or not (measured: 141 MB without it, 190 MB
//! with), so the operator's list comes from the system and GStreamer starts only when a camera
//! is asked for. On Windows the names are the PnP friendly names, which are the same names
//! GStreamer's Media Foundation provider gives. Other systems list none until a camera is used.

/// The camera names now, each once.
pub fn list() -> Vec<String> {
    sys::list()
}

#[cfg(windows)]
mod sys {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVINFO_DATA, SPDRP_FRIENDLYNAME, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceRegistryPropertyW,
    };
    use windows_sys::core::GUID;

    /// `KSCATEGORY_VIDEO_CAMERA`.
    const CAMERA: GUID = GUID::from_u128(0xe5323777_f976_4f5b_9b55_b94699c46e44);

    pub fn list() -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        // SAFETY: SetupAPI calls on a device list that is created and destroyed here.
        unsafe {
            let set = SetupDiGetClassDevsW(&CAMERA, null(), null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE);
            if set as isize == -1 {
                return names;
            }
            let mut index = 0;
            loop {
                let mut data: SP_DEVINFO_DATA = std::mem::zeroed();
                data.cbSize = size_of::<SP_DEVINFO_DATA>() as u32;
                if SetupDiEnumDeviceInfo(set, index, &mut data) == 0 {
                    break;
                }
                index += 1;
                let mut buffer = [0u16; 256];
                let ok = SetupDiGetDeviceRegistryPropertyW(set, &data, SPDRP_FRIENDLYNAME, null_mut(), buffer.as_mut_ptr().cast::<u8>(), (buffer.len() * 2) as u32, null_mut());
                if ok != 0 {
                    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
                    let name = String::from_utf16_lossy(&buffer[..end]);
                    if !name.is_empty() && !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
            SetupDiDestroyDeviceInfoList(set);
        }
        names
    }
}

#[cfg(not(windows))]
mod sys {
    pub fn list() -> Vec<String> {
        Vec::new()
    }
}
