//! The setup's own window: 700 by 440 points, borderless, one window that changes in place.
//! Ported from the `opera` look of `examples/looks.rs` (the direction the owner picked, after
//! Opera's web installer; research in `~/Shared/inspo/artosia-installer/`). The screens, sizes
//! and copy are that look's; what is added is what a running window needs: presses, keys, the
//! worker's progress, and the screen the machine in `artosia_setup::flow` says it is on.
//!
//! The window draws a `Model` and turns presses and arrivals into `flow::Event`s. The work
//! (`install::install`, `install::uninstall`) runs on a thread that writes `Shared` and wakes the
//! window; `poll` drains it. Nothing here decides which screen follows which: that is `flow`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use artosia_setup::flow::{Command, Event, Mode, Model, Screen};
use artosia_setup::install::{self, Kind, Step};
use artosia_setup::places::Places;
use artosia_setup::strings::{Key, Language, Strings};
use artosia_update::TrustedKeys;
use rune::color::hex;
use rune::{
    AppSpec, Align, Color, Cx, El, Icon, Image, ImageCache, Key as RuneKey, KeyEvent, LoopSpec, Shape, TextStyle, Tx, Waker, WindowOptions, canvas, col, el, icon, oklch, row, spacer,
    text,
};

/// What the window is told when it starts.
pub struct Config {
    pub mode: Mode,
    pub language: Language,
    pub places: Places,
    pub self_exe: PathBuf,
    pub feed: String,
    /// Whether Artosia is started when the install is done (never under a test root or `--no-start`).
    pub start: bool,
    /// The failure a headless render of the failure screen shows (`--fail`).
    pub shown_failure: Kind,
}

/// The size of the window in points, on every screen.
const SIZE: (f32, f32) = (700.0, 440.0);

/// The exit code the run ends with, set by the machine's `Exit` command.
static EXIT: AtomicI32 = AtomicI32::new(0);
/// True while the files are being swapped in or removed: closing the window then must not end the process.
static HOLD: AtomicBool = AtomicBool::new(false);
static WORKER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Runs the window to its end and returns the exit code of the run, or why it could not open.
pub fn run(config: Config) -> Result<i32, String> {
    let title: &'static str = Box::leak(config.strings().get(Key::Title).to_string().into_boxed_str());
    // The mark on the taskbar and Alt+Tab, as the operator sets it.
    let _ = rune::set_app_icon(include_bytes!("../../../assets/brand/artosia-icon-256.png").to_vec());
    let spec = AppSpec {
        min_size: Some(SIZE),
        scenes: SCENES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new(title, SIZE, init_unused, view)
    };
    let options = WindowOptions { borderless: true, resizable: false, min_size: Some(SIZE), max_size: Some(SIZE), ..WindowOptions::default() };
    let app = rune::App::new(spec)
        .init(move || Ui::new(config))
        .configured_options(options.into())
        .tab_visits_controls(true)
        .on_ready(centre)
        .on_close(|_: &mut Ui, _: &mut Tx| {});
    app.try_run()?;
    // A window closed while the files are being swapped in or removed (Alt+F4) leaves the work to finish.
    if HOLD.load(Ordering::SeqCst) {
        let worker = WORKER.lock().map_err(|e| e.to_string())?.take();
        if let Some(worker) = worker {
            let _ = worker.join();
        }
    }
    Ok(EXIT.load(Ordering::SeqCst))
}

/// The window's top-left that puts a window of `size` (physical pixels) in the middle of the
/// work area `(left, top, right, bottom)`. Rune's own `center` follows AppKit and leaves a quarter
/// of the spare height above, which on a Windows screen reads as too high.
fn centred(work: (i32, i32, i32, i32), size: (u32, u32)) -> (i32, i32) {
    let (width, height) = (work.2 - work.0, work.3 - work.1);
    (work.0 + (width - size.0 as i32) / 2, work.1 + (height - size.1 as i32) / 2)
}

/// Puts the window in the middle of the main screen's work area (the screen less the taskbar).
fn centre(window: &rune::WindowHandle) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
    let mut work = RECT::default();
    // SAFETY: the call fills the one RECT it is given.
    let known = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, Some((&raw mut work).cast()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) }.is_ok();
    match window.placement() {
        Some(placed) if known => window.set_position(centred((work.left, work.top, work.right, work.bottom), placed.content_size)),
        _ => window.center(),
    }
}

impl Config {
    fn strings(&self) -> Strings {
        Strings::new(self.language)
    }
}

// ---------------------------------------------------------------- state

