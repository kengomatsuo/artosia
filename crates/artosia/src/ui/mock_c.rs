//! UIOVER1 direction C: Drawer and forms, after FreeShow. Sources per page: `~/Shared/inspo/artosia-ui-overhaul/directions.md`.
//! Drawn only by the `mock-c-p1` to `mock-c-p7` scenes; nothing here is reachable in the program.
//! Every word is hardcoded Indonesian, because this is a mock and never ships.

use std::f32::consts::PI;

use artosia_core::deck::{NoticeKind, Slide, SlideKind};
use artosia_slides::audience::{Detail, Media, audience};
use artosia_slides::frame::OutputState;
use artosia_slides::look::{Look, NoticeStyle, presets};
use artosia_slides::notice;
use rune::{Color, Cx, El, Icon, TextStyle, col, el, icon, row, spacer, text};
use rune_kit::{Kit, SegmentedControl, Switch};

use crate::state::App;
use crate::theme::{self, icons, space, type_};

use super::widgets::btn;

macro_rules! glyph {
    ($name:ident, $id:literal, $body:literal) => {
        static $name: Icon = Icon { name: $id, view: 16.0, body: $body };
    };
}

// Icons this mock needs and the window's own set lacks.
glyph!(CALENDAR, "c-calendar", r#"<rect x="2" y="3" width="12" height="11" rx="1.5" stroke="currentColor"/><path d="M2 6.5H14M5.5 1.8V4.2M10.5 1.8V4.2" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(CHEVRON_DOWN, "c-chevron-down", r#"<path d="M4 6L8 10L12 6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(CHEVRON_RIGHT, "c-chevron-right", r#"<path d="M6 4L10 8L6 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(CHEVRON_LEFT, "c-chevron-left", r#"<path d="M10 4L6 8L10 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(EYEDROPPER, "c-eyedropper", r#"<path d="M11.2 1.8C12 1 13.3 1 14.1 1.9C14.9 2.7 14.9 4 14.1 4.8L12.5 6.4L9.6 3.5L11.2 1.8Z" stroke="currentColor" stroke-linejoin="round"/><path d="M9.6 3.5L3 10.1V13H5.9L12.5 6.4" stroke="currentColor" stroke-linejoin="round"/>"#);
glyph!(VOLUME, "c-volume", r#"<path d="M2.5 6.5V9.5H5L8.5 12.5V3.5L5 6.5H2.5Z" stroke="currentColor" stroke-linejoin="round"/><path d="M11 5.5C12 6.3 12.5 7 12.5 8C12.5 9 12 9.7 11 10.5M12.8 3.5C14.3 4.7 15 6.2 15 8C15 9.8 14.3 11.3 12.8 12.5" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(FILM, "c-film", r#"<rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor"/><path d="M5 3V13M11 3V13M2 6H5M2 10H5M11 6H14M11 10H14" stroke="currentColor"/>"#);
glyph!(GRID, "c-grid", r#"<rect x="2" y="2" width="5" height="5" rx="1" stroke="currentColor"/><rect x="9" y="2" width="5" height="5" rx="1" stroke="currentColor"/><rect x="2" y="9" width="5" height="5" rx="1" stroke="currentColor"/><rect x="9" y="9" width="5" height="5" rx="1" stroke="currentColor"/>"#);
glyph!(SHEET, "c-sheet", r#"<rect x="2" y="3" width="12" height="10" rx="1.5" stroke="currentColor"/><path d="M2 6.5H14M2 9.8H14M6.5 3V13" stroke="currentColor"/>"#);
glyph!(FILE, "c-file", r#"<path d="M4 1.8H9.5L12.5 4.8V14.2H4V1.8Z" stroke="currentColor" stroke-linejoin="round"/><path d="M9.5 1.8V4.8H12.5" stroke="currentColor" stroke-linejoin="round"/>"#);
glyph!(LAYERS, "c-layers", r#"<path d="M8 2L14 5.5L8 9L2 5.5L8 2Z" stroke="currentColor" stroke-linejoin="round"/><path d="M2 8.5L8 12L14 8.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(SIGMA, "c-sigma", r#"<path d="M12 3.5H4L8 8L4 12.5H12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(BROOM, "c-clear", r#"<path d="M3 3L13 13M13 3L3 13" stroke="currentColor" stroke-linecap="round"/>"#);

const RIGHT_W: f32 = 320.0;

// ---------------------------------------------------------------------------------------------
// Small parts

fn tx(s: impl Into<String>, style: TextStyle, color: Color) -> El {
    text(s.into()).font(style).color(color)
}

const NUM: TextStyle = TextStyle::new(13.0, 500.0).lh(18.0).features("'tnum' 1");
const NUM_STRONG: TextStyle = TextStyle::new(13.0, 700.0).lh(18.0).features("'tnum' 1");

fn hex(s: &str) -> Color {
    rune::color::hex(s).unwrap_or(Color::BLACK)
}

/// A title strip over a column: FreeShow's "Service" bar.
fn strip(k: &Kit, title: &str) -> El {
    row().w_full().h(36.0).px(space::S3).center().fill(theme::surf().header).child(tx(title, type_::TITLE, k.c.text_base).truncate().min_w(0.0))
}

/// The one bottom action of a list or a drawer: FreeShow's "New section".
fn action_bar(k: &Kit, label: &str) -> El {
    row().w_full().h(34.0).gap(6.0).center().fill(theme::surf().header).child(icon(&icons::PLUS, 14.0).color(k.c.text_muted)).child(tx(label, type_::BODY_STRONG, k.c.text_base))
}

fn section(k: &Kit, label: &str) -> El {
    row().w_full().h(26.0).center().fill(theme::surf().chrome).child(tx(label, type_::SMALL_STRONG, k.c.text_base))
}

fn item(k: &Kit, ic: &'static Icon, label: &str, on: bool) -> El {
    let accent = theme::accent();
    let mut r = row()
        .w_full()
        .h(36.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .child(icon(ic, 16.0).color(if on { accent } else { k.c.text_muted }))
        .child(tx(label, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate().min_w(0.0).grow(1.0));
    if on {
        r = r.fill(accent.alpha(0.14)).border(1.5, accent);
    }
    r
}

fn chip(k: &Kit, label: &str, on: bool) -> El {
    let (fill, ink) = if on { (theme::accent(), Color::WHITE) } else { (k.c.layer_02, k.c.text_base) };
    let mut c = row().h(28.0).px(space::S3).center().radius(14.0).fill(fill).child(tx(label, type_::SMALL_STRONG, ink));
    if !on {
        c = c.border(1.0, k.c.border_base);
    }
    c
}

fn day(k: &Kit, label: &str, on: bool, d: f32) -> El {
    let mut e = el().size(d, d).radius(d / 2.0).center().fill(if on { theme::accent() } else { k.c.layer_02 });
    if !on {
        e = e.border(1.0, k.c.border_base);
    }
    e.child(tx(label, type_::SMALL_STRONG, if on { Color::WHITE } else { k.c.text_base }))
}

/// A boxed value with its label above it, as FreeShow's Material inputs are.
fn field(k: &Kit, label: &str, value: &str, trailing: Option<&'static Icon>) -> El {
    let mut v = row().w_full().gap(space::S2).center_y().child(tx(value, type_::BODY, k.c.text_base).truncate().min_w(0.0).grow(1.0));
    if let Some(i) = trailing {
        v = v.child(icon(i, 16.0).color(k.c.text_muted));
    }
    col().w_full().h(46.0).px(space::S3).justify_center().gap(1.0).radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(tx(label, type_::TINY, k.c.text_muted)).child(v)
}

fn label(k: &Kit, s: &str) -> El {
    tx(s, type_::SMALL_STRONG, k.c.text_muted)
}

fn slider(k: &Kit, v: f32) -> El {
    row()
        .h(18.0)
        .grow(1.0)
        .min_w(0.0)
        .center_y()
        .child(el().h(4.0).grow(v).radius(2.0).fill(theme::accent()))
        .child(el().size(16.0, 16.0).radius(8.0).fill(Color::WHITE).shadow(1.0, 4.0, Color::BLACK.alpha(0.4)))
        .child(el().h(4.0).grow(1.0 - v).radius(2.0).fill(k.c.layer_04))
}

fn switch(k: &Kit, on: bool, name: &str) -> El {
    Switch::new(on).label(name).hide_label().build(k)
}

fn seg(k: &Kit, labels: &[&str], at: usize) -> El {
    SegmentedControl::new(labels, Some(at)).label("pilihan").fit().build(k)
}

fn card(k: &Kit) -> El {
    col().radius(k.radius.lg).fill(k.c.layer_02).border(1.0, k.c.border_muted)
}

fn money(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Live slides: the program's own announcement code, scaled into a tile

fn names() -> Vec<(String, u32)> {
    const WORDS: [&str; 20] = ["Alfa", "Beta", "Gama", "Delta", "Epsilon", "Zeta", "Eta", "Teta", "Iota", "Kapa", "Lamda", "Mu", "Nu", "Ksi", "Omikron", "Pi", "Ro", "Sigma", "Tau", "Upsilon"];
    let wants = [2, 1, 2, 1, 2, 1];
    let mut next = 0;
    (0..30)
        .map(|at| {
            let (first, second) = (WORDS[next % WORDS.len()], WORDS[(next + 1) % WORDS.len()]);
            let words = if wants[at % wants.len()] == 2 && first.len() + second.len() <= 6 { vec![first, second] } else { vec![first] };
            next += words.len();
            (format!("Contoh {}", words.join(" ")), (at + 1 - usize::from(at % 6 == 3)) as u32)
        })
        .collect()
}

const GIFTS: [(&str, &str, u64); 9] = [
    ("Kel. Contoh Alfa", "Syukur", 1_500_000),
    ("Ibu Contoh Beta", "Persepuluhan", 500_000),
    ("Bpk. Contoh Gama", "Syukur", 100_000),
    ("Kel. Contoh Delta", "Pembangunan", 2_000_000),
    ("Kel. Contoh Zeta", "Persepuluhan", 3_000_000),
    ("Ibu Contoh Eta", "Syukur", 400_000),
    ("Ibu Contoh Teta dan Sdr. Contoh Iota", "Pembangunan", 500_000),
    ("Ibu Contoh Kapa", "Syukur", 250_000),
    ("Ibu Contoh Lamda", "Persepuluhan", 200_000),
];

const SCHEDULE: [(&str, &str, &str); 6] = [
    ("Minggu", "06.30", "Ibadah Raya 1"),
    ("", "09.00", "Ibadah Raya 2"),
    ("", "09.00", "Sekolah Minggu"),
    ("Rabu", "18.30", "Ibadah doa"),
    ("Jumat", "18.30", "Persekutuan pemuda"),
    ("Sabtu", "17.00", "Latihan pujian"),
];

fn notice_title(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Schedule => "Jadwal kegiatan gereja",
        NoticeKind::Birthdays => "Ulang tahun bulan November",
        NoticeKind::Money => "Persembahan syukur",
        NoticeKind::Account => "Persembahan lewat transfer",
        NoticeKind::Plain => "Ibadah Padang",
    }
}

fn notice_text(kind: NoticeKind) -> String {
    let lines: Vec<String> = match kind {
        NoticeKind::Schedule => SCHEDULE.iter().map(|(d, t, e)| format!("{d}\t{t}\t{e}")).collect(),
        NoticeKind::Birthdays => names().into_iter().map(|(n, d)| format!("{n}\t{d} November")).collect(),
        NoticeKind::Money => {
            let mut lines: Vec<String> = GIFTS.iter().map(|(n, _, v)| format!("{n}\tRp {}", money(*v))).collect();
            lines.push(format!("== Total\tRp {}", money(GIFTS.iter().map(|g| g.2).sum())));
            lines
        }
        NoticeKind::Account => ["Bank Contoh", "123 456 7890", "a.n. Gereja Contoh"].map(String::from).to_vec(),
        NoticeKind::Plain => ["Sabtu, 8 November 2026", "Jam 10.00"].map(String::from).to_vec(),
    };
    lines.join("\n")
}

/// One announcement drawn by the projector's own code, in a tile `width` wide.
fn slide_tile(kind: NoticeKind, style: NoticeStyle, width: f32) -> El {
    let (w, h) = (1920.0, 1080.0);
    let look = Look { notice_style: style, ..Look::default() };
    let state = OutputState { slide: None, look: look.clone(), ..OutputState::default() };
    let slide = Slide { kind: SlideKind::Announcement, notice: kind, label: notice_title(kind).to_string(), heading: notice_title(kind).to_string(), text: notice_text(kind), ..Slide::default() };
    let page = el().size(w, h).child(audience(&state, w, h, Media::default(), Detail::Preview, 0.0).absolute(0.0, 0.0)).child(notice::notice(&slide, &look, w, h));
    el().size(width, (width * 9.0 / 16.0).round()).radius(4.0).clip_rounded(4.0).child(page.absolute(0.0, 0.0).origin(0.0, 0.0).scale(width / w).inert())
}

/// A scene drawn over a look's own background, with a logo in the middle.
fn scene_tile(width: f32) -> El {
    let (w, h) = (1920.0, 1080.0);
    let look = presets().into_iter().nth(3).unwrap_or_default();
    let state = OutputState { slide: None, look, ..OutputState::default() };
    let page = el().size(w, h).child(audience(&state, w, h, Media::default(), Detail::Preview, 0.0).absolute(0.0, 0.0));
    let height = (width * 9.0 / 16.0).round();
    el()
        .size(width, height)
        .radius(4.0)
        .clip_rounded(4.0)
        .child(page.absolute(0.0, 0.0).origin(0.0, 0.0).scale(width / w).inert())
        .child(col().absolute(0.0, 0.0).size(width, height).center().gap(6.0).child(icon(&icons::LOGO, width * 0.16).color(Color::WHITE)).child(tx("Terima kasih", TextStyle::new(width * 0.06, 700.0), Color::WHITE)))
}

/// A tile in a drawer: the slide, and its name under it.
fn tile_cell(k: &Kit, kind: NoticeKind, style: NoticeStyle, width: f32, name: &str, on: bool) -> El {
    let mut t = el().radius(6.0).p_wrap();
    t = t.child(slide_tile(kind, style, width));
    if on {
        t = t.border(2.0, theme::accent());
    }
    col().gap(4.0).items_center().child(t).child(tx(name, if on { type_::SMALL_STRONG } else { type_::SMALL }, if on { k.c.text_base } else { k.c.text_muted }).truncate().max_w(width))
}

trait Pad {
    fn p_wrap(self) -> Self;
}
impl Pad for El {
    /// A tile's frame sits outside its picture, so a ring never covers a corner of the slide.
    fn p_wrap(self) -> Self {
        self.padding(2.0)
    }
}

// ---------------------------------------------------------------------------------------------
// Skeleton: three columns over a drawer with icon-and-word tabs

struct Drawer<'a> {
    tab: usize,
    height: f32,
    cats: &'a [(&'a str, &'a str)],
    cat_on: usize,
    action: &'a str,
    body: El,
}

fn drawer(k: &Kit, d: Drawer) -> El {
    let sf = theme::surf();
    let tabs: [(&'static Icon, &str); 6] = [(&icons::SONG, "Lagu"), (&icons::IMAGE, "Media"), (&VOLUME, "Audio"), (&GRID, "Templat"), (&CALENDAR, "Kalender"), (&icons::BOOK, "Alkitab")];
    let accent = theme::accent();
    let mut strip = row().w_full().h(36.0).px(space::S2).gap(2.0).center_y().fill(sf.chrome);
    for (at, (ic, name)) in tabs.into_iter().enumerate() {
        let on = at == d.tab;
        let mut tab = row().h(36.0).px(space::S3).gap(6.0).center().child(icon(ic, 16.0).color(if on { accent } else { k.c.text_muted })).child(tx(name, if on { type_::BODY_STRONG } else { type_::BODY }, if on { k.c.text_base } else { k.c.text_muted }));
        if on {
            tab = tab.fill(sf.work).border_sides([2.0, 0.0, 0.0, 0.0], accent);
        }
        strip = strip.child(tab);
    }
    strip = strip.child(spacer()).child(
        row().w(190.0).h(26.0).px(space::S2).gap(6.0).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(icon(&rune_kit::glyphs::SEARCH, 14.0).color(k.c.text_muted)).child(tx("Cari", type_::SMALL, k.c.text_muted)),
    );
    let mut cats = col().w_full().grow(1.0).min_h(0.0);
    for (at, (name, count)) in d.cats.iter().enumerate() {
        let on = at == d.cat_on;
        cats = cats.child(
            row()
                .w_full()
                .h(34.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .fill(if on { sf.work } else { Color::TRANSPARENT })
                .child(el().size(8.0, 8.0).radius(4.0).fill(if on { accent } else { k.c.text_faint }))
                .child(tx(*name, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate().min_w(0.0).grow(1.0))
                .child(tx(*count, type_::SMALL, k.c.text_muted)),
        );
    }
    let left = col().w(200.0).h_full().fill(sf.side).child(cats).child(action_bar(k, "Kategori baru"));
    let right = col().grow(1.0).min_w(0.0).h_full().fill(sf.work).child(d.body).child(action_bar(k, d.action));
    col().w_full().h(d.height).child(strip).child(row().w_full().grow(1.0).min_h(0.0).child(left).child(el().w(1.0).h_full().fill(sf.edge)).child(right))
}

/// The window's top bar, with Indonesian words.
fn topbar(k: &Kit, cx: &Cx<App>) -> El {
    let insets = cx.chrome_insets();
    let sf = theme::surf();
    row()
        .w_full()
        .h(theme::size::TOPBAR)
        .window_drag()
        .pl(space::S4 + insets.leading)
        .pr(space::S4 + insets.trailing)
        .gap(space::S2)
        .center_y()
        .fill(sf.chrome)
        .child(row().w(theme::size::TOP_LEAD - space::S4 - space::S3).gap(space::S2).center_y().child(icon(&icons::LOGO, 18.0).color(k.c.text_base)).child(tx("Artosia", type_::TITLE, k.c.text_base)))
        .child(
            row()
                .w(300.0)
                .h(34.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.lg)
                .fill(k.c.layer_02)
                .border(1.0, k.c.border_base)
                .child(icon(&rune_kit::glyphs::SEARCH, 16.0).color(k.c.text_muted))
                .child(tx("Cari lagu, ayat, pengumuman", type_::BODY, k.c.text_muted).truncate().min_w(0.0))
                .child(spacer())
                .child(super::widgets::keycap(k, "Ctrl K", false)),
        )
        .child(spacer())
        .child(btn("Tampilan").icon(&icons::PALETTE).ghost().build(k, |_, _| {}))
        .child(btn("Layar").icon(&icons::SCREEN).ghost().build(k, |_, _| {}))
        .child(btn("Telepon").icon(&icons::PHONE).ghost().build(k, |_, _| {}))
        .child(btn("Siaran").icon(&icons::STREAM).ghost().build(k, |_, _| {}))
        .child(btn("Dengar").icon(&icons::MIC).ghost().build(k, |_, _| {}))
        .child(el().w(1.0).h(20.0).fill(k.c.border_base))
        .child(btn("Pengaturan").icon(&icons::SLIDERS).ghost().build(k, |_, _| {}))
}

fn frame(s: &App, cx: &mut Cx<App>, k: &Kit, left: El, center: El, right: El, drawer: El) -> El {
    frame_w(s, cx, k, left, theme::size::PLAN, center, right, drawer)
}

#[allow(clippy::too_many_arguments)]
fn frame_w(s: &App, cx: &mut Cx<App>, k: &Kit, left: El, left_w: f32, center: El, right: El, drawer: El) -> El {
    let sf = theme::surf();
    let hline = || el().h(1.0).w_full().fill(sf.edge).a11y_hidden();
    let vline = || el().w(1.0).h_full().fill(sf.edge).a11y_hidden();
    col()
        .key("window")
        .w_full()
        .h_full()
        .fill(sf.chrome)
        .child(topbar(k, cx))
        .child(hline())
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(left.w(left_w).h_full())
                .child(vline())
                .child(center.grow(1.0).min_w(0.0).h_full().fill(sf.work))
                .child(vline())
                .child(right.w(RIGHT_W).h_full()),
        )
        .child(hline())
        .child(drawer)
}

/// The right column: a slide preview with a form panel under it.
fn right_col(k: &Kit, preview: El, form_title: &str, form: El) -> El {
    col().fill(theme::surf().side).child(preview).child(strip(k, form_title)).child(form.grow(1.0).min_h(0.0))
}

fn plan_col(k: &Kit, on: &str, take: usize) -> El {
    let rows: [(u8, &'static Icon, &str); 9] = [
        (0, &icons::SONG, "Pembukaan"),
        (1, &icons::SONG, "Contoh Pujian Alfa"),
        (0, &icons::SONG, "Warta jemaat"),
        (1, &icons::ANNOUNCE, "Jadwal kegiatan"),
        (1, &icons::ANNOUNCE, "Ulang tahun November"),
        (1, &icons::ANNOUNCE, "Persembahan syukur"),
        (0, &icons::SONG, "Khotbah"),
        (1, &icons::BOOK, "Contoh 1:1-5"),
        (1, &icons::SONG, "Contoh Pujian Beta"),
    ];
    let mut list = col().w_full().grow(1.0).min_h(0.0);
    for (kind, ic, name) in rows.into_iter().take(take) {
        list = list.child(if kind == 0 { section(k, name) } else { item(k, ic, name, name == on) });
    }
    col().fill(theme::surf().side).child(strip(k, "Ibadah Raya 1")).child(list).child(action_bar(k, "Bagian baru"))
}

fn tiles_drawer(k: &Kit, height: f32, tiles: &[(NoticeKind, NoticeStyle, &str, bool)]) -> El {
    let mut row_el = row().w_full().padding(space::S3).gap(space::S3).items_start();
    for (kind, style, name, on) in tiles {
        row_el = row_el.child(tile_cell(k, *kind, *style, 124.0, name, *on));
    }
    drawer(
        k,
        Drawer {
            tab: 3,
            height,
            cats: &[("Semua", "42"), ("Pengumuman", "15"), ("Lagu", "6"), ("Alkitab", "6")],
            cat_on: 1,
            action: "Templat baru",
            body: row_el,
        },
    )
}

// ---------------------------------------------------------------------------------------------
// Pages

pub(super) fn page(s: &App, cx: &mut Cx<App>, k: &Kit, page: u8, layer: &mut Vec<El>) -> El {
    let _ = layer;
    match page {
        1 => p1(s, cx, k),
        2 => p2(s, cx, k),
        3 => p3(s, cx, k),
        4 => p4(s, cx, k),
        5 => p5(s, cx, k),
        6 => p6(s, cx, k),
        _ => p7(s, cx, k),
    }
}

// P1: announcement editor. Form panel under the slide preview, templates in the drawer.
fn schedule_rows(k: &Kit) -> El {
    let mut rows = col().w_full().gap(6.0);
    for (d, time, title) in SCHEDULE {
        let day_chip = if d.is_empty() {
            row().w(84.0).h(30.0).center().child(icon(&icons::ARROW_DOWN, 14.0).color(k.c.text_faint))
        } else {
            row().w(84.0).h(30.0).gap(4.0).center().radius(15.0).fill(k.c.layer_03).child(tx(d, type_::SMALL_STRONG, k.c.text_base)).child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_muted))
        };
        rows = rows.child(
            row()
                .w_full()
                .h(44.0)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(k.c.layer_02)
                .child(icon(&icons::GRIP, 14.0).color(k.c.text_faint))
                .child(day_chip)
                .child(row().w(100.0).h(30.0).gap(6.0).center().radius(15.0).fill(k.c.layer_03).child(icon(&icons::HISTORY, 14.0).color(k.c.text_muted)).child(tx(time, NUM_STRONG, k.c.text_base)))
                .child(row().h(30.0).px(space::S3).gap(6.0).center_y().radius(15.0).fill(k.c.layer_03).grow(1.0).min_w(0.0).child(tx(title, type_::BODY, k.c.text_base).truncate().min_w(0.0).grow(1.0)).child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_muted)))
                .child(icon(&icons::TRASH, 14.0).color(k.c.text_faint)),
        );
    }
    rows
}

fn p1_center(k: &Kit) -> El {
    col()
        .padding(space::S4)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(icon(&icons::ANNOUNCE, 18.0).color(theme::accent()))
                .child(tx("Baris jadwal", type_::HEADING, k.c.text_base))
                .child(spacer())
                .child(switch(k, true, "Ambil dari Kalender"))
                .child(tx("Ambil dari Kalender", type_::BODY, k.c.text_base))
                .child(btn("Buka Kalender").icon(&CALENDAR).build(k, |_, _| {})),
        )
        .child(tx("Hari dan jam diambil dari acara mingguan. Ubah di Kalender, slide ikut berubah.", type_::SMALL, k.c.text_muted))
        .child(schedule_rows(k))
        .child(row().w_full().h(40.0).gap(6.0).center().radius(k.radius.md).border(1.0, k.c.border_base).child(icon(&icons::PLUS, 14.0).color(k.c.text_muted)).child(tx("Tambah baris", type_::BODY_STRONG, k.c.text_base)))
}

fn p1_form(k: &Kit) -> El {
    let styles = [(NoticeStyle::Hairline, "Hairline", true), (NoticeStyle::Ledger, "Ledger", false), (NoticeStyle::Band, "Band", false)];
    let mut tiles = row().w_full().gap(space::S2);
    for (style, name, on) in styles {
        tiles = tiles.child(tile_cell(k, NoticeKind::Schedule, style, 90.0, name, on));
    }
    col()
        .padding(space::S3)
        .gap(space::S2)
        .child(field(k, "Judul", "Jadwal kegiatan gereja", None))
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(el().grow(1.0).min_w(0.0).child(field(k, "Jenis", "Jadwal", Some(&CHEVRON_DOWN))))
                .child(
                    row()
                        .w(112.0)
                        .h(46.0)
                        .px(space::S3)
                        .gap(space::S2)
                        .center_y()
                        .radius(k.radius.md)
                        .fill(k.c.layer_02)
                        .border(1.0, k.c.border_base)
                        .child(el().size(22.0, 22.0).radius(11.0).fill(hex("#4caf50")).border(2.0, Color::WHITE.alpha(0.8)))
                        .child(tx("Hijau", type_::BODY, k.c.text_base).grow(1.0))
                        .child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_muted)),
                ),
        )
        .child(label(k, "Gaya"))
        .child(tiles)
}

fn p1_frame(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let tiles = [
        (NoticeKind::Schedule, NoticeStyle::Hairline, "Jadwal Hairline", true),
        (NoticeKind::Schedule, NoticeStyle::Ledger, "Jadwal Ledger", false),
        (NoticeKind::Schedule, NoticeStyle::Band, "Jadwal Band", false),
        (NoticeKind::Birthdays, NoticeStyle::Hairline, "Ulang tahun", false),
        (NoticeKind::Money, NoticeStyle::Ledger, "Persembahan", false),
        (NoticeKind::Account, NoticeStyle::Band, "Transfer", false),
    ];
    frame(
        s,
        cx,
        k,
        plan_col(k, "Jadwal kegiatan", 9),
        p1_center(k),
        right_col(k, slide_tile(NoticeKind::Schedule, NoticeStyle::Hairline, RIGHT_W), "Pengumuman", p1_form(k)),
        tiles_drawer(k, 236.0, &tiles),
    )
}

fn p1(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    p1_frame(s, cx, k)
}

// P2: colour choice. A Material swatch grid, then a square picker with a hue strip and an eyedropper.
const MATERIAL: [(&str, &str); 21] = [
    ("#f44336", "Merah"),
    ("#e91e63", "Merah muda"),
    ("#9c27b0", "Ungu"),
    ("#673ab7", "Ungu tua"),
    ("#3f51b5", "Nila"),
    ("#2196f3", "Biru"),
    ("#03a9f4", "Biru muda"),
    ("#00bcd4", "Sian"),
    ("#009688", "Hijau kebiruan"),
    ("#4caf50", "Hijau"),
    ("#8bc34a", "Hijau muda"),
    ("#cddc39", "Limau"),
    ("#ffeb3b", "Kuning"),
    ("#ffc107", "Kuning emas"),
    ("#ff9800", "Jingga"),
    ("#ff5722", "Jingga tua"),
    ("#795548", "Cokelat"),
    ("#9e9e9e", "Abu-abu"),
    ("#607d8b", "Abu kebiruan"),
    ("#000000", "Hitam"),
    ("#ffffff", "Putih"),
];

fn swatch(hexcode: &str, d: f32, on: bool) -> El {
    let c = hex(hexcode);
    let ink = if c.luminance() > 0.35 { Color::BLACK } else { Color::WHITE };
    let mut e = el().size(d, d).radius(d / 2.0).center().fill(c).border(1.0, Color::BLACK.alpha(0.18));
    if on {
        e = e.child(icon(&icons::CHECK, d * 0.55).color(ink));
    }
    e
}

fn p2(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let accent = theme::accent();
    let mut grid = row().w_full().gap(8.0).flex_wrap();
    for (code, _) in MATERIAL {
        grid = grid.child(swatch(code, 32.0, code == "#4caf50"));
    }
    let own = row().gap(8.0).child(swatch("#4caf50", 32.0, true)).child(swatch("#ffffff", 32.0, false)).child(swatch("#9e9e9e", 32.0, false));
    let left = col()
        .w(300.0)
        .gap(space::S2)
        .child(label(k, "Warna di slide ini"))
        .child(own)
        .child(el().h(space::S1))
        .child(label(k, "Pilih warna"))
        .child(grid)
        .child(row().gap(space::S2).center_y().mt(space::S2).child(swatch("#4caf50", 20.0, false)).child(tx("Hijau", type_::BODY_STRONG, k.c.text_base)));
    // The square picker: the hue's own colour, white fading in from the left, black from the bottom.
    let (sw, sh) = (300.0, 164.0);
    let square = el()
        .size(sw, sh)
        .radius(8.0)
        .clip_rounded(8.0)
        .fill(hex("#14ff2e"))
        .child(el().absolute(0.0, 0.0).size(sw, sh).linear_gradient(PI / 2.0, [(0.0, Color::WHITE), (1.0, Color::WHITE.alpha(0.0))]))
        .child(el().absolute(0.0, 0.0).size(sw, sh).linear_gradient(PI, [(0.0, Color::BLACK.alpha(0.0)), (1.0, Color::BLACK)]))
        .child(el().absolute(sw * 0.62 - 9.0, sh * 0.34 - 9.0).size(18.0, 18.0).radius(9.0).border(2.5, Color::WHITE).shadow(1.0, 4.0, Color::BLACK.alpha(0.5)));
    let hues = ["#ff0000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff", "#ff0000"];
    let stops: Vec<(f32, Color)> = hues.iter().enumerate().map(|(i, h)| (i as f32 / 6.0, hex(h))).collect();
    let strip_w = 300.0;
    let hue = el()
        .size(strip_w, 16.0)
        .radius(8.0)
        .linear_gradient(PI / 2.0, stops)
        .child(el().absolute(strip_w * 0.34 - 9.0, -1.0).size(18.0, 18.0).radius(9.0).border(2.5, Color::WHITE).shadow(1.0, 4.0, Color::BLACK.alpha(0.5)));
    let right = col()
        .w(300.0)
        .gap(space::S3)
        .child(label(k, "Warna sendiri"))
        .child(square)
        .child(hue)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(btn("Pipet").icon(&EYEDROPPER).build(k, |_, _| {}))
                .child(
                    row()
                        .h(32.0)
                        .grow(1.0)
                        .radius(k.radius.md)
                        .clip_rounded(k.radius.md)
                        .border(1.0, k.c.border_base)
                        .child(row().grow(1.0).h_full().center().fill(hex("#4caf50")).child(tx("Sekarang", type_::TINY, Color::WHITE)))
                        .child(row().grow(1.0).h_full().center().fill(hex("#2fdc4a")).child(tx("Baru", type_::TINY, Color::BLACK))),
                ),
        )
        .child(row().w_full().gap(space::S3).center_y().child(tx("Kepekatan", type_::SMALL, k.c.text_muted)).child(slider(k, 1.0)).child(tx("100%", NUM, k.c.text_base)))
        .child(row().w_full().h(32.0).gap(6.0).center_y().child(icon(&CHEVRON_RIGHT, 14.0).color(k.c.text_muted)).child(tx("Lainnya", type_::BODY, k.c.text_muted)));
    let popover = col()
        .w(700.0)
        .radius(k.radius.xl)
        .fill(theme::surf().raised)
        .border(1.0, k.c.border_strong)
        .shadow(12.0, 40.0, Color::BLACK.alpha(0.5))
        .child(
            row()
                .w_full()
                .h(48.0)
                .px(space::S5)
                .center_y()
                .child(tx("Warna aksen", type_::HEADING, k.c.text_base))
                .child(spacer())
                .child(icon(&icons::CLOSE, 16.0).color(k.c.text_muted)),
        )
        .child(row().w_full().px(space::S5).py(space::S3).gap(space::S6).items_start().child(left).child(right))
        .child(
            row()
                .w_full()
                .px(space::S5)
                .py(space::S3)
                .gap(space::S2)
                .justify_end()
                .child(btn("Batal").build(k, |_, _| {}))
                .child(btn("Pakai").primary().build(k, |_, _| {})),
        );
    let _ = accent;
    p1_frame(s, cx, k).child(col().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).center().fill(Color::BLACK.alpha(0.5)).child(popover))
}

// P3: weekly schedule as FreeShow's event form, and the Kalender drawer with a date picker beside the week.
fn event_chip(k: &Kit, time: &str, title: &str, color: &str) -> El {
    let c = hex(color);
    row()
        .w_full()
        .min_h(36.0)
        .py(2.0)
        .gap(6.0)
        .center_y()
        .radius(6.0)
        .clip_rounded(6.0)
        .fill(c.alpha(0.16))
        .child(el().w(4.0).self_stretch().fill(c))
        .child(col().min_w(0.0).grow(1.0).child(tx(time, TextStyle::new(11.0, 600.0).lh(14.0).features("'tnum' 1"), k.c.text_muted)).child(tx(title, type_::SMALL_STRONG, k.c.text_base).wrap()))
}

fn mini_month(k: &Kit) -> El {
    let accent = theme::accent();
    let mut m = col().w(244.0).gap(2.0).child(
        row().w_full().h(28.0).center_y().child(icon(&CHEVRON_LEFT, 14.0).color(k.c.text_muted)).child(spacer()).child(tx("November 2026", type_::BODY_STRONG, k.c.text_base)).child(spacer()).child(icon(&CHEVRON_RIGHT, 14.0).color(k.c.text_muted)),
    );
    let mut head = row().w_full();
    for d in ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"] {
        head = head.child(row().w(34.0).h(20.0).center().child(tx(d, type_::TINY, k.c.text_muted)));
    }
    m = m.child(head);
    let weeks: [[u32; 7]; 5] = [[1, 2, 3, 4, 5, 6, 7], [8, 9, 10, 11, 12, 13, 14], [15, 16, 17, 18, 19, 20, 21], [22, 23, 24, 25, 26, 27, 28], [29, 30, 0, 0, 0, 0, 0]];
    for week in weeks {
        let mut r = row().w_full();
        for (col_at, d) in week.into_iter().enumerate() {
            let mut cell = col().w(34.0).h(26.0).center();
            if d == 0 {
                r = r.child(cell);
                continue;
            }
            // The dots mark a day with a weekly event: Sunday, Wednesday, Friday, Saturday.
            let has = matches!(col_at, 0 | 3 | 5 | 6);
            let selected = d == 15;
            let today = d == 14;
            let mut num = el().size(24.0, 22.0).radius(11.0).center();
            if selected {
                num = num.fill(accent);
            } else if today {
                num = num.border(1.5, accent);
            }
            num = num.child(tx(d.to_string(), type_::SMALL_STRONG, if selected { Color::WHITE } else { k.c.text_base }));
            cell = cell.child(num);
            if has && !selected {
                cell = cell.child(el().size(4.0, 4.0).radius(2.0).fill(accent).mt(-1.0));
            }
            r = r.child(cell);
        }
        m = m.child(r);
    }
    m
}

