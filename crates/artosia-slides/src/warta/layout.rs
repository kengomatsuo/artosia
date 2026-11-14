//! A slide as boxes of words: which part of a card each text is, where it sits in the 1920x1080 space,
//! and how the size search came out. The editor draws its selection and its caret from this, and the
//! second slide is cut by it, so nothing is measured twice.

use artosia_core::deck::{NoticeKind, NoticeStyle, Slide};

use super::{At, BODY_FLOOR, Item, Pics, Sheet, build, data, forced, metrics, number, plain_body_range, sheet};

/// The part of a card a box shows. The first two are typed on the slide; the others come from a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Title,
    Body,
    /// A schedule's rows, from the Calendar.
    Rows,
    /// A month's or week's birthdays, from Members.
    Names,
    /// The funds of the offerings, from Offerings.
    Funds,
    /// The church's account, from Offerings.
    Account,
    /// A passage's verses, from the Bible.
    Verse,
}

impl Field {
    /// Whether words are typed into it on the slide.
    pub fn typed(self) -> bool {
        matches!(self, Field::Title | Field::Body)
    }
}

/// The texts of a slide that are not the card's own words and not a window's rows.
const DRESSING: [&str; 11] = ["kicker", "kicker_y", "subtitle", "sub", "sub_s", "ribbon", "sticker", "badge", "pill_p", "caption", "atas"];

/// Which part of a card a text of type role `role` shows; None for what a style adds around the words.
fn field_of(role: &str, kind: NoticeKind) -> Option<Field> {
    match role {
        "title" | "title_w" | "n_title" => Some(Field::Title),
        "n_body" | "n_body_b" => Some(Field::Body),
        _ if DRESSING.contains(&role) => None,
        _ => match kind {
            NoticeKind::Schedule => Some(Field::Rows),
            NoticeKind::Birthdays => Some(Field::Names),
            NoticeKind::Money => Some(Field::Funds),
            NoticeKind::Account => Some(Field::Account),
            NoticeKind::Verse => Some(Field::Verse),
            NoticeKind::Plain => None,
        },
    }
}

/// The three bytes of a `#RRGGBB` colour.
fn bytes(code: &str) -> [u8; 3] {
    let byte = |at: usize| u8::from_str_radix(code.get(at..at + 2).unwrap_or("00"), 16).unwrap_or(0);
    [byte(1), byte(3), byte(5)]
}

/// One box of words in the 1920x1080 space, with what an editor needs to set the same words over it.
#[derive(Clone, Debug)]
pub struct TextBox {
    pub field: Field,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// The largest size of its words, and how many lines they take.
    pub size: f32,
    pub lines: usize,
    /// What the words stand on and how they are written.
    pub ground: Option<[u8; 3]>,
    pub ink: [u8; 3],
    pub family: rune::Family,
    pub weight: f32,
    pub italic: bool,
    pub tracking: f32,
    /// Where the lines sit across the box as the slide draws them, which the caret's words keep.
    pub align: artosia_core::deck::Align,
    /// How much room the box keeps round its words, which the caret's words keep too.
    pub pad: f32,
}

/// How a size search came out for one typed box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fit {
    pub size: f32,
    /// The size asked for: the card's own, else the style's.
    pub ceiling: f32,
    pub floor: f32,
    /// The least the style asks for, and the most it sets.
    pub max: f32,
    pub lines: usize,
    pub fits: bool,
}

impl Fit {
    pub fn shrunk(&self) -> bool {
        self.size < self.ceiling
    }

    pub fn at_floor(&self) -> bool {
        self.size <= self.floor
    }
}

/// One line of words as set: its place and size, and the words.
#[derive(Clone, Debug)]
pub struct Line {
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
    pub size: f32,
    pub text: String,
    pub family: rune::Family,
    pub weight: f32,
    pub italic: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// The boxes of the slide as the room sees it: for a body that does not hold, the part that does.
    pub boxes: Vec<TextBox>,
    pub title: Option<Fit>,
    pub body: Option<Fit>,
    /// The lines of a body that do not hold, where they would stand under the box, and the height
    /// at which the next slide starts.
    pub ghost: Vec<Line>,
    pub split_y: Option<f32>,
}

