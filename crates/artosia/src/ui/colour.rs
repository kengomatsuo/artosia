//! One colour picker for every colour the program keeps (OVCOLOR1, `mock_b.rs` page 2). A row
//! offers the usual colours as squares and a More button; the picker opens under the row, never
//! over the slide, so the slide shows each colour as it is tried and the dropper can take a colour
//! from it. Its parts follow the references in `~/Shared/inspo/artosia-ui-overhaul/directions.md`
//! (P2): the colours already in the design first, as PowerPoint's theme colours and Canva's
//! document colours; a grid of samples; a saturation square with a hue strip, as Adobe Express and
//! FreeShow's `MaterialColorInput`; the colours used last; the colour code only behind its own
//! disclosure, so nothing is typed first.

use artosia_slides::look::{Background, Look, hex_of, parse_hex, rgb};
use rune::{Color, Cursor, Drag, El, Key, Phase, TextEdit, Tx, col, el, icon, row, text};
use rune_kit::{Kit, TextInput};

use super::sheets::setting;
use super::widgets::btn;
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

/// What a colour picker is choosing for. Every colour of the program is one of these, so one
/// picker serves all of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColourFor {
    /// The words on the projector.
    Ink,
    /// The line round the words.
    Outline,
}

impl ColourFor {
    fn key(self) -> &'static str {
        match self {
            ColourFor::Ink => "ink",
            ColourFor::Outline => "outline",
        }
    }
}

/// The open picker, if any, and what it holds between frames.
#[derive(Default)]
pub struct ColourPick {
    pub open: Option<ColourFor>,
    /// The colour when the picker opened, put back by Cancel.
    pub before: [u8; 3],
    /// Hue in degrees, saturation and value from 0 to 1, kept so a grey keeps its hue.
    pub hsv: [f32; 3],
    pub code: TextEdit,
    /// The colour code's disclosure is open.
    pub more: bool,
    /// The next press on the slide takes its colour.
    pub dropper: bool,
    /// The background picture decoded small for the dropper: path, width, height, RGB bytes.
    pub sample: Option<(String, u32, u32, Vec<u8>)>,
}

/// The most colours kept as used last.
const RECENT: usize = 8;

/// The sample grid: four lightnesses by ten hues of one strength, in OKLCH so each row reads as
/// one lightness.
const LIGHTNESS: [f32; 4] = [0.88, 0.76, 0.62, 0.46];
const HUES: [f32; 10] = [25.0, 55.0, 90.0, 130.0, 150.0, 190.0, 230.0, 262.0, 300.0, 340.0];

pub fn current(s: &App, target: ColourFor) -> [u8; 3] {
    let look = s.look();
    match target {
        ColourFor::Ink => look.ink,
        ColourFor::Outline => look.outline_ink,
    }
}

fn apply(s: &mut App, target: ColourFor, colour: [u8; 3]) {
    let mut look = s.look();
    match target {
        ColourFor::Ink => look.ink = colour,
        ColourFor::Outline => look.outline_ink = colour,
    }
    s.set_look(look);
}

impl App {
    /// Opens the picker for `target`, or closes it, keeping the colour, when it is already open.
    pub fn toggle_colour(&mut self, target: ColourFor) {
        if self.colour.open == Some(target) {
            self.finish_colour(true);
            return;
        }
        if self.colour.open.is_some() {
            self.finish_colour(true);
        }
        let now = current(self, target);
        self.colour.open = Some(target);
        self.colour.before = now;
        self.colour.hsv = rgb_to_hsv(now);
        self.colour.code = TextEdit::new(hex_of(now));
        self.colour.more = false;
        self.colour.dropper = false;
    }

    /// A colour picked whole: a square, a sample, the dropper, a typed code.
    pub fn pick_colour(&mut self, colour: [u8; 3]) {
        let Some(target) = self.colour.open else { return };
        self.colour.hsv = rgb_to_hsv(colour);
        self.colour.code = TextEdit::new(hex_of(colour));
        apply(self, target, colour);
    }

