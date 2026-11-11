//! Listening to the room: naming the verse or the song from what is said and sung.
//!
//! This crate delivers the ears: it captures an audio input, reports a level, and (when a speech
//! model is installed) turns phrases into text. Matching that text to songs and verses is the
//! operator app's job. See `README.md` for the research and the numbers behind the choices.

mod audio;
mod capture;
mod recogniser;
mod segment;
mod tempo;
mod text;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub use audio::{LevelMeter, RECOGNISER_RATE, Resampler, level_from_rms, mix_to_mono, read_wav, rms};
pub use segment::{Phrase, Segmenter, SegmenterConfig, audio_ctx_for, coalesce};
pub use tempo::{MAX_BPM, MIN_BPM, Tempo, TempoTracker};
pub use text::clean_text;

/// An audio input the system offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Stable text to pass back to [`Listener::start`].
    pub id: String,
    /// What to show a person.
    pub name: String,
}

/// Microphones and line inputs (a 3.5 mm jack appears as one on most PCs).
pub fn devices() -> Vec<Device> {
    capture::input_devices()
}

pub use capture::{SYSTEM_AUDIO, system_audio_available};

/// What opening a source delivered over a short time; see [`probe`].
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
    pub loopback: bool,
    /// Mono frames that arrived.
    pub frames: usize,
    pub peak: f32,
    pub rms: f32,
}

/// Opens a source through the code the listener uses (`None` is the default input,
/// [`SYSTEM_AUDIO`] the computer's own sound), reads it for `duration`, and reports what arrived.
/// Keeps no audio. `on_block` sees each mono block, for tools that feed a detector.
pub fn probe(device: Option<&str>, duration: Duration, mut on_block: impl FnMut(&[f32], u32)) -> Result<Probe, ListenError> {
    let (tx, rx) = mpsc::sync_channel(QUEUE_BLOCKS);
    let running = capture::open(device, tx)?;
    let began = std::time::Instant::now();
    let (mut frames, mut peak, mut sum_squares) = (0usize, 0.0f32, 0.0f64);
    while began.elapsed() < duration {
        if let Ok(capture::Capture::Audio(block)) = rx.recv_timeout(Duration::from_millis(50)) {
            frames += block.len();
            for &s in &block {
                peak = peak.max(s.abs());
                sum_squares += f64::from(s) * f64::from(s);
            }
            on_block(&block, running.sample_rate);
        }
    }
    Ok(Probe {
        sample_rate: running.sample_rate,
        channels: running.channels,
        sample_format: running.sample_format.to_string(),
        loopback: running.loopback,
        frames,
        peak,
        rms: if frames == 0 { 0.0 } else { (sum_squares / frames as f64).sqrt() as f32 },
    })
}

/// What the listener reports.
#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    /// Loudness 0..1 on a decibel scale, four times a second.
    Level(f32),
    /// The beat, twice a second: `None` while the room has no steady pulse.
    Tempo(Option<Tempo>),
    /// A progress read of a phrase still being said or sung. It may change or vanish.
    Partial(String),
    /// A phrase that ended. Search with this one.
    Final(String),
    /// Something a person should be told, in words (no model installed, a device unplugged).
    Problem(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenError {
    /// The chosen device is missing or there is no default input.
    NoDevice(String),
    /// The device refused to open.
    Stream(String),
    /// The worker thread could not start.
    Thread(String),
}

impl fmt::Display for ListenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice(m) | Self::Stream(m) | Self::Thread(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for ListenError {}

/// Capture is running while this exists. Dropping it stops listening, frees the model and waits
/// for the worker threads (up to the length of one recognition in progress).
pub struct Listener {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

type Emit = Arc<dyn Fn(Heard) + Send + Sync>;

const LEVELS_PER_SECOND: u32 = 4;
/// Blocks of audio the capture callback may queue before it drops some (about 5 s at 10 ms blocks).
const QUEUE_BLOCKS: usize = 512;

impl Listener {
    /// Opens `device` (`None` is the default input) and starts reporting.
    ///
    /// Levels work whatever `model` is. If the model file is missing or the build has no
    /// recogniser, one `Heard::Problem` says so and listening continues for levels only.
    /// `language` is `"id"`, `"en"`, or `"auto"`.
    pub fn start(
        device: Option<&str>,
        model: &Path,
        language: &str,
        on_heard: impl Fn(Heard) + Send + 'static,
    ) -> Result<Listener, ListenError> {
        let on_heard = Mutex::new(on_heard);
        let emit: Emit = Arc::new(move |h| {
            if let Ok(f) = on_heard.lock() {
                f(h);
            }
        });
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), ListenError>>(1);
        let job = Job {
            device: device.map(str::to_string),
            model: model.to_path_buf(),
            language: language.to_string(),
            emit,
            stop: stop.clone(),
        };
        let worker = std::thread::Builder::new()
            .name("artosia-listen".into())
            .spawn(move || job.run(ready_tx))
            .map_err(|e| ListenError::Thread(e.to_string()))?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Listener { stop, worker: Some(worker) }),
            Ok(Err(e)) => {
                let _ = worker.join();
                Err(e)
            }
            Err(_) => {
                let _ = worker.join();
                Err(ListenError::Thread("the listening thread stopped before it began".into()))
            }
        }
    }
}

