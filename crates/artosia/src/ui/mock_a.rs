//! UIOVER1 direction A: Inspector, after ProPresenter 7. Sources per page: `~/Shared/inspo/artosia-ui-overhaul/directions.md`.
//! Drawn only by the `mock-a-p1` to `mock-a-p7` scenes; nothing here is reachable in the program.
//!
//! The skeleton is ProPresenter's: the plan on the left with coloured header rows, the slide on a
//! canvas in the middle with a media bin under it, a property inspector with tabs on the right,
//! and the separate windows (Calendar, Themes) as floating dialogs over the window. Every word is
//! Indonesian and hardcoded: this is a mock.

use artosia_core::deck::{NoticeKind, Slide, SlideKind};
use artosia_slides::frame::OutputState;
use artosia_slides::look::{Background, NoticeStyle, presets, rgb};
use artosia_slides::stage::group_color;
use rune::{Color, Cx, El, Icon, ScrollSpec, col, el, icon, oklch, row, spacer, text};
use rune_kit::{Checkbox, Dialog, Kit, SegmentedControl, Select, SelectAppearance, Switch};

use super::widgets::{btn, section_label, state_pill};
use crate::state::App;
use crate::theme::{self, icons, size, space, type_};

// Icons the app's set lacks, drawn on the same 16 point grid.
static CALENDAR: Icon = Icon { name: "mock-a-calendar", view: 16.0, body: r#"<rect x="2" y="3" width="12" height="11" rx="1.5" stroke="currentColor"/><path d="M2 6.5H14M5 1.8V4.2M11 1.8V4.2" stroke="currentColor" stroke-linecap="round"/>"# };
static EYEDROPPER: Icon = Icon { name: "mock-a-eyedropper", view: 16.0, body: r#"<path d="M10.2 2.8L13.2 5.8M11.4 4L5 10.4L4.2 12.8L3.2 13.8L2.2 12.8L3.2 11.8L5.6 11L12 4.6" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/><path d="M9.5 3.5C10.4 2.6 11.6 2.4 12.4 3.2C13.2 4 13 5.2 12.1 6.1" stroke="currentColor" stroke-linecap="round"/>"# };
static SPEAKER: Icon = Icon { name: "mock-a-speaker", view: 16.0, body: r#"<path d="M2.5 6.2V9.8H5L8.5 12.5V3.5L5 6.2H2.5Z" stroke="currentColor" stroke-linejoin="round"/><path d="M10.8 5.8C11.5 6.4 11.8 7.1 11.8 8C11.8 8.9 11.5 9.6 10.8 10.2M12.6 4C13.8 5 14.4 6.4 14.4 8C14.4 9.6 13.8 11 12.6 12" stroke="currentColor" stroke-linecap="round"/>"# };
static LAYERS: Icon = Icon { name: "mock-a-layers", view: 16.0, body: r#"<path d="M8 2L14 5.4L8 8.8L2 5.4L8 2Z" stroke="currentColor" stroke-linejoin="round"/><path d="M2 8.2L8 11.6L14 8.2M2 10.8L8 14.2L14 10.8" stroke="currentColor" stroke-linejoin="round" stroke-linecap="round"/>"# };
static STOP: Icon = Icon { name: "mock-a-stop", view: 16.0, body: r#"<rect x="4" y="4" width="8" height="8" rx="1.2" stroke="currentColor"/>"# };

pub(super) fn page(s: &App, cx: &mut Cx<App>, k: &Kit, page: u8, layer: &mut Vec<El>) -> El {
    match page {
        1 => announcement(s, cx, k, false),
        2 => {
            layer.push(colour_popover(k));
            announcement(s, cx, k, true)
        }
        3 => {
            calendar(k, layer);
            announcement(s, cx, k, false)
        }
        4 => birthdays_page(s, cx, k),
        5 => money_page(s, cx, k),
        6 => {
            themes(s, k, layer);
            announcement(s, cx, k, false)
        }
        _ => rundown(s, cx, k),
    }
}

// ---------------------------------------------------------------- made-up data

const SCHEDULE: [(&str, &str, &str); 6] = [
    ("Minggu", "06.30", "Ibadah Raya 1"),
    ("", "09.00", "Ibadah Raya 2"),
    ("", "09.00", "Sekolah Minggu"),
    ("Rabu", "18.30", "Ibadah doa"),
    ("Jumat", "18.30", "Persekutuan pemuda"),
    ("Sabtu", "17.00", "Latihan pujian"),
];

const GIFTS: [(&str, u64); 9] = [
    ("Kel. Contoh Alfa", 1_500_000),
    ("Ibu Contoh Beta", 500_000),
    ("Bpk. Contoh Gama", 100_000),
    ("Kel. Contoh Delta", 2_000_000),
    ("Kel. Contoh Zeta", 3_000_000),
    ("Ibu Contoh Eta", 400_000),
    ("Ibu Contoh Teta", 500_000),
    ("Ibu Contoh Kapa", 250_000),
    ("Ibu Contoh Lamda", 200_000),
];

/// "Rp 1.500.000".
fn rupiah(amount: u64) -> String {
    let digits = amount.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push('.');
        }
        out.push(digit);
    }
    format!("Rp {out}")
}

/// Thirty names with a day of November each, in day order.
fn birthdays() -> Vec<(String, u32)> {
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

const TODAY: u32 = 14;

fn notice_state(kind: NoticeKind, style: NoticeStyle, look: usize) -> OutputState {
    let (title, body) = match kind {
        NoticeKind::Schedule => ("Jadwal kegiatan gereja", SCHEDULE.iter().map(|(day, time, what)| format!("{day}\t{time}\t{what}")).collect::<Vec<_>>().join("\n")),
        NoticeKind::Birthdays => ("Ulang tahun bulan November", birthdays().iter().map(|(name, day)| format!("{name}\t{day} November")).collect::<Vec<_>>().join("\n")),
        NoticeKind::Money => {
            let total: u64 = GIFTS.iter().map(|(_, amount)| amount).sum();
            let mut lines: Vec<String> = GIFTS.iter().map(|(name, amount)| format!("{name}\t{}", rupiah(*amount))).collect();
            lines.push(format!("== Total\t{}", rupiah(total)));
            ("Persembahan syukur", lines.join("\n"))
        }
        NoticeKind::Account => ("Persembahan lewat transfer", "Bank Contoh\n123 456 7890\na.n. Gereja Contoh".to_string()),
        NoticeKind::Plain => ("Ibadah Padang", "Sabtu, 8 November 2026\nJam 10.00".to_string()),
    };
    let mut look_value = presets()[look].clone();
    look_value.notice_style = style;
    OutputState {
        slide: Some(Slide { kind: SlideKind::Announcement, notice: kind, label: title.to_string(), heading: title.to_string(), text: body, ..Slide::default() }),
        look: look_value,
        ..OutputState::default()
    }
}

// ---------------------------------------------------------------- small parts

fn body_text(k: &Kit, words: impl Into<String>) -> El {
    text(words.into()).font(type_::BODY).color(k.c.text_base).truncate().min_w(0.0)
}

fn muted_text(k: &Kit, words: impl Into<String>) -> El {
    text(words.into()).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0)
}

fn vline() -> El {
    el().w(1.0).h_full().fill(theme::surf().edge).a11y_hidden()
}

fn hline(k: &Kit) -> El {
    el().h(1.0).w_full().fill(k.c.border_muted).a11y_hidden()
}

/// The title strip of a panel.
fn panel_head(k: &Kit, title: &str, sub: &str, trailing: Vec<El>) -> El {
    row()
        .w_full()
        .h(36.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .fill(theme::surf().header)
        .child(text(title).font(type_::BODY_STRONG).color(k.c.text_base).a11y_hidden())
        .children((!sub.is_empty()).then(|| muted_text(k, sub)))
        .child(spacer())
        .children(trailing)
}

/// ProPresenter's tab strip over the inspector, here as words.
fn tabs(k: &Kit, labels: &[&str], on: usize) -> El {
    let mut strip = row().w_full().h(36.0).fill(theme::surf().header);
    for (at, label) in labels.iter().enumerate() {
        let selected = at == on;
        strip = strip.child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .center()
                .role("tab")
                .label(*label)
                .child(text(*label).font(if selected { type_::BODY_STRONG } else { type_::BODY }).color(if selected { k.c.text_base } else { k.c.text_muted }).a11y_hidden())
                .children(selected.then(|| el().pin(Some(12.0), None, Some(12.0), Some(0.0)).h(2.0).fill(theme::accent()).a11y_hidden())),
        );
    }
    strip
}

/// A boxed value: what a field, a picker button or a closed drop-down looks like.
fn field(k: &Kit, value: &str, lead: Option<&'static Icon>, drop: bool) -> El {
    row()
        .w_full()
        .h(32.0)
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .children(lead.map(|glyph| icon(glyph, 16.0).color(k.c.text_muted).a11y_hidden()))
        .child(body_text(k, value).grow(1.0))
        .children(drop.then(|| icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden()))
}

/// A small label over a control.
fn labelled(k: &Kit, label: &str, body: El) -> El {
    col().w_full().gap(4.0).child(section_label(k, label)).child(body)
}

/// A label at the left, the control after it.
fn prop(k: &Kit, label: &str, body: El) -> El {
    row().w_full().min_h(36.0).gap(space::S2).center_y().child(text(label).font(type_::SMALL).color(k.c.text_muted).w(92.0).a11y_hidden()).child(col().grow(1.0).min_w(0.0).child(body))
}

fn chip(k: &Kit, label: &str, on: bool) -> El {
    row()
        .h(26.0)
        .px(10.0)
        .center()
        .radius(13.0)
        .fill(if on { theme::accent() } else { k.c.layer_02 })
        .border(1.0, if on { theme::accent() } else { k.c.border_base })
        .child(text(label).font(type_::SMALL_STRONG).color(if on { Color::WHITE } else { k.c.text_base }))
}

/// A square of colour with a word, the way a colour is shown in a field.
fn colour_button(k: &Kit, color: Color, name: &str, ring: bool) -> El {
    let e = row()
        .w_full()
        .h(32.0)
        .px(8.0)
        .gap(8.0)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .child(el().size(18.0, 18.0).radius(4.0).fill(color).border(1.0, k.c.border_strong).a11y_hidden())
        .child(body_text(k, name).grow(1.0))
        .child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden());
    if ring { e.outline(2.0, 1.0, theme::accent()) } else { e }
}

fn ghost_add(k: &Kit, label: &str) -> El {
    btn(label).icon(&icons::PLUS).ghost().small().build(k, |_, _| {})
}

// ---------------------------------------------------------------- tables

struct Column {
    title: &'static str,
    width: f32,
    right: bool,
}

fn cell(column: &Column, inner: El) -> El {
    let e = row().h_full().px(10.0).center_y().min_w(0.0);
    let e = if column.width > 0.0 { e.w(column.width) } else { e.w(0.0).grow(1.0).shrink() };
    if column.right { e.justify_end().child(inner) } else { e.child(inner) }
}

fn table_head(k: &Kit, columns: &[Column]) -> El {
    let mut head = row().w_full().h(30.0).child(el().pin(Some(0.0), None, Some(0.0), Some(0.0)).h(1.0).fill(k.c.border_weak).a11y_hidden());
    for column in columns {
        head = head.child(cell(column, text(column.title).font(type_::SMALL_STRONG).color(k.c.text_muted).truncate().min_w(0.0)));
    }
    head
}

fn table_row(k: &Kit, columns: &[Column], cells: Vec<El>, height: f32, fill: Option<Color>) -> El {
    let mut r = row().w_full().h(height);
    if let Some(fill) = fill {
        r = r.fill(fill);
    }
    r = r.child(el().pin(Some(0.0), None, Some(0.0), Some(0.0)).h(1.0).fill(k.c.border_weak).a11y_hidden());
    for (column, inner) in columns.iter().zip(cells) {
        r = r.child(cell(column, inner));
    }
    r
}

fn framed(k: &Kit, body: El) -> El {
    body.w_full().radius(k.radius.md).border(1.0, k.c.border_base).clip_soft(0.0)
}

// ---------------------------------------------------------------- the window

/// The operator window of direction A: plan, canvas and inspector, between the real top and status bars.
fn window(s: &App, cx: &mut Cx<App>, k: &Kit, left: (El, f32), centre: El, right: (El, f32)) -> El {
    let sf = theme::surf();
    let rule = || el().h(1.0).w_full().fill(sf.edge).a11y_hidden();
    col()
        .key("window")
        .w_full()
        .h_full()
        .fill(sf.chrome)
        .child(super::topbar(s, k, cx.chrome_insets(), false).fill(sf.chrome))
        .child(rule())
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(left.0.w(left.1).min_w(left.1).h_full().fill(sf.side))
                .child(vline())
                .child(centre.grow(1.0).min_w(0.0).h_full().fill(sf.work))
                .child(vline())
                .child(right.0.w(right.1).min_w(right.1).h_full().fill(sf.side)),
        )
        .child(rule())
        .child(super::statusbar(s, k).fill(sf.chrome))
}

fn plan_head(label: &str, group: usize) -> El {
    let color = group_color(group);
    row().w_full().h(26.0).px(space::S3).center_y().fill(theme::tint_of(color, true)).child(text(label).font(type_::SMALL_STRONG).color(theme::ink_of(color)).truncate().a11y_hidden()).role("heading").label(label.to_string())
}

/// A row of the plan: an icon, a title, a line under it, then whatever belongs at the end.
fn plan_item(k: &Kit, glyph: &'static Icon, title: &str, sub: &str, on: bool, trailing: Vec<El>) -> El {
    let e = row()
        .w_full()
        .h(size::ROW)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .role("row")
        .label(title.to_string())
        .child(icon(glyph, 16.0).color(if on { theme::accent() } else { k.c.text_muted }).a11y_hidden())
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(text(title.to_string()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0).a11y_hidden())
                .children((!sub.is_empty()).then(|| text(sub.to_string()).font(type_::TINY).color(k.c.text_muted).truncate().min_w(0.0).a11y_hidden())),
        )
        .children(trailing);
    if on { e.fill(theme::accent().alpha(0.22)) } else { e }
}

