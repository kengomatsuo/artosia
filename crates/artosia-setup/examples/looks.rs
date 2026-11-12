//! SETUPUI1: four directions for the window of Artosia's setup, drawn with Rune and rendered
//! headless, one picture per screen. Each direction copies one real installer's language
//! (research folder `~/Shared/inspo/artosia-installer`, `directions.md`).
//!
//! `cargo run -p artosia-setup --example looks -- --look opera --screen S02 --snap out.png [--light|--dark]`
//! looks: opera, firefox, epic, vscode. screens: S01 start, S02 progress, S03 failure, S04 done,
//! S05 remove, S06 close first. `--lang id` renders the Indonesian table (new strings stay
//! English), `--fail offline|damaged|disk` picks the failure sentence, `--contrast` prints the
//! contrast of every text pair and exits. Nothing here is shipped: it is an example.

use artosia_setup::strings::{Key, Language, Strings};
use rune::color::hex;
use rune::{
    AppSpec, Align, Color, Cx, El, Icon, Image, ImageCache, Shape, SurfaceFit, TextStyle, Titlebar, Waker, WindowOptions,
    canvas, col, el, icon, oklch, row, spacer, text,
};

// ---------------------------------------------------------------- the choice

#[derive(Clone, Copy, PartialEq, Debug)]
enum Look {
    Opera,
    Firefox,
    Epic,
    Vscode,
}

impl Look {
    const ALL: [Look; 4] = [Look::Opera, Look::Firefox, Look::Epic, Look::Vscode];

    fn parse(name: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|look| look.name() == name)
    }

    fn name(self) -> &'static str {
        match self {
            Look::Opera => "opera",
            Look::Firefox => "firefox",
            Look::Epic => "epic",
            Look::Vscode => "vscode",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Screen {
    Start,
    Progress,
    Failure,
    Done,
    Remove,
    CloseFirst,
}

impl Screen {
    const ALL: [Screen; 6] = [Screen::Start, Screen::Progress, Screen::Failure, Screen::Done, Screen::Remove, Screen::CloseFirst];

    fn parse(id: &str) -> Option<Screen> {
        Screen::ALL.into_iter().enumerate().find(|(at, _)| format!("S0{}", at + 1) == id).map(|(_, screen)| screen)
    }
}

/// The window of one look and screen, in points: the size the source product's window has.
/// Firefox's uninstall and already-running windows are smaller than its stub; Epic's download
/// splash is its own borderless window; the others keep one size.
fn window_size(look: Look, screen: Screen) -> (f32, f32) {
    match (look, screen) {
        (Look::Opera, _) => (700.0, 440.0),
        (Look::Firefox, Screen::Remove) => (513.0, 399.0),
        (Look::Firefox, Screen::CloseFirst) => (430.0, 190.0),
        (Look::Firefox, _) => (784.0, 506.0),
        (Look::Epic, Screen::Progress | Screen::Done) => (770.0, 430.0),
        (Look::Epic, _) => (615.0, 485.0),
        (Look::Vscode, _) => (600.0, 470.0),
    }
}

/// Windows with no title bar of the system: Opera's whole window and Epic's download splash.
fn borderless(look: Look, screen: Screen) -> bool {
    look == Look::Opera || (look == Look::Epic && matches!(screen, Screen::Progress | Screen::Done))
}

const TITLE_BAR: f32 = 32.0;
/// The download the progress screen shows, in megabytes (the card's figures).
const DONE_MB: &str = "27";
const TOTAL_MB: &str = "161";
const DONE_FRACTION: f32 = 27.0 / 161.0;
const VERSION: &str = "0.1.14";

// ---------------------------------------------------------------- words

/// Texts the setup's table does not have yet. Proposed, English only; not added to `strings.rs`.
#[derive(Clone, Copy)]
enum New {
    Tagline,
    StartBody,
    Install,
    FeatureSongs,
    FeatureBible,
    FeaturePhone,
    FeatureProjector,
    WaitBody,
    ClickInstall,
    InstallTo,
    Version,
    DoneHeading,
    DoneBody,
    OpenArtosia,
    Finish,
}

impl New {
    fn english(self) -> &'static str {
        match self {
            New::Tagline => "Song words on the projector",
            New::StartBody => "Artosia installs for you alone and needs no administrator.",
            New::Install => "Install",
            New::FeatureSongs => "Songs found by any line",
            New::FeatureBible => "Bible verses by reference or by words",
            New::FeaturePhone => "A remote on your phone",
            New::FeatureProjector => "A projector that keeps running",
            New::WaitBody => "Keep this window open. Artosia starts when it is ready.",
            New::ClickInstall => "Click Install to continue.",
            New::InstallTo => "Install Artosia to:",
            New::Version => "Version",
            New::DoneHeading => "Artosia is installed",
            New::DoneBody => "Artosia is opening now.",
            New::OpenArtosia => "Open Artosia",
            New::Finish => "Finish",
        }
    }
}

struct Words {
    table: Strings,
}

impl Words {
    fn get(&self, key: Key) -> String {
        self.table.get(key).to_string()
    }
    fn proposed(&self, key: New) -> String {
        key.english().to_string()
    }
    fn downloading(&self) -> String {
        self.table.fill(Key::StepDownloading, &[("done", DONE_MB), ("total", TOTAL_MB)])
    }
}

// ---------------------------------------------------------------- colour (a rule: pure, tested)

fn c(hex_text: &str) -> Color {
    hex(hex_text).unwrap_or(Color::BLACK)
}

/// WCAG contrast ratio of two opaque colours.
fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (a.luminance(), b.luminance());
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

// The brand is black on light and white on dark (docs/brand.md, 2026-11-12), as `src/window.rs` draws it.
const INK: &str = "#111111";
const PAPER: &str = "#f4f4f4";

