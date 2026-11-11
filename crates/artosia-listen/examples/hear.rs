//! Lists the audio inputs, then listens on one.
//!
//! cargo run -p artosia-listen --example hear -- [seconds] [device-id|default] [model-file] [id|en|auto]
//!
//! Prints the level as a bar a few times a second, and the length (never the words) of any text
//! unless `HEAR_SHOW_TEXT=1` is set. No audio is kept.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use artosia_listen::{Heard, Listener, devices};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seconds: u64 = args.first().and_then(|s| s.parse().ok()).unwrap_or(5);
    let device = args.get(1).filter(|d| d.as_str() != "default").cloned();
    let model = PathBuf::from(args.get(2).map_or("no-model-here.bin", String::as_str));
    let language = args.get(3).map_or("id", String::as_str).to_string();
    let show_text = std::env::var("HEAR_SHOW_TEXT").is_ok_and(|v| v == "1");

    println!("Audio inputs:");
    for d in devices() {
        println!("  {}  [{}]", d.name, d.id);
    }

    let (tx, rx) = mpsc::channel();
    let listener = match Listener::start(device.as_deref(), &model, &language, move |h| {
        let _ = tx.send(h);
    }) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Could not listen: {e}");
            std::process::exit(1);
        }
    };

    let began = Instant::now();
    let (mut levels, mut min, mut max) = (0u32, 1.0f32, 0.0f32);
    while began.elapsed() < Duration::from_secs(seconds) {
        let Ok(heard) = rx.recv_timeout(Duration::from_millis(200)) else {
            continue;
        };
        match heard {
            Heard::Level(l) => {
                levels += 1;
                min = min.min(l);
                max = max.max(l);
                let bar = "#".repeat((l * 40.0).round() as usize);
                println!("{:5.1}s  level {l:.3} |{bar}", began.elapsed().as_secs_f32());
            }
            Heard::Partial(t) | Heard::Final(t) if !show_text => println!("text arrived ({} characters)", t.chars().count()),
            Heard::Partial(t) => println!("partial: {t}"),
            Heard::Final(t) => println!("final: {t}"),
            Heard::Tempo(Some(t)) => println!("tempo {:.1} BPM, confidence {:.2}, last beat {:.2} s ago", t.bpm, t.confidence, t.since_beat),
            Heard::Tempo(None) => {}
            // This example keeps no phrases, so neither arrives.
            Heard::Kept { .. } | Heard::Read { .. } => {}
            Heard::Speed { sound, took } => println!("read {sound:.1} s of sound in {took:.2} s"),
            Heard::Problem(p) => println!("problem: {p}"),
        }
    }
    drop(listener);
    println!("{levels} level reports in {seconds} s, range {min:.2} to {max:.2}");
}
