//! UIOVER1 direction B: Data windows, after Planning Center and ChurchSuite. Sources per page: `~/Shared/inspo/artosia-ui-overhaul/directions.md`.
//! Drawn only by the `mock-b-p1` to `mock-b-p7` scenes; nothing here is reachable in the program.
//! Every word is Indonesian and hardcoded, because this is a picture to compare and never ships.

use rune::{Color, Cx, El, Icon, TextStyle, col, el, icon, oklch, row, spacer, text};
use rune_kit::{Dialog, Kit, SegmentedControl};

use super::widgets::{btn, tag_chip};
use crate::state::App;
use crate::theme::{self, icons, space, type_};

macro_rules! glyph {
    ($name:ident, $id:literal, $body:literal) => {
        static $name: Icon = Icon { name: $id, view: 16.0, body: $body };
    };
}

glyph!(CALENDAR, "b-calendar", r#"<rect x="2" y="3" width="12" height="10.5" rx="1.5" stroke="currentColor"/><path d="M2 6.5H14M5.5 1.5V4M10.5 1.5V4" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(PERSON, "b-person", r#"<circle cx="8" cy="5.5" r="2.7" stroke="currentColor"/><path d="M2.8 13.5C3.3 10.8 5.3 9.6 8 9.6C10.7 9.6 12.7 10.8 13.2 13.5" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(COIN, "b-coin", r#"<circle cx="8" cy="8" r="5.8" stroke="currentColor"/><circle cx="8" cy="8" r="2.6" stroke="currentColor"/>"#);
glyph!(SPEAKER, "b-speaker", r#"<path d="M2.5 6.2V9.8H5L8.5 12.5V3.5L5 6.2H2.5Z" stroke="currentColor" stroke-linejoin="round"/><path d="M10.8 5.8C11.6 6.4 12 7.1 12 8C12 8.9 11.6 9.6 10.8 10.2M12.6 4C13.9 5 14.5 6.4 14.5 8C14.5 9.6 13.9 11 12.6 12" stroke="currentColor" stroke-linecap="round"/>"#);
glyph!(DROPPER, "b-dropper", r#"<path d="M9.5 3.2L12.8 6.5M11 2L14 5L12.6 6.4L9.6 3.4L11 2ZM9.4 4.6L3.4 10.6L2.6 13.4L5.4 12.6L11.4 6.6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
glyph!(STOP, "b-stop", r#"<rect x="3.5" y="3.5" width="9" height="9" rx="1.5" fill="currentColor"/>"#);
glyph!(CLEAR, "b-clear", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor"/><path d="M4 12L12 4" stroke="currentColor" stroke-linecap="round"/>"#);

const TABS: [&str; 7] = ["Rencana ibadah", "Pengumuman", "Kalender", "Jemaat", "Persembahan", "Templat", "Tampilan"];
const CAL: [&str; 5] = ["Ibadah umum", "Sekolah Minggu", "Persekutuan pemuda", "Doa", "Musik"];
/// Sunday first, as a Planning Center and ChurchSuite month does.
const DAYS: [&str; 7] = ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"];
const FIRST: [&str; 10] = ["Alfa", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Teta", "Iota", "Kapa"];
/// Made-up schedule: day, time, name, calendar.
const WEEK: [(&str, &str, &str, usize); 6] = [
    ("Minggu", "06.30", "Ibadah Raya 1", 0),
    ("Minggu", "09.00", "Ibadah Raya 2", 0),
    ("Minggu", "09.00", "Sekolah Minggu", 1),
    ("Rabu", "18.30", "Ibadah doa", 3),
    ("Jumat", "18.30", "Persekutuan pemuda", 2),
    ("Sabtu", "17.00", "Latihan pujian", 4),
];
/// Nine made-up gifts: date, giver (empty is the plate), fund, method, rupiah.
const GIFTS: [(&str, &str, usize, &str, u64); 9] = [
    ("01 Nov 2026", "Contoh Alfa Beta", 0, "Transfer", 2_500_000),
    ("01 Nov 2026", "", 1, "Tunai", 1_275_000),
    ("01 Nov 2026", "Contoh Gamma Delta", 2, "Transfer", 5_000_000),
    ("08 Nov 2026", "Contoh Epsilon Zeta", 0, "Transfer", 1_800_000),
    ("08 Nov 2026", "", 1, "Tunai", 1_340_000),
    ("08 Nov 2026", "Contoh Eta Teta", 3, "Transfer", 750_000),
    ("08 Nov 2026", "", 4, "Tunai", 400_000),
    ("08 Nov 2026", "Contoh Iota Kapa", 5, "Transfer", 350_000),
    ("08 Nov 2026", "", 2, "Tunai", 600_000),
];
const FUNDS: [&str; 6] = ["Persepuluhan", "Persembahan umum", "Pembangunan", "Misi", "Diakonia", "Syukur"];

/// A calendar's colour: it belongs to the calendar and never to one event (Planning Center Calendar).
fn hue(i: usize) -> Color {
    [oklch(0.62, 0.15, 255.0), oklch(0.66, 0.15, 150.0), oklch(0.72, 0.15, 55.0), oklch(0.6, 0.17, 300.0), oklch(0.66, 0.17, 0.0), oklch(0.66, 0.11, 200.0)][i % 6]
}

fn t(words: impl Into<String>, style: TextStyle, ink: Color) -> El {
    text(words).font(style).color(ink)
}

fn dot(c: Color, d: f32) -> El {
    el().size(d, d).radius(d / 2.0).fill(c).a11y_hidden()
}

fn rp(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    format!("Rp {out}")
}

fn total() -> u64 {
    GIFTS.iter().map(|g| g.4).sum()
}

fn fund_total(fund: usize) -> u64 {
    GIFTS.iter().filter(|g| g.2 == fund).map(|g| g.4).sum()
}

fn person(i: usize) -> String {
    let a = i % 10;
    let mut b = (i * 3 + i / 10 + 1) % 10;
    if b == a {
        b = (b + 1) % 10;
    }
    format!("Contoh {} {}", FIRST[a], FIRST[b])
}

fn clock(seconds: u32) -> String {
    format!("{:02}.{:02}", seconds / 3600, seconds % 3600 / 60)
}

fn length(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn cap(k: &Kit, words: &str) -> El {
    t(words.to_uppercase(), type_::TINY.tracking(0.8), k.c.text_muted)
}

fn label(k: &Kit, words: &str) -> El {
    t(words, type_::SMALL_STRONG, k.c.text_muted)
}

fn panel(k: &Kit) -> El {
    let sf = theme::surf();
    let _ = k;
    col().radius(10.0).fill(sf.raised).border(1.0, sf.edge)
}

fn picker(k: &Kit, value: &str, lead: Option<&'static Icon>, w: Option<f32>) -> El {
    let mut e = row().h(32.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base);
    if let Some(l) = lead {
        e = e.child(icon(l, 16.0).color(k.c.text_muted).a11y_hidden());
    }
    e = e.child(t(value, type_::BODY, k.c.text_base).truncate().grow(1.0).min_w(0.0)).child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden());
    match w {
        Some(w) => e.w(w),
        None => e,
    }
}

fn day_toggle(k: &Kit, name: &str, on: bool) -> El {
    let e = row().w(46.0).h(34.0).center().radius(k.radius.md);
    if on {
        e.fill(theme::accent()).child(t(name, type_::BODY_STRONG, Color::WHITE))
    } else {
        e.fill(k.c.layer_02).border(1.0, k.c.border_base).child(t(name, type_::BODY, k.c.text_base))
    }
}

fn tick(k: &Kit, on: bool, c: Color) -> El {
    let e = row().size(16.0, 16.0).center().radius(4.0);
    if on { e.fill(c).child(icon(&icons::CHECK, 12.0).color(Color::WHITE)) } else { e.border(1.0, k.c.border_strong) }
}

fn seg(k: &Kit, labels: &[&str], at: usize) -> El {
    SegmentedControl::new(labels, Some(at)).fit().build(k)
}

fn small_btn(k: &Kit, words: &str, glyph: Option<&'static Icon>) -> El {
    let b = btn(words).small();
    let b = match glyph {
        Some(g) => b.icon(g),
        None => b,
    };
    b.build(k, |_, _| {})
}

fn hline() -> El {
    el().h(1.0).w_full().fill(theme::surf().edge).a11y_hidden()
}

fn frame(s: &App, cx: &mut Cx<App>, k: &Kit, tab: usize, body: El) -> El {
    let sf = theme::surf();
    let insets = cx.chrome_insets();
    let mut tabs = row().w_full().h(44.0).px(space::S4).gap(space::S1).center_y().fill(sf.side);
    for (i, name) in TABS.iter().enumerate() {
        let on = i == tab;
        let pill = row().h(30.0).px(space::S3).center_y().radius(15.0);
        tabs = tabs.child(if on {
            pill.fill(theme::accent().alpha(0.2)).child(t(*name, type_::BODY_STRONG, k.c.text_base))
        } else {
            pill.child(t(*name, type_::BODY, k.c.text_muted))
        });
    }
    tabs = tabs.child(spacer()).child(picker(k, "Gereja Contoh", None, Some(180.0)));
    col()
        .key("window")
        .w_full()
        .h_full()
        .fill(sf.chrome)
        .child(super::topbar(s, k, insets, false).fill(sf.chrome))
        .child(hline())
        .child(tabs)
        .child(hline())
        .child(body.w_full().grow(1.0).min_h(0.0).fill(sf.work))
}

pub(super) fn page(s: &App, cx: &mut Cx<App>, k: &Kit, page: u8, layer: &mut Vec<El>) -> El {
    match page {
        1 => frame(s, cx, k, 1, p1(k)),
        2 => frame(s, cx, k, 6, p2(k)),
        3 => {
            let body = p3(k);
            repeat_dialog(k, layer);
            frame(s, cx, k, 2, body)
        }
        4 => frame(s, cx, k, 3, p4(k)),
        5 => frame(s, cx, k, 4, p5(k)),
        6 => frame(s, cx, k, 5, p6(k)),
        _ => frame(s, cx, k, 0, p7(k)),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Schedule,
    Birthdays,
    Money,
    Transfer,
    Plain,
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Hairline,
    Ledger,
    Band,
}

/// A slide drawn small from the church's data: one template reads the window it names. `w` is
/// its width in points; `ink` overrides the title's role colour.
fn slide(kind: Kind, style: Style, w: f32, title_ink: Option<Color>) -> El {
    let f = w / 960.0;
    let (bg, ink, dim, role) = match style {
        Style::Hairline => (oklch(0.19, 0.02, 265.0), oklch(0.97, 0.0, 0.0), oklch(0.72, 0.02, 265.0), oklch(0.8, 0.12, 230.0)),
        Style::Ledger => (oklch(0.96, 0.012, 85.0), oklch(0.25, 0.02, 265.0), oklch(0.5, 0.02, 265.0), oklch(0.46, 0.15, 25.0)),
        Style::Band => (oklch(0.24, 0.07, 265.0), oklch(0.98, 0.0, 0.0), oklch(0.8, 0.04, 265.0), oklch(0.86, 0.15, 85.0)),
    };
    let role = title_ink.unwrap_or(role);
    let ts = |size: f32, weight: f32| TextStyle::new(size * f, weight).lh(size * f * 1.3).features("'tnum' 1");
    let title = match kind {
        Kind::Schedule => "Jadwal ibadah",
        Kind::Birthdays => "Ulang tahun November",
        Kind::Money => "Persembahan",
        Kind::Transfer => "Transfer persembahan",
        Kind::Plain => "Selamat datang",
    };
    let rule = ink.alpha(0.22);
    let mut root = col().w(w).h(w * 9.0 / 16.0).radius(8.0 * f.max(0.5)).fill(bg).clip_rounded(8.0 * f.max(0.5));
    let mut body = col().w_full().px(56.0 * f).pt(30.0 * f).gap(4.0 * f);
    match style {
        Style::Band => {
            root = root.child(row().w_full().h(120.0 * f).px(56.0 * f).center_y().fill(oklch(0.5, 0.17, 262.0)).child(t(title, ts(52.0, 700.0), title_ink.unwrap_or(Color::WHITE))));
        }
        Style::Hairline => {
            body = body.child(t(title, ts(52.0, 700.0), role)).child(el().h((1.5 * f).max(1.0)).w_full().fill(rule).mb(14.0 * f));
        }
        Style::Ledger => {
            body = body.child(t(title, ts(52.0, 700.0), role)).child(el().h((3.0 * f).max(1.0)).w_full().fill(ink).mb(14.0 * f));
        }
    }
    let line = |left: String, mid: String, right: String| {
        row().w_full().h(60.0 * f).center_y().child(t(left, ts(30.0, 400.0), dim).w(210.0 * f)).child(t(mid, ts(30.0, 700.0), role).w(150.0 * f)).child(t(right, ts(30.0, 500.0), ink).grow(1.0).min_w(0.0)).child_if(style == Style::Ledger, || el().pin(Some(0.0), None, Some(0.0), Some(0.0)).h(1.0).fill(rule))
    };
    match kind {
        Kind::Schedule => {
            for (day, time, name, _) in WEEK {
                body = body.child(line(day.to_string(), time.to_string(), name.to_string()));
            }
        }
        Kind::Birthdays => {
            let mut cols = row().w_full().gap(24.0 * f);
            for c in 0..3 {
                let mut one = col().grow(1.0).w(0.0).gap(2.0 * f);
                for i in c * 10..c * 10 + 10 {
                    one = one.child(row().h(33.0 * f).center_y().gap(10.0 * f).child(t(format!("{:02}", i + 1), ts(22.0, 700.0), role).w(34.0 * f)).child(t(person(i), ts(22.0, 400.0), ink).truncate().min_w(0.0)));
                }
                cols = cols.child(one);
            }
            body = body.child(cols);
        }
        Kind::Money => {
            for (i, name) in FUNDS.iter().enumerate().take(4) {
                body = body.child(row().w_full().h(52.0 * f).center_y().child(t(*name, ts(30.0, 400.0), ink).grow(1.0)).child(t(rp(fund_total(i)), ts(30.0, 500.0), ink)));
            }
            body = body.child(el().h((1.5 * f).max(1.0)).w_full().fill(rule).mt(8.0 * f)).child(row().w_full().h(64.0 * f).center_y().child(t("Total", ts(32.0, 700.0), role).grow(1.0)).child(t(rp(total()), ts(36.0, 700.0), role)));
        }
        Kind::Transfer => {
            body = body.child(t("Bank Contoh", ts(34.0, 400.0), dim)).child(t("123 456 7890", ts(84.0, 700.0), role)).child(t("a.n. Gereja Contoh", ts(34.0, 500.0), ink)).child(t("Berita: Persembahan", ts(26.0, 400.0), dim));
        }
        Kind::Plain => {
            body = body.child(t("Kebaktian akan segera dimulai.", ts(42.0, 400.0), ink)).child(t("Mari memuji Tuhan bersama-sama.", ts(42.0, 400.0), dim));
        }
    }
    root.child(body)
}

trait ChildIf {
    fn child_if(self, on: bool, make: impl FnOnce() -> El) -> El;
    fn pad(self, v: f32) -> El;
}

impl ChildIf for El {
    fn child_if(self, on: bool, make: impl FnOnce() -> El) -> El {
        if on { self.child(make()) } else { self }
    }
    fn pad(self, v: f32) -> El {
        self.px(v).py(v)
    }
}

/// The selection frame Planning Center draws round a block: an outline with eight round handles.
fn handles(l: f32, tp: f32, r: f32, b: f32) -> El {
    let ring = theme::accent();
    let handle = |x: Option<f32>, y: Option<f32>, rx: Option<f32>, by: Option<f32>| el().size(9.0, 9.0).radius(4.5).fill(Color::WHITE).border(2.0, ring).pin(x, y, rx, by);
    el()
        .pin(Some(l), Some(tp), Some(r), Some(b))
        .border(2.0, ring)
        .child(handle(Some(-5.0), Some(-5.0), None, None))
        .child(handle(None, Some(-5.0), Some(-5.0), None))
        .child(handle(Some(-5.0), None, None, Some(-5.0)))
        .child(handle(None, None, Some(-5.0), Some(-5.0)))
        .a11y_hidden()
}

// ---------------------------------------------------------------- p1

fn source_row(k: &Kit, glyph: &'static Icon, name: &str, note: &str, on: bool) -> El {
    let e = row().w_full().h(46.0).px(space::S3).gap(space::S3).center_y().radius(k.radius.md);
    let e = if on { e.fill(theme::accent().alpha(0.16)).border(1.5, theme::accent()) } else { e.border(1.0, k.c.border_base) };
    e.child(icon(glyph, 18.0).color(if on { theme::accent() } else { k.c.text_muted }))
        .child(col().grow(1.0).min_w(0.0).child(t(name, type_::BODY_STRONG, k.c.text_base)).child(t(note, type_::SMALL, k.c.text_muted).truncate()))
        .child(if on { icon(&icons::CHECK, 16.0).color(theme::accent()) } else { el().size(16.0, 16.0) })
}

fn template_row(k: &Kit, glyph: &'static Icon, name: &str, from: &str, on: bool) -> El {
    let sf = theme::surf();
    let e = row().w_full().h(52.0).px(space::S3).gap(space::S3).center_y().radius(k.radius.md);
    let e = if on { e.fill(theme::accent().alpha(0.18)) } else { e.hover(k.c.hover) };
    e.child(row().size(32.0, 32.0).center().radius(8.0).fill(if on { theme::accent() } else { sf.header }).child(icon(glyph, 16.0).color(if on { Color::WHITE } else { k.c.text_muted })))
        .child(col().grow(1.0).min_w(0.0).child(t(name, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate()).child(t(from, type_::SMALL, k.c.text_muted).truncate()))
}

fn p1(k: &Kit) -> El {
    let sf = theme::surf();
    let left = col()
        .w(300.0)
        .h_full()
        .fill(sf.side)
        .pad(space::S4)
        .gap(space::S1)
        .child(row().h(32.0).center_y().child(t("Templat pengumuman", type_::TITLE, k.c.text_base)).child(spacer()).child(icon(&icons::PLUS, 16.0).color(k.c.text_muted)))
        .child(t("Pilih satu. Isinya dibaca dari jendela data di sebelah kanan.", type_::SMALL, k.c.text_muted).wrap())
        .child(el().h(space::S2))
        .child(template_row(k, &CALENDAR, "Jadwal ibadah", "Membaca Kalender", true))
        .child(template_row(k, &icons::BADGE, "Ulang tahun", "Membaca Jemaat", false))
        .child(template_row(k, &COIN, "Persembahan", "Membaca Persembahan", false))
        .child(template_row(k, &icons::SEND, "Transfer persembahan", "Membaca Pengaturan gereja", false))
        .child(template_row(k, &icons::ANNOUNCE, "Teks biasa", "Tidak membaca data", false))
        .child(el().h(space::S3))
        .child(label(k, "Gaya"))
        .child(seg(k, &["Hairline", "Ledger", "Band"], 0))
        .child(spacer())
        .child(small_btn(k, "Lihat semua templat", Some(&icons::ARROW_RIGHT)));

    let centre = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(t("Pengumuman", type_::BODY, k.c.text_muted))
                .child(icon(k.glyph.caret_right, 12.0).color(k.c.text_faint))
                .child(t("Jadwal ibadah", type_::HEADING, k.c.text_base))
                .child(spacer())
                .child(row().h(24.0).px(space::S2).gap(6.0).center_y().radius(12.0).fill(theme::accent().alpha(0.16)).child(icon(&CALENDAR, 14.0).color(theme::accent())).child(t("Dibaca dari Kalender, 6 acara", type_::SMALL_STRONG, theme::accent()))),
        )
        .child(el().w(640.0).h(360.0).radius(8.0).child(slide(Kind::Schedule, Style::Hairline, 640.0, None)).child(handles(26.0, 84.0, 26.0, 26.0)))
        .child(t("Isi slide disusun dari Kalender setiap kali ditampilkan. Tidak ada yang diketik di slide.", type_::BODY, k.c.text_muted).wrap().w(640.0))
        .child(
            row()
                .w(640.0)
                .gap(space::S2)
                .child(small_btn(k, "Pratinjau", Some(&icons::PLAY)))
                .child(small_btn(k, "Simpan", None))
                .child(spacer())
                .child(btn("Tampilkan di rencana").icon(&icons::SEND).primary().build(k, |_, _| {})),
        );

    let mut cals = row().w_full().gap(space::S1).flex_wrap();
    for (i, name) in CAL.iter().enumerate() {
        cals = cals.child(row().h(26.0).px(space::S2).gap(6.0).center_y().radius(13.0).fill(theme::tint_of(hue(i), false)).child(icon(&icons::CHECK, 12.0).color(theme::ink_of(hue(i)))).child(t(*name, type_::SMALL_STRONG, theme::ink_of(hue(i)))));
    }
    let mut roles = row().gap(space::S2).center_y();
    for (i, c) in [oklch(0.22, 0.04, 265.0), Color::WHITE, oklch(0.8, 0.12, 230.0), oklch(0.68, 0.15, 256.0), oklch(0.86, 0.16, 92.0)].into_iter().enumerate() {
        let swatch = dot(c, 24.0);
        roles = roles.child(if i == 2 { row().size(32.0, 32.0).center().radius(16.0).border(2.0, theme::accent()).child(swatch) } else { row().size(32.0, 32.0).center().child(swatch.border(1.0, k.c.border_base)) });
    }
    let right = col()
        .w(360.0)
        .h_full()
        .fill(sf.side)
        .pad(space::S4)
        .gap(space::S2)
        .child(row().h(32.0).center_y().child(t("Sumber data", type_::TITLE, k.c.text_base)))
        .child(source_row(k, &CALENDAR, "Kalender", "5 kalender, 6 acara minggu ini", true))
        .child(source_row(k, &icons::BADGE, "Jemaat", "30 ulang tahun bulan ini", false))
        .child(source_row(k, &COIN, "Persembahan", "9 pemberian, jumlah dijumlah otomatis", false))
        .child(el().h(space::S1))
        .child(label(k, "Kalender yang dipakai"))
        .child(cals)
        .child(el().h(space::S1))
        .child(label(k, "Tampilkan"))
        .child(seg(k, &["Minggu ini", "Bulan ini"], 0))
        .child(label(k, "Lama tampil"))
        .child(seg(k, &["10 detik", "15 detik", "20 detik"], 1))
        .child(label(k, "Judul slide"))
        .child(row().h(32.0).px(space::S3).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(t("Jadwal ibadah", type_::BODY, k.c.text_base)))
        .child(label(k, "Warna judul"))
        .child(roles);

    row().w_full().h_full().child(left).child(el().w(1.0).h_full().fill(sf.edge)).child(centre).child(el().w(1.0).h_full().fill(sf.edge)).child(right)
}

// ---------------------------------------------------------------- p2

fn role_row(k: &Kit, name: &str, c: Color, on: bool) -> El {
    let e = row().w_full().h(40.0).px(space::S3).gap(space::S3).center_y().radius(k.radius.md);
    let e = if on { e.fill(theme::accent().alpha(0.18)) } else { e };
    e.child(dot(c, 22.0).border(1.0, k.c.border_base)).child(t(name, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).grow(1.0)).child(icon(&icons::PENCIL, 14.0).color(k.c.text_muted))
}

fn p2(k: &Kit) -> El {
    let sf = theme::surf();
    let gold = oklch(0.8, 0.13, 90.0);
    let roles = [("Latar belakang", oklch(0.22, 0.04, 265.0)), ("Teks", Color::WHITE), ("Judul", gold), ("Aksen", oklch(0.68, 0.15, 256.0)), ("Sorotan", oklch(0.86, 0.16, 92.0))];
    let mut list = col().w_full().gap(2.0);
    for (i, (name, c)) in roles.iter().enumerate() {
        list = list.child(role_row(k, name, *c, i == 2));
    }
    let left = col()
        .w(400.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S3)
        .child(t("Tema warna", type_::HEADING, k.c.text_base))
        .child(picker(k, "Malam biru", Some(&icons::LOOK), None))
        .child(cap(k, "Warna menurut peran"))
        .child(panel(k).w_full().pad(space::S2).child(list))
        .child(cap(k, "Contoh pada slide"))
        .child(slide(Kind::Schedule, Style::Hairline, 360.0, Some(gold)));

    let mut theme_swatches = row().gap(space::S3);
    for (i, (name, c)) in roles.iter().enumerate() {
        let big = dot(*c, 34.0).border(1.0, k.c.border_base);
        let ring = row().size(44.0, 44.0).center().radius(22.0);
        let ring = if i == 2 { ring.border(2.5, theme::accent()) } else { ring };
        theme_swatches = theme_swatches.child(col().items_center().gap(4.0).w(62.0).child(ring.child(big)).child(t(*name, type_::TINY, k.c.text_muted)));
    }
    let mut cal_dots = row().gap(space::S3).flex_wrap();
    for (i, name) in CAL.iter().enumerate() {
        cal_dots = cal_dots.child(row().h(26.0).px(space::S2).gap(6.0).center_y().radius(13.0).border(1.0, k.c.border_base).child(dot(hue(i), 12.0)).child(t(*name, type_::SMALL, k.c.text_base)));
    }
    let lightness = [0.88, 0.76, 0.62, 0.46];
    let hues = [0.0, 25.0, 55.0, 90.0, 140.0, 175.0, 215.0, 256.0, 300.0, 340.0];
    let mut grid = col().gap(6.0);
    for (r, l) in lightness.iter().enumerate() {
        let mut line = row().gap(6.0);
        for (c, h) in hues.iter().enumerate() {
            let sw = el().size(28.0, 28.0).radius(7.0).fill(oklch(*l, 0.14, *h));
            line = line.child(if r == 1 && c == 3 { row().size(28.0, 28.0).radius(7.0).border(2.5, theme::accent()).child(sw.size(22.0, 22.0).radius(5.0)).center() } else { sw });
        }
        grid = grid.child(line);
    }
    let blue = oklch(0.6, 0.2, 256.0);
    let square = el()
        .w(250.0)
        .h(190.0)
        .radius(8.0)
        .fill(blue)
        .clip_rounded(8.0)
        .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, Color::WHITE), (1.0, Color::WHITE.alpha(0.0))]))
        .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).linear_gradient(std::f32::consts::PI, [(0.0, Color::BLACK.alpha(0.0)), (1.0, Color::BLACK)]))
        .child(el().size(16.0, 16.0).radius(8.0).border(2.5, Color::WHITE).shadow(1.0, 3.0, Color::BLACK.alpha(0.5)).pin(Some(176.0), Some(36.0), None, None));
    let stops: Vec<(f32, Color)> = (0..=7).map(|i| (i as f32 / 7.0, oklch(0.68, 0.19, i as f32 * 51.0))).collect();
    let strip = el()
        .w(20.0)
        .h(190.0)
        .radius(10.0)
        .linear_gradient(std::f32::consts::PI, stops)
        .child(el().size(24.0, 24.0).radius(12.0).border(2.5, Color::WHITE).shadow(1.0, 3.0, Color::BLACK.alpha(0.5)).pin(Some(-2.0), Some(112.0), None, None));
    let mut recent = row().gap(6.0).center_y();
    for c in [oklch(0.8, 0.13, 90.0), oklch(0.68, 0.15, 256.0), oklch(0.86, 0.16, 92.0), oklch(0.66, 0.15, 150.0), oklch(0.66, 0.17, 0.0), oklch(0.97, 0.0, 0.0)] {
        recent = recent.child(dot(c, 24.0).border(1.0, k.c.border_base));
    }
    let custom = col()
        .w(300.0)
        .gap(space::S3)
        .child(cap(k, "Warna sendiri"))
        .child(row().gap(space::S3).child(square).child(strip))
        .child(row().gap(space::S2).center_y().child(btn("Pipet").icon(&DROPPER).build(k, |_, _| {})).child(dot(gold, 32.0).border(1.0, k.c.border_base)).child(col().child(t("Kuning emas", type_::BODY_STRONG, k.c.text_base)).child(t("dari peran Judul", type_::SMALL, k.c.text_muted))))
        .child(cap(k, "Terakhir dipakai"))
        .child(recent);
    let preset = col().w(340.0).gap(space::S3).child(cap(k, "Warna tema")).child(theme_swatches).child(cap(k, "Warna kalender")).child(cal_dots).child(cap(k, "Contoh warna")).child(grid);
    let more = row().w_full().h(36.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md).hover(k.c.hover).child(icon(k.glyph.caret_right, 12.0).color(k.c.text_muted)).child(t("Lainnya", type_::BODY, k.c.text_muted)).child(t("kode warna", type_::SMALL, k.c.text_faint));
    let pick = panel(k)
        .w(720.0)
        .shadow(8.0, 24.0, Color::BLACK.alpha(0.25))
        .child(row().w_full().h(48.0).px(space::S4).center_y().child(t("Warna judul", type_::TITLE, k.c.text_base)).child(spacer()).child(icon(&icons::CLOSE, 14.0).color(k.c.text_muted)))
        .child(hline())
        .child(row().w_full().pad(space::S4).gap(space::S5).child(preset).child(el().w(1.0).fill(sf.edge)).child(custom))
        .child(hline())
        .child(col().w_full().px(space::S2).py(space::S1).child(more))
        .child(hline())
        .child(row().w_full().pad(space::S3).gap(space::S2).justify_end().child(btn("Batal").build(k, |_, _| {})).child(btn("Pakai warna").primary().build(k, |_, _| {})));
    row().w_full().h_full().child(left).child(el().w(1.0).h_full().fill(sf.edge)).child(col().grow(1.0).min_w(0.0).h_full().pad(space::S5).child(pick))
}

// ---------------------------------------------------------------- p3

fn event_chip(k: &Kit, time: &str, name: &str, c: Color) -> El {
    let _ = k;
    row()
        .w_full()
        .h(21.0)
        .pr(5.0)
        .gap(5.0)
        .center_y()
        .radius(4.0)
        .fill(theme::tint_of(c, true))
        .clip_soft(0.0)
        .child(el().w(3.0).h_full().fill(c).a11y_hidden())
        .child(t(name, TextStyle::new(11.0, 600.0).lh(14.0), theme::ink_of(c)).truncate().grow(1.0).min_w(0.0))
        .child(t(time, TextStyle::new(11.0, 500.0).lh(14.0).features("'tnum' 1"), theme::ink_of(c)))
}

fn p3(k: &Kit) -> El {
    let sf = theme::surf();
    let mut rail = col().w(250.0).h_full().fill(sf.side).pad(space::S4).gap(space::S2).child(row().h(28.0).center_y().child(t("Kalender", type_::TITLE, k.c.text_base)).child(spacer()).child(small_btn(k, "Kelola", None)));
    for (i, name) in CAL.iter().enumerate() {
        rail = rail.child(row().h(28.0).gap(space::S2).center_y().child(tick(k, true, hue(i))).child(t(*name, type_::BODY, k.c.text_base)));
    }
    rail = rail
        .child(el().h(space::S2))
        .child(label(k, "Lihat menurut"))
        .child(seg(k, &["Kalender", "Jenis"], 0))
        .child(el().h(space::S3))
        .child(label(k, "Dibaca oleh pengumuman"))
        .child(slide(Kind::Schedule, Style::Hairline, 216.0, None))
        .child(t("Templat Jadwal ibadah membaca kalender ini. Ubah acara di sini, slide ikut.", type_::SMALL, k.c.text_muted).wrap());

    let toolbar = row()
        .w_full()
        .h(56.0)
        .px(space::S4)
        .gap(space::S2)
        .center_y()
        .child(small_btn(k, "Hari ini", None))
        .child(btn("Sebelumnya").icon(&icons::ARROW_LEFT).icon_only().small().build(k, |_, _| {}))
        .child(btn("Berikutnya").icon(&icons::ARROW_RIGHT).icon_only().small().build(k, |_, _| {}))
        .child(t("November 2026", type_::HEADING, k.c.text_base).mx(8.0))
        .child(spacer())
        .child(seg(k, &["Minggu", "Bulan"], 1))
        .child(btn("Acara").icon(&icons::PLUS).primary().build(k, |_, _| {}));
    let mut heads = row().w_full().h(26.0);
    for d in DAYS {
        heads = heads.child(row().w(0.0).grow(1.0).px(space::S2).justify_end().center_y().child(cap(k, d)));
    }
    let mut grid = col().w_full().grow(1.0).min_h(0.0).child(hline()).child(heads);
    for week in 0..5 {
        let mut line = row().w_full().grow(1.0).min_h(0.0);
        let mut cells = Vec::new();
        for dow in 0..7 {
            let n = week * 7 + dow + 1;
            let (number, inside) = if n <= 30 { (n, true) } else { (n - 30, false) };
            let today = inside && number == 14;
            let mut cell = col().w(0.0).grow(1.0).h_full().pad(4.0).gap(2.0).border_sides([0.0, 0.0, 1.0, 1.0], sf.edge).fill(if inside { Color::TRANSPARENT } else { sf.side.alpha(0.6) });
            let num = t(number.to_string(), type_::SMALL_STRONG, if today { Color::WHITE } else if inside { k.c.text_base } else { k.c.text_faint });
            cell = cell.child(row().w_full().justify_end().child(if today { row().size(24.0, 24.0).center().radius(12.0).fill(theme::accent()).child(num) } else { row().h(24.0).px(4.0).center_y().child(num) }));
            for (_, time, name, c) in WEEK.iter().filter(|e| e.0 == ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"][dow]) {
                cell = cell.child(event_chip(k, time, name, hue(*c)));
            }
            cells.push(cell);
        }
        for c in cells {
            line = line.child(c);
        }
        grid = grid.child(line).child(hline());
    }
    row().w_full().h_full().child(rail).child(el().w(1.0).h_full().fill(sf.edge)).child(col().grow(1.0).min_w(0.0).h_full().child(toolbar).child(grid))
}

fn repeat_dialog(k: &Kit, layer: &mut Vec<El>) {
    let mut days = row().gap(space::S1);
    for (i, d) in DAYS.iter().enumerate() {
        days = days.child(day_toggle(k, d, i == 0));
    }
    let mut dates = row().w_full().gap(space::S1).flex_wrap();
    for d in ["01 Nov", "08 Nov", "15 Nov", "22 Nov", "29 Nov", "06 Des"] {
        dates = dates.child(row().h(26.0).pl(space::S2).pr(6.0).gap(6.0).center_y().radius(13.0).fill(k.c.layer_03).child(t(d, type_::SMALL_STRONG, k.c.text_base)).child(icon(&icons::CLOSE, 10.0).color(theme::live())));
    }
    dates = dates.child(t("dan 46 tanggal lain", type_::SMALL, k.c.text_muted).mx(8.0)).child(small_btn(k, "Tambah tanggal", Some(&icons::PLUS)));
    let body = col()
        .w_full()
        .px(space::S4)
        .pb(space::S4)
        .gap(space::S3)
        .child(col().gap(4.0).child(label(k, "Nama acara")).child(row().h(32.0).px(space::S3).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, theme::accent()).child(t("Ibadah Raya 1", type_::BODY, k.c.text_base))))
        .child(col().gap(4.0).child(label(k, "Kalender")).child(row().h(32.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(dot(hue(0), 12.0)).child(t("Ibadah umum", type_::BODY, k.c.text_base).grow(1.0)).child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted))))
        .child(col().gap(4.0).child(label(k, "Hari")).child(days))
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .child(col().gap(4.0).grow(1.0).w(0.0).child(label(k, "Mulai")).child(picker(k, "06.30", Some(&icons::HISTORY), None)))
                .child(col().gap(4.0).grow(1.0).w(0.0).child(label(k, "Selesai")).child(picker(k, "08.00", Some(&icons::HISTORY), None))),
        )
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .child(col().gap(4.0).grow(1.0).w(0.0).child(label(k, "Mulai tanggal")).child(picker(k, "01 Nov 2026", Some(&CALENDAR), None)))
                .child(col().gap(4.0).grow(1.0).w(0.0).child(label(k, "Berulang")).child(picker(k, "Setiap minggu", None, None)))
                .child(col().gap(4.0).grow(1.0).w(0.0).child(label(k, "Sampai")).child(picker(k, "Tanpa akhir", Some(&CALENDAR), None))),
        )
        .child(row().w_full().pad(space::S3).radius(k.radius.md).fill(theme::accent().alpha(0.14)).child(t("Setiap hari Minggu pukul 06.30 sampai 08.00, mulai 1 November 2026, tanpa akhir.", type_::BODY_STRONG, k.c.text_base).wrap()))
        .child(col().w_full().gap(4.0).child(label(k, "Tanggal yang dibuat, klik x untuk membuang satu")).child(dates))
        .child(row().w_full().gap(space::S2).justify_end().child(btn("Batal").build(k, |_, _| {})).child(btn("Simpan acara").primary().build(k, |_, _| {})));
    Dialog::new("repeat-event", "Acara berulang").open(true).width(580.0).fit().on_close(|_: &mut App| {}).child(body).build(k, layer);
}

