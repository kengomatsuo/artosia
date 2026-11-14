//! Draws each slide of a .pptx as a PNG and prints the tool, the count and the time, never text.
//!
//! `cargo run -p artosia-office --example export -- <deck.pptx> <out-dir> [width]`

use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [pptx, out, rest @ ..] = args.as_slice() else {
        eprintln!("usage: export <deck.pptx> <out-dir> [width]");
        return;
    };
    let width = rest.first().and_then(|w| w.parse().ok()).unwrap_or(1600);
    let start = Instant::now();
    match artosia_office::slides_as_pictures(Path::new(pptx), Path::new(out), width) {
        Ok(done) => println!("{:?}: {} slides in {:.1} s", done.tool, done.files.len(), start.elapsed().as_secs_f64()),
        Err(e) => println!("error: {e} ({:.1} s)", start.elapsed().as_secs_f64()),
    }
}
