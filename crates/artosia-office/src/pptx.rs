//! Reading a .pptx: slides in presentation order with their text, tables and pictures.
//!
//! Order comes from `presentation.xml` and its relationships, so a deck whose parts are named
//! out of order still reads right. Shapes are sorted by where they sit on the slide, because the
//! order in the XML is the order the author created them in, not the order a person reads.

use std::collections::HashMap;
use std::path::Path;

use crate::Table;
use crate::error::Error;
use crate::package::Package;
use crate::picture::{ImageFormat, Picture, Rect};
use crate::xml::Element;

/// A paragraph of text with its list level (0 is the top level).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paragraph {
    /// One paragraph. A soft line break inside it (Shift+Enter) is a `\n`.
    pub text: String,
    pub level: u8,
}

/// Where an entry of a slide sits in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Paragraph(usize),
    Table(usize),
    Picture(usize),
}

#[derive(Clone, Debug, Default)]
pub struct Slide {
    /// 1-based position in the presentation.
    pub number: usize,
    /// The slide marked "hide slide" in PowerPoint.
    pub hidden: bool,
    /// The title placeholder's text, when the slide has one.
    pub title: Option<String>,
    /// Text outside the title, in reading order. Empty paragraphs are dropped.
    pub paragraphs: Vec<Paragraph>,
    pub tables: Vec<Table>,
    pub pictures: Vec<Picture>,
    /// Paragraphs, tables and pictures mixed in reading order (top to bottom, then left to right).
    pub order: Vec<Item>,
}

#[derive(Clone, Debug, Default)]
pub struct Pptx {
    /// The slide size in EMU (914,400 per inch); 16:9 is 12192000 by 6858000.
    pub size: (i64, i64),
    pub slides: Vec<Slide>,
}

