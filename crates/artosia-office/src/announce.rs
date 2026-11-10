//! A read bulletin as drafts for Artosia's announcements, which hold a title and lines of text.
//!
//! Lost on the way, by design: fonts, colours, sizes, bold, bullets, list levels, exact
//! positions, animations, backgrounds and shapes that carry no text. A slide's pictures are kept
//! only when the picture is all the slide has.

use crate::picture::Picture;
use crate::pptx::{Item, Pptx, Slide};

#[derive(Clone, Debug)]
pub struct Draft {
    /// 1-based slide the draft came from.
    pub slide: usize,
    /// The slide's title; with none, a short first line of text stands in; else empty.
    pub title: String,
    /// One line per `\n`. A table row is one line with its cells joined by a TAB.
    pub body: String,
    /// Set only when the slide is nothing but a picture (the first one if there are several).
    pub picture: Option<Picture>,
    /// Pictures on the slide that the draft does not carry.
    pub dropped_pictures: usize,
}

/// The longest first line that is taken as a title when the slide has no title placeholder.
const MAX_TITLE: usize = 60;

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn lines(slide: &Slide) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in &slide.order {
        match *item {
            Item::Paragraph(i) => out.extend(slide.paragraphs[i].text.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string)),
            Item::Table(i) => {
                for row in &slide.tables[i].rows {
                    if row.iter().all(|c| c.trim().is_empty()) {
                        continue;
                    }
                    // Trailing empty cells (a merged cell's tail) would only add tabs.
                    let mut cells: Vec<String> = row.iter().map(|c| one_line(c)).collect();
                    while cells.last().is_some_and(String::is_empty) {
                        cells.pop();
                    }
                    out.push(cells.join("\t"));
                }
            }
            Item::Picture(_) => {}
        }
    }
    out
}

/// One draft per slide that has text, a table or a picture, in presentation order. Hidden slides
/// are included: the operator can drop them, but the bulletin's author may still want them.
pub fn to_announcements(deck: &Pptx) -> Vec<Draft> {
    let mut drafts = Vec::new();
    for slide in &deck.slides {
        let mut title = slide.title.as_deref().map(one_line).unwrap_or_default();
        let mut body = lines(slide);
        if title.is_empty()
            && matches!(slide.order.first(), Some(Item::Paragraph(_)))
            && let Some(first) = body.first()
            && first.chars().count() <= MAX_TITLE
            && !first.contains('\t')
        {
            title = body.remove(0);
        }
        if body.is_empty() && title.is_empty() {
            if let Some(picture) = slide.pictures.first() {
                drafts.push(Draft { slide: slide.number, title, body: String::new(), picture: Some(picture.clone()), dropped_pictures: slide.pictures.len() - 1 });
            }
            continue;
        }
        drafts.push(Draft { slide: slide.number, title, body: body.join("\n"), picture: None, dropped_pictures: slide.pictures.len() });
    }
    drafts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_files::pptx_bytes;

    #[test]
    fn a_table_row_is_one_line_with_tabs_and_text_follows_reading_order() {
        let drafts = to_announcements(&Pptx::from_bytes(&pptx_bytes()).unwrap());
        assert_eq!(drafts.len(), 3);
        assert_eq!(drafts[1].title, "Sunday times");
        assert_eq!(drafts[1].body, "Welcome Alex
Left box
Right box
Time	Event
08.00	Service one
Joint
Bottom box");
    }

    #[test]
    fn soft_breaks_become_lines() {
        let drafts = to_announcements(&Pptx::from_bytes(&pptx_bytes()).unwrap());
        assert_eq!(drafts[0].title, "Youth service");
        assert_eq!(drafts[0].body, "Notice line one\nline two");
        assert!(drafts[0].picture.is_none());
        assert_eq!(drafts[0].dropped_pictures, 1);
    }

    #[test]
    fn a_picture_only_slide_becomes_a_picture_draft() {
        let drafts = to_announcements(&Pptx::from_bytes(&pptx_bytes()).unwrap());
        let d = &drafts[2];
        assert!(d.title.is_empty() && d.body.is_empty());
        assert_eq!(d.picture.as_ref().map(|p| p.name.as_str()), Some("pic one.png"));
        assert_eq!(d.slide, 3);
    }
}