/// A row nested under a plan item, as a layer of a scene is.
fn plan_sub(k: &Kit, glyph: &'static Icon, title: &str, sub: &str) -> El {
    row()
        .w_full()
        .h(30.0)
        .pl(space::S3 + 22.0)
        .pr(space::S3)
        .gap(space::S2)
        .center_y()
        .child(icon(glyph, 14.0).color(k.c.text_muted).a11y_hidden())
        .child(text(title.to_string()).font(type_::SMALL).color(k.c.text_base).truncate().min_w(0.0))
        .child(spacer())
        .child(text(sub.to_string()).font(type_::TINY).color(k.c.text_muted).truncate().min_w(0.0))
}

fn filter_box(k: &Kit) -> El {
    row()
        .w_full()
        .h(30.0)
        .px(space::S2)
        .gap(6.0)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .child(icon(&rune_kit::glyphs::SEARCH, 14.0).color(k.c.text_muted).a11y_hidden())
        .child(text("Saring").font(type_::SMALL).color(k.c.text_muted))
}

/// The plan on the left. `announcement` picks which of the three announcements is open.
fn plan_column(k: &Kit, announcement: usize) -> El {
    let names = [("Jadwal kegiatan gereja", "Jadwal"), ("Ulang tahun November", "30 nama"), ("Persembahan syukur", "9 pemberian")];
    let mut list = col().w_full().child(plan_head("Pembukaan", 5));
    for (at, (title, sub)) in names.iter().enumerate() {
        list = list.child(plan_item(k, &icons::ANNOUNCE, title, sub, at == announcement, vec![]));
    }
    list = list
        .child(plan_head("Pujian", 0))
        .child(plan_item(k, &icons::SONG, "Contoh Pujian Alfa", "4 bagian", false, vec![]))
        .child(plan_item(k, &icons::SONG, "Contoh Pujian Beta", "3 bagian", false, vec![]))
        .child(plan_head("Firman", 1))
        .child(plan_item(k, &icons::BOOK, "Yohanes 3:16 sampai 21", "Alkitab", false, vec![]))
        .child(plan_head("Penutup", 4))
        .child(plan_item(k, &icons::SONG, "Contoh Doa Penutup", "2 bagian", false, vec![]));
    col()
        .child(panel_head(k, "Ibadah Raya 1", "", vec![icon(&icons::PLUS, 16.0).color(k.c.text_muted).a11y_hidden()]))
        .child(hline(k))
        .child(list.grow(1.0).min_h(0.0))
        .child(filter_row(k))
}