/// Reads one phrase and says what to report, if anything. The live listener and
/// [`hear_samples`] both go through here, so a test hears what the microphone path hears.
fn read_phrase(recogniser: &mut dyn recogniser::Recogniser, phrase: &Phrase, last_partial: &mut String) -> Option<Heard> {
    let raw = match recogniser.transcribe(&phrase.samples, phrase.is_final) {
        Ok(raw) => raw,
        Err(message) => return Some(Heard::Problem(message)),
    };
    let cleaned = clean_text(&raw);
    if phrase.is_final {
        last_partial.clear();
        cleaned.map(Heard::Final)
    } else {
        let text = cleaned?;
        if text == *last_partial {
            return None;
        }
        last_partial.clone_from(&text);
        Some(Heard::Partial(text))
    }
}

/// Runs recorded audio through the same resampling, phrase cutting and recognising as the
/// microphone, in place of a microphone, and reports every phrase that ended as `Heard::Final`
/// (progress reads are skipped: nobody is watching them). Blocks until done, so call it from a
/// test or a tool, never from a window's thread. `samples` are mono at `sample_rate`.
pub fn hear_samples(
    samples: &[f32],
    sample_rate: u32,
    model: &Path,
    language: &str,
    mut on_heard: impl FnMut(Heard),
) -> Result<(), String> {
    let mut recogniser = recogniser::load(model, language)?;
    let mut resampler = Resampler::new(sample_rate, RECOGNISER_RATE);
    let mut segmenter = Segmenter::new(SegmenterConfig::default());
    let mut last_partial = String::new();
    let mut resampled = Vec::new();
    let mut phrases = Vec::new();
    // Blocks of 10 ms, as a capture callback delivers them.
    for block in samples.chunks((sample_rate / 100).max(1) as usize) {
        resampled.clear();
        resampler.push(block, &mut resampled);
        phrases.extend(segmenter.push(&resampled));
    }
    phrases.extend(segmenter.flush());
    for phrase in phrases.iter().filter(|p| p.is_final) {
        if let Some(heard) = read_phrase(recogniser.as_mut(), phrase, &mut last_partial) {
            on_heard(heard);
        }
    }
    Ok(())
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct Job {
    device: Option<String>,
    model: PathBuf,
    language: String,
    emit: Emit,
    stop: Arc<AtomicBool>,
}

impl Job {
    /// Runs on the worker thread, which owns the (non-`Send`) capture stream.
    fn run(self, ready: mpsc::SyncSender<Result<(), ListenError>>) {
        let (tx, rx) = mpsc::sync_channel(QUEUE_BLOCKS);
        let running = match capture::open(self.device.as_deref(), tx) {
            Ok(r) => {
                let _ = ready.send(Ok(()));
                r
            }
            Err(e) => {
                let _ = ready.send(Err(e));
                return;
            }
        };

        // The decoder runs on its own thread so a slow recognition never stalls capture or levels.
        let decoder = self.spawn_decoder();
        let (phrase_tx, handle) = match decoder {
            Some((tx, handle)) => (Some(tx), Some(handle)),
            None => (None, None),
        };

        let mut meter = LevelMeter::new(running.sample_rate, LEVELS_PER_SECOND);
        let mut resampler = Resampler::new(running.sample_rate, RECOGNISER_RATE);
        let mut tracker = TempoTracker::new(running.sample_rate);
        let mut segmenter = Segmenter::new(SegmenterConfig::default());
        let mut resampled = Vec::new();
        let mut last_error = String::new();
        let mut silence: Vec<f32> = Vec::new();

        while !self.stop.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(capture::Capture::Audio(block)) => {
                    meter.push(&block, |level| (self.emit)(Heard::Level(level)));
                    tracker.push(&block, |tempo| (self.emit)(Heard::Tempo(tempo)));
                    if let Some(phrase_tx) = &phrase_tx {
                        resampled.clear();
                        resampler.push(&block, &mut resampled);
                        for phrase in segmenter.push(&resampled) {
                            let _ = phrase_tx.send(phrase);
                        }
                    }
                }
                Ok(capture::Capture::Error(message)) => {
                    if message != last_error {
                        (self.emit)(Heard::Problem(format!("The audio input reported: {message}")));
                        last_error = message;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    // The computer's own sound arrives only while something plays: the quiet in
                    // between is silence, or the meter would hold its last loud reading.
                    if running.loopback {
                        silence.clear();
                        silence.resize((running.sample_rate / 10) as usize, 0.0);
                        meter.push(&silence, |level| (self.emit)(Heard::Level(level)));
                        tracker.push(&silence, |tempo| (self.emit)(Heard::Tempo(tempo)));
                        if let Some(phrase_tx) = &phrase_tx {
                            resampled.clear();
                            resampler.push(&silence, &mut resampled);
                            for phrase in segmenter.push(&resampled) {
                                let _ = phrase_tx.send(phrase);
                            }
                        }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }

        drop(running);
        if let Some(phrase_tx) = phrase_tx {
            if let Some(last) = segmenter.flush() {
                let _ = phrase_tx.send(last);
            }
            drop(phrase_tx);
        }
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }

    /// Starts the recognition thread, or reports why there is none and returns `None`.
    fn spawn_decoder(&self) -> Option<(mpsc::Sender<Phrase>, JoinHandle<()>)> {
        if !self.model.is_file() {
            (self.emit)(Heard::Problem(format!(
                "No speech model installed (looked for {}). The level meter works; text needs the model.",
                self.model.display()
            )));
            return None;
        }
        let (tx, rx) = mpsc::channel::<Phrase>();
        let emit = self.emit.clone();
        let model = self.model.clone();
        let language = self.language.clone();
        let spawned = std::thread::Builder::new().name("artosia-listen-decode".into()).spawn(move || {
            let mut recogniser = match recogniser::load(&model, &language) {
                Ok(r) => r,
                Err(message) => {
                    emit(Heard::Problem(message));
                    // Keep draining so the sender never blocks, then end when it closes.
                    while rx.recv().is_ok() {}
                    return;
                }
            };
            let mut last_partial = String::new();
            while let Ok(first) = rx.recv() {
                let mut jobs = vec![first];
                jobs.extend(rx.try_iter());
                for phrase in coalesce(jobs) {
                    if let Some(heard) = read_phrase(recogniser.as_mut(), &phrase, &mut last_partial) {
                        emit(heard);
                    }
                }
            }
        });
        match spawned {
            Ok(handle) => Some((tx, handle)),
            Err(e) => {
                (self.emit)(Heard::Problem(format!("Could not start the recogniser thread: {e}")));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_that_is_not_there_is_named_in_the_error() {
        let error = probe(Some("no such microphone"), Duration::from_millis(10), |_, _| {}).err();
        assert!(matches!(error, Some(ListenError::NoDevice(m)) if m.contains("no such microphone")));
    }
}