    /// A colour moved on the square or the hue strip.
    fn pick_hsv(&mut self, hsv: [f32; 3]) {
        let Some(target) = self.colour.open else { return };
        self.colour.hsv = hsv;
        let colour = hsv_to_rgb(hsv);
        self.colour.code = TextEdit::new(hex_of(colour));
        apply(self, target, colour);
    }

    /// Closes the picker; Cancel puts back the colour it opened with.
    pub fn finish_colour(&mut self, keep: bool) {
        let Some(target) = self.colour.open.take() else { return };
        self.colour.dropper = false;
        if !keep {
            apply(self, target, self.colour.before);
            return;
        }
        let colour = current(self, target);
        let recent = &mut self.settings.recent_colours;
        recent.retain(|kept| *kept != colour);
        recent.insert(0, colour);
        recent.truncate(RECENT);
        self.touch_settings();
    }

    /// The dropper's press on the slide, at a fraction of its width and height.
    pub fn drop_colour(&mut self, fx: f32, fy: f32) {
        self.colour.dropper = false;
        let look = self.look();
        if let Background::Image(path) = &look.background
            && self.colour.sample.as_ref().map(|(kept, ..)| kept) != Some(path)
        {
            self.colour.sample = decode_small(path).map(|(w, h, bytes)| (path.clone(), w, h, bytes));
        }
        if let Some(colour) = sample(&look, self.colour.sample.as_ref(), fx, fy) {
            self.pick_colour(colour);
        }
    }
}

/// The picture at most 320 points on its long side, enough to take a colour from and small to keep.
fn decode_small(path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let picture = image::open(path).ok()?.thumbnail(320, 320).to_rgb8();
    Some((picture.width(), picture.height(), picture.into_raw()))
}

/// The colour the slide shows behind the words at a fraction of its width and height. A picture
/// covers the slide, as the projector draws it; a video or a camera gives none.
pub fn sample(look: &Look, picture: Option<&(String, u32, u32, Vec<u8>)>, fx: f32, fy: f32) -> Option<[u8; 3]> {
    let (fx, fy) = (fx.clamp(0.0, 1.0), fy.clamp(0.0, 1.0));
    match &look.background {
        Background::Solid(colour) => Some(*colour),
        Background::Gradient(top, bottom) => Some([0, 1, 2].map(|i| (f32::from(top[i]) + (f32::from(bottom[i]) - f32::from(top[i])) * fy).round() as u8)),
        Background::Image(path) => {
            let (kept, w, h, bytes) = picture?;
            if kept != path || *w == 0 || *h == 0 {
                return None;
            }
            // Cover a 16 by 9 slide: the picture scaled to fill it, its overflow cut evenly.
            let (w, h) = (*w as f32, *h as f32);
            let scale = (16.0 / w).max(9.0 / h);
            let (px, py) = ((fx * 16.0 + (w * scale - 16.0) / 2.0) / scale, (fy * 9.0 + (h * scale - 9.0) / 2.0) / scale);
            let (x, y) = ((px as u32).min(w as u32 - 1), (py as u32).min(h as u32 - 1));
            let at = ((y * w as u32 + x) * 3) as usize;
            bytes.get(at..at + 3).map(|rgb| [rgb[0], rgb[1], rgb[2]])
        }
        Background::Video(_) | Background::Camera(_) => None,
    }
}

