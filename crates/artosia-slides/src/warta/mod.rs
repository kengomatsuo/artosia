//! The four poster styles of an announcement (Editorial greige, Pill poster, Neon night, Split
//! panel), ported from the designer's layouts in `assets/warta/tokens.json`.
//!
//! A layout is a function that fills a [`Sheet`], a display list in a 1920x1080 space whose every
//! number is explicit, from a slide's data and the style's tokens. [`Sheet::build`] turns the list
//! into Rune elements at the slide's size. A layout that cannot hold its rows at the floor size
//! returns [`Split`], and [`paginate`] then cuts the card into slides that can.

pub mod fit;
mod layout;

pub use layout::{Field, Fit, Layout, Line, TextBox, cut, layout, layout_typing};

mod d1;
mod d2;
mod d3;
mod d4;
mod data;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use artosia_core::deck::{Align, Face, NoticeKind, NoticeStyle, Slide};
use rune::{Color, El, Family, Pixels, SurfaceFit, TextStyle, el, text};
use serde_json::Value;

use crate::fonts;
use crate::look::rgb;
use data::Data;

/// The layout could not hold its rows at the smallest size a projector can read.
#[derive(Debug)]
pub struct Split;

pub type Fits = Result<(), Split>;

/// The width and height of the space every layout is drawn in.
pub const W: f32 = 1920.0;
pub const H: f32 = 1080.0;

/// The pictures a slide may show beyond its look's background.
#[derive(Clone, Copy, Default)]
pub struct Pics<'a> {
    pub photo: Option<&'a Pixels>,
    /// The card's second picture; the first stands in for it when there is none.
    pub photo2: Option<&'a Pixels>,
    pub qris: Option<&'a Pixels>,
}

fn tokens() -> &'static Value {
    static ALL: OnceLock<Value> = OnceLock::new();
    ALL.get_or_init(|| serde_json::from_str(include_str!("../../assets/warta/tokens.json")).unwrap_or(Value::Null))
}

fn dir_of(style: NoticeStyle) -> &'static Value {
    let key = match style {
        NoticeStyle::Greige => "d1",
        NoticeStyle::Pill => "d2",
        NoticeStyle::Neon => "d3",
        _ => "d4",
    };
    &tokens()["directions"][key]
}

#[derive(Clone, Copy)]
struct Metrics {
    cap: f32,
    asc: f32,
    desc: f32,
}

/// A face's cap height, ascent and descent per em, read once from its file.
fn metrics(file: &str) -> Metrics {
    static KEPT: Mutex<Option<HashMap<String, Metrics>>> = Mutex::new(None);
    let mut kept = KEPT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(found) = kept.get_or_insert_with(HashMap::new).get(file) {
        return *found;
    }
    use read_fonts::{FontRef, TableProvider};
    let read = || -> Option<Metrics> {
        let font = FontRef::new(fonts::face_bytes(file)?).ok()?;
        let em = f32::from(font.head().ok()?.units_per_em());
        let hhea = font.hhea().ok()?;
        let cap = font.os2().ok().and_then(|os2| os2.s_cap_height()).map_or(0.7, |cap| f32::from(cap) / em);
        Some(Metrics { cap, asc: f32::from(hhea.ascender().to_i16()) / em, desc: -f32::from(hhea.descender().to_i16()) / em })
    };
    let found = read().unwrap_or(Metrics { cap: 0.7, asc: 0.95, desc: 0.25 });
    kept.get_or_insert_with(HashMap::new).insert(file.to_string(), found);
    found
}

fn interned(name: &str) -> &'static str {
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(kept) = names.iter().find(|kept| **kept == name) {
        return kept;
    }
    let kept: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.push(kept);
    kept
}

fn number(v: &Value) -> f32 {
    v.as_f64().unwrap_or(0.0) as f32
}

fn hex(code: &str) -> Color {
    let byte = |at: usize| u8::from_str_radix(code.get(at..at + 2).unwrap_or("00"), 16).unwrap_or(0);
    rgb([byte(1), byte(3), byte(5)])
}

#[derive(Clone, Debug)]
pub struct R {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    fill: Option<String>,
    rx: f32,
    op: f32,
    bw: f32,
    bc: String,
    bop: f32,
    rot: f32,
}

impl R {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h, fill: None, rx: 0.0, op: 1.0, bw: 0.0, bc: String::new(), bop: 1.0, rot: 0.0 }
    }

    pub fn fill(mut self, colour: &str) -> R {
        self.fill = Some(colour.to_string());
        self
    }

    pub fn rx(mut self, radius: f32) -> R {
        self.rx = radius;
        self
    }

    /// A pill: as round as it is tall.
    pub fn pill(self) -> R {
        let radius = self.h / 2.0;
        self.rx(radius)
    }

    pub fn op(mut self, opacity: f32) -> R {
        self.op = opacity;
        self
    }

    pub fn border(mut self, width: f32, colour: &str) -> R {
        self.bw = width;
        self.bc = colour.to_string();
        self
    }

    pub fn bop(mut self, opacity: f32) -> R {
        self.bop = opacity;
        self
    }

    /// Turned by `deg` degrees about its centre.
    pub fn rot(mut self, deg: f32) -> R {
        self.rot = deg;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pic {
    Photo,
    Photo2,
    Qris,
}

/// Where a covering picture sits along the side it overflows, as the designs' `preserveAspectRatio` says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pos {
    Min,
    Mid,
    Max,
}

