//! `cargo run -p artosia-lyrics --example gema -- "<query>"` reads Gema SABDA's lists (about a
//! minute, one page a second) and prints how many titles they hold and what the query finds.

fn main() {
    let query = std::env::args().nth(1).unwrap_or_default();
    match artosia_lyrics::GemaIndex::read("today") {
        Ok(index) => {
            println!("{} titles", index.songs.len());
            for found in index.search(&query, 5) {
                println!("{} -> {}", found.title, found.url);
            }
        }
        Err(why) => println!("error: {why}"),
    }
}
