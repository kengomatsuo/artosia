//! Tiny Office files built in code, with made-up content.

use std::io::Write;

use zip::write::SimpleFileOptions;

pub(crate) struct Zip {
    parts: Vec<(String, Vec<u8>)>,
}

impl Zip {
    pub fn new() -> Self {
        Zip { parts: Vec::new() }
    }

    pub fn add(mut self, name: &str, bytes: &[u8]) -> Self {
        self.parts.push((name.to_string(), bytes.to_vec()));
        self
    }

    pub fn finish(self) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, bytes) in &self.parts {
            writer.start_file(name.as_str(), options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
}

/// The signature and header of a 1x1 PNG; the reader never decodes pixels.
pub(crate) const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0x0D, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4,
    0x89,
];

const RELS_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;
const T: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn rel(id: &str, kind: &str, target: &str) -> String {
    format!(r#"<Relationship Id="{id}" Type="{T}/{kind}" Target="{target}"/>"#)
}

fn rels(items: &[String]) -> String {
    format!("{RELS_HEAD}{}</Relationships>", items.concat())
}

fn text_shape(name: &str, x: i64, y: i64, runs: &[&str]) -> String {
    let runs: String = runs.iter().map(|r| format!("<a:r><a:t>{r}</a:t></a:r>")).collect();
    format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="{name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="2000000" cy="500000"/></a:xfrm></p:spPr><p:txBody><a:p>{runs}</a:p></p:txBody></p:sp>"#
    )
}

fn cell(text: &str, extra: &str) -> String {
    let body = if text.is_empty() { "<a:p/>".to_string() } else { format!("<a:p><a:r><a:t>{text}</a:t></a:r></a:p>") };
    format!("<a:tc{extra}><a:txBody>{body}</a:txBody></a:tc>")
}

fn slide(tree: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?><p:sld xmlns:a="a" xmlns:p="p" xmlns:r="r"><p:cSld><p:spTree>{tree}</p:spTree></p:cSld></p:sld>"#)
}