fn filter_row(k: &Kit) -> El {
    row().w_full().px(space::S2).py(space::S2).child(filter_box(k))
}

// ---------------------------------------------------------------- page 1: the announcement editor

/// The media bin under the slides: backgrounds as tiles, a looped video among them.
fn media_bin(k: &Kit, height: f32) -> El {
    let sf = theme::surf();
    let mut tiles = row().gap(space::S3).px(space::S3).py(space::S2);
    for (at, look) in presets().iter().enumerate() {
        let fill = |tile: El| match &look.background {
            Background::Solid(c) => tile.fill(rgb(*c)),
            Background::Gradient(a, b) => tile.gradient(rgb(*a), rgb(*b)),
            _ => tile,
        };
        let moving = at == 0;
        let mut face = fill(el().size(96.0, 54.0).radius(4.0).border(if at == 0 { 2.0 } else { 1.0 }, if at == 0 { theme::accent() } else { k.c.border_base }).center());
        if moving {
            face = face.child(icon(&icons::LOOP, 16.0).color(Color::WHITE.alpha(0.85)).a11y_hidden());
        }
        tiles = tiles.child(col().gap(4.0).child(face).child(text(look.name.clone()).font(type_::TINY).color(k.c.text_muted).truncate().w(96.0)));
    }
    col()
        .w_full()
        .h(height)
        .fill(sf.side)
        .child(hline(k))
        .child(
            row()
                .w_full()
                .h(34.0)
                .px(space::S3)
                .gap(space::S4)
                .center_y()
                .child(text("Media").font(type_::SMALL_STRONG).color(k.c.text_muted))
                .child(text("Latar").font(type_::BODY_STRONG).color(k.c.text_base))
                .child(text("Video").font(type_::BODY).color(k.c.text_muted))
                .child(text("Audio").font(type_::BODY).color(k.c.text_muted))
                .child(text("Logo").font(type_::BODY).color(k.c.text_muted))
                .child(spacer())
                .child(el().w(150.0).child(filter_box(k))),
        )
        .child(tiles)
}

fn canvas_card(s: &App, k: &Kit, state: &OutputState, width: f32) -> El {
    el().radius(2.0).border(1.0, k.c.border_strong).shadow(6.0, 18.0, Color::BLACK.alpha(0.35)).child(super::thumb(s, state, width))
}

fn centre_for_notice(s: &App, k: &Kit, title: &str, kind: &str, state: &OutputState, width: f32) -> El {
    col()
        .child(panel_head(
            k,
            title,
            kind,
            vec![btn("Simpan").small().build(k, |_, _| {}), btn("Tampilkan").icon(&icons::PLAY).small().primary().hint("Enter").build(k, |_, _| {})],
        ))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .center()
                .gap(space::S2)
                .child(canvas_card(s, k, state, width))
                .child(row().w(width).child(muted_text(k, "1920 x 1080  |  Lapisan pengumuman")).child(spacer()).child(muted_text(k, "Tampak 100%"))),
        )
        .child(media_bin(k, 178.0))
}