/// One look in one theme: every colour as sRGB hex so the contrast of each pair is computed from
/// the same text the picture is drawn from.
#[derive(Clone, Copy)]
struct Pal {
    page: &'static str,
    ink: &'static str,
    muted: &'static str,
    line: &'static str,
    track: &'static str,
    accent: &'static str,
    on_accent: &'static str,
    /// A button that is not the main one: fill, border, words.
    second: &'static str,
    second_line: &'static str,
    /// The strip under the page that holds the buttons, and the dialog surface.
    strip: &'static str,
    dialog: &'static str,
    /// The side panel (Opera, VS Code) or banner (Epic): two ends of its fill and the words on it.
    panel_a: &'static str,
    panel_b: &'static str,
    on_panel: &'static str,
    /// The system's title bar and its words.
    title_bg: &'static str,
    title_fg: &'static str,
}

impl Pal {
    /// Every text pair of the look: name, words, ground. Each must reach 4.5 to 1.
    fn pairs(&self) -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("ink on page", self.ink, self.page),
            ("muted on page", self.muted, self.page),
            ("button words on accent", self.on_accent, self.accent),
            ("ink on second button", self.ink, self.second),
            ("ink on strip", self.ink, self.strip),
            ("muted on strip", self.muted, self.strip),
            ("ink on dialog", self.ink, self.dialog),
            ("muted on dialog", self.muted, self.dialog),
            ("panel words on panel start", self.on_panel, self.panel_a),
            ("panel words on panel end", self.on_panel, self.panel_b),
            ("title bar words", self.title_fg, self.title_bg),
        ]
    }
}

