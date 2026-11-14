//! The output process: the projector window and the stage window. It listens on a local
//! port for the operator and keeps showing its last state when the operator goes away, so an
//! operator crash never blacks out the room.
//!
//! `artosia-output [--port N] [--audience I] [--stage I] [--stage-window]`. Headless:
//! `--window audience|stage --scene <name> --snap out.png --size 1920x1080 --scale 1`.
//! `--demo-video <file>` starts on sample lyrics over that video, for measuring RAM without
//! an operator; with `ARTOSIA_TRACE` set it prints the playback position every two seconds.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "video")]
mod cameras;
mod demo;
mod displays;
#[cfg(feature = "video")]
mod items;
mod media;
mod placer;
#[cfg(feature = "video")]
mod video;
mod warta_data;

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use artosia_slides::audience::{Detail, Media, audience_over};
use artosia_slides::frame::{DisplayInfo, FromOutput, MediaEnd, MediaItem, MediaKind, OutputState, PORT, ToOutput, decode, encode, trace};
use artosia_core::deck::NoticeKind;
use artosia_slides::look::{Background, NoticeStyle};
use artosia_slides::stage::stage;
use rune::{AppSpec, Color, Cx, El, Image, ImageCache, NativeSurface, TextStyle, Tx, Waker, WindowOptions, WindowSpec, col, el, text};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    Audience,
    Stage,
    /// No window: keeps the process and its wake-ups alive while no projector is shown.
    Anchor,
}

/// What both windows and the socket thread share.
struct Shared {
    state: Mutex<OutputState>,
    /// Bumped on every change, so a window knows to redraw.
    version: AtomicU64,
    identify: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
    hello: Mutex<FromOutput>,
    /// The screens now, and the index of the one each window covers.
    layout: Mutex<(Vec<displays::Disp>, Option<usize>, Option<usize>)>,
    cameras: Mutex<Vec<String>>,
    client: Mutex<Option<TcpStream>>,
    surface: NativeSurface,
    /// The frames of the rundown's video item, drawn over the background.
    item_surface: NativeSurface,
    /// The item layer is drawn: a video item is playing or has ended on black.
    item_live: AtomicBool,
    /// The video surface holds frames of the current background.
    video_live: AtomicBool,
    /// The video is held back while a window opens (see `placer.rs`).
    quiet: AtomicBool,
    /// Set by a click or Escape on the projector window the operator asked for.
    hide_request: AtomicBool,
    /// The size the audience view last laid out for, in points, and its scale, for the trace.
    last_view: Mutex<(f32, f32, f32)>,
}

/// The corner pin and colour correction of the projector picture, shared with its compositor.
fn warp() -> &'static artosia_warp::WarpHandle {
    static WARP: OnceLock<artosia_warp::WarpHandle> = OnceLock::new();
    WARP.get_or_init(artosia_warp::WarpHandle::default)
}

fn shared() -> &'static Arc<Shared> {
    static SHARED: OnceLock<Arc<Shared>> = OnceLock::new();
    SHARED.get_or_init(|| {
        Arc::new(Shared {
            state: Mutex::new(OutputState::default()),
            version: AtomicU64::new(1),
            identify: AtomicBool::new(false),
            wakers: Mutex::new(Vec::new()),
            hello: Mutex::new(FromOutput::Hello { displays: Vec::new(), audience: None, stage: None, video: cfg!(feature = "video"), cameras: Vec::new() }),
            layout: Mutex::new((Vec::new(), None, None)),
            cameras: Mutex::new(Vec::new()),
            client: Mutex::new(None),
            surface: NativeSurface::new(),
            item_surface: NativeSurface::new(),
            item_live: AtomicBool::new(false),
            video_live: AtomicBool::new(false),
            quiet: AtomicBool::new(false),
            hide_request: AtomicBool::new(false),
            last_view: Mutex::new((0.0, 0.0, 0.0)),
        })
    })
}

impl Shared {
    fn touch(&self) {
        self.version.fetch_add(1, Ordering::AcqRel);
        if let Ok(wakers) = self.wakers.lock() {
            for wake in wakers.iter() {
                wake();
            }
        }
    }

    fn set_state(&self, state: OutputState) {
        warp().set(state.warp);
        if let Ok(mut current) = self.state.lock() {
            *current = state;
        }
        self.touch();
    }

    fn state(&self) -> OutputState {
        self.state.lock().map(|state| state.clone()).unwrap_or_default()
    }

    /// Tells the operator something, if one is connected.
    fn tell(&self, message: &FromOutput) {
        if let Ok(mut client) = self.client.lock()
            && let Some(stream) = client.as_mut() {
                let _ = stream.write_all(encode(message).as_bytes());
            }
    }
}