// ---------------------------------------------------------------- p4

fn group_of(age: u32) -> (&'static str, usize) {
    if age < 13 { ("Anak", 1) } else if age < 25 { ("Pemuda", 2) } else { ("Dewasa", 0) }
}

/// A fixed column when `w` is above zero, else a flex share of `-w`.
fn cell(w: f32, end: bool, child: El) -> El {
    let e = row().h_full().px(10.0).center_y().min_w(0.0).clip_soft(0.0);
    let e = if w > 0.0 { e.w(w) } else { e.w(0.0).grow(-w) };
    if end { e.justify_end().child(child) } else { e.child(child) }
}

fn head_cell(k: &Kit, w: f32, end: bool, words: &str, sorted: bool) -> El {
    let mut inner = row().gap(2.0).center_y().child(t(words, type_::SMALL_STRONG, if sorted { k.c.text_base } else { k.c.text_muted }));
    if sorted {
        inner = inner.child(icon(k.glyph.chevron_up, 14.0).color(k.c.text_base));
    }
    cell(w, end, inner)
}

fn p4(k: &Kit) -> El {
    let sf = theme::surf();
    let months = ["Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des"];
    let counts = [18, 22, 25, 19, 21, 28, 24, 23, 26, 20, 30, 27];
    let mut strip = row().w_full().gap(space::S1);
    for (i, m) in months.iter().enumerate() {
        let on = i == 10;
        let e = col().w(0.0).grow(1.0).h(46.0).center().radius(k.radius.md);
        let e = if on { e.fill(theme::accent()) } else { e.border(1.0, k.c.border_base).fill(k.c.layer_02) };
        let (a, b) = if on { (Color::WHITE, Color::WHITE.alpha(0.85)) } else { (k.c.text_base, k.c.text_muted) };
        strip = strip.child(e.child(t(*m, type_::BODY_STRONG, a)).child(t(counts[i].to_string(), type_::TINY, b)));
    }
    let filters = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(row().h(32.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).w(220.0).child(icon(k.glyph.search, 16.0).color(k.c.text_muted)).child(t("Cari nama", type_::BODY, k.c.text_muted)))
        .child(row().h(32.0).pl(space::S3).pr(space::S2).gap(space::S2).center_y().radius(k.radius.md).fill(theme::accent().alpha(0.18)).child(t("Bulan lahir", type_::BODY, k.c.text_muted)).child(t("November", type_::BODY_STRONG, k.c.text_base)).child(icon(&icons::CLOSE, 12.0).color(k.c.text_muted)))
        .child(row().h(32.0).pl(space::S3).pr(space::S2).gap(space::S2).center_y().radius(k.radius.md).fill(k.c.layer_03).child(t("Status", type_::BODY, k.c.text_muted)).child(t("Aktif", type_::BODY_STRONG, k.c.text_base)).child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted)))
        .child(spacer())
        .child(btn("Impor").icon(&icons::IMPORT).build(k, |_, _| {}))
        
        .child(btn("Tambah orang").icon(&icons::PLUS).primary().build(k, |_, _| {}));

    let head = row()
        .w_full()
        .h(30.0)
        .fill(sf.header)
        .child(cell(40.0, false, tick(k, false, theme::accent())))
        .child(head_cell(k, -2.0, false, "Nama", false))
        .child(head_cell(k, 130.0, false, "Tanggal lahir", false))
        .child(head_cell(k, 210.0, false, "Ulang tahun", true))
        .child(head_cell(k, 120.0, false, "Kelompok", false))
        .child(head_cell(k, 70.0, true, "Umur", false))
        .child(cell(40.0, false, el()));
    let mut rows = col().w_full();
    for i in 13..24usize {
        let day = i + 1;
        let year = 1950 + (i * 7) % 62;
        let age = (2026 - year) as u32;
        let today = day == 14;
        let (group, gi) = group_of(age);
        let words = if today { "hari ini".to_string() } else { format!("dalam {} hari", day - 14) };
        let initials = format!("C{}", &FIRST[i % 10][..1]);
        let mut r = row().w_full().h(36.0).center_y();
        r = if today { r.fill(theme::accent().alpha(0.14)) } else if i % 2 == 1 { r.fill(k.c.text_base.alpha(0.03)) } else { r };
        r = r
            .child(cell(40.0, false, tick(k, false, theme::accent())))
            .child(cell(-2.0, false, row().gap(space::S2).center_y().child(row().size(24.0, 24.0).center().radius(12.0).fill(theme::tint_of(hue(i), true)).child(t(initials, type_::TINY, theme::ink_of(hue(i))))).child(t(person(i), if today { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate().min_w(0.0))))
            .child(cell(130.0, false, t(format!("{day:02} Nov {year}"), type_::BODY, k.c.text_base)))
            .child(cell(210.0, false, row().gap(space::S2).center_y().child(t(format!("{day:02} Nov 2026"), type_::BODY, k.c.text_base)).child(t(words, if today { type_::SMALL_STRONG } else { type_::SMALL }, if today { theme::accent() } else { k.c.text_muted }))))
            .child(cell(120.0, false, tag_chip(group, hue(gi))))
            .child(cell(70.0, true, t(age.to_string(), type_::BODY, k.c.text_base)))
            .child(cell(40.0, false, icon(&icons::DOTS, 16.0).color(k.c.text_muted)));
        rows = rows.child(r).child(el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden());
    }
    let table = col().w_full().grow(1.0).min_h(0.0).clip_soft(0.0).child(head).child(hline()).child(rows);
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S3)
        .child(row().w_full().gap(space::S3).center_y().child(t("Jemaat", type_::HEADING, k.c.text_base)).child(t("30 orang lahir di bulan November", type_::BODY, k.c.text_muted)))
        .child(strip)
        .child(filters)
        .child(table)
        .child(row().w_full().h(24.0).center_y().child(t("Menampilkan 11 dari 30, urut ulang tahun terdekat", type_::SMALL, k.c.text_muted)));
    let side = col()
        .w(300.0)
        .h_full()
        .fill(sf.side)
        .pad(space::S4)
        .gap(space::S3)
        .child(t("Dipakai oleh", type_::TITLE, k.c.text_base))
        .child(t("Templat Ulang tahun membaca daftar ini, 30 nama pada satu slide. Tidak ada nama yang diketik di slide.", type_::SMALL, k.c.text_muted).wrap())
        .child(slide(Kind::Birthdays, Style::Ledger, 268.0, None))
        .child(row().w_full().gap(space::S2).center_y().child(icon(&icons::ANNOUNCE, 16.0).color(k.c.text_muted)).child(t("Pengumuman: Ulang tahun November", type_::SMALL_STRONG, k.c.text_base).truncate().min_w(0.0)))
        .child(small_btn(k, "Buka templat", Some(&icons::ARROW_RIGHT)))
        .child(el().h(space::S2))
        .child(t("Impor daftar", type_::TITLE, k.c.text_base))
        .child(t("Dari berkas Excel atau CSV. Langkah berikutnya mencocokkan kolom: nama, tanggal lahir.", type_::SMALL, k.c.text_muted).wrap())
        .child(row().w_full().h(64.0).center().radius(k.radius.md).border(1.5, k.c.border_strong).child(t("Taruh berkas di sini", type_::BODY, k.c.text_muted)));
    row().w_full().h_full().child(main).child(el().w(1.0).h_full().fill(sf.edge)).child(side)
}

