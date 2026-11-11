//! The audience screen: text over a background, drawn the same in a preview and on the projector.
//!
//! Every size is a fraction of the height, with 1080 as the reference, so a 1920x1080 canvas
//! scaled down wraps its lines exactly where the projector does.

use std::hash::{Hash, Hasher};

use artosia_core::deck::{Slide, SlideKind};
use rune::{Align, Color, El, NativeSurface, Pixels, SurfaceFit, TextStyle, Transition, col, el, row, text};

use crate::frame::{Countdown, OutputState};
use crate::fonts;
use crate::look::{Background, Look, Outline, Place, rgb};

/// Pictures and video a screen draws; the caller loads them.
#[derive(Clone, Copy, Default)]
pub struct Media<'a> {
    /// The decoded background picture, or a video's first frame in a preview.
    pub picture: Option<&'a Pixels>,
    /// Live video frames, on the output only.
    pub video: Option<&'a NativeSurface>,
    pub logo: Option<&'a Pixels>,
    /// The picture of the slide being drawn, when it has one and it has loaded.
    pub slide: Option<&'a Pixels>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Detail {
    /// The real screen: soft text shadow, cross-fades.
    Output,
    /// A preview in the operator window: nothing that costs a blur pass, and a thinner ring of
    /// copies for the outline.
    Preview,
    /// A tile a few hundred points wide, such as a look's sample: no shadow and no outline.
    Tile,
}

/// Seconds left as "4:59", or "1:04:59" past an hour.
pub fn clock_text(seconds: f64) -> String {
    let total = seconds.max(0.0).ceil() as u64;
    let (hours, minutes, seconds) = (total / 3600, total / 60 % 60, total % 60);
    if hours > 0 { format!("{hours}:{minutes:02}:{seconds:02}") } else { format!("{minutes}:{seconds:02}") }
}

fn key_of(slide: &Slide) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (&slide.label, &slide.heading, &slide.text, &slide.second, &slide.image).hash(&mut hasher);
    format!("slide-{:x}", hasher.finish())
}

fn background(look: &Look, w: f32, h: f32, media: Media) -> El {
    let layer = el().key("background").absolute(0.0, 0.0).size(w, h).a11y_hidden();
    match &look.background {
        Background::Solid(color) => layer.fill(rgb(*color)),
        Background::Gradient(top, bottom) => layer.gradient(rgb(*top), rgb(*bottom)),
        Background::Image(_) | Background::Video(_) | Background::Camera(_) => {
            let layer = layer.fill(Color::BLACK);
            match (media.video, media.picture) {
                (Some(video), _) => layer.native_surface(video, SurfaceFit::Fill),
                (None, Some(picture)) => layer.pixels_fit(picture, SurfaceFit::Fill),
                (None, None) => layer,
            }
        }
    }
}

struct Sizes {
    main: TextStyle,
    second: TextStyle,
    heading: TextStyle,
    gap: f32,
}

fn sizes(size: f32, look: &Look) -> Sizes {
    let family = fonts::family(&look.font);
    Sizes {
        main: TextStyle::new(size, look.weight).family(family).lh((size * 1.24).round()),
        second: TextStyle::new((size * 0.7).round(), (look.weight - 200.0).max(400.0)).family(family).italic().lh((size * 0.7 * 1.26).round()),
        heading: TextStyle::new((size * 1.3).round(), 800.0).family(family).lh((size * 1.3 * 1.15).round()),
        gap: (size * 0.42).round(),
    }
}

