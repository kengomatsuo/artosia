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