fn announcement(s: &App, cx: &mut Cx<App>, k: &Kit, colour_open: bool) -> El {
    let state = notice_state(NoticeKind::Schedule, NoticeStyle::Hairline, 0);
    let mut kinds = row().w_full().gap(6.0).flex_wrap();
    for (label, on) in [("Jadwal", true), ("Ulang tahun", false), ("Persembahan", false), ("Rekening", false), ("Biasa", false)] {
        kinds = kinds.child(chip(k, label, on));
    }
    let columns = [Column { title: "Hari", width: 82.0, right: false }, Column { title: "Jam", width: 74.0, right: false }, Column { title: "Kegiatan", width: 0.0, right: false }];
    let mut rows = col().w_full().child(table_head(k, &columns));
    for (day, time, what) in SCHEDULE {
        rows = rows.child(table_row(
            k,
            &columns,
            vec![
                if day.is_empty() { muted_text(k, "sama") } else { body_text(k, day) },
                row().gap(4.0).center_y().child(icon(&icons::HISTORY, 12.0).color(k.c.text_muted).a11y_hidden()).child(body_text(k, time)),
                body_text(k, what),
            ],
            30.0,
            None,
        ));
    }
    let inspector = col()
        .child(tabs(k, &["Isi", "Gaya", "Aksi"], 0))
        .child(hline(k))
        .child(
            pane_body()
                .child(labelled(k, "Judul", field(k, "Jadwal kegiatan gereja", None, false)))
                .child(labelled(k, "Jenis", kinds))
                .child(labelled(k, "Gaya", SegmentedControl::new(&["Garis", "Buku", "Pita"], Some(0)).width(None).label("Gaya").build(k)))
                .child(
                    col()
                        .w_full()
                        .gap(4.0)
                        .child(prop(k, "Warna judul", colour_button(k, Color::WHITE, "Putih", colour_open)))
                        .child(prop(k, "Warna aksen", colour_button(k, rgb([124, 196, 106]), "Hijau daun", false))),
                )
                .child(labelled(k, "Baris", col().w_full().gap(4.0).child(framed(k, rows)).child(ghost_add(k, "Tambah baris")))),
        );
    window(s, cx, k, (plan_column(k, 0), size::PLAN), centre_for_notice(s, k, "Pengumuman", "Jadwal kegiatan gereja", &state, 640.0), (inspector, 340.0))
}

/// The scrolling body of an inspector tab.
fn pane_body() -> El {
    col().w_full().grow(1.0).min_h(0.0).px(space::S3).py(space::S3).gap(space::S3)
}

// ---------------------------------------------------------------- page 2: the colour choice

fn hsv(hue: f32, saturation: f32, value: f32) -> Color {
    let sector = (hue / 60.0).rem_euclid(6.0);
    let chroma = value * saturation;
    let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;
    let channel = |v: f32| rune::color::srgb_decode((v + m).clamp(0.0, 1.0));
    Color { r: channel(r), g: channel(g), b: channel(b), a: 1.0 }
}

fn swatch(k: &Kit, color: Color, side: f32, selected: bool) -> El {
    let e = el().size(side, side).radius(4.0).fill(color).border(1.0, k.c.border_strong).role("button");
    if selected { e.outline(2.0, 2.0, theme::accent()) } else { e }
}

fn colour_popover(k: &Kit) -> El {
    let deck: [(&str, Color); 6] = [
        ("Putih", Color::WHITE),
        ("Malam", rgb([16, 24, 48])),
        ("Hijau", rgb([124, 196, 106])),
        ("Ungu", rgb([104, 72, 184])),
        ("Teal", rgb([22, 184, 170])),
        ("Emas", rgb([255, 214, 0])),
    ];
    let mut deck_row = row().w_full().justify_between();
    for (at, (name, color)) in deck.iter().enumerate() {
        deck_row = deck_row.child(col().gap(4.0).items_center().child(swatch(k, *color, 34.0, at == 0)).child(text(*name).font(type_::TINY).color(k.c.text_muted)));
    }
    // A grid of every colour: greys first, then ten hues in five lightnesses.
    let mut grid = col().gap(4.0);
    let mut greys = row().gap(4.0);
    for at in 0..10 {
        greys = greys.child(swatch(k, oklch(0.97 - at as f32 * 0.095, 0.0, 0.0), 24.0, false));
    }
    grid = grid.child(greys);
    for lightness in [0.86, 0.74, 0.62, 0.5, 0.38] {
        let mut line = row().gap(4.0);
        for at in 0..10 {
            line = line.child(swatch(k, oklch(lightness, 0.15, 20.0 + at as f32 * 36.0), 24.0, false));
        }
        grid = grid.child(line);
    }
    // The square picker: the hue, white to the left and black below, then a strip of hues.
    let hue = 215.0;
    let hues: Vec<(f32, Color)> = (0..=6).map(|at| (at as f32 / 6.0, hsv(at as f32 * 60.0, 1.0, 1.0))).collect();
    let square = el()
        .w(276.0)
        .h(116.0)
        .radius(4.0)
        .fill(hsv(hue, 1.0, 1.0))
        .border(1.0, k.c.border_strong)
        .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(4.0).linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, Color::WHITE), (1.0, Color::WHITE.alpha(0.0))]))
        .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(4.0).linear_gradient(std::f32::consts::PI, [(0.0, Color::BLACK.alpha(0.0)), (1.0, Color::BLACK)]))
        .child(el().pin(Some(5.0), Some(5.0), None, None).size(14.0, 14.0).radius(7.0).border(2.0, Color::WHITE).shadow(0.0, 3.0, Color::BLACK.alpha(0.5)));
    let strip = el()
        .w(276.0)
        .h(14.0)
        .radius(7.0)
        .linear_gradient(std::f32::consts::FRAC_PI_2, hues)
        .child(el().pin(Some(215.0 / 360.0 * 262.0), Some(-1.0), None, None).size(16.0, 16.0).radius(8.0).border(2.0, Color::WHITE).fill(hsv(hue, 1.0, 1.0)).shadow(0.0, 3.0, Color::BLACK.alpha(0.5)));
    let eyedropper = row().size(32.0, 32.0).center().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base).role("button").label("Pipet warna").child(icon(&EYEDROPPER, 16.0).color(k.c.text_base).a11y_hidden());
    let more = row().w_full().h(28.0).gap(6.0).center_y().child(icon(k.glyph.caret_right, 14.0).color(k.c.text_muted).a11y_hidden()).child(text("Lainnya").font(type_::SMALL_STRONG).color(k.c.text_muted));
    let panel = col()
        .w(300.0)
        .radius(k.radius.lg)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_strong)
        .shadow(8.0, 28.0, Color::BLACK.alpha(0.45))
        .child(
            row()
                .w_full()
                .h(40.0)
                .px(12.0)
                .center_y()
                .child(text("Warna judul").font(type_::TITLE).color(k.c.text_base))
                .child(spacer())
                .child(icon(&icons::CLOSE, 14.0).color(k.c.text_muted).a11y_hidden()),
        )
        .child(hline(k))
        .child(
            col()
                .w_full()
                .px(12.0)
                .py(12.0)
                .gap(12.0)
                .child(labelled(k, "Warna dek ini", deck_row))
                .child(labelled(k, "Semua warna", grid))
                .child(
                    labelled(
                        k,
                        "Pilih sendiri",
                        col()
                            .w_full()
                            .gap(10.0)
                            .child(square)
                            .child(strip)
                            .child(
                                row()
                                    .w_full()
                                    .gap(8.0)
                                    .center_y()
                                    .child(eyedropper)
                                    .child(el().size(32.0, 32.0).radius(k.radius.md).fill(Color::WHITE).border(1.0, k.c.border_strong).a11y_hidden())
                                    .child(col().grow(1.0).child(text("Putih").font(type_::BODY_STRONG).color(k.c.text_base)).child(muted_text(k, "Pilihan sekarang")))
                                    .child(btn("Pakai").small().primary().build(k, |_, _| {})),
                            ),
                    ),
                )
                .child(more),
        );
    panel.absolute(1366.0 - 340.0 - 300.0 - 10.0, 118.0)
}

