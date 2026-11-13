//! Reads a bulletin and a birthday list by path and prints COUNTS ONLY, never a name or an
//! amount, so it is safe to run on real files.
//!
//! `cargo run -p artosia-office --example read -- <bulletin.pptx> <birthdays.docx> [out-dir]`
//!
//! With an out-dir, the first picture that needs converting is written there as a PNG.

use std::path::PathBuf;

use artosia_office::{Docx, Pptx, dedup, from_docx, from_pptx, in_month, to_announcements};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [pptx, docx, rest @ ..] = args.as_slice() else {
        eprintln!("usage: read <bulletin.pptx> <birthdays.docx> [out-dir]");
        return;
    };

    match Pptx::open(pptx) {
        Err(e) => println!("pptx: {e}"),
        Ok(deck) => {
            println!("pptx: {} slides, size {:?}", deck.slides.len(), deck.size);
            for s in &deck.slides {
                let rows: usize = s.tables.iter().map(|t| t.rows.len()).sum();
                println!(
                    "  slide {}: title {}, {} paragraphs, {} tables ({} rows), {} pictures, hidden {}",
                    s.number,
                    if s.title.is_some() { "yes" } else { "no" },
                    s.paragraphs.len(),
                    s.tables.len(),
                    rows,
                    s.pictures.len(),
                    s.hidden
                );
                for p in &s.pictures {
                    println!("    picture {} {:?} {} bytes at {:?}", p.name, p.format, p.bytes.len(), p.rect);
                }
            }
            for d in to_announcements(&deck) {
                println!(
                    "  draft from slide {}: title {} chars, body {} lines ({} with tabs), picture {}, dropped pictures {}",
                    d.slide,
                    d.title.chars().count(),
                    d.body.lines().count(),
                    d.body.lines().filter(|l| l.contains('\t')).count(),
                    d.picture.is_some(),
                    d.dropped_pictures
                );
            }
            let found = from_pptx(&deck);
            println!("  birthdays in the bulletin's tables: {} people, {} unread rows, {} header rows", found.people.len(), found.unread, found.headers);
            if let Some(out) = rest.first() {
                let out = PathBuf::from(out);
                let _ = std::fs::create_dir_all(&out);
                for d in to_announcements(&deck) {
                    if let Some(p) = &d.picture {
                        match p.drawable(1600) {
                            Ok((format, bytes)) => {
                                let target = out.join(format!("slide{}-{}.png", d.slide, p.name.replace('.', "_")));
                                println!("  converted {} ({:?} to {:?}), {} bytes -> {}", p.name, p.format, format, bytes.len(), target.display());
                                let _ = std::fs::write(target, bytes);
                            }
                            Err(e) => println!("  convert {}: {e}", p.name),
                        }
                    }
                }
            }
        }
    }

    match Docx::open(docx) {
        Err(e) => println!("docx: {e}"),
        Ok(doc) => {
            let tables: Vec<_> = doc.tables().collect();
            let rows: usize = tables.iter().map(|t| t.rows.len()).sum();
            println!("docx: {} paragraphs, {} tables, {} rows", doc.paragraphs().count(), tables.len(), rows);
            let found = from_docx(&doc);
            println!("  {} people, {} unread rows, {} header rows", found.people.len(), found.unread, found.headers);
            for month in 1..=12u8 {
                println!("  month {month:2}: {}", in_month(&found.people, month).len());
            }
            let unique = dedup(found.people);
            println!("  {} after dropping repeats of the same name and date", unique.len());
            for month in 1..=12u8 {
                println!("  unique month {month:2}: {}", in_month(&unique, month).len());
            }
        }
    }
}
