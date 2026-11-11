//! Tempo tracking from the same mono samples the level meter gets.
//!
//! Pipeline (the README holds the sources for each step):
//!
//! 1. Average down to about 8 kHz. Onsets live below 4 kHz and this makes every later step cheap.
//! 2. A 256-point FFT every 5 ms, 24 mel-spaced bands from 60 Hz to 3.8 kHz, log-compressed.
//! 3. Spectral flux with a maximum filter over the neighbouring bands of the frame two hops back
//!    (SuperFlux): the rise in energy, with the slow glide of a sung note cancelled out.
//! 4. Every half second, autocorrelation of the last 6 s of that onset envelope. Peaks at one,
//!    two and three beat periods are added (as aubio does) and the sum is weighted by a log-normal
//!    window around 120 BPM (Ellis 2007, librosa), which bounds the half-time and double-time
//!    mistakes.
//! 5. Beat phase: the offset at which a pulse train at that period collects the most envelope.
//!
//! No model, no dependency, and the whole of it is one FFT size and a few hundred multiplies per
//! envelope frame.

use std::collections::VecDeque;
use std::f32::consts::PI;

/// Slowest and fastest tempo reported.
pub const MIN_BPM: f32 = 50.0;
pub const MAX_BPM: f32 = 200.0;

const TARGET_RATE: f32 = 8000.0;
const FRAME: usize = 256;
const HOP_SECONDS: f32 = 0.005;
const BANDS: usize = 24;
const LOW_HZ: f32 = 60.0;
const HIGH_HZ: f32 = 3800.0;
/// Log compression gain, as `log10(1 + MUL * magnitude)` (Böck's SuperFlux uses `mul = 1, add = 1`
/// on unnormalised magnitudes; this one is on magnitudes where a full-scale sine reads 1).
const MUL: f32 = 100.0;
const DIFF_FRAMES: usize = 2;
const WINDOW_SECONDS: f32 = 6.0;
/// The earliest an estimate is made. A tempo settles about one report after this.
const MIN_SECONDS: f32 = 3.0;
const REPORT_SECONDS: f32 = 0.5;
const PRIOR_BPM: f32 = 120.0;
const PRIOR_OCTAVES: f32 = 0.9;
/// Below this RMS over the window (-70 dBFS) nothing is playing.
const SILENCE_RMS: f32 = 3e-4;
/// A raw estimate starts a report at this confidence and keeps one going at the lower figure.
const ENTER: f32 = 0.28;
const KEEP: f32 = 0.15;
/// Two raw estimates agree when their periods differ by less than this fraction.
const AGREE: f32 = 0.04;
/// Reports in a row without a usable estimate before the tracker goes quiet.
const MISSES: u32 = 4;
/// How long after a click's start the envelope peaks: half a frame, found with the click-track test.
const ONSET_DELAY: f32 = 0.016;

/// What the tracker believes about the beat. Time is on the caller's clock: `since_beat` is
/// how many seconds before the end of the audio fed so far the last beat fell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    /// Beats per minute, `MIN_BPM` to `MAX_BPM`.
    pub bpm: f32,
    /// 0 to 1: how periodic the room sounds. Music with a steady pulse reads above 0.5.
    pub confidence: f32,
    /// Seconds from one beat to the next.
    pub period: f32,
    /// Seconds since the last beat, 0 up to `period`.
    pub since_beat: f32,
}

impl Tempo {
    /// Where in the beat it is `after` seconds later than the report: 0 on a beat, up to 1.
    pub fn phase_after(&self, after: f32) -> f32 {
        ((self.since_beat + after) / self.period).rem_euclid(1.0)
    }
}

struct Fft {
    cos: Vec<f32>,
    sin: Vec<f32>,
    rev: Vec<usize>,
}

