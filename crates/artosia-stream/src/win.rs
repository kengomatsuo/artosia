//! The few Windows calls the program needs: list windows and monitors, and catch Ctrl+C.
//!
//! Declared by hand so that no extra crate is pulled in. On other systems the lists are empty
//! and the capture elements are not available, so the program says so when it starts.

use std::sync::atomic::{AtomicBool, Ordering};

/// A top-level window that could be captured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub handle: u64,
    pub title: String,
    pub pid: u32,
    pub minimized: bool,
}

/// A monitor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    pub index: u32,
    pub handle: u64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
}

/// Set when Ctrl+C, Ctrl+Break or a console close arrives.
pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
/// The console event that set it, for the log.
pub static LAST_EVENT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// Picks the window whose title is exactly `wanted`, else the first whose title contains it
/// (ignoring case). Minimised windows come last: they give no frames.
pub fn pick_window<'a>(
    windows: &'a [WindowInfo],
    wanted: &str,
    pid: Option<u32>,
) -> Option<&'a WindowInfo> {
    let lower = wanted.to_lowercase();
    let best = |pred: &dyn Fn(&WindowInfo) -> bool| {
        windows
            .iter()
            .filter(|w| pid.is_none_or(|p| w.pid == p))
            .filter(|w| pred(w))
            .min_by_key(|w| w.minimized)
    };
    best(&|w| w.title == wanted).or_else(|| best(&|w| w.title.to_lowercase().contains(&lower)))
}

#[cfg(windows)]
mod imp {
    use super::{INTERRUPTED, LAST_EVENT, MonitorInfo, WindowInfo};
    use std::ffi::c_void;
    use std::sync::atomic::Ordering;