/// A line of text with what sits under it: the outline, as copies of the text offset round a
/// circle in the outline's colour (Rune has no text stroke), then the shadow. `paint` makes the
/// text in a colour; `h` is the screen's height, which the outline's width is a fraction of.
fn dressed(look: &Look, detail: Detail, h: f32, width: f32, ink: Color, paint: impl Fn(Color) -> El) -> El {
    let u = h / 1080.0;
    let body = paint(ink);
    let shaded = look.shadow && detail == Detail::Output;
    // A thumbnail is too small to show a line round the letters, and a plan can hold many.
    let outline = if detail == Detail::Tile { Outline::Off } else { look.outline };
    if !shaded && outline == Outline::Off {
        return body;
    }
    let mut stack = el().w(width);
    if shaded {
        // A blurred black copy under the text keeps it readable over a bright video frame.
        stack = stack.child(paint(Color::BLACK).absolute(0.0, (3.0 * u).max(1.0)).blur((7.0 * u).max(1.0)).opacity(0.75).a11y_hidden());
    }
    if outline != Outline::Off {
        let reach = outline.reach(h);
        // The projector draws the full ring; a preview, drawn small, needs only eight.
        let copies = if detail == Detail::Output { outline.copies() } else { 8 };
        let line = rgb(look.outline_ink);
        for step in 0..copies {
            let angle = std::f32::consts::TAU * step as f32 / copies as f32;
            stack = stack.child(paint(line).absolute(reach * angle.cos(), reach * angle.sin()).a11y_hidden());
        }
    }
    stack.child(body)
}

/// A slide's words as a table: a line holding a tab is a row and the tabs part its cells.
/// None when no line holds one. A bulletin's schedule or a birthday list arrives this way.
fn table_of(text: &str) -> Option<Vec<Vec<&str>>> {
    text.contains('\t').then(|| text.lines().map(|line| line.split('\t').map(str::trim).collect()).collect())
}

/// The width each column needs at a text style, and the gap between columns.
fn column_widths(rows: &[Vec<&str>], style: TextStyle) -> (Vec<f32>, f32) {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0f32; columns];
    for cells in rows {
        // A line with no tab in a table is a heading across it, and claims no column.
        if cells.len() < 2 {
            continue;
        }
        for (at, cell) in cells.iter().enumerate() {
            widths[at] = widths[at].max(rune::text::measure(cell, &[], style, None).0.ceil());
        }
    }
    (widths, (style.size * 1.1).round())
}

/// The largest text size at which a slide fits its box.
fn fit(slide: &Slide, look: &Look, width: f32, height: f32, base: f32, floor: f32) -> f32 {
    let mut size = base;
    if let Some(rows) = table_of(&slide.text) {
        // A table never wraps a cell: it shrinks until its widest row and all its rows fit.
        for _ in 0..40 {
            let s = sizes(size, look);
            let (widths, gap) = column_widths(&rows, s.main);
            let wide = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
            let mut tall = rows.len() as f32 * (size * 1.24).round();
            if !slide.heading.is_empty() {
                tall += rune::text::measure(&slide.heading, &[], s.heading, Some(width)).1 + s.gap;
            }
            if (wide <= width && tall <= height) || size <= floor {
                break;
            }
            size = (size * 0.94).floor();
        }
        return size.max(floor);
    }
    if slide.kind == SlideKind::Lyrics {
        // A sung line reads best unbroken: shrink until the widest line fits before wrapping any.
        let style = sizes(size, look).main;
        let widest = slide.text.lines().map(|line| rune::text::measure(line, &[], style, None).0).fold(0.0, f32::max);
        if widest > width {
            size = (size * width / widest).floor().max((base * 0.6).floor());
        }
    }
    for _ in 0..28 {
        let s = sizes(size, look);
        let mut total = rune::text::measure(&slide.text, &[], s.main, Some(width)).1;
        if !slide.heading.is_empty() {
            total += rune::text::measure(&slide.heading, &[], s.heading, Some(width)).1 + s.gap;
        }
        if !slide.second.is_empty() {
            total += rune::text::measure(&slide.second, &[], s.second, Some(width)).1 + s.gap;
        }
        if total <= height || size <= floor {
            break;
        }
        size = (size * 0.94).floor();
    }
    size.max(floor)
}

/// A slide's picture: the whole screen when the slide has no words, the upper part when it has.
fn slide_picture(slide: &Slide, picture: &Pixels, w: f32, h: f32) -> El {
    let wordless = slide.heading.trim().is_empty() && slide.text.trim().is_empty();
    let (mx, my) = ((w * 0.04).round(), (h * 0.05).round());
    let tall = if wordless { h - 2.0 * my } else { (h * PICTURE_SHARE).round() - my };
    el().key(format!("{}-picture", key_of(slide))).absolute(mx, my).size(w - 2.0 * mx, tall).pixels_fit(picture, SurfaceFit::Fit).transition(Transition::opacity()).a11y_hidden()
}