fn palette(look: Look, dark: bool) -> Pal {
    // The brand's four colours (docs/brand.md) with warm neutrals between them. Red is the
    // projector's and appears nowhere; yellow is the failure mark.
    let base = if dark {
        Pal {
            page: "#1b1611",
            ink: PAPER,
            muted: "#c8bda9",
            line: "#3d352b",
            track: "#3a3127",
            accent: PAPER,
            on_accent: INK,
            second: "#2a241d",
            second_line: "#7a6f5e",
            strip: "#231d17",
            dialog: "#2a241d",
            panel_a: "#2a1a0d",
            panel_b: "#5a3414",
            on_panel: PAPER,
            title_bg: "#202020",
            title_fg: "#ffffff",
        }
    } else {
        Pal {
            page: "#ffffff",
            ink: INK,
            muted: "#5a5044",
            line: "#d9d0c0",
            track: "#e6dccb",
            accent: INK,
            on_accent: "#ffffff",
            second: "#ffffff",
            second_line: "#9a8f7c",
            strip: "#f4f0e8",
            dialog: "#ffffff",
            panel_a: "#3e220b",
            panel_b: "#7a4a1f",
            on_panel: PAPER,
            title_bg: "#f3f3f3",
            title_fg: "#1b1b1b",
        }
    };
    match (look, dark) {
        (Look::Opera, _) => base,
        (Look::Firefox, true) => Pal { page: "#0b0c0f", strip: "#241810", ..base },
        (Look::Firefox, false) => Pal { page: "#f7f2e8", strip: "#ffffff", ..base },
        (Look::Epic, true) => Pal { page: "#26211b", strip: "#26211b", panel_a: "#12100d", panel_b: "#12100d", ..base },
        (Look::Epic, false) => Pal { page: "#f0f0f0", strip: "#f0f0f0", panel_a: INK, panel_b: INK, ..base },
        (Look::Vscode, _) => Pal { panel_a: INK, panel_b: INK, ..base },
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
glyph!(ARROW_RIGHT, "a-arrow-right", r#"<path d="M2.5 8H13.5M9 3.5L13.5 8L9 12.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);

// ---------------------------------------------------------------- the brand's pictures

macro_rules! brand {
    ($file:literal) => {
        include_bytes!(concat!("../../../assets/brand/", $file)).as_slice()
    };
}

struct Art {
    mark: Image,
    mark_on_dark: Image,
    lockup: Image,
    lockup_on_dark: Image,
    wordmark: Image,
    wordmark_on_dark: Image,
    stacked_on_dark: Image,
    hero_light: Image,
    hero_dark: Image,
}

impl Art {
    fn load(cache: &ImageCache) -> Art {
        let svg = |bytes: &'static [u8]| cache.svg(bytes, 640.0, 2.0);
        let art = Art {
            mark: svg(brand!("artosia-mark.svg")),
            mark_on_dark: svg(brand!("artosia-mark-on-dark.svg")),
            lockup: svg(brand!("artosia-lockup-horizontal.svg")),
            lockup_on_dark: svg(brand!("artosia-lockup-horizontal-on-dark.svg")),
            wordmark: svg(brand!("artosia-wordmark.svg")),
            wordmark_on_dark: svg(brand!("artosia-wordmark-on-dark.svg")),
            stacked_on_dark: svg(brand!("artosia-lockup-stacked-on-dark.svg")),
            hero_light: cache.load(include_bytes!("../../../site/img/en/hero-0-light.png").as_slice()),
            hero_dark: cache.load(include_bytes!("../../../site/img/en/hero-0-dark.png").as_slice()),
        };
        // A headless frame is drawn once, so every picture is decoded before it.
        for image in [&art.mark, &art.mark_on_dark, &art.lockup, &art.lockup_on_dark, &art.wordmark, &art.wordmark_on_dark, &art.stacked_on_dark, &art.hero_light, &art.hero_dark] {
            let _ = image.wait();
        }
        art
    }
}

fn pic(image: &Image, width: f32) -> El {
    el().w(width).pixels(image.pixels())
}

// ---------------------------------------------------------------- state

struct State {
    look: Look,
    screen: Screen,
    dark: bool,
    words: Words,
    failure: Key,
    cache: Option<ImageCache>,
    art: Option<Art>,
}

fn flag(name: &str) -> Option<String> {
    let args = rune::extra_args();
    args.iter().position(|arg| arg == name).and_then(|at| args.get(at + 1)).cloned()
}

fn choice() -> (Look, Screen) {
    let look = flag("--look").and_then(|name| Look::parse(&name)).unwrap_or(Look::Opera);
    let screen = flag("--screen").and_then(|id| Screen::parse(&id)).unwrap_or(Screen::Start);
    (look, screen)
}

fn init() -> State {
    let (look, screen) = choice();
    let language = flag("--lang").and_then(|code| Language::named(&code)).unwrap_or(Language::ENGLISH);
    let failure = match flag("--fail").as_deref() {
        Some("damaged") => Key::FailedDamaged,
        Some("disk") => Key::FailedDisk,
        _ => Key::FailedOffline,
    };
    // Dark unless `--light`, as a headless Rune run is.
    let dark = !rune::extra_args().iter().any(|arg| arg == "--light");
    State { look, screen, dark, words: Words { table: Strings::new(language) }, failure, cache: None, art: None }
}

fn setup(s: &mut State, waker: Waker) {
    let cache = ImageCache::new(waker);
    s.art = Some(Art::load(&cache));
    s.cache = Some(cache);
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

fn btn(b: Btn) -> El {
    let mut e = el().role("button").label(b.label.clone()).w(b.w).h(b.h).radius(b.radius).fill(b.fill).center().focusable(true).child(t(b.label, b.size, b.weight, b.fg));
    if let Some((width, color)) = b.border {
        e = e.border(width, color);
    }
    e
}

fn primary(label: String, p: &Pal, w: f32, h: f32, radius: f32, size: f32) -> El {
    btn(Btn { label, fill: c(p.accent), fg: c(p.on_accent), border: None, w, h, radius, size, weight: 600.0 })
}

fn secondary(label: String, p: &Pal, w: f32, h: f32, radius: f32, size: f32) -> El {
    btn(Btn { label, fill: c(p.second), fg: c(p.ink), border: Some((1.0, c(p.second_line))), w, h, radius, size, weight: 500.0 })
}

/// The button the keyboard is on: a ring in the main colour (removal opens on Cancel).
fn focused(label: String, p: &Pal, w: f32, h: f32, radius: f32, size: f32) -> El {
    btn(Btn { label, fill: c(p.second), fg: c(p.ink), border: Some((2.0, c(p.accent))), w, h, radius, size, weight: 500.0 })
}

/// A bar: a track with an inset and a fill, `value` 0..1; `None` is the waiting bar, a segment.
fn bar(width: f32, height: f32, radius: f32, inset: f32, track: Color, fill: Color, value: Option<f32>) -> El {
    let inner = width - inset * 2.0;
    let fill_h = height - inset * 2.0;
    let segment = match value {
        Some(v) => el().w(inner * v).h(fill_h).radius((radius - inset).max(0.0)).fill(fill),
        None => el().w(inner * 0.3).h(fill_h).radius((radius - inset).max(0.0)).fill(fill).ml(inner * 0.35),
    };
    row().role("progressbar").w(width).h(height).radius(radius).fill(track).padding(inset).clip_soft(0.0).child(segment)
}

fn disk(size: f32, fill: Color, glyph: &'static Icon, fg: Color) -> El {
    el().size(size, size).radius(size / 2.0).fill(fill).center().child(icon(glyph, size * 0.5).color(fg).a11y_hidden())
}

/// The failure mark: a yellow disk with a cross.
fn failure_mark(size: f32) -> El {
    disk(size, warn(), &CLOSE, c(INK))
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

/// A soft round shape for a page's art: a big ellipse in a gradient, laid at an angle.
fn wave(left: f32, top: f32, w: f32, h: f32, turn: f32, from: Color, to: Color) -> El {
    el().absolute(left, top).size(w, h).radius(h / 2.0).linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, from), (1.0, to)]).rotate(turn)
}

fn checkbox(on: bool, label: String, p: &Pal) -> El {
    let boxed = if on {
        el().size(16.0, 16.0).radius(3.0).fill(c(p.accent)).center().child(icon(&CHECK, 12.0).color(c(p.on_accent)).a11y_hidden())
    } else {
        el().size(16.0, 16.0).radius(3.0).fill(c(p.second)).border(1.0, c(p.second_line))
    };
    row().role("checkbox").label(label.clone()).checked(on).gap(8.0).center_y().child(boxed).child(t(label, 13.0, 400.0, c(p.ink)))
}

/// The title bar the system draws, drawn by the app because Rune's `Titlebar` hands it over; the
/// window's own caption buttons sit over its trailing edge.
fn title_band(cx: &Cx<State>, s: &State, p: &Pal, label: &str) -> El {
    let a = art(s);
    let mark = if s.dark { &a.mark_on_dark } else { &a.mark };
    row()
        .w_full()
        .h(TITLE_BAR)
        .fill(c(p.title_bg))
        .center_y()
        .pl(12.0)
        .gap(8.0)
        .window_drag()
        .child(pic(mark, 16.0))
        .child(t(label, 12.0, 400.0, c(p.title_fg)))
        .child(spacer())
        .child(el().w(cx.chrome_insets().trailing))
}

/// `setup` runs before the first view, so the pictures are there.
#[allow(clippy::expect_used)]
fn art(s: &State) -> &Art {
    s.art.as_ref().expect("art loaded in setup")
}

// ---------------------------------------------------------------- the view

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let p = palette(s.look, s.dark);
    cx.caption_color(c(p.title_fg));
    cx.foreground(c(p.ink));
    match s.look {
        Look::Opera => opera(s, &p),
        Look::Firefox => firefox(s, cx, &p),
        Look::Epic => epic(s, cx, &p),
        Look::Vscode => vscode(s, cx, &p),
    }
}

// ================================================================ Opera: split panel and one button
// products/e-browsers-tools/24-opera-gfg-install.png, 25-opera-gfg-extracting.png
// 700 x 440, no title bar, one close cross, the brand at the top of the right half.

fn opera(s: &State, p: &Pal) -> El {
    let w = &s.words;
    let a = art(s);
    let lockup = if s.dark { &a.lockup_on_dark } else { &a.lockup };
    let cross = el().absolute(700.0 - 38.0, 12.0).size(26.0, 26.0).center().role("button").label(w.get(Key::Close)).child(icon(&CLOSE, 14.0).color(c(p.muted)));
    let page = |body: El| el().w_full().h_full().fill(c(p.page)).clip_soft(0.0).child(body).child(cross);

    match s.screen {
        Screen::Start => {
            let features = [(&SONG, New::FeatureSongs), (&BOOK, New::FeatureBible), (&PHONE, New::FeaturePhone), (&SCREEN, New::FeatureProjector)];
            let mut list = col().gap(30.0).pl(36.0);
            for (glyph, key) in features {
                list = list.child(row().gap(16.0).center_y().child(icon(glyph, 26.0).color(c(p.on_panel))).child(para(w.proposed(key), 15.0, 400.0, c(p.on_panel), 190.0)));
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
                .child(para(w.proposed(New::Tagline), 22.0, 700.0, c(p.ink), 320.0))
                .child(el().h(12.0))
                .child(para(w.proposed(New::StartBody), 13.0, 400.0, c(p.muted), 320.0))
                .child(spacer())
                .child(row().justify_end().child(primary(w.proposed(New::Install), p, 122.0, 34.0, 3.0, 14.0)));
            page(row().w_full().h_full().child(left).child(right))
        }
        Screen::Progress => {
            let preview = if s.dark { &a.hero_dark } else { &a.hero_light };
            let words = col()
                .absolute(36.0, 214.0)
                .child(para(w.get(Key::InstallHeading), 22.0, 700.0, c(p.ink), 290.0))
                .child(el().h(10.0))
                .child(para(w.proposed(New::WaitBody), 14.0, 400.0, c(p.ink), 290.0));
            let status = row().absolute(36.0, 100.0).gap(14.0).child(t(w.downloading(), 14.0, 400.0, c(p.ink))).child(t("17%", 14.0, 400.0, c(p.ink)));
            let shot = el()
                .absolute(372.0, 200.0)
                .size(328.0, 240.0)
                .corners([8.0, 0.0, 0.0, 0.0])
                .clip_soft(0.0)
                .shadow(0.0, 16.0, Color::BLACK.alpha(0.25))
                .child(el().absolute(0.0, 0.0).size(512.0, 320.0).pixels_fit(preview.pixels(), SurfaceFit::Fill));
            let cancel = el().absolute(36.0, 396.0).role("button").label(w.get(Key::Cancel)).focusable(true).child(t(w.get(Key::Cancel), 13.0, 500.0, c(p.ink)));
            page(
                el()
                    .w_full()
                    .h_full()
                    .child(el().absolute(36.0, 34.0).child(pic(lockup, 120.0)))
                    .child(status)
                    .child(el().absolute(36.0, 132.0).child(bar(628.0, 17.0, 0.0, 0.0, c(p.track), c(p.accent), Some(DONE_FRACTION))))
                    .child(words)
                    .child(shot)
                    .child(cancel),
            )
        }
        Screen::Failure => page(
            el()
                .w_full()
                .h_full()
                .child(el().absolute(36.0, 34.0).child(pic(lockup, 120.0)))
                .child(
                    col()
                        .w_full()
                        .h_full()
                        .center()
                        .pt(30.0)
                        .gap(12.0)
                        .child(failure_mark(56.0))
                        .child(el().h(6.0))
                        .child(centered(w.get(Key::FailedHeading), 22.0, 700.0, c(p.ink), 440.0))
                        .child(centered(w.get(s.failure), 14.0, 400.0, c(p.muted), 440.0))
                        .child(el().h(14.0))
                        .child(row().gap(12.0).child(secondary(w.get(Key::Close), p, 140.0, 38.0, 4.0, 14.0)).child(primary(w.get(Key::TryAgain), p, 140.0, 38.0, 4.0, 14.0))),
                ),
        ),
        Screen::Done => page(
            el().w_full().h_full().child(el().absolute(36.0, 34.0).child(pic(lockup, 120.0))).child(
                col()
                    .w_full()
                    .h_full()
                    .center()
                    .pt(30.0)
                    .gap(12.0)
                    .child(disk(56.0, c(p.accent), &CHECK, c(p.on_accent)))
                    .child(el().h(6.0))
                    .child(centered(w.proposed(New::DoneHeading), 22.0, 700.0, c(p.ink), 440.0))
                    .child(centered(w.proposed(New::DoneBody), 14.0, 400.0, c(p.muted), 440.0)),
            ),
        ),
        Screen::Remove => page(
            el()
                .w_full()
                .h_full()
                .child(el().absolute(36.0, 34.0).child(pic(lockup, 120.0)))
                .child(col().absolute(36.0, 120.0).child(para(w.get(Key::RemoveHeading), 22.0, 700.0, c(p.ink), 540.0)).child(el().h(12.0)).child(para(w.get(Key::RemoveBody), 14.0, 400.0, c(p.muted), 540.0)))
                .child(el().absolute(700.0 - 36.0 - 292.0, 440.0 - 36.0 - 38.0).child(row().gap(12.0).child(focused(w.get(Key::Cancel), p, 140.0, 38.0, 4.0, 14.0)).child(primary(w.get(Key::RemoveYes), p, 140.0, 38.0, 4.0, 14.0)))),
        ),
        Screen::CloseFirst => {
            let ring = el().size(88.0, 88.0).center().child(el().absolute(0.0, 0.0).child(arc(88.0, 3.0, 1.0, c(p.line)))).child(el().absolute(0.0, 0.0).child(arc(88.0, 3.0, 0.25, c(p.accent)))).child(pic(if s.dark { &a.mark_on_dark } else { &a.mark }, 38.0));
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
                        .child(secondary(w.get(Key::Cancel), p, 140.0, 38.0, 4.0, 14.0)),
                ),
            )
        }
    }
}

