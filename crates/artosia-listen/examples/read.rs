//! Reads kept sound with a model and settings of choice, for measuring one against another on
//! the same sound (`examples/tape.rs` keeps it).
//!
//! cargo run --release -p artosia-listen --features whisper --example read -- <model> <language> <folder> [name=value ...]
//!
//!   window=12     seconds read at once; 0 reads each kept phrase as it was cut
//!   hop=12        seconds between the starts of two windows
//!   ctx=short     short: encode only the seconds that hold sound; full: the whole 30 s window
//!   beam=0        0: greedy; N: beam search of that width
//!   best=1        greedy: how many candidates are sampled
//!   fallback=0    1: read again at a higher temperature when the first reading looks wrong
//!   prompt=<file> words the model is told to expect
//!   threads=4
//!
//! Prints one line a window: its start in seconds, the seconds the reading took, and the text.
//! The text is what was sung: send it to a file outside the repository, never into a log.

use std::path::{Path, PathBuf};
use std::time::Instant;

use artosia_listen::{RECOGNISER_RATE, audio_ctx_for, clean_text, read_wav};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        eprintln!("usage: read <model> <language> <folder> [window=12 hop=12 ctx=short beam=0 best=1 fallback=0 prompt=<file> threads=4]");
        std::process::exit(2);
    }
    let (model, language, folder) = (PathBuf::from(&args[0]), args[1].clone(), PathBuf::from(&args[2]));
    let value = |name: &str| args.iter().skip(3).find_map(|arg| arg.strip_prefix(&format!("{name}=")).map(str::to_string));
    let number = |name: &str, otherwise: f32| value(name).and_then(|v| v.parse().ok()).unwrap_or(otherwise);
    let (window, hop) = (number("window", 12.0), number("hop", 12.0));
    let full_ctx = value("ctx").is_some_and(|v| v == "full");
    let (beam, best) = (number("beam", 0.0) as i32, number("best", 1.0) as i32);
    let fallback = number("fallback", 0.0) > 0.0;
    let threads = number("threads", 4.0) as i32;
    let prompt = value("prompt").map(|file| std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}")));

    // The kept phrases in their order, and all of them end to end.
    let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
        .unwrap_or_else(|e| panic!("{}: {e}", folder.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wav"))
        .collect();
    files.sort();
    let phrases: Vec<Vec<f32>> = files.iter().map(|file| samples_of(file)).collect();
    let rate = RECOGNISER_RATE as usize;
    let windows: Vec<(f32, Vec<f32>)> = if window <= 0.0 {
        let mut at = 0.0;
        phrases
            .iter()
            .map(|phrase| {
                let start = at;
                at += phrase.len() as f32 / rate as f32;
                (start, phrase.clone())
            })
            .collect()
    } else {
        let all: Vec<f32> = phrases.concat();
        let (long, step) = ((window * rate as f32) as usize, ((hop * rate as f32) as usize).max(rate));
        (0..all.len())
            .step_by(step)
            // A last piece shorter than three seconds holds too little to read.
            .filter(|start| all.len() - start >= 3 * rate)
            .map(|start| (start as f32 / rate as f32, all[start..(start + long).min(all.len())].to_vec()))
            .collect()
    };

    let ctx = WhisperContext::new_with_params(&model, WhisperContextParameters::default()).unwrap_or_else(|e| panic!("{}: {e}", model.display()));
    let mut state = ctx.create_state().unwrap_or_else(|e| panic!("{e}"));
    eprintln!("the model has {} audio layers", ctx.model_n_audio_layer());
    let (mut sound, mut spent) = (0.0f32, 0.0f32);
    for (start, samples) in &windows {
        let strategy = if beam > 0 { SamplingStrategy::BeamSearch { beam_size: beam, patience: -1.0 } } else { SamplingStrategy::Greedy { best_of: best } };
        let mut params = FullParams::new(strategy);
        params.set_language(if language == "auto" { None } else { Some(&language) });
        params.set_n_threads(threads);
        params.set_translate(false);
        params.set_no_context(true);
        // A window as long as a kept phrase comes back as one piece, as the listener reads it.
        params.set_single_segment(window <= 12.0);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        if !full_ctx {
            params.set_audio_ctx(audio_ctx_for(samples.len()));
        }
        if !fallback {
            params.set_temperature_inc(0.0);
        }
        if let Some(prompt) = &prompt {
            params.set_initial_prompt(prompt);
        }
        let began = Instant::now();
        state.full(params, samples).unwrap_or_else(|e| panic!("{e}"));
        let took = began.elapsed().as_secs_f32();
        let raw: String = state.as_iter().filter_map(|segment| segment.to_str_lossy().ok().map(|piece| piece.to_string())).collect();
        sound += samples.len() as f32 / rate as f32;
        spent += took;
        println!("{start:.1}\t{took:.2}\t{}", clean_text(&raw).unwrap_or_default());
    }
    eprintln!("{} windows, {sound:.0} s of sound read in {spent:.1} s: {:.2} of real time", windows.len(), spent / sound.max(1.0));
}

fn samples_of(file: &Path) -> Vec<f32> {
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let (samples, rate) = read_wav(&bytes).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    assert_eq!(rate, RECOGNISER_RATE, "{} is not at the recogniser's rate", file.display());
    samples
}