// ---------------------------------------------------------------- page 3: the weekly schedule

fn day_toggle(k: &Kit, label: &str, on: bool) -> El {
    row()
        .grow(1.0)
        .h(32.0)
        .center()
        .radius(k.radius.md)
        .fill(if on { theme::accent() } else { k.c.layer_02 })
        .border(1.0, if on { theme::accent() } else { k.c.border_base })
        .role("checkbox")
        .label(label.to_string())
        .child(text(label).font(type_::SMALL_STRONG).color(if on { Color::WHITE } else { k.c.text_base }).a11y_hidden())
}

fn month_grid(k: &Kit) -> El {
    let mut grid = col().gap(2.0).child(
        row()
            .w(196.0)
            .h(28.0)
            .center_y()
            .child(icon(&icons::ARROW_LEFT, 14.0).color(k.c.text_muted).a11y_hidden())
            .child(spacer())
            .child(text("November 2026").font(type_::BODY_STRONG).color(k.c.text_base))
            .child(spacer())
            .child(icon(&icons::ARROW_RIGHT, 14.0).color(k.c.text_muted).a11y_hidden()),
    );
    let mut heads = row();
    for day in ["M", "S", "S", "R", "K", "J", "S"] {
        heads = heads.child(row().size(28.0, 20.0).center().child(text(day).font(type_::TINY).color(k.c.text_muted)));
    }
    grid = grid.child(heads);
    // 1 November 2026 is a Sunday.
    for week in 0..5 {
        let mut line = row();
        for weekday in 0..7 {
            let day = week * 7 + weekday + 1;
            let selected = day == 15;
            let cell = row().size(28.0, 26.0).center().radius(6.0);
            let cell = if day > 30 {
                cell
            } else if selected {
                cell.fill(theme::accent()).child(text(day.to_string()).font(type_::SMALL_STRONG).color(Color::WHITE))
            } else if day == TODAY {
                cell.border(1.0, theme::accent()).child(text(day.to_string()).font(type_::SMALL).color(k.c.text_base))
            } else {
                cell.child(text(day.to_string()).font(type_::SMALL).color(k.c.text_base))
            };
            line = line.child(cell);
        }
        grid = grid.child(line);
    }
    grid
}

fn time_columns(k: &Kit) -> El {
    let pick = |values: &[&str], chosen: usize, title: &str| {
        let mut column = col().gap(2.0).child(row().w(44.0).h(20.0).center().child(text(title.to_string()).font(type_::TINY).color(k.c.text_muted)));
        for (at, value) in values.iter().enumerate() {
            let cell = row().w(44.0).h(26.0).center().radius(6.0);
            column = column.child(if at == chosen {
                cell.fill(theme::accent()).child(text(value.to_string()).font(type_::SMALL_STRONG).color(Color::WHITE))
            } else {
                cell.child(text(value.to_string()).font(type_::SMALL).color(k.c.text_base))
            });
        }
        column
    };
    row().gap(4.0).child(pick(&["07", "08", "09", "10", "11"], 2, "Jam")).child(pick(&["00", "05", "10", "15", "20"], 0, "Menit"))
}

fn calendar(k: &Kit, layer: &mut Vec<El>) {
    let events = [
        ("Ibadah Raya 1", "Minggu 06.30"),
        ("Ibadah Raya 2", "Minggu 09.00"),
        ("Sekolah Minggu", "Minggu 09.00"),
        ("Ibadah doa", "Rabu 18.30"),
        ("Persekutuan pemuda", "Jumat 18.30"),
        ("Latihan pujian", "Sabtu 17.00"),
    ];
    let mut list = col().w_full();
    for (at, (name, when)) in events.iter().enumerate() {
        let e = row().w_full().h(48.0).px(space::S3).center_y().child(
            col().grow(1.0).min_w(0.0).child(text(*name).font(type_::BODY_STRONG).color(k.c.text_base).truncate()).child(text(format!("Setiap {when}")).font(type_::SMALL).color(k.c.text_muted).truncate()),
        );
        list = list.child(if at == 1 { e.fill(theme::accent().alpha(0.22)) } else { e });
    }
    let left = col()
        .w(210.0)
        .h_full()
        .fill(theme::surf().side)
        .child(row().w_full().h(40.0).px(space::S3).center_y().child(Checkbox::new(true).label("Kalender aktif").build(k)))
        .child(hline(k))
        .child(list.grow(1.0))
        .child(hline(k))
        .child(row().w_full().px(space::S2).py(space::S2).child(ghost_add(k, "Tambah kegiatan")));
    let toggles = ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"];
    let mut days = row().w_full().gap(4.0);
    for (at, day) in toggles.iter().enumerate() {
        days = days.child(day_toggle(k, day, at == 0));
    }
    let middle = col()
        .w(396.0)
        .h_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S3)
        .child(labelled(k, "Nama", field(k, "Ibadah Raya 2", None, false)))
        .child(
            labelled(
                k,
                "Mulai",
                col()
                    .w_full()
                    .gap(6.0)
                    .child(row().w_full().gap(8.0).child(el().grow(1.0).child(field(k, "Min, 15 Nov 2026", Some(&CALENDAR), true))).child(el().w(112.0).child(field(k, "09.00", Some(&icons::HISTORY), true))))
                    .child(
                        card(k).child(row().gap(12.0).child(month_grid(k)).child(el().w(1.0).h(150.0).fill(k.c.border_muted)).child(time_columns(k))),
                    ),
            ),
        )
        .child(labelled(k, "Ulangi setiap", col().w_full().gap(6.0).child(days).child(muted_text(k, "Setiap Minggu pukul 09.00, tanpa akhir"))))
        .child(Checkbox::new(false).label("Berakhir pada tanggal").build(k))
        .child(labelled(
            k,
            "Aksi",
            row().w_full().gap(8.0).child(Select::new("cal-action", &["Buka rencana: Ibadah Raya 2"]).current(Some("Buka rencana: Ibadah Raya 2")).appearance(SelectAppearance::Field).width(250.0).trigger(k)).child(btn("Coba").build(k, |_, _| {})),
        ));
    // The week the schedule makes.
    let week: [(&str, Vec<(&str, &str, usize)>); 7] = [
        ("Min", vec![("06.30", "Ibadah Raya 1", 0), ("09.00", "Ibadah Raya 2", 0), ("09.00", "Sekolah Minggu", 1)]),
        ("Sen", vec![]),
        ("Sel", vec![]),
        ("Rab", vec![("18.30", "Ibadah doa", 2)]),
        ("Kam", vec![]),
        ("Jum", vec![("18.30", "Persekutuan pemuda", 3)]),
        ("Sab", vec![("17.00", "Latihan pujian", 4)]),
    ];
    let mut grid = col().w_full();
    for (day, items) in week {
        let mut line = row().w_full().h(52.0).gap(6.0).center_y().child(text(day).font(type_::SMALL_STRONG).color(k.c.text_muted).w(34.0));
        for (time, name, group) in items {
            let color = group_color(group);
            let chosen = name == "Ibadah Raya 2";
            let chip = row().h(34.0).px(8.0).gap(6.0).center_y().radius(6.0).fill(theme::tint_of(color, true)).child(text(time).font(type_::SMALL_STRONG).color(theme::ink_of(color))).child(text(name).font(type_::SMALL).color(k.c.text_base).truncate().min_w(0.0));
            line = line.child(if chosen { chip.outline(2.0, 1.0, theme::accent()) } else { chip });
        }
        grid = grid.child(line).child(el().h(1.0).w_full().fill(k.c.border_muted).a11y_hidden());
    }
    let right = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S3)
        .fill(theme::surf().side)
        .child(row().w_full().child(text("Seminggu").font(type_::TITLE).color(k.c.text_base)).child(spacer()).child(muted_text(k, "Yang berikutnya: Minggu 15 Nov, 06.30")))
        .child(grid);
    Dialog::new("calendar", "Kalender")
        .width(1100.0)
        .fit()
        .open(true)
        .on_close(|_: &mut App| {})
        .child(row().w_full().h(540.0).child(left).child(vline()).child(middle).child(vline()).child(right))
        .build(k, layer);
}