pub(crate) struct Screen {
    role: Role,
    /// The screen's number as the operator lists it; 0 for an ordinary window.
    number: usize,
    seen: u64,
    tick: u64,
    now: f64,
    images: Option<ImageCache>,
    picture: Option<(String, Image)>,
    logo: Option<(String, Image)>,
    /// The picture of the slide on screen, when it has one.
    slide_picture: Option<(String, Image)>,
    slide_qris: Option<(String, Image)>,
    slide_image2: Option<(String, Image)>,
}

impl Screen {
    fn new(role: Role, number: usize) -> Screen {
        Screen { role, number, seen: 0, tick: 0, now: unix_now(), images: None, picture: None, logo: None, slide_picture: None, slide_qris: None, slide_image2: None }
    }
}

fn process_start() -> std::time::Instant {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    *START.get_or_init(std::time::Instant::now)
}

fn unix_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn flag_value(flag: &str) -> Option<String> {
    let args = rune::extra_args();
    args.iter().position(|arg| arg == flag).and_then(|at| args.get(at + 1)).cloned()
}

fn has_flag(flag: &str) -> bool {
    rune::extra_args().iter().any(|arg| arg == flag)
}

fn setup(s: &mut Screen, waker: Waker) {
    let shared = shared();
    if let Ok(mut wakers) = shared.wakers.lock() {
        wakers.push(waker.clone());
    }
    if s.role == Role::Audience {
        shared.surface.set_waker(waker.clone());
        shared.item_surface.set_waker(waker.clone());
        warp().add_waker(waker.clone());
        s.images = Some(ImageCache::new(waker));
    }
}

/// A headless snapshot draws once, so it waits for a slide's picture to decode; the projector never blocks.
fn waited(image: &Image) {
    if std::env::args().any(|arg| arg == "--snap") {
        let _ = image.wait();
    }
}

fn poll(s: &mut Screen, _clock: f64, tx: &mut Tx) -> bool {
    let shared = shared();
    placer::tick();
    placer::trace_stats();
    if let Some(after) = flag_value("--demo-panic").and_then(|secs| secs.parse::<u64>().ok())
        && process_start().elapsed().as_secs() >= after
    {
        panic!("--demo-panic: a deliberate panic to test the restart");
    }
    let version = shared.version.load(Ordering::Acquire);
    let state = shared.state();
    let now = unix_now();
    // The stage clock shows minutes; a countdown shows seconds.
    let ticking = state.countdown.is_some();
    let tick = if ticking { now.floor() as u64 } else { (now / 60.0).floor() as u64 };
    // A window that is opening is checked often, so the old one closes as soon as it can.
    let wait = if placer::busy() { 50 } else if ticking { 200 } else { 2000 };
    tx.wake_after(std::time::Duration::from_millis(wait));
    if let Some(images) = &s.images {
        let wanted = match &state.look.background {
            Background::Image(path) => Some(path.clone()),
            _ => None,
        };
        if s.picture.as_ref().map(|(path, _)| path) != wanted.as_ref() {
            s.picture = wanted.map(|path| {
                let image = images.file(&path, 1920.0, 1.0);
                (path, image)
            });
        }
        let logo = Some(state.logo.clone()).filter(|path| !path.is_empty());
        if s.logo.as_ref().map(|(path, _)| path) != logo.as_ref() {
            s.logo = logo.map(|path| {
                let image = images.file(&path, 1920.0, 1.0);
                (path, image)
            });
        }
        let second = state.slide.as_ref().map(|slide| slide.image2.clone()).filter(|path| !path.is_empty());
        if s.slide_image2.as_ref().map(|(path, _)| path) != second.as_ref() {
            s.slide_image2 = second.map(|path| {
                let image = images.file(&path, 1920.0, 1.0);
                waited(&image);
                (path, image)
            });
        }
        let qris = state.slide.as_ref().map(|slide| slide.qris.clone()).filter(|path| !path.is_empty());
        if s.slide_qris.as_ref().map(|(path, _)| path) != qris.as_ref() {
            s.slide_qris = qris.map(|path| {
                let image = images.file(&path, 1920.0, 1.0);
                waited(&image);
                (path, image)
            });
        }
        let slide = state.slide.as_ref().map(|slide| slide.image.clone()).filter(|path| !path.is_empty());
        if s.slide_picture.as_ref().map(|(path, _)| path) != slide.as_ref() {
            s.slide_picture = slide.map(|path| {
                let image = images.file(&path, 1920.0, 1.0);
                waited(&image);
                (path, image)
            });
        }
    }
    let changed = version != s.seen || tick != s.tick;
    s.seen = version;
    s.tick = tick;
    s.now = now;
    changed
}

fn identify(number: usize, w: f32, h: f32) -> El {
    let label = if number == 0 { "Window".to_string() } else { number.to_string() };
    col()
        .key("identify")
        .absolute(0.0, 0.0)
        .size(w, h)
        .center()
        .fill(Color::BLACK.alpha(0.86))
        .child(text(label).font(TextStyle::new((h * 0.5).round(), 800.0)).color(Color::WHITE))
}