impl Fft {
    fn new(n: usize) -> Self {
        let bits = n.trailing_zeros();
        let rev = (0..n).map(|i| i.reverse_bits() >> (usize::BITS - bits)).collect();
        let cos = (0..n / 2).map(|k| (2.0 * PI * k as f32 / n as f32).cos()).collect();
        let sin = (0..n / 2).map(|k| -(2.0 * PI * k as f32 / n as f32).sin()).collect();
        Self { cos, sin, rev }
    }

    fn forward(&self, re: &mut [f32], im: &mut [f32]) {
        let n = re.len();
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let step = n / len;
            for start in (0..n).step_by(len) {
                for k in 0..len / 2 {
                    let (wr, wi) = (self.cos[k * step], self.sin[k * step]);
                    let (a, b) = (start + k, start + k + len / 2);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len *= 2;
        }
    }
}

/// What one analysis of the window found.
struct Raw {
    period: f32,
    confidence: f32,
    /// Frames between the newest envelope frame and the last beat.
    age: f32,
}

/// The grid the reports are read from: when a beat fell (in envelope frames since the start) and
/// how long a beat is.
#[derive(Clone, Copy)]
struct Grid {
    beat_at: f32,
    period: f32,
    confidence: f32,
}

pub struct TempoTracker {
    factor: usize,
    sum: f32,
    count: usize,
    hop: usize,
    hop_seconds: f32,
    ring: Vec<f32>,
    ring_at: usize,
    since_frame: usize,
    started: bool,
    window: Vec<f32>,
    window_sum: f32,
    fft: Fft,
    re: Vec<f32>,
    im: Vec<f32>,
    bands: Vec<(usize, usize)>,
    logs: VecDeque<[f32; BANDS]>,
    env: VecDeque<f32>,
    energy: VecDeque<f32>,
    frame_energy: f32,
    frames: u64,
    report_every: u64,
    previous: Option<Raw>,
    grid: Option<Grid>,
    misses: u32,
}

impl TempoTracker {
    pub fn new(sample_rate: u32) -> Self {
        let rate_in = sample_rate.max(8000) as f32;
        let factor = (rate_in / TARGET_RATE).round().max(1.0) as usize;
        let rate = rate_in / factor as f32;
        let hop = ((rate * HOP_SECONDS).round() as usize).max(1);
        let hop_seconds = hop as f32 / rate;
        let window = (0..FRAME).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / FRAME as f32).cos()).collect::<Vec<f32>>();
        let window_sum = window.iter().sum();
        let mel = |f: f32| 2595.0 * (1.0 + f / 700.0).log10();
        let unmel = |m: f32| 700.0 * (10f32.powf(m / 2595.0) - 1.0);
        let high = HIGH_HZ.min(rate / 2.0 - 1.0);
        let (m0, m1) = (mel(LOW_HZ), mel(high));
        let bin_hz = rate / FRAME as f32;
        let mut bands = Vec::with_capacity(BANDS);
        let mut last_end = (LOW_HZ / bin_hz).round() as usize;
        for b in 0..BANDS {
            let edge = unmel(m0 + (m1 - m0) * (b + 1) as f32 / BANDS as f32);
            let start = last_end.max(1);
            let end = ((edge / bin_hz).round() as usize).max(start + 1).min(FRAME / 2);
            bands.push((start.min(FRAME / 2 - 1), end));
            last_end = end;
        }
        Self {
            factor,
            sum: 0.0,
            count: 0,
            hop,
            hop_seconds,
            ring: vec![0.0; FRAME],
            ring_at: 0,
            since_frame: 0,
            started: false,
            window,
            window_sum,
            fft: Fft::new(FRAME),
            re: vec![0.0; FRAME],
            im: vec![0.0; FRAME],
            bands,
            logs: VecDeque::new(),
            env: VecDeque::new(),
            energy: VecDeque::new(),
            frame_energy: 0.0,
            frames: 0,
            report_every: (REPORT_SECONDS / hop_seconds).round() as u64,
            previous: None,
            grid: None,
            misses: 0,
        }
    }

    /// Feeds mono samples at the rate given to `new`. Calls `report` every half second of audio with
    /// what the tracker now believes, `None` while it hears no steady pulse.
    pub fn push(&mut self, samples: &[f32], mut report: impl FnMut(Option<Tempo>)) {
        for &s in samples {
            self.sum += s;
            self.count += 1;
            if self.count < self.factor {
                continue;
            }
            let value = self.sum / self.count as f32;
            self.sum = 0.0;
            self.count = 0;
            self.ring[self.ring_at] = value;
            self.ring_at = (self.ring_at + 1) % FRAME;
            self.frame_energy += value * value;
            self.since_frame += 1;
            let ready = if self.started { self.since_frame >= self.hop } else { self.since_frame >= FRAME };
            if ready {
                self.started = true;
                self.since_frame = 0;
                self.frame();
                if self.frames.is_multiple_of(self.report_every) {
                    self.estimate_and_report(&mut report);
                }
            }
        }
    }

    /// The belief as of the audio fed so far.
    pub fn current(&self) -> Option<Tempo> {
        self.grid.map(|g| self.tempo_of(g))
    }

    fn tempo_of(&self, g: Grid) -> Tempo {
        let now = self.frames as f32;
        let period = g.period * self.hop_seconds;
        let since = ((now - g.beat_at) * self.hop_seconds + ONSET_DELAY).rem_euclid(period);
        Tempo { bpm: 60.0 / period, confidence: g.confidence, period, since_beat: since }
    }

    /// One 5 ms step: spectrum, log bands, flux.
    fn frame(&mut self) {
        for i in 0..FRAME {
            self.re[i] = self.ring[(self.ring_at + i) % FRAME] * self.window[i];
            self.im[i] = 0.0;
        }
        self.fft.forward(&mut self.re, &mut self.im);
        let scale = 2.0 / self.window_sum;
        let mut log = [0.0f32; BANDS];
        for (b, &(start, end)) in self.bands.iter().enumerate() {
            let mut sum = 0.0;
            for k in start..end {
                sum += (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt();
            }
            log[b] = (1.0 + MUL * scale * sum).log10();
        }
        let mut flux = 0.0;
        if self.logs.len() > DIFF_FRAMES - 1 {
            let before = &self.logs[self.logs.len() + 1 - DIFF_FRAMES];
            for (b, &now) in log.iter().enumerate() {
                let lo = b.saturating_sub(1);
                let hi = (b + 1).min(BANDS - 1);
                let reference = before[lo..=hi].iter().fold(f32::MIN, |m, v| m.max(*v));
                flux += (now - reference).max(0.0);
            }
        }
        self.logs.push_back(log);
        while self.logs.len() > DIFF_FRAMES {
            self.logs.pop_front();
        }
        self.frames += 1;
        let capacity = (WINDOW_SECONDS / self.hop_seconds) as usize;
        self.env.push_back(flux);
        self.energy.push_back(self.frame_energy / self.hop.max(1) as f32);
        self.frame_energy = 0.0;
        while self.env.len() > capacity {
            self.env.pop_front();
            self.energy.pop_front();
        }
    }

    fn estimate_and_report(&mut self, report: &mut impl FnMut(Option<Tempo>)) {
        let raw = self.estimate();
        let usable = |r: &Raw, floor: f32| r.confidence >= floor;
        let near = |a: f32, b: f32| ((a - b) / b).abs() < AGREE;
        let now = self.frames as f32;
        let mut accepted: Option<Grid> = None;
        if let Some(r) = &raw {
            let grid = Grid { beat_at: now - r.age, period: r.period, confidence: r.confidence };
            let agrees_before = self.previous.as_ref().is_some_and(|p| usable(p, ENTER) && near(p.period, r.period));
            let agrees_grid = self.grid.is_some_and(|g| near(g.period, r.period));
            if usable(r, ENTER) && (agrees_before || agrees_grid) || usable(r, KEEP) && agrees_grid {
                accepted = Some(grid);
            }
        }
        self.previous = raw;
        match accepted {
            Some(grid) => {
                self.misses = 0;
                self.grid = Some(grid);
            }
            None => {
                self.misses += 1;
                if self.misses >= MISSES {
                    self.grid = None;
                }
            }
        }
        report(self.current());
    }

    /// Autocorrelation tempo and comb phase over the window.
    fn estimate(&self) -> Option<Raw> {
        let n = self.env.len();
        if (n as f32) * self.hop_seconds < MIN_SECONDS {
            return None;
        }
        let rms = (self.energy.iter().sum::<f32>() / n as f32).sqrt();
        if rms < SILENCE_RMS {
            return None;
        }
        let mean = self.env.iter().sum::<f32>() / n as f32;
        let x: Vec<f32> = self.env.iter().map(|v| v - mean).collect();
        let ac0: f32 = x.iter().map(|v| v * v).sum();
        if ac0 <= 1e-9 {
            return None;
        }
        let lag_of = |bpm: f32| 60.0 / (bpm * self.hop_seconds);
        let lo = lag_of(MAX_BPM).floor() as usize - 1;
        let hi = lag_of(MIN_BPM).ceil() as usize + 1;
        let top = (3 * hi).min(n - 1);
        let mut ac = vec![0.0f32; top + 1];
        let mut needed = vec![false; top + 1];
        for l in lo..=hi {
            for a in 1..=3 {
                if a * l <= top {
                    needed[a * l] = true;
                }
            }
        }
        for lag in 0..=top {
            if needed[lag] {
                let sum: f32 = x[lag..].iter().zip(&x[..n - lag]).map(|(a, b)| a * b).sum();
                ac[lag] = sum / ac0;
            }
        }
        let weights = [1.0, 1.0 / 3.0, 1.0 / 5.0];
        let score = |l: usize| -> f32 { (1..=3).filter(|a| a * l <= top).map(|a| weights[a - 1] * ac[a * l]).sum::<f32>().max(0.0) };
        let prior = |l: f32| -> f32 {
            let bpm = 60.0 / (l * self.hop_seconds);
            let octaves = (bpm / PRIOR_BPM).log2() / PRIOR_OCTAVES;
            (-0.5 * octaves * octaves).exp()
        };
        let mut best = (0.0f32, 0usize);
        for l in lo + 1..hi {
            let value = score(l) * prior(l as f32);
            if value > best.0 {
                best = (value, l);
            }
        }
        if best.1 == 0 {
            return None;
        }
        let l = best.1;
        let (a, b, c) = (score(l - 1), score(l), score(l + 1));
        let denom = a - 2.0 * b + c;
        let shift = if denom < -1e-9 { (0.5 * (a - c) / denom).clamp(-0.5, 0.5) } else { 0.0 };
        let period = l as f32 + shift;
        let peak = |centre: f32, reach: usize| -> f32 {
            let at = centre.round() as usize;
            (at.saturating_sub(reach)..=(at + reach).min(top)).map(|i| ac[i]).fold(0.0, f32::max)
        };
        let c1 = peak(period, 1);
        let c2 = if 2 * (l + 1) <= top { peak(2.0 * period, 2) } else { 0.0 };
        let confidence = ((c1 * c2).sqrt() - 0.08).clamp(0.0, 1.0);

        // Phase: smooth the envelope a little, then slide a pulse train over it.
        let smooth: Vec<f32> = (0..n)
            .map(|i| {
                let left = x[i.saturating_sub(1)].max(0.0);
                let right = x[(i + 1).min(n - 1)].max(0.0);
                0.25 * left + 0.5 * x[i].max(0.0) + 0.25 * right
            })
            .collect();
        let mut best_phase = (f32::MIN, 0.0f32);
        let mut phase = 0.0;
        while phase < period {
            let mut total = 0.0;
            let mut weight = 1.0;
            let mut k = 0.0;
            loop {
                let back = (phase + k * period).round() as usize;
                if back >= n {
                    break;
                }
                total += weight * smooth[n - 1 - back];
                weight *= 0.9;
                k += 1.0;
            }
            if total > best_phase.0 {
                best_phase = (total, phase);
            }
            phase += 1.0;
        }
        Some(Raw { period, confidence, age: best_phase.1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic noise source.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 11) as f64 / (1u64 << 53) as f64) as f32 * 2.0 - 1.0
        }
    }

    const RATE: u32 = 48_000;

    /// A drum-like hit: a decaying 90 Hz thump and a short noise burst.
    fn hit(out: &mut [f32], at: usize, gain: f32, rng: &mut Rng) {
        for i in 0..(RATE as usize / 10) {
            let t = i as f32 / RATE as f32;
            let thump = (2.0 * PI * 90.0 * t).sin() * (-t * 30.0).exp();
            let burst = rng.next() * (-t * 120.0).exp() * 0.8;
            if let Some(slot) = out.get_mut(at + i) {
                *slot += gain * 0.5 * (thump + burst);
            }
        }
    }

    /// Hits on a tempo that may change: `(from second, bpm)` pairs. Returns the audio and the
    /// beat times in seconds.
    fn track(seconds: f32, tempos: &[(f32, f32)], accent: Option<f32>, seed: u64) -> (Vec<f32>, Vec<f32>) {
        let mut rng = Rng(seed);
        let mut out = vec![0.0f32; (seconds * RATE as f32) as usize];
        let mut beats = Vec::new();
        let mut t = 0.4;
        let mut count = 0;
        while t < seconds - 0.2 {
            let gain = match accent {
                Some(soft) if count % 2 == 1 => soft,
                _ => 1.0,
            };
            hit(&mut out, (t * RATE as f32) as usize, gain, &mut rng);
            beats.push(t);
            let bpm = tempos.iter().rev().find(|(from, _)| t >= *from).map_or(tempos[0].1, |(_, b)| *b);
            t += 60.0 / bpm;
            count += 1;
        }
        (out, beats)
    }

    fn noise(out: &mut [f32], level: f32, seed: u64) {
        let mut rng = Rng(seed);
        let mut low = 0.0;
        for s in out.iter_mut() {
            low = 0.7 * low + 0.3 * rng.next();
            *s += level * low * 1.8;
        }
    }

    /// A Schroeder reverberator: four combs and two all-passes, a tail of about 1.5 s.
    fn reverb(input: &[f32], wet: f32) -> Vec<f32> {
        let delays = [1687usize, 1601, 2053, 2251].map(|d| d * 3);
        let feedback = 0.89f32;
        let mut out = vec![0.0f32; input.len()];
        for d in delays {
            let mut line = vec![0.0f32; d];
            let mut at = 0;
            for (i, &x) in input.iter().enumerate() {
                let y = line[at];
                line[at] = x + feedback * y;
                at = (at + 1) % d;
                out[i] += y * 0.25;
            }
        }
        for d in [556usize * 3, 441 * 3] {
            let mut line = vec![0.0f32; d];
            let mut at = 0;
            for v in out.iter_mut() {
                let buffered = line[at];
                let y = -*v + buffered;
                line[at] = *v + 0.5 * buffered;
                at = (at + 1) % d;
                *v = y;
            }
        }
        input.iter().zip(&out).map(|(dry, w)| dry + wet * w).collect()
    }

    /// Runs the tracker over `signal` in 441-sample blocks. Returns `(seconds, report)`.
    fn run(signal: &[f32]) -> Vec<(f32, Option<Tempo>)> {
        let mut tracker = TempoTracker::new(RATE);
        let mut reports = Vec::new();
        let mut fed = 0usize;
        for block in signal.chunks(441) {
            fed += block.len();
            tracker.push(block, |t| reports.push((fed as f32 / RATE as f32, t)));
        }
        reports
    }

    fn settled(reports: &[(f32, Option<Tempo>)], from: f32) -> Vec<Tempo> {
        reports.iter().filter(|(t, _)| *t >= from).filter_map(|(_, r)| *r).collect()
    }

    /// Every report from `from` seconds on carries a tempo within `tolerance` BPM of `bpm`.
    fn assert_tempo(reports: &[(f32, Option<Tempo>)], from: f32, bpm: f32, tolerance: f32) {
        let later: Vec<_> = reports.iter().filter(|(t, _)| *t >= from).collect();
        assert!(!later.is_empty());
        for (t, r) in later {
            let tempo = r.unwrap_or_else(|| panic!("nothing reported at {t:.1} s, wanted {bpm}"));
            assert!((tempo.bpm - bpm).abs() <= tolerance, "at {t:.1} s: {:.1} BPM (confidence {:.2}), wanted {bpm}", tempo.bpm, tempo.confidence);
        }
    }

    #[test]
    fn click_tracks_are_found_within_two_bpm() {
        for bpm in [60.0, 90.0, 120.0, 150.0] {
            let (audio, _) = track(30.0, &[(0.0, bpm)], None, 7);
            let reports = run(&audio);
            assert_tempo(&reports, 8.0, bpm, 2.0);
        }
    }

    #[test]
    fn a_tempo_settles_within_a_few_seconds() {
        for bpm in [60.0, 90.0, 120.0, 150.0] {
            let (audio, _) = track(15.0, &[(0.0, bpm)], None, 11);
            let reports = run(&audio);
            let first = reports.iter().find(|(_, r)| r.is_some_and(|t| (t.bpm - bpm).abs() <= 2.0)).map(|(t, _)| *t);
            assert!(first.is_some_and(|t| t <= 5.0), "{bpm} BPM first right at {first:?}");
        }
    }

    #[test]
    fn the_beat_time_falls_on_the_hits() {
        let (audio, beats) = track(20.0, &[(0.0, 100.0)], None, 3);
        let mut tracker = TempoTracker::new(RATE);
        let mut worst = 0.0f32;
        let mut checked = 0;
        let mut fed = 0usize;
        for block in audio.chunks(441) {
            fed += block.len();
            let now = fed as f32 / RATE as f32;
            tracker.push(block, |t| {
                if let (Some(t), true) = (t, now > 8.0) {
                    let last = now - t.since_beat;
                    let nearest = beats.iter().map(|b| (b - last).abs()).fold(f32::MAX, f32::min);
                    worst = worst.max(nearest);
                    checked += 1;
                }
            });
        }
        assert!(checked > 10);
        assert!(worst < 0.03, "the reported beat was {worst:.3} s from a hit");
    }

    #[test]
    fn clicks_in_noise_are_found() {
        for bpm in [75.0, 120.0] {
            let (mut audio, _) = track(30.0, &[(0.0, bpm)], None, 5);
            // Noise at the level of the hits' own average loudness.
            noise(&mut audio, 0.12, 99);
            assert_tempo(&run(&audio), 10.0, bpm, 2.0);
        }
    }

    #[test]
    fn clicks_in_a_reverberant_hall_are_found() {
        for bpm in [70.0, 110.0] {
            let (audio, _) = track(30.0, &[(0.0, bpm)], None, 13);
            let wet = reverb(&audio, 1.5);
            assert_tempo(&run(&wet), 10.0, bpm, 2.0);
        }
    }

    #[test]
    fn a_change_of_tempo_is_followed() {
        let (audio, _) = track(40.0, &[(0.0, 90.0), (20.0, 130.0)], None, 17);
        let reports = run(&audio);
        assert_tempo(&reports[..reports.iter().position(|(t, _)| *t > 19.5).unwrap()], 10.0, 90.0, 2.0);
        // The window is six seconds long: the new tempo is read within ten seconds of the change.
        assert_tempo(&reports, 30.0, 130.0, 2.0);
    }

    #[test]
    fn silence_and_steady_noise_report_nothing() {
        let silent = vec![0.0f32; RATE as usize * 20];
        assert!(settled(&run(&silent), 0.0).is_empty());
        let mut hiss = vec![0.0f32; RATE as usize * 30];
        noise(&mut hiss, 0.1, 21);
        assert!(settled(&run(&hiss), 0.0).is_empty(), "steady noise found a tempo");
        let mut quiet = vec![0.0f32; RATE as usize * 20];
        noise(&mut quiet, 0.00005, 22);
        assert!(settled(&run(&quiet), 0.0).is_empty());
    }

    /// Bursts of noise with the rhythm of talking: syllables of 120 to 250 ms, gaps of 30 to
    /// 300 ms, a longer pause now and then.
    fn speech_like(seconds: f32, seed: u64) -> Vec<f32> {
        let mut rng = Rng(seed);
        let mut out = vec![0.0f32; (seconds * RATE as f32) as usize];
        let mut t = 0.2;
        let mut formant = 0.0f32;
        while t < seconds - 0.4 {
            let length = 0.12 + 0.13 * (rng.next() * 0.5 + 0.5);
            let samples = (length * RATE as f32) as usize;
            let start = (t * RATE as f32) as usize;
            for i in 0..samples.min(out.len() - start) {
                let shape = (PI * i as f32 / samples as f32).sin();
                formant = 0.8 * formant + 0.2 * rng.next();
                let voiced = (2.0 * PI * 140.0 * i as f32 / RATE as f32).sin();
                out[start + i] += 0.25 * shape * (0.6 * voiced + 2.0 * formant);
            }
            t += length + 0.03 + 0.27 * (rng.next() * 0.5 + 0.5);
            if rng.next() > 0.7 {
                t += 0.5;
            }
        }
        out
    }

    #[test]
    fn plain_speech_reports_nothing() {
        for seed in [1, 2, 3, 4, 5, 6] {
            let audio = speech_like(40.0, seed * 1234567);
            let found = settled(&run(&audio), 0.0);
            assert!(found.is_empty(), "seed {seed}: speech read as {:.0} BPM at {:.2}", found[0].bpm, found[0].confidence);
        }
    }

    #[test]
    fn half_time_and_double_time_mistakes_stay_bounded() {
        // Hits that alternate loud and soft invite a reading at half the tempo (one loud hit per
        // two beats). Whatever the tracker says must be the true tempo or half of it, never any
        // other number.
        for bpm in [80.0, 100.0, 140.0, 170.0] {
            for soft in [0.5, 0.25] {
                let (audio, _) = track(30.0, &[(0.0, bpm)], Some(soft), 29);
                for tempo in settled(&run(&audio), 10.0) {
                    let ok = (tempo.bpm - bpm).abs() <= 2.0 || (tempo.bpm - bpm / 2.0).abs() <= 2.0 || (tempo.bpm - bpm * 2.0).abs() <= 2.0;
                    assert!(ok, "{bpm} BPM with soft hits at {soft} read as {:.1}", tempo.bpm);
                }
            }
        }
    }

    #[test]
    fn it_works_at_44100_hz() {
        let (audio, _) = track(20.0, &[(0.0, 100.0)], None, 41);
        // Resample by picking samples: close enough for a rhythm test.
        let resampled: Vec<f32> = (0..(audio.len() as f32 * 44_100.0 / 48_000.0) as usize).map(|i| audio[(i as f32 * 48_000.0 / 44_100.0) as usize]).collect();
        let mut tracker = TempoTracker::new(44_100);
        let mut last = None;
        for block in resampled.chunks(441) {
            tracker.push(block, |t| last = t);
        }
        let tempo = last.expect("a tempo at 44.1 kHz");
        assert!((tempo.bpm - 100.0).abs() <= 2.0, "{:.1}", tempo.bpm);
    }
}