/// A picture `iw` by `ih` covering a `w` by `h` box, placed at `pos`: its left, top, width and height
/// relative to the box.
pub fn cover_at(iw: f32, ih: f32, w: f32, h: f32, pos: Pos) -> (f32, f32, f32, f32) {
    let scale = (w / iw).max(h / ih);
    let (dw, dh) = (iw * scale, ih * scale);
    let along = |room: f32| match pos {
        Pos::Min => 0.0,
        Pos::Mid => room / 2.0,
        Pos::Max => room,
    };
    (-along(dw - w), -along(dh - h), dw, dh)
}

#[derive(Clone, Debug)]
struct Im {
    pic: Pic,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rx: f32,
    contain: bool,
    pos: Pos,
}

#[derive(Clone, Debug)]
struct Tt {
    x: f32,
    y: f32,
    s: String,
    family: &'static str,
    weight: f32,
    italic: bool,
    file: String,
    size: f32,
    ls: f32,
    fill: String,
    op: f32,
    rot: f32,
    role: String,
    /// The advance of the text as set, and where it was anchored in its column.
    w: f32,
    at: At,
    maxw: Option<f32>,
}

#[derive(Clone, Debug)]
enum Item {
    Rect(R),
    Image(Im),
    Text(Tt),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum At {
    Left,
    Right,
    Centre,
}

/// How a text is placed and styled over its type style.
#[derive(Clone)]
pub struct Tx {
    x: f32,
    y: f32,
    at: At,
    cy: bool,
    size: Option<f32>,
    fill: Option<String>,
    op: f32,
    font: Option<String>,
    rot: f32,
    maxw: Option<f32>,
}

impl Tx {
    pub fn l(x: f32, y: f32) -> Tx {
        Tx { x, y, at: At::Left, cy: false, size: None, fill: None, op: 1.0, font: None, rot: 0.0, maxw: None }
    }

    pub fn r(x: f32, y: f32) -> Tx {
        Tx { at: At::Right, ..Tx::l(x, y) }
    }

    pub fn c(x: f32, y: f32) -> Tx {
        Tx { at: At::Centre, ..Tx::l(x, y) }
    }

    /// `y` is the middle of the capitals, not the baseline.
    pub fn cy(mut self) -> Tx {
        self.cy = true;
        self
    }

    pub fn size(mut self, size: f32) -> Tx {
        self.size = Some(size);
        self
    }

    pub fn fill(mut self, colour: &str) -> Tx {
        self.fill = Some(colour.to_string());
        self
    }

    pub fn op(mut self, opacity: f32) -> Tx {
        self.op = opacity;
        self
    }

    pub fn font(mut self, key: &str) -> Tx {
        self.font = Some(key.to_string());
        self
    }

    pub fn rot(mut self, deg: f32) -> Tx {
        self.rot = deg;
        self
    }