fn week_view(k: &Kit) -> El {
    let days = [
        ("Min 15", vec![("06.30", "Ibadah Raya 1", "#4caf50"), ("09.00", "Ibadah Raya 2", "#4caf50"), ("09.00", "Sekolah Minggu", "#ff9800")]),
        ("Sen 16", vec![]),
        ("Sel 17", vec![]),
        ("Rab 18", vec![("18.30", "Ibadah doa", "#2196f3")]),
        ("Kam 19", vec![]),
        ("Jum 20", vec![("18.30", "Persekutuan pemuda", "#9c27b0")]),
        ("Sab 21", vec![("17.00", "Latihan pujian", "#e91e63")]),
    ];
    let mut r = row().grow(1.0).min_w(0.0).gap(6.0).items_start();
    for (name, events) in days {
        let mut c = col().grow(1.0).min_w(0.0).gap(4.0).child(
            row().w_full().h(24.0).center().radius(6.0).fill(if name == "Min 15" { theme::accent().alpha(0.18) } else { k.c.layer_02 }).child(tx(name, type_::SMALL_STRONG, k.c.text_base)),
        );
        for (time, title, color) in events {
            c = c.child(event_chip(k, time, title, color));
        }
        r = r.child(c);
    }
    r
}

fn p3(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    // Left: the weekly events, grouped by day, the open one outlined.
    let events = [
        ("Minggu", vec![("06.30", "Ibadah Raya 1", "#4caf50"), ("09.00", "Ibadah Raya 2", "#4caf50"), ("09.00", "Sekolah Minggu", "#ff9800")]),
        ("Rabu", vec![("18.30", "Ibadah doa", "#2196f3")]),
        ("Jumat", vec![("18.30", "Persekutuan pemuda", "#9c27b0")]),
        ("Sabtu", vec![("17.00", "Latihan pujian", "#e91e63")]),
    ];
    let mut list = col().w_full().grow(1.0).min_h(0.0);
    for (dayname, rows) in events {
        list = list.child(section(k, dayname));
        for (time, title, color) in rows {
            let on = title == "Ibadah Raya 1";
            let mut r = row().w_full().h(36.0).px(space::S3).gap(space::S2).center_y().child(tx(time, NUM, k.c.text_muted)).child(el().size(10.0, 10.0).radius(5.0).fill(hex(color))).child(tx(title, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate().min_w(0.0).grow(1.0));
            if on {
                r = r.fill(theme::accent().alpha(0.14)).border(1.5, theme::accent());
            }
            list = list.child(r);
        }
    }
    let left = col().fill(sf.side).child(strip(k, "Acara mingguan")).child(list).child(action_bar(k, "Acara baru"));
    // Centre: the event form. A day is a toggle, a time a picker, a date a picker, a repeat a switch and a sentence.
    let mut days = row().gap(6.0);
    for (name, on) in [("Min", true), ("Sen", false), ("Sel", false), ("Rab", false), ("Kam", false), ("Jum", false), ("Sab", false)] {
        days = days.child(day(k, name, on, 38.0));
    }
    let form = col()
        .w(400.0)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(el().grow(1.0).min_w(0.0).child(field(k, "Nama acara", "Ibadah Raya 1", None)))
                .child(row().w(76.0).h(46.0).gap(6.0).center().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(el().size(22.0, 22.0).radius(11.0).fill(hex("#4caf50"))).child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_muted))),
        )
        .child(label(k, "Hari"))
        .child(days)
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(el().grow(1.0).min_w(0.0).child(field(k, "Mulai tanggal", "15 Nov 2026", Some(&CALENDAR))))
                .child(el().w(104.0).child(field(k, "Jam mulai", "06.30", Some(&icons::HISTORY)).border(1.5, theme::accent())))
                .child(el().w(104.0).child(field(k, "Jam selesai", "08.00", Some(&icons::HISTORY)))),
        )
        .child(
            card(k)
                .w_full()
                .padding(space::S3)
                .gap(space::S2)
                .child(row().w_full().gap(space::S3).center_y().child(tx("Ulangi", type_::BODY_STRONG, k.c.text_base)).child(spacer()).child(switch(k, true, "Ulangi")))
                .child(tx("Setiap minggu pada hari Minggu, mulai 15 November 2026, tanpa akhir.", type_::BODY, k.c.text_base).wrap())
                .child(seg(k, &["Tidak pernah", "Pada tanggal", "Setelah"], 0)),
        );
    // The time picker, drawn open beside the form: hours and minutes as lists, and the times this church already uses.
    let col_list = |items: &[&str], at: usize| {
        let mut c = col().gap(2.0).items_center();
        for (i, v) in items.iter().enumerate() {
            let on = i == at;
            c = c.child(row().w(64.0).h(34.0).center().radius(k.radius.md).fill(if on { theme::accent() } else { Color::TRANSPARENT }).child(tx(*v, if on { TextStyle::new(17.0, 700.0).lh(22.0).features("'tnum' 1") } else { TextStyle::new(15.0, 500.0).lh(20.0).features("'tnum' 1") }, if on { Color::WHITE } else { k.c.text_muted })));
        }
        c
    };
    let mut recent = row().w_full().gap(6.0).flex_wrap();
    for (t, on) in [("06.30", true), ("09.00", false), ("17.00", false), ("18.30", false)] {
        recent = recent.child(chip(k, t, on));
    }
    let picker = card(k)
        .w(270.0)
        .padding(space::S4)
        .gap(space::S3)
        .child(tx("Jam mulai", type_::TITLE, k.c.text_base))
        .child(row().w_full().gap(space::S2).justify_center().items_center().child(col_list(&["05", "06", "07"], 1)).child(tx(".", type_::HEADING, k.c.text_muted)).child(col_list(&["15", "30", "45"], 1)))
        .child(label(k, "Jam yang sering dipakai"))
        .child(recent)
        .child(row().w_full().h(30.0).gap(6.0).center_y().child(tx("Selesai otomatis 1 jam 30 menit setelahnya", type_::SMALL, k.c.text_muted)));
    let center = col()
        .padding(space::S4)
        .gap(space::S3)
        .child(row().w_full().gap(space::S3).center_y().child(icon(&CALENDAR, 18.0).color(theme::accent())).child(tx("Acara", type_::HEADING, k.c.text_base)).child(spacer()).child(btn("Simpan").primary().icon(&icons::CHECK).build(k, |_, _| {})).child(btn("Simpan semua minggu").build(k, |_, _| {})))
        .child(row().w_full().gap(space::S5).items_start().child(form).child(picker));
    let note = col().padding(space::S3).gap(space::S2).child(tx("Slide Jadwal ikut acara mingguan.", type_::BODY, k.c.text_base).wrap()).child(row().w_full().gap(space::S3).center_y().child(switch(k, true, "Ambil dari Kalender")).child(tx("Ambil dari Kalender", type_::BODY, k.c.text_base)));
    let right = right_col(k, slide_tile(NoticeKind::Schedule, NoticeStyle::Hairline, RIGHT_W), "Hasil di layar", note);
    let body = row().w_full().grow(1.0).min_h(0.0).padding(space::S3).gap(space::S5).items_start().child(mini_month(k)).child(el().w(1.0).h_full().fill(sf.edge)).child(week_view(k));
    frame(
        s,
        cx,
        k,
        left,
        center,
        right,
        drawer(k, Drawer { tab: 4, height: 292.0, cats: &[("Semua acara", "8"), ("Ibadah", "6"), ("Pemuda", "2"), ("Ulang tahun", "30")], cat_on: 0, action: "Acara baru", body }),
    )
}

