//! Pure audio maths: mixing to mono, the level meter and resampling to the recogniser's rate.

use std::f64::consts::PI;

/// The sample rate every recogniser here expects.
pub const RECOGNISER_RATE: u32 = 16_000;

/// Averages interleaved frames of `channels` channels into one mono sample each.
pub fn mix_to_mono(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    if channels <= 1 {
        out.extend_from_slice(interleaved);
        return;
    }
    let scale = 1.0 / channels as f32;
    out.extend(interleaved.chunks_exact(channels).map(|frame| frame.iter().sum::<f32>() * scale));
}

/// Root mean square of a block of samples (0 for an empty block).
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Maps an RMS value to 0..1 on a decibel scale: -60 dBFS and below is 0, full scale is 1.
pub fn level_from_rms(rms: f32) -> f32 {
    const FLOOR_DB: f32 = -60.0;
    if rms <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * rms.log10();
    ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0)
}

/// Collects mono samples and reports one level per interval of audio.
pub struct LevelMeter {
    sum_squares: f64,
    count: usize,
    per_report: usize,
}

impl LevelMeter {
    pub fn new(sample_rate: u32, reports_per_second: u32) -> Self {
        let per_report = (sample_rate / reports_per_second.max(1)).max(1) as usize;
        Self { sum_squares: 0.0, count: 0, per_report }
    }

    /// Feeds samples; calls `report` once for every full interval collected.
    pub fn push(&mut self, samples: &[f32], mut report: impl FnMut(f32)) {
        for &s in samples {
            self.sum_squares += f64::from(s) * f64::from(s);
            self.count += 1;
            if self.count >= self.per_report {
                let rms = (self.sum_squares / self.count as f64).sqrt() as f32;
                report(level_from_rms(rms));
                self.sum_squares = 0.0;
                self.count = 0;
            }
        }
    }
}

const PHASES: usize = 256;
const LOBES: f64 = 8.0;

/// Streaming windowed-sinc resampler (mono, f32). Low-passes at the output Nyquist rate when
/// it decimates, so 48 kHz or 44.1 kHz input does not alias into the recogniser's band.
pub struct Resampler {
    passthrough: bool,
    step: f64,
    half: usize,
    /// `PHASES` rows of `2 * half` taps; row `p` serves a fractional position `p / PHASES`.
    table: Vec<f32>,
    buf: Vec<f32>,
    pos: f64,
}

impl Resampler {
    pub fn new(in_rate: u32, out_rate: u32) -> Self {
        let in_rate = in_rate.max(1);
        let out_rate = out_rate.max(1);
        let cutoff = (f64::from(out_rate) / f64::from(in_rate)).min(1.0);
        let half = (LOBES / cutoff).ceil() as usize;
        let mut table = Vec::with_capacity(PHASES * 2 * half);
        for p in 0..PHASES {
            let frac = p as f64 / PHASES as f64;
            let mut row = Vec::with_capacity(2 * half);
            for k in 0..2 * half {
                // Tap k reads the input sample at offset k - (half - 1) from the centre.
                let x = (k as f64 - (half as f64 - 1.0)) - frac;
                let arg = cutoff * x;
                let sinc = if arg.abs() < 1e-9 { 1.0 } else { (PI * arg).sin() / (PI * arg) };
                let window = (0.5 + 0.5 * (PI * x / half as f64).cos()).max(0.0);
                row.push(cutoff * sinc * window);
            }
            let sum: f64 = row.iter().sum();
            table.extend(row.iter().map(|v| (v / sum) as f32));
        }
        Self {
            passthrough: in_rate == out_rate,
            step: f64::from(in_rate) / f64::from(out_rate),
            half,
            table,
            buf: vec![0.0; half],
            pos: half as f64,
        }
    }

    /// Appends the resampled form of `input` to `out`.
    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) {
        if self.passthrough {
            out.extend_from_slice(input);
            return;
        }
        self.buf.extend_from_slice(input);
        let taps = 2 * self.half;
        loop {
            let centre = self.pos.floor() as usize;
            if centre + self.half >= self.buf.len() {
                break;
            }
            let phase = (((self.pos - centre as f64) * PHASES as f64) as usize).min(PHASES - 1);
            let row = &self.table[phase * taps..(phase + 1) * taps];
            let start = centre + 1 - self.half;
            let acc: f32 = row.iter().zip(&self.buf[start..start + taps]).map(|(h, x)| h * x).sum();
            out.push(acc);
            self.pos += self.step;
        }
        let drop = (self.pos.floor() as usize).saturating_sub(self.half);
        if drop > 0 {
            self.buf.drain(..drop);
            self.pos -= drop as f64;
        }
    }
}

