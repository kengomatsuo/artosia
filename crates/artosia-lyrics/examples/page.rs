//! `cargo run -p artosia-lyrics --example page -- <address>` reads one page as Import would and
//! prints its title, source and counts only, never the words.

fn main() {
    let url = std::env::args().nth(1).unwrap_or_default();
    match artosia_lyrics::from_page(&url) {
        Ok(song) => {
            let stanzas = song.text.split("\n\n").count();
            let lines = song.text.lines().filter(|l| !l.trim().is_empty()).count();
            let first = song.text.lines().next().unwrap_or_default().chars().count();
            println!("title {:?}, source {}, {stanzas} stanzas, {lines} lines, first line {first} chars", song.title, song.source);
        }
        Err(why) => println!("error: {why}"),
    }
}