/// A bordered card on the second layer.
fn card(k: &Kit) -> El {
    col().w_full().padding(8.0).radius(k.radius.md).border(1.0, k.c.border_base).fill(k.c.layer_02)
}

// ---------------------------------------------------------------- page 4: birthdays

fn avatar(name: &str, at: usize) -> El {
    let initial = name.strip_prefix("Contoh ").unwrap_or(name).chars().next().unwrap_or('A').to_string();
    el().size(26.0, 26.0).radius(13.0).center().fill(oklch(0.72, 0.09, 30.0 + at as f32 * 47.0)).child(text(initial).font(type_::SMALL_STRONG).color(Color::BLACK.alpha(0.7)))
}

fn birthdays_page(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let state = notice_state(NoticeKind::Birthdays, NoticeStyle::Ledger, 0);
    let people = birthdays();
    let group_of = |day: u32| match day {
        TODAY => 0,
        15..=21 => 1,
        22.. => 2,
        _ => 3,
    };
    let names = ["Hari ini", "Minggu ini", "Nanti bulan ini", "Sudah lewat"];
    let mut list = col().w_full().gap(0.0);
    let mut index = 0;
    for (group, name) in names.iter().enumerate() {
        let members: Vec<&(String, u32)> = people.iter().filter(|(_, day)| group_of(*day) == group).collect();
        list = list.child(
            row().w_full().h(30.0).px(space::S3).gap(6.0).center_y().fill(k.c.layer_01).child(text(*name).font(type_::SMALL_STRONG).color(k.c.text_muted)).child(state_pill(&members.len().to_string(), k.c.layer_03, k.c.text_muted)),
        );
        for (person, day) in members {
            list = list.child(
                row()
                    .w_full()
                    .h(38.0)
                    .px(space::S3)
                    .gap(space::S2)
                    .center_y()
                    .child(avatar(person, index))
                    .child(body_text(k, person.clone()).grow(1.0))
                    .child(row().h(26.0).px(8.0).gap(6.0).center_y().radius(k.radius.md).border(1.0, k.c.border_base).child(icon(&CALENDAR, 14.0).color(k.c.text_muted).a11y_hidden()).child(text(format!("{day} Nov")).font(type_::SMALL).color(k.c.text_base)))
                    .child(icon(&icons::TRASH, 14.0).color(k.c.text_faint).a11y_hidden()),
            );
            index += 1;
        }
    }
    let import = card(k)
        .gap(8.0)
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(icon(&icons::IMPORT, 16.0).color(k.c.text_base).a11y_hidden())
                .child(col().grow(1.0).child(text("Impor dari berkas").font(type_::BODY_STRONG).color(k.c.text_base)).child(muted_text(k, "daftar-ultah.xlsx, 30 baris")))
                .child(btn("Pilih berkas").small().build(k, |_, _| {})),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(text("Nama dari").font(type_::SMALL).color(k.c.text_muted))
                .child(Select::new("map-name", &["Kolom A"]).current(Some("Kolom A")).appearance(SelectAppearance::Field).width(110.0).trigger(k))
                .child(text("Tanggal dari").font(type_::SMALL).color(k.c.text_muted))
                .child(Select::new("map-date", &["Kolom B"]).current(Some("Kolom B")).appearance(SelectAppearance::Field).width(110.0).trigger(k)),
        );
    let inspector = col()
        .child(tabs(k, &["Isi", "Data", "Gaya"], 1))
        .child(hline(k))
        .child(
            pane_body()
                .child(import)
                .child(SegmentedControl::new(&["Hari ini", "Besok", "Mendatang", "Semua"], Some(3)).width(None).label("Tampilkan").build(k))
                .child(framed(k, col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(list)).grow(1.0).min_h(0.0))
                .child(ghost_add(k, "Tambah nama")),
        );
    window(s, cx, k, (plan_column(k, 1), size::PLAN), centre_for_notice(s, k, "Pengumuman", "Ulang tahun bulan November", &state, 640.0), (inspector, 430.0))
}

// ---------------------------------------------------------------- page 5: money

fn money_page(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let state = notice_state(NoticeKind::Money, NoticeStyle::Band, 0);
    let columns = [Column { title: "Nama", width: 0.0, right: false }, Column { title: "Jumlah", width: 140.0, right: true }, Column { title: "", width: 36.0, right: true }];
    let mut rows = col().w_full().child(table_head(k, &columns));
    for (at, (name, amount)) in GIFTS.iter().enumerate() {
        let editing = at == 2;
        let amount_cell = if editing {
            row().h(26.0).px(6.0).gap(2.0).center_y().radius(4.0).outline(1.5, 0.0, theme::accent()).child(text(rupiah(*amount)).font(type_::BODY).color(k.c.text_base)).child(el().w(1.5).h(16.0).fill(theme::accent()))
        } else {
            row().child(text(rupiah(*amount)).font(type_::BODY).color(k.c.text_base))
        };
        rows = rows.child(table_row(k, &columns, vec![body_text(k, *name), amount_cell, icon(&icons::TRASH, 14.0).color(k.c.text_faint).a11y_hidden()], 32.0, None));
    }
    // The total sums itself: nobody types it.
    let total: u64 = GIFTS.iter().map(|(_, amount)| amount).sum();
    rows = rows.child(table_row(
        k,
        &columns,
        vec![text("Total").font(type_::BODY_STRONG).color(k.c.text_base), text(rupiah(total)).font(type_::BODY_STRONG).color(k.c.text_base), el()],
        36.0,
        Some(k.c.layer_03),
    ));
    let inspector = col()
        .child(tabs(k, &["Isi", "Data", "Gaya"], 1))
        .child(hline(k))
        .child(
            pane_body()
                .child(row().w_full().gap(8.0).center_y().child(col().grow(1.0).child(text("Pemberian").font(type_::TITLE).color(k.c.text_base)).child(muted_text(k, "9 baris, diubah langsung di tabel"))).child(btn("Impor").icon(&icons::IMPORT).small().build(k, |_, _| {})))
                .child(framed(k, rows))
                .child(ghost_add(k, "Tambah baris"))
                .child(muted_text(k, "Total dijumlah sendiri; tidak diketik.")),
        );
    window(s, cx, k, (plan_column(k, 2), size::PLAN), centre_for_notice(s, k, "Pengumuman", "Persembahan syukur", &state, 640.0), (inspector, 430.0))
}

