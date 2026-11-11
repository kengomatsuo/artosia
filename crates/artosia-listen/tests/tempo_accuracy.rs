//! How close the tempo tracker gets on made-up music: five kinds of rhythm at ten tempos, each
//! with a little noise. No files, no downloads: the audio is built here. Run it with
//! `cargo test -p artosia-listen --test tempo_accuracy -- --nocapture` to see the table.
//!
//! A row passes when the reading at the end is within 2 BPM of what the tracker should say and
//! it held that reading from 8.5 s on. Up to `MAX_BPM` that is the tempo itself, and a doubled or
//! halved reading is a failure. Above it the tracker says half the tempo or nothing, never
//! another number: real slow songs were read at double speed until readings stopped at 132
//! (README, "Measured on real music").

use std::f32::consts::PI;

use artosia_listen::{MAX_BPM, TempoTracker};

const RATE: u32 = 48_000;
const SECONDS: f32 = 24.0;
const TEMPOS: [f32; 10] = [60.0, 72.0, 84.0, 96.0, 108.0, 120.0, 128.0, 140.0, 160.0, 176.0];
/// The tracker must settle on the truth by this many seconds and stay there.
const LOCK_BY: f32 = 8.5;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        ((self.0 >> 11) as f64 / (1u64 << 53) as f64) as f32 * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
enum Pattern {
    FourOnTheFloor,
    Backbeat,
    Syncopated,
    Strum,
    PadWithPulse,
}

const PATTERNS: [(Pattern, &str); 5] = [
    (Pattern::FourOnTheFloor, "four-on-floor"),
    (Pattern::Backbeat, "backbeat+hats"),
    (Pattern::Syncopated, "syncopated"),
    (Pattern::Strum, "guitar strum"),
    (Pattern::PadWithPulse, "pad, quiet pulse"),
];

fn kick(out: &mut [f32], at: usize, gain: f32) {
    for i in 0..(RATE as usize / 5) {
        let t = i as f32 / RATE as f32;
        // The pitch falls from 120 Hz to 50 Hz as a kick's does.
        let phase = 2.0 * PI * (50.0 * t + 70.0 * (1.0 - (-t * 40.0).exp()) / 40.0);
        if let Some(slot) = out.get_mut(at + i) {
            *slot += gain * 0.8 * phase.sin() * (-t * 22.0).exp();
        }
    }
}

fn snare(out: &mut [f32], at: usize, gain: f32, rng: &mut Rng) {
    for i in 0..(RATE as usize / 6) {
        let t = i as f32 / RATE as f32;
        let tone = (2.0 * PI * 190.0 * t).sin() * (-t * 35.0).exp();
        let burst = rng.next() * (-t * 28.0).exp();
        if let Some(slot) = out.get_mut(at + i) {
            *slot += gain * 0.4 * (0.5 * tone + burst);
        }
    }
}

fn hat(out: &mut [f32], at: usize, gain: f32, rng: &mut Rng) {
    let mut last = 0.0;
    for i in 0..(RATE as usize / 25) {
        let t = i as f32 / RATE as f32;
        let n = rng.next();
        let high = n - last;
        last = n;
        if let Some(slot) = out.get_mut(at + i) {
            *slot += gain * 0.4 * high * (-t * 150.0).exp();
        }
    }
}

/// One strum: six notes of an E chord, each a few harmonics with a soft attack, 9 ms apart.
fn strum(out: &mut [f32], at: usize, gain: f32, down: bool) {
    let notes = [82.41f32, 123.47, 164.81, 207.65, 246.94, 329.63];
    for (n, &f) in notes.iter().enumerate() {
        let order = if down { n } else { notes.len() - 1 - n };
        let start = at + order * (RATE as usize * 9 / 1000);
        for i in 0..(RATE as usize * 6 / 10) {
            let t = i as f32 / RATE as f32;
            let attack = (t / 0.018).min(1.0);
            let mut v = 0.0;
            for h in 1..=4 {
                v += (2.0 * PI * f * h as f32 * t).sin() / (h * h) as f32;
            }
            if let Some(slot) = out.get_mut(start + i) {
                *slot += gain * 0.12 * attack * v * (-t * 4.5).exp();
            }
        }
    }
}

/// A soft low tick: a 140 Hz bump with a 25 ms rise.
fn tick(out: &mut [f32], at: usize, gain: f32) {
    for i in 0..(RATE as usize * 3 / 10) {
        let t = i as f32 / RATE as f32;
        let env = (t / 0.025).min(1.0) * (-t * 14.0).exp();
        if let Some(slot) = out.get_mut(at + i) {
            *slot += gain * (2.0 * PI * 140.0 * t).sin() * env;
        }
    }
}