// ---------------------------------------------------------------- p5

fn p5(k: &Kit) -> El {
    let sf = theme::surf();
    let big = |figure: String, words: &str| col().gap(2.0).child(t(figure, type_::HEADING, k.c.text_base)).child(t(words, type_::SMALL, k.c.text_muted));
    let filters = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(col().gap(2.0).w(210.0).child(label(k, "Tanggal")).child(picker(k, "01 Nov - 30 Nov 2026", Some(&CALENDAR), None)))
        .child(col().gap(2.0).w(180.0).child(label(k, "Dana")).child(picker(k, "Semua dana", None, None)))
        .child(col().gap(2.0).w(150.0).child(label(k, "Metode")).child(picker(k, "Semua", None, None)))
        .child(spacer())
        .child(big(GIFTS.len().to_string(), "pemberian"))
        .child(el().w(space::S5))
        .child(big(rp(total()), "jumlah"));
    let head = row()
        .w_full()
        .h(32.0)
        .fill(sf.header)
        .child(head_cell(k, 140.0, false, "Tanggal", false))
        .child(head_cell(k, -1.4, false, "Pemberi", false))
        .child(head_cell(k, 190.0, false, "Dana", false))
        .child(head_cell(k, 100.0, false, "Metode", false))
        .child(head_cell(k, 170.0, true, "Jumlah", false))
        .child(cell(40.0, false, el()));
    let mut rows = col().w_full();
    for (i, (date, giver, fund, method, amount)) in GIFTS.iter().enumerate() {
        let editing = i == 3;
        let amount_cell = if editing {
            row().h(28.0).w(140.0).px(space::S2).center_y().justify_end().radius(k.radius.sm).fill(k.c.layer_02).border(1.5, theme::accent()).child(t(rp(*amount), type_::BODY, k.c.text_base))
        } else {
            row().child(t(rp(*amount), type_::BODY, k.c.text_base))
        };
        let mut r = row().w_full().h(36.0).center_y();
        if i % 2 == 1 {
            r = r.fill(k.c.text_base.alpha(0.03));
        }
        r = r
            .child(cell(140.0, false, row().gap(6.0).center_y().child(t(*date, type_::BODY, k.c.text_base)).child(icon(&CALENDAR, 14.0).color(k.c.text_faint))))
            .child(cell(-1.4, false, t(if giver.is_empty() { "Kolekte" } else { giver }, type_::BODY, if giver.is_empty() { k.c.text_muted } else { k.c.text_base }).truncate().min_w(0.0)))
            .child(cell(190.0, false, row().h(22.0).px(space::S2).gap(6.0).center_y().radius(11.0).fill(theme::tint_of(hue(*fund), true)).child(dot(hue(*fund), 8.0)).child(t(FUNDS[*fund], type_::SMALL_STRONG, theme::ink_of(hue(*fund))))))
            .child(cell(100.0, false, t(*method, type_::BODY, k.c.text_base)))
            .child(cell(170.0, true, amount_cell))
            .child(cell(40.0, false, icon(&icons::TRASH, 14.0).color(k.c.text_faint)));
        rows = rows.child(r).child(el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden());
    }
    let summary = row()
        .w_full()
        .h(52.0)
        .px(space::S4)
        .gap(space::S4)
        .center_y()
        .fill(sf.header)
        .child(small_btn(k, "Tambah baris", Some(&icons::PLUS)))
        .child(spacer())
        .child(t("TOTAL", type_::TINY.tracking(0.8), k.c.text_muted))
        .child(t(rp(total()), type_::HEADING, k.c.text_base).w(170.0).align(rune::Align::End))
        .child(el().w(24.0));
    let table = col().w_full().radius(10.0).border(1.0, sf.edge).fill(sf.raised).clip_rounded(10.0).child(head).child(hline()).child(rows).child(summary);
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S3)
        .child(row().w_full().gap(space::S3).center_y().child(t("Persembahan", type_::HEADING, k.c.text_base)).child(t("Ibadah Raya, November 2026", type_::BODY, k.c.text_muted)).child(spacer()).child(small_btn(k, "Ekspor", Some(&icons::EXPORT))))
        .child(filters)
        .child(table)
        .child(t("Jumlah dijumlah sendiri dari baris di atas. Ubah angka di tempat, total ikut berubah.", type_::SMALL, k.c.text_muted));
    let mut funds = col().w_full().gap(6.0);
    for (i, name) in FUNDS.iter().enumerate() {
        funds = funds.child(row().w_full().gap(space::S2).center_y().child(dot(hue(i), 10.0)).child(t(*name, type_::SMALL, k.c.text_base).grow(1.0)).child(t(rp(fund_total(i)), type_::SMALL_STRONG, k.c.text_base)));
    }
    let side = col()
        .w(340.0)
        .h_full()
        .fill(sf.side)
        .pad(space::S4)
        .gap(space::S3)
        .child(t("Dipakai oleh", type_::TITLE, k.c.text_base))
        .child(t("Templat Persembahan membaca tabel ini dan menulis jumlahnya pada slide.", type_::SMALL, k.c.text_muted).wrap())
        .child(slide(Kind::Money, Style::Band, 308.0, None))
        .child(cap(k, "Per dana"))
        .child(funds)
        .child(small_btn(k, "Buka templat", Some(&icons::ARROW_RIGHT)));
    row().w_full().h_full().child(main).child(el().w(1.0).h_full().fill(sf.edge)).child(side)
}