fn view(s: &Screen, cx: &mut Cx<Screen>) -> El {
    let shared = shared();
    let mut root = el().key("screen").w_full().h_full().fill(Color::BLACK);
    if s.role == Role::Anchor {
        return root;
    }
    // The window's own laid-out size. Until layout has one, or while it is smaller than a
    // plausible window, only the background is drawn: text laid out for a guessed size is what
    // showed twice too large for one frame.
    let Some([w, h]) = cx.layout_size("screen").filter(|[w, h]| *w >= 16.0 && *h >= 9.0) else {
        trace(|| format!("{:?} view: size not known yet, drawing the background only", s.role));
        return root;
    };
    let state = shared.state();
    if s.role == Role::Audience {
        let scale = cx.env().scale;
        if let Ok(mut last) = shared.last_view.lock()
            && *last != (w, h, scale)
        {
            *last = (w, h, scale);
            trace(|| format!("projector view laid out for {w}x{h} points at scale {scale}"));
        }
    }
    root = match s.role {
        Role::Audience => {
            let video = (shared.video_live.load(Ordering::Acquire) && !shared.quiet.load(Ordering::Acquire)).then_some(&shared.surface);
            let media = Media {
                picture: s.picture.as_ref().map(|(_, image)| image.pixels()),
                video: video.filter(|_| state.look.is_moving()),
                logo: s.logo.as_ref().map(|(_, image)| image.pixels()),
                slide: s.slide_picture.as_ref().map(|(_, image)| image.pixels()),
                qris: s.slide_qris.as_ref().map(|(_, image)| image.pixels()),
                photo2: s.slide_image2.as_ref().map(|(_, image)| image.pixels()),
                item: None,
            };
            let item = (shared.item_live.load(Ordering::Acquire) && !shared.quiet.load(Ordering::Acquire) && state.media.as_ref().is_some_and(|item| item.kind == MediaKind::Video)).then_some(&shared.item_surface);
            match demo::sheet(w, h).or_else(|| demo::direct().then(|| demo::through_notice(&state, w, h))) {
                Some(sheet) => root.child(sheet),
                None => root.child(audience_over(&state, w, h, media, item, Detail::Output, s.now).absolute(0.0, 0.0)),
            }
        }
        Role::Stage => root.child(stage(&state, w, h, s.now).absolute(0.0, 0.0)),
        Role::Anchor => root,
    };
    if shared.identify.load(Ordering::Acquire) {
        root = root.child(identify(s.number, w, h));
    }
    // The projector window the operator asked for on a single screen hides on a click.
    if s.role == Role::Audience && s.number == 0 {
        root = root.on_press(|_: &mut Screen| hide_projector_window());
    }
    root
}

/// Asks the placer to hide the projector window, as Escape or a click does.
fn hide_projector_window() {
    shared().hide_request.store(true, Ordering::Release);
    shared().touch();
}

fn on_key(s: &mut Screen, key: &rune::KeyEvent, _tx: &mut Tx) -> bool {
    if s.role == Role::Audience && s.number == 0 && matches!(key.key, rune::Key::Escape) {
        hide_projector_window();
        return true;
    }
    false
}

/// Wakes every window, for the threads that cannot name `shared()` in a closure.
fn wake_all() {
    shared().touch();
}

fn apply(message: ToOutput) {
    let shared = shared();
    match message {
        ToOutput::State(state) => {
            trace(|| {
                let slide = state.slide.as_ref().map(|slide| slide.label.clone()).unwrap_or_else(|| "none".to_string());
                format!("state: slide {slide}, black {}, look {}", state.black, state.look.name)
            });
            #[cfg(feature = "video")]
            video::want(match &state.look.background {
                Background::Video(path) => video::Want::File(path.clone()),
                Background::Camera(name) => video::Want::Camera(name.clone()),
                _ => video::Want::Nothing,
            });
            #[cfg(feature = "video")]
            items::want(state.media.clone(), state.music.clone());
            #[cfg(not(feature = "video"))]
            if state.media.is_some() || state.music.is_some() {
                shared.tell(&FromOutput::MediaError("item-none".to_string()));
            }
            #[cfg(not(feature = "video"))]
            if state.look.is_moving() {
                // A kind, worded by the operator in its own language (`why.rs` `media_problem`).
                shared.tell(&FromOutput::MediaError("video-none".to_string()));
            }
            shared.set_state(*state);
        }
        ToOutput::Identify(on) => {
            shared.identify.store(on, Ordering::Release);
            shared.touch();
        }
        ToOutput::Place { audience, stage, stage_window, here, off } => {
            // The operator numbers screens as the last Hello listed them.
            let list = shared.layout.lock().map(|layout| layout.0.clone()).unwrap_or_default();
            let named = |index: Option<usize>| index.and_then(|index| list.get(index)).map(|disp| disp.name.clone());
            trace(|| format!("place: projector {audience:?}, stage {stage:?}, stage window {stage_window}, here {here}, off {off}"));
            let pins = displays::Pins { audience: named(audience), stage: named(stage), stage_window, here, off };
            displays::set_pins(pins, wake_all);
        }
        ToOutput::Quit => {
            trace(|| "asked to quit".to_string());
            std::process::exit(0)
        }
    }
}

