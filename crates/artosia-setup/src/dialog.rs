//! What a person sees: the platform's own task dialog, never a window of ours. One generic
//! `show` builds a `TASKDIALOGCONFIG` and runs it; a 100 ms-ish timer notification calls the
//! caller's `tick`, which updates the dialog and says when to close it. The exe manifest must ask
//! for common controls 6 or `TaskDialogIndirect` is not found when the file starts.

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{HWND, LPARAM, S_OK, WPARAM};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOG_FLAGS, TASKDIALOG_NOTIFICATIONS, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_ERROR_ICON, TDE_CONTENT, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_CALLBACK_TIMER, TDF_SHOW_PROGRESS_BAR, TDF_USE_HICON_MAIN, TDM_CLICK_BUTTON, TDM_ENABLE_BUTTON, TDM_SET_MARQUEE_PROGRESS_BAR, TDM_SET_PROGRESS_BAR_MARQUEE,
    TDM_SET_PROGRESS_BAR_POS, TDM_UPDATE_ELEMENT_TEXT, TDN_CREATED, TDN_TIMER, TaskDialogIndirect,
};
use windows::Win32::UI::WindowsAndMessaging::{ICON_BIG, ICON_SMALL, IDCANCEL, SendMessageW, WM_SETICON};
use windows::core::{HRESULT, PCWSTR};

use artosia_setup::install::Step;
use artosia_setup::strings::{Key, Strings};
use artosia_setup::win::{app_icon, wide};

/// The id of the button that cancels; Escape and the close box press it.
const CANCEL: i32 = IDCANCEL.0;
/// The id of the one other button of a dialog.
const ACT: i32 = 1001;

struct Config<'a> {
    title: String,
    heading: String,
    content: String,
    error_icon: bool,
    progress_bar: bool,
    buttons: Vec<(i32, String)>,
    default_button: i32,
    /// Called on creation and on every timer notification; true closes the dialog.
    tick: &'a dyn Fn(HWND) -> bool,
}

struct Context<'a> {
    tick: &'a dyn Fn(HWND) -> bool,
    closing: Cell<bool>,
}

/// Runs one dialog and returns the id of the button pressed, and whether `tick` closed it.
fn show(config: &Config) -> io::Result<(i32, bool)> {
    let context = Context { tick: config.tick, closing: Cell::new(false) };
    let (title, heading, content) =
        (wide(OsStr::new(&config.title)), wide(OsStr::new(&config.heading)), wide(OsStr::new(&config.content)));
    let texts: Vec<Vec<u16>> = config.buttons.iter().map(|(_, text)| wide(OsStr::new(text))).collect();
    let buttons: Vec<TASKDIALOG_BUTTON> =
        config.buttons.iter().zip(&texts).map(|((id, _), text)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: PCWSTR(text.as_ptr()) }).collect();

    let mut flags = TDF_ALLOW_DIALOG_CANCELLATION.0 | TDF_CALLBACK_TIMER.0;
    if !config.error_icon {
        flags |= TDF_USE_HICON_MAIN.0;
    }
    if config.progress_bar {
        flags |= TDF_SHOW_PROGRESS_BAR.0;
    }
    // Artosia's mark is the main icon of every dialog but a failure, where the system's error
    // icon says what the dialog is. Every dialog's title bar and taskbar button show the mark.
    let main_icon = if config.error_icon { TASKDIALOGCONFIG_0 { pszMainIcon: TD_ERROR_ICON } } else { TASKDIALOGCONFIG_0 { hMainIcon: app_icon(true) } };
    let dialog = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TASKDIALOG_FLAGS(flags),
        pszWindowTitle: PCWSTR(title.as_ptr()),
        Anonymous1: main_icon,
        pszMainInstruction: PCWSTR(heading.as_ptr()),
        pszContent: PCWSTR(content.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: config.default_button,
        pfCallback: Some(callback),
        lpCallbackData: &context as *const Context as isize,
        ..Default::default()
    };

    let mut pressed = 0i32;
    // SAFETY: every buffer the config points at lives to the end of this function, and the
    // callback data is `context`, which outlives the call.
    unsafe { TaskDialogIndirect(&dialog, Some(&mut pressed), None, None) }.map_err(io::Error::other)?;
    Ok((pressed, context.closing.get()))
}

unsafe extern "system" fn callback(hwnd: HWND, message: TASKDIALOG_NOTIFICATIONS, _wparam: WPARAM, _lparam: LPARAM, data: isize) -> HRESULT {
    if message == TDN_CREATED || message == TDN_TIMER {
        // SAFETY: `data` is the `Context` that `show` keeps alive while the dialog runs.
        let context = unsafe { &*(data as *const Context) };
        if message == TDN_CREATED {
            for (size, big) in [(ICON_SMALL, false), (ICON_BIG, true)] {
                send(hwnd, WM_SETICON, size as usize, app_icon(big).0 as isize);
            }
        }
        if !context.closing.get() && (context.tick)(hwnd) {
            context.closing.set(true);
            send(hwnd, TDM_ENABLE_BUTTON.0 as u32, CANCEL as usize, 1);
            send(hwnd, TDM_CLICK_BUTTON.0 as u32, CANCEL as usize, 0);
        }
    }
    S_OK
}

fn send(hwnd: HWND, message: u32, wparam: usize, lparam: isize) {
    // SAFETY: a plain message to the task dialog this callback belongs to.
    unsafe { SendMessageW(hwnd, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))) };
}

fn set_content(hwnd: HWND, text: &str) {
    let text = wide(OsStr::new(text));
    send(hwnd, TDM_UPDATE_ELEMENT_TEXT.0 as u32, TDE_CONTENT.0 as usize, text.as_ptr() as isize);
}

