//! Cuts a continuous 16 kHz stream into phrases a recogniser can read, and keeps a slow CPU
//! from drowning in work: only speech is kept, progress reads are throttled, long runs are split.

use crate::audio::{RECOGNISER_RATE, rms};

/// A stretch of audio to read.
#[derive(Debug, Clone, PartialEq)]
pub struct Phrase {
    pub samples: Vec<f32>,
    /// False for a progress read of a phrase still going on, true when it ended.
    pub is_final: bool,
    /// The number of the wave file the listener kept this phrase in, when it keeps them.
    pub kept: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SegmenterConfig {
    /// Frame length used for the loudness decisions.
    pub frame_ms: u32,
    /// Absolute RMS below which a frame is never speech.
    pub min_speech_rms: f32,
    /// A frame is speech when its RMS exceeds the noise floor times this.
    pub floor_factor: f32,
    /// Speech must last this long before a phrase opens (ignores clicks).
    pub onset_ms: u32,
    /// Silence this long closes the phrase.
    pub hangover_ms: u32,
    /// Audio kept from before the onset so the first word is not clipped.
    pub preroll_ms: u32,
    /// A phrase longer than this is closed anyway (singing has no pauses).
    pub max_phrase_ms: u32,
    /// Emit a progress read after this much new speech.
    pub partial_every_ms: u32,
    /// Phrases shorter than this are dropped.
    pub min_phrase_ms: u32,
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self {
            frame_ms: 20,
            min_speech_rms: 0.006,
            floor_factor: 3.0,
            onset_ms: 100,
            hangover_ms: 700,
            preroll_ms: 300,
            max_phrase_ms: 12_000,
            partial_every_ms: 2_000,
            min_phrase_ms: 500,
        }
    }
}

pub struct Segmenter {
    cfg: SegmenterConfig,
    frame_len: usize,
    pending: Vec<f32>,
    floor: f32,
    /// Recent audio before a phrase opens.
    preroll: Vec<f32>,
    onset_frames: u32,
    /// The open phrase, if any.
    phrase: Vec<f32>,
    open: bool,
    silent_frames: u32,
    samples_since_partial: usize,
}

fn ms_to_samples(ms: u32) -> usize {
    (RECOGNISER_RATE as usize * ms as usize) / 1000
}

impl Segmenter {
    pub fn new(cfg: SegmenterConfig) -> Self {
        let frame_len = ms_to_samples(cfg.frame_ms).max(1);
        Self {
            cfg,
            frame_len,
            pending: Vec::new(),
            floor: 0.002,
            preroll: Vec::new(),
            onset_frames: 0,
            phrase: Vec::new(),
            open: false,
            silent_frames: 0,
            samples_since_partial: 0,
        }
    }

    /// True while a phrase is open.
    pub fn in_phrase(&self) -> bool {
        self.open
    }

    pub fn push(&mut self, samples: &[f32]) -> Vec<Phrase> {
        let mut out = Vec::new();
        self.pending.extend_from_slice(samples);
        let mut start = 0;
        while self.pending.len() - start >= self.frame_len {
            let frame: Vec<f32> = self.pending[start..start + self.frame_len].to_vec();
            start += self.frame_len;
            self.frame(&frame, &mut out);
        }
        self.pending.drain(..start);
        out
    }

    /// Closes the open phrase, if any (call when listening stops).
    pub fn flush(&mut self) -> Option<Phrase> {
        self.close()
    }

    fn frame(&mut self, frame: &[f32], out: &mut Vec<Phrase>) {
        let level = rms(frame);
        let threshold = (self.floor * self.cfg.floor_factor).max(self.cfg.min_speech_rms);
        let speech = level > threshold;
        if !speech && !self.open {
            // Track the noise floor only outside phrases: quickly down, slowly up.
            self.floor = if level < self.floor {
                0.5 * self.floor + 0.5 * level
            } else {
                self.floor + (level - self.floor) * 0.002
            };
        }

        if self.open {
            self.phrase.extend_from_slice(frame);
            self.samples_since_partial += frame.len();
            if speech {
                self.silent_frames = 0;
            } else {
                self.silent_frames += 1;
            }
            let hangover = self.cfg.hangover_ms / self.cfg.frame_ms.max(1);
            if self.silent_frames >= hangover
                || self.phrase.len() >= ms_to_samples(self.cfg.max_phrase_ms)
            {
                out.extend(self.close());
            } else if self.samples_since_partial >= ms_to_samples(self.cfg.partial_every_ms) {
                self.samples_since_partial = 0;
                out.push(Phrase { samples: self.phrase.clone(), is_final: false, kept: None });
            }
            return;
        }

        // Closed: keep a rolling pre-roll and count speech frames towards the onset.
        self.preroll.extend_from_slice(frame);
        let keep = ms_to_samples(self.cfg.preroll_ms + self.cfg.onset_ms);
        if self.preroll.len() > keep {
            let excess = self.preroll.len() - keep;
            self.preroll.drain(..excess);
        }
        if speech {
            self.onset_frames += 1;
            if self.onset_frames * self.cfg.frame_ms >= self.cfg.onset_ms {
                self.open = true;
                self.silent_frames = 0;
                self.samples_since_partial = 0;
                self.phrase = std::mem::take(&mut self.preroll);
                self.onset_frames = 0;
            }
        } else {
            self.onset_frames = 0;
        }
    }