impl Layout {
    pub fn of(&self, field: Field) -> Option<&TextBox> {
        self.boxes.iter().find(|found| found.field == field)
    }
}

/// How much room a box keeps around its words, which is where its outline is drawn.
fn pad_of(field: Field) -> f32 {
    match field {
        Field::Title => 8.0,
        Field::Body => 14.0,
        _ => 12.0,
    }
}

impl Sheet {
    fn text_boxes(&self, kind: NoticeKind) -> (Vec<TextBox>, Vec<Line>) {
        let mut boxes: Vec<TextBox> = Vec::new();
        let mut rows: Vec<Line> = Vec::new();
        for (at, item) in self.items.iter().enumerate() {
            let Item::Text(t) = item else { continue };
            let Some(field) = field_of(&t.role, kind).filter(|_| t.rot == 0.0) else { continue };
            let m = metrics(&t.file);
            let (top, bottom) = (t.y - m.asc * t.size, t.y + m.desc * t.size);
            if field == Field::Body {
                rows.push(Line { x: t.x, top, bottom, size: t.size, text: t.s.clone(), family: rune::Family::Named(t.family), weight: t.weight, italic: t.italic });
            }
            if let Some(found) = boxes.iter_mut().find(|found| found.field == field) {
                let (x1, y1) = ((found.x + found.w).max(t.x + t.w), (found.y + found.h).max(bottom));
                (found.x, found.y) = (found.x.min(t.x), found.y.min(top));
                (found.w, found.h) = (x1 - found.x, y1 - found.y);
                found.size = found.size.max(t.size);
                found.lines += 1;
                continue;
            }
            // The last flat colour drawn before the words that holds their middle is what they stand on.
            let (cx, cy) = (t.x + t.w / 2.0, t.y);
            let ground = self.items[..at].iter().rev().find_map(|earlier| match earlier {
                Item::Rect(r) if r.fill.as_deref().is_some_and(|fill| fill.starts_with('#')) && r.op >= 0.9 && cx >= r.x && cx <= r.x + r.w && cy >= r.y && cy <= r.y + r.h => r.fill.as_deref().map(bytes),
                _ => None,
            });
            boxes.push(TextBox { field, x: t.x, y: top, w: t.w, h: bottom - top, size: t.size, lines: 1, ground, ink: bytes(&t.fill), family: rune::Family::Named(t.family), weight: t.weight, italic: t.italic, tracking: t.ls, align: match t.at { At::Left => artosia_core::deck::Align::Start, At::Centre => artosia_core::deck::Align::Centre, At::Right => artosia_core::deck::Align::End }, pad: 0.0 });
        }
        for found in &mut boxes {
            let pad = pad_of(found.field);
            (found.x, found.y, found.w, found.h) = (found.x - pad, found.y - pad, found.w + 2.0 * pad, found.h + 2.0 * pad);
            found.pad = pad;
        }
        (boxes, rows)
    }
}

fn fit_of(field: Field, ceiling: f32, max: f32, floor: f32, fits: bool, boxes: &[TextBox]) -> Option<Fit> {
    let found = boxes.iter().find(|found| found.field == field)?;
    Some(Fit { size: found.size, ceiling: ceiling.max(found.size), floor, max, lines: found.lines, fits })
}

pub(super) fn have_of(slide: &Slide, pics: &Pics) -> data::Have {
    data::Have { photo: pics.photo.is_some() || !slide.image.is_empty(), photo2: pics.photo2.is_some() || !slide.image2.is_empty(), qris: pics.qris.is_some() || !slide.qris.is_empty() }
}

/// The boxes of a slide in a poster style and, for a plain card, how its title and body fit. The
/// last answer is kept, since an editor asks for the same one every frame until a key is pressed.
pub fn layout(slide: &Slide, style: NoticeStyle, pics: &Pics) -> Layout {
    layout_with(slide, style, pics, false)
}

/// [`layout`] for a key being typed: a body that does not hold is shown whole, with no cut and no lines
/// that go on, so a key costs a few milliseconds and no more; the cut is worked out when the caret leaves.
pub fn layout_typing(slide: &Slide, style: NoticeStyle, pics: &Pics) -> Layout {
    layout_with(slide, style, pics, true)
}

