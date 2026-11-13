//! What the program asks of the system it runs on: the windows, the monitors, and Ctrl+C.
//!
//! Windows' calls are declared by hand so that no extra crate is pulled in. Linux asks X11
//! through `x11rb` and a Mac asks Core Graphics, both crates the window library already
//! brings into the build.

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

/// Set when Ctrl+C, Ctrl+Break, a console close or (off Windows) a kill arrives.
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

/// X11: the windows under the root and one step inside them (a window manager wraps each in a
/// frame of its own), and the monitors RandR knows. Under Wayland this sees the X11 programs
/// only, which is what Artosia's own windows are there.
#[cfg(target_os = "linux")]
mod imp {
    use super::{MonitorInfo, WindowInfo};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use x11rb::connection::Connection;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt as _, MapState, Window};
    use x11rb::rust_connection::RustConnection;

    fn atom(conn: &RustConnection, name: &str) -> Option<u32> {
        conn.intern_atom(false, name.as_bytes()).ok()?.reply().ok().map(|reply| reply.atom)
    }

    fn text(conn: &RustConnection, window: Window, property: u32) -> Option<String> {
        let reply = conn.get_property(false, window, property, AtomEnum::ANY, 0, 1024).ok()?.reply().ok()?;
        (!reply.value.is_empty()).then(|| String::from_utf8_lossy(&reply.value).into_owned())
    }

    fn children(conn: &RustConnection, window: Window) -> Vec<Window> {
        conn.query_tree(window).ok().and_then(|cookie| cookie.reply().ok()).map(|reply| reply.children).unwrap_or_default()
    }

    /// One question to X11, asked on a thread of its own and given two seconds. A display that
    /// takes the connection and never answers (WSL's own does, with nobody looking at it) must
    /// not hold the stream or its stopping; after one silence X11 is left alone for half a minute.
    fn ask<T: Send + 'static>(empty: T, question: impl FnOnce() -> T + Send + 'static) -> T {
        static SILENT_SINCE: Mutex<Option<Instant>> = Mutex::new(None);
        let silent = |since: &Option<Instant>| since.is_some_and(|at| at.elapsed() < Duration::from_secs(30));
        if SILENT_SINCE.lock().is_ok_and(|since| silent(&since)) {
            return empty;
        }
        let (answer, wait) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = answer.send(question());
        });
        match wait.recv_timeout(Duration::from_secs(2)) {
            Ok(answer) => answer,
            Err(_) => {
                if let Ok(mut since) = SILENT_SINCE.lock() {
                    *since = Some(Instant::now());
                }
                empty
            }
        }
    }

    pub fn windows() -> Vec<WindowInfo> {
        ask(Vec::new(), windows_now)
    }

    pub fn window_exists(handle: u64) -> bool {
        ask(false, move || window_exists_now(handle))
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        ask(Vec::new(), monitors_now)
    }

    fn windows_now() -> Vec<WindowInfo> {
        let Ok((conn, screen)) = x11rb::connect(None) else { return Vec::new() };
        let root = conn.setup().roots[screen].root;
        let (name, pid) = (atom(&conn, "_NET_WM_NAME"), atom(&conn, "_NET_WM_PID"));
        let mut look = children(&conn, root);
        let inside: Vec<Window> = look.iter().flat_map(|window| children(&conn, *window)).collect();
        look.extend(inside);
        look.into_iter()
            .filter_map(|window| {
                let title = name.and_then(|name| text(&conn, window, name)).or_else(|| text(&conn, window, AtomEnum::WM_NAME.into()))?;
                let shown = conn.get_window_attributes(window).ok().and_then(|cookie| cookie.reply().ok()).is_some_and(|a| a.map_state == MapState::VIEWABLE);
                let owner = pid
                    .and_then(|pid| conn.get_property(false, window, pid, AtomEnum::CARDINAL, 0, 1).ok()?.reply().ok())
                    .and_then(|reply| reply.value32().and_then(|mut values| values.next()))
                    .unwrap_or(0);
                Some(WindowInfo { handle: u64::from(window), title, pid: owner, minimized: !shown })
            })
            .collect()
    }

    fn window_exists_now(handle: u64) -> bool {
        let Ok((conn, _)) = x11rb::connect(None) else { return false };
        conn.get_window_attributes(handle as Window).ok().and_then(|cookie| cookie.reply().ok()).is_some()
    }

    fn monitors_now() -> Vec<MonitorInfo> {
        let Ok((conn, screen)) = x11rb::connect(None) else { return Vec::new() };
        let root = &conn.setup().roots[screen];
        let known = conn.randr_get_monitors(root.root, true).ok().and_then(|cookie| cookie.reply().ok()).map(|reply| reply.monitors).unwrap_or_default();
        if known.is_empty() {
            // No RandR (a bare virtual screen): the whole screen is the one monitor.
            return vec![MonitorInfo { index: 1, handle: 1, x: 0, y: 0, width: i32::from(root.width_in_pixels), height: i32::from(root.height_in_pixels), primary: true }];
        }
        known
            .iter()
            .enumerate()
            .map(|(i, m)| MonitorInfo {
                index: i as u32 + 1,
                handle: i as u64 + 1,
                x: i32::from(m.x),
                y: i32::from(m.y),
                width: i32::from(m.width),
                height: i32::from(m.height),
                primary: m.primary,
            })
            .collect()
    }

    pub fn install_ctrl_handler() {
        super::signals::install();
    }
}