impl Pptx {
    pub fn open(path: impl AsRef<Path>) -> Result<Pptx, Error> {
        read(Package::open(path.as_ref())?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Pptx, Error> {
        read(Package::from_bytes(bytes.to_vec())?)
    }
}

/// A shape's box in EMU.
#[derive(Clone, Copy, Debug)]
struct Emu {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// Maps a group's child coordinates onto the slide: `slide = child * scale + shift`.
#[derive(Clone, Copy, Debug)]
struct Map {
    sx: f64,
    sy: f64,
    tx: f64,
    ty: f64,
}

impl Map {
    const IDENTITY: Map = Map { sx: 1.0, sy: 1.0, tx: 0.0, ty: 0.0 };

    fn apply(self, r: Emu) -> Emu {
        Emu { x: r.x * self.sx + self.tx, y: r.y * self.sy + self.ty, w: r.w * self.sx, h: r.h * self.sy }
    }

    /// The map of a child group seen through this one.
    fn through(self, group: Map) -> Map {
        Map { sx: group.sx * self.sx, sy: group.sy * self.sy, tx: group.tx * self.sx + self.tx, ty: group.ty * self.sy + self.ty }
    }
}

enum Found {
    Text { paragraphs: Vec<Paragraph>, title: bool },
    Table(Table),
    Picture(Picture),
}

struct Entry {
    rect: Option<Emu>,
    found: Found,
}

struct Reader<'a> {
    package: &'a mut Package,
    /// Layouts and masters, parsed once.
    parts: HashMap<String, Option<Element>>,
}

fn emu(xfrm: &Element) -> Option<Emu> {
    let off = xfrm.child("off")?;
    let ext = xfrm.child("ext")?;
    let n = |e: &Element, k: &str| e.attr(k).and_then(|v| v.parse::<f64>().ok());
    Some(Emu { x: n(off, "x")?, y: n(off, "y")?, w: n(ext, "cx")?, h: n(ext, "cy")? })
}

fn group_map(group: &Element) -> Option<Map> {
    let xfrm = group.child("grpSpPr")?.child("xfrm")?;
    let off = emu(xfrm)?;
    let n = |k: &str| xfrm.child("chOff").and_then(|e| e.attr(k)).and_then(|v| v.parse::<f64>().ok());
    let m = |k: &str| xfrm.child("chExt").and_then(|e| e.attr(k)).and_then(|v| v.parse::<f64>().ok());
    let (cx, cy, cw, ch) = (n("x")?, n("y")?, m("cx")?, m("cy")?);
    if cw <= 0.0 || ch <= 0.0 {
        return None;
    }
    let (sx, sy) = (off.w / cw, off.h / ch);
    Some(Map { sx, sy, tx: off.x - cx * sx, ty: off.y - cy * sy })
}

/// A placeholder's identity: `type` and `idx` as written.
fn placeholder(shape: &Element) -> Option<(Option<String>, Option<String>)> {
    let ph = shape.path(&["nvSpPr", "nvPr", "ph"])?;
    Some((ph.attr("type").map(str::to_string), ph.attr("idx").map(str::to_string)))
}

fn is_title(kind: Option<&str>) -> bool {
    matches!(kind, Some("title" | "ctrTitle"))
}

/// Footers, dates and slide numbers repeat on every slide and say nothing.
fn is_furniture(kind: Option<&str>) -> bool {
    matches!(kind, Some("dt" | "ftr" | "sldNum" | "hdr"))
}

fn paragraphs_of(body: &Element) -> Vec<Paragraph> {
    body.children_named("p")
        .filter_map(|p| {
            let mut text = String::new();
            for part in p.elements() {
                match part.name.as_str() {
                    "r" | "fld" => {
                        if let Some(t) = part.child("t") {
                            text.push_str(&t.text());
                        }
                    }
                    "br" => text.push('\n'),
                    _ => {}
                }
            }
            let text = text.trim().to_string();
            let level = p.child("pPr").and_then(|p| p.attr("lvl")).and_then(|l| l.parse::<u8>().ok()).unwrap_or(0);
            (!text.is_empty()).then_some(Paragraph { text, level })
        })
        .collect()
}

fn table_of(tbl: &Element) -> Table {
    let rows = tbl
        .children_named("tr")
        .map(|tr| {
            tr.children_named("tc")
                .map(|tc| {
                    let lines: Vec<String> = tc.child("txBody").map(|b| paragraphs_of(b).into_iter().map(|p| p.text).collect()).unwrap_or_default();
                    lines.join("\n")
                })
                .collect()
        })
        .collect();
    Table { rows }
}

impl Reader<'_> {
    fn part(&mut self, name: &str) -> Option<&Element> {
        if !self.parts.contains_key(name) {
            let parsed = self.package.read_xml(name).ok();
            self.parts.insert(name.to_string(), parsed);
        }
        self.parts.get(name)?.as_ref()
    }

    /// The box a placeholder inherits from its layout, then its master.
    fn inherited_rect(&mut self, slide_part: &str, ph: &(Option<String>, Option<String>)) -> Option<Emu> {
        let layout = self.package.rels(slide_part).into_iter().find(|r| r.kind == "slideLayout")?.target;
        if let Some(rect) = self.part(&layout).and_then(|l| find_placeholder_rect(l, ph, true)) {
            return Some(rect);
        }
        let master = self.package.rels(&layout).into_iter().find(|r| r.kind == "slideMaster")?.target;
        self.part(&master).and_then(|m| find_placeholder_rect(m, ph, false))
    }

    fn walk(&mut self, slide_part: &str, rels: &[crate::package::Rel], tree: &Element, map: Map, out: &mut Vec<Entry>) {
        for shape in tree.elements() {
            match shape.name.as_str() {
                "sp" => self.shape(slide_part, shape, map, out),
                "pic" => {
                    let rect = shape.child("spPr").and_then(|s| s.child("xfrm")).and_then(emu).map(|r| map.apply(r));
                    self.picture(rels, shape, rect, out);
                }
                "grpSp" => {
                    let inner = group_map(shape).map_or(map, |g| map.through(g));
                    self.walk(slide_part, rels, shape, inner, out);
                }
                "graphicFrame" => {
                    let rect = shape.child("xfrm").and_then(emu).map(|r| map.apply(r));
                    let mut tables = Vec::new();
                    shape.descendants("tbl", &mut tables);
                    if tables.is_empty() {
                        // An embedded object (a spreadsheet, say) keeps a picture of itself.
                        let mut pics = Vec::new();
                        shape.descendants("pic", &mut pics);
                        for pic in pics {
                            let own = pic.child("spPr").and_then(|s| s.child("xfrm")).and_then(emu).map(|r| map.apply(r));
                            self.picture(rels, pic, own.or(rect), out);
                        }
                    }
                    for tbl in tables {
                        out.push(Entry { rect, found: Found::Table(table_of(tbl)) });
                    }
                }
                // Newer features come wrapped with an older fallback; read the newer one.
                "AlternateContent" => {
                    if let Some(branch) = shape.child("Choice").or_else(|| shape.child("Fallback")) {
                        self.walk(slide_part, rels, branch, map, out);
                    }
                }
                _ => {}
            }
        }
    }

    fn shape(&mut self, slide_part: &str, shape: &Element, map: Map, out: &mut Vec<Entry>) {
        let ph = placeholder(shape);
        let kind = ph.as_ref().and_then(|p| p.0.as_deref());
        if is_furniture(kind) {
            return;
        }
        let Some(body) = shape.child("txBody") else { return };
        let paragraphs = paragraphs_of(body);
        if paragraphs.is_empty() {
            return;
        }
        let own = shape.child("spPr").and_then(|s| s.child("xfrm")).and_then(emu);
        let rect = match (own, &ph) {
            (Some(r), _) => Some(r),
            (None, Some(ph)) => self.inherited_rect(slide_part, ph),
            (None, None) => None,
        };
        out.push(Entry { rect: rect.map(|r| map.apply(r)), found: Found::Text { paragraphs, title: is_title(kind) } });
    }

    fn picture(&mut self, rels: &[crate::package::Rel], pic: &Element, rect: Option<Emu>, out: &mut Vec<Entry>) {
        let Some(id) = pic.path(&["blipFill", "blip"]).and_then(|b| b.prefixed("embed")) else { return };
        let Some(rel) = rels.iter().find(|r| r.id == id && !r.external) else { return };
        let Some(bytes) = self.package.read(&rel.target) else { return };
        let name = rel.target.rsplit('/').next().unwrap_or(&rel.target).to_string();
        let alt = pic.path(&["nvPicPr", "cNvPr"]).and_then(|c| c.attr("descr")).unwrap_or("").to_string();
        let format = ImageFormat::detect(&bytes, &name);
        out.push(Entry { rect, found: Found::Picture(Picture { name, bytes, format, rect: None, alt }) });
    }
}

fn find_placeholder_rect(part: &Element, want: &(Option<String>, Option<String>), by_idx: bool) -> Option<Emu> {
    let mut shapes = Vec::new();
    part.descendants("sp", &mut shapes);
    fn norm(t: &str) -> &str {
        match t {
            "ctrTitle" => "title",
            "subTitle" => "body",
            other => other,
        }
    }
    let want_type = want.0.as_deref().map(norm);
    shapes.into_iter().find_map(|s| {
        let (kind, idx) = placeholder(s)?;
        let same_idx = by_idx && want.1.is_some() && idx == want.1;
        // A placeholder with no type is a body placeholder.
        let same_type = kind.as_deref().map(norm).or(Some("body")) == want_type.or(Some("body")) && (!by_idx || want.1.is_none() || idx.is_none());
        if same_idx || same_type { s.child("spPr").and_then(|s| s.child("xfrm")).and_then(emu) } else { None }
    })
}

fn read(mut package: Package) -> Result<Pptx, Error> {
    let root_rels = package.rels("");
    let presentation = root_rels.iter().find(|r| r.kind == "officeDocument").map_or("ppt/presentation.xml", |r| r.target.as_str()).to_string();
    let doc = package.read_xml(&presentation)?;
    let size = doc
        .child("sldSz")
        .and_then(|s| Some((s.attr("cx")?.parse::<i64>().ok()?, s.attr("cy")?.parse::<i64>().ok()?)))
        .filter(|(w, h)| *w > 0 && *h > 0)
        .unwrap_or((12_192_000, 6_858_000));
    let rels = package.rels(&presentation);
    let slide_parts: Vec<String> = doc
        .path(&["sldIdLst"])
        .map(|list| list.children_named("sldId").filter_map(|s| s.prefixed("id")).filter_map(|id| rels.iter().find(|r| r.id == id)).map(|r| r.target.clone()).collect())
        .unwrap_or_default();

    let mut reader = Reader { package: &mut package, parts: HashMap::new() };
    let mut slides = Vec::new();
    for (at, part) in slide_parts.iter().enumerate() {
        let xml = reader.package.read_xml(part)?;
        let slide_rels = reader.package.rels(part);
        let mut entries = Vec::new();
        if let Some(tree) = xml.path(&["cSld", "spTree"]) {
            reader.walk(part, &slide_rels, tree, Map::IDENTITY, &mut entries);
        }
        slides.push(build(at + 1, xml.attr("show") == Some("0"), entries, size));
    }
    Ok(Pptx { size, slides })
}

fn build(number: usize, hidden: bool, mut entries: Vec<Entry>, size: (i64, i64)) -> Slide {
    // Shapes whose tops fall within the upper half of the same shape sit side by side and read
    // left to right, however unevenly they were placed: the church's birthday slide holds three
    // tall tables in a row with tops a few percent apart, and reading them by top alone shuffled
    // the months. Shapes with no position keep their XML order, last.
    let mut placed: Vec<Entry> = Vec::new();
    let mut loose: Vec<Entry> = Vec::new();
    for entry in entries.drain(..) {
        if entry.rect.is_some() { placed.push(entry) } else { loose.push(entry) }
    }
    placed.sort_by(|a, b| a.rect.map(|r| r.y).partial_cmp(&b.rect.map(|r| r.y)).unwrap_or(std::cmp::Ordering::Equal));
    let mut rows: Vec<Vec<Entry>> = Vec::new();
    let mut edge = f64::MIN;
    for entry in placed {
        let Some(r) = entry.rect else { continue };
        if r.y < edge && !rows.is_empty() {
            if let Some(row) = rows.last_mut() {
                row.push(entry);
            }
        } else {
            edge = r.y + r.h * 0.5;
            rows.push(vec![entry]);
        }
    }
    for mut row in rows {
        row.sort_by(|a, b| a.rect.map(|r| r.x).partial_cmp(&b.rect.map(|r| r.x)).unwrap_or(std::cmp::Ordering::Equal));
        entries.extend(row);
    }
    entries.extend(loose);
    let fraction = |r: Emu| Rect { x: (r.x / size.0 as f64) as f32, y: (r.y / size.1 as f64) as f32, w: (r.w / size.0 as f64) as f32, h: (r.h / size.1 as f64) as f32 };
    let mut slide = Slide { number, hidden, ..Slide::default() };
    let mut titles: Vec<String> = Vec::new();
    for entry in entries {
        match entry.found {
            Found::Text { paragraphs, title: true } => titles.extend(paragraphs.into_iter().map(|p| p.text.replace('\n', " "))),
            Found::Text { paragraphs, title: false } => {
                for p in paragraphs {
                    slide.order.push(Item::Paragraph(slide.paragraphs.len()));
                    slide.paragraphs.push(p);
                }
            }
            Found::Table(table) => {
                slide.order.push(Item::Table(slide.tables.len()));
                slide.tables.push(table);
            }
            Found::Picture(mut picture) => {
                picture.rect = entry.rect.map(fraction);
                slide.order.push(Item::Picture(slide.pictures.len()));
                slide.pictures.push(picture);
            }
        }
    }
    if !titles.is_empty() {
        slide.title = Some(titles.join(" "));
    }
    slide
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_files::pptx_bytes;

    #[test]
    fn slides_follow_the_presentation_not_the_file_names() {
        let zip = pptx_bytes();
        let deck = Pptx::from_bytes(&zip).unwrap();
        assert_eq!(deck.slides.len(), 3);
        // slide2.xml is listed first in the presentation.
        assert_eq!(deck.slides[0].title.as_deref(), Some("Youth service"));
        assert_eq!(deck.slides[1].title.as_deref(), Some("Sunday times"));
    }

    #[test]
    fn runs_join_into_one_paragraph_and_boxes_read_top_to_bottom_then_left_to_right() {
        let deck = Pptx::from_bytes(&pptx_bytes()).unwrap();
        let slide = &deck.slides[1];
        let texts: Vec<&str> = slide.paragraphs.iter().map(|p| p.text.as_str()).collect();
        // XML order is bottom, right, left, top.
        assert_eq!(texts, ["Welcome Alex", "Left box", "Right box", "Bottom box"]);
    }

    #[test]
    fn tall_boxes_side_by_side_read_left_to_right_though_their_tops_differ() {
        // Three columns of a list, the middle one set a little higher and the left one lowest,
        // as on the bulletin this was found on. Read by top alone they came out middle, right, left.
        let column = |x: f64, y: f64, word: &str| Entry {
            rect: Some(Emu { x, y, w: 3_000_000.0, h: 4_000_000.0 }),
            found: Found::Text { paragraphs: vec![Paragraph { text: word.into(), level: 0 }], title: false },
        };
        let heading = Entry { rect: Some(Emu { x: 500_000.0, y: 200_000.0, w: 9_000_000.0, h: 600_000.0 }), found: Found::Text { paragraphs: vec![Paragraph { text: "Heading".into(), level: 0 }], title: false } };
        let entries = vec![column(8_000_000.0, 1_500_000.0, "right"), column(500_000.0, 1_700_000.0, "left"), heading, column(4_200_000.0, 1_300_000.0, "middle")];
        let slide = build(1, false, entries, (12_192_000, 6_858_000));
        let texts: Vec<&str> = slide.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["Heading", "left", "middle", "right"]);
    }

