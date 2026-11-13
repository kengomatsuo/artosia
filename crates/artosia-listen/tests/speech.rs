//! Proof that the recogniser hears, with no microphone: spoken samples made by
//! `tools/make-speech-sample.ps1` (Windows' own synthesiser, made-up sentences) go through
//! `hear_samples`, the same resampling, phrase cutting and recognising the microphone path uses.
//!
//! cargo test --release -p artosia-listen --features whisper --test speech -- --ignored --nocapture
//!
//! Needs the model in `%APPDATA%\artosia\models` (or `ARTOSIA_MODEL`) and the waves in
//! `fixtures/speech` (or `ARTOSIA_SPEECH_DIR`). Ignored because both are outside the repository.
#![cfg(feature = "whisper")]

use std::path::PathBuf;
use std::time::Instant;

use artosia_listen::{Heard, hear_samples, read_wav};

fn model() -> PathBuf {
    if let Some(path) = std::env::var_os("ARTOSIA_MODEL") {
        return path.into();
    }
    let appdata = std::env::var_os("APPDATA").expect("APPDATA is not set");
    PathBuf::from(appdata).join("artosia").join("models").join("ggml-base-q5_1.bin")
}

fn wave(name: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("ARTOSIA_SPEECH_DIR")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/speech"), PathBuf::from);
    let path = dir.join(format!("{name}.wav"));
    path.is_file().then_some(path)
}

/// Peak working set of this process in megabytes (Windows only; 0 elsewhere).
fn working_set_mb() -> f64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct Counters {
            size: u32,
            page_faults: u32,
            peak_working_set: usize,
            working_set: usize,
            rest: [usize; 6],
        }
        unsafe extern "system" {
            fn GetCurrentProcess() -> isize;
            fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, size: u32) -> i32;
        }
        let mut counters = Counters { size: size_of::<Counters>() as u32, page_faults: 0, peak_working_set: 0, working_set: 0, rest: [0; 6] };
        // SAFETY: `counters` is a valid, correctly sized out parameter for this call.
        let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.size) };
        if ok != 0 {
            return counters.peak_working_set as f64 / 1_048_576.0;
        }
    }
    0.0
}

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// Runs one sample and returns what was heard, printing the timings.
fn hear(name: &str, language: &str) -> Option<String> {
    let path = wave(name)?;
    let (samples, rate) = read_wav(&std::fs::read(path).expect("wave file")).expect("valid wave");
    let seconds = samples.len() as f64 / f64::from(rate);
    let before = working_set_mb();
    let started = Instant::now();
    let mut heard = Vec::new();
    hear_samples(&samples, rate, &model(), language, |h| {
        if let Heard::Final(text) = h {
            heard.push(text);
        }
    })
    .expect("recogniser runs");
    let took = started.elapsed().as_secs_f64();
    let text = heard.join(" ");
    println!(
        "{name} [{language}]: heard {text:?}; {took:.2} s (load included) for {seconds:.2} s of audio; peak working set {before:.0} MB before, {:.0} MB after",
        working_set_mb()
    );
    Some(text)
}

fn overlap(expected: &str, heard: &str) -> f64 {
    let heard = words(heard);
    let expected = words(expected);
    expected.iter().filter(|w| heard.contains(w)).count() as f64 / expected.len() as f64
}

#[test]
#[ignore = "needs the model and the spoken samples, see the file header"]
fn hears_a_made_up_english_sentence() {
    let text = hear("english-sentence", "en").expect("fixtures/speech/english-sentence.wav is missing");
    let found = overlap("The blue lantern waits beside the wooden gate", &text);
    assert!(found >= 0.7, "only {found:.2} of the words were heard: {text:?}");
}

#[test]
#[ignore = "needs the model and the spoken samples, see the file header"]
fn hears_a_made_up_indonesian_sentence() {
    let Some(text) = hear("indonesian-sentence", "id") else {
        println!("no Indonesian sample: no Indonesian voice is installed on this PC");
        return;
    };
    let found = overlap("Lampu biru menunggu di samping pintu kayu", &text);
    println!("indonesian overlap {found:.2}");
    assert!(found >= 0.4, "only {found:.2} of the words were heard: {text:?}");
}

#[test]
#[ignore = "needs the model and the spoken samples, see the file header"]
fn memory_of_the_loaded_recogniser() {
    let idle = working_set_mb();
    hear("english-sentence", "en").expect("fixtures/speech/english-sentence.wav is missing");
    println!("process peak working set: {idle:.0} MB before the model, {:.0} MB with the model loaded and one phrase read", working_set_mb());
}