// ---------------------------------------------------------------- p6

fn tile(k: &Kit, kind: Kind, style: Style, name: &str, from: &str, glyph: &'static Icon, on: bool) -> El {
    let w = 330.0;
    let mut frame = el().w(w).h(w * 9.0 / 16.0).radius(8.0).child(slide(kind, style, w, None));
    if on {
        frame = frame
            .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(8.0).border(2.5, theme::accent()).a11y_hidden())
            .child(row().pin(None, Some(10.0), Some(10.0), None).h(30.0).px(space::S3).gap(6.0).center_y().radius(k.radius.md).fill(Color::WHITE).child(icon(&icons::PLUS, 14.0).color(Color::BLACK)).child(t("Pakai templat", type_::BODY_STRONG, Color::BLACK)));
    }
    col().w(w).gap(space::S2).child(frame).child(row().gap(space::S2).center_y().child(row().size(28.0, 28.0).center().radius(14.0).fill(theme::accent().alpha(0.2)).child(icon(glyph, 14.0).color(theme::accent()))).child(col().child(t(name, type_::BODY_STRONG, k.c.text_base)).child(t(from, type_::SMALL, k.c.text_muted))))
}

fn p6(k: &Kit) -> El {
    let sf = theme::surf();
    let mut rail = col().w(220.0).h_full().fill(sf.side).pad(space::S3).gap(2.0);
    for (i, (name, glyph)) in [("Semua templat", &icons::FOLDER), ("Jadwal ibadah", &CALENDAR), ("Ulang tahun", &icons::BADGE), ("Persembahan", &COIN), ("Transfer persembahan", &icons::SEND), ("Teks biasa", &icons::ANNOUNCE)].into_iter().enumerate() {
        let r = row().w_full().h(36.0).px(space::S3).gap(space::S3).center_y().radius(k.radius.md);
        rail = rail.child((if i == 0 { r.fill(theme::accent().alpha(0.18)) } else { r }).child(icon(glyph, 16.0).color(k.c.text_muted)).child(t(name, if i == 0 { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).truncate().min_w(0.0)));
    }
    rail = rail.child(el().h(space::S4)).child(label(k, "Gaya")).child(el().h(2.0));
    for (i, name) in ["Semua", "Hairline", "Ledger", "Band"].into_iter().enumerate() {
        rail = rail.child(row().w_full().h(30.0).px(space::S3).center_y().radius(k.radius.md).child(t(name, if i == 0 { type_::BODY_STRONG } else { type_::BODY }, if i == 0 { k.c.text_base } else { k.c.text_muted })));
    }
    let grid = col()
        .gap(space::S5)
        .child(row().gap(space::S5).child(tile(k, Kind::Schedule, Style::Hairline, "Jadwal ibadah", "Hairline, membaca Kalender", &CALENDAR, true)).child(tile(k, Kind::Birthdays, Style::Ledger, "Ulang tahun", "Ledger, 30 nama pada satu slide", &icons::BADGE, false)).child(tile(k, Kind::Money, Style::Band, "Persembahan", "Band, dengan total", &COIN, false)))
        .child(row().gap(space::S5).child(tile(k, Kind::Transfer, Style::Hairline, "Transfer persembahan", "Hairline, dari Pengaturan", &icons::SEND, false)).child(tile(k, Kind::Plain, Style::Ledger, "Teks biasa", "Ledger", &icons::ANNOUNCE, false)).child(tile(k, Kind::Schedule, Style::Band, "Jadwal ibadah", "Band, membaca Kalender", &CALENDAR, false)));
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S4)
        .child(row().w_full().gap(space::S3).center_y().child(t("Templat pengumuman", type_::HEADING, k.c.text_base)).child(t("Tampil dengan data gereja ini, bukan contoh", type_::BODY, k.c.text_muted)).child(spacer()).child(row().w(240.0).h(32.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).child(icon(k.glyph.search, 16.0).color(k.c.text_muted)).child(t("Cari templat", type_::BODY, k.c.text_muted))))
        .child(grid)
        .child(row().w_full().gap(space::S3).center_y().child(t("Terapkan ke", type_::BODY, k.c.text_muted)).child(seg(k, &["Slide ini", "Semua pengumuman", "Rencana ini"], 0)));
    row().w_full().h_full().child(rail).child(el().w(1.0).h_full().fill(sf.edge)).child(main)
}