/// What the worker thread tells the window. The window reads it in `poll`; nothing else is shared.
struct Shared {
    step: Mutex<Step>,
    result: Mutex<Option<Result<(), Kind>>>,
    /// The last (step kind, megabytes, percent) a wake was sent for, so a byte count that changes
    /// nothing a person can see costs nothing.
    woken: Mutex<(u8, u64, u8)>,
}

pub struct Ui {
    model: Model,
    config: Config,
    strings: Strings,
    shared: Arc<Shared>,
    waker: Option<Waker>,
    cache: Option<ImageCache>,
    art: Option<Art>,
    shown: (Screen, Step),
    focused_first: bool,
}

impl Ui {
    fn new(config: Config) -> Ui {
        let model = Model::new(config.mode);
        let strings = config.strings();
        let shared = Arc::new(Shared { step: Mutex::new(model.step), result: Mutex::new(None), woken: Mutex::new((0, 0, 0)) });
        Ui { shown: (model.screen, model.step), model, config, strings, shared, waker: None, cache: None, art: None, focused_first: false }
    }

    /// Feeds one event to the machine and does what it says, including the events those things cause.
    fn send(&mut self, event: Event, tx: &mut Tx) {
        let mut queue = vec![event];
        while let Some(event) = queue.pop() {
            for command in self.model.on(event) {
                match command {
                    Command::StartWork => self.start_work(),
                    Command::StartApp => {
                        if self.config.start && !rune::is_headless() {
                            crate::launch::start(&self.config.places.app_exe());
                        }
                    }
                    Command::CheckBusy => queue.push(Event::Busy(self.busy())),
                    Command::Exit(code) => {
                        EXIT.store(code, Ordering::SeqCst);
                        tx.close_window();
                    }
                }
            }
        }
        HOLD.store(self.model.screen == Screen::Removing || (self.model.screen == Screen::Installing && !self.model.can_cancel()), Ordering::SeqCst);
    }

    fn busy(&self) -> bool {
        !install::running_programs(&self.config.places, &self.config.self_exe).is_empty()
    }

    fn start_work(&mut self) {
        // A headless render (`--snap`) draws a scene and does nothing: no worker, never any file.
        if rune::is_headless() {
            return;
        }
        let shared = self.shared.clone();
        shared.result.lock().map(|mut r| *r = None).ok();
        shared.step.lock().map(|mut s| *s = self.model.step).ok();
        let (places, self_exe, feed, waker) = (self.config.places.clone(), self.config.self_exe.clone(), self.config.feed.clone(), self.waker.clone());
        let mode = self.model.mode;
        let handle = std::thread::spawn(move || {
            let wake = || {
                if let Some(waker) = &waker {
                    waker();
                }
            };
            let outcome = match mode {
                Mode::Install => {
                    let report = |step: Step| {
                        let seen = wake_key(step);
                        if let Ok(mut now) = shared.step.lock() {
                            *now = step;
                        }
                        if shared.woken.lock().map(|mut last| std::mem::replace(&mut *last, seen) != seen).unwrap_or(true) {
                            wake();
                        }
                    };
                    install::install(&places, &feed, &TrustedKeys::compiled(), &self_exe, &report).map(|_| ())
                }
                Mode::Uninstall => install::uninstall(&places, &self_exe).inspect(|()| {
                    let _ = install::self_delete(&places, &self_exe);
                }),
            };
            let outcome = outcome.map_err(|failed| {
                crate::launch::log_cause(&failed.detail);
                failed.kind
            });
            if let Ok(mut result) = shared.result.lock() {
                *result = Some(outcome);
            }
            wake();
        });
        if let Ok(mut worker) = WORKER.lock() {
            *worker = Some(handle);
        }
    }
}

/// What a person can tell apart in a step: its kind, the whole megabytes and the percent. The
/// worker wakes the window only when this changes, so a byte count that moves nothing visible
/// costs a lock and a compare and no frame.
fn wake_key(step: Step) -> (u8, u64, u8) {
    match step {
        Step::Checking => (0, 0, 0),
        Step::Downloading { done, .. } => (1, done / 1_048_576, step.percent().unwrap_or(0)),
        Step::SettingUp => (2, 0, 0),
        Step::Removing => (3, 0, 0),
    }
}

fn init_unused() -> Ui {
    // `App::init` builds the real state; this is only the `AppSpec` field Rune insists on.
    Ui::new(Config {
        mode: Mode::Install,
        language: Language::ENGLISH,
        places: Places::under(std::path::Path::new(".")),
        self_exe: PathBuf::new(),
        feed: String::new(),
        start: false,
        shown_failure: Kind::Network,
    })
}