// ---------------------------------------------------------------- page 6: the template gallery

fn radio(k: &Kit, label: &str, on: bool) -> El {
    row()
        .w_full()
        .h(30.0)
        .gap(8.0)
        .center_y()
        .child(el().size(16.0, 16.0).radius(8.0).border(if on { 5.0 } else { 1.5 }, if on { theme::accent() } else { k.c.border_strong }).a11y_hidden())
        .child(text(label.to_string()).font(type_::BODY).color(k.c.text_base))
}

fn themes(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let roles = [("Jadwal", NoticeKind::Schedule), ("Ulang tahun", NoticeKind::Birthdays), ("Persembahan", NoticeKind::Money), ("Rekening", NoticeKind::Account), ("Biasa", NoticeKind::Plain)];
    let mut list = col().w_full();
    for (at, (name, _)) in roles.iter().enumerate() {
        let e = row().w_full().h(36.0).px(space::S3).gap(8.0).center_y().child(icon(&icons::ANNOUNCE, 14.0).color(k.c.text_muted).a11y_hidden()).child(text(*name).font(type_::BODY).color(k.c.text_base));
        list = list.child(if at == 2 { e.fill(theme::accent().alpha(0.22)) } else { e });
    }
    let left = col().w(190.0).h_full().fill(theme::surf().side).child(row().w_full().h(36.0).px(space::S3).center_y().child(section_label(k, "Jenis pengumuman"))).child(hline(k)).child(list);
    let styles = [("Garis", NoticeStyle::Hairline), ("Buku", NoticeStyle::Ledger), ("Pita", NoticeStyle::Band)];
    let mut gallery = col().w_full().gap(10.0);
    for (name, style) in styles {
        let mut line = row().gap(12.0);
        for (look, look_name) in [(0, "Malam"), (3, "Hutan")] {
            let chosen = style == NoticeStyle::Ledger && look == 0;
            let tile = el().radius(3.0).border(if chosen { 2.0 } else { 1.0 }, if chosen { theme::accent() } else { k.c.border_strong }).child(super::thumb(s, &notice_state(NoticeKind::Money, style, look), 236.0));
            line = line.child(col().gap(4.0).child(tile).child(row().w(236.0).child(text(name).font(type_::SMALL_STRONG).color(k.c.text_base)).child(spacer()).child(muted_text(k, look_name))));
        }
        gallery = gallery.child(line);
    }
    let centre = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S3)
        .child(row().w_full().child(text("Gaya untuk Persembahan, dengan data gereja ini").font(type_::BODY_STRONG).color(k.c.text_base)))
        .child(gallery);
    let right = col()
        .w(330.0)
        .h_full()
        .fill(theme::surf().side)
        .px(space::S3)
        .py(space::S3)
        .gap(space::S3)
        .child(el().radius(3.0).border(1.0, k.c.border_strong).child(super::thumb(s, &notice_state(NoticeKind::Money, NoticeStyle::Ledger, 0), 304.0)))
        .child(col().w_full().gap(2.0).child(text("Buku, latar Malam").font(type_::TITLE).color(k.c.text_base)).child(muted_text(k, "Tab di sisi dan tabel bergaris")))
        .child(labelled(k, "Pakai untuk", col().w_full().child(radio(k, "Pengumuman ini saja", false)).child(radio(k, "Semua pengumuman Persembahan", true)).child(radio(k, "Semua pengumuman", false))))
        .child(spacer())
        .child(row().w_full().gap(8.0).justify_end().child(btn("Batal").build(k, |_, _| {})).child(btn("Pakai").primary().build(k, |_, _| {})));
    Dialog::new("themes", "Tema pengumuman")
        .width(1100.0)
        .fit()
        .open(true)
        .on_close(|_: &mut App| {})
        .child(row().w_full().h(548.0).child(left).child(vline()).child(centre).child(vline()).child(right))
        .build(k, layer);
}

// ---------------------------------------------------------------- page 7: the rundown

fn slider(k: &Kit, fraction: f32, width: f32) -> El {
    el()
        .w(width)
        .h(18.0)
        .role("slider")
        .child(el().w(width).h(4.0).radius(2.0).fill(k.c.layer_03).border(1.0, k.c.border_base).absolute(0.0, 7.0))
        .child(el().w(width * fraction).h(4.0).radius(2.0).fill(theme::accent()).absolute(0.0, 7.0))
        .child(el().size(16.0, 16.0).radius(8.0).fill(Color::WHITE).shadow(1.0, 4.0, Color::BLACK.alpha(0.4)).absolute(width * fraction - 8.0, 1.0))
}

/// A small word with an icon, as the end action of an item shows in the plan.
fn action_chip(k: &Kit, glyph: &'static Icon, label: &str) -> El {
    row().h(22.0).px(6.0).gap(4.0).center_y().radius(6.0).fill(k.c.layer_03).child(icon(glyph, 12.0).color(k.c.text_muted).a11y_hidden()).child(text(label.to_string()).font(type_::TINY).color(k.c.text_base))
}

fn scene_picture(k: &Kit, width: f32) -> El {
    let (w, h) = (width, (width * 9.0 / 16.0).round());
    let look = &presets()[0];
    let Background::Gradient(a, b) = look.background else { return el() };
    el()
        .size(w, h)
        .radius(2.0)
        .border(1.0, k.c.border_strong)
        .gradient(rgb(a), rgb(b))
        .child(el().absolute(w * 0.2, h * 0.14).size(w * 0.6, h * 0.62).radius(3.0).fill(oklch(0.34, 0.05, 250.0)).border(1.0, Color::WHITE.alpha(0.2)).center().child(icon(&icons::PLAY, 28.0).color(Color::WHITE.alpha(0.85))))
        .child(row().absolute(w * 0.2, h * 0.82).gap(6.0).center_y().child(icon(&icons::LOGO, 18.0).color(Color::WHITE)).child(text("Gereja Contoh").font(type_::BODY_STRONG).color(Color::WHITE)))
}