// ================================================================ Firefox: dark page, huge title
// products/e-browsers-tools/16-firefox-stub-welcome.png, 05-firefox-stub-timeout.png,
// 04-firefox-stub-error.png, 14-firefox-uninstall-refresh-official.png, 15-firefox-stub-already-running.png,
// github/code/firefox-installing_page.css (label 40, bar 60 percent wide and 24 high, radius 8, inset 3, line 20).

/// The stub's soft wave in the lower right, in greys.
fn firefox_waves(p: &Pal, dark: bool, w: f32, h: f32) -> El {
    let (a, b, d) = if dark { (c("#8c8c8c").alpha(0.55), c("#2e2e2e").alpha(0.9), c("#0b0c0f").alpha(0.9)) } else { (c("#e4e4e4"), c("#d0d0d0"), c("#ececec")) };
    let _ = p;
    el().absolute(0.0, 0.0).size(w, h).clip_soft(0.0).child(wave(w * 0.30, h * 0.72, w * 1.1, h * 0.55, -0.12, d, b)).child(wave(w * 0.52, h * 0.84, w * 0.9, h * 0.45, -0.2, b, a))
}

fn firefox(s: &State, cx: &Cx<State>, p: &Pal) -> El {
    let w = &s.words;
    let a = art(s);
    let (width, height) = window_size(Look::Firefox, s.screen);
    let wordmark = if s.dark { &a.wordmark_on_dark } else { &a.wordmark };
    let lockup = if s.dark { &a.lockup_on_dark } else { &a.lockup };
    let mark = if s.dark { &a.mark_on_dark } else { &a.mark };
    let body_h = height - TITLE_BAR;
    // The body has the height under the title bar, so a strip pinned to its foot stays in the window.
    let window = |body: El| col().w_full().h_full().fill(c(p.page)).child(title_band(cx, s, p, &w.get(Key::Title))).child(el().w_full().h(body_h).clip_soft(0.0).child(body));
    // The brand bottom left and the slogan bottom right, as the stub draws them.
    let foot = |extra: El| {
        el().w_full().h_full().child(firefox_waves(p, s.dark, width, body_h)).child(extra).child(el().absolute(36.0, body_h - 74.0).child(pic(lockup, 168.0))).child(
            el().absolute(width - 36.0 - 260.0, body_h - 62.0).child(para(w.proposed(New::Tagline), 16.0, 400.0, c(p.ink), 260.0).align(Align::End)),
        )
    };
    // The page as the progress and done screens use it: one label, then what follows, centred.
    let stage = |label: String, line: El, below: El| {
        col()
            .absolute(0.0, 0.0)
            .w(width)
            .pt(86.0)
            .center_y()
            .child(centered(label, 40.0, 300.0, c(p.ink), width - 80.0))
            .child(el().h(20.0))
            .child(line)
            .child(el().h(20.0))
            .child(below)
    };
    let scrim = || el().absolute(0.0, 0.0).size(width, body_h).fill(Color::BLACK.alpha(if s.dark { 0.45 } else { 0.3 }));

    match s.screen {
        Screen::Start => {
            let top = row().absolute(36.0, 28.0).gap(30.0).child(pic(wordmark, 236.0)).child(col().gap(8.0).child(para(w.proposed(New::Tagline), 17.0, 600.0, c(p.ink), 330.0)).child(para(w.proposed(New::StartBody), 14.0, 400.0, c(p.muted), 330.0)));
            // The big mark cropped by the button strip, as the globe is.
            let art_mark = el().absolute(width / 2.0 - 190.0, 150.0).child(pic(mark, 380.0));
            let strip = row().absolute(0.0, body_h - 56.0).w(width).h(56.0).fill(c(p.strip)).center_y().justify_end().px(20.0).gap(10.0).child(primary(w.proposed(New::Install), p, 96.0, 30.0, 4.0, 13.0)).child(secondary(w.get(Key::Cancel), p, 96.0, 30.0, 4.0, 13.0));
            window(el().w_full().h_full().clip_soft(0.0).child(firefox_waves(p, s.dark, width, body_h)).child(top).child(art_mark).child(strip))
        }
        Screen::Progress => window(foot(stage(
            w.get(Key::InstallHeading),
            bar(width * 0.6, 30.0, 8.0, 3.0, c(p.track), c(p.accent), Some(DONE_FRACTION)),
            col().center_y().gap(14.0).child(centered(w.downloading(), 20.0, 400.0, c(p.ink), 560.0)).child(el().role("button").label(w.get(Key::Cancel)).focusable(true).child(t(w.get(Key::Cancel), 13.0, 500.0, c(p.muted)))),
        ))),
        Screen::Failure => {
            let dialog = col()
                .w(440.0)
                .radius(8.0)
                .fill(c(p.dialog))
                .clip_soft(0.0)
                .shadow(8.0, 32.0, Color::BLACK.alpha(0.5))
                .child(row().w_full().h(32.0).center_y().pl(12.0).gap(8.0).fill(c(p.title_bg)).child(pic(mark, 16.0)).child(t(w.get(Key::Title), 12.0, 400.0, c(p.title_fg))).child(spacer()).child(el().w(46.0).h(32.0).center().child(icon(&CLOSE, 12.0).color(c(p.title_fg)))))
                .child(row().px(24.0).py(28.0).gap(16.0).child(failure_mark(32.0)).child(para(w.get(s.failure), 13.0, 400.0, c(p.ink), 340.0)))
                .child(row().w_full().h(56.0).fill(c(p.strip)).center_y().justify_end().px(16.0).gap(8.0).child(primary(w.get(Key::TryAgain), p, 100.0, 30.0, 4.0, 13.0)).child(secondary(w.get(Key::Close), p, 100.0, 30.0, 4.0, 13.0)));
            let progress = foot(stage(
                w.get(Key::InstallHeading),
                bar(width * 0.6, 30.0, 8.0, 3.0, c(p.track), c(p.accent), Some(DONE_FRACTION)),
                centered(w.downloading(), 20.0, 400.0, c(p.ink), 560.0),
            ));
            window(el().w_full().h_full().child(progress).child(scrim()).child(col().absolute(0.0, 0.0).w(width).h(body_h).center().child(dialog)))
        }
        Screen::Done => window(foot(stage(w.proposed(New::DoneHeading), pic(mark, 84.0), centered(w.proposed(New::DoneBody), 20.0, 400.0, c(p.ink), 560.0)))),
        Screen::Remove => {
            // products/e-browsers-tools/14: an art panel on the left, a question, two short lines, the buttons at the foot.
            let panel = el().w(170.0).h_full().fill(c("#0b0c0f")).clip_soft(0.0).child(el().absolute(-120.0, 70.0).child(pic(&a.mark_on_dark, 400.0)));
            let right = col().grow(1.0).h_full().pl(20.0).pr(24.0).pt(56.0).gap(14.0).child(para(w.get(Key::RemoveHeading), 22.0, 700.0, c(p.ink), 290.0)).child(para(w.get(Key::RemoveBody), 13.0, 400.0, c(p.ink), 290.0));
            let strip = row().w_full().h(54.0).fill(c(p.strip)).center_y().justify_end().px(16.0).gap(8.0).child(primary(w.get(Key::RemoveYes), p, 96.0, 28.0, 4.0, 13.0)).child(focused(w.get(Key::Cancel), p, 96.0, 28.0, 4.0, 13.0));
            window(col().w_full().h_full().child(row().w_full().grow(1.0).child(panel).child(right)).child(strip))
        }
        Screen::CloseFirst => {
            // products/e-browsers-tools/15: a small native message, two short paragraphs, one button.
            let message = row().px(24.0).pt(26.0).gap(16.0).child(pic(mark, 32.0).self_start()).child(col().gap(10.0).child(para(w.get(Key::CloseFirstHeading), 13.0, 600.0, c(p.ink), 330.0)).child(para(w.get(Key::CloseFirstBody), 13.0, 400.0, c(p.ink), 330.0)));
            let strip = row().w_full().h(54.0).fill(c(p.strip)).center_y().justify_end().px(16.0).child(secondary(w.get(Key::Cancel), p, 88.0, 28.0, 4.0, 13.0));
            window(col().w_full().h_full().child(el().grow(1.0).child(message)).child(strip))
        }
    }
}