fn setup(s: &mut Ui, waker: Waker) {
    let cache = ImageCache::new(waker.clone());
    s.art = Some(Art::load(&cache, s.config.language));
    s.cache = Some(cache);
    s.waker = Some(waker);
}

/// Every frame: what the worker has said, the clock for the done and close-first screens.
fn poll(s: &mut Ui, clock: f64, tx: &mut Tx) -> bool {
    if !s.focused_first {
        s.focused_first = true;
        // The question before removing opens with the keyboard on Cancel.
        if s.model.focus_starts_on_cancel() {
            tx.focus("cancel", false);
        }
    }
    // A headless render shows its scene as it is: no worker to hear, no clock to run.
    if rune::is_headless() {
        return false;
    }
    let step = s.shared.step.lock().map_or(s.model.step, |step| *step);
    if s.model.screen == Screen::Installing && step != s.model.step {
        s.send(Event::Progress(step), tx);
    }
    let result = s.shared.result.lock().ok().and_then(|mut result| result.take());
    match result {
        Some(Ok(())) => s.send(Event::Done { now: clock }, tx),
        Some(Err(kind)) => s.send(Event::Failed(kind), tx),
        None => {}
    }
    s.send(Event::Tick { now: clock }, tx);
    if matches!(s.model.screen, Screen::Done | Screen::CloseFirst) {
        tx.wake_after(Duration::from_millis(100));
    }
    let now = (s.model.screen, s.model.step);
    std::mem::replace(&mut s.shown, now) != now
}

/// Escape, and Enter when nothing has focus (a focused button presses itself on Enter).
fn on_key(s: &mut Ui, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.mods != rune::Mods::default() {
        return false;
    }
    match ev.key {
        RuneKey::Escape => {
            s.send(Event::Escape, tx);
            true
        }
        RuneKey::Enter if tx.focused().is_none() => {
            let screen = s.model.screen;
            let press = match screen {
                Screen::Start => Some(Event::Install),
                Screen::Failed => Some(Event::TryAgain),
                Screen::Remove => Some(Event::Remove { busy: s.busy(), now: tx.clock() }),
                Screen::Done => Some(Event::Close),
                _ => None,
            };
            press.is_some_and(|event| {
                s.send(event, tx);
                true
            })
        }
        _ => false,
    }
}

/// Enter presses the button that has focus, as Space does. Rune follows AppKit, where Return
/// belongs to the default button and only Space presses the focused one; on Windows a focused
/// button takes both (the operator's `enter_key` says why).
fn enter_key(_: &mut Ui, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key != RuneKey::Enter || ev.mods != rune::Mods::default() {
        return false;
    }
    if !tx.key_repeat() {
        tx.send_key(KeyEvent { key: RuneKey::Text(" ".into()), mods: rune::Mods::default() });
    }
    true
}

// ---------------------------------------------------------------- headless scenes (`--scene S01` ...)

/// The shown figures of the progress screen: 27 of 161 megabytes.
const SHOWN_STEP: Step = Step::Downloading { done: 27 * 1_048_576, total: 161 * 1_048_576 };

type Scene = (&'static str, fn(&mut Ui));

const SCENES: &[Scene] = &[
    ("S01", |s| set(s, Mode::Install, Screen::Start, Step::Checking)),
    ("S02", |s| set(s, Mode::Install, Screen::Installing, SHOWN_STEP)),
    ("S02-setting-up", |s| set(s, Mode::Install, Screen::Installing, Step::SettingUp)),
    ("S03", |s| {
        set(s, Mode::Install, Screen::Failed, Step::Checking);
        s.model.failure = s.config.shown_failure;
    }),
    ("S04", |s| set(s, Mode::Install, Screen::Done, Step::SettingUp)),
    ("S05", |s| set(s, Mode::Uninstall, Screen::Remove, Step::Removing)),
    ("S06", |s| set(s, Mode::Uninstall, Screen::CloseFirst, Step::Removing)),
    ("removing", |s| set(s, Mode::Uninstall, Screen::Removing, Step::Removing)),
    ("remove-failed", |s| {
        set(s, Mode::Uninstall, Screen::Failed, Step::Removing);
        s.model.failure = Kind::Disk;
    }),
];

fn set(s: &mut Ui, mode: Mode, screen: Screen, step: Step) {
    s.model.mode = mode;
    s.model.screen = screen;
    s.model.step = step;
    // What `poll` reads from the worker, so a scene is not overwritten by the idle one.
    s.shared.step.lock().map(|mut shared| *shared = step).ok();
}

// ---------------------------------------------------------------- colour: the Opera look's own

fn c(hex_text: &str) -> Color {
    hex(hex_text).unwrap_or(Color::BLACK)
}

/// Every colour as sRGB hex, so the contrast test reads the same text the window is drawn from.
#[derive(Clone, Copy)]
pub struct Pal {
    pub page: &'static str,
    pub ink: &'static str,
    pub muted: &'static str,
    pub line: &'static str,
    pub track: &'static str,
    pub accent: &'static str,
    pub on_accent: &'static str,
    pub second: &'static str,
    pub second_line: &'static str,
    pub panel_a: &'static str,
    pub panel_b: &'static str,
    pub on_panel: &'static str,
}

impl Pal {
    /// Every text pair: name, words, ground. Each must reach 4.5 to 1.
    #[cfg(test)]
    pub fn pairs(&self) -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("ink on page", self.ink, self.page),
            ("muted on page", self.muted, self.page),
            ("button words on accent", self.on_accent, self.accent),
            ("ink on second button", self.ink, self.second),
            ("panel words on panel start", self.on_panel, self.panel_a),
            ("panel words on panel end", self.on_panel, self.panel_b),
        ]
    }
}