// P4: birthdays. An import that maps columns, then the list by month in the Kalender drawer.
fn p4(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    let steps = [("Pilih berkas", "ulang-tahun-jemaat.csv", true, false), ("Petakan kolom", "4 kolom ditemukan", false, true), ("Tinjau", "30 orang, 12 bulan", false, false)];
    let mut stepper = col().w_full().padding(space::S3).gap(space::S3);
    for (at, (title, sub, done, now)) in steps.into_iter().enumerate() {
        let dot = el().size(24.0, 24.0).radius(12.0).center().fill(if done { k.c.text_base } else if now { theme::accent() } else { k.c.layer_03 }).child(if done { icon(&icons::CHECK, 14.0).color(k.c.page) } else { tx((at + 1).to_string(), type_::SMALL_STRONG, if now { Color::WHITE } else { k.c.text_muted }) });
        stepper = stepper.child(row().w_full().gap(space::S3).center_y().child(dot).child(col().min_w(0.0).grow(1.0).child(tx(title, if now { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base)).child(tx(sub, type_::SMALL, k.c.text_muted).truncate().min_w(0.0))));
    }
    let sources = col().w_full().child(section(k, "Impor dari")).child(item(k, &FILE, "Berkas CSV", true)).child(item(k, &SHEET, "Excel", false)).child(item(k, &icons::PASTE, "Tempel dari papan klip", false));
    let left = col().fill(sf.side).child(strip(k, "Impor")).child(col().w_full().grow(1.0).min_h(0.0).child(stepper).child(sources)).child(action_bar(k, "Impor lain"));
    let map = |from: &str, sample: &str, to: &str, on: bool| {
        row()
            .w_full()
            .h(56.0)
            .px(space::S3)
            .gap(space::S3)
            .center_y()
            .child(col().w(190.0).child(tx("Kolom di berkas", type_::TINY, k.c.text_muted)).child(row().h(32.0).px(space::S3).gap(6.0).center_y().radius(k.radius.md).fill(k.c.layer_03).child(icon(&icons::GRIP, 12.0).color(k.c.text_faint)).child(tx(from, type_::BODY, k.c.text_base).truncate().min_w(0.0))))
            .child(icon(&icons::ARROW_RIGHT, 16.0).color(k.c.text_muted))
            .child(col().w(190.0).child(tx("Contoh isinya", type_::TINY, k.c.text_muted)).child(row().h(32.0).px(space::S3).center_y().child(tx(sample, NUM, k.c.text_base).truncate().min_w(0.0))))
            .child(col().grow(1.0).min_w(0.0).child(tx("Jadi", type_::TINY, k.c.text_muted)).child(row().h(32.0).px(space::S3).gap(6.0).center_y().radius(k.radius.md).fill(if on { theme::accent().alpha(0.16) } else { k.c.layer_02 }).border(1.0, if on { theme::accent() } else { k.c.border_base }).child(tx(to, type_::BODY_STRONG, if on { k.c.text_base } else { k.c.text_muted }).grow(1.0).truncate().min_w(0.0)).child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_muted))))
    };
    let center = col()
        .padding(space::S4)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(icon(&icons::IMPORT, 18.0).color(theme::accent()))
                .child(tx("Petakan kolom", type_::HEADING, k.c.text_base))
                .child(spacer())
                .child(chip(k, "ulang-tahun-jemaat.csv", false))
                .child(tx("30 baris", type_::SMALL, k.c.text_muted)),
        )
        .child(tx("Pilih untuk apa tiap kolom dipakai. Cukup sekali, pilihan diingat untuk berkas berikutnya.", type_::SMALL, k.c.text_muted))
        .child(
            card(k)
                .w_full()
                .child(map("Nama lengkap", "Contoh Alfa Beta", "Nama", true))
                .child(el().h(1.0).w_full().fill(k.c.border_muted))
                .child(map("Tanggal lahir", "12/11/1984", "Tanggal lahir (hh/bb/tttt)", true))
                .child(el().h(1.0).w_full().fill(k.c.border_muted))
                .child(map("No. HP", "0812 0000 0000", "Abaikan", false))
                .child(el().h(1.0).w_full().fill(k.c.border_muted))
                .child(map("Keterangan", "Anggota pemuda", "Abaikan", false)),
        )
        .child(row().w_full().gap(space::S3).center_y().child(switch(k, true, "Baris pertama adalah judul")).child(tx("Baris pertama adalah judul", type_::BODY, k.c.text_base)).child(spacer()).child(btn("Kembali").build(k, |_, _| {})).child(btn("Lanjut").primary().icon(&icons::ARROW_RIGHT).build(k, |_, _| {})));
    let note = col().padding(space::S3).gap(space::S2).child(tx("Slide ini mengambil bulan yang sedang berjalan. Nama bertambah, slide ikut.", type_::BODY, k.c.text_base).wrap()).child(row().w_full().gap(space::S3).center_y().child(switch(k, true, "Bulan ini otomatis")).child(tx("Bulan ini otomatis", type_::BODY, k.c.text_base)));
    let right = right_col(k, slide_tile(NoticeKind::Birthdays, NoticeStyle::Band, RIGHT_W), "Ulang tahun November", note);
    // The list by month.
    let months = [("Jan", 12), ("Feb", 9), ("Mar", 14), ("Apr", 11), ("Mei", 10), ("Jun", 13), ("Jul", 8), ("Agu", 15), ("Sep", 11), ("Okt", 12), ("Nov", 30), ("Des", 14)];
    let mut chips = row().w_full().gap(6.0).px(space::S3).h(44.0).center_y();
    for (name, n) in months {
        let on = name == "Nov";
        chips = chips.child(row().h(28.0).px(space::S3).gap(6.0).center().radius(14.0).fill(if on { theme::accent() } else { k.c.layer_02 }).border(1.0, if on { theme::accent() } else { k.c.border_base }).child(tx(name, type_::SMALL_STRONG, if on { Color::WHITE } else { k.c.text_base })).child(tx(n.to_string(), type_::TINY, if on { Color::WHITE.alpha(0.85) } else { k.c.text_muted })));
    }
    let mut sorted = names();
    sorted.sort_by_key(|(_, d)| *d);
    let mut columns = row().w_full().px(space::S3).gap(space::S4).items_start();
    for chunk in sorted.chunks(6) {
        let mut c = col().grow(1.0).min_w(0.0);
        for (name, d) in chunk {
            let today = *d == 14;
            c = c.child(
                row()
                    .w_full()
                    .h(30.0)
                    .px(space::S2)
                    .gap(space::S2)
                    .center_y()
                    .radius(6.0)
                    .fill(if today { theme::accent().alpha(0.16) } else { Color::TRANSPARENT })
                    .child(el().size(22.0, 22.0).radius(11.0).center().fill(k.c.layer_03).child(tx(name.chars().nth(7).unwrap_or('A').to_string(), type_::TINY, k.c.text_base)))
                    .child(tx(name.clone(), type_::BODY, k.c.text_base).truncate().min_w(0.0).grow(1.0))
                    .child(tx(format!("{d} Nov"), NUM, if today { theme::accent() } else { k.c.text_muted })),
            );
        }
        columns = columns.child(c);
    }
    let body = col().w_full().grow(1.0).min_h(0.0).child(chips).child(columns);
    frame(s, cx, k, left, center, right, drawer(k, Drawer { tab: 4, height: 322.0, cats: &[("Semua acara", "8"), ("Ibadah", "6"), ("Pemuda", "2"), ("Ulang tahun", "30")], cat_on: 3, action: "Orang baru", body }))
}