/// How much of the screen's height a picture takes when words sit under it.
const PICTURE_SHARE: f32 = 0.56;

fn slide_text(slide: &Slide, look: &Look, w: f32, h: f32, detail: Detail, under_picture: bool) -> El {
    let u = h / 1080.0;
    let ink = look.ink_color();
    let (mx, mut my) = ((w * 0.07).round(), (h * 0.085).round());
    let caption_room = if slide.caption.is_empty() { 0.0 } else { (64.0 * u).round() };
    let (box_w, mut box_h) = (w - 2.0 * mx, h - 2.0 * my - caption_room);
    if under_picture {
        // The words take what the picture leaves.
        let top = (h * PICTURE_SHARE).round();
        box_h = (h - top - my * 0.6).max(1.0);
        my = top;
    }
    let base = match slide.kind {
        SlideKind::Lyrics => 96.0,
        SlideKind::Scripture => 78.0,
        SlideKind::Announcement => 66.0,
    };
    let size = fit(slide, look, box_w, box_h, (base * u * look.size).round(), (26.0 * u).round().max(4.0));
    let s = sizes(size, look);
    let align = if look.centered { Align::Center } else { Align::Start };
    let line = |content: &str, style: TextStyle, color: Color| -> El {
        dressed(look, detail, h, box_w, color, |paint| text(content).font(style).color(paint).align(align).w(box_w).wrap())
    };
    let mut block = col().key(key_of(slide)).absolute(mx, my).size(box_w, box_h).gap(s.gap).transition(Transition::opacity());
    block = match look.place {
        Place::Top => block.justify_start(),
        Place::Middle => block.justify_center(),
        Place::Bottom => block.justify_end(),
    };
    if !slide.heading.is_empty() {
        block = block.child(line(&slide.heading, s.heading, ink).key("heading"));
    }
    if let Some(rows) = table_of(&slide.text) {
        let (widths, gap) = column_widths(&rows, s.main);
        let wide = widths.iter().sum::<f32>() + gap * widths.len().saturating_sub(1) as f32;
        let cell = |content: &str, width: f32| -> El { dressed(look, detail, h, width, ink, |paint| text(content).font(s.main).color(paint).w(width).truncate()) };
        let mut table = col().key("table").w(wide.min(box_w));
        for (at, cells) in rows.iter().enumerate() {
            let mut line = row().key(format!("row-{at}")).w(wide.min(box_w)).h(s.main.line_height).gap(gap);
            if cells.len() < 2 {
                line = line.child(cell(cells.first().copied().unwrap_or(""), wide.min(box_w)));
            } else {
                for (column, content) in cells.iter().enumerate() {
                    line = line.child(cell(content, widths.get(column).copied().unwrap_or(0.0)));
                }
            }
            table = table.child(line);
        }
        // The table sits in the middle of the slide with its own columns flush left.
        block = block.child(row().w(box_w).justify_center().child(table));
    } else {
        block = block.child(line(&slide.text, s.main, ink).key("text"));
    }
    if !slide.second.is_empty() {
        block = block.child(line(&slide.second, s.second, ink.alpha(0.86)).key("second"));
    }
    let mut layer = el().key("text-layer").absolute(0.0, 0.0).size(w, h).child(block);
    if !slide.caption.is_empty() {
        let style = TextStyle::new((30.0 * u).round().max(3.0), 500.0).family(fonts::family(&look.font));
        layer = layer.child(
            text(slide.caption.clone())
                .key(format!("{}-caption", key_of(slide)))
                .font(style)
                .color(ink.alpha(0.7))
                .align(Align::Center)
                .w(box_w)
                .truncate()
                .absolute(mx, h - my - (38.0 * u).round())
                .transition(Transition::opacity()),
        );
    }
    layer
}