/// Reads a RIFF/WAVE file (16-bit PCM, 32-bit PCM or 32-bit float) into mono samples and its
/// sample rate. Used by tests and tools that stand in for a microphone; it keeps no state.
pub fn read_wav(bytes: &[u8]) -> Result<(Vec<f32>, u32), String> {
    let u16_at = |at: usize| bytes.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let u32_at = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    if bytes.get(..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("not a WAVE file".into());
    }
    let (mut format, mut data) = (None, None);
    let mut at = 12;
    while let (Some(id), Some(size)) = (bytes.get(at..at + 4), u32_at(at + 4)) {
        let body = at + 8;
        let end = (body + size as usize).min(bytes.len());
        match id {
            b"fmt " => {
                let tag = u16_at(body).ok_or("short fmt chunk")?;
                let channels = u16_at(body + 2).ok_or("short fmt chunk")?;
                let rate = u32_at(body + 4).ok_or("short fmt chunk")?;
                let bits = u16_at(body + 14).ok_or("short fmt chunk")?;
                format = Some((tag, channels as usize, rate, bits));
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }
        at = body + size as usize + (size as usize & 1);
    }
    let ((tag, channels, rate, bits), data) = format.zip(data).ok_or("missing fmt or data chunk")?;
    if channels == 0 {
        return Err("zero channels".into());
    }
    let interleaved: Vec<f32> = match (tag, bits) {
        (1, 16) => data.as_chunks::<2>().0.iter().map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0).collect(),
        (1, 32) => data.as_chunks::<4>().0.iter().map(|b| (i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64 / 2_147_483_648.0) as f32).collect(),
        (3, 32) => data.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect(),
        _ => return Err(format!("unsupported WAVE format {tag} with {bits} bits")),
    };
    let mut mono = Vec::with_capacity(interleaved.len() / channels);
    mix_to_mono(&interleaved, channels, &mut mono);
    Ok((mono, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wave_file_is_read_to_mono() {
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(2u16.to_le_bytes()); // stereo
        wav.extend(22_050u32.to_le_bytes());
        wav.extend((22_050u32 * 4).to_le_bytes());
        wav.extend(4u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend(8u32.to_le_bytes());
        for s in [16384i16, 0, -16384, -16384] {
            wav.extend(s.to_le_bytes());
        }
        let (mono, rate) = read_wav(&wav).expect("valid wave");
        assert_eq!(rate, 22_050);
        assert_eq!(mono, vec![0.25, -0.5]);
        assert!(read_wav(b"nonsense").is_err());
    }

    fn sine(freq: f64, rate: u32, seconds: f64) -> Vec<f32> {
        (0..(f64::from(rate) * seconds) as usize)
            .map(|i| (2.0 * PI * freq * i as f64 / f64::from(rate)).sin() as f32 * 0.5)
            .collect()
    }

    #[test]
    fn stereo_is_averaged() {
        let mut out = Vec::new();
        mix_to_mono(&[1.0, 0.0, 0.5, 0.5], 2, &mut out);
        assert_eq!(out, vec![0.5, 0.5]);
    }

    #[test]
    fn level_runs_from_silence_to_full_scale() {
        assert_eq!(level_from_rms(0.0), 0.0);
        assert_eq!(level_from_rms(0.0001), 0.0);
        assert!((level_from_rms(1.0) - 1.0).abs() < 1e-6);
        assert!((level_from_rms(0.0316) - 0.5).abs() < 0.01, "-30 dBFS is half way");
    }

    #[test]
    fn meter_reports_on_interval() {
        let mut meter = LevelMeter::new(1000, 4);
        let mut reports = Vec::new();
        meter.push(&[0.5; 600], |l| reports.push(l));
        assert_eq!(reports.len(), 2, "250 samples per report");
        meter.push(&[0.5; 150], |l| reports.push(l));
        assert_eq!(reports.len(), 3);
        assert!(reports.iter().all(|l| (l - level_from_rms(0.5)).abs() < 1e-5));
    }

    #[test]
    fn resampling_keeps_length_and_tone() {
        let input = sine(440.0, 48_000, 1.0);
        let mut r = Resampler::new(48_000, 16_000);
        let mut out = Vec::new();
        // Feed in odd-sized blocks, as a device would.
        for block in input.chunks(441) {
            r.push(block, &mut out);
        }
        assert!((out.len() as i64 - 16_000).abs() < 40, "got {}", out.len());
        let settled = &out[200..out.len() - 200];
        let amp = rms(settled) * std::f32::consts::SQRT_2;
        assert!((amp - 0.5).abs() < 0.03, "amplitude {amp}");
    }

    #[test]
    fn resampling_44100_length() {
        let input = sine(300.0, 44_100, 2.0);
        let mut r = Resampler::new(44_100, 16_000);
        let mut out = Vec::new();
        r.push(&input, &mut out);
        assert!((out.len() as i64 - 32_000).abs() < 40, "got {}", out.len());
    }

    #[test]
    fn tones_above_the_output_band_are_removed() {
        // 12 kHz is above the 8 kHz Nyquist of the output: it must not alias back in.
        let input = sine(12_000.0, 48_000, 1.0);
        let mut r = Resampler::new(48_000, 16_000);
        let mut out = Vec::new();
        r.push(&input, &mut out);
        let settled = &out[200..out.len() - 200];
        assert!(rms(settled) < 0.02, "aliased energy {}", rms(settled));
    }

    #[test]
    fn same_rate_passes_through() {
        let mut r = Resampler::new(16_000, 16_000);
        let mut out = Vec::new();
        r.push(&[0.1, 0.2, 0.3], &mut out);
        assert_eq!(out, vec![0.1, 0.2, 0.3]);
    }
}