fn layout_with(slide: &Slide, style: NoticeStyle, pics: &Pics, typing: bool) -> Layout {
    thread_local! {
        static LAST: std::cell::RefCell<Option<(u64, Layout)>> = const { std::cell::RefCell::new(None) };
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (format!("{slide:?}"), typing, style, have_of(slide, pics).photo, have_of(slide, pics).photo2, have_of(slide, pics).qris).hash(&mut hasher);
    let key = hasher.finish();
    if let Some(found) = LAST.with(|last| last.borrow().as_ref().filter(|(kept, _)| *kept == key).map(|(_, found)| found.clone())) {
        return found;
    }
    let found = worked_out(slide, style, pics, typing);
    LAST.with(|last| *last.borrow_mut() = Some((key, found.clone())));
    found
}

fn worked_out(slide: &Slide, style: NoticeStyle, pics: &Pics, typing: bool) -> Layout {
    let have = have_of(slide, pics);
    let held = sheet(slide, style, have);
    let ok = held.is_ok();
    let Ok(whole) = held.or_else(|_| forced(slide, style, have)) else { return Layout::default() };
    if slide.notice != NoticeKind::Plain {
        return Layout { boxes: whole.text_boxes(slide.notice).0, ..Layout::default() };
    }
    let spec = |key: &str| number(&whole.d["s06"][key]);
    let title_ceiling = if slide.title_face.size > 0 { f32::from(slide.title_face.size).min(spec("title_max")) } else { spec("title_max") };
    let body_max = number(&whole.d["type"]["n_body"]["size"]);
    let body_ceiling = if slide.body_face.size > 0 { f32::from(slide.body_face.size) } else { body_max };
    // Without its body the title alone either holds or it is the title that is too long.
    let title_holds = ok || title_alone_holds(slide, style, have);
    // A body that does not hold is shown by the part that does, with the rest as ghost lines under it.
    let cutting = if title_holds && !ok && !typing { cut_at(slide, style, have) } else { None };
    let part = cutting.as_ref().and_then(|(first, _)| sheet(first, style, have).ok());
    let (boxes, rows) = part.as_ref().unwrap_or(&whole).text_boxes(slide.notice);
    // What goes on is set at the size of what stays, so the lines under the box are the ones the next slide starts with.
    let size = boxes.iter().find(|found| found.field == Field::Body).map_or(BODY_FLOOR, |found| found.size);
    let rest_rows = cutting.as_ref().and_then(|(_, rest)| build(rest, style, have, Some(size), true).ok()).map(|rest| rest.text_boxes(slide.notice).1).unwrap_or_default();
    let (ghost, split_y) = ghost_of(&rows, &rest_rows);
    let title = fit_of(Field::Title, title_ceiling, spec("title_max"), spec("title_floor"), title_holds, &boxes);
    let body = fit_of(Field::Body, body_ceiling, body_max, BODY_FLOOR.min(body_ceiling), ok, &boxes);
    Layout { boxes, title, body, ghost, split_y }
}

/// The lines of the body that goes on, set under the last of those that hold at their pitch.
fn ghost_of(held: &[Line], rest: &[Line]) -> (Vec<Line>, Option<f32>) {
    let (Some(last), false) = (held.last(), rest.is_empty()) else { return (Vec::new(), None) };
    let pitch = if held.len() > 1 { last.top - held[held.len() - 2].top } else { last.bottom - last.top };
    let ghost: Vec<Line> = rest.iter().enumerate().map(|(at, line)| Line { top: last.top + pitch * (at + 1) as f32, bottom: last.bottom + pitch * (at + 1) as f32, x: last.x.min(line.x), ..line.clone() }).collect();
    let split = (last.bottom + ghost[0].top) / 2.0;
    (ghost, Some(split))
}

fn title_alone_holds(slide: &Slide, style: NoticeStyle, have: data::Have) -> bool {
    sheet(&Slide { text: String::new(), ..slide.clone() }, style, have).is_ok()
}

/// A body's words with a mark where its lines end, so a cut keeps the lines a person typed.
fn tokens(text: &str) -> Vec<(&str, bool)> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .flat_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let last = words.len().saturating_sub(1);
            words.into_iter().enumerate().map(move |(at, word)| (word, at == last))
        })
        .collect()
}