    fn close(&mut self) -> Option<Phrase> {
        if !self.open {
            return None;
        }
        self.open = false;
        self.onset_frames = 0;
        self.samples_since_partial = 0;
        let mut samples = std::mem::take(&mut self.phrase);
        // Trim the trailing silence down to a short tail.
        let tail = ms_to_samples(200);
        let trim = (self.silent_frames as usize * self.frame_len).saturating_sub(tail);
        samples.truncate(samples.len().saturating_sub(trim));
        self.silent_frames = 0;
        (samples.len() >= ms_to_samples(self.cfg.min_phrase_ms))
            .then_some(Phrase { samples, is_final: true, kept: None })
    }
}

/// Drops work that a newer job makes pointless: a progress read before a final is stale, and of
/// several progress reads only the last is worth the CPU. Finals always stay, in order.
pub fn coalesce(jobs: Vec<Phrase>) -> Vec<Phrase> {
    let last_final = jobs.iter().rposition(|j| j.is_final);
    let last_partial = jobs.iter().rposition(|j| !j.is_final);
    jobs.into_iter()
        .enumerate()
        .filter(|(i, j)| {
            j.is_final || (Some(*i) == last_partial && last_final.is_none_or(|f| *i > f))
        })
        .map(|(_, j)| j)
        .collect()
}

/// The number of audio frames (50 per second) whisper needs to cover a phrase plus a margin,
/// rounded up to whole seconds, capped at the model's 30 second window of 1500.
pub fn audio_ctx_for(samples: usize) -> i32 {
    const FRAMES_PER_SECOND: usize = 50;
    const MARGIN_SECONDS: usize = 1;
    const MIN_SECONDS: usize = 3;
    let seconds = samples.div_ceil(RECOGNISER_RATE as usize) + MARGIN_SECONDS;
    (seconds.max(MIN_SECONDS) * FRAMES_PER_SECOND).min(1500) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    const R: usize = RECOGNISER_RATE as usize;

    fn tone(seconds: f32, amp: f32) -> Vec<f32> {
        (0..(R as f32 * seconds) as usize).map(|i| (i as f32 * 0.3).sin() * amp).collect()
    }
    fn silence(seconds: f32) -> Vec<f32> {
        vec![0.0; (R as f32 * seconds) as usize]
    }

    #[test]
    fn silence_makes_no_phrase() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        assert!(s.push(&silence(5.0)).is_empty());
        assert!(!s.in_phrase());
    }

    #[test]
    fn a_spoken_phrase_closes_after_the_pause() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        let mut audio = silence(1.0);
        audio.extend(tone(1.5, 0.2));
        audio.extend(silence(1.5));
        let phrases = s.push(&audio);
        let finals: Vec<_> = phrases.iter().filter(|p| p.is_final).collect();
        assert_eq!(finals.len(), 1);
        let secs = finals[0].samples.len() as f32 / R as f32;
        // Pre-roll + speech + short tail, and far less than the 1.5 s of trailing silence.
        assert!((1.6..2.4).contains(&secs), "phrase of {secs} s");
    }

    #[test]
    fn a_click_is_ignored() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        let mut audio = silence(1.0);
        audio.extend(tone(0.04, 0.5));
        audio.extend(silence(2.0));
        assert!(s.push(&audio).is_empty());
    }

    #[test]
    fn long_singing_is_split_and_gets_progress_reads() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        let mut audio = silence(0.5);
        audio.extend(tone(30.0, 0.2));
        let phrases = s.push(&audio);
        let finals = phrases.iter().filter(|p| p.is_final).count();
        let partials = phrases.iter().filter(|p| !p.is_final).count();
        assert!(finals >= 2, "{finals} finals");
        assert!(partials >= 8, "{partials} partials");
        assert!(phrases.iter().all(|p| p.samples.len() <= 12 * R + R / 10));
    }

    #[test]
    fn a_quiet_hum_is_not_speech() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        assert!(s.push(&tone(5.0, 0.0057)).is_empty());
    }

    #[test]
    fn flush_closes_an_open_phrase() {
        let mut s = Segmenter::new(SegmenterConfig::default());
        s.push(&tone(2.0, 0.2));
        assert!(s.in_phrase());
        assert!(s.flush().is_some_and(|p| p.is_final));
        assert!(s.flush().is_none());
    }

    fn job(is_final: bool, tag: f32) -> Phrase {
        Phrase { samples: vec![tag], is_final, kept: None }
    }

    fn tags(jobs: Vec<Phrase>) -> Vec<f32> {
        coalesce(jobs).iter().map(|j| j.samples[0]).collect()
    }

    #[test]
    fn coalescing_keeps_finals_and_the_latest_partial() {
        let jobs = vec![job(false, 1.0), job(false, 2.0), job(true, 3.0), job(false, 4.0), job(false, 5.0)];
        assert_eq!(tags(jobs), vec![3.0, 5.0]);
        assert_eq!(tags(vec![job(false, 1.0), job(false, 2.0)]), vec![2.0]);
        assert_eq!(tags(vec![job(true, 1.0), job(true, 2.0)]), vec![1.0, 2.0]);
    }

    #[test]
    fn audio_context_covers_the_phrase() {
        assert_eq!(audio_ctx_for(R), 150, "short phrases get the 3 s minimum");
        assert_eq!(audio_ctx_for(5 * R), 300);
        assert_eq!(audio_ctx_for(12 * R), 650);
        assert_eq!(audio_ctx_for(60 * R), 1500);
    }
}
