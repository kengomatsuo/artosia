//! `cargo run -p artosia-lyrics --example find -- "<query>" [--fetch N]`
//!
//! Prints the matches (title, artist, source, URL). With `--fetch N`, also the number of
//! stanzas and lines of match N (1-based), never the text.

fn main() {
    let mut query = None;
    let mut pick = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--fetch" {
            pick = args.next().and_then(|n| n.parse::<usize>().ok());
        } else {
            query = Some(arg);
        }
    }
    let Some(query) = query else {
        eprintln!("usage: find \"<query>\" [--fetch N]");
        std::process::exit(2);
    };
    let found = match artosia_lyrics::search(&query) {
        Ok(found) => found,
        Err(error) => {
            eprintln!("search failed: {error}");
            std::process::exit(1);
        }
    };
    println!("{} match(es) for {query:?}", found.len());
    for (index, item) in found.iter().enumerate() {
        println!("{:>2}. {} | {} | {} | {}", index + 1, item.title, item.artist, item.source, item.url);
    }
    let Some(number) = pick else { return };
    let Some(item) = number.checked_sub(1).and_then(|at| found.get(at)) else {
        eprintln!("no match number {number}");
        std::process::exit(1);
    };
    match artosia_lyrics::fetch(item) {
        Ok(song) => {
            let stanzas = song.text.split("\n\n").filter(|s| !s.trim().is_empty()).count();
            let lines = song.text.lines().filter(|l| !l.trim().is_empty()).count();
            println!("match {number}: {stanzas} stanza(s), {lines} line(s), from {}", song.source);
        }
        Err(error) => {
            eprintln!("fetch failed: {error}");
            std::process::exit(1);
        }
    }
}
