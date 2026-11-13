//! Which screens exist right now, and which screen each output window belongs on.
//!
//! Rune lists the screens once at start. On Windows this module lists them again whenever the
//! system says the layout changed (`WM_DISPLAYCHANGE`, with a slow poll as a safety net) and
//! publishes the new list; the windows react in `placer.rs`. Other systems keep the start list,
//! because winit 0.30 has no monitor-changed event and Rune does not expose its monitor list
//! after start. What they would need is in `docs/output.md`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use artosia_slides::frame::trace;

/// One screen, in desktop pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Disp {
    /// The system's device name, such as `\\.\DISPLAY2`. It outlives a change of order.
    pub name: String,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub primary: bool,
    /// Physical pixels per point: 1.0 is 100%.
    pub scale: f32,
}

/// What the operator or the command line asked for, by screen name.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pins {
    pub audience: Option<String>,
    pub stage: Option<String>,
    /// The stage view is wanted in a window where it has no screen.
    pub stage_window: bool,
    /// The projector is wanted in a window when no screen is left for it.
    pub here: bool,
    /// No projector at all.
    pub off: bool,
}

/// Where one window sits.
#[derive(Clone, Debug, PartialEq)]
pub enum Spot {
    /// An ordinary window on whatever screen is left.
    Window,
    /// Borderless and full-screen on this screen; `number` is its place in the list, from 1.
    Screen { number: usize, disp: Disp },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// None when no projector window is wanted: one screen and nobody asked for it.
    pub audience: Option<Spot>,
    /// None when the stage view is not wanted at all.
    pub stage: Option<Spot>,
    /// Indexes into the list, for `Hello`.
    pub audience_index: Option<usize>,
    pub stage_index: Option<usize>,
}

/// Decides where each window goes, like a slide show: with a second screen the projector takes
/// it full-screen, with one screen there is no projector window unless `here` asks for one. A
/// pinned screen is used while it exists. The stage view only exists on its own screen or when
/// `stage_window` asks for a window.
///
/// The first screen is never covered. It holds Artosia's own window, and a full-screen
/// projector over it, always on top, locks the volunteer out of the program: a pin on it is
/// passed over (owner, 2026-11-10: "cant return back to screen 1 as the app only"; on
/// 2026-11-11 a saved pin on screen 1 covered the owner's main screen at start).
pub fn plan(displays: &[Disp], pins: &Pins) -> Plan {
    let find = |name: &Option<String>| name.as_ref().and_then(|name| displays.iter().position(|disp| &disp.name == name)).filter(|index| !displays[*index].primary);
    let mut stage_index = find(&pins.stage);
    let audience_index = if pins.off {
        None
    } else {
        find(&pins.audience).or_else(|| displays.iter().enumerate().find(|(index, disp)| !disp.primary && Some(*index) != stage_index).map(|(index, _)| index))
    };
    // Two windows never share a screen: the projector keeps it.
    if stage_index.is_some() && stage_index == audience_index {
        stage_index = None;
    }
    let spot = |index: usize| Spot::Screen { number: index + 1, disp: displays[index].clone() };
    Plan {
        audience: match audience_index {
            Some(index) => Some(spot(index)),
            None if pins.here && !pins.off => Some(Spot::Window),
            None => None,
        },
        stage: match stage_index {
            Some(index) => Some(spot(index)),
            None if pins.stage_window => Some(Spot::Window),
            None => None,
        },
        audience_index,
        stage_index,
    }
}

static PINS: Mutex<Option<Pins>> = Mutex::new(None);

/// What was asked for last.
pub fn pins() -> Pins {
    PINS.lock().ok().and_then(|pins| pins.clone()).unwrap_or_default()
}

/// Replaces what is asked for and makes the windows react.
pub fn set_pins(pins: Pins, wake: fn()) {
    if let Ok(mut stored) = PINS.lock() {
        *stored = Some(pins);
    }
    EPOCH.fetch_add(1, Ordering::AcqRel);
    wake();
}