// ---------------------------------------------------------------- p7

#[derive(Clone, Copy, PartialEq)]
enum Item {
    Song,
    Video,
    Audio,
    Notice,
    Message,
    Scene,
}

fn item_hue(i: Item) -> Color {
    match i {
        Item::Song => oklch(0.66, 0.15, 150.0),
        Item::Video => oklch(0.62, 0.15, 255.0),
        Item::Audio => oklch(0.66, 0.11, 200.0),
        Item::Notice => oklch(0.6, 0.17, 300.0),
        Item::Message => oklch(0.72, 0.15, 55.0),
        Item::Scene => oklch(0.66, 0.17, 0.0),
    }
}

fn item_name(i: Item) -> &'static str {
    match i {
        Item::Song => "Lagu",
        Item::Video => "Video",
        Item::Audio => "Audio",
        Item::Notice => "Pengumuman",
        Item::Message => "Khotbah",
        Item::Scene => "Adegan",
    }
}

fn item_icon(i: Item) -> &'static Icon {
    match i {
        Item::Song => &icons::SONG,
        Item::Video => &icons::CAMERA,
        Item::Audio => &SPEAKER,
        Item::Notice => &icons::ANNOUNCE,
        Item::Message => &icons::BOOK,
        Item::Scene => &icons::SCREEN,
    }
}