/// Three slides, stored out of presentation order:
/// `slide2.xml` is first (inherited placeholders, a footer, a picture in a group), `slide1.xml`
/// second (shuffled boxes, a table with a merged cell) and `slide3.xml` last (one picture).
pub(crate) fn pptx_bytes() -> Vec<u8> {
    let youth = slide(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="t"/><p:cNvSpPr/><p:nvPr><p:ph type="ctrTitle"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>Youth service</a:t></a:r></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="b"/><p:cNvSpPr/><p:nvPr><p:ph type="body" idx="1"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>Not</a:t></a:r><a:r><a:t>ice line</a:t></a:r><a:r><a:t> one</a:t></a:r><a:br/><a:r><a:t>line two</a:t></a:r></a:p><a:p><a:endParaRPr/></a:p></p:txBody></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="4" name="f"/><p:cNvSpPr/><p:nvPr><p:ph type="ftr" idx="11"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>Footer text</a:t></a:r></a:p></p:txBody></p:sp>
<p:grpSp><p:nvGrpSpPr><p:cNvPr id="5" name="g"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="6096000" cy="6858000"/><a:chOff x="0" y="0"/><a:chExt cx="12192000" cy="6858000"/></a:xfrm></p:grpSpPr>
<p:pic><p:nvPicPr><p:cNvPr id="6" name="p" descr="a small square"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr><a:xfrm><a:off x="6096000" y="0"/><a:ext cx="6096000" cy="3429000"/></a:xfrm></p:spPr></p:pic></p:grpSp>"#,
    );
    let sunday = slide(&format!(
        r#"{bottom}{right}{left}{top}<p:sp><p:nvSpPr><p:cNvPr id="9" name="t"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="9000000" cy="500000"/></a:xfrm></p:spPr><p:txBody><a:p><a:r><a:t>Sunday </a:t></a:r><a:r><a:t>times</a:t></a:r></a:p></p:txBody></p:sp>
<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="8" name="tb"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="500000" y="3000000"/><a:ext cx="8000000" cy="1500000"/></p:xfrm><a:graphic><a:graphicData><a:tbl><a:tr>{r1}</a:tr><a:tr>{r2}</a:tr><a:tr>{r3}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        bottom = text_shape("bottom", 0, 5_000_000, &["Bottom box"]),
        right = text_shape("right", 6_000_000, 2_010_000, &["Right box"]),
        left = text_shape("left", 500_000, 2_000_000, &["Left box"]),
        top = text_shape("top", 0, 1_000_000, &["Welcome ", "Alex"]),
        r1 = cell("Time", "") + &cell("Event", ""),
        r2 = cell("08.00", "") + &cell("Service one", ""),
        r3 = cell("Joint", r#" gridSpan="2""#) + &cell("", r#" hMerge="1""#),
    ));
    let picture = slide(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="2" name="p"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rId2"/></p:blipFill><p:spPr><a:xfrm><a:off x="3048000" y="3429000"/><a:ext cx="6096000" cy="1714500"/></a:xfrm></p:spPr></p:pic>"#,
    );
    let layout = slide(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="t"/><p:cNvSpPr/><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="12192000" cy="900000"/></a:xfrm></p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="b"/><p:cNvSpPr/><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="1000000"/><a:ext cx="12192000" cy="4000000"/></a:xfrm></p:spPr></p:sp>"#,
    );
    let presentation = r#"<?xml version="1.0"?><p:presentation xmlns:p="p" xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/><p:sldId id="258" r:id="rId3"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/></p:presentation>"#;
    Zip::new()
        .add("_rels/.rels", rels(&[rel("rId1", "officeDocument", "ppt/presentation.xml")]).as_bytes())
        .add("ppt/presentation.xml", presentation.as_bytes())
        .add(
            "ppt/_rels/presentation.xml.rels",
            rels(&[rel("rId1", "slide", "slides/slide2.xml"), rel("rId2", "slide", "slides/slide1.xml"), rel("rId3", "slide", "slides/slide3.xml")]).as_bytes(),
        )
        .add("ppt/slides/slide1.xml", sunday.as_bytes())
        .add("ppt/slides/slide2.xml", youth.as_bytes())
        .add("ppt/slides/slide3.xml", picture.as_bytes())
        .add("ppt/slides/_rels/slide2.xml.rels", rels(&[rel("rId1", "slideLayout", "../slideLayouts/slideLayout1.xml"), rel("rId2", "image", "../media/pic%20one.png")]).as_bytes())
        .add("ppt/slides/_rels/slide3.xml.rels", rels(&[rel("rId2", "image", "../media/pic%20one.png")]).as_bytes())
        .add("ppt/slideLayouts/slideLayout1.xml", layout.as_bytes())
        .add("ppt/media/pic one.png", PNG_1X1)
        .finish()
}

fn w_cell(text: &str) -> String {
    format!("<w:tc><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:tc>")
}

fn w_row(cells: &[&str]) -> String {
    format!("<w:tr>{}</w:tr>", cells.iter().map(|c| w_cell(c)).collect::<String>())
}

/// A heading, a header-plus-two-row table, a paragraph with a tab, a break and an entity, and a
/// second table.
pub(crate) fn docx_bytes() -> Vec<u8> {
    let body = format!(
        r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Maret</w:t></w:r></w:p><w:tbl>{}{}{}</w:tbl><w:p><w:r><w:t>a</w:t><w:tab/><w:t>b</w:t><w:br/></w:r><w:r><w:t>c &amp; d</w:t></w:r><w:r><w:delText>gone</w:delText></w:r></w:p><w:tbl>{}</w:tbl><w:sectPr/></w:body></w:document>"#,
        w_row(&["No", "NAMA", "TANGGAL LAHIR"]),
        w_row(&["1", "Test Person", "05 Januari"]),
        w_row(&["2", "Other Person", "6 Feb"]),
        w_row(&["1", "Third Person", "7 Mei"]),
    );
    Zip::new()
        .add("_rels/.rels", rels(&[rel("rId1", "officeDocument", "word/document.xml")]).as_bytes())
        .add("word/document.xml", body.as_bytes())
        .finish()
}