    pub fn maxw(mut self, width: f32) -> Tx {
        self.maxw = Some(width);
        self
    }
}

/// Where a text was put: its left edge and width.
#[derive(Clone, Copy)]
pub struct Tm {
    pub x: f32,
    pub w: f32,
}

/// A style's type role resolved.
struct Ty {
    font: String,
    size: f32,
    ls: f32,
    caps: bool,
    color: String,
}

pub struct Sheet {
    d: &'static Value,
    items: Vec<Item>,
    /// A body's size and a title's ceiling the search of [`sheet`] is trying.
    body_px: Option<f32>,
    title_cap: Option<f32>,
    /// Draw at the floors whatever does not fit, so a card nothing can hold is still seen.
    force: bool,
    /// The type roles a card's bold or italic applies to: role, bold, italic.
    styled: Vec<(String, bool, bool)>,
}

thread_local! {
    /// A key is being typed: a slide that does not hold is drawn as it is and not cut into slides first.
    static TYPING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `build` with slides drawn for a key being typed when `typing`, as `layout_typing` works them out.
pub fn while_typing<R>(typing: bool, build: impl FnOnce() -> R) -> R {
    let before = TYPING.with(|flag| flag.replace(typing));
    let built = build();
    TYPING.with(|flag| flag.set(before));
    built
}

thread_local! {
    /// Advances already measured, by face, size and words: a balanced wrap, a size search and every
    /// layout of the same words ask for the same ones again and again.
    static ADVANCES: std::cell::RefCell<Advances> = std::cell::RefCell::new(HashMap::new());
}

/// Advances kept by face (family, weight, slant), size and words.
type Advances = HashMap<(&'static str, u32, bool, u32, String), f32>;

/// The size a body is never set below, and the least a person at the back can read.
pub const BODY_FLOOR: f32 = 40.0;

impl Sheet {
    fn new(style: NoticeStyle) -> Sheet {
        Sheet { d: dir_of(style), items: Vec::new(), body_px: None, title_cap: None, force: false, styled: Vec::new() }
    }

    /// `found`, or the floor when a drawing is forced; the layout cannot hold it otherwise.
    pub fn must<T>(&self, found: Option<T>, floor: T) -> Result<T, Split> {
        found.or(self.force.then_some(floor)).ok_or(Split)
    }

    /// Ok while `holds`, or while a drawing is forced.
    pub fn need(&self, holds: bool) -> Fits {
        if holds || self.force { Ok(()) } else { Err(Split) }
    }

    /// The font of `key` in the weight and slant a card asked for: the same family's nearest cut that
    /// is heavier or slanted, else the key itself.
    fn restyle(&self, key: &str, bold: bool, italic: bool) -> String {
        let fonts = &self.d["fonts"];
        let Some(own) = fonts.get(key) else { return key.to_string() };
        let (family, weight, slanted) = (own["family"].as_str().unwrap_or(""), number(&own["weight"]), own["italic"].as_bool().unwrap_or(false));
        let mut best: Option<(&str, f32)> = None;
        for (other, spec) in fonts.as_object().into_iter().flatten() {
            let (w, s) = (number(&spec["weight"]), spec["italic"].as_bool().unwrap_or(false));
            if spec["family"].as_str() != Some(family) || (italic && !slanted && !s) || (!italic && s != slanted) || (bold && w <= weight) {
                continue;
            }
            let near = (w - weight).abs();
            if best.is_none_or(|(_, kept)| near < kept) {
                best = Some((other, near));
            }
        }
        match best {
            Some((other, _)) => other.to_string(),
            // A family with no slanted cut is slanted by the renderer; the mark says so.
            None if italic && !slanted => format!("{key}~i"),
            None => key.to_string(),
        }
    }

    /// `key` as the card's bold and italic set it for texts of `role`.
    fn styled_key(&self, role: &str, key: &str) -> String {
        match self.styled.iter().find(|(own, ..)| own == role) {
            Some((_, bold, italic)) => self.restyle(key, *bold, *italic),
            None => key.to_string(),
        }
    }

    /// Moves the lines of the texts in `roles` across the column each was set in; Start is the style's own.
    fn align(&mut self, align: Align, roles: &[&str]) {
        if align == Align::Start {
            return;
        }
        for item in &mut self.items {
            let Item::Text(t) = item else { continue };
            let Some(max) = t.maxw.filter(|_| t.rot == 0.0 && roles.contains(&t.role.as_str())) else { continue };
            let (from, to) = match t.at {
                At::Left => (t.x, t.x + max),
                At::Right => (t.x + t.w - max, t.x + t.w),
                At::Centre => (t.x + t.w / 2.0 - max / 2.0, t.x + t.w / 2.0 + max / 2.0),
            };
            t.x = if align == Align::Centre { from + (to - from - t.w) / 2.0 } else { to - t.w };
        }
    }

    /// Makes the fonts of a type role's family follow a card's bold and italic.
    fn style_roles(&mut self, roles: &[&str], face: Face) {
        if !face.bold && !face.italic {
            return;
        }
        self.styled.extend(roles.iter().map(|role| (role.to_string(), face.bold, face.italic)));
    }

    /// A grid number.
    pub fn g(&self, key: &str) -> f32 {
        number(&self.d["grid"][key])
    }

    /// A number of a slide kind's own section, such as `s01`.
    pub fn s(&self, section: &str, key: &str) -> f32 {
        number(&self.d[section][key])
    }

    /// A reader of one section's numbers that does not borrow the sheet.
    pub fn sec(&self, section: &'static str) -> impl Fn(&str) -> f32 + use<> {
        let (d, cap) = (self.d, self.title_cap);
        move |key: &str| {
            let value = number(&d[section][key]);
            match cap {
                Some(cap) if key == "title_max" => value.min(cap),
                _ => value,
            }
        }
    }

    pub fn list(&self, section: &str, key: &str) -> Vec<Vec<f32>> {
        self.d[section][key].as_array().map(|rows| rows.iter().map(|row| row.as_array().map(|cells| cells.iter().map(number).collect()).unwrap_or_default()).collect()).unwrap_or_default()
    }

    /// A palette key or a hex colour as hex.
    pub fn c(&self, key: &str) -> String {
        if key.starts_with('#') || key.starts_with("grad:") { key.to_string() } else { self.d["palette"][key].as_str().unwrap_or("#FF00FF").to_string() }
    }

    fn ty(&self, role: &str) -> Ty {
        let spec = &self.d["type"][role];
        let size = if matches!(role, "n_body" | "n_body_b") { self.body_px.unwrap_or(number(&spec["size"])) } else { number(&spec["size"]) };
        Ty { font: self.styled_key(role, spec["font"].as_str().unwrap_or("sans")), size, ls: number(&spec["ls"]), caps: spec["caps"].as_bool().unwrap_or(false), color: spec["color"].as_str().unwrap_or("ink").to_string() }
    }

    pub fn ty_size(&self, role: &str) -> f32 {
        self.ty(role).size
    }

    fn font_of(&self, key: &str) -> (&'static str, f32, bool, String) {
        let (key, slant) = key.strip_suffix("~i").map_or((key, false), |own| (own, true));
        let spec = &self.d["fonts"][key];
        let slanted = spec["italic"].as_bool().unwrap_or(false) || slant;
        (interned(spec["family"].as_str().unwrap_or("")), number(&spec["weight"]), slanted, spec["file"].as_str().unwrap_or("").to_string())
    }

    fn rune_style(family: &'static str, weight: f32, italic: bool, size: f32) -> TextStyle {
        let style = TextStyle::new(size, weight).family(Family::Named(family));
        if italic { style.italic() } else { style }
    }

    /// The advance of `s` in a face at `size`, with `ls` added between glyphs.
    pub fn adv(&self, font: &str, s: &str, size: f32, ls: f32) -> f32 {
        let (family, weight, italic, _) = self.font_of(font);
        let key = (family, weight as u32, italic, size.to_bits(), s.to_string());
        let kept = ADVANCES.with(|all| all.borrow().get(&key).copied());
        let width = kept.unwrap_or_else(|| {
            let width = rune::text::advance(s, Sheet::rune_style(family, weight, italic, size));
            ADVANCES.with(|all| {
                let mut all = all.borrow_mut();
                // A person typing makes new words at every key; the oldest are let go when there are too many.
                if all.len() > 60_000 {
                    all.clear();
                }
                all.insert(key, width);
            });
            width
        });
        width + ls * (s.chars().count().max(1) - 1) as f32
    }

    /// The width of `s` in a type role, optionally at another size or face.
    pub fn measure(&self, role: &str, s: &str, size: Option<f32>, font: Option<&str>) -> f32 {
        let ty = self.ty(role);
        let size = size.unwrap_or(ty.size);
        let s = if ty.caps { s.to_uppercase() } else { s.to_string() };
        self.adv(font.unwrap_or(&ty.font), &s, size, ty.ls * size)
    }

    /// The capital height of a type role's face at `size`.
    pub fn cap(&self, font: &str, size: f32) -> f32 {
        metrics(&self.font_of(font).3).cap * size
    }

    /// The face key of a role.
    pub fn face_of(&self, role: &str) -> String {
        self.ty(role).font
    }

    pub fn push(&mut self, rect: R) {
        let r = |v: f32| (v * 100.0).round() / 100.0;
        let (fill, bc) = (rect.fill.as_deref().map(|fill| self.c(fill)), if rect.bc.is_empty() { String::new() } else { self.c(&rect.bc) });
        self.items.push(Item::Rect(R { x: r(rect.x), y: r(rect.y), w: r(rect.w), h: r(rect.h), fill, bc, ..rect }));
    }

    #[allow(clippy::too_many_arguments)] // a picture's place, size, corner and fit, as the layouts name them
    pub fn image(&mut self, pic: Pic, x: f32, y: f32, w: f32, h: f32, rx: f32, contain: bool, pos: Pos) {
        self.items.push(Item::Image(Im { pic, x, y, w, h, rx, contain, pos }));
    }

    /// Puts `s` in the type role `role`. A text wider than `maxw` is set smaller to fit it, to the
    /// label floor of 32 px, which is the least a person at the back can read.
    pub fn text(&mut self, role: &str, s: &str, o: Tx) -> Tm {
        let ty = self.ty(role);
        let mut size = o.size.unwrap_or(ty.size);
        let s = if ty.caps { s.to_uppercase() } else { s.to_string() };
        let font_key = o.font.as_deref().map_or_else(|| ty.font.clone(), |own| self.styled_key(role, own));
        let (family, weight, italic, file) = self.font_of(&font_key);
        let mut w = self.adv(&font_key, &s, size, ty.ls * size);
        if let Some(max) = o.maxw.filter(|max| w > *max + 0.5) {
            size = (size * max / w).floor().max(32.0_f32.min(size));
            w = self.adv(&font_key, &s, size, ty.ls * size);
        }
        let left = match o.at {
            At::Left => o.x,
            At::Right => o.x - w,
            At::Centre => o.x - w / 2.0,
        };
        let cap = metrics(&file).cap * size;
        let base = if o.cy { o.y + cap / 2.0 } else { o.y };
        let fill = self.c(o.fill.as_deref().unwrap_or(&ty.color));
        self.items.push(Item::Text(Tt { x: (left * 100.0).round() / 100.0, y: (base * 100.0).round() / 100.0, s, family, weight, italic, file, size, ls: ty.ls * size, fill, op: o.op, rot: o.rot, role: role.to_string(), w, at: o.at, maxw: o.maxw }));
        Tm { x: left, w }
    }

    /// Text of a role centred on (cx, cy) and turned `deg` about that centre.
    pub fn spin(&mut self, role: &str, s: &str, cx: f32, cy: f32, deg: f32, size: Option<f32>) {
        let ty = self.ty(role);
        let z = size.unwrap_or(ty.size);
        let w = self.measure(role, s, Some(z), None);
        let cap = self.cap(&ty.font, z);
        let (c, sn) = ((deg * std::f32::consts::PI / 180.0).cos(), (deg * std::f32::consts::PI / 180.0).sin());
        let (vx, vy) = (-w / 2.0, cap / 2.0);
        let mut o = Tx::l(cx + vx * c - vy * sn, cy + vx * sn + vy * c).rot(deg);
        o.size = Some(z);
        self.text(role, s, o);
    }

    /// The display list as elements `w` by `h` points, over a ground that fills the screen.
    fn build(&self, w: f32, h: f32, pics: &Pics) -> El {
        let u = h / H;
        let ox = ((w - W * u) / 2.0).max(0.0);
        let mut layer = el().key("warta").absolute(0.0, 0.0).size(w, h);
        let gradient = |name: &str| -> Option<(Color, Color)> {
            let stops = self.d["gradients"][name]["stops"].as_array()?;
            let at = |row: &Value| Some(hex(row[1].as_str()?).alpha(number(&row[2])));
            Some((at(stops.first()?)?, at(stops.last()?)?))
        };
        for (at, item) in self.items.iter().enumerate() {
            match item {
                Item::Rect(r) => {
                    let ground = r.x <= 0.0 && r.y <= 0.0 && r.w >= W && r.h >= H;
                    let (x, y, rw, rh) = if ground { (0.0, 0.0, w, h) } else { (ox + r.x * u, r.y * u, r.w * u, r.h * u) };
                    let mut e = el().key(format!("r{at}")).absolute(x, y).size(rw, rh).a11y_hidden();
                    if let Some(fill) = &r.fill {
                        match fill.strip_prefix("grad:").and_then(gradient) {
                            Some((top, bottom)) => e = e.fill(top).gradient(top, bottom),
                            None => e = e.fill(hex(fill).alpha(r.op)),
                        }
                    }
                    if r.rx > 0.0 {
                        e = e.radius(r.rx * u);
                    }
                    if r.bw > 0.0 {
                        e = e.border((r.bw * u).max(0.5), hex(&r.bc).alpha(r.bop));
                    }
                    if r.rot != 0.0 {
                        e = e.rotate(r.rot * std::f32::consts::PI / 180.0);
                    }
                    layer = layer.child(e);
                }
                Item::Image(im) => {
                    let pixels = match im.pic {
                        Pic::Photo => pics.photo,
                        Pic::Photo2 => pics.photo2.or(pics.photo),
                        Pic::Qris => pics.qris,
                    };
                    let mut e = el().key(format!("i{at}")).absolute(ox + im.x * u, im.y * u).size(im.w * u, im.h * u).a11y_hidden();
                    e = match pixels {
                        Some(pixels) if !im.contain && im.pos != Pos::Mid && pixels.size().0 > 0 => {
                            // A cover picture that sits at an edge is placed by hand inside a clipped box.
                            let (iw, ih) = pixels.size();
                            let (dx, dy, dw, dh) = cover_at(iw as f32, ih as f32, im.w, im.h, im.pos);
                            e.clip_rounded(im.rx * u).child(el().key("pic").absolute(dx * u, dy * u).size(dw * u, dh * u).pixels_fit(pixels, SurfaceFit::Fill))
                        }
                        Some(pixels) => e.pixels_fit(pixels, if im.contain { SurfaceFit::Fit } else { SurfaceFit::Fill }),
                        None => e.fill(Color::WHITE.alpha(0.12)),
                    };
                    if im.rx > 0.0 {
                        e = e.clip_rounded(im.rx * u);
                    }
                    layer = layer.child(e);
                }
                Item::Text(t) => {
                    let size = t.size * u;
                    let m = metrics(&t.file);
                    let lh = (size * 1.3).ceil();
                    let base_off = (lh - (m.asc + m.desc) * size) / 2.0 + m.asc * size;
                    let style = Sheet::rune_style(t.family, t.weight, t.italic, size).lh(lh).tracking(t.ls * u);
                    let mut e = text(t.s.as_str()).font(style).color(hex(&t.fill).alpha(t.op)).absolute(ox + t.x * u, t.y * u - base_off).inert();
                    if t.rot != 0.0 {
                        e = e.origin(0.0, base_off / lh).rotate(t.rot * std::f32::consts::PI / 180.0);
                    }
                    layer = layer.child(e);
                }
            }
        }
        layer
    }
}

/// The largest whole size from `hi` down to `lo` at which `ok` holds, None when none does.
pub fn largest(hi: f32, lo: f32, ok: impl Fn(f32) -> bool) -> Option<f32> {
    largest_step(hi.floor(), lo.floor(), 1.0, ok)
}

/// The largest size from `hi` down to `lo` in steps of `step` at which `ok` holds: a size that holds keeps
/// holding as it gets smaller, so it is found by halving, a few measures where a walk down took dozens.
pub fn largest_step(hi: f32, lo: f32, step: f32, ok: impl Fn(f32) -> bool) -> Option<f32> {
    if hi < lo {
        return None;
    }
    let steps = ((hi - lo) / step).floor() as u32;
    let at = |n: u32| hi - n as f32 * step;
    if ok(at(0)) {
        return Some(at(0));
    }
    if !ok(at(steps)) {
        return None;
    }
    // at(first) does not hold and at(last) does.
    let (mut first, mut last) = (0, steps);
    while last - first > 1 {
        let mid = (first + last) / 2;
        if ok(at(mid)) { last = mid } else { first = mid }
    }
    Some(at(last))
}

/// The largest size at which every name fits `width` in `role`, shortened by the ladder where it must be.
pub fn largest_name_size(sl: &Sheet, names: &[&str], role: &str, width: f32, hi: f32, lo: f32) -> Option<f32> {
    largest(hi, lo, |z| names.iter().all(|name| fit::ladder(name, width, &|s| sl.measure(role, s, Some(z), None)).is_some()))
}

/// A name as it fits `width` at `size` in `role`, with whether it had to be shortened.
pub fn name_in(sl: &Sheet, name: &str, role: &str, width: f32, size: f32) -> (String, bool) {
    match fit::ladder(name, width, &|s| sl.measure(role, s, Some(size), None)) {
        Some(text) => (text.clone(), text != name),
        None => (fit::initials(name), true),
    }
}

/// `text` in balanced lines of at most `width` in a face at `size`.
pub fn paragraph(sl: &Sheet, text: &str, font: &str, size: f32, width: f32) -> Vec<String> {
    let space = sl.adv(font, " ", size, 0.0);
    let word = |s: &str| sl.adv(font, s, size, 0.0);
    fit::balanced(&fit::words(text), width, space, &word, &word).0.iter().map(|line| fit::join(line)).collect()
}

/// Each line of `text` as a paragraph in balanced lines, one after another.
pub fn paragraphs(sl: &Sheet, text: &str, font: &str, size: f32, width: f32) -> Vec<String> {
    text.lines().filter(|line| !line.trim().is_empty()).flat_map(|line| paragraph(sl, line, font, size, width)).collect()
}

/// The height one more line of text costs, as a share of its size.
pub const LINE: f32 = 1.12;

/// Each text on one line, or on two balanced lines when it is wider than `width` at `z`; None when one needs three.
fn cell_lines(sl: &Sheet, role: &str, texts: &[&str], z: f32, width: f32) -> Option<Vec<Vec<String>>> {
    let face = sl.face_of(role);
    texts
        .iter()
        .map(|text| {
            if sl.measure(role, text, Some(z), None) <= width {
                return Some(vec![(*text).to_string()]);
            }
            Some(paragraph(sl, text, &face, z, width)).filter(|lines| lines.len() <= 2 && lines.iter().all(|line| sl.measure(role, line, Some(z), None) <= width))
        })
        .collect()
}

/// The size and lines at which `texts` fill their cells: the largest size where every text is on one line and
/// `rows_fit(size, 0)` holds; failing that, the largest where each is on at most two balanced lines and
/// `rows_fit(size, extra)` holds, `extra` being the height of the second line. None when not even the floor does.
pub fn fit_cells(sl: &Sheet, role: &str, texts: &[&str], width_at: impl Fn(f32) -> f32, hi: f32, lo: f32, rows_fit: impl Fn(f32, f32) -> bool) -> Option<(f32, Vec<Vec<String>>, f32)> {
    let single = largest(hi, lo, |z| rows_fit(z, 0.0) && texts.iter().all(|text| sl.measure(role, text, Some(z), None) <= width_at(z)));
    if let Some(z) = single {
        return Some((z, texts.iter().map(|text| vec![(*text).to_string()]).collect(), 0.0));
    }
    let two = |z: f32| cell_lines(sl, role, texts, z, width_at(z)).filter(|lines| rows_fit(z, if lines.iter().any(|line| line.len() > 1) { LINE * z } else { 0.0 }));
    let z = largest(hi, lo, |z| two(z).is_some())?;
    let lines = two(z)?;
    let extra = if lines.iter().any(|line| line.len() > 1) { LINE * z } else { 0.0 };
    Some((z, lines, extra))
}

/// `lines` of a cell centred on `cy`, one under another.
pub fn put_lines(sl: &mut Sheet, role: &str, lines: &[String], x: f32, cy: f32, z: f32, width: f32) {
    for (at, line) in lines.iter().enumerate() {
        sl.text(role, line, Tx::l(x, cy + (at as f32 - (lines.len() as f32 - 1.0) / 2.0) * LINE * z).cy().size(z).maxw(width));
    }
}

/// Where a verse goes: its left edge, top, width and the height it may take, and the face it is set in.
pub struct VerseBox<'a> {
    pub x: f32,
    pub top: f32,
    pub w: f32,
    pub room: f32,
    pub font: &'a str,
    pub section: &'a str,
}

/// A verse set in the section's size search with its numbers raised in the same line. Returns the size.
pub fn verse(sl: &mut Sheet, verses: &[(u32, String)], b: &VerseBox) -> Result<f32, Split> {
    let (scale, gap, rise, lh) = (sl.s(b.section, "sup_scale"), sl.s(b.section, "sup_gap"), sl.s(b.section, "sup_rise"), sl.s(b.section, "lh"));
    let toks: Vec<fit::Tok> = verses.iter().flat_map(|(n, t)| std::iter::once(fit::Tok::Num(n.to_string())).chain(fit::words(t))).collect();
    let lines_at = |sl: &Sheet, z: f32| {
        let space = sl.adv(b.font, " ", z, 0.0);
        let word = |s: &str| sl.adv(b.font, s, z, 0.0);
        let num = |s: &str| sl.adv(b.font, s, z * scale, 0.0) + gap * z;
        fit::balanced(&toks, b.w, space, &word, &num).0
    };
    let z = largest(sl.s(b.section, "size_max"), sl.s(b.section, "size_floor"), |z| lines_at(sl, z).len() as f32 * lh * z <= b.room).ok_or(Split)?;
    let lines = lines_at(sl, z);
    let cap = sl.cap(b.font, z);
    let space = sl.adv(b.font, " ", z, 0.0);
    for (i, line) in lines.iter().enumerate() {
        let base = b.top + (i as f32 + 0.5) * lh * z + cap / 2.0;
        let (mut x, mut run): (f32, Vec<&str>) = (b.x, Vec::new());
        let flush = |sl: &mut Sheet, x: &mut f32, run: &mut Vec<&str>| {
            if !run.is_empty() {
                let at = sl.text("verse", &run.join(" "), Tx::l(*x, base).size(z));
                *x += at.w + space;
                run.clear();
            }
        };
        for tok in line {
            match tok {
                fit::Tok::Num(n) => {
                    flush(sl, &mut x, &mut run);
                    let at = sl.text("verse_n", n, Tx::l(x, base - rise * z).size(32.0_f32.max((z * scale).round())));
                    x += at.w + gap * z;
                }
                fit::Tok::Word(word) => run.push(word),
            }
        }
        flush(sl, &mut x, &mut run);
    }
    Ok(z)
}

/// A slide drawn in a poster style: the whole screen, its own ground included. A layout that fails
/// at the floor size is drawn again at the first half of its rows, so nothing is cut off; the loop
/// shows a card that long as several slides through [`paginate`].
pub fn draw(slide: &Slide, style: NoticeStyle, w: f32, h: f32, pics: &Pics) -> El {
    let have = data::Have { photo: pics.photo.is_some(), photo2: pics.photo2.is_some(), qris: pics.qris.is_some() };
    let typing = TYPING.with(std::cell::Cell::get);
    let sheet = sheet(slide, style, have).or_else(|split| if typing { forced(slide, style, have) } else { Err(split) }).unwrap_or_else(|_| {
        let mut half = slide.clone();
        while let Some((first, _)) = split(&half, style) {
            half = first;
            if let Ok(sheet) = sheet(&half, style, have) {
                return sheet;
            }
        }
        // What nothing can hold is drawn at its floors, so the editor still shows it.
        forced(slide, style, have).unwrap_or_else(|_| Sheet::new(style))
    });
    sheet.build(w, h, pics)
}

/// The slide at the floors of its styles, whatever does not fit.
fn forced(slide: &Slide, style: NoticeStyle, have: data::Have) -> Result<Sheet, Split> {
    let floor = if slide.notice == NoticeKind::Plain { Some(plain_body_range(slide, style).1) } else { None };
    build(slide, style, have, floor, true)
}

/// The largest and smallest size a plain card's body is set in: the card's own ceiling or the
/// style's, down to the floor.
fn plain_body_range(slide: &Slide, style: NoticeStyle) -> (f32, f32) {
    let spec = number(&dir_of(style)["type"]["n_body"]["size"]);
    let hi = if slide.body_face.size > 0 { f32::from(slide.body_face.size) } else { spec }.floor();
    (hi, BODY_FLOOR.min(hi))
}

/// A plain card's body is set as large as its ceiling lets the whole slide hold, down to the floor.
fn sheet(slide: &Slide, style: NoticeStyle, have: data::Have) -> Result<Sheet, Split> {
    if slide.notice != NoticeKind::Plain {
        return build(slide, style, have, None, false);
    }
    let (hi, lo) = plain_body_range(slide, style);
    if let Ok(found) = build(slide, style, have, Some(hi), false) {
        return Ok(found);
    }
    let mut best = build(slide, style, have, Some(lo), false)?;
    let (mut ok, mut bad) = (lo, hi);
    while bad - ok > 1.0 {
        let mid = ((ok + bad) / 2.0).floor();
        match build(slide, style, have, Some(mid), false) {
            Ok(found) => (best, ok) = (found, mid),
            Err(_) => bad = mid,
        }
    }
    Ok(best)
}

fn build(slide: &Slide, style: NoticeStyle, have: data::Have, body: Option<f32>, force: bool) -> Result<Sheet, Split> {
    let data = Data::of(slide);
    let mut sheet = Sheet::new(style);
    (sheet.body_px, sheet.force) = (body, force);
    if slide.notice == NoticeKind::Plain {
        sheet.title_cap = (slide.title_face.size > 0).then(|| f32::from(slide.title_face.size));
        sheet.style_roles(&["title", "title_w", "n_title"], slide.title_face);
        sheet.style_roles(&["n_body", "n_body_b"], slide.body_face);
    }
    match style {
        NoticeStyle::Greige => d1::draw(&mut sheet, &data, have)?,
        NoticeStyle::Pill => d2::draw(&mut sheet, &data, have)?,
        NoticeStyle::Neon => d3::draw(&mut sheet, &data, have)?,
        _ => d4::draw(&mut sheet, &data, have)?,
    }
    foot(&mut sheet, slide, style);
    if slide.notice == NoticeKind::Plain {
        sheet.align(slide.title_face.align, &["title", "title_w", "n_title"]);
        sheet.align(slide.body_face.align, &["n_body", "n_body_b"]);
    }
    Ok(sheet)
}

/// Which part of a long card a slide is and what it holds, small at the foot of the slide.
fn foot(sheet: &mut Sheet, slide: &Slide, style: NoticeStyle) {
    let Some((at, of)) = slide.part else { return };
    let note = if slide.caption.is_empty() { format!("{at}/{of}") } else { format!("{at}/{of}  {}", slide.caption) };
    let (colour, x) = match style {
        NoticeStyle::Greige => ("mute", sheet.g("mx")),
        NoticeStyle::Pill => ("pill", sheet.g("mx")),
        NoticeStyle::Neon => ("white", sheet.g("mx")),
        _ => ("blue", sheet.g("mx")),
    };
    sheet.text("kicker", &note, Tx::l(x, 1040.0).cy().fill(colour));
}

/// Whether a layout holds this slide, and the least it has to give up when it does not.
pub fn holds(slide: &Slide, style: NoticeStyle) -> bool {
    sheet(slide, style, layout::have_of(slide, &Pics::default())).is_ok()
}

/// The slide in two halves of its rows, each with its place in `part`; None for a kind with no rows to cut.
fn split(slide: &Slide, style: NoticeStyle) -> Option<(Slide, Slide)> {
    if slide.notice == NoticeKind::Plain {
        return cut(slide, style);
    }
    let lines: Vec<&str> = slide.text.lines().filter(|line| !line.trim().is_empty()).collect();
    let rows: Vec<&str> = lines.iter().copied().filter(|line| !line.starts_with("== ")).collect();
    let total: Option<&str> = lines.iter().copied().find(|line| line.starts_with("== "));
    if !matches!(slide.notice, NoticeKind::Schedule | NoticeKind::Birthdays | NoticeKind::Money) || rows.len() < 2 {
        return None;
    }
    let cut = rows.len().div_ceil(2);
    let piece = |from: usize, to: usize| -> Slide {
        let mut text = rows[from..to].join("\n");
        if let Some(total) = total {
            text = format!("{text}\n{total}");
        }
        let mut piece = slide.clone();
        piece.text = text;
        piece.marked = slide.marked.iter().filter(|at| (from..to).contains(at)).map(|at| at - from).collect();
        piece.dates = slide.dates.get(from..to.min(slide.dates.len())).map(<[_]>::to_vec).unwrap_or_default();
        piece
    };
    Some((piece(0, cut), piece(cut, rows.len())))
}

/// A card's pages with each long one cut into slides that hold, for the cards drawn in a poster style.
pub fn paginate_pages(pages: Vec<(usize, Slide)>, look_style: NoticeStyle) -> Vec<(usize, Slide)> {
    pages
        .into_iter()
        .flat_map(|(at, slide)| {
            let style = slide.style.unwrap_or(look_style);
            let cut = if style.is_poster() { paginate(&slide, style) } else { vec![slide] };
            cut.into_iter().map(move |slide| (at, slide))
        })
        .collect()
}

/// The slides a card needs in a poster style: itself when it fits, else halves of its rows, each
/// with its place in `part` and the range it holds in `caption`.
pub fn paginate(slide: &Slide, style: NoticeStyle) -> Vec<Slide> {
    fn go(slide: &Slide, style: NoticeStyle, depth: u8, out: &mut Vec<Slide>) {
        if depth >= 3 || holds(slide, style) {
            out.push(slide.clone());
            return;
        }
        match split(slide, style) {
            Some((first, second)) => {
                go(&first, style, depth + 1, out);
                go(&second, style, depth + 1, out);
            }
            None => out.push(slide.clone()),
        }
    }
    let mut pages = Vec::new();
    go(slide, style, 0, &mut pages);
    let count = pages.len() as u16;
    if count > 1 {
        for (at, page) in pages.iter_mut().enumerate() {
            page.part = Some((at as u16 + 1, count));
            page.caption = data::range_of(page);
        }
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tokens_of_all_four_directions_are_there_with_their_faces() {
        for style in [NoticeStyle::Greige, NoticeStyle::Pill, NoticeStyle::Neon, NoticeStyle::Split] {
            let d = dir_of(style);
            assert!(d["palette"].is_object() && d["grid"]["mx"].is_number(), "{style:?}");
            for font in d["fonts"].as_object().into_iter().flatten().map(|(_, spec)| spec) {
                let file = font["file"].as_str().unwrap_or("");
                assert!(fonts::face_bytes(file).is_some(), "{file} is not bundled");
            }
        }
    }

    #[test]
    fn a_covering_picture_overflows_on_its_long_side_and_sits_where_the_design_says() {
        assert_eq!(cover_at(200.0, 100.0, 100.0, 100.0, Pos::Min), (0.0, 0.0, 200.0, 100.0));
        assert_eq!(cover_at(200.0, 100.0, 100.0, 100.0, Pos::Mid), (-50.0, 0.0, 200.0, 100.0));
        assert_eq!(cover_at(200.0, 100.0, 100.0, 100.0, Pos::Max), (-100.0, 0.0, 200.0, 100.0));
        assert_eq!(cover_at(100.0, 200.0, 100.0, 100.0, Pos::Max), (0.0, -100.0, 100.0, 200.0));
    }

    #[test]
    fn a_card_with_more_rows_than_a_slide_holds_becomes_slides_with_their_places() {
        let rows: Vec<String> = (0..60).map(|at| format!("Contoh Nama Lengkap {at}\t{} November", at % 28 + 1)).collect();
        let slide = Slide { notice: NoticeKind::Birthdays, text: rows.join("\n"), ..Slide::default() };
        let (first, second) = split(&slide, NoticeStyle::Greige).expect("rows can be cut");
        assert_eq!(first.text.lines().count() + second.text.lines().count(), 60);
        assert!(split(&Slide { notice: NoticeKind::Plain, text: "a\nb".into(), ..Slide::default() }, NoticeStyle::Greige).is_none());
    }
}