// ================================================================ Epic: dark banner wizard and splash
// products/d-launchers/01-epic-destination-install.png (615 x 485 wizard), 02-epic-setup-progress.png,
// 03-epic-downloading-update.png (770 x 430 borderless splash).

fn epic(s: &State, cx: &Cx<State>, p: &Pal) -> El {
    let w = &s.words;
    let a = art(s);
    let (width, height) = window_size(Look::Epic, s.screen);

    if borderless(Look::Epic, s.screen) {
        // The splash: always dark, as Epic's is; the mark large and centred.
        let ground = c("#1d1812");
        let ink = c(PAPER);
        let soft = c("#cfc4b0");
        let track = c("#3a3229");
        let inner = match s.screen {
            Screen::Progress => col()
                .w_full()
                .h_full()
                .center_y()
                .pt(78.0)
                .child(pic(&a.stacked_on_dark, 84.0))
                .child(el().h(54.0))
                .child(t(w.get(Key::InstallHeading), 15.0, 500.0, ink))
                .child(el().h(20.0))
                .child(bar(431.0, 25.0, 0.0, 0.0, track, ink, Some(DONE_FRACTION)))
                .child(el().h(28.0))
                .child(t(w.downloading(), 14.0, 400.0, soft))
                .child(el().h(20.0))
                .child(el().role("button").label(w.get(Key::Cancel)).focusable(true).child(t(w.get(Key::Cancel), 13.0, 500.0, soft))),
            _ => col()
                .w_full()
                .h_full()
                .center_y()
                .pt(78.0)
                .child(pic(&a.stacked_on_dark, 84.0))
                .child(el().h(40.0))
                .child(disk(40.0, ink, &CHECK, ground))
                .child(el().h(18.0))
                .child(t(w.proposed(New::DoneHeading), 18.0, 600.0, ink))
                .child(el().h(8.0))
                .child(t(w.proposed(New::DoneBody), 14.0, 400.0, soft)),
        };
        return el().w(width).h(height).fill(ground).radius(2.0).clip_soft(0.0).child(inner);
    }

    // The wizard: banner (title, one line, the mark at the right edge) over a quiet body.
    let (banner_title, banner_line) = match s.screen {
        Screen::Start => (w.proposed(New::Install) + " Artosia", w.proposed(New::ClickInstall)),
        Screen::Failure => (w.get(Key::FailedHeading), String::new()),
        Screen::Remove => (w.get(Key::RemoveHeading), String::new()),
        _ => (w.get(Key::CloseFirstHeading), String::new()),
    };
    let banner = row()
        .w_full()
        .h(74.0)
        .fill(c(p.panel_a))
        .center_y()
        .px(24.0)
        .child(col().gap(6.0).child(t(banner_title, 16.0, 700.0, c(p.on_panel))).child(if banner_line.is_empty() { el() } else { el().pl(16.0).child(t(banner_line, 12.0, 400.0, c(p.on_panel))) }))
        .child(spacer())
        .child(pic(&a.mark_on_dark, 36.0));
    let version = t(format!("{} {VERSION}", w.proposed(New::Version)), 12.0, 400.0, c(p.muted));
    let footer = |buttons: El| {
        col().w_full().child(el().w_full().h(1.0).fill(c(p.line))).child(row().w_full().h(66.0).center_y().px(16.0).child(version).child(spacer()).child(buttons))
    };
    let body = match s.screen {
        Screen::Start => col()
            .w_full()
            .grow(1.0)
            .px(32.0)
            .pt(36.0)
            .gap(16.0)
            .child(t(w.proposed(New::InstallTo), 13.0, 400.0, c(p.ink)))
            .child(row().w(520.0).h(30.0).radius(2.0).fill(c(p.line)).border(1.0, c(p.second_line)).center_y().px(8.0).child(t("%LOCALAPPDATA%\\Programs\\Artosia", 13.0, 400.0, c(p.muted))))
            .child(para(w.proposed(New::StartBody), 13.0, 400.0, c(p.muted), 520.0)),
        Screen::Failure => row().w_full().grow(1.0).px(32.0).pt(36.0).gap(18.0).child(failure_mark(40.0)).child(para(w.get(s.failure), 14.0, 400.0, c(p.ink), 480.0)),
        Screen::Remove => col().w_full().grow(1.0).px(32.0).pt(36.0).child(para(w.get(Key::RemoveBody), 14.0, 400.0, c(p.ink), 540.0)),
        _ => col().w_full().grow(1.0).px(32.0).pt(36.0).gap(26.0).child(para(w.get(Key::CloseFirstBody), 14.0, 400.0, c(p.ink), 540.0)).child(bar(540.0, 14.0, 0.0, 0.0, c(p.track), c(p.accent), None)),
    };
    let buttons = match s.screen {
        Screen::Start => row().gap(10.0).child(primary(w.proposed(New::Install), p, 92.0, 26.0, 4.0, 13.0)).child(secondary(w.get(Key::Cancel), p, 92.0, 26.0, 4.0, 13.0)),
        Screen::Failure => row().gap(10.0).child(secondary(w.get(Key::Close), p, 92.0, 26.0, 4.0, 13.0)).child(primary(w.get(Key::TryAgain), p, 92.0, 26.0, 4.0, 13.0)),
        Screen::Remove => row().gap(10.0).child(primary(w.get(Key::RemoveYes), p, 92.0, 26.0, 4.0, 13.0)).child(focused(w.get(Key::Cancel), p, 92.0, 26.0, 4.0, 13.0)),
        _ => row().child(secondary(w.get(Key::Cancel), p, 92.0, 26.0, 4.0, 13.0)),
    };
    col().w_full().h_full().fill(c(p.page)).child(title_band(cx, s, p, &w.get(Key::Title))).child(banner).child(body).child(footer(buttons))
}