fn render(pattern: Pattern, bpm: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng(seed);
    let total = (SECONDS * RATE as f32) as usize;
    let mut out = vec![0.0f32; total];
    let beat = 60.0 / bpm;
    let at = |beats: f32| ((0.3 + beats * beat) * RATE as f32) as usize;
    let beats = ((SECONDS - 1.0) / beat) as usize;
    match pattern {
        Pattern::FourOnTheFloor => {
            for b in 0..beats {
                kick(&mut out, at(b as f32), 1.0);
            }
        }
        Pattern::Backbeat => {
            for b in 0..beats {
                if b % 2 == 0 {
                    kick(&mut out, at(b as f32), 1.0);
                } else {
                    snare(&mut out, at(b as f32), 1.0, &mut rng);
                }
                hat(&mut out, at(b as f32), 0.8, &mut rng);
                hat(&mut out, at(b as f32 + 0.5), 0.6, &mut rng);
            }
        }
        Pattern::Syncopated => {
            // Sixteenths in a bar of four beats: kicks on 0, 6, 10; snares on 4 and 13; hats on eighths.
            for bar in 0..=(beats / 4) {
                let base = (bar * 4) as f32;
                for (step, kind) in [(0.0, 'k'), (1.5, 'k'), (2.5, 'k'), (1.0, 's'), (3.25, 's')] {
                    let when = at(base + step);
                    if when + RATE as usize / 5 >= total {
                        continue;
                    }
                    if kind == 'k' {
                        kick(&mut out, when, if step == 0.0 { 1.0 } else { 0.8 });
                    } else {
                        snare(&mut out, when, 0.9, &mut rng);
                    }
                }
                for e in 0..8 {
                    hat(&mut out, at(base + e as f32 * 0.5), 0.5, &mut rng);
                }
            }
        }
        Pattern::Strum => {
            // Down on 1, 2, and-of-2, and-of-3... the classic D, D U, U D U shape over four beats.
            for b in 0..=(beats / 4) {
                let base = (b * 4) as f32;
                for (eighth, down, gain) in [(0, true, 1.0), (2, true, 0.7), (3, false, 0.5), (5, false, 0.5), (6, true, 0.8), (7, false, 0.5)] {
                    let when = at(base + eighth as f32 * 0.5);
                    if when + RATE as usize / 2 < total {
                        strum(&mut out, when, gain, down);
                    }
                }
            }
        }
        Pattern::PadWithPulse => {
            for (i, s) in out.iter_mut().enumerate() {
                let t = i as f32 / RATE as f32;
                let swell = 0.8 + 0.2 * (2.0 * PI * 0.13 * t).sin();
                for f in [220.0f32, 277.18, 329.63, 440.0] {
                    *s += 0.05 * swell * (2.0 * PI * f * t).sin();
                }
            }
            for b in 0..beats {
                tick(&mut out, at(b as f32), 0.03);
            }
        }
    }
    // A little room noise, about -46 dBFS rms.
    let mut low = 0.0;
    for s in out.iter_mut() {
        low = 0.7 * low + 0.3 * rng.next();
        *s += 0.005 * low * 1.8;
    }
    out
}

struct Row {
    detected: Option<f32>,
    locked_at: Option<f32>,
    held: bool,
    /// From `LOCK_BY` on, every reading that was made was right; some reports may be empty.
    never_wrong: bool,
}

fn measure(signal: &[f32], truth: f32) -> Row {
    let mut tracker = TempoTracker::new(RATE);
    let mut reports: Vec<(f32, Option<f32>)> = Vec::new();
    let mut fed = 0usize;
    for block in signal.chunks(441) {
        fed += block.len();
        tracker.push(block, |t| reports.push((fed as f32 / RATE as f32, t.map(|t| t.bpm))));
    }
    let right = |r: &(f32, Option<f32>)| r.1.is_some_and(|b| (b - truth).abs() <= 2.0);
    // The first report after which every report is right.
    let locked_at = (0..reports.len()).find(|&i| reports[i..].iter().all(right)).map(|i| reports[i].0);
    let held = reports.iter().filter(|r| r.0 >= LOCK_BY).all(right);
    let never_wrong = reports.iter().filter(|r| r.0 >= LOCK_BY).all(|r| r.1.is_none() || right(r));
    Row { detected: reports.last().and_then(|r| r.1), locked_at, held, never_wrong }
}

#[test]
fn tempo_accuracy_table() {
    println!("{:<18} {:>6} {:>9} {:>8}  result", "pattern", "true", "detected", "locked");
    let mut failures = Vec::new();
    // The first seed's table is printed; two more noise seeds must pass as well, so the
    // detector is not tuned to one draw of the noise.
    for (round, seed) in [0x9E37_79B9u64, 0x1234_5678, 0x0BAD_CAFE].into_iter().enumerate() {
        for (pattern, name) in PATTERNS {
            for bpm in TEMPOS {
                let signal = render(pattern, bpm, seed + bpm as u64);
                let said = if bpm > MAX_BPM { bpm / 2.0 } else { bpm };
                let row = measure(&signal, said);
                let ok = match bpm > MAX_BPM {
                    false => row.held && row.detected.is_some_and(|d| (d - said).abs() <= 2.0) && row.locked_at.is_some_and(|t| t <= LOCK_BY),
                    // Half the tempo is a slower pulse to find, and a syncopated bar does not
                    // repeat at it: no reading is allowed there, a wrong one is not.
                    true => row.never_wrong,
                };
                if round == 0 {
                    println!(
                        "{name:<18} {bpm:>6.0} {:>9} {:>8}  {}",
                        row.detected.map_or("none".to_string(), |d| format!("{d:.1}")),
                        row.locked_at.map_or("never".to_string(), |t| format!("{t:.1} s")),
                        if ok { "ok" } else { "FAIL" }
                    );
                }
                if !ok {
                    failures.push(format!("{name} at {bpm} (seed {round}): read {:?}, right from {:?}", row.detected, row.locked_at));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{} rows failed: {failures:?}", failures.len());
}