const CRUST: &str = "#6a3913";
const INK: &str = "#251e16";
const CRUMB: &str = "#f0e7d6";

pub fn palette(dark: bool) -> Pal {
    if dark {
        Pal {
            page: "#1b1611",
            ink: CRUMB,
            muted: "#c8bda9",
            line: "#3d352b",
            track: "#3a3127",
            accent: CRUMB,
            on_accent: INK,
            second: "#2a241d",
            second_line: "#7a6f5e",
            panel_a: "#2a1a0d",
            panel_b: "#5a3414",
            on_panel: CRUMB,
        }
    } else {
        Pal {
            page: "#ffffff",
            ink: INK,
            muted: "#5a5044",
            line: "#d9d0c0",
            track: "#e6dccb",
            accent: CRUST,
            on_accent: "#ffffff",
            second: "#ffffff",
            second_line: "#9a8f7c",
            panel_a: "#3e220b",
            panel_b: "#7a4a1f",
            on_panel: CRUMB,
        }
    }
}

/// Yellow means a warning: the failure mark, as the operator's `theme::warn`.
fn warn() -> Color {
    oklch(0.86, 0.16, 92.0)
}

// ---------------------------------------------------------------- icons (the operator's own line icons)

macro_rules! glyph {
    ($name:ident, $id:literal, $body:literal) => {
        static $name: Icon = Icon { name: $id, view: 16.0, body: $body };
    };
}