// ================================================================ Visual Studio Code: white wizard with side panel
// products/c-editors/01-vscode-installing-extracting.png, 02-vscode-finish-launch.png,
// 05-vscode-close-first-modern.png, 06 and 07 (the modal), github/code/vscode-code.iss (WizardStyle=modern).

fn vscode(s: &State, cx: &Cx<State>, p: &Pal) -> El {
    let w = &s.words;
    let a = art(s);
    let small = if s.dark { &a.mark_on_dark } else { &a.mark };
    let window = |page: El, buttons: El| {
        col()
            .w_full()
            .h_full()
            .fill(c(p.page))
            .child(title_band(cx, s, p, &w.get(Key::Title)))
            .child(el().w_full().grow(1.0).child(page))
            .child(el().w_full().h(1.0).fill(c(p.line)))
            .child(row().w_full().h(56.0).fill(c(p.strip)).center_y().justify_end().px(16.0).gap(8.0).child(buttons))
    };
    // First and last pages: the full-height solid panel with the large mark, then the words.
    let panel_page = |heading: String, body: El| {
        row()
            .w_full()
            .h_full()
            .child(el().w(164.0).h_full().fill(c(p.panel_a)).child(el().absolute(27.0, 40.0).child(pic(&a.mark_on_dark, 110.0))))
            .child(col().grow(1.0).h_full().pl(20.0).pr(24.0).pt(24.0).gap(14.0).child(para(heading, 18.0, 700.0, c(p.ink), 380.0)).child(body))
    };
    // The other pages: a header (bold title, one line, the small mark at the right), then the body.
    let header_page = |title: String, line: Option<String>, body: El| {
        col()
            .w_full()
            .h_full()
            .child(
                row()
                    .w_full()
                    .h(60.0)
                    .center_y()
                    .px(20.0)
                    .child(col().gap(3.0).child(t(title, 14.0, 700.0, c(p.ink))).child(match line {
                        Some(line) => el().pl(16.0).child(t(line, 12.0, 400.0, c(p.muted))),
                        None => el(),
                    }))
                    .child(spacer())
                    .child(pic(small, 36.0)),
            )
            .child(el().w_full().h(1.0).fill(c(p.line)))
            .child(body)
    };

    match s.screen {
        Screen::Start => window(
            panel_page(
                w.proposed(New::Install) + " Artosia",
                col().gap(10.0).child(para(w.proposed(New::Tagline) + ".", 13.0, 400.0, c(p.ink), 380.0)).child(para(w.proposed(New::StartBody), 13.0, 400.0, c(p.muted), 380.0)).child(para(w.proposed(New::ClickInstall), 13.0, 400.0, c(p.ink), 380.0)),
            ),
            row().gap(8.0).child(primary(w.proposed(New::Install), p, 92.0, 30.0, 4.0, 13.0)).child(secondary(w.get(Key::Cancel), p, 92.0, 30.0, 4.0, 13.0)),
        ),
        Screen::Progress => window(
            header_page(
                w.get(Key::InstallHeading),
                Some(w.proposed(New::WaitBody)),
                col().w_full().px(20.0).pt(36.0).gap(14.0).child(t(w.downloading(), 13.0, 400.0, c(p.ink))).child(bar(560.0, 20.0, 2.0, 0.0, c(p.track), c(p.accent), Some(DONE_FRACTION))),
            ),
            secondary(w.get(Key::Cancel), p, 92.0, 30.0, 4.0, 13.0),
        ),
        Screen::Failure => {
            // The modal of products/c-editors/06 over the progress page: the mark, the sentence, the ways forward as arrow rows.
            let choice = |label: String| row().w_full().h(40.0).center_y().px(24.0).gap(12.0).child(icon(&ARROW_RIGHT, 16.0).color(c(p.accent))).child(t(label, 14.0, 600.0, c(p.ink)));
            let modal = col()
                .w(440.0)
                .radius(6.0)
                .fill(c(p.dialog))
                .border(1.0, c(p.line))
                .clip_soft(0.0)
                .shadow(8.0, 32.0, Color::BLACK.alpha(0.45))
                .child(el().px(16.0).py(8.0).child(t(w.get(Key::Title), 12.0, 400.0, c(p.muted))))
                .child(row().px(24.0).py(16.0).gap(16.0).child(failure_mark(32.0)).child(para(w.get(s.failure), 13.0, 400.0, c(p.ink), 340.0)))
                .child(col().w_full().pb(12.0).child(choice(w.get(Key::TryAgain))).child(choice(w.get(Key::Close))));
            let page = header_page(w.get(Key::InstallHeading), Some(w.proposed(New::WaitBody)), col().w_full().px(20.0).pt(36.0).gap(14.0).child(t(w.downloading(), 13.0, 400.0, c(p.ink))).child(bar(560.0, 20.0, 2.0, 0.0, c(p.track), c(p.accent), Some(DONE_FRACTION))));
            window(
                el().w_full().h_full().child(page).child(el().absolute(0.0, 0.0).w(600.0).h(470.0 - TITLE_BAR - 57.0).fill(Color::BLACK.alpha(if s.dark { 0.45 } else { 0.3 }))).child(col().absolute(0.0, 0.0).w(600.0).h(470.0 - TITLE_BAR - 57.0).center().child(modal)),
                secondary(w.get(Key::Cancel), p, 92.0, 30.0, 4.0, 13.0),
            )
        }
        Screen::Done => window(
            panel_page(
                w.proposed(New::DoneHeading),
                col().gap(14.0).child(para(w.proposed(New::DoneBody), 13.0, 400.0, c(p.ink), 380.0)).child(el().h(6.0)).child(checkbox(true, w.proposed(New::OpenArtosia), p)),
            ),
            primary(w.proposed(New::Finish), p, 92.0, 30.0, 4.0, 13.0),
        ),
        Screen::Remove => window(
            panel_page(w.get(Key::RemoveHeading), para(w.get(Key::RemoveBody), 13.0, 400.0, c(p.ink), 380.0)),
            row().gap(8.0).child(focused(w.get(Key::Cancel), p, 92.0, 30.0, 4.0, 13.0)).child(primary(w.get(Key::RemoveYes), p, 92.0, 30.0, 4.0, 13.0)),
        ),
        Screen::CloseFirst => {
            // products/c-editors/05: the running program named in a list.
            let listing = col().w(560.0).h(150.0).radius(3.0).fill(c(p.strip)).border(1.0, c(p.line)).padding(8.0).child(t("Artosia", 13.0, 400.0, c(p.ink)));
            window(
                header_page(
                    w.get(Key::CloseFirstHeading),
                    None,
                    col().w_full().px(20.0).pt(24.0).gap(16.0).child(row().gap(14.0).child(failure_mark(24.0)).child(para(w.get(Key::CloseFirstBody), 13.0, 400.0, c(p.ink), 510.0))).child(listing),
                ),
                secondary(w.get(Key::Cancel), p, 92.0, 30.0, 4.0, 13.0),
            )
        }
    }
}