/// What the worker tells the dialog. The dialog's timer reads it; nothing else is shared.
pub struct Shared {
    step: Mutex<Step>,
    finished: AtomicBool,
}

impl Shared {
    pub fn set(&self, step: Step) {
        if let Ok(mut now) = self.step.lock() {
            *now = step;
        }
    }
}

/// Runs `work` on a thread while a progress dialog shows what it reports. None when the person
/// pressed Cancel: the caller exits, and anything downloaded stays for the updater to resume.
pub fn progress<T: Send + 'static>(
    strings: &Strings,
    heading: Key,
    first: Step,
    work: impl FnOnce(&Shared) -> T + Send + 'static,
) -> io::Result<Option<T>> {
    let shared = Arc::new(Shared { step: Mutex::new(first), finished: AtomicBool::new(false) });
    let worker = {
        let shared = shared.clone();
        std::thread::spawn(move || {
            // Set `finished` even when `work` panics, or the dialog would never close.
            struct Finish(Arc<Shared>);
            impl Drop for Finish {
                fn drop(&mut self) {
                    self.0.finished.store(true, Ordering::SeqCst);
                }
            }
            let _finish = Finish(shared.clone());
            work(&shared)
        })
    };

    let text_of = |step: Step| step.text(strings);
    let shown = RefCell::new(text_of(first));
    // None before the first tick; Some(true) while the bar fills, Some(false) while it slides.
    let filling = Cell::new(None::<bool>);
    let cancel_on = Cell::new(true);
    let tick = |hwnd: HWND| {
        if shared.finished.load(Ordering::SeqCst) {
            return true;
        }
        let step = shared.step.lock().map_or(first, |step| *step);
        let text = text_of(step);
        if *shown.borrow() != text {
            set_content(hwnd, &text);
            *shown.borrow_mut() = text;
        }
        // A bar that fills once the size is known; a sliding one before that and while the
        // updater swaps files in, which cannot be told in percent.
        match step {
            Step::Downloading { done, total } if total > 0 => {
                if filling.replace(Some(true)) != Some(true) {
                    send(hwnd, TDM_SET_MARQUEE_PROGRESS_BAR.0 as u32, 0, 0);
                }
                send(hwnd, TDM_SET_PROGRESS_BAR_POS.0 as u32, (done.min(total) * 100 / total) as usize, 0);
            }
            _ => {
                if filling.replace(Some(false)) != Some(false) {
                    send(hwnd, TDM_SET_MARQUEE_PROGRESS_BAR.0 as u32, 1, 0);
                    send(hwnd, TDM_SET_PROGRESS_BAR_MARQUEE.0 as u32, 1, 30);
                }
            }
        }
        // Past the download a swap is under way; stopping it half way would only be undone.
        if matches!(step, Step::SettingUp | Step::Removing) && cancel_on.replace(false) {
            send(hwnd, TDM_ENABLE_BUTTON.0 as u32, CANCEL as usize, 0);
        }
        false
    };
    let (_, closed_by_tick) = show(&Config {
        title: strings.get(Key::Title).to_string(),
        heading: strings.get(heading).to_string(),
        content: text_of(first),
        error_icon: false,
        progress_bar: true,
        buttons: vec![(CANCEL, strings.get(Key::Cancel).to_string())],
        default_button: CANCEL,
        tick: &tick,
    })?;
    if !closed_by_tick {
        return Ok(None);
    }
    match worker.join() {
        Ok(result) => Ok(Some(result)),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// A failed install: what happened and what to do. True when the person asks to try again.
pub fn failure(strings: &Strings, heading: Key, body: Key) -> io::Result<bool> {
    let (pressed, _) = show(&Config {
        title: strings.get(Key::Title).to_string(),
        heading: strings.get(heading).to_string(),
        content: strings.get(body).to_string(),
        error_icon: true,
        progress_bar: false,
        buttons: vec![(ACT, strings.get(Key::TryAgain).to_string()), (CANCEL, strings.get(Key::Close).to_string())],
        default_button: ACT,
        tick: &|_| false,
    })?;
    Ok(pressed == ACT)
}

/// The question before removing. True on Remove. Focus starts on Cancel.
pub fn confirm_remove(strings: &Strings) -> io::Result<bool> {
    let (pressed, _) = show(&Config {
        title: strings.get(Key::Title).to_string(),
        heading: strings.get(Key::RemoveHeading).to_string(),
        content: strings.get(Key::RemoveBody).to_string(),
        error_icon: false,
        progress_bar: false,
        buttons: vec![(ACT, strings.get(Key::RemoveYes).to_string()), (CANCEL, strings.get(Key::Cancel).to_string())],
        default_button: CANCEL,
        tick: &|_| false,
    })?;
    Ok(pressed == ACT)
}

/// Asks the person to close Artosia and waits, checking each second, until `busy` says no.
/// False when they cancel instead.
pub fn wait_until_closed(strings: &Strings, busy: &dyn Fn() -> bool) -> io::Result<bool> {
    let last_check = Cell::new(None::<std::time::Instant>);
    let tick = |_: HWND| {
        if last_check.get().is_some_and(|at| at.elapsed().as_millis() < 1000) {
            return false;
        }
        last_check.set(Some(std::time::Instant::now()));
        !busy()
    };
    let (_, freed) = show(&Config {
        title: strings.get(Key::Title).to_string(),
        heading: strings.get(Key::CloseFirstHeading).to_string(),
        content: strings.get(Key::CloseFirstBody).to_string(),
        error_icon: false,
        progress_bar: false,
        buttons: vec![(CANCEL, strings.get(Key::Cancel).to_string())],
        default_button: CANCEL,
        tick: &tick,
    })?;
    Ok(freed)
}