glyph!(SONG, "a-song", r#"<path d="M6 12.5V3.6L13 2.2V11" stroke="currentColor" stroke-linejoin="round"/><circle cx="4.3" cy="12.5" r="1.7" stroke="currentColor"/><circle cx="11.3" cy="11" r="1.7" stroke="currentColor"/>"#);
glyph!(BOOK, "a-book", r#"<path d="M8 4.5V13.5M8 4.5C7 3.5 5.2 3 2.5 3V12C5.2 12 7 12.5 8 13.5M8 4.5C9 3.5 10.8 3 13.5 3V12C10.8 12 9 12.5 8 13.5" stroke="currentColor" stroke-linejoin="round"/>"#);
glyph!(PHONE, "a-phone", r#"<rect x="4.5" y="1.5" width="7" height="13" rx="1.5" stroke="currentColor"/><path d="M7.4 12.3H8.6" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(SCREEN, "a-screen", r#"<rect x="1.5" y="2.5" width="13" height="9" rx="1" stroke="currentColor"/><path d="M5.5 14H10.5M8 11.5V14" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(CLOSE, "a-close", r#"<path d="M4 4L12 12M12 4L4 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"/>"#);
glyph!(CHECK, "a-check", r#"<path d="M3 8.5L6.5 12L13 4.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8"/>"#);

// ---------------------------------------------------------------- the brand's pictures

macro_rules! brand {
    ($file:literal) => {
        include_bytes!(concat!("../../../assets/brand/", $file)).as_slice()
    };
}

macro_rules! preview {
    ($file:literal) => {
        include_bytes!(concat!("../assets/", $file)).as_slice()
    };
}

struct Art {
    mark: Image,
    mark_crumb: Image,
    lockup: Image,
    lockup_crumb: Image,
    /// The real Artosia window in the setup's language, the part of it the progress screen
    /// shows: light, then dark.
    previews: [Image; 2],
}

impl Art {
    fn load(cache: &ImageCache, language: Language) -> Art {
        let svg = |bytes: &'static [u8]| cache.svg(bytes, 640.0, 2.0);
        let art = Art {
            mark: svg(brand!("artosia-mark.svg")),
            mark_crumb: svg(brand!("artosia-mark-on-dark.svg")),
            lockup: svg(brand!("artosia-lockup-horizontal.svg")),
            lockup_crumb: svg(brand!("artosia-lockup-horizontal-on-dark.svg")),
            previews: preview_bytes(language).map(|bytes| cache.load(bytes)),
        };
        // A headless frame is drawn once, so every picture is decoded before it.
        for image in [&art.mark, &art.mark_crumb, &art.lockup, &art.lockup_crumb].into_iter().chain(art.previews.iter()) {
            let _ = image.wait();
        }
        art
    }

    fn preview(&self, dark: bool) -> &Image {
        &self.previews[usize::from(dark)]
    }
}

/// The two pictures of a language, light then dark. `assets/make-previews.ps1` writes them from
/// the homepage's renders; a language with no render of its own shows the English window.
fn preview_bytes(language: Language) -> [&'static [u8]; 2] {
    match language.code() {
        "id" => [preview!("preview-id-light.png"), preview!("preview-id-dark.png")],
        _ => [preview!("preview-en-light.png"), preview!("preview-en-dark.png")],
    }
}

fn pic(image: &Image, width: f32) -> El {
    el().w(width).pixels(image.pixels())
}

// ---------------------------------------------------------------- small parts

fn style(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).lh((size * 1.4).round())
}

fn t(content: impl Into<String>, size: f32, weight: f32, color: Color) -> El {
    text(content).font(style(size, weight)).color(color)
}

/// A paragraph that wraps inside `width`.
fn para(content: impl Into<String>, size: f32, weight: f32, color: Color, width: f32) -> El {
    t(content, size, weight, color).wrap().w(width)
}

fn centered(content: impl Into<String>, size: f32, weight: f32, color: Color, width: f32) -> El {
    para(content, size, weight, color, width).align(Align::Center)
}

struct Btn {
    label: String,
    fill: Color,
    fg: Color,
    border: Option<(f32, Color)>,
    w: f32,
    h: f32,
    radius: f32,
    size: f32,
    weight: f32,
}

/// A button's width, height, corner radius and type size, in points.
#[derive(Clone, Copy)]
struct Dim {
    w: f32,
    h: f32,
    radius: f32,
    size: f32,
}

/// The Install button, as Opera's is.
const INSTALL: Dim = Dim { w: 122.0, h: 34.0, radius: 3.0, size: 14.0 };
/// Every other button.
const BIG: Dim = Dim { w: 140.0, h: 38.0, radius: 4.0, size: 14.0 };

/// A button the keyboard can reach and Enter presses; `key` names it for `Tx::focus`.
fn btn(key: &'static str, b: Btn, press: fn(&mut Ui, &mut Tx)) -> El {
    let mut e = el().key(key).role("button").label(b.label.clone()).w(b.w).h(b.h).radius(b.radius).fill(b.fill).center().focusable(true).on_key(enter_key).on_press_tx(press).child(t(b.label, b.size, b.weight, b.fg));
    if let Some((width, color)) = b.border {
        e = e.border(width, color);
    }
    e
}

fn primary(key: &'static str, label: String, p: &Pal, d: Dim, press: fn(&mut Ui, &mut Tx)) -> El {
    btn(key, Btn { label, fill: c(p.accent), fg: c(p.on_accent), border: None, w: d.w, h: d.h, radius: d.radius, size: d.size, weight: 600.0 }, press)
}

fn secondary(key: &'static str, label: String, p: &Pal, d: Dim, press: fn(&mut Ui, &mut Tx)) -> El {
    btn(key, Btn { label, fill: c(p.second), fg: c(p.ink), border: Some((1.0, c(p.second_line))), w: d.w, h: d.h, radius: d.radius, size: d.size, weight: 500.0 }, press)
}

/// The button the keyboard is on: a ring in the main colour (removal opens on Cancel).
fn focused(key: &'static str, label: String, p: &Pal, d: Dim, press: fn(&mut Ui, &mut Tx)) -> El {
    btn(key, Btn { label, fill: c(p.second), fg: c(p.ink), border: Some((2.0, c(p.accent))), w: d.w, h: d.h, radius: d.radius, size: d.size, weight: 500.0 }, press)
}

/// A bar: a track and a fill, `value` 0 to 1; `None` is the waiting bar, a segment that slides.
fn bar(width: f32, height: f32, track: Color, fill: Color, value: Option<f32>) -> El {
    let segment = match value {
        Some(v) => el().w(width * v).h(height).fill(fill),
        None => el().w(width * 0.25).h(height).fill(fill).looping(LoopSpec::slide(1.4, [-1.0, 0.0], [4.0, 0.0])),
    };
    row().role("progressbar").w(width).h(height).fill(track).clip_soft(0.0).child(segment)
}

fn disk(size: f32, fill: Color, glyph: &'static Icon, fg: Color) -> El {
    el().size(size, size).radius(size / 2.0).fill(fill).center().child(icon(glyph, size * 0.5).color(fg).a11y_hidden())
}

/// A ring that fills clockwise: `fraction` of a turn from the top.
fn arc(size: f32, width: f32, fraction: f32, color: Color) -> El {
    let radius = size / 2.0 - width / 2.0;
    let points: Vec<[f32; 2]> = (0..=48)
        .map(|step| {
            let angle = -std::f32::consts::FRAC_PI_2 + fraction * std::f32::consts::TAU * step as f32 / 48.0;
            [size / 2.0 + radius * angle.cos(), size / 2.0 + radius * angle.sin()]
        })
        .collect();
    canvas(Shape::new().stroke(points, width, color)).size(size, size)
}

// ---------------------------------------------------------------- the view

fn art(s: &Ui) -> &Art {
    // `setup` runs before the first frame, so the pictures are there.
    s.art.as_ref().unwrap_or_else(|| unreachable!("setup loads the pictures before any view"))
}

fn press_install(s: &mut Ui, tx: &mut Tx) {
    s.send(Event::Install, tx);
}
fn press_try_again(s: &mut Ui, tx: &mut Tx) {
    s.send(Event::TryAgain, tx);
}
fn press_cancel(s: &mut Ui, tx: &mut Tx) {
    s.send(Event::Cancel, tx);
}
fn press_close(s: &mut Ui, tx: &mut Tx) {
    s.send(Event::Close, tx);
}
fn press_remove(s: &mut Ui, tx: &mut Tx) {
    let busy = s.busy();
    s.send(Event::Remove { busy, now: tx.clock() }, tx);
}

fn view(s: &Ui, cx: &mut Cx<Ui>) -> El {
    let p = palette(cx.system_dark());
    let dark = cx.system_dark();
    cx.foreground(c(p.ink));
    cx.focus_ring(2.0, 2.0, c(p.accent));
    let w = &s.strings;
    let a = art(s);
    let lockup = if dark { &a.lockup_crumb } else { &a.lockup };
    let mark = if dark { &a.mark_crumb } else { &a.mark };
    let close_label = w.get(Key::Close).to_string();
    let cross = el()
        .key("cross")
        .absolute(SIZE.0 - 38.0, 12.0)
        .size(26.0, 26.0)
        .center()
        .role("button")
        .label(close_label)
        .focusable(true)
        .on_key(enter_key)
        .on_press_tx(press_close as fn(&mut Ui, &mut Tx))
        .child(icon(&CLOSE, 14.0).color(c(p.muted)));
    // The page: movable by dragging any part of it that is not a button.
    let page = |body: El| el().w_full().h_full().fill(c(p.page)).clip_soft(0.0).window_drag().child(body).child(cross);
    let corner_lockup = || el().absolute(36.0, 34.0).child(pic(lockup, 120.0));
    let model = &s.model;

    match model.screen {
        Screen::Start => {
            let features = [(&SONG, Key::FeatureSongs), (&BOOK, Key::FeatureBible), (&PHONE, Key::FeaturePhone), (&SCREEN, Key::FeatureProjector)];
            let mut list = col().gap(30.0).pl(36.0);
            for (glyph, key) in features {
                list = list.child(row().gap(16.0).center_y().child(icon(glyph, 26.0).color(c(p.on_panel))).child(para(w.get(key), 15.0, 400.0, c(p.on_panel), 190.0)));
            }
            let left = col().w(280.0).h_full().justify_center().gradient(c(p.panel_a), c(p.panel_b)).child(list);
            let right = col()
                .grow(1.0)
                .h_full()
                .px(40.0)
                .pt(112.0)
                .pb(36.0)
                .child(pic(lockup, 150.0))
                .child(el().h(26.0))
                .child(para(w.get(Key::StartTagline), 22.0, 700.0, c(p.ink), 320.0))
                .child(el().h(12.0))
                .child(para(w.get(Key::StartBody), 13.0, 400.0, c(p.muted), 320.0))
                .child(spacer())
                .child(row().justify_end().child(primary("install", w.get(Key::Install).to_string(), &p, INSTALL, press_install)));
            page(row().w_full().h_full().child(left).child(right))
        }
        Screen::Installing | Screen::Removing => {
            let removing = model.screen == Screen::Removing;
            let preview = a.preview(dark);
            let (heading, line) = if removing { (Key::RemovingHeading, Key::WaitRemoveBody) } else { (Key::InstallHeading, Key::WaitBody) };
            let words = col()
                .absolute(36.0, 214.0)
                .child(para(w.get(heading), 22.0, 700.0, c(p.ink), 290.0))
                .child(el().h(10.0))
                .child(para(w.get(line), 14.0, 400.0, c(p.ink), 290.0));
            let mut status = row().absolute(36.0, 100.0).gap(14.0).child(t(model.step.text(w), 14.0, 400.0, c(p.ink)));
            if let Some(percent) = model.step.percent() {
                status = status.child(t(format!("{percent}%"), 14.0, 400.0, c(p.ink)));
            }
            // A download fills the bar by its bytes; the swap leaves it full; removing slides.
            let value = match model.step {
                Step::Downloading { .. } => model.step.percent().map(|percent| f32::from(percent) / 100.0),
                Step::Checking => Some(0.0),
                Step::SettingUp => Some(1.0),
                Step::Removing => None,
            };
            let shot = el()
                .absolute(372.0, 200.0)
                .size(328.0, 240.0)
                .corners([8.0, 0.0, 0.0, 0.0])
                .clip_soft(0.0)
                .shadow(0.0, 16.0, Color::BLACK.alpha(0.25))
                .child(el().size(328.0, 240.0).pixels_fit(preview.pixels(), rune::SurfaceFit::Fill));
            let mut body = el()
                .w_full()
                .h_full()
                .child(corner_lockup())
                .child(status)
                .child(el().absolute(36.0, 132.0).child(bar(628.0, 17.0, c(p.track), c(p.accent), value)))
                .child(words)
                .child(shot);
            if !removing {
                // Cancel stays on screen while the files are swapped in, and says nothing then.
                let cancel_words = w.get(Key::Cancel).to_string();
                let cancel = if model.can_cancel() {
                    el().key("cancel").absolute(36.0, 396.0).role("button").label(cancel_words.clone()).focusable(true).on_key(enter_key).on_press_tx(press_cancel as fn(&mut Ui, &mut Tx)).child(t(cancel_words, 13.0, 500.0, c(p.ink)))
                } else {
                    el().absolute(36.0, 396.0).role("button").label(cancel_words.clone()).opacity(0.45).child(t(cancel_words, 13.0, 500.0, c(p.ink)))
                };
                body = body.child(cancel);
            }
            page(body)
        }
        Screen::Failed => {
            let (heading, line) = model.failure_words();
            page(el().w_full().h_full().child(corner_lockup()).child(
                col()
                    .w_full()
                    .h_full()
                    .center()
                    .pt(30.0)
                    .gap(12.0)
                    .child(disk(56.0, warn(), &CLOSE, c(INK)))
                    .child(el().h(6.0))
                    .child(centered(w.get(heading), 22.0, 700.0, c(p.ink), 440.0))
                    .child(centered(w.get(line), 14.0, 400.0, c(p.muted), 440.0))
                    .child(el().h(14.0))
                    .child(
                        row()
                            .gap(12.0)
                            .child(secondary("close", w.get(Key::Close).to_string(), &p, BIG, press_close))
                            .child(primary("try-again", w.get(Key::TryAgain).to_string(), &p, BIG, press_try_again)),
                    ),
            ))
        }
        Screen::Done => page(el().w_full().h_full().child(corner_lockup()).child(
            col()
                .w_full()
                .h_full()
                .center()
                .pt(30.0)
                .gap(12.0)
                .child(disk(56.0, c(p.accent), &CHECK, c(p.on_accent)))
                .child(el().h(6.0))
                .child(centered(w.get(Key::DoneHeading), 22.0, 700.0, c(p.ink), 440.0))
                .child(centered(w.get(Key::DoneBody), 14.0, 400.0, c(p.muted), 440.0)),
        )),
        Screen::Remove => page(
            el()
                .w_full()
                .h_full()
                .child(corner_lockup())
                .child(col().absolute(36.0, 120.0).child(para(w.get(Key::RemoveHeading), 22.0, 700.0, c(p.ink), 540.0)).child(el().h(12.0)).child(para(w.get(Key::RemoveBody), 14.0, 400.0, c(p.muted), 540.0)))
                .child(
                    el().absolute(SIZE.0 - 36.0 - 292.0, SIZE.1 - 36.0 - 38.0).child(
                        row()
                            .gap(12.0)
                            .child(focused("cancel", w.get(Key::Cancel).to_string(), &p, BIG, press_cancel))
                            .child(primary("remove", w.get(Key::RemoveYes).to_string(), &p, BIG, press_remove)),
                    ),
                ),
        ),
        Screen::CloseFirst => {
            let ring = el().size(88.0, 88.0).center().child(el().absolute(0.0, 0.0).child(arc(88.0, 3.0, 1.0, c(p.line)))).child(el().absolute(0.0, 0.0).child(arc(88.0, 3.0, 0.25, c(p.accent)))).child(pic(mark, 38.0));
            page(
                el().w_full().h_full().child(
                    col()
                        .w_full()
                        .h_full()
                        .center()
                        .gap(12.0)
                        .child(ring)
                        .child(el().h(8.0))
                        .child(centered(w.get(Key::CloseFirstHeading), 22.0, 700.0, c(p.ink), 440.0))
                        .child(centered(w.get(Key::CloseFirstBody), 14.0, 400.0, c(p.muted), 440.0))
                        .child(el().h(10.0))
                        .child(secondary("cancel", w.get(Key::Cancel).to_string(), &p, BIG, press_cancel)),
                ),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color, b: Color) -> f32 {
        let (a, b) = (a.luminance(), b.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn every_text_and_button_pair_of_the_window_reads_at_four_and_a_half() {
        for dark in [false, true] {
            for (name, fg, bg) in palette(dark).pairs() {
                let ratio = contrast(c(fg), c(bg));
                assert!(ratio >= 4.5, "{} {name}: {fg} on {bg} is {ratio:.2}", if dark { "dark" } else { "light" });
            }
        }
    }

    #[test]
    fn a_wake_is_asked_for_only_when_a_person_could_see_the_step_change() {
        let at = |done: u64| wake_key(Step::Downloading { done, total: 168_820_736 });
        assert_eq!(at(1_048_576), at(1_048_576 + 65_535), "a chunk inside the same megabyte shows nothing new");
        assert_ne!(at(1_048_576), at(2 * 1_048_576), "the next megabyte does");
        assert_ne!(wake_key(Step::Checking), at(0));
        assert_ne!(at(0), wake_key(Step::SettingUp));
        // A whole install: 161 megabytes in 64 KiB chunks wake the window once per megabyte and once
        // per percent, about 260 times.
        let (mut last, mut wakes) = (wake_key(Step::Checking), 0);
        for chunk in 0..(168_820_736u64 / 65_536) {
            let now = at(chunk * 65_536);
            if now != last {
                wakes += 1;
                last = now;
            }
        }
        assert!((250..=262).contains(&wakes), "{wakes} wakes for a whole download");
    }

    /// The per-event path of the worker: one `wake_key` and one compare for each chunk the
    /// updater reports. Printed with `--nocapture`; the bound only catches a slow path.
    #[test]
    fn the_progress_path_costs_next_to_nothing() {
        let start = std::time::Instant::now();
        let mut last = wake_key(Step::Checking);
        let mut wakes = 0u32;
        for chunk in 0..1_000_000u64 {
            let now = wake_key(Step::Downloading { done: (chunk % 2_576) * 65_536, total: 168_820_736 });
            if std::mem::replace(&mut last, now) != now {
                wakes += 1;
            }
        }
        let each = start.elapsed().as_nanos() as f64 / 1_000_000.0;
        println!("progress path: {each:.1} ns per report, {wakes} changes in 1,000,000 reports");
        assert!(each < 2_000.0, "{each} ns per report");
    }

    #[test]
    fn a_window_is_centred_in_the_work_area_whatever_its_corner() {
        // The 875 by 550 pixel window of a 125 percent screen, 2560 by 1020 of work area.
        assert_eq!(centred((0, 0, 2560, 1020), (875, 550)), (842, 235));
        // A taskbar on the left takes 48 pixels; a second screen's corner is not the origin.
        assert_eq!(centred((48, 0, 2560, 1040), (875, 550)), (866, 245));
        assert_eq!(centred((-1920, 0, 0, 1080), (700, 440)), (-1310, 320));
    }

    #[test]
    fn the_failure_mark_cross_reads_on_yellow() {
        assert!(contrast(c(INK), warn()) >= 4.5);
    }
}