static LAYOUT: Mutex<Vec<Disp>> = Mutex::new(Vec::new());
static EPOCH: AtomicU64 = AtomicU64::new(0);
static CHANGED_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// The newest list of screens.
pub fn current() -> Vec<Disp> {
    LAYOUT.lock().map(|list| list.clone()).unwrap_or_default()
}

/// Counts the changes seen so far; a window compares it with the one it last acted on.
pub fn epoch() -> u64 {
    EPOCH.load(Ordering::Acquire)
}

/// When the newest change was noticed, for measuring how long the room waited.
pub fn changed_at() -> Option<Instant> {
    CHANGED_AT.lock().ok().and_then(|at| *at)
}

/// Stores a new list; true when it differs from the one before.
fn publish(list: Vec<Disp>, reason: &str, wake: fn()) -> bool {
    let Ok(mut stored) = LAYOUT.lock() else { return false };
    if *stored == list {
        return false;
    }
    trace(|| format!("displays changed ({reason}): {} -> {}", describe(&stored), describe(&list)));
    *stored = list;
    drop(stored);
    if let Ok(mut at) = CHANGED_AT.lock() {
        *at = Some(Instant::now());
    }
    EPOCH.fetch_add(1, Ordering::AcqRel);
    wake();
    true
}

pub fn describe(list: &[Disp]) -> String {
    let parts: Vec<String> = list
        .iter()
        .map(|disp| format!("{} {}x{}@{},{} x{}{}", disp.name, disp.size.0, disp.size.1, disp.position.0, disp.position.1, disp.scale, if disp.primary { " primary" } else { "" }))
        .collect();
    if parts.is_empty() { "no screens".to_string() } else { parts.join(" | ") }
}

/// Screens named in the file `ARTOSIA_HIDE_DISPLAYS_FILE` (one per line) are left out of the
/// list, so a test can unplug and replug a screen without touching the real one. Unset in use.
fn hidden_by_test() -> Vec<String> {
    let Some(path) = std::env::var_os("ARTOSIA_HIDE_DISPLAYS_FILE") else { return Vec::new() };
    std::fs::read_to_string(path).map(|text| text.lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect()).unwrap_or_default()
}

fn test_mode() -> bool {
    std::env::var_os("ARTOSIA_HIDE_DISPLAYS_FILE").is_some()
}

/// Lists the screens now, or None where this system cannot be asked.
pub fn enumerate() -> Option<Vec<Disp>> {
    let mut list = sys::enumerate()?;
    let hidden = hidden_by_test();
    list.retain(|disp| !hidden.contains(&disp.name));
    // The primary screen comes first, then the rest by position, so numbers stay steady.
    list.sort_by_key(|disp| (!disp.primary, disp.position));
    Some(list)
}

/// Puts the start list in place and begins watching. `wake` runs after every change, from the
/// watcher's thread.
pub fn watch(start: Vec<Disp>, wake: fn()) {
    if let Ok(mut stored) = LAYOUT.lock() {
        *stored = start;
    }
    sys::watch(wake);
}

fn rescan(reason: &str, wake: fn()) {
    if let Some(list) = enumerate() {
        publish(list, reason, wake);
    }
}