/// Serves one operator at a time. The newest connection wins; when it drops, the screens
/// keep their last state and wait for the next one.
fn listen(listener: TcpListener) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let _ = stream.set_nodelay(true);
        let shared = shared();
        let Ok(reader) = stream.try_clone() else { continue };
        if let Ok(mut client) = shared.client.lock() {
            *client = Some(stream);
        }
        trace(|| "an operator connected".to_string());
        let hello = shared.hello.lock().map(|hello| hello.clone()).ok();
        if let Some(hello) = hello {
            shared.tell(&hello);
        }
        #[cfg(feature = "video")]
        video::operator_connected(true);
        std::thread::spawn(move || {
            for line in BufReader::new(reader).lines() {
                let Ok(line) = line else { break };
                if let Some(message) = decode::<ToOutput>(&line) {
                    apply(message);
                }
            }
            #[cfg(feature = "video")]
            video::operator_connected(false);
        });
    }
}

type Scene = (&'static str, fn(&mut Screen));

const SCENES: &[Scene] = &[
    ("lyrics", |_| shared().set_state(demo::lyrics())),
    ("lyrics-two-languages", |_| shared().set_state(demo::two_languages())),
    ("languages-two", |_| shared().set_state(demo::languages(&["id", "en"], "Midnight", false))),
    ("languages-three", |_| shared().set_state(demo::languages(&["id", "en", "zh-Hans"], "Midnight", false))),
    ("languages-three-light", |_| shared().set_state(demo::languages(&["id", "en", "zh-Hans"], "Paper", false))),
    ("languages-split", |_| shared().set_state(demo::languages(&["id", "en", "zh-Hans"], "Midnight", true))),
    ("languages-one", |_| shared().set_state(demo::languages(&["id"], "Midnight", false))),
    ("scripture", |_| shared().set_state(demo::scripture())),
    ("announcement", |_| shared().set_state(demo::announcement())),
    // The state in the JSON file `--state-file` names, as `examples/notice_matrix.rs` writes it.
    ("state-file", |_| shared().set_state(flag_value("--state-file").and_then(|path| std::fs::read_to_string(path).ok()).and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default())),
    // The announcement slides in three styles, for the owner to choose from.
    ("notice-hairline-schedule", |_| shared().set_state(demo::notice_scene(NoticeStyle::Hairline, NoticeKind::Schedule))),
    ("notice-hairline-birthdays", |_| shared().set_state(demo::notice_scene(NoticeStyle::Hairline, NoticeKind::Birthdays))),
    ("notice-hairline-money", |_| shared().set_state(demo::notice_scene(NoticeStyle::Hairline, NoticeKind::Money))),
    ("notice-hairline-account", |_| shared().set_state(demo::notice_scene(NoticeStyle::Hairline, NoticeKind::Account))),
    ("notice-hairline-plain", |_| shared().set_state(demo::notice_scene(NoticeStyle::Hairline, NoticeKind::Plain))),
    ("notice-ledger-schedule", |_| shared().set_state(demo::notice_scene(NoticeStyle::Ledger, NoticeKind::Schedule))),
    ("notice-ledger-birthdays", |_| shared().set_state(demo::notice_scene(NoticeStyle::Ledger, NoticeKind::Birthdays))),
    ("notice-ledger-money", |_| shared().set_state(demo::notice_scene(NoticeStyle::Ledger, NoticeKind::Money))),
    ("notice-ledger-account", |_| shared().set_state(demo::notice_scene(NoticeStyle::Ledger, NoticeKind::Account))),
    ("notice-ledger-plain", |_| shared().set_state(demo::notice_scene(NoticeStyle::Ledger, NoticeKind::Plain))),
    ("notice-band-schedule", |_| shared().set_state(demo::notice_scene(NoticeStyle::Band, NoticeKind::Schedule))),
    ("notice-band-birthdays", |_| shared().set_state(demo::notice_scene(NoticeStyle::Band, NoticeKind::Birthdays))),
    ("notice-hairline-birthdays-marked", |_| shared().set_state(demo::notice_marked_scene(NoticeStyle::Hairline))),
    ("notice-ledger-birthdays-marked", |_| shared().set_state(demo::notice_marked_scene(NoticeStyle::Ledger))),
    ("notice-band-birthdays-marked", |_| shared().set_state(demo::notice_marked_scene(NoticeStyle::Band))),
    ("notice-band-money", |_| shared().set_state(demo::notice_scene(NoticeStyle::Band, NoticeKind::Money))),
    ("notice-band-account", |_| shared().set_state(demo::notice_scene(NoticeStyle::Band, NoticeKind::Account))),
    ("notice-band-plain", |_| shared().set_state(demo::notice_scene(NoticeStyle::Band, NoticeKind::Plain))),
    ("notice-style-greige-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Schedule, true))),
    ("notice-style-greige-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Birthdays, true))),
    ("notice-style-greige-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Money, true))),
    ("notice-style-greige-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Account, true))),
    ("notice-style-greige-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Verse, true))),
    ("notice-style-greige-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Plain, true))),
    ("notice-style-greige-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Plain, false))),
    ("notice-style-greige-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::Greige, NoticeKind::Account, false))),
    ("notice-style-pill-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Schedule, true))),
    ("notice-style-pill-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Birthdays, true))),
    ("notice-style-pill-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Money, true))),
    ("notice-style-pill-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Account, true))),
    ("notice-style-pill-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Verse, true))),
    ("notice-style-pill-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Plain, true))),
    ("notice-style-pill-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Plain, false))),
    ("notice-style-pill-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::Pill, NoticeKind::Account, false))),
    ("notice-style-neon-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Schedule, true))),
    ("notice-style-neon-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Birthdays, true))),
    ("notice-style-neon-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Money, true))),
    ("notice-style-neon-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Account, true))),
    ("notice-style-neon-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Verse, true))),
    ("notice-style-neon-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Plain, true))),
    ("notice-style-neon-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Plain, false))),
    ("notice-style-neon-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::Neon, NoticeKind::Account, false))),
    ("notice-style-split-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Schedule, true))),
    ("notice-style-split-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Birthdays, true))),
    ("notice-style-split-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Money, true))),
    ("notice-style-split-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Account, true))),
    ("notice-style-split-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Verse, true))),
    ("notice-style-split-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Plain, true))),
    ("notice-style-split-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Plain, false))),
    ("notice-style-split-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::Split, NoticeKind::Account, false))),
    ("notice-style-simple-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Schedule, true))),
    ("notice-style-simple-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Birthdays, true))),
    ("notice-style-simple-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Money, true))),
    ("notice-style-simple-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Account, true))),
    ("notice-style-simple-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Verse, true))),
    ("notice-style-simple-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Plain, true))),
    ("notice-style-simple-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Plain, false))),
    ("notice-style-simple-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::Simple, NoticeKind::Account, false))),
    ("notice-style-greige-birthdays-c1", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Greige, 1))),
    ("notice-style-greige-birthdays-c2", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Greige, 2))),
    ("notice-style-greige-birthdays-c3", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Greige, 3))),
    ("notice-style-greige-birthdays-c4", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Greige, 4))),
    ("notice-style-pill-birthdays-c1", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Pill, 1))),
    ("notice-style-pill-birthdays-c2", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Pill, 2))),
    ("notice-style-pill-birthdays-c3", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Pill, 3))),
    ("notice-style-pill-birthdays-c4", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Pill, 4))),
    ("notice-style-neon-birthdays-c1", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Neon, 1))),
    ("notice-style-neon-birthdays-c2", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Neon, 2))),
    ("notice-style-neon-birthdays-c3", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Neon, 3))),
    ("notice-style-neon-birthdays-c4", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Neon, 4))),
    ("notice-style-split-birthdays-c1", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Split, 1))),
    ("notice-style-split-birthdays-c2", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Split, 2))),
    ("notice-style-split-birthdays-c3", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Split, 3))),
    ("notice-style-split-birthdays-c4", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Split, 4))),
    ("notice-style-simple-birthdays-c1", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Simple, 1))),
    ("notice-style-simple-birthdays-c2", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Simple, 2))),
    ("notice-style-simple-birthdays-c3", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Simple, 3))),
    ("notice-style-simple-birthdays-c4", |_| shared().set_state(demo::colourway_scene(NoticeStyle::Simple, 4))),
    ("notice-style-simpledark-schedule", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Schedule, true))),
    ("notice-style-simpledark-birthdays", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Birthdays, true))),
    ("notice-style-simpledark-money", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Money, true))),
    ("notice-style-simpledark-account", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Account, true))),
    ("notice-style-simpledark-verse", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Verse, true))),
    ("notice-style-simpledark-plain", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Plain, true))),
    ("notice-style-simpledark-plain-nopic", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Plain, false))),
    ("notice-style-simpledark-account-noqris", |_| shared().set_state(demo::poster_scene(NoticeStyle::SimpleDark, NoticeKind::Account, false))),
    ("notice-style-greige-schedule-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Greige, NoticeKind::Schedule))),
    ("notice-style-greige-birthdays-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Greige, NoticeKind::Birthdays))),
    ("notice-style-pill-schedule-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Pill, NoticeKind::Schedule))),
    ("notice-style-pill-birthdays-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Pill, NoticeKind::Birthdays))),
    ("notice-style-neon-schedule-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Neon, NoticeKind::Schedule))),
    ("notice-style-neon-birthdays-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Neon, NoticeKind::Birthdays))),
    ("notice-style-split-schedule-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Split, NoticeKind::Schedule))),
    ("notice-style-split-birthdays-little", |_| shared().set_state(demo::poster_little_scene(NoticeStyle::Split, NoticeKind::Birthdays))),
    ("notice-style-greige-schedule-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Greige, NoticeKind::Schedule))),
    ("notice-style-greige-money-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Greige, NoticeKind::Money))),
    ("notice-style-pill-schedule-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Pill, NoticeKind::Schedule))),
    ("notice-style-pill-money-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Pill, NoticeKind::Money))),
    ("notice-style-neon-schedule-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Neon, NoticeKind::Schedule))),
    ("notice-style-neon-money-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Neon, NoticeKind::Money))),
    ("notice-style-split-schedule-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Split, NoticeKind::Schedule))),
    ("notice-style-split-money-long", |_| shared().set_state(demo::poster_long_scene(NoticeStyle::Split, NoticeKind::Money))),
    ("notice-sheet-hairline", |_| shared().set_state(demo::show_sheet(NoticeStyle::Hairline))),
    ("notice-sheet-ledger", |_| shared().set_state(demo::show_sheet(NoticeStyle::Ledger))),
    ("notice-sheet-band", |_| shared().set_state(demo::show_sheet(NoticeStyle::Band))),
    ("countdown", |_| shared().set_state(demo::countdown(unix_now()))),
    ("message", |_| shared().set_state(demo::message())),
    ("classic", |_| shared().set_state(demo::classic())),
    ("paper", |_| shared().set_state(demo::paper())),
    ("black", |_| shared().set_state(demo::black())),
    ("styled", |_| shared().set_state(demo::styled())),
    ("outlined", |_| shared().set_state(demo::outlined())),
    ("missing-font", |_| shared().set_state(demo::missing_font())),
    ("identify", |_| {
        shared().set_state(demo::lyrics());
        shared().identify.store(true, Ordering::Release);
    }),
];

fn spec(title: &'static str) -> AppSpec<Screen> {
    AppSpec {
        min_size: Some((320.0, 180.0)),
        scenes: SCENES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        fonts: artosia_slides::fonts::bundled(),
        ..AppSpec::new(title, (960.0, 540.0), || Screen::new(Role::Audience, 0), view)
    }
}

fn fullscreen(display: &displays::Disp) -> WindowOptions {
    WindowOptions {
        borderless: true,
        always_on_top: true,
        resizable: false,
        position: Some(display.position),
        physical_size: Some(display.size),
        min_size: Some((0.0, 0.0)),
        ..WindowOptions::default()
    }
}

/// A window for `role` at `spot`. `moved` marks one opened after start, which has no handle yet
/// for the placer to adopt. Every window starts hidden: the placer fits it to its screen, takes
/// it off the taskbar and shows it. The projector draws through the warp compositor.
pub(crate) fn make_spec(role: Role, key: String, spot: &displays::Spot, moved: bool) -> WindowSpec<Screen> {
    let (title, number, options) = match spot {
        displays::Spot::Window => (None, 0, WindowOptions { visible: false, ..WindowOptions::default() }),
        displays::Spot::Screen { number, disp } => (None::<()>, *number, WindowOptions { visible: false, ..fullscreen(disp) }),
    };
    let _ = title;
    let base = match role {
        Role::Audience => {
            let app = rune::App::new(spec("Artosia projector")).compositor(artosia_warp::WarpCompositor::new(warp().clone()));
            WindowSpec::from_app(key, app)
        }
        Role::Stage => WindowSpec::new(key, spec("Artosia stage")),
        Role::Anchor => WindowSpec::from_app(key, rune::App::new(spec("Artosia output")).windowless()),
    };
    let base = base.state(Screen::new(role, number)).options(options);
    if moved { base } else { base.on_created(move |handle| placer::adopt(role, handle)) }
}

/// Stores the screens and where the windows are, and tells the operator.
pub(crate) fn set_layout(list: Vec<displays::Disp>, audience: Option<usize>, stage: Option<usize>) {
    if let Ok(mut layout) = shared().layout.lock() {
        *layout = (list, audience, stage);
    }
    send_hello();
}

/// Builds `Hello` from the screens and cameras known now, keeps it for the next operator and
/// sends it to the one connected.
pub(crate) fn send_hello() {
    let shared = shared();
    let Ok(layout) = shared.layout.lock().map(|layout| layout.clone()) else { return };
    let cameras = shared.cameras.lock().map(|cameras| cameras.clone()).unwrap_or_default();
    let (list, audience, stage) = layout;
    // Before the screens are known there is nothing true to say; `begin` sends the first Hello.
    if list.is_empty() {
        return;
    }
    let hello = FromOutput::Hello {
        displays: list.iter().map(|disp| DisplayInfo { name: disp.name.clone(), width: disp.size.0, height: disp.size.1, primary: disp.primary, scale: disp.scale }).collect(),
        audience,
        stage,
        video: cfg!(feature = "video"),
        cameras,
    };
    trace(|| match &hello {
        FromOutput::Hello { displays, audience, stage, cameras, .. } => {
            format!("hello: {} screens, projector {audience:?}, stage {stage:?}, cameras {cameras:?}", displays.len())
        }
        _ => String::new(),
    });
    if let Ok(mut kept) = shared.hello.lock() {
        *kept = hello.clone();
    }
    shared.tell(&hello);
}

/// Binds the operator port. A copy started by `install_panic_restart` waits a few seconds for
/// the one that is dying to let go; any other copy gives up at once, because the first one owns
/// the screens.
fn bind(port: u16) -> Option<TcpListener> {
    let patience = if has_flag("--takeover") { 100 } else { 1 };
    for attempt in 0..patience {
        if let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
            trace(|| format!("port {port} bound after {attempt} retries"));
            return Some(listener);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    None
}

/// Rune (wgpu) panics in a few situations a window cannot avoid, such as a surface that is
/// reconfigured while the GPU is busy. A panic on the main thread starts a fresh copy that takes
/// over the port, so the operator reconnects and sends its state again instead of the room
/// staying dark until someone notices. Three restarts in a row at most, then it gives up.
fn install_panic_restart() {
    let started = std::time::Instant::now();
    let restarts = flag_value("--restarts").and_then(|n| n.parse::<u32>().ok()).unwrap_or(0);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        if std::thread::current().name() != Some("main") {
            return;
        }
        // A copy that ran for two minutes before failing starts the count again.
        let so_far = if started.elapsed().as_secs() > 120 { 0 } else { restarts };
        trace(|| format!("panic on the main thread after {} s, restart {} of 3: {info}", started.elapsed().as_secs(), so_far + 1));
        if so_far >= 3 {
            return;
        }
        let Ok(exe) = std::env::current_exe() else { return };
        let mut args: Vec<String> = Vec::new();
        let mut skip = false;
        for arg in std::env::args().skip(1) {
            match (skip, arg.as_str()) {
                (true, _) => skip = false,
                (false, "--restarts") => skip = true,
                (false, "--takeover" | "--demo-panic") => skip = arg == "--demo-panic",
                _ => args.push(arg),
            }
        }
        args.extend(["--takeover".to_string(), "--restarts".to_string(), (so_far + 1).to_string()]);
        let _ = std::process::Command::new(exe).args(args).spawn();
    }));
}

fn from_rune(displays: &[rune::windows::Display]) -> Vec<displays::Disp> {
    displays
        .iter()
        .map(|display| displays::Disp { name: display.name.clone().unwrap_or_default(), position: display.position, size: display.size, primary: display.primary, scale: display.scale as f32 })
        .collect()
}

/// On a Wayland desktop the projector and stage windows go through the desktop's X11 server
/// (Xwayland) when it answers. Wayland lets no program place a window, so the projector could
/// not be put on its screen, kept above other windows or fitted to the screen there; an X11
/// program may ask for all three (mutter's `window-x11.c` allows a program's own position by
/// default), and the streaming program can then take the window's picture without the desktop
/// asking. The window library takes Wayland whenever these variables are set, so they are
/// cleared for this process. `ARTOSIA_WAYLAND=1` keeps the windows on Wayland.
#[cfg(target_os = "linux")]
fn prefer_x11() {
    let set = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    if !(set("WAYLAND_DISPLAY") || set("WAYLAND_SOCKET")) || !set("DISPLAY") || set("ARTOSIA_WAYLAND") {
        return;
    }
    // Asked on a thread with two seconds to answer: WSL's X display takes a connection and
    // never answers (the streaming program's `win.rs` asks the same way).
    let (answer, wait) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = answer.send(x11rb::connect(None).is_ok());
    });
    if wait.recv_timeout(std::time::Duration::from_secs(2)) == Ok(true) {
        // SAFETY: among the first lines of `main`; the one other thread has sent its answer and
        // reads no variable after that.
        unsafe {
            std::env::remove_var("WAYLAND_DISPLAY");
            std::env::remove_var("WAYLAND_SOCKET");
        }
        trace(|| "windows: through the desktop's X11 server".to_string());
    }
}

fn main() {
    // The operator asks whether this program can start at all: off Windows it links the
    // system's GStreamer, and without those libraries the loader ends it before this line.
    if std::env::args().any(|arg| arg == "--probe") {
        return;
    }
    let headless = rune::is_headless();
    #[cfg(target_os = "linux")]
    if !headless {
        prefer_x11();
    }
    process_start();
    if headless {
        // A headless run has no operator, so every window starts on a sample slide.
        shared().set_state(demo::lyrics());
    }
    let port = flag_value("--port").and_then(|port| port.parse::<u16>().ok()).unwrap_or(PORT);
    // A second copy has nothing to do: the first one already owns the screens.
    let listener = if headless {
        None
    } else {
        install_panic_restart();
        match bind(port) {
            Some(listener) => Some(listener),
            None => return,
        }
    };
    #[cfg(feature = "video")]
    if !headless {
        video::start();
        items::start();
    }
    if let Some(path) = flag_value("--demo-item") {
        // A rundown item on sample lyrics, with its end, level and a sound under it from flags.
        let end = match flag_value("--demo-end").as_deref() {
            Some("loop") => MediaEnd::Loop,
            Some("next") => MediaEnd::Next,
            _ => MediaEnd::Stop,
        };
        let kind = if has_flag("--demo-audio") { MediaKind::Audio } else { MediaKind::Video };
        let volume = flag_value("--demo-volume").and_then(|v| v.parse().ok()).unwrap_or(100);
        let item = |id, path: String, kind, end, volume| MediaItem { id, path, kind, end, volume };
        let mut state = demo::lyrics();
        state.media = Some(item(1, path, kind, end, volume));
        state.music = flag_value("--demo-music").map(|path| item(2, path, MediaKind::Audio, MediaEnd::Loop, flag_value("--demo-music-volume").and_then(|v| v.parse().ok()).unwrap_or(60)));
        if let Some(path) = flag_value("--demo-video") {
            state.look.background = Background::Video(path);
        }
        // `--demo-clear-after N` sends the same slide without the item after N seconds.
        if let Some(secs) = flag_value("--demo-clear-after").and_then(|secs| secs.parse::<u64>().ok()) {
            let mut cleared = state.clone();
            cleared.media = None;
            cleared.music = None;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(secs));
                apply(ToOutput::State(Box::new(cleared)));
            });
        }
        apply(ToOutput::State(Box::new(state)));
    }
    if let Some(path) = flag_value("--demo-video").filter(|_| !has_flag("--demo-item")) {
        let mut state = demo::lyrics();
        state.look.background = Background::Video(path);
        state.look.shadow = !has_flag("--demo-no-shadow");
        if has_flag("--demo-no-text") {
            state.slide = None;
        }
        apply(ToOutput::State(Box::new(state)));
    }
    #[cfg(feature = "video")]
    if has_flag("--demo-camera") {
        let mut state = demo::lyrics();
        state.look.background = Background::Camera(flag_value("--demo-camera-name").unwrap_or_default());
        state.look.shadow = !has_flag("--demo-no-shadow");
        apply(ToOutput::State(Box::new(state)));
    }
    // On Windows the screens can be listed now, before Rune starts, so the operator's Hello goes
    // out as soon as it connects and never waits for a window.
    let mut listener = listener;
    let mut plan = None;
    if !headless && let Some(start) = displays::enumerate() {
        plan = Some(begin(start, &mut listener));
    }
    rune::run_windows(move |rune_displays| {
        if headless {
            let mut windows = vec![WindowSpec::new("audience", spec("Artosia projector")).state(Screen::new(Role::Audience, 0))];
            windows.push(WindowSpec::new("stage", spec("Artosia stage")).state(Screen::new(Role::Stage, 0)));
            return windows;
        }
        let plan = plan.take().unwrap_or_else(|| begin(from_rune(rune_displays), &mut listener));
        trace(|| format!("Rune saw: {}", displays::describe(&from_rune(rune_displays))));
        let mut windows = vec![make_spec(Role::Anchor, "anchor".to_string(), &displays::Spot::Window, false)];
        if let Some(spot) = &plan.audience {
            windows.push(make_spec(Role::Audience, "audience".to_string(), spot, false));
        }
        if let Some(spot) = &plan.stage {
            windows.push(make_spec(Role::Stage, "stage".to_string(), spot, false));
        }
        windows
    });
}

/// Works out what the command line asks for, starts watching the screens, tells the operator
/// what there is, and starts serving the operator. Returns where the windows go.
fn begin(start: Vec<displays::Disp>, listener: &mut Option<TcpListener>) -> displays::Plan {
    trace(|| format!("start: {}", displays::describe(&start)));
    let named = |flag: &str| flag_value(flag).and_then(|index| index.parse::<usize>().ok()).and_then(|index| start.get(index)).map(|disp| disp.name.clone());
    let pins = displays::Pins {
        audience: flag_value("--audience-name").or_else(|| named("--audience")),
        stage: flag_value("--stage-name").or_else(|| named("--stage")),
        stage_window: has_flag("--stage-window"),
        here: has_flag("--here"),
        off: has_flag("--no-projector"),
    };
    let plan = displays::plan(&start, &pins);
    displays::set_pins(pins, || shared().touch());
    displays::watch(start.clone(), || shared().touch());
    set_layout(start, plan.audience_index, plan.stage_index);
    placer::init(&plan);
    if let Some(listener) = listener.take() {
        std::thread::spawn(move || listen(listener));
    }
    plan
}