fn rundown(s: &App, cx: &mut Cx<App>, k: &Kit) -> El {
    let plan = col()
        .child(panel_head(k, "Ibadah Raya 1", "", vec![icon(&icons::PLUS, 16.0).color(k.c.text_muted).a11y_hidden()]))
        .child(hline(k))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(plan_head("Pujian", 0))
                .child(plan_item(k, &icons::SONG, "Contoh Pujian Alfa", "Lagu, 4 bagian", false, vec![]))
                .child(plan_item(k, &icons::CAMERA, "Video kesaksian", "Video, 3:20", false, vec![action_chip(k, &STOP, "Berhenti"), action_chip(k, &SPEAKER, "80%")]))
                .child(plan_item(k, &SPEAKER, "Musik doa", "Audio, 4:05", false, vec![action_chip(k, &icons::LOOP, "Ulang"), action_chip(k, &SPEAKER, "60%")]))
                .child(plan_head("Penutup", 4))
                .child(plan_item(k, &LAYERS, "Penutup", "Adegan, 4 lapisan", true, vec![]))
                .child(plan_sub(k, &icons::IMAGE, "Latar", "Malam"))
                .child(plan_sub(k, &icons::CAMERA, "Video penutup", "0:45"))
                .child(plan_sub(k, &SPEAKER, "Musik", "doa-penutup"))
                .child(plan_sub(k, &icons::BADGE, "Logo", "Gereja Contoh")),
        )
        .child(filter_row(k));

    // Looks: which layer shows on which screen, after ProPresenter's Looks window.
    let looks_columns = [Column { title: "Lapisan", width: 0.0, right: false }, Column { title: "Layar utama", width: 104.0, right: false }, Column { title: "Panggung", width: 88.0, right: false }];
    let mut looks = col().w_full().child(table_head(k, &looks_columns));
    for (name, main, stage) in [("Latar", true, false), ("Video", true, false), ("Musik", true, true), ("Logo", true, false)] {
        looks = looks.child(table_row(k, &looks_columns, vec![body_text(k, name), Checkbox::new(main).hide_label().label(name).build(k), Checkbox::new(stage).hide_label().label(name).build(k)], 30.0, None));
    }
    let layers_columns = [
        Column { title: "Lapisan", width: 130.0, right: false },
        Column { title: "Isi", width: 0.0, right: false },
        Column { title: "Mulai", width: 130.0, right: false },
        Column { title: "Setelah selesai", width: 200.0, right: false },
    ];
    let mut layers = col().w_full().child(table_head(k, &layers_columns));
    let rows: [(&'static Icon, &str, &str, &str, (&'static Icon, &str)); 4] = [
        (&icons::IMAGE, "Latar", "Malam, gradien", "Saat tampil", (&icons::PLAY, "Tetap")),
        (&icons::CAMERA, "Video penutup", "penutup.mp4, suara 80%", "Setelah 2 detik", (&STOP, "Bersihkan video")),
        (&SPEAKER, "Musik", "doa-penutup.mp3", "Bersama video", (&icons::HISTORY, "Pudar 3 d, lalu bersihkan")),
        (&icons::BADGE, "Logo", "Gereja Contoh", "Video selesai", (&icons::CLOSE, "Bersihkan setelah 10 detik")),
    ];
    for (at, (glyph, name, what, start, (end_icon, end))) in rows.into_iter().enumerate() {
        let r = table_row(
            k,
            &layers_columns,
            vec![row().gap(6.0).center_y().child(icon(glyph, 14.0).color(k.c.text_muted).a11y_hidden()).child(body_text(k, name)), body_text(k, what), body_text(k, start), action_chip(k, end_icon, end)],
            34.0,
            None,
        );
        layers = layers.child(if at == 1 { r.fill(theme::accent().alpha(0.22)) } else { r });
    }
    let centre = col()
        .child(panel_head(k, "Adegan Penutup", "Tampilan: Utama", vec![btn("Bersihkan").icon(&icons::CLOSE).small().build(k, |_, _| {}), btn("Tampilkan adegan").icon(&icons::PLAY).small().primary().build(k, |_, _| {})]))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .px(space::S3)
                .py(space::S3)
                .gap(space::S3)
                .child(
                    row()
                        .w_full()
                        .gap(space::S4)
                        .child(scene_picture(k, 320.0))
                        .child(col().grow(1.0).min_w(0.0).gap(4.0).child(section_label(k, "Tampilan (Looks)")).child(framed(k, looks))),
                )
                .child(labelled(k, "Lapisan dan aksi", col().w_full().gap(4.0).child(framed(k, layers)).child(ghost_add(k, "Tambah lapisan")))),
        )
        .child(media_bin_for(k));
    let inspector = col()
        .child(tabs(k, &["Item", "Aksi", "Tampilan"], 0))
        .child(hline(k))
        .child(
            pane_body()
                .child(row().w_full().gap(8.0).center_y().child(icon(&icons::CAMERA, 16.0).color(theme::accent()).a11y_hidden()).child(text("Video penutup").font(type_::TITLE).color(k.c.text_base)))
                .child(labelled(k, "Berkas", row().w_full().gap(6.0).child(el().grow(1.0).child(field(k, "penutup.mp4", Some(&icons::CAMERA), false))).child(btn("Ganti").build(k, |_, _| {}))))
                .child(labelled(k, "Setelah selesai", SegmentedControl::new(&["Berhenti", "Ulang", "Lanjut"], Some(0)).width(None).label("Setelah selesai").build(k)))
                .child(muted_text(k, "Berhenti di gambar terakhir, adegan tetap tampil."))
                .child(labelled(
                    k,
                    "Suara video",
                    row().w_full().gap(8.0).center_y().child(icon(&SPEAKER, 16.0).color(k.c.text_base).a11y_hidden()).child(slider(k, 0.8, 200.0)).child(text("80%").font(type_::BODY_STRONG).color(k.c.text_base)),
                ))
                .child(Switch::new(true).label("Kecilkan musik selama video bersuara").build(k))
                .child(labelled(k, "Musik turun ke", row().w_full().gap(8.0).center_y().child(icon(&SPEAKER, 16.0).color(k.c.text_muted).a11y_hidden()).child(slider(k, 0.2, 200.0)).child(text("20%").font(type_::BODY_STRONG).color(k.c.text_base))))
                .child(labelled(k, "Mulai", Select::new("scene-start", &["Setelah 2 detik"]).current(Some("Setelah 2 detik")).appearance(SelectAppearance::Field).width(316.0).trigger(k)))
                .child(
                    labelled(
                        k,
                        "Setelah video selesai",
                        col().w_full().gap(6.0).child(Checkbox::new(true).label("Bersihkan video").build(k)).child(Checkbox::new(true).label("Pudarkan lalu bersihkan musik").build(k)).child(Checkbox::new(false).label("Bersihkan logo").build(k)),
                    ),
                ),
        );
    window(s, cx, k, (plan, 300.0), centre, (inspector, 340.0))
}

fn media_bin_for(k: &Kit) -> El {
    media_bin(k, 150.0)
}
