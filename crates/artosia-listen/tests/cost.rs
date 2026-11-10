//! Run with `cargo test -p artosia-listen --release --test cost -- --ignored --nocapture`:
//! prints how long the resampler and segmenter take for 60 s of 48 kHz audio.

use std::time::Instant;

use artosia_listen::{Resampler, Segmenter, SegmenterConfig};

#[test]
#[ignore = "timing only"]
fn processing_cost_of_one_minute_of_audio() {
    let input: Vec<f32> = (0..48_000 * 60).map(|i| (i as f32 * 0.05).sin() * 0.2).collect();
    let started = Instant::now();
    let mut resampler = Resampler::new(48_000, 16_000);
    let mut segmenter = Segmenter::new(SegmenterConfig::default());
    let mut out = Vec::new();
    let mut phrases = 0;
    for block in input.chunks(480) {
        out.clear();
        resampler.push(block, &mut out);
        phrases += segmenter.push(&out).len();
    }
    let took = started.elapsed();
    println!("60 s of audio processed in {took:?} ({:.2}% of one core), {phrases} phrases", took.as_secs_f64() / 60.0 * 100.0);
}
