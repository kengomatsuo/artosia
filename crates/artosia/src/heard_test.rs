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
                // How near the phrase came: the best titles the search finds for all of it, with
                // how many of its words sit together in one line of that song.
                let near: Vec<String> = app.index.search(&text, 3).into_iter().filter_map(|hit| app.songs.get(hit.song).map(|song| format!("{} ({})", song.title, hit.together))).collect();
                println!("        nearest {near:?}");
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

/// Runs kept readings (the listen crate's `examples/read.rs`) through the naming the live
/// listener uses and prints, for each file, how many phrases had the file's song first or among
/// the nearest three, and what was offered and when: titles and counts, never the words.
///
/// ARTOSIA_DATA=<library folder> ARTOSIA_HEARD=<folder of label.tsv> ARTOSIA_TRUTH="label=words of the title;..." \
///   cargo test --release -p artosia kept_readings -- --ignored --nocapture
#[test]
#[ignore = "needs a library and readings kept outside the repository"]
fn kept_readings_name_their_song() {
    let folder = PathBuf::from(std::env::var_os("ARTOSIA_HEARD").expect("ARTOSIA_HEARD is not set"));
    let truth: Vec<(String, String)> = std::env::var("ARTOSIA_TRUTH")
        .unwrap_or_default()
        .split(';')
        .filter_map(|pair| pair.split_once('=').map(|(label, title)| (label.trim().to_string(), title.trim().to_lowercase())))
        .collect();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&folder).expect("the folder of readings").flatten().map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|ext| ext == "tsv")).collect();
    files.sort();
    let (mut all, mut all_first, mut all_three) = (0u32, 0u32, 0u32);
    for file in files {
        let label = file.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
        let wanted = truth.iter().find(|(known, _)| *known == label).map(|(_, title)| title.clone()).filter(|title| !title.is_empty());
        let mut app = App::load();
        let start = app.now;
        let (mut phrases, mut worded, mut first, mut three) = (0u32, 0u32, 0u32, 0u32);
        let mut offered: Vec<(String, f64)> = Vec::new();
        for line in std::fs::read_to_string(&file).expect("a reading").lines() {
            let mut parts = line.splitn(3, '\t');
            let at: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let text = parts.nth(1).unwrap_or("").to_string();
            phrases += 1;
            // A phrase is read when it ends, twelve seconds after it began.
            app.now = start + at + 12.0;
            if text.trim().is_empty() {
                continue;
            }
            worded += 1;
            let near: Vec<String> = app.index.search(&text, 3).into_iter().filter(|hit| hit.together >= 2).filter_map(|hit| app.songs.get(hit.song).map(|song| song.title.to_lowercase())).collect();
            if let Some(wanted) = &wanted {
                first += u32::from(near.first().is_some_and(|title| title.contains(wanted)));
                three += u32::from(near.iter().any(|title| title.contains(wanted)));
            }
            app.heard_final(text);
            for offer in crate::suggest::suggestions(&app).into_iter().filter(|offer| offer.why == crate::suggest::Why::Heard) {
                if !offered.iter().any(|(known, _)| *known == offer.title) {
                    offered.push((offer.title, at + 12.0));
                }
            }
        }
        let offers: Vec<String> = offered.iter().map(|(title, at)| format!("{title} at {at:.0} s")).collect();
        println!("{label:8} {phrases:2} phrases, {worded:2} with words, first {first:2}, in three {three:2}; offered {offers:?}");
        if wanted.is_some() {
            (all, all_first, all_three) = (all + phrases, all_first + first, all_three + three);
        }
    }
    if all > 0 {
        println!("songs: {all} phrases, right song first {}%, in three {}%", 100 * all_first / all, 100 * all_three / all);
    }
}