/// Hue in degrees, saturation and value from 0 to 1, to sRGB bytes.
pub fn hsv_to_rgb([h, s, v]: [f32; 3]) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r, g, b].map(|channel| ((channel + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

pub fn rgb_to_hsv(colour: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = colour.map(|channel| f32::from(channel) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    [h, if max == 0.0 { 0.0 } else { d / max }, max]
}

/// An OKLCH colour as sRGB bytes, clipped to what a screen can show.
pub fn oklch_to_rgb(l: f32, c: f32, h: f32) -> [u8; 3] {
    let linear = rune::oklch(l, c, h);
    let encode = |x: f32| {
        let x = x.clamp(0.0, 1.0);
        let v = if x <= 0.003_130_8 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (v * 255.0).round() as u8
    };
    [encode(linear.r), encode(linear.g), encode(linear.b)]
}

/// A square of colour that picks itself; the chosen one has a ring.
fn swatch(k: &Kit, key: String, name: String, colour: [u8; 3], size: f32, on: bool) -> El {
    let ring = if on { k.c.text_base } else { Color::TRANSPARENT };
    el().key(key)
        .role("button")
        .label(name)
        .size(size + 8.0, size + 8.0)
        .center()
        .radius(k.radius.md + 2.0)
        .border(2.0, ring)
        .focusable(true)
        .on_key(super::widgets::enter_key)
        .focus_selected(on)
        .cursor(Cursor::Pointer)
        .child(el().size(size, size).radius(k.radius.md).fill(rgb(colour)).border(1.0, k.c.border_strong).hover(Color::WHITE.alpha(0.12)))
        .on_press(move |s: &mut App| s.pick_colour(colour))
}

fn caption(k: &Kit, words: &str) -> El {
    text(words.to_uppercase()).font(type_::TINY.tracking(0.8)).color(k.c.text_muted)
}

/// A setting row of a colour: the usual colours as squares, then More, which opens the picker
/// under the row. `quick` names each usual colour.
pub fn rows(s: &App, k: &Kit, target: ColourFor, label: &'static str, quick: &[(&'static str, [u8; 3])]) -> Vec<El> {
    let now = current(s, target);
    let open = s.colour.open == Some(target);
    // One choice of several: the arrows choose as they move, as in the switches of this panel.
    let mut squares = row().role("toolbar").label(label).gap(space::S1).center_y().focus_group().focus_follows();
    for (name, colour) in quick {
        let colour = *colour;
        let key = format!("look-{}-{}", target.key(), hex_of(colour));
        squares = squares.child(swatch(k, key, (*name).to_string(), colour, 26.0, now == colour).on_press(move |s: &mut App| {
            if s.colour.open == Some(target) {
                s.pick_colour(colour);
            } else {
                apply(s, target, colour);
            }
        }));
    }
    // More shows the colour in use when it is none of the usual ones.
    let custom = !quick.iter().any(|(_, colour)| *colour == now);
    let more = row()
        .key(format!("colour-more-{}", target.key()))
        .role("button")
        .label(t::COLOUR_MORE())
        .checked(open)
        .h(36.0)
        .pl(6.0)
        .pr(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(if open { theme::accent().alpha(0.18) } else { k.c.layer_02 })
        .border(1.0, if open { theme::accent() } else { k.c.border_base })
        .hover(k.c.hover)
        .focusable(true)
        .on_key(super::widgets::enter_key)
        .cursor(Cursor::Pointer)
        .child(if custom { el().size(24.0, 24.0).radius(k.radius.sm).fill(rgb(now)).border(1.0, k.c.border_strong).a11y_hidden() } else { el().size(24.0, 24.0).radius(k.radius.sm).linear_gradient(std::f32::consts::FRAC_PI_2, rainbow()).a11y_hidden() })
        .child(text(t::COLOUR_MORE()).font(type_::BODY).color(k.c.text_base))
        .child(icon(if open { k.glyph.chevron_up } else { k.glyph.chevron_down }, 14.0).color(k.c.text_muted).a11y_hidden())
        .on_press(move |s: &mut App| s.toggle_colour(target));
    let line = setting(k, label, row().grow(1.0).min_w(0.0).gap(space::S2).center_y().child(squares).child(more));
    let mut out = vec![line];
    if open {
        out.push(panel(s, k, target, label));
    }
    out
}

fn rainbow() -> Vec<(f32, Color)> {
    (0..=6).map(|i| (i as f32 / 6.0, rgb(oklch_to_rgb(0.72, 0.16, i as f32 * 60.0)))).collect()
}

/// The picker itself, under its row.
fn panel(s: &App, k: &Kit, target: ColourFor, label: &'static str) -> El {
    let sf = theme::surf();
    let now = current(s, target);
    let look = s.look();
    // The colours already on the slide: the words, the line, and the background's own.
    let mut theirs = vec![look.ink, look.outline_ink];
    match &look.background {
        Background::Solid(colour) => theirs.push(*colour),
        Background::Gradient(top, bottom) => theirs.extend([*top, *bottom]),
        _ => {}
    }
    theirs.extend([[255, 255, 255], [255, 244, 222], [255, 214, 0], [0, 0, 0]]);
    let mut seen = Vec::new();
    theirs.retain(|colour| {
        let fresh = !seen.contains(colour);
        seen.push(*colour);
        fresh
    });
    let line_of = |colours: &[[u8; 3]], prefix: &str| {
        let mut line = row().role("toolbar").gap(space::S1).flex_wrap().focus_group().focus_follows();
        for colour in colours {
            line = line.child(swatch(k, format!("colour-{prefix}-{}", hex_of(*colour)), format!("#{}", hex_of(*colour)), *colour, 24.0, *colour == now));
        }
        line
    };
    let mut grid = col().key("colour-grid").gap(2.0);
    for l in LIGHTNESS {
        let colours: Vec<[u8; 3]> = HUES.iter().map(|h| oklch_to_rgb(l, 0.14, *h)).collect();
        grid = grid.child(line_of(&colours, "grid"));
    }
    let head = row()
        .w_full()
        .gap(space::S3)
        .center_y()
        .child(el().size(32.0, 32.0).radius(16.0).fill(rgb(now)).border(1.0, k.c.border_strong).a11y_hidden())
        .child(text(label).font(type_::TITLE).color(k.c.text_base).grow(1.0).truncate().min_w(0.0))
        .child(btn(t::BTN_CANCEL()).small().focusable().hint("Esc").build(k, |s, _| s.finish_colour(false)))
        .child(btn(t::COLOUR_USE()).small().primary().icon(&icons::CHECK).focusable().build(k, |s, _| s.finish_colour(true)).key("colour-use"));
    // The samples and a colour of one's own side by side, as the mock draws them.
    let own = col().gap(space::S2).child(caption(k, t::COLOUR_OWN())).child(row().gap(space::S3).child(shade(s, k)).child(hue(s))).child(dropper(s, k));
    let pickers = row().w_full().gap(space::S5).child(col().gap(space::S2).child(caption(k, t::COLOUR_SAMPLES())).child(grid)).child(own);
    let recent = &s.settings.recent_colours;
    let code = col()
        .w_full()
        .gap(space::S2)
        .child(
            row()
                .key("colour-code-toggle")
                .role("button")
                .label(t::COLOUR_CODE())
                .checked(s.colour.more)
                .h(28.0)
                .px(space::S1)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .focusable(true)
                .on_key(super::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .child(icon(if s.colour.more { k.glyph.chevron_down } else { k.glyph.caret_right }, 12.0).color(k.c.text_muted).a11y_hidden())
                .child(text(t::COLOUR_CODE()).font(type_::BODY).color(k.c.text_muted))
                .on_press(|s: &mut App| s.colour.more = !s.colour.more),
        )
        .children(s.colour.more.then(|| {
            let field = TextInput::new(&s.colour.code)
                .width(None)
                .label(t::LOOK_COLOUR_HEX())
                .placeholder(t::LOOK_COLOUR_HEX_HINT())
                .on_edit(|s: &mut App| &mut s.colour.code)
                .on_change(|s: &mut App, _: &mut Tx| {
                    if let (Some(target), Some(colour)) = (s.colour.open, parse_hex(&s.colour.code.text)) {
                        s.colour.hsv = rgb_to_hsv(colour);
                        apply(s, target, colour);
                    }
                });
            row().gap(space::S2).center_y().child(text("#").font(type_::BODY).color(k.c.text_muted).a11y_hidden()).child(el().w(140.0).child(field.build(k)))
        }));
    col()
        .key("colour-panel")
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S3)
        .radius(10.0)
        .fill(sf.raised)
        .border(1.0, sf.edge)
        .child(head)
        .child(caption(k, t::COLOUR_DESIGN()))
        .child(line_of(&theirs, "design"))
        .child(pickers)
        .children((!recent.is_empty()).then(|| caption(k, t::COLOUR_RECENT())))
        .children((!recent.is_empty()).then(|| line_of(recent, "recent")))
        .child(code)
}

/// The square's size: as high as the four rows of samples beside it.
const SHADE: [f32; 2] = [170.0, 136.0];

/// Saturation left to right and value bottom to top, at the picker's hue. Arrows move the ring.
fn shade(s: &App, k: &Kit) -> El {
    let [h, sat, val] = s.colour.hsv;
    let pure = rgb(hsv_to_rgb([h, 1.0, 1.0]));
    let (x, y) = (sat * SHADE[0], (1.0 - val) * SHADE[1]);
    el().key("colour-shade")
        .role("slider")
        .label(t::COLOUR_SHADE())
        .size(SHADE[0], SHADE[1])
        .radius(k.radius.md)
        .fill(pure)
        .clip_rounded(k.radius.md)
        .focusable(true)
        .cursor(Cursor::Crosshair)
        .child(el().absolute(0.0, 0.0).size(SHADE[0], SHADE[1]).linear_gradient(std::f32::consts::FRAC_PI_2, [(0.0, Color::WHITE), (1.0, Color::WHITE.alpha(0.0))]).a11y_hidden())
        .child(el().absolute(0.0, 0.0).size(SHADE[0], SHADE[1]).linear_gradient(std::f32::consts::PI, [(0.0, Color::BLACK.alpha(0.0)), (1.0, Color::BLACK)]).a11y_hidden())
        .child(el().absolute(x - 8.0, y - 8.0).size(16.0, 16.0).radius(8.0).border(2.5, Color::WHITE).shadow(1.0, 3.0, Color::BLACK.alpha(0.5)).a11y_hidden())
        .on_drag(|s: &mut App, d: &mut Drag, _: &mut Tx| {
            if d.phase != Phase::Up && d.w > 0.0 && d.h > 0.0 {
                let [h, ..] = s.colour.hsv;
                s.pick_hsv([h, (d.x / d.w).clamp(0.0, 1.0), 1.0 - (d.y / d.h).clamp(0.0, 1.0)]);
            }
        })
        .on_key(|s: &mut App, ev: &rune::KeyEvent, _: &mut Tx| {
            let [h, sat, val] = s.colour.hsv;
            let step = 0.04;
            let next = match ev.key {
                Key::Left => [h, (sat - step).max(0.0), val],
                Key::Right => [h, (sat + step).min(1.0), val],
                Key::Up => [h, sat, (val + step).min(1.0)],
                Key::Down => [h, sat, (val - step).max(0.0)],
                _ => return false,
            };
            s.pick_hsv(next);
            true
        })
}

/// The hue strip, top to bottom round the colour wheel. Up and Down move it.
fn hue(s: &App) -> El {
    let [h, ..] = s.colour.hsv;
    let stops: Vec<(f32, Color)> = (0..=12).map(|i| (i as f32 / 12.0, rgb(hsv_to_rgb([i as f32 * 30.0, 1.0, 1.0])))).collect();
    el().key("colour-hue")
        .role("slider")
        .label(t::COLOUR_HUE())
        .range_value(f64::from(h), 0.0, 360.0)
        .size(20.0, SHADE[1])
        .radius(10.0)
        .linear_gradient(std::f32::consts::PI, stops)
        .focusable(true)
        .cursor(Cursor::Pointer)
        .child(el().absolute(-3.0, h / 360.0 * SHADE[1] - 13.0).size(26.0, 26.0).radius(13.0).border(2.5, Color::WHITE).shadow(1.0, 3.0, Color::BLACK.alpha(0.5)).a11y_hidden())
        .on_drag(|s: &mut App, d: &mut Drag, _: &mut Tx| {
            if d.phase != Phase::Up && d.h > 0.0 {
                let [_, sat, val] = s.colour.hsv;
                s.pick_hsv([(d.y / d.h).clamp(0.0, 0.999) * 360.0, sat, val]);
            }
        })
        .on_key(|s: &mut App, ev: &rune::KeyEvent, _: &mut Tx| {
            let [h, sat, val] = s.colour.hsv;
            let next = match ev.key {
                Key::Up => (h - 6.0).rem_euclid(360.0),
                Key::Down => (h + 6.0).rem_euclid(360.0),
                _ => return false,
            };
            s.pick_hsv([next, sat, val]);
            true
        })
}

/// The dropper: lit while the next press on the slide takes its colour. A video or a camera has
/// no colour to take, so the button is off then and says why.
fn dropper(s: &App, k: &Kit) -> El {
    let usable = matches!(s.look().background, Background::Solid(_) | Background::Gradient(..) | Background::Image(_));
    let button = btn(t::COLOUR_DROPPER()).icon(&icons::DROPPER).active(s.colour.dropper).focusable();
    let button = if usable { button.build(k, |s, _| s.colour.dropper = !s.colour.dropper) } else { button.disabled(true).help(t::COLOUR_DROPPER_NONE()).build(k, |_, _| {}) };
    row().gap(space::S2).center_y().child(button.key("colour-dropper")).children(s.colour.dropper.then(|| text(t::COLOUR_DROPPER_ON()).font(type_::SMALL).color(theme::accent()).wrap().w(110.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_goes_to_hsv_and_back_unchanged() {
        for colour in [[255, 214, 0], [0, 0, 0], [255, 255, 255], [18, 52, 86], [200, 30, 120], [128, 128, 128]] {
            assert_eq!(hsv_to_rgb(rgb_to_hsv(colour)), colour, "{colour:?}");
        }
        assert_eq!(hsv_to_rgb([0.0, 1.0, 1.0]), [255, 0, 0]);
        assert_eq!(hsv_to_rgb([120.0, 1.0, 1.0]), [0, 255, 0]);
        assert_eq!(hsv_to_rgb([360.0, 1.0, 1.0]), [255, 0, 0], "the strip's end is its start");
    }

    #[test]
    fn oklch_white_and_black_are_the_screen_s_own() {
        assert_eq!(oklch_to_rgb(1.0, 0.0, 0.0), [255, 255, 255]);
        assert_eq!(oklch_to_rgb(0.0, 0.0, 0.0), [0, 0, 0]);
        // A row of the grid keeps one lightness: no sample is black or white.
        for h in HUES {
            let colour = oklch_to_rgb(0.62, 0.14, h);
            assert!(colour.iter().any(|c| *c > 40) && colour.iter().any(|c| *c < 230), "{h}: {colour:?}");
        }
    }

    #[test]
    fn the_dropper_reads_the_background_where_it_was_pressed() {
        let solid = Look { background: Background::Solid([10, 20, 30]), ..Look::default() };
        assert_eq!(sample(&solid, None, 0.3, 0.9), Some([10, 20, 30]));
        let gradient = Look { background: Background::Gradient([0, 0, 0], [200, 100, 50]), ..Look::default() };
        assert_eq!(sample(&gradient, None, 0.5, 0.0), Some([0, 0, 0]));
        assert_eq!(sample(&gradient, None, 0.5, 1.0), Some([200, 100, 50]));
        // A square picture covers the slide: its top and bottom are cut, its left half is red.
        let mut bytes = Vec::new();
        for _y in 0..4 {
            for x in 0..4 {
                bytes.extend(if x < 2 { [255, 0, 0] } else { [0, 0, 255] });
            }
        }
        let picture = Look { background: Background::Image("p.png".into()), ..Look::default() };
        let decoded = ("p.png".to_string(), 4, 4, bytes);
        assert_eq!(sample(&picture, Some(&decoded), 0.1, 0.5), Some([255, 0, 0]));
        assert_eq!(sample(&picture, Some(&decoded), 0.9, 0.5), Some([0, 0, 255]));
        assert_eq!(sample(&picture, None, 0.9, 0.5), None, "not decoded yet");
        let video = Look { background: Background::Video("v.mp4".into()), ..Look::default() };
        assert_eq!(sample(&video, None, 0.5, 0.5), None);
    }
}