fn volume(k: &Kit, percent: f32) -> El {
    let track = 84.0;
    row()
        .gap(6.0)
        .center_y()
        .child(icon(&SPEAKER, 14.0).color(k.c.text_muted))
        .child(el().w(track).h(4.0).radius(2.0).fill(k.c.layer_03).child(el().w(track * percent / 100.0).h(4.0).radius(2.0).fill(theme::accent())))
        .child(t(format!("{percent:.0}%"), type_::SMALL, k.c.text_muted).w(34.0))
}

fn end_action(k: &Kit, at: usize) -> El {
    let mut r = row().gap(2.0).center_y().child(t("Setelah selesai", type_::SMALL, k.c.text_muted)).child(el().w(4.0));
    for (i, (name, glyph)) in [("Berhenti", &STOP), ("Ulang", &icons::LOOP), ("Lanjut", &icons::ARROW_RIGHT)].into_iter().enumerate() {
        let c = row().h(22.0).px(space::S2).gap(4.0).center_y().radius(k.radius.sm);
        r = r.child(if i == at { c.fill(theme::accent()).child(icon(glyph, 12.0).color(Color::WHITE)).child(t(name, type_::SMALL_STRONG, Color::WHITE)) } else { c.border(1.0, k.c.border_base).child(icon(glyph, 12.0).color(k.c.text_muted)).child(t(name, type_::SMALL, k.c.text_base)) });
    }
    r
}