#[cfg(windows)]
mod sys {
    use super::{Disp, rescan, test_mode};
    use std::ptr::{null, null_mut};
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW, SetTimer, TranslateMessage, WM_DISPLAYCHANGE, WM_TIMER,
        WNDCLASSW,
    };

    /// The window class of the hidden window that receives `WM_DISPLAYCHANGE`.
    pub const CLASS: &str = "ArtosiaDisplayWatch";
    const MONITORINFOF_PRIMARY: u32 = 1;

    static WAKE: OnceLock<fn()> = OnceLock::new();

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe extern "system" fn collect(monitor: HMONITOR, _dc: HDC, _rect: *mut RECT, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Vec` that `enumerate` passed in and keeps alive for the call.
        let list = unsafe { &mut *(data as *mut Vec<Disp>) };
        let mut info = MONITORINFOEXW { monitorInfo: MONITORINFO { cbSize: size_of::<MONITORINFOEXW>() as u32, ..Default::default() }, szDevice: [0; 32] };
        // SAFETY: `info` is a MONITORINFOEXW whose size is set, which GetMonitorInfoW accepts.
        if unsafe { GetMonitorInfoW(monitor, (&mut info as *mut MONITORINFOEXW).cast::<MONITORINFO>()) } != 0 {
            let rect = info.monitorInfo.rcMonitor;
            let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
            // SAFETY: plain query with two out values.
            let known = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } == 0;
            let end = info.szDevice.iter().position(|c| *c == 0).unwrap_or(info.szDevice.len());
            list.push(Disp {
                name: String::from_utf16_lossy(&info.szDevice[..end]),
                position: (rect.left, rect.top),
                size: ((rect.right - rect.left).max(0) as u32, (rect.bottom - rect.top).max(0) as u32),
                primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
                scale: if known { dpi_x as f32 / 96.0 } else { 1.0 },
            });
        }
        1
    }

    pub fn enumerate() -> Option<Vec<Disp>> {
        // Without this the sizes are the scaled ones; winit asks for the same awareness later.
        // SAFETY: a process-wide setting that fails harmlessly when it is already made.
        unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let mut list: Vec<Disp> = Vec::new();
        // SAFETY: the callback only writes to `list`, which outlives the call.
        let ok = unsafe { EnumDisplayMonitors(null_mut(), null(), Some(collect), (&mut list as *mut Vec<Disp>) as LPARAM) };
        (ok != 0 && !list.is_empty()).then_some(list)
    }

    unsafe extern "system" fn proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let wake = WAKE.get().copied();
        match (message, wake) {
            (WM_DISPLAYCHANGE, Some(wake)) => {
                artosia_slides::frame::trace(|| format!("WM_DISPLAYCHANGE received, {}x{} bits {}", lparam & 0xffff, (lparam >> 16) & 0xffff, wparam));
                rescan("WM_DISPLAYCHANGE", wake)
            }
            (WM_TIMER, Some(wake)) => rescan("poll", wake),
            _ => {}
        }
        // SAFETY: forwards the message this window procedure received, unchanged.
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    /// Takes the window off the taskbar and out of Alt-Tab, as a slide show is. Call it while
    /// the window is hidden: Windows only updates the taskbar on a show.
    pub fn hide_from_taskbar(hwnd: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_TOOLWINDOW};
        let hwnd = hwnd as HWND;
        // SAFETY: reads and writes the extended style of a window of this process, from the thread that owns it.
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (style | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize));
        }
    }

    pub fn watch(wake: fn()) {
        let _ = WAKE.set(wake);
        let spawned = std::thread::Builder::new().name("artosia-displays".into()).spawn(move || {
            let class = wide(CLASS);
            // SAFETY: plain Win32 calls on this thread: register a class, make an invisible
            // top-level window (message-only windows get no broadcasts), pump its messages.
            unsafe {
                let instance = GetModuleHandleW(null());
                let window_class = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: instance, lpszClassName: class.as_ptr(), ..std::mem::zeroed() };
                RegisterClassW(&window_class);
                let hwnd = CreateWindowExW(0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0, null_mut(), null_mut(), instance, null());
                if hwnd.is_null() {
                    artosia_slides::frame::trace(|| "display watcher: no window".to_string());
                    return;
                }
                artosia_slides::frame::trace(|| "display watcher: listening".to_string());
                // A change can arrive without the message, such as a screen woken from sleep.
                SetTimer(hwnd, 1, if test_mode() { 200 } else { 2000 }, None);
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        });
        if spawned.is_err() {
            artosia_slides::frame::trace(|| "display watcher: no thread".to_string());
        }
    }
}

#[cfg(not(windows))]
mod sys {
    use super::Disp;

    pub fn enumerate() -> Option<Vec<Disp>> {
        None
    }

    pub fn watch(_wake: fn()) {}

    pub fn hide_from_taskbar(_hwnd: isize) {}
}

/// Whether the display server tells a program where its windows are and lets it move them.
/// Wayland does neither, by design: its window protocol has no position for a window, so
/// Rune's `WindowHandle::placement` is `None` there for good and `set_position` does nothing.
pub fn tells_positions() -> bool {
    let wayland = ["WAYLAND_DISPLAY", "WAYLAND_SOCKET"].iter().any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()));
    !(cfg!(target_os = "linux") && wayland)
}

/// Takes a window off the taskbar where the system has one; nothing elsewhere.
pub fn hide_from_taskbar(handle: &rune::WindowHandle) {
    let hwnd = handle
        .with_raw_handle(|raw| match raw.as_raw() {
            rune::raw_window_handle::RawWindowHandle::Win32(win) => Some(win.hwnd.get()),
            _ => None,
        })
        .ok()
        .flatten();
    if let Some(hwnd) = hwnd {
        sys::hide_from_taskbar(hwnd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disp(name: &str, x: i32, primary: bool) -> Disp {
        Disp { name: name.to_string(), position: (x, 0), size: (1920, 1080), primary, scale: 1.0 }
    }

    #[test]
    fn one_screen_gives_no_window_until_asked() {
        let list = [disp("A", 0, true)];
        let quiet = plan(&list, &Pins::default());
        assert_eq!(quiet.audience, None);
        assert_eq!(quiet.stage, None);
        assert_eq!(plan(&list, &Pins { here: true, ..Pins::default() }).audience, Some(Spot::Window));
        assert_eq!(plan(&list, &Pins { here: true, off: true, ..Pins::default() }).audience, None);
    }

    #[test]
    fn a_second_screen_takes_the_projector() {
        let list = [disp("A", 0, true), disp("B", 1920, false)];
        let plan = plan(&list, &Pins::default());
        assert_eq!(plan.audience_index, Some(1));
        assert!(matches!(plan.audience, Some(Spot::Screen { number: 2, .. })));
        assert_eq!(super::plan(&list, &Pins { off: true, ..Pins::default() }).audience, None);
    }

    #[test]
    fn a_pinned_stage_has_no_window_when_its_screen_is_gone() {
        let pins = Pins { stage: Some("C".into()), ..Pins::default() };
        let list = [disp("A", 0, true), disp("B", 1920, false), disp("C", 3840, false)];
        assert_eq!(plan(&list, &pins).stage_index, Some(2));
        assert_eq!(plan(&list, &pins).audience_index, Some(1));
        assert_eq!(plan(&list[..2], &pins).stage, None);
        let asked = Pins { stage_window: true, ..pins };
        assert_eq!(plan(&list[..2], &asked).stage, Some(Spot::Window));
    }

    #[test]
    fn a_pinned_projector_is_honoured_while_it_exists() {
        let list = [disp("A", 0, true), disp("B", 1920, false), disp("C", 3840, false)];
        let pins = Pins { audience: Some("C".into()), ..Pins::default() };
        assert_eq!(plan(&list, &pins).audience_index, Some(2));
        assert_eq!(plan(&list[..2], &pins).audience_index, Some(1), "its screen is gone, so the other one");
        // A pin on the first screen, where Artosia's own window is, is passed over.
        let own = Pins { audience: Some("A".into()), ..Pins::default() };
        assert_eq!(plan(&list[..2], &own).audience_index, Some(1));
        assert_eq!(plan(&list[..1], &own).audience, None, "one screen: no projector window");
        assert_eq!(plan(&list[..1], &Pins { here: true, ..own.clone() }).audience, Some(Spot::Window));
        assert_eq!(plan(&list, &Pins { stage: Some("A".into()), ..Pins::default() }).stage, None, "nor the stage view");
        let gone = Pins { audience: Some("Z".into()), ..Pins::default() };
        assert_eq!(plan(&list[..1], &gone).audience, None);
        assert_eq!(plan(&list, &gone).audience_index, Some(1));
    }

    #[test]
    fn the_projector_keeps_a_screen_both_want() {
        let pins = Pins { audience: Some("B".into()), stage: Some("B".into()), ..Pins::default() };
        let list = [disp("A", 0, true), disp("B", 1920, false)];
        let plan = plan(&list, &pins);
        assert_eq!(plan.audience_index, Some(1));
        assert_eq!(plan.stage, None);
    }
}