fn joined(words: &[(&str, bool)]) -> String {
    let mut out = String::new();
    for (at, (word, ends)) in words.iter().enumerate() {
        out.push_str(word);
        if at + 1 < words.len() {
            out.push(if *ends { '\n' } else { ' ' });
        }
    }
    out
}

/// A plain card's body cut after the most words that still hold at the floor size: the part that
/// stays and the part that goes on the next slide. None when it holds, has one word, or the title
/// alone does not hold, which no cut can mend.
pub fn cut(slide: &Slide, style: NoticeStyle) -> Option<(Slide, Slide)> {
    if slide.notice != NoticeKind::Plain || !style.is_poster() {
        return None;
    }
    let have = have_of(slide, &Pics::default());
    if sheet(slide, style, have).is_ok() || !title_alone_holds(slide, style, have) {
        return None;
    }
    cut_at(slide, style, have)
}

/// [`cut`] for a slide already known not to hold whose title alone does.
fn cut_at(slide: &Slide, style: NoticeStyle, have: data::Have) -> Option<(Slide, Slide)> {
    let words = tokens(slide.text.as_str());
    if words.len() < 2 {
        return None;
    }
    let floor = plain_body_range(slide, style).1;
    let with = |count: usize| Slide { text: joined(&words[..count]), ..slide.clone() };
    // Fewer words always hold more easily, so the most that hold is found by halving.
    let (mut holds, mut fails) = (0, words.len());
    while fails - holds > 1 {
        let mid = (holds + fails) / 2;
        if build(&with(mid), style, have, Some(floor), false).is_ok() {
            holds = mid;
        } else {
            fails = mid;
        }
    }
    let holds = holds.max(1);
    Some((with(holds), Slide { text: joined(&words[holds..]), ..slide.clone() }))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::warta::holds;
    use artosia_core::deck::{Align, Face};

    const WORDS: &str = "Pendaftaran di meja informasi sampai 6 Desember Biaya Rp 350.000 per orang anak di bawah lima tahun gratis Bawa perlengkapan tidur alat mandi dan Alkitab sendiri Bus berangkat Sabtu pukul 06.00 dari halaman gereja dan kembali Minggu sore";
    const POSTERS: [NoticeStyle; 4] = [NoticeStyle::Greige, NoticeStyle::Pill, NoticeStyle::Neon, NoticeStyle::Split];

    fn card(title: &str, body: &str) -> Slide {
        Slide { notice: NoticeKind::Plain, heading: title.into(), text: body.into(), ..Slide::default() }
    }

    #[test]
    fn a_plain_card_shows_a_title_box_over_a_body_box_in_every_poster_style() {
        for style in POSTERS {
            let found = layout(&card("Retret keluarga jemaat", "Sabtu 12 Desember\nWisma Bukit Doa"), style, &Pics::default());
            let (title, body) = (found.of(Field::Title).expect("title box"), found.of(Field::Body).expect("body box"));
            assert!(title.y < body.y, "{style:?}: the title is above the body");
            for each in [title, body] {
                assert!(each.w > 100.0 && each.h > 30.0 && each.x >= 0.0 && each.x + each.w <= 1920.0 && each.y >= 0.0 && each.y + each.h <= 1080.0, "{style:?} {each:?}");
            }
            assert_eq!(body.lines, 2, "{style:?}: one line a typed line");
        }
    }

    #[test]
    fn a_long_title_shrinks_by_whole_sizes_between_the_ceiling_and_the_floor() {
        let short = layout(&card("Retret", "x"), NoticeStyle::Greige, &Pics::default()).title.expect("fit");
        let long = layout(&card("Retret keluarga jemaat bersama pemuda anak", "x"), NoticeStyle::Greige, &Pics::default()).title.expect("fit");
        assert!(short.size == short.ceiling && !short.shrunk(), "{short:?}");
        assert!(long.shrunk() && long.size >= long.floor && long.size.fract() == 0.0, "{long:?}");
        assert!(long.size < short.size && long.fits);
    }

    #[test]
    fn a_title_of_three_lines_at_the_floor_does_not_fit_and_a_second_slide_cannot_mend_it() {
        let slide = card("Retret keluarga jemaat bersama seluruh pemuda dan anak anak dari semua cabang pelayanan di kota ini dan sekitarnya", "Pendaftaran di meja informasi");
        let found = layout(&slide, NoticeStyle::Greige, &Pics::default());
        assert!(found.title.is_some_and(|fit| !fit.fits), "{:?}", found.title);
        assert!(cut(&slide, NoticeStyle::Greige).is_none());
    }

    #[test]
    fn a_body_past_the_floor_is_cut_into_a_part_that_holds_and_the_rest() {
        let body = std::iter::repeat_n(WORDS, 3).collect::<Vec<_>>().join(" ");
        for style in POSTERS {
            let slide = card("Retret keluarga jemaat", &body);
            assert!(!holds(&slide, style), "{style:?}: three times the words cannot fit");
            let (first, rest) = cut(&slide, style).unwrap_or_else(|| panic!("{style:?} has a cut"));
            assert!(holds(&first, style), "{style:?}: the first part holds");
            assert!(!rest.text.is_empty() && first.heading == rest.heading);
            assert_eq!(format!("{} {}", first.text, rest.text).split_whitespace().count(), body.split_whitespace().count(), "{style:?}: no word is lost");
            let found = layout(&slide, style, &Pics::default());
            assert!(found.body.is_some_and(|fit| !fit.fits && fit.at_floor()), "{style:?}: {:?}", found.body);
            assert!(!found.ghost.is_empty() && found.split_y.is_some_and(|y| y > found.of(Field::Body).map_or(0.0, |each| each.y)), "{style:?}: the lines that go on show under the box");
            assert!(found.of(Field::Body).is_some_and(|each| each.y + each.h < found.ghost[0].bottom + 1.0), "{style:?}");
        }
    }

    #[test]
    fn bold_and_italic_take_the_heavier_and_slanted_cut_of_the_same_family() {
        // The first line of a body is already heavy; the second is the plain cut.
        let plain = card("Retret", "Sabtu 12 Desember
Pendaftaran di meja informasi sampai 6 Desember");
        let width = |slide: &Slide| layout(slide, NoticeStyle::Greige, &Pics::default()).of(Field::Body).map_or(0.0, |each| each.w);
        let bold = Slide { body_face: Face { bold: true, ..Face::default() }, ..plain.clone() };
        let italic = Slide { body_face: Face { italic: true, ..Face::default() }, ..plain.clone() };
        assert!(width(&bold) > width(&plain) + 5.0, "{} then {}", width(&plain), width(&bold));
        let slanted = |slide: &Slide| layout(slide, NoticeStyle::Greige, &Pics::default()).of(Field::Body).is_some_and(|each| each.italic);
        assert!(slanted(&italic) && !slanted(&plain));
    }

    #[test]
    fn the_layout_a_key_asks_for_is_worked_out_in_a_frame() {
        let body = std::iter::repeat_n(WORDS, 2).collect::<Vec<_>>().join(" ");
        let slide = card("Retret keluarga jemaat", &body);
        // A key press is a miss: the words changed. Every frame after it is a hit.
        // Fonts are read the first time any text is measured, as the window does when it draws its first slide.
        for style in POSTERS {
            let _ = layout(&card("Warm", "up"), style, &Pics::default());
        }
        // Twenty keys typed into the title of an overfull card, in each style: the slowest one counts.
        let keys = |style: NoticeStyle| {
            (0..20)
                .map(|n| {
                    let started = std::time::Instant::now();
                    // What a key costs: the layout, and the drawing of the slide as it is.
                    let typed = Slide { heading: format!("Retret keluarga {}", "x".repeat(n)), ..slide.clone() };
                    let _ = layout_typing(&typed, style, &Pics::default());
                    let _ = crate::warta::while_typing(true, || crate::warta::draw(&typed, style, 780.0, 439.0, &Pics::default()));
                    started.elapsed().as_secs_f64() * 1000.0
                })
                .fold(0.0, f64::max)
        };
        let worst = POSTERS.iter().map(|style| keys(*style)).fold(0.0, f64::max);
        let started = std::time::Instant::now();
        for _ in 0..100 {
            let _ = layout(&slide, NoticeStyle::Split, &Pics::default());
        }
        let _ = layout(&slide, NoticeStyle::Split, &Pics::default());
        let hit = started.elapsed().as_secs_f64() * 10.0;
        // The usual key: a card that fits, one letter longer each time.
        let typing = card("Retret keluarga jemaat", "Sabtu 12 Desember
Pendaftaran di meja informasi sampai 6 Desember");
        let started = std::time::Instant::now();
        for n in 0..20 {
            let _ = layout(&Slide { heading: format!("Retret keluarga jemaat {}", "x".repeat(n)), ..typing.clone() }, NoticeStyle::Greige, &Pics::default());
        }
        let usual = started.elapsed().as_secs_f64() * 50.0;
        let leaving = (0..10)
            .map(|n| {
                let started = std::time::Instant::now();
                let _ = layout(&Slide { heading: format!("Retret keluarga {}", "y".repeat(n)), ..slide.clone() }, NoticeStyle::Pill, &Pics::default());
                started.elapsed().as_secs_f64() * 1000.0
            })
            .fold(0.0, f64::max);
        println!("overfull plain card: {worst:.2} ms a key typed, {leaving:.2} ms the frame the caret leaves, a card that fits {usual:.2} ms, {hit:.3} ms a frame after");
        assert!(worst < 300.0 && usual < 100.0 && hit < 1.0, "{worst} ms, {usual} ms, {hit} ms");
        // A release build keeps a key, layout and drawing together, inside a 60 Hz frame.
        if !cfg!(debug_assertions) {
            assert!(worst + usual < 16.7 && leaving < 16.7, "{worst} ms a key, {leaving} ms leaving");
        }
    }

    #[test]
    fn a_body_that_holds_has_no_cut() {
        assert!(cut(&card("Retret", "Sabtu 12 Desember"), NoticeStyle::Greige).is_none());
    }

    #[test]
    fn lines_a_person_typed_stay_lines_when_a_body_is_cut() {
        assert_eq!(joined(&tokens("a b\nc d e")), "a b\nc d e");
        assert_eq!(joined(&tokens("a b\n\nc d e")[..3]), "a b\nc");
    }

    #[test]
    fn a_schedule_gives_a_title_and_a_rows_box_and_the_window_is_never_typed() {
        let slide = Slide { notice: NoticeKind::Schedule, heading: "Jadwal".into(), text: "Minggu\t07.00\tIbadah Raya I\n\t09.30\tIbadah Raya II".into(), ..Slide::default() };
        let found = layout(&slide, NoticeStyle::Greige, &Pics::default());
        assert!(found.of(Field::Title).is_some_and(|each| each.field.typed()));
        assert!(found.of(Field::Rows).is_some_and(|each| !each.field.typed()));
        assert!(found.title.is_none() && found.body.is_none(), "only a plain card is fitted by the editor");
    }

    #[test]
    fn a_ceiling_of_the_card_lowers_the_title_and_alignment_moves_its_lines() {
        let plain = card("Retret", "x");
        let big = layout(&plain, NoticeStyle::Greige, &Pics::default());
        let small = layout(&Slide { title_face: Face { size: 90, ..Face::default() }, ..plain.clone() }, NoticeStyle::Greige, &Pics::default());
        assert!(small.title.is_some_and(|fit| fit.size == 90.0) && big.title.is_some_and(|fit| fit.size > 90.0));
        let left = big.of(Field::Title).map(|each| each.x).unwrap_or_default();
        let end = layout(&Slide { title_face: Face { align: Align::End, ..Face::default() }, ..plain }, NoticeStyle::Greige, &Pics::default()).of(Field::Title).map(|each| each.x).unwrap_or_default();
        assert!(end > left + 100.0, "{end} against {left}");
    }
}