// P5: money. A sheet with cells in place and a total row that sums itself.
fn p5(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    let total: u64 = GIFTS.iter().map(|g| g.2).sum();
    let widths = (40.0, 150.0, 150.0);
    let head = |text_: &str, w: Option<f32>, end: bool| {
        let c = row().h(28.0).px(space::S3).center_y().fill(sf.header).child(tx(text_, type_::SMALL_STRONG, k.c.text_muted));
        let c = if end { c.justify_end() } else { c };
        match w {
            Some(w) => c.w(w),
            None => c.grow(1.0).min_w(0.0),
        }
    };
    let mut sheet = col()
        .w_full()
        .radius(k.radius.md)
        .clip_rounded(k.radius.md)
        .border(1.0, k.c.border_base)
        .child(row().w_full().child(head("", Some(widths.0), false)).child(head("Nama", None, false)).child(head("Jenis", Some(widths.1), false)).child(head("Jumlah (Rp)", Some(widths.2), true)));
    for (at, (name, kind, value)) in GIFTS.iter().enumerate() {
        let cell = |c: El, w: Option<f32>| {
            let c = c.h(28.0).px(space::S3).center_y().border_sides([0.0, 1.0, 1.0, 0.0], k.c.border_muted);
            match w {
                Some(w) => c.w(w),
                None => c.grow(1.0).min_w(0.0),
            }
        };
        sheet = sheet.child(
            row()
                .w_full()
                .fill(k.c.layer_01)
                .child(cell(row().justify_center().child(tx((at + 1).to_string(), NUM, k.c.text_faint)), Some(widths.0)))
                .child(cell(row().child(tx(*name, type_::BODY, k.c.text_base).truncate().min_w(0.0)), None))
                .child(cell(row().gap(6.0).child(tx(*kind, type_::BODY, k.c.text_base)).child(spacer()).child(icon(&CHEVRON_DOWN, 12.0).color(k.c.text_faint)), Some(widths.1)))
                .child(cell(row().justify_end().child(tx(money(*value), NUM, k.c.text_base)), Some(widths.2))),
        );
    }
    let accent = theme::accent();
    let total_cell = |c: El, w: Option<f32>| {
        let c = c.h(34.0).px(space::S3).center_y().fill(accent.alpha(0.16));
        match w {
            Some(w) => c.w(w),
            None => c.grow(1.0).min_w(0.0),
        }
    };
    sheet = sheet.child(
        row()
            .w_full()
            .child(total_cell(row().justify_center().child(icon(&SIGMA, 14.0).color(accent)), Some(widths.0)))
            .child(total_cell(row().gap(space::S2).center_y().child(tx("Jumlah", type_::BODY_STRONG, k.c.text_base)).child(state_badge(k, "dihitung otomatis")), None))
            .child(total_cell(row(), Some(widths.1)))
            .child(total_cell(row().justify_end().child(tx(money(total), NUM_STRONG, k.c.text_base)), Some(widths.2)).border(2.0, accent)),
    );
    let center = col()
        .padding(space::S4)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(icon(&SHEET, 18.0).color(accent))
                .child(tx("Persembahan 9 November", type_::HEADING, k.c.text_base))
                .child(spacer())
                .child(btn("Impor dari berkas").icon(&icons::IMPORT).build(k, |_, _| {})),
        )
        .child(
            row()
                .w_full()
                .h(32.0)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(k.c.layer_02)
                .child(tx("fx", TextStyle::new(12.0, 700.0).lh(16.0), k.c.text_muted))
                .child(tx("D11", NUM_STRONG, k.c.text_base))
                .child(tx("=JUMLAH(D2:D10)", NUM, k.c.text_muted)),
        )
        .child(sheet)
        .child(row().w_full().h(36.0).gap(6.0).center().radius(k.radius.md).border(1.0, k.c.border_base).child(icon(&icons::PLUS, 14.0).color(k.c.text_muted)).child(tx("Tambah baris", type_::BODY_STRONG, k.c.text_base)));
    let styles = [(NoticeStyle::Hairline, "Hairline", false), (NoticeStyle::Ledger, "Ledger", true), (NoticeStyle::Band, "Band", false)];
    let mut tiles = row().w_full().gap(space::S2);
    for (style, name, on) in styles {
        tiles = tiles.child(tile_cell(k, NoticeKind::Money, style, 90.0, name, on));
    }
    let form = col()
        .padding(space::S3)
        .gap(space::S2)
        .child(label(k, "Gaya"))
        .child(tiles)
        .child(row().w_full().gap(space::S3).center_y().child(switch(k, true, "Tampilkan jumlah")).child(tx("Tampilkan jumlah", type_::BODY, k.c.text_base)))
        .child(row().w_full().gap(space::S3).center_y().child(switch(k, false, "Tampilkan nama")).child(tx("Nama sebagai Hamba Tuhan", type_::BODY, k.c.text_base)));
    let right = right_col(k, slide_tile(NoticeKind::Money, NoticeStyle::Ledger, RIGHT_W), "Persembahan syukur", form);
    let tiles = [
        (NoticeKind::Money, NoticeStyle::Hairline, "Hairline", false),
        (NoticeKind::Money, NoticeStyle::Ledger, "Ledger", true),
        (NoticeKind::Money, NoticeStyle::Band, "Band", false),
        (NoticeKind::Account, NoticeStyle::Ledger, "Transfer", false),
    ];
    frame(s, cx, k, plan_col(k, "Persembahan syukur", 9), center, right, tiles_drawer(k, 236.0, &tiles))
}

