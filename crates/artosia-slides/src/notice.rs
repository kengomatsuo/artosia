//! Announcement slides in three structures (Hairline, Ledger, Band) for five kinds of notice.
//!
//! Every size is a fraction of the slide's height, so the same code draws the projector at
//! 1920x1080 and a sheet's small copy. A slide's words are lines; a tab splits a line into cells
//! (`artosia_core::deck::notice_body`), and the kind decides what the cells mean.

use artosia_core::deck::{NoticeBody, NoticeKind, Slide, notice_body};
use rune::{Align, Color, El, Family, TextStyle, col, el, row, text};

use crate::fonts;
use crate::look::{Look, NoticeStyle, rgb};
use crate::warta::{Field, Layout, Pics, TextBox};

/// The word a kind's label, tab and band line show.
pub fn kind_name(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Schedule => "JADWAL",
        NoticeKind::Birthdays => "ULANG TAHUN",
        NoticeKind::Money | NoticeKind::Account => "PERSEMBAHAN",
        NoticeKind::Plain => "PENGUMUMAN",
        NoticeKind::Verse => "AYAT",
    }
}

/// GUESS: a look has no accent colour, so each style carries the accent of the card it follows.
fn accent_rgb(style: NoticeStyle) -> [u8; 3] {
    match style {
        NoticeStyle::Hairline => [124, 196, 106],
        NoticeStyle::Ledger => [104, 72, 184],
        NoticeStyle::Band => [22, 184, 170],
        // The poster styles carry their own palette and never read this.
        _ => [255, 255, 255],
    }
}

fn accent(style: NoticeStyle) -> Color {
    rgb(accent_rgb(style))
}

/// Whether a small label would only repeat the title above or below it, as "ULANG TAHUN" over "Ulang Tahun".
fn repeats(label: &str, title: &str) -> bool {
    label.trim().to_lowercase() == title.trim().to_lowercase()
}

/// A colour's relative lightness, 0 for black to 1 for white.
fn luminance(rgb: [u8; 3]) -> f32 {
    (0.2126 * f32::from(rgb[0]) + 0.7152 * f32::from(rgb[1]) + 0.0722 * f32::from(rgb[2])) / 255.0
}

/// What the words of a kind are called in the editor: a plain notice's lines are its body.
fn body_field(kind: NoticeKind) -> Field {
    match kind {
        NoticeKind::Schedule => Field::Rows,
        NoticeKind::Birthdays => Field::Names,
        NoticeKind::Money => Field::Funds,
        NoticeKind::Account => Field::Account,
        NoticeKind::Verse => Field::Verse,
        NoticeKind::Plain => Field::Body,
    }
}

/// What the words of these styles stand on: the dark scrim over the picture, a ledger's paper, a band's charcoal.
const DARK: [u8; 3] = [14, 14, 18];
const PAPER: [u8; 3] = [250, 250, 246];
const CHARCOAL: [u8; 3] = [32, 32, 34];
const WHITE: [u8; 3] = [255, 255, 255];

/// The colour a style draws its title in until its card picks one.
pub fn title_default(style: NoticeStyle) -> [u8; 3] {
    match style {
        NoticeStyle::Hairline => [124, 196, 106],
        _ => [255, 255, 255],
    }
}

/// The weight of a row that stands out; its inverted ground is the other cue.
const MARKED_WEIGHT: f32 = 900.0;

struct Ctx {
    w: f32,
    h: f32,
    u: f32,
    sans: Family,
    /// The face of a ledger's rows.
    serif: Family,
    accent: Color,
    /// Whether the look's ink is dark, so the look is light and a Hairline keeps it light.
    light: bool,
    ink: [u8; 3],
    ground: [u8; 3],
    /// The title colour the card chose; None leaves the style's own.
    title: Option<Color>,
    /// Where each part of the words was put, for an editor to draw its boxes over.
    boxes: std::cell::RefCell<Vec<TextBox>>,
}

impl Ctx {
    /// Notes the box of one part of the words: its rectangle, its type and what it stands on.
    #[allow(clippy::too_many_arguments)]
    fn record(&self, field: Field, rect: (f32, f32, f32, f32), style: TextStyle, lines: usize, ground: [u8; 3], ink: [u8; 3]) {
        self.boxes.borrow_mut().push(TextBox { field, x: rect.0, y: rect.1, w: rect.2, h: rect.3, size: style.size, lines, ground: Some(ground), ink, family: style.family, weight: style.weight, italic: false, tracking: 0.0, align: artosia_core::deck::Align::Start, pad: 0.0 });
    }
}

/// The cells of a kind, split into blocks of rows side by side.
struct Table {
    blocks: Vec<Vec<Vec<String>>>,
    right: &'static [bool],
    /// The column drawn bold in a Hairline.
    key: usize,
    /// The column that takes the room a wider block leaves.
    stretch: usize,
    /// Per block, per row: the row stands out. Empty when none does.
    marked: Vec<Vec<bool>>,
}