    #[test]
    fn tables_come_out_as_rows_with_merged_cells_kept_in_their_column() {
        let deck = Pptx::from_bytes(&pptx_bytes()).unwrap();
        let table = &deck.slides[1].tables[0];
        assert_eq!(table.rows, vec![vec!["Time", "Event"], vec!["08.00", "Service one"], vec!["Joint", ""]]);
    }

    #[test]
    fn pictures_keep_their_bytes_format_and_place() {
        let deck = Pptx::from_bytes(&pptx_bytes()).unwrap();
        let slide = &deck.slides[2];
        assert!(slide.title.is_none() && slide.paragraphs.is_empty());
        let pic = &slide.pictures[0];
        assert_eq!(pic.name, "pic one.png");
        assert_eq!(pic.format, ImageFormat::Png);
        assert_eq!(pic.bytes, crate::test_files::PNG_1X1);
        let r = pic.rect.unwrap();
        assert!((r.x - 0.25).abs() < 1e-4 && (r.y - 0.5).abs() < 1e-4 && (r.w - 0.5).abs() < 1e-4 && (r.h - 0.25).abs() < 1e-4);
    }

    #[test]
    fn a_picture_inside_a_group_is_placed_through_the_group() {
        let deck = Pptx::from_bytes(&pptx_bytes()).unwrap();
        // The group spans the left half; its child sits in the middle of the group's own space.
        let r = deck.slides[0].pictures[0].rect.unwrap();
        assert!((r.x - 0.25).abs() < 1e-4 && (r.w - 0.25).abs() < 1e-4, "{r:?}");
    }

    #[test]
    fn placeholders_without_a_box_take_the_layout_one_and_footers_are_dropped() {
        let deck = Pptx::from_bytes(&pptx_bytes()).unwrap();
        let slide = &deck.slides[0];
        assert_eq!(slide.paragraphs.iter().map(|p| p.text.as_str()).collect::<Vec<_>>(), ["Notice line one\nline two"]);
    }

    #[test]
    fn not_a_zip_is_an_error() {
        assert!(matches!(Pptx::from_bytes(b"plain text"), Err(Error::Zip(_))));
    }
}