fn state_badge(k: &Kit, label: &str) -> El {
    row().h(20.0).px(7.0).center().radius(6.0).fill(k.c.layer_03).child(tx(label, type_::TINY, k.c.text_muted))
}

// P6: template gallery. A drawer of live-rendered tiles, a preview, one Use button and apply modes.
fn p6(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    let kinds = [(NoticeKind::Schedule, "Jadwal"), (NoticeKind::Birthdays, "Ulang tahun"), (NoticeKind::Money, "Persembahan"), (NoticeKind::Account, "Transfer"), (NoticeKind::Plain, "Biasa")];
    let styles = [(NoticeStyle::Hairline, "Hairline", "garis tipis"), (NoticeStyle::Ledger, "Ledger", "tabel berlajur"), (NoticeStyle::Band, "Band", "pita judul")];
    let mut grid = col().w_full().padding(space::S3).gap(space::S2);
    for (style, name, note) in styles {
        let mut r = row().w_full().gap(space::S3).items_start().child(col().w(78.0).gap(2.0).pt(space::S2).child(tx(name, type_::BODY_STRONG, k.c.text_base)).child(tx(note, type_::SMALL, k.c.text_muted)));
        for (kind, kname) in kinds {
            let on = name == "Ledger" && kname == "Jadwal";
            r = r.child(tile_cell(k, kind, style, 134.0, kname, on));
        }
        grid = grid.child(r);
    }
    let accent = theme::accent();
    let apply = col()
        .w(264.0)
        .h_full()
        .fill(sf.side)
        .padding(space::S3)
        .gap(space::S2)
        .child(tx("Ledger, Jadwal", type_::TITLE, k.c.text_base))
        .child(label(k, "Terapkan ke"))
        .child(seg(k, &["Slide ini", "Semua pengumuman"], 1).w_full())
        .child(label(k, "Isi"))
        .child(seg(k, &["Tetap", "Ganti"], 0))
        .child(spacer())
        .child(btn("Pakai templat").primary().tall().icon(&icons::CHECK).build(k, |_, _| {}).w_full());
    let body = row().w_full().grow(1.0).min_h(0.0).child(grid).child(el().w(1.0).h_full().fill(sf.edge)).child(apply);
    let dr = drawer(
        k,
        Drawer {
            tab: 3,
            height: 414.0,
            cats: &[("Semua", "42"), ("Pengumuman", "15"), ("Lagu", "6"), ("Alkitab", "6"), ("Latar polos", "6"), ("Tanpa kategori", "9")],
            cat_on: 1,
            action: "Templat baru",
            body,
        },
    );
    // Above the drawer: the open announcement as the room will see it, with its neighbours in the plan.
    let mut slides = row().w_full().padding(space::S3).gap(space::S3).items_start();
    for (i, (style, _, _)) in styles.into_iter().enumerate() {
        slides = slides.child(col().gap(4.0).child(el().padding(2.0).radius(6.0).border(if i == 1 { 2.0 } else { 0.0 }, accent).child(slide_tile(NoticeKind::Schedule, style, 188.0))).child(tx(format!("{}", i + 1), type_::SMALL_STRONG, k.c.text_muted)));
    }
    let center = col().child(slides);
    let right = right_col(k, slide_tile(NoticeKind::Schedule, NoticeStyle::Ledger, RIGHT_W), "Pratinjau", col().padding(space::S3).gap(space::S2).child(tx("Isi dari acara mingguan.", type_::SMALL, k.c.text_muted)));
    frame(s, cx, k, plan_col(k, "Jadwal kegiatan", 6), center, right, dr)
}