fn countdown(countdown: &Countdown, look: &Look, w: f32, h: f32, now: f64) -> El {
    let u = h / 1080.0;
    let ink = look.ink_color();
    let family = fonts::family(&look.font);
    let digits = TextStyle::new((260.0 * u).round(), 700.0).family(family).features("'tnum' 1");
    let label = TextStyle::new((54.0 * u).round(), 500.0).family(family);
    let mut stack = col().key("countdown").absolute(0.0, 0.0).size(w, h).center().gap((12.0 * u).round());
    if !countdown.label.is_empty() {
        stack = stack.child(text(countdown.label.clone()).font(label).color(ink.alpha(0.8)));
    }
    stack.child(text(clock_text(countdown.ends_at - now)).key("digits").font(digits).color(ink))
}

fn message(message: &str, w: f32, h: f32) -> El {
    let u = h / 1080.0;
    let style = TextStyle::new((40.0 * u).round().max(4.0), 600.0);
    let pill = el()
        .key("message")
        .px((40.0 * u).round())
        .py((18.0 * u).round())
        .radius((20.0 * u).round())
        .max_w(w * 0.86)
        .fill(Color::BLACK.alpha(0.82))
        .border((2.0 * u).max(1.0), Color::WHITE.alpha(0.22))
        .transition(Transition::opacity())
        .child(text(message).font(style).color(Color::WHITE).align(Align::Center).wrap());
    col().key("message-layer").absolute(0.0, (h * 0.045).round()).w(w).center().child(pill)
}

/// The whole audience screen at `w` by `h` points. `now` is seconds since the Unix epoch.
pub fn audience(state: &OutputState, w: f32, h: f32, media: Media, detail: Detail, now: f64) -> El {
    let look = &state.look;
    let mut root = el().key("audience").size(w, h).fill(Color::BLACK).role("img");
    if state.black {
        return root;
    }
    root = root.child(background(look, w, h, media));
    if look.is_media() && look.dim > 0.0 {
        root = root.child(el().key("dim").absolute(0.0, 0.0).size(w, h).fill(Color::BLACK.alpha(look.dim.clamp(0.0, 1.0))).a11y_hidden());
    }
    let logo = media.logo.filter(|_| state.logo_on);
    if let Some(logo) = logo {
        let (lw, lh) = ((w * 0.5).round(), (h * 0.5).round());
        root = root.child(
            el().key("logo")
                .absolute(((w - lw) / 2.0).round(), ((h - lh) / 2.0).round())
                .size(lw, lh)
                .pixels_fit(logo, SurfaceFit::Fit)
                .transition(Transition::opacity()),
        );
    } else {
        match state.slide.as_ref().filter(|_| !state.text_hidden && !state.logo_on) {
            Some(slide) => {
                let picture = media.slide.filter(|_| !slide.image.is_empty());
                if let Some(picture) = picture {
                    root = root.child(slide_picture(slide, picture, w, h));
                }
                root = root.child(slide_text(slide, look, w, h, detail, picture.is_some()));
            }
            None => {
                if let Some(timer) = &state.countdown {
                    root = root.child(countdown(timer, look, w, h, now));
                }
            }
        }
    }
    if !state.message.trim().is_empty() {
        root = root.child(message(state.message.trim(), w, h));
    }
    root
}

/// A thumbnail narrower than this draws no outline: the line would be under a point wide.
const TILE_WIDTH: f32 = 300.0;

/// The audience screen drawn on a 1920x1080 canvas and scaled into a `width` wide tile, so a
/// preview breaks its lines where the projector does.
pub fn thumbnail(state: &OutputState, width: f32, media: Media, now: f64) -> El {
    let (w, h) = (1920.0, 1080.0);
    let scale = width / w;
    let height = (width * 9.0 / 16.0).round();
    el().key("thumbnail")
        .size(width, height)
        .child(audience(state, w, h, media, if width < TILE_WIDTH { Detail::Tile } else { Detail::Preview }, now).absolute(0.0, 0.0).origin(0.0, 0.0).scale(scale).inert())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_text_rounds_up_and_shows_hours() {
        assert_eq!(clock_text(299.2), "5:00");
        assert_eq!(clock_text(59.0), "0:59");
        assert_eq!(clock_text(3900.0), "1:05:00");
        assert_eq!(clock_text(-4.0), "0:00");
    }
}