impl Table {
    /// `marked` are the rows, by place, that stand out; only birthdays have them.
    fn of(kind: NoticeKind, rows: &[Vec<String>], marked: &[usize]) -> Table {
        match kind {
            NoticeKind::Schedule => Table { blocks: vec![rows.to_vec()], right: &[false, true, false], key: 0, stretch: 2, marked: Vec::new() },
            NoticeKind::Birthdays => {
                // A name and its date; 11 to 20 names take two columns, more three. The date orders the list, so no number is drawn.
                let blocks = match rows.len() {
                    0..=10 => 1,
                    11..=20 => 2,
                    _ => 3,
                };
                let per = rows.len().div_ceil(blocks).max(1);
                let marks = if marked.is_empty() { Vec::new() } else { (0..rows.len()).collect::<Vec<_>>().chunks(per).map(|block| block.iter().map(|at| marked.contains(at)).collect()).collect() };
                Table { blocks: rows.chunks(per).map(<[_]>::to_vec).collect(), right: &[false, true], key: 0, stretch: 0, marked: marks }
            }
            NoticeKind::Money => Table { blocks: vec![rows.to_vec()], right: &[false, true], key: 0, stretch: 0, marked: Vec::new() },
            NoticeKind::Plain | NoticeKind::Account | NoticeKind::Verse => Table { blocks: vec![rows.to_vec()], right: &[false], key: 0, stretch: 0, marked: Vec::new() },
        }
    }

    /// The same table with each birthday name's middle names as initials, for a slide too narrow for them whole.
    fn shortened(&self, kind: NoticeKind) -> Table {
        let blocks = if kind == NoticeKind::Birthdays {
            self.blocks.iter().map(|block| block.iter().map(|cells| cells.iter().enumerate().map(|(at, cell)| if at == 0 { crate::warta::fit::initials(cell) } else { cell.clone() }).collect()).collect()).collect()
        } else {
            self.blocks.clone()
        };
        Table { blocks, right: self.right, key: self.key, stretch: self.stretch, marked: self.marked.clone() }
    }

    fn rows_tall(&self) -> usize {
        self.blocks.first().map_or(1, Vec::len).max(1)
    }

    fn is_marked(&self, block: usize, row: usize) -> bool {
        self.marked.get(block).and_then(|rows| rows.get(row)).copied().unwrap_or(false)
    }

    fn has_marks(&self) -> bool {
        self.marked.iter().flatten().any(|marked| *marked)
    }
}

fn style_of(family: Family, size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).family(family).lh((size * 1.25).round()).features("'tnum' 1")
}

fn width_of(content: &str, style: TextStyle) -> f32 {
    rune::text::measure(content, &[], style, None).0.ceil() + 1.0
}

/// The largest size, from `max` down to `floor`, at which `need(size)` (width, height) fits.
fn fit_size(need: impl Fn(f32) -> (f32, f32), room: (f32, f32), max: f32, floor: f32) -> f32 {
    let mut size = max.max(floor);
    for _ in 0..80 {
        let (wide, tall) = need(size);
        if (wide <= room.0 && tall <= room.1) || size <= floor {
            break;
        }
        size = (size * 0.96).max(floor);
    }
    size
}

/// The title's style: `size`, shrunk until the words fit `width` on one line.
fn title_style(family: Family, content: &str, size: f32, weight: f32, width: f32) -> TextStyle {
    let style = TextStyle::new(size, weight).family(family).lh((size * 1.1).round());
    let wide = width_of(content, style);
    if wide <= width {
        return style;
    }
    let size = (size * width / wide).floor();
    TextStyle::new(size, weight).family(family).lh((size * 1.1).round())
}

/// Blocks of rows laid out as columns on shared edges.
struct Grid<'a> {
    table: &'a Table,
    styles: Vec<TextStyle>,
    /// The same styles at the weight a marked row is drawn in.
    heavy: Vec<TextStyle>,
    gap: f32,
    pad: f32,
    /// Per block, per column.
    widths: Vec<Vec<f32>>,
}

struct Paint {
    colors: Vec<Color>,
    row_h: f32,
    row_line: Option<Color>,
    col_line: Option<Color>,
    /// A marked row's ground and the ink of its words; the rows stand out by this and by weight.
    marked: (Color, Color),
    /// The width the blocks and the gaps between them are spread to, when they should fill it.
    spread: Option<f32>,
    block_gap: f32,
}

