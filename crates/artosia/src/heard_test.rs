//! The whole listening path in the operator, with no microphone: a spoken title goes through
//! the listen crate's resampling, phrase cutting and recogniser, and each phrase heard is handed
//! to `App::heard_final`, exactly as the live listener's inbox does.
//!
//! Needs the model and a spoken sample (see `crates/artosia-listen/tests/speech.rs`):
//! cargo test --release -p artosia --features speech heard_test -- --ignored --nocapture

use std::path::PathBuf;

use artosia_listen::{Heard, hear_samples, read_wav};

use crate::state::{App, HeardHit};

fn model() -> PathBuf {
    if let Some(path) = std::env::var_os("ARTOSIA_MODEL") {
        return path.into();
    }
    let appdata = std::env::var_os("APPDATA").expect("APPDATA is not set");
    PathBuf::from(appdata).join("artosia").join("models").join("ggml-base-q5_1.bin")
}

fn wave(name: &str) -> PathBuf {
    let dir = std::env::var_os("ARTOSIA_SPEECH_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/speech"), PathBuf::from);
    dir.join(format!("{name}.wav"))
}

#[test]
#[ignore = "needs the model and a spoken sample outside the repository"]
fn a_spoken_practice_title_comes_out_as_that_song() {
    let (samples, rate) = read_wav(&std::fs::read(wave("english-title")).expect("fixtures/speech/english-title.wav")).expect("valid wave");
    let mut app = App::practice();
    let mut heard = Vec::new();
    hear_samples(&samples, rate, &model(), "en", |h| {
        if let Heard::Final(text) = h {
            heard.push(text);
        }
    })
    .expect("recogniser runs");
    println!("heard: {heard:?}");
    for text in heard {
        app.heard_final(text);
    }
    let titles: Vec<&str> = app
        .heard_hits
        .iter()
        .filter_map(|hit| match hit {
            HeardHit::Song(index) => app.songs.get(*index).map(|song| song.title.as_str()),
            HeardHit::Passage(..) => None,
        })
        .collect();
    println!("hits: {titles:?}");
    assert_eq!(titles.first().copied(), Some("Hill of Morning Stars"), "heard {:?}", app.heard);
}

/// Listens to whatever this computer is playing (a song started by hand in a browser, say) and
/// prints, for every phrase read, how many words it had and the titles it named: never the words.
/// The library is the one `ARTOSIA_DATA` names. Nothing is asserted: it is a measurement.
///
/// ARTOSIA_DATA=<library folder> ARTOSIA_HEAR_SECONDS=90 ARTOSIA_HEAR_LANGUAGE=id \
///   cargo test --release -p artosia --features speech names_its_song -- --ignored --nocapture
#[test]
#[ignore = "needs the model, a library and music playing on this computer"]
fn music_playing_on_this_computer_names_its_song() {
    let seconds: u64 = std::env::var("ARTOSIA_HEAR_SECONDS").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
    let language = std::env::var("ARTOSIA_HEAR_LANGUAGE").unwrap_or_else(|_| "id".to_string());
    let mut app = App::load();
    println!("library: {} songs", app.songs.len());
    let (tx, rx) = std::sync::mpsc::channel();
    let listener = artosia_listen::Listener::start(Some(artosia_listen::SYSTEM_AUDIO), &model(), &language, move |heard| {
        let _ = tx.send(heard);
    })
    .expect("the computer's own sound opens");
    let began = std::time::Instant::now();
    let start = app.now;
    let mut named: Vec<(String, u32)> = Vec::new();
    while began.elapsed() < std::time::Duration::from_secs(seconds) {
        let Ok(heard) = rx.recv_timeout(std::time::Duration::from_millis(200)) else { continue };
        match heard {
            Heard::Final(text) => {
                app.now = start + began.elapsed().as_secs_f64();
                let words = text.split_whitespace().count();
                app.heard_final(text);
                let offers: Vec<String> = crate::suggest::suggestions(&app).into_iter().filter(|offer| offer.why == crate::suggest::Why::Heard).map(|offer| offer.title).collect();
                println!("{:5.1} s  {words:2} words  named {offers:?}", began.elapsed().as_secs_f32());
                for title in offers {
                    match named.iter_mut().find(|(known, _)| *known == title) {
                        Some((_, times)) => *times += 1,
                        None => named.push((title, 1)),
                    }
                }
            }
            Heard::Problem(problem) => println!("problem: {problem}"),
            _ => {}
        }
    }
    drop(listener);
    named.sort_by(|a, b| b.1.cmp(&a.1));
    println!("named in all: {named:?}");
}
