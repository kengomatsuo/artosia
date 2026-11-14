//! Reads a church's own office files: the weekly bulletin (.pptx) and a birthday list (.docx, .xlsx or .csv).
//!
//! No UI and no PowerPoint or Word needed: both formats are zips of XML. What comes out is
//! plain data (text, tables, pictures with their place on the slide); fonts, colours, animations
//! and exact positions are dropped on purpose, see the README.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod announce;
mod birthdays;
mod docx;
mod error;
mod export;
mod mapping;
mod package;
mod picture;
mod pptx;
mod redraw;
mod sheet;
mod xml;

#[cfg(test)]
mod test_files;

pub use announce::{Draft, to_announcements};
pub use birthdays::{Birthday, Found, dedup, from_docx, from_pptx, from_tables, in_month, in_range, parse_date, parse_date_year};
pub use docx::{Block, Docx, DocxParagraph};
pub use error::Error;
pub use export::{Cancel, Exported, Tool, slides_as_pictures};
pub use mapping::{Column, Mapping, guess, read};
pub use picture::{ConvertError, ImageFormat, Picture, Rect};
pub use pptx::{Item, Paragraph, Pptx, Slide};
pub use redraw::{Gift, Mapped, Money, TableKind, classify, parse_rupiah, redraw};
pub use sheet::{csv_from_bytes, xlsx_from_bytes, xlsx_table};

/// A table as rows of cell text. A merged cell leaves its continuation cells empty so that the
/// columns stay aligned. Several paragraphs in one cell are joined with `\n`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub rows: Vec<Vec<String>>,
}
