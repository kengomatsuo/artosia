//! Reading a .docx body as paragraphs and tables in document order.

use std::path::Path;

use crate::Table;
use crate::error::Error;
use crate::package::Package;
use crate::xml::Element;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocxParagraph {
    /// A tab in the text is `\t`, a soft line break `\n`.
    pub text: String,
    /// The paragraph style id (`Heading1`), when the author set one.
    pub style: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Paragraph(DocxParagraph),
    Table(Table),
}

#[derive(Clone, Debug, Default)]
pub struct Docx {
    pub blocks: Vec<Block>,
}

impl Docx {
    pub fn open(path: impl AsRef<Path>) -> Result<Docx, Error> {
        read(Package::open(path.as_ref())?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Docx, Error> {
        read(Package::from_bytes(bytes.to_vec())?)
    }

    /// Every table, in order.
    pub fn tables(&self) -> impl Iterator<Item = &Table> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Table(t) => Some(t),
            Block::Paragraph(_) => None,
        })
    }

    pub fn paragraphs(&self) -> impl Iterator<Item = &DocxParagraph> {
        self.blocks.iter().filter_map(|b| match b {
            Block::Paragraph(p) => Some(p),
            Block::Table(_) => None,
        })
    }
}

/// Text of a run-level container: runs, links, tracked insertions and the like. Deleted text is
/// stored as `delText`, which is not read, so tracked deletions stay out.
fn collect(container: &Element, out: &mut String) {
    for e in container.elements() {
        match e.name.as_str() {
            "r" => {
                for part in e.elements() {
                    match part.name.as_str() {
                        "t" => out.push_str(&part.text()),
                        "tab" => out.push('\t'),
                        "br" | "cr" => out.push('\n'),
                        "noBreakHyphen" => out.push('-'),
                        _ => {}
                    }
                }
            }
            "hyperlink" | "ins" | "smartTag" | "fldSimple" | "sdt" | "sdtContent" | "moveTo" => collect(e, out),
            _ => {}
        }
    }
}

fn paragraph(p: &Element) -> DocxParagraph {
    let mut text = String::new();
    collect(p, &mut text);
    let style = p.path(&["pPr", "pStyle"]).and_then(|s| s.prefixed("val")).map(str::to_string);
    DocxParagraph { text, style }
}

fn table(tbl: &Element) -> Table {
    let rows = tbl
        .children_named("tr")
        .map(|tr| {
            // Cells can hold a nested table; only the cell's own paragraphs are read.
            tr.children_named("tc").map(|tc| tc.children_named("p").map(|p| paragraph(p).text).collect::<Vec<_>>().join("\n")).collect()
        })
        .collect();
    Table { rows }
}

fn body(container: &Element, out: &mut Vec<Block>) {
    for e in container.elements() {
        match e.name.as_str() {
            "p" => out.push(Block::Paragraph(paragraph(e))),
            "tbl" => out.push(Block::Table(table(e))),
            "sdt" => {
                if let Some(content) = e.child("sdtContent") {
                    body(content, out);
                }
            }
            _ => {}
        }
    }
}

fn read(mut package: Package) -> Result<Docx, Error> {
    let main = package.rels("").into_iter().find(|r| r.kind == "officeDocument").map_or("word/document.xml".to_string(), |r| r.target);
    let doc = package.read_xml(&main)?;
    let mut blocks = Vec::new();
    if let Some(b) = doc.child("body") {
        body(b, &mut blocks);
    }
    Ok(Docx { blocks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_files::docx_bytes;

    #[test]
    fn paragraphs_and_tables_stay_in_document_order() {
        let doc = Docx::from_bytes(&docx_bytes()).unwrap();
        assert_eq!(doc.blocks.len(), 4);
        assert!(matches!(&doc.blocks[0], Block::Paragraph(p) if p.text == "Maret" && p.style.as_deref() == Some("Heading1")));
        assert!(matches!(&doc.blocks[1], Block::Table(t) if t.rows.len() == 3));
        assert!(matches!(&doc.blocks[2], Block::Paragraph(p) if p.text == "a\tb\nc & d"));
        assert!(matches!(&doc.blocks[3], Block::Table(_)));
    }

    #[test]
    fn a_table_reads_as_rows_of_cells() {
        let doc = Docx::from_bytes(&docx_bytes()).unwrap();
        let first = doc.tables().next().unwrap();
        assert_eq!(first.rows[0], ["No", "NAMA", "TANGGAL LAHIR"]);
        assert_eq!(first.rows[1], ["1", "Test Person", "05 Januari"]);
    }

    #[test]
    fn a_zip_without_a_document_is_an_error() {
        let zip = crate::test_files::Zip::new().add("hello.txt", b"x").finish();
        assert!(matches!(Docx::from_bytes(&zip), Err(Error::MissingPart(_))));
    }
}
