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

pub use audio::{LevelMeter, RECOGNISER_RATE, Resampler, level_from_rms, mix_to_mono, rms};
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
                Err(RecvTimeoutError::Timeout) => {}
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
                    let raw = match recogniser.transcribe(&phrase.samples, phrase.is_final) {
                        Ok(raw) => raw,
                        Err(message) => {
                            emit(Heard::Problem(message));
                            continue;
                        }
                    };
                    let cleaned = clean_text(&raw);
                    if phrase.is_final {
                        last_partial.clear();
                        if let Some(text) = cleaned {
                            emit(Heard::Final(text));
                        }
                    } else if let Some(text) = cleaned
                        && text != last_partial
                    {
                        last_partial.clone_from(&text);
                        emit(Heard::Partial(text));
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