/// A Mac: the windows and screens Core Graphics lists. A window's title is told only to a
/// program the person allowed to record the screen; until then a window goes by the name of
/// the program that owns it. The screens are numbered from 0 with the main one first, as
/// GStreamer's screen capture counts them (`avfvideosrc.m`: its index goes into
/// `[NSScreen screens]`), which is the same order for one or two screens.
#[cfg(target_os = "macos")]
mod imp {
    use super::{MonitorInfo, WindowInfo};
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{CGDisplay, CGRect};
    use core_graphics::window::{copy_window_info, kCGNullWindowID, kCGWindowBounds, kCGWindowListOptionOnScreenOnly, kCGWindowName, kCGWindowNumber, kCGWindowOwnerName, kCGWindowOwnerPID};

    struct Seen {
        info: WindowInfo,
        bounds: Option<CGRect>,
    }

    fn seen() -> Vec<Seen> {
        let Some(list) = copy_window_info(kCGWindowListOptionOnScreenOnly, kCGNullWindowID) else { return Vec::new() };
        list.iter()
            .filter_map(|item| {
                // SAFETY: Core Graphics documents each item of this list as a dictionary.
                let window: CFDictionary<CFString, CFType> = unsafe { CFDictionary::wrap_under_get_rule(*item as _) };
                // SAFETY: the keys are constants of Core Graphics that live as long as the program.
                let key = |key| unsafe { CFString::wrap_under_get_rule(key) };
                let number = |name| window.find(key(name)).and_then(|value| value.downcast::<CFNumber>()).and_then(|n| n.to_i64());
                let words = |name| window.find(key(name)).and_then(|value| value.downcast::<CFString>()).map(|s| s.to_string());
                let handle = number(unsafe { kCGWindowNumber })? as u64;
                let pid = number(unsafe { kCGWindowOwnerPID }).unwrap_or(0) as u32;
                // The title when macOS tells it, else the name of the program that owns the window.
                let title = words(unsafe { kCGWindowName }).filter(|title| !title.is_empty()).or_else(|| words(unsafe { kCGWindowOwnerName })).unwrap_or_default();
                let bounds = window.find(key(unsafe { kCGWindowBounds })).and_then(|value| value.downcast::<CFDictionary>()).and_then(|d| CGRect::from_dict_representation(&d));
                Some(Seen { info: WindowInfo { handle, title, pid, minimized: false }, bounds })
            })
            .collect()
    }

    pub fn windows() -> Vec<WindowInfo> {
        seen().into_iter().map(|seen| seen.info).collect()
    }

    pub fn window_exists(handle: u64) -> bool {
        seen().iter().any(|seen| seen.info.handle == handle)
    }

    fn displays() -> Vec<(u32, CGRect)> {
        CGDisplay::active_displays().unwrap_or_default().into_iter().map(|id| (id, CGDisplay::new(id).bounds())).collect()
    }

    pub fn monitors() -> Vec<MonitorInfo> {
        let main = CGDisplay::main().id;
        displays()
            .into_iter()
            .enumerate()
            .map(|(i, (id, bounds))| MonitorInfo {
                index: i as u32,
                handle: i as u64,
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: bounds.size.width as i32,
                height: bounds.size.height as i32,
                primary: id == main,
            })
            .collect()
    }

    /// Whether macOS lets this program record the screen. The first time it is asked, macOS
    /// shows its own box naming Artosia and lists it under Screen & System Audio Recording;
    /// after that it only answers (Core Graphics, `CGRequestScreenCaptureAccess`, macOS 10.15).
    pub fn screen_allowed() -> bool {
        let access = core_graphics::access::ScreenCaptureAccess;
        access.preflight() || access.request()
    }

    /// The number of the screen that holds the middle of this window, or the first screen.
    pub fn screen_of_window(handle: u64) -> u64 {
        let Some(bounds) = seen().into_iter().find(|seen| seen.info.handle == handle).and_then(|seen| seen.bounds) else { return 0 };
        let (x, y) = (bounds.origin.x + bounds.size.width / 2.0, bounds.origin.y + bounds.size.height / 2.0);
        displays()
            .iter()
            .position(|(_, screen)| x >= screen.origin.x && x < screen.origin.x + screen.size.width && y >= screen.origin.y && y < screen.origin.y + screen.size.height)
            .unwrap_or(0) as u64
    }

    pub fn install_ctrl_handler() {
        super::signals::install();
    }
}

/// Ctrl+C in a terminal and a plain kill, off Windows: both ask the program to finish its
/// files, as Windows' console events do.
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod signals {
    use super::{INTERRUPTED, LAST_EVENT};
    use std::sync::atomic::Ordering;

    extern "C" fn stop(signal: libc::c_int) {
        LAST_EVENT.store(signal as u32, Ordering::SeqCst);
        INTERRUPTED.store(true, Ordering::SeqCst);
    }

    pub fn install() {
        let handler = stop as extern "C" fn(libc::c_int) as libc::sighandler_t;
        // SAFETY: the handler only stores to atomics, which a signal handler may do.
        unsafe {
            libc::signal(libc::SIGINT, handler);
            libc::signal(libc::SIGTERM, handler);
        }
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
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

/// On a Mac, the screen a window is on, as the screen capture numbers it.
#[cfg(target_os = "macos")]
pub fn screen_of_window(handle: u64) -> u64 {
    imp::screen_of_window(handle)
}

/// Whether the system lets this program take the screen's picture. Only a Mac asks.
pub fn screen_allowed() -> bool {
    #[cfg(target_os = "macos")]
    return imp::screen_allowed();
    #[cfg(not(target_os = "macos"))]
    true
}

/// The visible titled windows.
pub fn windows() -> Vec<WindowInfo> {
    imp::windows()
}

/// Whether the handle still names a window.
pub fn window_exists(handle: u64) -> bool {
    imp::window_exists(handle)
}

/// The monitors, numbered in the order the system lists them.
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
