//! LYRICSEDIT1: three directions for the song editor. Sources per page:
//! `~/Shared/inspo/artosia-lyrics-editor/directions.md`. Drawn only by the `lyrics-*` scenes;
//! nothing here is reachable in the program. The song and its words are made up for Artosia, in
//! Indonesian, because this is a picture to compare and never ships.

use rune::{Align, Color, Cx, El, TextStyle, col, el, icon, oklch, row, spacer, text};
use rune_kit::{Kit, SegmentedControl};

use super::widgets::btn;
use crate::state::App;
use crate::theme::{self, icons, space, type_};

/// The kinds of part, each with its colour; the colour follows the part everywhere.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Verse,
    Pre,
    Chorus,
    Bridge,
    End,
}

impl Kind {
    fn colour(self) -> Color {
        match self {
            Kind::Verse => oklch(0.64, 0.14, 255.0),
            Kind::Pre => oklch(0.68, 0.11, 195.0),
            Kind::Chorus => oklch(0.64, 0.18, 350.0),
            Kind::Bridge => oklch(0.74, 0.15, 60.0),
            Kind::End => oklch(0.62, 0.09, 300.0),
        }
    }
}

/// One part: its kind, its name, and its slides, each a few lines.
struct Part {
    kind: Kind,
    name: &'static str,
    slides: &'static [&'static [&'static str]],
    english: &'static [&'static [&'static str]],
}

/// A made-up song in the worst shape the editor meets: three verses, a pre-chorus, a chorus,
/// a bridge, an ending, one line too long for the screen and a second language.
const SONG: [Part; 7] = [
    Part { kind: Kind::Verse, name: "Bait 1", slides: &[&["Terang pagi menyentuh lembah sunyi", "Langkah kecil kami berjalan pelan"], &["Di setiap tikungan Engkau menanti", "Tangan-Mu menuntun sepanjang jalan"]], english: &[&["Morning light upon the quiet valley", "Our small steps are walking slowly"], &["At every turn You wait for us", "Your hand leads all along the road"]] },
    Part { kind: Kind::Pre, name: "Pra-reff", slides: &[&["Dan saat malam datang mendekat", "Hati kami tetap tenang"]], english: &[&["And when the night comes drawing near", "Our hearts will stay at rest"]] },
    Part { kind: Kind::Chorus, name: "Reff", slides: &[&["Bersinarlah di jalan kami", "Bersinar sampai fajar tiba"], &["Engkau pelita di tengah gelap", "Kami bernyanyi tanpa takut"]], english: &[&["Shine upon the road we walk", "Shine until the dawn has come"], &["You are the lamp within the dark", "We sing and have no fear"]] },
    Part { kind: Kind::Verse, name: "Bait 2", slides: &[&["Angin dingin bertiup dari bukit", "Namun api kecil tetap menyala"], &["Kami saling menggandeng tangan", "Melangkah bersama dalam kasih"]], english: &[&["A cold wind blows down from the hill", "And still the little fire burns"], &["We take each other by the hand", "And walk together in love"]] },
    Part { kind: Kind::Bridge, name: "Jembatan", slides: &[&["Tak ada gunung terlalu tinggi", "Tak ada laut terlalu dalam"]], english: &[&["No mountain is too high", "No sea is ever too deep"]] },
    Part { kind: Kind::Verse, name: "Bait 3", slides: &[&["Kami akan terus berjalan bersama melewati setiap lembah yang paling gelap", "Sampai kami tiba di rumah-Mu"]], english: &[&["We will keep on walking through every darkest valley", "Until we reach Your house"]] },
    Part { kind: Kind::End, name: "Penutup", slides: &[&["Bersinar sampai fajar tiba"]], english: &[&["Shine until the dawn has come"]] },
];

/// The sung order, as places in `SONG`, with how many times in a row.
const ORDER: [(usize, u8); 9] = [(0, 1), (1, 1), (2, 1), (3, 1), (1, 1), (2, 1), (4, 1), (2, 2), (6, 1)];
/// Characters a line may hold on the projector at the church's size before it wraps.
const FIT: usize = 52;
const TABS: [&str; 7] = ["Rundown ibadah", "Pengumuman", "Kalender", "Jemaat", "Persembahan", "Templat", "Tema"];

fn t(words: impl Into<String>, style: TextStyle, ink: Color) -> El {
    text(words).font(style).color(ink)
}

fn hline() -> El {
    el().h(1.0).w_full().fill(theme::surf().edge).a11y_hidden()
}

fn dot(c: Color, d: f32) -> El {
    el().size(d, d).radius(d / 2.0).fill(c).a11y_hidden()
}

fn too_long(line: &str) -> bool {
    line.chars().count() > FIT
}

fn slide_count() -> usize {
    SONG.iter().map(|part| part.slides.len()).sum()
}

/// The window around a direction: the real title bar, the tabs on the rundown, the plan column
/// and Live, so the editor is judged in the space it really gets.
fn frame(s: &App, cx: &mut Cx<App>, k: &Kit, editor: El) -> El {
    let sf = theme::surf();
    let insets = cx.chrome_insets();
    let mut tabs = row().w_full().h(44.0).px(space::S4).gap(space::S1).center_y().fill(sf.side);
    for (i, name) in TABS.iter().enumerate() {
        let pill = row().h(30.0).px(space::S3).center_y().radius(15.0);
        tabs = tabs.child(if i == 0 { pill.fill(theme::accent().alpha(0.2)).child(t(*name, type_::BODY_STRONG, k.c.text_base)) } else { pill.child(t(*name, type_::BODY, k.c.text_muted)) });
    }
    let mut plan = col().w(256.0).h_full().px(space::S3).py(space::S3).gap(space::S2).fill(sf.side);
    for name in ["Pengumuman", "Terang di Lembah", "Bukit Bintang Pagi", "Kasih Setia-Mu", "Yohanes 3:16-17"] {
        let on = name == "Terang di Lembah";
        let item = row().h(34.0).px(space::S3).gap(space::S2).center_y().radius(k.radius.md);
        plan = plan.child(if on { item.fill(theme::accent().alpha(0.18)).child(t(name, type_::BODY_STRONG, k.c.text_base)) } else { item.child(t(name, type_::BODY, k.c.text_muted)) });
    }
    let live = col()
        .w(292.0)
        .h_full()
        .padding(space::S4)
        .gap(space::S3)
        .fill(sf.side)
        .child(t("Live", type_::BODY_STRONG, k.c.text_base))
        .child(el().w_full().h(146.0).radius(k.radius.md).fill(Color::BLACK).child(col().w_full().h_full().center().child(t("Belum ada yang tayang", type_::SMALL, k.c.text_faint))));
    col()
        .key("window")
        .w_full()
        .h_full()
        .fill(sf.chrome)
        .child(super::topbar(s, k, insets, false).fill(sf.chrome))
        .child(hline())
        .child(tabs)
        .child(hline())
        .child(row().w_full().grow(1.0).min_h(0.0).child(plan).child(el().w(1.0).h_full().fill(sf.edge)).child(editor.grow(1.0).min_w(0.0).h_full().fill(sf.work)).child(el().w(1.0).h_full().fill(sf.edge)).child(live))
}

/// The editor's head, the same in every direction: the title is the song's name in large type,
/// edited where it stands, and details sit behind one button.
fn head(k: &Kit, note: &str) -> El {
    row()
        .w_full()
        .px(space::S5)
        .pt(space::S4)
        .pb(space::S3)
        .gap(space::S2)
        .center_y()
        .child(col().gap(2.0).grow(1.0).min_w(0.0).child(t("Terang di Lembah", type_::HEADING, k.c.text_base)).child(t(note, type_::SMALL, k.c.text_muted)))
        .child(btn("Detail lagu").icon(&icons::SLIDERS).ghost().build(k, |_, _| {}))
        .child(btn("Batal").hint("Esc").build(k, |_, _| {}))
        .child(btn("Simpan").primary().hint("Ctrl S").build(k, |_, _| {}))
}

pub(super) fn page(s: &App, cx: &mut Cx<App>, k: &Kit, direction: char, page: u8, layer: &mut Vec<El>) -> El {
    let _ = layer;
    let editor = match direction {
        'x' => cards(k, page),
        'y' => reflow(k, page),
        _ => stage(k, page),
    };
    frame(s, cx, k, editor)
}

// ---- A. Part cards, after ElevenLabs Music ----

fn kind_chip(k: &Kit, part: &Part) -> El {
    row()
        .h(28.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(14.0)
        .fill(part.kind.colour().alpha(0.22))
        .child(dot(part.kind.colour(), 8.0))
        .child(t(part.name, type_::BODY_STRONG, k.c.text_base))
        .child(icon(k.glyph.chevron_down, 12.0).color(k.c.text_muted).a11y_hidden())
}

fn mini_slide(k: &Kit, lines: &[&str], c: Color) -> El {
    let mut face = col().w(112.0).h(63.0).radius(4.0).fill(Color::BLACK).border(1.0, c.alpha(0.7)).center().gap(1.0).px(4.0);
    for line in lines {
        face = face.child(t(*line, type_::TINY, Color::WHITE).align(Align::Center).truncate().w_full());
    }
    let _ = k;
    face
}

fn card(k: &Kit, part: &Part, english: bool, focused: bool) -> El {
    let c = part.kind.colour();
    let mut words = col().grow(1.0).min_w(0.0).gap(space::S1);
    for (at, slide) in part.slides.iter().enumerate() {
        if at > 0 {
            words = words.child(row().w_full().gap(space::S2).center_y().py(2.0).child(el().h(1.0).grow(1.0).border(1.0, c.alpha(0.5)).border_dash(4.0, 4.0)).child(t(format!("slide {}", at + 1), type_::TINY, k.c.text_faint)).child(el().h(1.0).grow(1.0).border(1.0, c.alpha(0.5)).border_dash(4.0, 4.0)));
        }
        for (n, line) in slide.iter().enumerate() {
            let long = too_long(line);
            let mut l = row().w_full().gap(space::S3).center_y().child(t(*line, type_::BODY, k.c.text_base).grow(1.0).min_w(0.0).wrap());
            if english {
                l = l.child(t(part.english[at][n], type_::BODY, k.c.text_muted).grow(1.0).min_w(0.0).wrap());
            }
            if long {
                l = l.child(row().h(22.0).px(space::S2).gap(4.0).center_y().radius(11.0).fill(theme::warn_ink().alpha(0.16)).child(t("Terlalu panjang", type_::TINY, theme::warn_ink())).child(t("Pecah", type_::TINY, k.c.text_base)));
            }
            words = words.child(l);
        }
    }
    let mut thumbs = col().gap(space::S1);
    if !english {
        for slide in part.slides {
            thumbs = thumbs.child(mini_slide(k, slide, c));
        }
    }
    let grip = col().w(14.0).gap(3.0).pt(6.0).children((0..3).map(|_| row().gap(3.0).child(dot(k.c.text_faint, 3.0)).child(dot(k.c.text_faint, 3.0))));
    let mut body = col()
        .w_full()
        .gap(space::S2)
        .padding(space::S3)
        .radius(10.0)
        .fill(c.alpha(0.08))
        .border(if focused { 2.0 } else { 1.0 }, if focused { c } else { c.alpha(0.35) })
        .child(row().w_full().gap(space::S2).center_y().child(grip).child(kind_chip(k, part)).child(spacer()).child(t(format!("{} slide", part.slides.len()), type_::SMALL, k.c.text_muted)).child(icon(&icons::DOTS, 16.0).color(k.c.text_muted)));
    if english {
        body = body.child(row().w_full().gap(space::S3).pl(22.0).child(t("Indonesia", type_::TINY, k.c.text_faint).grow(1.0)).child(t("English", type_::TINY, k.c.text_faint).grow(1.0)));
    }
    body.child(row().w_full().gap(space::S3).pl(22.0).child(words).child(thumbs))
}

fn order_strip(k: &Kit) -> El {
    let total = ORDER.iter().map(|(part, times)| SONG[*part].slides.len() * usize::from(*times)).sum::<usize>() as f32;
    let mut strip = row().w_full().h(40.0).gap(3.0);
    for (part, times) in ORDER {
        let p = &SONG[part];
        let share = (p.slides.len() * usize::from(times)) as f32 / total;
        let mut chip = row().h_full().grow(share * 100.0).min_w(if times > 1 { 96.0 } else { 74.0 }).px(space::S2).gap(4.0).center_y().radius(6.0).fill(p.kind.colour().alpha(0.32)).border(1.0, p.kind.colour().alpha(0.6)).child(t(p.name, type_::SMALL_STRONG, k.c.text_base).truncate().min_w(0.0));
        if times > 1 {
            chip = chip.child(t(format!("×{times}"), type_::SMALL_STRONG, k.c.text_base));
        }
        strip = strip.child(chip);
    }
    strip = strip.child(row().size(40.0, 40.0).center().radius(6.0).border(1.0, k.c.border_strong).child(icon(&icons::PLUS, 16.0).color(k.c.text_base)));
    col()
        .w_full()
        .px(space::S5)
        .py(space::S3)
        .gap(space::S2)
        .fill(theme::surf().side)
        .child(row().w_full().gap(space::S2).center_y().child(t("Urutan nyanyi", type_::SMALL_STRONG, k.c.text_muted)).child(t(format!("{} slide di layar", slide_count() + 4), type_::SMALL, k.c.text_faint)).child(spacer()).child(t("Klik bagian untuk menambah, seret untuk memindah", type_::SMALL, k.c.text_faint)))
        .child(strip)
}

fn add_tile(k: &Kit) -> El {
    let mut kinds = row().gap(space::S2);
    for (kind, name) in [(Kind::Verse, "Bait"), (Kind::Pre, "Pra-reff"), (Kind::Chorus, "Reff"), (Kind::Bridge, "Jembatan"), (Kind::End, "Penutup")] {
        kinds = kinds.child(row().h(30.0).px(space::S3).gap(space::S2).center_y().radius(15.0).border(1.0, kind.colour().alpha(0.6)).child(dot(kind.colour(), 8.0)).child(t(name, type_::SMALL_STRONG, k.c.text_base)));
    }
    row().w_full().padding(space::S3).gap(space::S3).center_y().radius(10.0).border(1.0, k.c.border_strong).border_dash(6.0, 4.0).child(icon(&icons::PLUS, 16.0).color(k.c.text_muted)).child(t("Bagian baru", type_::BODY_STRONG, k.c.text_muted)).child(spacer()).child(kinds)
}

fn paste_proposal(k: &Kit) -> El {
    let mut list = col().w_full().gap(space::S2);
    for (part, first) in [(0usize, "Terang pagi menyentuh lembah sunyi"), (2, "Bersinarlah di jalan kami"), (3, "Angin dingin bertiup dari bukit"), (2, "Bersinarlah di jalan kami"), (4, "Tak ada gunung terlalu tinggi")] {
        let p = &SONG[part];
        list = list.child(row().w_full().h(40.0).px(space::S3).gap(space::S3).center_y().radius(8.0).fill(p.kind.colour().alpha(0.1)).child(dot(p.kind.colour(), 10.0)).child(t(p.name, type_::BODY_STRONG, k.c.text_base).w(110.0)).child(t(first, type_::BODY, k.c.text_muted).truncate().grow(1.0).min_w(0.0)).child(if p.kind == Kind::Chorus { t("Diulang 2×", type_::SMALL, k.c.text_faint) } else { el() }));
    }
    col()
        .w_full()
        .padding(space::S4)
        .gap(space::S3)
        .radius(12.0)
        .fill(theme::surf().raised)
        .border(1.0, k.c.border_base)
        .child(row().w_full().gap(space::S2).center_y().child(icon(&icons::PASTE, 16.0).color(theme::accent())).child(t("Lirik yang ditempel dibagi jadi 5 bagian", type_::BODY_STRONG, k.c.text_base)).child(spacer()).child(btn("Atur sendiri").ghost().build(k, |_, _| {})).child(btn("Pakai semua").primary().build(k, |_, _| {})))
        .child(t("Bagian yang sama persis dikenali sebagai Reff. Nama tiap bagian bisa diganti sesudahnya.", type_::SMALL, k.c.text_muted))
        .child(list)
}

fn cards(k: &Kit, page: u8) -> El {
    let english = page == 3;
    let mut stack = col().w_full().px(space::S5).py(space::S3).gap(space::S3);
    if page == 2 {
        stack = stack.child(paste_proposal(k));
    } else {
        for (at, part) in SONG.iter().enumerate() {
            stack = stack.child(card(k, part, english, at == 2));
        }
        stack = stack.child(add_tile(k));
    }
    let note = if english { "Indonesia dan English, bagian demi bagian" } else { "7 bagian, 11 slide" };
    col().w_full().h_full().min_h(0.0).child(head(k, note)).child(col().w_full().grow(1.0).min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(stack)).child(order_strip(k))
}

// ---- B. Reflow, after ProPresenter's Reflow editor ----

fn order_line(k: &Kit) -> El {
    let mut line = row().gap(space::S1).center_y().flex_wrap();
    for (i, (part, times)) in ORDER.iter().enumerate() {
        let p = &SONG[*part];
        if i > 0 {
            line = line.child(icon(&icons::ARROW_RIGHT, 12.0).color(k.c.text_faint));
        }
        let mut chip = row().h(24.0).px(space::S2).gap(4.0).center_y().radius(4.0).fill(p.kind.colour().alpha(0.25)).child(t(p.name, type_::SMALL_STRONG, k.c.text_base));
        if *times > 1 {
            chip = chip.child(t(format!("×{times}"), type_::SMALL_STRONG, p.kind.colour()));
        }
        line = line.child(chip);
    }
    row().w_full().px(space::S5).py(space::S2).gap(space::S3).center_y().fill(theme::surf().side).child(t("Urutan", type_::SMALL_STRONG, k.c.text_muted)).child(line.grow(1.0).min_w(0.0)).child(btn("Ubah urutan").small().build(k, |_, _| {}))
}

fn reflow(k: &Kit, page: u8) -> El {
    let english = page == 3;
    let proposing = page == 2;
    let mut text_col = col().w_full().gap(0.0);
    let mut grid = col().gap(space::S2);
    let mut cells = row().gap(space::S2);
    let mut in_row = 0;
    let mut number = 0;
    for part in SONG.iter() {
        let c = part.kind.colour();
        let mut run = col().grow(1.0).min_w(0.0);
        for (at, slide) in part.slides.iter().enumerate() {
            number += 1;
            run = run.child(row().w_full().h(20.0).px(space::S2).center_y().fill(k.c.layer_03).child(t(format!("{number}"), type_::TINY, k.c.text_muted)).child(spacer()).child(icon(k.glyph.chevron_down, 10.0).color(k.c.text_faint)));
            let mut face = col().w_full().py(space::S2).gap(2.0).center();
            for (n, line) in slide.iter().enumerate() {
                let ink = if too_long(line) { theme::warn_ink() } else { k.c.text_base };
                face = face.child(t(*line, type_::BODY_STRONG, ink).align(Align::Center).w_full().wrap());
                if english {
                    face = face.child(t(part.english[at][n], type_::SMALL, k.c.text_muted).align(Align::Center).w_full().wrap());
                }
            }
            run = run.child(face.fill(k.c.layer_02));
            let mut thumb = col().w(118.0).child(col().w(118.0).h(66.0).fill(Color::BLACK).center().gap(1.0).px(4.0).children(slide.iter().map(|line| t(*line, type_::TINY, Color::WHITE).align(Align::Center).truncate().w_full())));
            thumb = thumb.child(row().w(118.0).h(18.0).px(4.0).gap(4.0).center_y().fill(c).child(t(format!("{number}"), type_::TINY, Color::WHITE)).child(if at == 0 { t(part.name, type_::TINY, Color::WHITE).truncate() } else { el() }));
            cells = cells.child(thumb);
            in_row += 1;
            if in_row == 3 {
                grid = grid.child(cells);
                cells = row().gap(space::S2);
                in_row = 0;
            }
        }
        let gutter_label = if proposing { format!("{}?", part.name) } else { part.name.to_string() };
        let gutter = col().w(86.0).pt(26.0).px(space::S2).gap(4.0).child(row().gap(4.0).center_y().child(dot(c, 8.0)).child(t(gutter_label, type_::SMALL_STRONG, c)));
        text_col = text_col.child(row().w_full().child(el().w(4.0).fill(c)).child(gutter).child(run).pb(space::S1));
    }
    if in_row > 0 {
        grid = grid.child(cells);
    }
    let mut left = col().w_full().px(space::S4).py(space::S3).gap(space::S2);
    if proposing {
        left = left.child(row().w_full().padding(space::S3).gap(space::S2).center_y().radius(8.0).fill(theme::accent().alpha(0.12)).child(icon(&icons::PASTE, 16.0).color(theme::accent())).child(t("Nama bagian diusulkan dari lirik yang ditempel. Klik nama di pinggir untuk mengganti.", type_::SMALL, k.c.text_base).grow(1.0).wrap()).child(btn("Pakai").primary().small().build(k, |_, _| {})));
    }
    left = left.child(text_col);
    let foot = row().w_full().px(space::S4).py(space::S2).gap(space::S2).center_y().child(btn("Sisipkan pemisah slide").small().hint("Ctrl Enter").build(k, |_, _| {})).child(spacer()).child(t(if english { "Baris kedua: English" } else { "11 slide" }, type_::SMALL, k.c.text_faint));
    let panes = row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(col().grow(1.0).min_w(0.0).h_full().child(col().w_full().grow(1.0).min_h(0.0).scroll(rune::ScrollSpec { stick_bottom: false }).child(left)).child(foot))
        .child(el().w(1.0).h_full().fill(theme::surf().edge))
        .child(col().w(394.0).h_full().padding(space::S3).scroll(rune::ScrollSpec { stick_bottom: false }).child(grid));
    col().w_full().h_full().min_h(0.0).child(head(k, "Teks di kiri, slide di kanan")).child(order_line(k)).child(panes)
}

// ---- C. Stage and lists, after WorshipTools Presenter ----

fn stage(k: &Kit, page: u8) -> El {
    let english = page == 3;
    let proposing = page == 2;
    let current = &SONG[2];
    let c = current.kind.colour();
    let mut face = col().w_full().h_full().center().gap(space::S2).px(space::S5);
    for (n, line) in current.slides[0].iter().enumerate() {
        let words = if english { current.english[0][n] } else { *line };
        face = face.child(t(words, TextStyle::new(26.0, 600.0).lh(34.0), Color::WHITE).align(Align::Center).w_full().wrap());
    }
    let screen = el()
        .w_full()
        .h(318.0)
        .radius(8.0)
        .fill(Color::BLACK)
        .padding(space::S3)
        .child(el().w_full().h_full().border(1.0, Color::WHITE.alpha(0.25)).border_dash(6.0, 5.0).radius(4.0).child(face));
    let over = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(row().h(30.0).px(space::S3).gap(space::S2).center_y().radius(6.0).fill(c.alpha(0.25)).child(dot(c, 8.0)).child(t("Reff a", type_::BODY_STRONG, k.c.text_base)).child(icon(k.glyph.chevron_down, 12.0).color(k.c.text_muted)))
        .child(spacer())
        .child(SegmentedControl::new(&["Indonesia", "English"], Some(usize::from(english))).fit().build(k))
        .child(t("6 dari 11", type_::SMALL, k.c.text_muted))
        .child(btn("").icon(&icons::ARROW_LEFT).icon_only().small().build(k, |_, _| {}))
        .child(btn("").icon(&icons::ARROW_RIGHT).icon_only().small().build(k, |_, _| {}));
    let under = row().w_full().gap(space::S3).center_y().child(t("2 baris, paling panjang 26 huruf", type_::SMALL, k.c.text_muted)).child(spacer()).child(btn("Pecah slide di kursor").small().hint("Ctrl Enter").build(k, |_, _| {}));
    let mut lines = col().w_full().gap(space::S1).child(t("Baris di slide ini", type_::SMALL_STRONG, k.c.text_muted));
    for (n, line) in current.slides[0].iter().enumerate() {
        let words = if english { current.english[0][n] } else { *line };
        lines = lines.child(row().w_full().h(36.0).px(space::S3).gap(space::S3).center_y().radius(6.0).fill(k.c.layer_02).border(1.0, if n == 0 { theme::accent() } else { k.c.border_base }).child(t(format!("{}", n + 1), type_::SMALL_STRONG, k.c.text_faint)).child(t(words, type_::BODY, k.c.text_base).grow(1.0).min_w(0.0).truncate()).child(t(format!("{} huruf", words.chars().count()), type_::SMALL, k.c.text_faint)));
    }
    let mut centre = col().grow(1.0).min_w(0.0).h_full().padding(space::S4).gap(space::S3).child(over).child(screen).child(under).child(lines);
    if proposing {
        centre = centre.child(col().w_full().padding(space::S3).gap(space::S2).radius(8.0).fill(theme::accent().alpha(0.12)).child(row().w_full().gap(space::S2).center_y().child(icon(&icons::PASTE, 16.0).color(theme::accent())).child(t("11 slide dibuat dari lirik yang ditempel, namanya diusulkan", type_::BODY_STRONG, k.c.text_base)).child(spacer()).child(btn("Pakai semua").primary().small().build(k, |_, _| {}))).child(t("Cek nama di daftar Slide. Bagian yang sama persis jadi Reff.", type_::SMALL, k.c.text_muted)));
    }
    let row_of = |k: &Kit, colour: Color, name: String, on: bool, plus: bool| {
        row()
            .w_full()
            .h(36.0)
            .px(space::S3)
            .gap(space::S2)
            .center_y()
            .radius(6.0)
            .fill(if on { colour.alpha(0.2) } else { Color::TRANSPARENT })
            .child(el().w(4.0).h(20.0).radius(2.0).fill(colour))
            .child(t(name, if on { type_::BODY_STRONG } else { type_::BODY }, k.c.text_base).grow(1.0).min_w(0.0).truncate())
            .child(icon(if plus { &icons::PLUS } else { &icons::CLOSE }, 14.0).color(k.c.text_muted))
    };
    let mut slides = col().w(178.0).h_full().py(space::S3).px(space::S2).gap(2.0).child(t("Slide", type_::SMALL_STRONG, k.c.text_muted).px(space::S2).pb(space::S1));
    for part in SONG.iter() {
        for at in 0..part.slides.len() {
            let name = if part.slides.len() > 1 { format!("{} {}", part.name, ['a', 'b', 'c'][at]) } else { part.name.to_string() };
            let shown = if proposing { format!("{name}?") } else { name.clone() };
            slides = slides.child(row_of(k, part.kind.colour(), shown, part.name == "Reff" && at == 0, true));
        }
    }
    slides = slides.child(row().w_full().h(36.0).px(space::S3).gap(space::S2).center_y().radius(6.0).border(1.0, k.c.border_base).border_dash(5.0, 4.0).child(icon(&icons::PLUS, 14.0).color(k.c.text_muted)).child(t("Slide baru", type_::BODY, k.c.text_muted)));
    let mut order = col().w(178.0).h_full().py(space::S3).px(space::S2).gap(2.0).fill(theme::surf().side).child(t("Urutan nyanyi", type_::SMALL_STRONG, k.c.text_muted).px(space::S2).pb(space::S1));
    for (part, times) in ORDER {
        let p = &SONG[part];
        for _ in 0..times {
            order = order.child(row_of(k, p.kind.colour(), p.name.to_string(), false, false));
        }
    }
    order = order.child(row().w_full().h(36.0).px(space::S3).center().radius(6.0).fill(k.c.layer_03).child(t("Tambah jeda", type_::BODY, k.c.text_base)));
    let body = row().w_full().grow(1.0).min_h(0.0).child(centre).child(el().w(1.0).h_full().fill(theme::surf().edge)).child(slides.scroll(rune::ScrollSpec { stick_bottom: false })).child(order.scroll(rune::ScrollSpec { stick_bottom: false }));
    col().w_full().h_full().min_h(0.0).child(head(k, "Satu slide di layar, daftar di kanan")).child(body)
}