    type Bool = i32;
    type Hwnd = *mut c_void;
    type Lparam = isize;

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct MonitorInfoRaw {
        size: u32,
        monitor: Rect,
        work: Rect,
        flags: u32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumWindows(cb: unsafe extern "system" fn(Hwnd, Lparam) -> Bool, l: Lparam) -> Bool;
        fn IsWindowVisible(h: Hwnd) -> Bool;
        fn IsIconic(h: Hwnd) -> Bool;
        fn IsWindow(h: Hwnd) -> Bool;
        fn GetWindowTextLengthW(h: Hwnd) -> i32;
        fn GetWindowTextW(h: Hwnd, buf: *mut u16, max: i32) -> i32;
        fn GetWindowThreadProcessId(h: Hwnd, pid: *mut u32) -> u32;
        fn EnumDisplayMonitors(
            dc: *mut c_void,
            clip: *const Rect,
            cb: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut Rect, Lparam) -> Bool,
            l: Lparam,
        ) -> Bool;
        fn GetMonitorInfoW(m: *mut c_void, info: *mut MonitorInfoRaw) -> Bool;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> Bool>,
            add: Bool,
        ) -> Bool;
    }

    unsafe extern "system" fn window_cb(h: Hwnd, l: Lparam) -> Bool {
        // SAFETY: `l` is the address of the Vec passed by `windows()`, alive for the call.
        let out = unsafe { &mut *(l as *mut Vec<WindowInfo>) };
        // SAFETY: plain queries on a handle EnumWindows gave us.
        unsafe {
            if IsWindowVisible(h) == 0 {
                return 1;
            }
            let len = GetWindowTextLengthW(h);
            if len <= 0 {
                return 1;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let got = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
            if got <= 0 {
                return 1;
            }
            let title = String::from_utf16_lossy(&buf[..got as usize]);
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            out.push(WindowInfo {
                handle: h as usize as u64,
                title,
                pid,
                minimized: IsIconic(h) != 0,
            });
        }
        1
    }

    pub fn windows() -> Vec<WindowInfo> {
        let mut out: Vec<WindowInfo> = Vec::new();
        // SAFETY: the callback only runs during this call and `out` outlives it.
        unsafe {
            EnumWindows(window_cb, &mut out as *mut Vec<WindowInfo> as Lparam);
        }
        out
    }

    pub fn window_exists(handle: u64) -> bool {
        // SAFETY: IsWindow accepts any value and only reports whether it is a live window.
        unsafe { IsWindow(handle as usize as Hwnd) != 0 }
    }

    unsafe extern "system" fn monitor_cb(
        m: *mut c_void,
        _dc: *mut c_void,
        _r: *mut Rect,
        l: Lparam,
    ) -> Bool {
        // SAFETY: `l` is the address of the Vec passed by `monitors()`, alive for the call.
        let out = unsafe { &mut *(l as *mut Vec<MonitorInfo>) };
        let mut info = MonitorInfoRaw {
            size: std::mem::size_of::<MonitorInfoRaw>() as u32,
            monitor: Rect::default(),
            work: Rect::default(),
            flags: 0,
        };
        // SAFETY: `info` is a valid MONITORINFO with its size set.
        if unsafe { GetMonitorInfoW(m, &mut info) } != 0 {
            out.push(MonitorInfo {
                index: out.len() as u32,
                handle: m as usize as u64,
                x: info.monitor.left,
                y: info.monitor.top,
                width: info.monitor.right - info.monitor.left,
                height: info.monitor.bottom - info.monitor.top,
                primary: info.flags & 1 != 0,
            });
        }
        1
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        let mut out: Vec<MonitorInfo> = Vec::new();
        // SAFETY: the callback only runs during this call and `out` outlives it.
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                monitor_cb,
                &mut out as *mut Vec<MonitorInfo> as Lparam,
            );
        }
        out
    }

    unsafe extern "system" fn ctrl_cb(kind: u32) -> Bool {
        // 0 Ctrl+C, 1 Ctrl+Break, 2 console closed, 5 logoff, 6 shutdown.
        if matches!(kind, 0 | 1 | 2 | 5 | 6) {
            LAST_EVENT.store(kind, Ordering::SeqCst);
            INTERRUPTED.store(true, Ordering::SeqCst);
            return 1;
        }
        0
    }

    pub fn install_ctrl_handler() {
        // SAFETY: registers a handler that only stores to an atomic.
        unsafe {
            SetConsoleCtrlHandler(Some(ctrl_cb), 1);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{MonitorInfo, WindowInfo};

    pub fn windows() -> Vec<WindowInfo> {
        Vec::new()
    }
    pub fn window_exists(_handle: u64) -> bool {
        false
    }
    pub fn monitors() -> Vec<MonitorInfo> {
        Vec::new()
    }
    pub fn install_ctrl_handler() {}
}

/// The visible titled windows.
pub fn windows() -> Vec<WindowInfo> {
    imp::windows()
}

/// Whether the handle still names a window.
pub fn window_exists(handle: u64) -> bool {
    imp::window_exists(handle)
}

/// The monitors, numbered in the order Windows lists them.
pub fn monitors() -> Vec<MonitorInfo> {
    imp::monitors()
}

/// Makes Ctrl+C set [`INTERRUPTED`] instead of killing the process.
pub fn install_ctrl_handler() {
    imp::install_ctrl_handler();
}

/// Whether Ctrl+C has been pressed.
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(title: &str, minimized: bool) -> WindowInfo {
        WindowInfo {
            handle: 1,
            title: title.into(),
            pid: 1,
            minimized,
        }
    }

    #[test]
    fn exact_title_wins_over_a_longer_one() {
        let list = vec![
            w("Artosia projector - old", false),
            w("Artosia projector", false),
        ];
        assert_eq!(
            pick_window(&list, "Artosia projector", None).map(|x| x.title.as_str()),
            Some("Artosia projector")
        );
    }

    #[test]
    fn contains_is_the_fallback_and_ignores_case() {
        let list = vec![w("Notes", false), w("My ARTOSIA Projector", false)];
        assert!(pick_window(&list, "artosia projector", None).is_some());
        assert!(pick_window(&list, "missing", None).is_none());
    }

    #[test]
    fn a_process_id_picks_between_two_copies() {
        let mut a = w("Artosia projector", false);
        a.pid = 10;
        let mut b = w("Artosia projector", false);
        b.pid = 20;
        let list = vec![a, b];
        assert_eq!(
            pick_window(&list, "Artosia projector", Some(20)).map(|x| x.pid),
            Some(20)
        );
        assert!(pick_window(&list, "Artosia projector", Some(30)).is_none());
    }

    #[test]
    fn a_minimised_copy_loses_to_a_visible_one() {
        let mut a = w("Artosia projector", true);
        a.handle = 1;
        let mut b = w("Artosia projector", false);
        b.handle = 2;
        let list = vec![a, b];
        assert_eq!(
            pick_window(&list, "Artosia projector", None).map(|x| x.handle),
            Some(2)
        );
    }
}