// ---------------------------------------------------------------- main

fn print_contrast() -> bool {
    let mut all = true;
    for look in Look::ALL {
        for dark in [false, true] {
            for (name, fg, bg) in palette(look, dark).pairs() {
                let ratio = contrast(c(fg), c(bg));
                all &= ratio >= 4.5;
                println!("{:8} {:5} {:28} {} on {} {:5.2} {}", look.name(), if dark { "dark" } else { "light" }, name, fg, bg, ratio, if ratio >= 4.5 { "ok" } else { "FAIL" });
            }
        }
    }
    all
}

fn main() {
    if rune::extra_args().iter().any(|arg| arg == "--contrast") {
        std::process::exit(if print_contrast() { 0 } else { 1 });
    }
    let (look, screen) = choice();
    let size = window_size(look, screen);
    let spec = AppSpec { min_size: Some(size), setup: Some(setup), ..AppSpec::new("Artosia setup", size, init, view) };
    let options = WindowOptions { borderless: borderless(look, screen), resizable: false, min_size: Some(size), max_size: Some(size), ..WindowOptions::default() };
    let app = rune::App::new(spec).configured_options(options.into());
    let app = if borderless(look, screen) { app } else { app.titlebar(Titlebar::new(TITLE_BAR)) };
    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_matches_the_brand_page() {
        // The setup's ink on white 18.88, its paper on its ink 17.17.
        assert!((contrast(c(INK), c("#ffffff")) - 18.88).abs() < 0.02);
        assert!((contrast(c(PAPER), c(INK)) - 17.17).abs() < 0.02);
        assert!((contrast(c("#000000"), c("#ffffff")) - 21.0).abs() < 0.01);
    }

    #[test]
    fn every_text_and_button_pair_of_every_look_reads_at_four_and_a_half() {
        for look in Look::ALL {
            for dark in [false, true] {
                for (name, fg, bg) in palette(look, dark).pairs() {
                    let ratio = contrast(c(fg), c(bg));
                    assert!(ratio >= 4.5, "{} {} {name}: {fg} on {bg} is {ratio:.2}", look.name(), if dark { "dark" } else { "light" });
                }
            }
        }
    }

    #[test]
    fn a_screen_id_names_one_screen() {
        assert_eq!(Screen::parse("S01"), Some(Screen::Start));
        assert_eq!(Screen::parse("S06"), Some(Screen::CloseFirst));
        assert_eq!(Screen::parse("S07"), None);
    }
}