fn plan_row(k: &Kit, kind: Item, start: u32, len: u32, title: &str, detail: El, h: f32) -> El {
    let c = item_hue(kind);
    row()
        .w_full()
        .h(h)
        .pr(space::S4)
        .gap(space::S3)
        .center_y()
        .fill(theme::tint_of(c, false))
        .child(el().w(4.0).h_full().fill(c).a11y_hidden())
        .child(icon(&icons::GRIP, 16.0).color(k.c.text_faint))
        .child(t(clock(start), type_::CLOCK, k.c.text_muted).w(52.0))
        .child(t(length(len), type_::CLOCK, k.c.text_base).w(48.0))
        .child(row().size(26.0, 26.0).center().radius(13.0).fill(c).child(icon(item_icon(kind), 14.0).color(Color::WHITE)))
        .child(col().grow(1.0).min_w(0.0).gap(2.0).child(t(title, type_::BODY_STRONG, k.c.text_base).truncate()).child(detail))
        .child(tag_chip(item_name(kind), c))
        .child(icon(&icons::DOTS, 16.0).color(k.c.text_muted))
}

fn p7(k: &Kit) -> El {
    let sf = theme::surf();
    let start = 6 * 3600 + 30 * 60;
    let items: [(Item, u32); 7] = [(Item::Video, 90), (Item::Song, 270), (Item::Audio, 180), (Item::Notice, 60), (Item::Notice, 30), (Item::Message, 1500), (Item::Scene, 120)];
    let mut at = [0u32; 7];
    let mut now = start;
    for (i, (_, len)) in items.iter().enumerate() {
        at[i] = now;
        now += len;
    }
    let sum: u32 = items.iter().map(|i| i.1).sum();
    let bar = |words: &str| row().w_full().h(26.0).px(space::S4).center_y().fill(sf.header).child(cap(k, words));
    let note = |words: &str| t(words, type_::SMALL, k.c.text_muted).truncate();
    let source = |words: &str| row().gap(6.0).center_y().child(icon(&CALENDAR, 12.0).color(k.c.text_muted)).child(t(words, type_::SMALL, k.c.text_muted));
    let list = col()
        .w_full()
        .radius(10.0)
        .border(1.0, sf.edge)
        .clip_rounded(10.0)
        .fill(sf.raised)
        .child(bar("Awal ibadah"))
        .child(plan_row(k, Item::Video, at[0], 90, "Selamat datang", row().gap(space::S3).center_y().child(end_action(k, 2)).child(volume(k, 80.0)).child(note("Suara menyala, musik di bawahnya mengecil")), 52.0))
        .child(plan_row(k, Item::Song, at[1], 270, "Contoh Lagu Pembuka", row().gap(6.0).center_y().child(tag_chip("G", item_hue(Item::Song))).child(note("Bait 1, Refren, Bait 2, Refren")), 46.0))
        .child(plan_row(k, Item::Audio, at[2], 180, "Musik doa, instrumen", row().gap(space::S3).center_y().child(end_action(k, 1)).child(volume(k, 40.0)), 52.0))
        .child(plan_row(k, Item::Notice, at[3], 60, "Jadwal ibadah", source("Membaca Kalender, 6 acara"), 46.0))
        .child(plan_row(k, Item::Notice, at[4], 30, "Ulang tahun November", source("Membaca Jemaat, 30 nama"), 46.0))
        .child(plan_row(k, Item::Message, at[5], 1500, "Contoh Khotbah", note("Pembicara: Contoh Alfa Beta"), 46.0))
        .child(bar("Penutup"));

    let layers = [("Latar", "Latar biru malam", Some(&icons::IMAGE)), ("Video penutup", "Terima kasih.mp4, 0:45, berhenti di akhir", Some(&icons::CAMERA)), ("Musik", "Musik penutup.mp3, volume 60%", Some(&SPEAKER)), ("Logo", "Logo gereja, pojok kanan bawah", Some(&icons::LOGO))];
    let mut layer_col = col().grow(1.0).w(0.0).gap(4.0).child(cap(k, "Lapisan"));
    for (kind, what, glyph) in layers {
        layer_col = layer_col.child(row().w_full().h(26.0).gap(space::S2).center_y().child(icon(glyph.unwrap_or(&icons::IMAGE), 14.0).color(item_hue(Item::Scene))).child(t(kind, type_::SMALL_STRONG, k.c.text_base).w(96.0)).child(t(what, type_::SMALL, k.c.text_muted).truncate().grow(1.0).min_w(0.0)).child(icon(&icons::CHECK, 14.0).color(theme::accent())));
    }
    let action = |glyph: &'static Icon, words: &str, c: Color| row().h(24.0).gap(space::S2).center_y().child(icon(glyph, 14.0).color(c)).child(t(words, type_::SMALL, k.c.text_base));
    let actions = col()
        .grow(1.0)
        .w(0.0)
        .gap(2.0)
        .child(cap(k, "Saat adegan tampil"))
        .child(action(&icons::PLAY, "Tampilkan latar dan logo", k.c.text_muted))
        .child(action(&icons::PLAY, "Putar video penutup", k.c.text_muted))
        .child(action(&icons::PLAY, "Mulai musik, volume 60%", k.c.text_muted));
    let clears = col().grow(1.0).w(0.0).gap(2.0).child(cap(k, "Saat selesai, hapus"))
        .child(action(&CLEAR, "Teks dan video", theme::warn_ink()))
        .child(action(&CLEAR, "Musik, memudar 3 detik", theme::warn_ink()))
        .child(action(&CLEAR, "Logo", theme::warn_ink()));
    let c = item_hue(Item::Scene);
    let scene = col().w_full().fill(theme::tint_of(c, false)).child(
        row()
            .w_full()
            .h(46.0)
            .pr(space::S4)
            .gap(space::S3)
            .center_y()
            .child(el().w(4.0).h_full().fill(c).a11y_hidden())
            .child(icon(&icons::GRIP, 16.0).color(k.c.text_faint))
            .child(t(clock(at[6]), type_::CLOCK, k.c.text_muted).w(52.0))
            .child(t(length(120), type_::CLOCK, k.c.text_base).w(48.0))
            .child(row().size(26.0, 26.0).center().radius(13.0).fill(c).child(icon(&icons::SCREEN, 14.0).color(Color::WHITE)))
            .child(t("Penutup", type_::BODY_STRONG, k.c.text_base).grow(1.0))
            .child(tag_chip("Adegan", c))
            .child(icon(k.glyph.chevron_up, 14.0).color(k.c.text_muted)),
    );
    let scene = scene.child(row().w_full().pl(space::S4 + 4.0 + 52.0 + 48.0 + 52.0).pr(space::S4).pb(space::S3).gap(space::S5).child(layer_col).child(actions).child(clears));
    let list = list.child(scene);

    let mut kinds = col().w_full().gap(2.0);
    for i in [Item::Song, Item::Video, Item::Audio, Item::Notice, Item::Message, Item::Scene] {
        kinds = kinds.child(row().w_full().h(30.0).px(space::S2).gap(space::S2).center_y().child(dot(item_hue(i), 14.0)).child(t(item_name(i), type_::BODY, k.c.text_base).grow(1.0)).child(icon(&icons::PENCIL, 13.0).color(k.c.text_muted)));
    }
    let rail = col()
        .w(250.0)
        .h_full()
        .fill(sf.side)
        .pad(space::S4)
        .gap(space::S3)
        .child(row().h(28.0).center_y().child(icon(&icons::HISTORY, 18.0).color(k.c.text_muted)).child(el().w(space::S2)).child(t("Waktu", type_::TITLE, k.c.text_base)).child(spacer()).child(small_btn(k, "Tambah", None)))
        .child(panel(k).w_full().pad(space::S3).gap(space::S2).child(cap(k, "Waktu ibadah")).child(row().gap(space::S2).center_y().child(row().size(28.0, 28.0).center().radius(14.0).fill(item_hue(Item::Song)).child(t("Mg", type_::TINY, Color::WHITE))).child(t("06.30", type_::CLOCK, k.c.text_base))).child(row().gap(space::S2).center_y().child(row().size(28.0, 28.0).center().radius(14.0).fill(item_hue(Item::Notice)).child(t("Sb", type_::TINY, Color::WHITE))).child(t("17.00 Latihan pujian", type_::BODY, k.c.text_base))))
        .child(row().h(28.0).center_y().child(t("Warna jenis item", type_::TITLE, k.c.text_base)).child(spacer()).child(small_btn(k, "Tambah", None)))
        .child(panel(k).w_full().pad(space::S2).child(kinds));
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .pad(space::S5)
        .gap(space::S3)
        .child(
            row()
                .w_full()
                .gap(space::S3)
                .center_y()
                .child(col().child(t("Ibadah Raya 1", type_::HEADING, k.c.text_base)).child(t("Minggu, 15 November 2026", type_::SMALL, k.c.text_muted)))
                .child(tag_chip("Terbit", oklch(0.66, 0.15, 150.0)))
                .child(spacer())
                .child(small_btn(k, "Item", Some(&icons::PLUS)))
                .child(small_btn(k, "Lagu", Some(&icons::SONG)))
                .child(small_btn(k, "Video", Some(&icons::CAMERA)))
                .child(small_btn(k, "Audio", Some(&SPEAKER)))
                .child(small_btn(k, "Adegan", Some(&icons::SCREEN))),
        )
        .child(row().w_full().h(20.0).center_y().child(t(format!("Panjang {}, selesai {}", length(sum), clock(now)), type_::SMALL_STRONG, k.c.text_muted)))
        .child(list);
    row().w_full().h_full().child(rail).child(el().w(1.0).h_full().fill(sf.edge)).child(main)
}