impl<'a> Grid<'a> {
    fn new(table: &'a Table, styles: Vec<TextStyle>, gap: f32, pad: f32) -> Grid<'a> {
        let heavy: Vec<TextStyle> = styles.iter().map(|style| TextStyle { weight: MARKED_WEIGHT, ..*style }).collect();
        let widths = table
            .blocks
            .iter()
            .enumerate()
            .map(|(b, block)| {
                (0..styles.len())
                    .map(|at| block.iter().enumerate().map(|(r, cells)| width_of(cells.get(at).map_or("", String::as_str), if table.is_marked(b, r) { heavy[at] } else { styles[at] })).fold(0.0, f32::max) + 2.0 * pad)
                    .collect()
            })
            .collect();
        Grid { table, styles, heavy, gap, pad, widths }
    }

    fn block_width(&self, block: usize) -> f32 {
        let widths = &self.widths[block];
        widths.iter().sum::<f32>() + self.gap * widths.len().saturating_sub(1) as f32
    }

    /// The width the blocks need side by side with `block_gap` between.
    fn natural(&self, block_gap: f32) -> f32 {
        (0..self.widths.len()).map(|at| self.block_width(at)).sum::<f32>() + block_gap * self.widths.len().saturating_sub(1) as f32
    }

    fn build(&self, paint: &Paint) -> El {
        let mut blocks = row().key("blocks").gap(paint.block_gap).items_start();
        for (at, block) in self.table.blocks.iter().enumerate() {
            let natural = self.block_width(at);
            // Wider room spreads the blocks; narrower room takes the difference from the stretching column, which truncates.
            let mut widths = self.widths[at].clone();
            let stretch = self.table.stretch.min(widths.len() - 1);
            let extra = paint.spread.map_or(0.0, |total| (total - self.natural(paint.block_gap)) / self.widths.len() as f32).max(-0.7 * widths[stretch]);
            let wide = natural + extra;
            widths[stretch] += extra;
            let mut lines = col().key(format!("block-{at}")).w(wide);
            if at > 0 && let Some(line) = paint.col_line {
                lines = lines.border_sides([0.0, 0.0, 0.0, 1.0], line);
            }
            for (index, cells) in block.iter().enumerate() {
                let marked = self.table.is_marked(at, index);
                let mut line = row().key(format!("row-{index}")).w(wide).h(paint.row_h).items_center().gap(self.gap);
                if marked {
                    line = line.fill(paint.marked.0);
                }
                if let Some(rule) = paint.row_line {
                    line = line.border_sides([0.0, 0.0, 1.0, 0.0], rule);
                }
                for (column, width) in widths.iter().enumerate() {
                    let content = cells.get(column).map_or("", String::as_str);
                    let rule = paint.col_line.filter(|_| column > 0);
                    let (style, colour) = if marked { (self.heavy[column], paint.marked.1) } else { (self.styles[column], paint.colors[column]) };
                    line = line.child(cell(content, style, colour, *width, self.table.right[column], self.pad, rule));
                }
                lines = lines.child(line);
            }
            blocks = blocks.child(lines);
        }
        blocks
    }
}

fn cell(content: &str, style: TextStyle, color: Color, width: f32, right: bool, pad: f32, rule: Option<Color>) -> El {
    let inner = (width - 2.0 * pad).max(1.0);
    let words = text(content).font(style).color(color).w(inner).align(if right { Align::End } else { Align::Start }).truncate();
    if pad == 0.0 && rule.is_none() {
        return words;
    }
    let mut wrap = row().w(width).h_full().items_center().justify_center().child(words);
    if let Some(rule) = rule {
        wrap = wrap.border_sides([0.0, 0.0, 0.0, 1.0], rule);
    }
    wrap
}

/// A Schedule's, Birthdays' or Money's rows at a text size, in the face and weights of a style.
fn styles_for(style: NoticeStyle, table: &Table, c: &Ctx, size: f32) -> Vec<TextStyle> {
    (0..table.right.len())
        .map(|at| match style {
            NoticeStyle::Hairline => style_of(c.sans, size, if at == table.key { 800.0 } else { 400.0 }),
            NoticeStyle::Ledger => style_of(c.serif, size, if at == table.key || table.right[at] { 600.0 } else { 500.0 }),
            _ => style_of(c.sans, size, 800.0),
        })
        .collect()
}

fn lines_of(slide: &Slide) -> Vec<String> {
    notice_body(&slide.text).rows.into_iter().map(|cells| cells.join(" ")).collect()
}

/// The account's lines: the bank, the number, the holder and the note, which may be empty.
fn account_parts(body: &NoticeBody) -> (String, String, String, String) {
    let line = |at: usize| body.rows.get(at).map(|cells| cells.join(" ")).unwrap_or_default();
    (line(0), line(1), line(2), line(3))
}

/// The structure a slide is drawn in: its card's own, else the look's.
fn style_of_slide(slide: &Slide, look: &Look) -> NoticeStyle {
    slide.style.unwrap_or(look.notice_style)
}

/// Whether a slide is drawn by `notice`: a kind with columns, or any announcement whose style is not the plain look.
pub fn draws_notice(slide: &Slide, look: &Look) -> bool {
    slide.notice != NoticeKind::Plain || style_of_slide(slide, look) != NoticeStyle::Hairline
}

/// The whole slide at `w` by `h` points, in its card's notice style, else the look's.
pub fn notice(slide: &Slide, look: &Look, w: f32, h: f32, pics: &Pics) -> El {
    let style = style_of_slide(slide, look);
    if style.is_poster() {
        return crate::warta::draw(slide, style, w, h, pics);
    }
    drawn(slide, look, w, h, style).0
}

/// The boxes of any announcement slide in the 1920x1080 space, whichever of the seven styles draws it
/// and whether the look's own slide does: where each part of its words is, for an editor to pick and
/// type in. `typing` is a key being typed (see `warta::layout_typing`).
pub fn layout(slide: &Slide, look: &Look, pics: &Pics, typing: bool) -> Layout {
    let style = style_of_slide(slide, look);
    if style.is_poster() {
        return if typing { crate::warta::layout_typing(slide, style, pics) } else { crate::warta::layout(slide, style, pics) };
    }
    if !draws_notice(slide, look) {
        return Layout { boxes: crate::audience::plain_boxes(slide, look, 1920.0, 1080.0, pics.photo.is_some() && !slide.image.is_empty()), ..Layout::default() };
    }
    Layout { boxes: drawn(slide, look, 1920.0, 1080.0, style).1, ..Layout::default() }
}

/// A slide in Hairline, Ledger or Band, and the boxes it put its words in.
fn drawn(slide: &Slide, look: &Look, w: f32, h: f32, style: NoticeStyle) -> (El, Vec<TextBox>) {
    let mut body = notice_body(&slide.text);
    if slide.notice == NoticeKind::Plain || slide.notice == NoticeKind::Account {
        // These kinds have no columns: a tab in them would only clip a word.
        body.rows = lines_of(slide).into_iter().map(|line| vec![line]).collect();
    }
    let sans = fonts::family(&look.font);
    let serif = if look.font.trim().is_empty() { fonts::family("Cambria") } else { sans };
    let light = luminance(look.ink) < 0.4;
    // On a light ground the accent is darkened until it reads as text.
    let accent = if light && style == NoticeStyle::Hairline { rgb(accent_rgb(style).map(|v| (f32::from(v) * 0.55) as u8)) } else { accent(style) };
    let ground = match look.background {
        crate::look::Background::Solid(colour) => colour,
        crate::look::Background::Gradient(from, _) => from,
        _ => [246, 242, 232],
    };
    let c = Ctx { w, h, u: h / 1080.0, sans, serif, accent, light, ink: look.ink, ground, title: slide.title_colour.map(rgb), boxes: Default::default() };
    let layer = el().key("notice").absolute(0.0, 0.0).size(w, h);
    let drawn = match style {
        NoticeStyle::Hairline => hairline(layer, slide, &body, &c),
        NoticeStyle::Ledger => ledger(layer, slide, &body, &c),
        _ => band(layer, slide, &body, &c),
    };
    (drawn, c.boxes.take())
}

fn hairline(layer: El, slide: &Slide, body: &NoticeBody, c: &Ctx) -> El {
    let (w, h, u) = (c.w, c.h, c.u);
    let mx = (w * 0.05).round();
    let inner = w - 2.0 * mx;
    let label = (0.026 * h).round();
    // A light look keeps its light ground under a light scrim and writes in its own ink.
    let (fg, scrim) = if c.light { (rgb(c.ink), rgb(c.ground).alpha(0.7)) } else { (Color::WHITE, Color::BLACK.alpha(0.62)) };
    let mut layer = layer.child(el().key("scrim").absolute(0.0, 0.0).size(w, h).fill(scrim).a11y_hidden());
    if !repeats(kind_name(slide.notice), &slide.heading) {
        layer = layer.child(text(kind_name(slide.notice)).font(TextStyle::new(label, 600.0).family(c.sans).tracking(label * 0.18)).color(fg.alpha(0.6)).absolute(mx, (0.04 * h).round()));
    }
    layer = layer
        .child(el().key("rule").absolute(mx, (0.082 * h).round()).size(inner, (1.5 * u).max(1.0)).fill(fg.alpha(0.3)).a11y_hidden())
        .child(text(slide.heading.as_str()).font(title_style(c.sans, &slide.heading, 0.09 * h, 800.0, inner)).color(c.title.unwrap_or(c.accent)).absolute(mx, (0.098 * h).round()));
    let (top, bottom) = ((0.33 * h).round(), (0.95 * h).round());
    let heading = title_style(c.sans, &slide.heading, 0.09 * h, 800.0, inner);
    let (ground_rgb, ink_rgb) = if c.light { (c.ground, c.ink) } else { (DARK, WHITE) };
    c.record(Field::Title, (mx, (0.098 * h).round(), width_of(&slide.heading, heading).min(inner), heading.line_height), heading, 1, ground_rgb, slide.title_colour.unwrap_or(if c.light { c.ink } else { title_default(NoticeStyle::Hairline) }));
    if slide.notice == NoticeKind::Account {
        let (bank, number, holder, note) = account_parts(body);
        let big = title_style(c.sans, &number, 0.17 * h, 800.0, inner);
        let at = (0.06 * h).round();
        let tall = (at * 1.25).round() + big.line_height + (0.05 * h * 1.25).round() + if note.is_empty() { 0.0 } else { (0.04 * h * 1.25).round() } + (0.03 * h).round() * if note.is_empty() { 2.0 } else { 3.0 };
        c.record(Field::Account, (mx, top, inner, tall), style_of(c.sans, at, 700.0), 3, ground_rgb, ink_rgb);
        let lines = col()
            .key("account")
            .absolute(mx, top)
            .w(inner)
            .gap((0.03 * h).round())
            .child(text(bank).font(style_of(c.sans, at, 700.0)).color(fg))
            .child(text(number).font(big.features("'tnum' 1")).color(fg))
            .child(text(holder).font(style_of(c.sans, (0.05 * h).round(), 400.0)).color(fg.alpha(0.78)))
            .children((!note.is_empty()).then(|| text(note).font(style_of(c.sans, (0.04 * h).round(), 400.0)).color(fg.alpha(0.6))));
        return layer.child(lines);
    }
    let mut table = Table::of(slide.notice, &body.rows, &slide.marked);
    let blocks = table.blocks.len() as f32;
    // Side by side, the columns of a block sit closer, so three blocks fit one slide.
    let gap_k = if blocks > 1.0 { 0.5 } else { 1.5 };
    // A row that stands out is filled, so its words keep a margin inside the fill, taken from the gaps between columns.
    let pad_k = if table.has_marks() { 0.1 } else { 0.0 };
    let (row_k, total_k, block_gap) = (1.35, 1.25, (0.02 * w).round());
    let natural_at = |table: &Table, size: f32| Grid::new(table, styles_for(NoticeStyle::Hairline, table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k).natural(block_gap);
    if natural_at(&table, 0.03 * h) > inner {
        table = table.shortened(slide.notice);
    }
    let total_tall = |size: f32| if body.total.is_some() { row_k * total_k * size + 0.03 * h } else { 0.0 };
    let total_wide = |size: f32| -> f32 {
        let style = style_of(c.sans, size * total_k, 800.0);
        body.total.as_ref().map_or(0.0, |cells| cells.iter().map(|cell| width_of(cell, style)).sum::<f32>() + size * 2.5)
    };
    let size = fit_size(
        |size| (Grid::new(&table, styles_for(NoticeStyle::Hairline, &table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k).natural(block_gap).max(total_wide(size)), table.rows_tall() as f32 * row_k * size + total_tall(size)),
        (inner, bottom - top),
        0.075 * h,
        0.03 * h,
    );
    let grid = Grid::new(&table, styles_for(NoticeStyle::Hairline, &table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k);
    let colors: Vec<Color> = (0..table.right.len()).map(|at| if at == table.key { fg } else { fg.alpha(0.85) }).collect();
    let too_wide = grid.natural(block_gap) > inner;
    let spread = (blocks > 1.0 || too_wide).then_some(inner);
    let table_w = if blocks > 1.0 || too_wide { inner } else { grid.natural(0.0).max(total_wide(size)).ceil() };
    c.record(body_field(slide.notice), (mx, top, table_w, table.rows_tall() as f32 * row_k * size + total_tall(size)), styles_for(NoticeStyle::Hairline, &table, c, size)[table.key], body.rows.len(), ground_rgb, ink_rgb);
    let mut lines = col().key("rows").absolute(mx, top).w(table_w).gap((0.02 * h).round());
    lines = lines.child(grid.build(&Paint { colors, row_h: row_k * size, row_line: None, col_line: None, marked: (c.accent, if c.light { Color::WHITE } else { rgb([18, 24, 18]) }), spread, block_gap }));
    if let Some(total) = &body.total {
        let style = style_of(c.sans, size * total_k, 800.0);
        let value = total.get(1).cloned().unwrap_or_default();
        let value_w = width_of(&value, style);
        lines = lines.child(
            row()
                .key("total")
                .w(table_w)
                .h((row_k * total_k * size).round())
                .items_center()
                .border_sides([(1.5 * u).max(1.0), 0.0, 0.0, 0.0], fg.alpha(0.45))
                .child(text(total.first().cloned().unwrap_or_default()).font(style).color(fg).w(table_w - value_w).truncate())
                .child(text(value).font(style).color(c.accent).w(value_w).align(Align::End)),
        );
    }
    layer = layer.child(lines);
    layer
}

fn ledger(layer: El, slide: &Slide, body: &NoticeBody, c: &Ctx) -> El {
    let (w, h, u) = (c.w, c.h, c.u);
    let tab_w = (w * 0.07).round();
    let (cx0, cw) = ((w * 0.11).round(), (w * 0.85).round());
    let heading = title_style(c.sans, &slide.heading, 0.07 * h, 700.0, cw);
    c.record(Field::Title, (cx0, (0.05 * h).round(), cw, heading.line_height), heading, 1, DARK, slide.title_colour.unwrap_or(title_default(NoticeStyle::Ledger)));
    // A ledger's title is centred over its panel.
    if let Some(title) = c.boxes.borrow_mut().last_mut() {
        title.align = artosia_core::deck::Align::Centre;
    }
    let ink = rgb([28, 28, 34]);
    let line = rgb([70, 72, 84]);
    let tint = rgb([224, 230, 246]);
    let paper = rgb([250, 250, 246]);
    let hair = (1.5 * u).max(1.0);
    // The kind's name runs up the tab: a box as long as the slide is tall, turned a quarter.
    let (length, thick) = ((0.7 * h).round(), (0.06 * h).round());
    let name = (0.036 * h).round();
    let mut layer = layer.child(el().key("tab").absolute(0.0, 0.0).size(tab_w, h).fill(c.accent).a11y_hidden()).child(
        col()
            .key("tab-name")
            .absolute((tab_w / 2.0 - length / 2.0).round(), ((h - thick) / 2.0).round())
            .size(length, thick)
            .center()
            .rotate(-std::f32::consts::FRAC_PI_2)
            .child(text(kind_name(slide.notice)).font(TextStyle::new(name, 800.0).family(c.sans).tracking(name * 0.2)).color(Color::WHITE).w(length).align(Align::Center)),
    );
    layer = layer.child(
        text(slide.heading.as_str())
            .font(title_style(c.sans, &slide.heading, 0.07 * h, 700.0, cw))
            .color(c.title.unwrap_or(Color::WHITE))
            .absolute(cx0, (0.05 * h).round())
            .w(cw)
            .align(Align::Center),
    );
    let top = (0.19 * h).round();
    let heading_row = |content: &str, size: f32, height: f32, fill: Color, width: f32| -> El {
        row()
            .key("panel-heading")
            .w(width)
            .h(height)
            .fill(fill)
            .items_center()
            .justify_center()
            .border_sides([0.0, 0.0, hair, 0.0], line)
            .child(text(content).font(TextStyle::new(size, 800.0).family(c.sans).tracking(size * 0.14)).color(ink))
    };
    if slide.notice == NoticeKind::Account {
        let (bank, number, holder, note) = account_parts(body);
        let panel_w = (cw * 0.9).round();
        let big = title_style(c.serif, &number, 0.16 * h, 700.0, panel_w * 0.92).features("'tnum' 1");
        let (bank_h, number_h, holder_h) = ((0.12 * h).round(), (0.30 * h).round(), (0.12 * h).round());
        let tall = bank_h + number_h + holder_h;
        let y = top + ((0.955 * h - top - tall) / 2.0).max(0.0).round();
        c.record(Field::Account, ((cx0 + (cw - panel_w) / 2.0).round(), y, panel_w, tall), big, 3, PAPER, [28, 28, 34]);
        let panel = col()
            .key("panel")
            .absolute((cx0 + (cw - panel_w) / 2.0).round(), y)
            .w(panel_w)
            .fill(paper)
            .border(hair, line)
            .child(heading_row(&bank, (0.05 * h).round(), bank_h, tint, panel_w))
            .child(row().w(panel_w).h(number_h).items_center().justify_center().child(text(number).font(big).color(ink)))
            .child(row().w(panel_w).h(holder_h).fill(tint).items_center().justify_center().border_sides([hair, 0.0, 0.0, 0.0], line).child(text(holder).font(style_of(c.serif, (0.05 * h).round(), 600.0)).color(ink)));
        layer = layer.child(panel);
        if !note.is_empty() {
            layer = layer.child(text(note).font(style_of(c.sans, (0.04 * h).round(), 500.0)).color(Color::WHITE.alpha(0.85)).absolute(cx0, y + tall + (0.03 * h).round()).w(cw).align(Align::Center).truncate());
        }
        return layer;
    }
    let mut table = Table::of(slide.notice, &body.rows, &slide.marked);
    let total = body.total.is_some();
    let pad_k = if table.blocks.len() > 1 { 0.25 } else { 0.4 };
    let limit = if total { (0.82 * h).round() } else { (0.955 * h).round() };
    let panel_w = if total { cw * 0.86 } else if table.blocks.len() > 1 { cw } else { cw * 0.9 }.round();
    let row_k = 1.55;
    if Grid::new(&table, styles_for(NoticeStyle::Ledger, &table, c, 0.03 * h), 0.0, 0.03 * h * pad_k).natural(0.0) > panel_w {
        table = table.shortened(slide.notice);
    }
    let headed = !repeats(kind_name(slide.notice), &slide.heading);
    let size = fit_size(
        |size| (Grid::new(&table, styles_for(NoticeStyle::Ledger, &table, c, size), 0.0, size * pad_k).natural(0.0), (table.rows_tall() + usize::from(headed)) as f32 * row_k * size),
        (panel_w, limit - top),
        0.06 * h,
        0.03 * h,
    );
    let row_h = (row_k * size).round();
    let grid = Grid::new(&table, styles_for(NoticeStyle::Ledger, &table, c, size), 0.0, size * pad_k);
    let colors = vec![ink; table.right.len()];
    let tall = (table.rows_tall() + usize::from(headed)) as f32 * row_h;
    let y = top + ((limit - top - tall) / 2.0).max(0.0).round();
    c.record(body_field(slide.notice), ((cx0 + (cw - panel_w) / 2.0).round(), y, panel_w, tall), styles_for(NoticeStyle::Ledger, &table, c, size)[table.key], body.rows.len(), PAPER, [28, 28, 34]);
    let panel = col()
        .key("panel")
        .absolute((cx0 + (cw - panel_w) / 2.0).round(), y)
        .w(panel_w)
        .fill(paper)
        .border(hair, line)
        .children(headed.then(|| heading_row(kind_name(slide.notice), size * 0.85, row_h, tint, panel_w)))
        .child(grid.build(&Paint { colors, row_h, row_line: Some(line), col_line: Some(line), marked: (c.accent, Color::WHITE), spread: Some(panel_w), block_gap: 0.0 }));
    layer = layer.child(panel);
    if let Some(cells) = &body.total {
        // Apart from the table, wider than it, and heavier and bigger than its rows.
        let (bar_h, bar_y) = ((0.11 * h).round(), (0.85 * h).round());
        let style = style_of(c.sans, (size * 1.3).min(0.07 * h), 900.0);
        let pad = (0.03 * w).round();
        layer = layer.child(
            row()
                .key("total-bar")
                .absolute(cx0, bar_y)
                .size(cw, bar_h)
                .fill(Color::WHITE)
                .items_center()
                .child(text(cells.first().cloned().unwrap_or_default().to_uppercase()).font(style).color(ink).w(cw / 2.0 - pad).truncate().ml(pad))
                .child(text(cells.get(1).cloned().unwrap_or_default()).font(style).color(ink).w(cw / 2.0 - pad).align(Align::End)),
        );
    }
    layer
}

fn band(layer: El, slide: &Slide, body: &NoticeBody, c: &Ctx) -> El {
    let (w, h, u) = (c.w, c.h, c.u);
    let mx = (w * 0.05).round();
    let inner = w - 2.0 * mx;
    let band_h = (0.27 * h).round();
    let notch = (0.06 * h).round();
    let ink = Color::WHITE;
    let title = title_style(c.sans, &slide.heading, 0.10 * h, 800.0, inner);
    let second = (0.045 * h).round();
    let band_text = title.line_height + (0.012 * h).round() + (second * 1.25).round();
    c.record(Field::Title, (mx, ((band_h - band_text) / 2.0).max(0.0), width_of(&slide.heading, title).min(inner), title.line_height), title, 1, accent_rgb(NoticeStyle::Band), slide.title_colour.unwrap_or(WHITE));
    let mut layer = layer
        .child(el().key("ground").absolute(0.0, 0.0).size(w, h).fill(rgb([32, 32, 34])).a11y_hidden())
        // A square turned a quarter, half under the band, is the notch that points down from it.
        .child(el().key("notch").absolute((mx + 0.03 * w - notch / 2.0).round(), (band_h - notch / 2.0).round()).size(notch, notch).fill(c.accent).rotate(std::f32::consts::FRAC_PI_4).a11y_hidden())
        .child(el().key("band").absolute(0.0, 0.0).size(w, band_h).fill(c.accent).a11y_hidden())
        .child(
            col()
                .key("band-title")
                .absolute(mx, 0.0)
                .size(inner, band_h)
                .justify_center()
                .gap((0.012 * h).round())
                .child(text(slide.heading.as_str()).font(title).color(c.title.unwrap_or(ink)))
                .children((!repeats(kind_name(slide.notice), &slide.heading)).then(|| text(kind_name(slide.notice)).font(TextStyle::new(second, 600.0).family(c.sans).tracking(second * 0.16)).color(ink.alpha(0.85)))),
        );
    let top = (band_h + 0.06 * h).round();
    let foot = (0.965 * h).round();
    let rule = Color::WHITE.alpha(0.14);
    if slide.notice == NoticeKind::Account {
        let (bank, number, holder, note) = account_parts(body);
        let big = title_style(c.sans, &number, 0.15 * h, 800.0, inner - 0.08 * h).features("'tnum' 1");
        let pill_h = (big.size * 1.45).round();
        let tall = (0.07 * h * 1.25).round() + pill_h + (0.05 * h * 1.25).round() + if note.is_empty() { 0.0 } else { (0.04 * h * 1.25).round() } + (0.03 * h).round() * if note.is_empty() { 2.0 } else { 3.0 };
        c.record(Field::Account, (mx, top, inner, tall), big, 3, CHARCOAL, WHITE);
        let lines = col()
            .key("account")
            .absolute(mx, top)
            .w(inner)
            .gap((0.03 * h).round())
            .items_start()
            .child(text(bank).font(style_of(c.sans, (0.07 * h).round(), 800.0)).color(ink))
            .child(row().h(pill_h).items_center().px((0.035 * h).round()).radius((pill_h / 2.0).round()).border((4.0 * u).max(1.0), c.accent).child(text(number).font(big).color(ink)))
            .child(text(holder).font(style_of(c.sans, (0.05 * h).round(), 600.0)).color(ink.alpha(0.8)))
            .children((!note.is_empty()).then(|| text(note).font(style_of(c.sans, (0.04 * h).round(), 400.0)).color(ink.alpha(0.6))));
        return layer.child(lines);
    }
    let mut table = Table::of(slide.notice, &body.rows, &slide.marked);
    let blocks = table.blocks.len() as f32;
    // Side by side, the columns of a block sit closer, so three blocks fit one slide.
    let gap_k = if blocks > 1.0 { 0.5 } else { 1.5 };
    // A row that stands out is filled, so its words keep a margin inside the fill, taken from the gaps between columns.
    let pad_k = if table.has_marks() { 0.1 } else { 0.0 };
    let (row_k, block_gap) = (1.4, (0.02 * w).round());
    let natural_at = |table: &Table, size: f32| Grid::new(table, styles_for(NoticeStyle::Band, table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k).natural(block_gap);
    if natural_at(&table, 0.03 * h) > inner {
        table = table.shortened(slide.notice);
    }
    let pill_k = 1.9;
    let pill_style = |size: f32| style_of(c.sans, (size * 1.1).min(0.06 * h), 800.0);
    let pill_h = |size: f32| if body.total.is_some() { (pill_style(size).size * pill_k).round() + 0.03 * h } else { 0.0 };
    let pill_wide = |size: f32| -> f32 {
        let style = pill_style(size);
        body.total.as_ref().map_or(0.0, |cells| cells.iter().map(|cell| width_of(cell, style)).sum::<f32>() + style.size * 3.5)
    };
    let size = fit_size(
        |size| (Grid::new(&table, styles_for(NoticeStyle::Band, &table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k).natural(block_gap).max(pill_wide(size)), table.rows_tall() as f32 * row_k * size + pill_h(size)),
        (inner, foot - top),
        0.075 * h,
        0.03 * h,
    );
    let grid = Grid::new(&table, styles_for(NoticeStyle::Band, &table, c, size), size * f32::max(gap_k - 3.0 * pad_k, 0.0), size * pad_k);
    let too_wide = grid.natural(block_gap) > inner;
    let spread = (blocks > 1.0 || too_wide).then_some(inner);
    let table_w = if blocks > 1.0 || too_wide { inner } else { grid.natural(0.0).max(pill_wide(size)).ceil() };
    c.record(body_field(slide.notice), (mx, top, table_w, table.rows_tall() as f32 * row_k * size + pill_h(size)), styles_for(NoticeStyle::Band, &table, c, size)[table.key], body.rows.len(), CHARCOAL, WHITE);
    let colors = vec![ink; table.right.len()];
    let mut lines = col().key("rows").absolute(mx, top).w(table_w);
    lines = lines.child(grid.build(&Paint { colors, row_h: (row_k * size).round(), row_line: Some(rule), col_line: None, marked: (c.accent, rgb([16, 22, 24])), spread, block_gap }));
    layer = layer.child(lines);
    if let Some(cells) = &body.total {
        let style = pill_style(size);
        let height = (style.size * pill_k).round();
        let value = cells.get(1).cloned().unwrap_or_default();
        let value_w = width_of(&value, style);
        let pad = (height / 2.0).round();
        layer = layer.child(
            row()
                .key("total-pill")
                .absolute(mx, foot - height)
                .size(table_w, height)
                .radius((height / 2.0).round())
                .border((4.0 * u).max(1.0), c.accent)
                .items_center()
                .child(text(cells.first().cloned().unwrap_or_default()).font(style).color(ink).w(table_w - value_w - 2.0 * pad).truncate().ml(pad))
                .child(text(value).font(style).color(c.accent).w(value_w).align(Align::End)),
        );
    }
    layer
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn rows(count: usize) -> Vec<Vec<String>> {
        (0..count).map(|at| vec![format!("Name {at}"), "1 November".to_string()]).collect()
    }

    #[test]
    fn birthdays_take_one_two_or_three_columns_and_the_date_orders_them_unnumbered() {
        for (count, columns) in [(6, 1), (10, 1), (11, 2), (20, 2), (21, 3), (30, 3)] {
            let table = Table::of(NoticeKind::Birthdays, &rows(count), &[]);
            assert_eq!(table.blocks.len(), columns, "{count} names");
            assert_eq!(table.blocks.iter().map(Vec::len).sum::<usize>(), count);
        }
        let table = Table::of(NoticeKind::Birthdays, &rows(30), &[]);
        assert_eq!(table.blocks[2][0], rows(30)[20], "the name comes first, with no number");
        assert_eq!(table.right, &[false, true]);
    }

    #[test]
    fn a_name_too_long_for_its_slide_takes_initials_and_a_light_look_keeps_its_ground() {
        let long = vec![vec!["Theresia Margaretha Kusumawardhani Simanjuntak".to_string(), "1 November".to_string()]];
        let table = Table::of(NoticeKind::Birthdays, &long, &[]).shortened(NoticeKind::Birthdays);
        assert_eq!(table.blocks[0][0][0], "Theresia M. K. Simanjuntak");
        assert!(repeats("ULANG TAHUN", "Ulang Tahun"));
        assert!(luminance([28, 26, 24]) < 0.4 && luminance([255, 255, 255]) > 0.4);
    }

    #[test]
    fn marked_rows_are_found_by_their_place_in_each_column_of_names() {
        let table = Table::of(NoticeKind::Birthdays, &rows(30), &[0, 10, 29]);
        assert!(table.has_marks());
        assert_eq!([table.is_marked(0, 0), table.is_marked(1, 0), table.is_marked(2, 9), table.is_marked(2, 8)], [true, true, true, false]);
        assert!(!Table::of(NoticeKind::Birthdays, &rows(30), &[]).has_marks());
        assert!(!Table::of(NoticeKind::Schedule, &rows(3), &[1]).has_marks());
    }

    #[test]
    fn every_style_gives_a_title_box_over_a_box_for_its_words_whichever_slide_draws_it() {
        let slide = |kind: NoticeKind, text: &str, style: Option<NoticeStyle>| Slide { notice: kind, style, heading: "Retret keluarga jemaat".into(), text: text.into(), ..Slide::default() };
        let cases = [
            (NoticeKind::Plain, "Sabtu 12 Desember
Pendaftaran di meja informasi", Field::Body),
            (NoticeKind::Schedule, "Minggu	07.00	Ibadah Raya I
	09.30	Ibadah Raya II", Field::Rows),
            (NoticeKind::Birthdays, "Ibu Susi	3 November
Bpk. Andi	5 November", Field::Names),
            (NoticeKind::Money, "Persepuluhan	Rp 1.500.000
Kas	Rp 500.000
== Jumlah	Rp 2.000.000", Field::Funds),
            (NoticeKind::Account, "Bank Contoh
1234 5678
Gereja Contoh
Berita: nama dan nomor", Field::Account),
        ];
        for style in [None, Some(NoticeStyle::Hairline), Some(NoticeStyle::Ledger), Some(NoticeStyle::Band)] {
            for (kind, text, field) in &cases {
                let found = layout(&slide(*kind, text, style), &Look::default(), &Pics::default(), false);
                let (title, words) = (found.of(Field::Title).expect("a title box"), found.of(*field).expect("a box for the words"));
                assert!(title.y < words.y, "{style:?} {kind:?}: the title is above the words");
                for each in [title, words] {
                    assert!(each.w > 50.0 && each.h > 20.0 && each.x >= 0.0 && each.x + each.w <= 1921.0 && each.y >= 0.0 && each.y + each.h <= 1081.0, "{style:?} {kind:?} {:?}: {each:?}", each.field);
                }
                assert!(found.title.is_none() && found.body.is_none(), "only the poster styles are fitted by the editor");
            }
        }
    }

    #[test]
    fn a_box_says_how_the_slide_aligns_its_lines_so_the_words_typed_in_it_stay_where_they_are() {
        use artosia_core::deck::Align;
        let plain = |style: Option<NoticeStyle>| Slide { notice: NoticeKind::Plain, style, heading: "Selamat datang".into(), text: "Kami senang".into(), ..Slide::default() };
        let aligns = |slide: &Slide, look: &Look| layout(slide, look, &Pics::default(), false).boxes.iter().map(|each| (each.field, each.align)).collect::<Vec<_>>();
        let (centred, left) = (Look { centered: true, ..Look::default() }, Look { centered: false, ..Look::default() });
        assert_eq!(aligns(&plain(None), &centred), [(Field::Title, Align::Centre), (Field::Body, Align::Centre)]);
        assert_eq!(aligns(&plain(None), &left), [(Field::Title, Align::Start), (Field::Body, Align::Start)]);
        assert_eq!(aligns(&plain(Some(NoticeStyle::Ledger)), &left)[0], (Field::Title, Align::Centre));
        assert_eq!(aligns(&plain(Some(NoticeStyle::Band)), &centred)[0], (Field::Title, Align::Start));
        // An editorial card with no picture is centred on its plate; with one it stands left.
        assert_eq!(aligns(&plain(Some(NoticeStyle::Greige)), &left)[0], (Field::Title, Align::Centre));
        let picture = Slide { image: "photo.png".into(), ..plain(Some(NoticeStyle::Greige)) };
        assert_eq!(aligns(&picture, &left)[0], (Field::Title, Align::Start));
    }

    #[test]
    fn a_card_style_beats_the_look_and_a_plain_card_in_the_old_look_is_not_a_notice() {
        let look = Look::default();
        let mut slide = Slide::default();
        assert!(!draws_notice(&slide, &look));
        slide.style = Some(NoticeStyle::Band);
        assert!(draws_notice(&slide, &look));
        slide.style = Some(NoticeStyle::Hairline);
        slide.notice = NoticeKind::Schedule;
        assert!(draws_notice(&slide, &Look { notice_style: NoticeStyle::Ledger, ..look }));
    }
}