// P7: rundown with a song, a video with sound, an audio item and the Penutup scene.
fn thumb_box(w: f32, h: f32, from: &str, to: &str, mark: &'static Icon) -> El {
    el().size(w, h).radius(6.0).clip_rounded(6.0).gradient(hex(from), hex(to)).center().child(icon(mark, 18.0).color(Color::WHITE.alpha(0.9)))
}

fn p7(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    let accent = theme::accent();
    // Left: the rundown. The video and the audio item carry their own end action and volume.
    let video_row = col()
        .w_full()
        .px(space::S3)
        .py(space::S2)
        .gap(6.0)
        .child(row().w_full().gap(space::S2).center_y().child(icon(&FILM, 16.0).color(k.c.text_muted)).child(tx("Video pembuka", type_::BODY, k.c.text_base).grow(1.0)).child(tx("0:42", NUM, k.c.text_muted)))
        .child(row().w_full().gap(space::S2).center_y().child(icon(&VOLUME, 14.0).color(k.c.text_muted)).child(slider(k, 0.8)).child(tx("80", NUM, k.c.text_base)))
        .child(row().w_full().gap(space::S2).center_y().child(tx("Sesudahnya", type_::SMALL, k.c.text_muted)).child(seg(k, &["Berhenti", "Ulangi", "Lanjut"], 2)));
    let audio_row = col()
        .w_full()
        .px(space::S3)
        .py(space::S2)
        .gap(6.0)
        .child(row().w_full().gap(space::S2).center_y().child(icon(&VOLUME, 16.0).color(k.c.text_muted)).child(tx("Musik pengiring", type_::BODY, k.c.text_base).grow(1.0)).child(tx("3:20", NUM, k.c.text_muted)))
        .child(row().w_full().gap(space::S2).center_y().child(icon(&VOLUME, 14.0).color(k.c.text_muted)).child(slider(k, 0.45)).child(tx("45", NUM, k.c.text_base)))
        .child(row().w_full().gap(space::S2).center_y().child(tx("Sesudahnya", type_::SMALL, k.c.text_muted)).child(seg(k, &["Berhenti", "Ulangi", "Lanjut"], 1)));
    let list = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(section(k, "Pembukaan"))
        .child(video_row)
        .child(el().h(1.0).w_full().fill(k.c.border_muted))
        .child(item(k, &icons::SONG, "Contoh Pujian Alfa", false))
        .child(audio_row)
        .child(section(k, "Penutup"))
        .child(item(k, &LAYERS, "Penutup", true));
    let left = col().fill(sf.side).child(strip(k, "Ibadah Raya 1")).child(list).child(action_bar(k, "Bagian baru"));
    // Centre: the scene as layers, each with what it plays.
    let layer_card = |thumb: El, title: &str, sub: &str, controls: El| {
        row().w_full().padding(space::S3).gap(space::S3).center_y().radius(k.radius.lg).fill(k.c.layer_02).border(1.0, k.c.border_muted).child(icon(&icons::GRIP, 14.0).color(k.c.text_faint)).child(thumb).child(col().w(176.0).gap(2.0).child(tx(title, type_::BODY_STRONG, k.c.text_base)).child(tx(sub, type_::SMALL, k.c.text_muted).truncate().min_w(0.0))).child(controls.grow(1.0).min_w(0.0))
    };
    let center = col()
        .padding(space::S4)
        .gap(space::S2)
        .child(row().w_full().gap(space::S3).center_y().child(icon(&LAYERS, 18.0).color(accent)).child(tx("Penutup", type_::HEADING, k.c.text_base)).child(state_badge(k, "adegan, 4 lapisan")).child(spacer()).child(btn("Mulai adegan").primary().icon(&icons::PLAY).build(k, |_, _| {})))
        .child(layer_card(thumb_box(80.0, 45.0, "#0e342c", "#041210", &icons::IMAGE), "Latar", "latar-hijau-lembut", row().w_full().gap(space::S2).center_y().child(icon(&icons::LOOK, 14.0).color(k.c.text_muted)).child(tx("Redup", type_::SMALL, k.c.text_muted)).child(slider(k, 0.4)).child(tx("40", NUM, k.c.text_base))))
        .child(layer_card(thumb_box(80.0, 45.0, "#4a1820", "#14060c", &FILM), "Video penutup", "video-penutup.mp4, 0:18", col().gap(6.0).child(row().w_full().gap(space::S2).center_y().child(icon(&VOLUME, 14.0).color(k.c.text_muted)).child(slider(k, 0.7)).child(tx("70", NUM, k.c.text_base))).child(row().w_full().gap(space::S2).center_y().child(tx("Sesudahnya", type_::SMALL, k.c.text_muted)).child(seg(k, &["Berhenti", "Ulangi", "Lanjut"], 0)))))
        .child(layer_card(thumb_box(80.0, 45.0, "#2a3a6a", "#10162c", &VOLUME), "Musik", "musik-penutup.mp3, 3:20", col().gap(6.0).child(row().w_full().gap(space::S2).center_y().child(icon(&VOLUME, 14.0).color(k.c.text_muted)).child(slider(k, 0.55)).child(tx("55", NUM, k.c.text_base))).child(row().w_full().gap(space::S2).center_y().child(tx("Pudar masuk", type_::SMALL, k.c.text_muted)).child(seg(k, &["Tidak", "3 dtk", "6 dtk"], 1)))))
        .child(layer_card(thumb_box(80.0, 45.0, "#202226", "#0b0c0e", &icons::LOGO), "Logo", "logo-gereja.png", row().w_full().gap(space::S2).center_y().child(tx("Ukuran", type_::SMALL, k.c.text_muted)).child(seg(k, &["Kecil", "Sedang", "Besar"], 1))))
        .child(
            card(k)
                .w_full()
                .padding(space::S3)
                .gap(space::S2)
                .child(row().w_full().gap(space::S2).center_y().child(icon(&BROOM, 14.0).color(k.c.text_muted)).child(tx("Saat dibersihkan", type_::BODY_STRONG, k.c.text_base)))
                .child(row().w_full().gap(6.0).flex_wrap().child(chip(k, "Hentikan video", true)).child(chip(k, "Pudarkan musik 3 dtk", true)).child(chip(k, "Sembunyikan logo", true)).child(chip(k, "Kosongkan latar", false))),
        );
    // Right: the preview, Clear all and its parts.
    let mut clear = row().w_full().gap(6.0).flex_wrap();
    for (ic, name) in [(&icons::NO_TEXT, "Teks"), (&icons::IMAGE, "Latar"), (&FILM, "Video"), (&VOLUME, "Audio"), (&icons::LOGO, "Logo")] {
        clear = clear.child(row().h(28.0).px(space::S3).gap(6.0).center().radius(14.0).fill(k.c.layer_02).border(1.0, k.c.border_base).child(icon(ic, 14.0).color(k.c.text_muted)).child(tx(name, type_::SMALL_STRONG, k.c.text_base)));
    }
    let form = col()
        .padding(space::S3)
        .gap(space::S2)
        .child(row().w_full().h(36.0).gap(6.0).center().radius(k.radius.md).fill(theme::live()).child(icon(&icons::CLOSE, 14.0).color(Color::WHITE)).child(tx("Bersihkan semua", type_::BODY_STRONG, Color::WHITE)))
        .child(clear)
        .child(label(k, "Saat dimulai"))
        .child(tx("Putar video penutup, lalu musik masuk 3 detik.", type_::SMALL, k.c.text_muted).wrap());
    let right = right_col(k, scene_tile(RIGHT_W), "Penutup", form);
    // Drawer: the media bin.
    let bin = [("latar-hijau-lembut", "#0e342c", "#041210", &icons::IMAGE), ("latar-biru-malam", "#101830", "#06080f", &icons::IMAGE), ("video-pembuka", "#3a2a5a", "#120c20", &FILM), ("video-penutup", "#4a1820", "#14060c", &FILM), ("musik-pengiring", "#2a3a6a", "#10162c", &VOLUME), ("musik-penutup", "#2a3a6a", "#10162c", &VOLUME)];
    let mut tiles = row().w_full().padding(space::S3).gap(space::S3).items_start();
    for (name, from, to, mark) in bin {
        tiles = tiles.child(col().gap(4.0).items_center().child(thumb_box(136.0, 76.0, from, to, mark)).child(tx(name, type_::SMALL, k.c.text_muted).truncate().max_w(136.0)));
    }
    let dr = drawer(k, Drawer { tab: 1, height: 236.0, cats: &[("Semua", "24"), ("Latar", "8"), ("Video", "7"), ("Audio", "6")], cat_on: 0, action: "Tambah berkas", body: tiles });
    frame_w(s, cx, k, left, 320.0, center, right, dr)
}
