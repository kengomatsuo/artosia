//! Keeps what an input hears as wave files, phrase by phrase, so the same sound can be read
//! again with another model or setting (`examples/read.rs`). No recogniser runs here.
//!
//! cargo run -p artosia-listen --example tape -- <seconds> <folder> [device-id]
//!
//! The device is the computer's own sound unless one is named. The files hold real sound:
//! keep the folder outside the repository and delete it when the measuring is done.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use artosia_listen::{Heard, Listener, SYSTEM_AUDIO};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (Some(seconds), Some(folder)) = (args.first().and_then(|s| s.parse::<u64>().ok()), args.get(1).map(PathBuf::from)) else {
        eprintln!("usage: tape <seconds> <folder> [device-id]");
        std::process::exit(2);
    };
    let device = args.get(2).map_or(SYSTEM_AUDIO, String::as_str).to_string();
    if let Err(e) = std::fs::create_dir_all(&folder) {
        eprintln!("Could not make {}: {e}", folder.display());
        std::process::exit(1);
    }
    let (tx, rx) = mpsc::channel();
    // A model that is not there: phrases are cut and kept all the same.
    let listener = match Listener::start_keeping(Some(&device), Path::new("no-model-here.bin"), "id", Some(&folder), move |h| {
        let _ = tx.send(h);
    }) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Could not listen: {e}");
            std::process::exit(1);
        }
    };
    let began = Instant::now();
    let (mut phrases, mut kept) = (0u32, 0.0f32);
    while began.elapsed() < Duration::from_secs(seconds) {
        if let Ok(Heard::Kept { seconds, .. }) = rx.recv_timeout(Duration::from_millis(200)) {
            phrases += 1;
            kept += seconds;
        }
    }
    drop(listener);
    println!("{phrases} phrases, {kept:.0} s of sound, in {}", folder.display());
}
