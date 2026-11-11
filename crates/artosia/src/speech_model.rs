//! The speech models: the files the listener needs to turn sound into words. A PC that has
//! none is offered the first in the Listen dialog, and one that reads it quickly is offered
//! the better one. Only a press fetches either.
//!
//! Why two: on eight real recordings of sung Indonesian the better model named six songs
//! and the first one three, and the better one takes four times as long to read
//! (`crates/artosia-listen/README.md`, "Measured: models and settings").

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::state::{App, Arrived};

/// One model as whisper.cpp's author publishes it, with the size and SHA-256 Hugging Face
/// lists for the file (`/api/models/ggerganov/whisper.cpp/tree/main`, read 2026-11-11).
pub struct File {
    pub name: &'static str,
    url: &'static str,
    size: u64,
    sha256: &'static str,
}

/// The first model: small enough for an old PC.
pub const FIRST: File = File {
    name: "ggml-base-q5_1.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
    size: 59_707_625,
    sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
};

/// The better model, for a PC that reads the first one quickly.
pub const BETTER: File = File {
    name: "ggml-small-q5_1.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small-q5_1.bin",
    size: 190_085_487,
    sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
};

/// A PC that reads the first model in this share of the sound's own length is offered the
/// better one, which took four times as long on the PC both were measured on.
const QUICK: f32 = 0.1;
/// The better model reading slower than this share falls behind the singing: the first one
/// is used again, for good.
const BEHIND: f32 = 0.6;
/// Readings before the speed is believed.
const READINGS: u32 = 3;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Model {
    /// Not looked for yet, or there is no library folder to keep it in (practice).
    #[default]
    Unknown,
    Missing,
    Downloading,
    Failed(String),
    Ready,
}

/// The better model's row in the Listen dialog.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Better {
    /// Nothing to show: it is here already, or this PC has not shown it is quick enough.
    #[default]
    No,
    Offered,
    Downloading,
    Failed(String),
}

impl App {
    fn model_file(&self, file: &File) -> Option<PathBuf> {
        self.folder.as_ref().map(|folder| folder.join("models").join(file.name))
    }

    /// The better model is on this PC and has not fallen behind on it.
    pub fn reads_with_better(&self) -> bool {
        !self.settings.listen_better_behind && self.model_file(&BETTER).is_some_and(|path| path.is_file())
    }

    /// The model the listener reads with: the better one when it is here and keeps up, else
    /// the first.
    pub fn model_path(&self) -> Option<PathBuf> {
        self.model_file(if self.reads_with_better() { &BETTER } else { &FIRST })
    }

    /// Whether this build can turn sound into words at all.
    pub fn speech_built() -> bool {
        cfg!(feature = "speech")
    }

    /// Looks whether a model is on this PC, and whether the better one is to be offered. A
    /// download in progress is left alone.
    pub fn check_model(&mut self) {
        if self.model != Model::Downloading {
            self.model = match self.model_path() {
                Some(path) if path.is_file() => Model::Ready,
                Some(_) => Model::Missing,
                None => Model::Unknown,
            };
        }
        if !matches!(self.better, Better::Downloading | Better::Failed(_)) {
            let wanted = self.settings.listen_quick && !self.settings.listen_better_behind && self.model == Model::Ready && self.model_file(&BETTER).is_some_and(|path| !path.is_file());
            self.better = if wanted { Better::Offered } else { Better::No };
        }
    }

    /// A phrase of `sound` seconds took `took` seconds to read. Three readings say whether
    /// this PC is quick enough for the better model, or too slow for it after all.
    pub fn note_reading(&mut self, sound: f32, took: f32) {
        // A second or two of sound is read in a fixed least time and says little.
        if sound < 3.0 {
            return;
        }
        let share = took / sound;
        let (readings, mean) = self.read_speed;
        self.read_speed = (readings + 1, if readings == 0 { share } else { mean * 0.7 + share * 0.3 });
        let (readings, mean) = self.read_speed;
        if readings < READINGS {
            return;
        }
        if self.reads_with_better() {
            if mean > BEHIND {
                self.settings.listen_better_behind = true;
                self.touch_settings();
                if self.listener.is_some() {
                    self.set_listening(true);
                }
                self.check_model();
            }
        } else if mean <= QUICK && !self.settings.listen_quick {
            self.settings.listen_quick = true;
            self.touch_settings();
            self.check_model();
        }
    }

    /// Fetches the first model on a worker. Only the button in the Listen dialog calls this.
    pub fn download_model(&mut self) {
        let Some(path) = self.model_file(&FIRST) else { return };
        if self.model == Model::Downloading {
            return;
        }
        self.model = Model::Downloading;
        self.fetch_on_a_worker(&FIRST, path, Arrived::Model);
    }

    /// Fetches the better model on a worker, from its row in the Listen dialog.
    pub fn download_better(&mut self) {
        let Some(path) = self.model_file(&BETTER) else { return };
        if self.better == Better::Downloading {
            return;
        }
        self.better = Better::Downloading;
        self.fetch_on_a_worker(&BETTER, path, Arrived::BetterModel);
    }

    fn fetch_on_a_worker(&self, file: &'static File, path: PathBuf, arrived: fn(Result<(), String>) -> Arrived) {
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = fetch(file, &path);
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(arrived(result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// The download ended: the model is ready, or the dialog says why it is not.
    pub fn model_arrived(&mut self, result: Result<(), String>) {
        self.model = match result {
            Ok(()) => Model::Ready,
            Err(why) => Model::Failed(why),
        };
        // Listening that was waiting for words starts again with the model in place.
        if self.model == Model::Ready && self.listener.is_some() {
            self.set_listening(true);
        }
    }

    /// The better model's download ended: the listener starts again and reads with it.
    pub fn better_arrived(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.better = Better::No;
                if self.listener.is_some() {
                    self.set_listening(true);
                }
            }
            Err(why) => self.better = Better::Failed(why),
        }
    }
}

/// Downloads to a side file and keeps it only when it is the file Hugging Face lists.
fn fetch(model: &File, path: &Path) -> Result<(), String> {
    let disk = |_: std::io::Error| crate::strings::WHY_DISK().to_string();
    let folder = path.parent().ok_or_else(|| crate::strings::WHY_DISK().to_string())?;
    std::fs::create_dir_all(folder).map_err(disk)?;
    let part = path.with_extension("part");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_global(Some(Duration::from_secs(1800)))
        .user_agent(concat!("Artosia/", env!("CARGO_PKG_VERSION"), " (free worship presentation app)"))
        .build()
        .into();
    let mut response = agent.get(model.url).call().map_err(crate::why::net)?;
    let mut file = std::fs::File::create(&part).map_err(disk)?;
    // A reader has ureq's 10 MB limit unless it is raised.
    std::io::copy(&mut response.body_mut().with_config().limit(model.size + 1024).reader(), &mut file).map_err(|_| crate::strings::WHY_BROKEN().to_string())?;
    drop(file);
    let right = std::fs::metadata(&part).map(|meta| meta.len()).ok() == Some(model.size) && artosia_update::manifest::sha256_hex_of_file(&part).ok().as_deref() == Some(model.sha256);
    if !right {
        let _ = std::fs::remove_file(&part);
        return Err(crate::strings::WHY_DAMAGED().to_string());
    }
    std::fs::rename(&part, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(name: &str) -> (App, PathBuf) {
        let folder = std::env::temp_dir().join(format!("artosia-model-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("models")).unwrap_or_else(|e| panic!("{e}"));
        let mut app = App::practice();
        app.folder = Some(folder.clone());
        (app, folder)
    }

    #[test]
    fn practice_has_no_place_for_a_model_and_a_library_says_missing() {
        let mut app = App::practice();
        app.check_model();
        assert_eq!(app.model, Model::Unknown);
        let (mut app, folder) = library("missing");
        app.check_model();
        assert_eq!(app.model, Model::Missing);
        std::fs::write(folder.join("models").join(FIRST.name), b"x").unwrap_or_else(|e| panic!("{e}"));
        app.check_model();
        assert_eq!(app.model, Model::Ready);
        app.model_arrived(Err("offline".into()));
        assert_eq!(app.model, Model::Failed("offline".into()));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_quick_pc_is_offered_the_better_model_and_reads_with_it_once_it_is_there() {
        let (mut app, folder) = library("quick");
        std::fs::write(folder.join("models").join(FIRST.name), b"x").unwrap_or_else(|e| panic!("{e}"));
        app.check_model();
        assert_eq!(app.better, Better::No, "nothing is offered before the PC has been timed");
        // Twelve seconds of sound read in half a second, three times.
        for _ in 0..3 {
            app.note_reading(12.0, 0.5);
        }
        assert_eq!(app.better, Better::Offered);
        assert!(app.model_path().is_some_and(|path| path.ends_with(FIRST.name)));
        std::fs::write(folder.join("models").join(BETTER.name), b"x").unwrap_or_else(|e| panic!("{e}"));
        app.better_arrived(Ok(()));
        app.check_model();
        assert_eq!(app.better, Better::No);
        assert!(app.model_path().is_some_and(|path| path.ends_with(BETTER.name)));
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_slow_pc_is_offered_nothing() {
        let (mut app, folder) = library("slow");
        std::fs::write(folder.join("models").join(FIRST.name), b"x").unwrap_or_else(|e| panic!("{e}"));
        app.check_model();
        // Twelve seconds of sound read in five.
        for _ in 0..5 {
            app.note_reading(12.0, 5.0);
        }
        assert_eq!(app.better, Better::No);
        assert!(!app.settings.listen_quick);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_better_model_that_falls_behind_gives_way_to_the_first_for_good() {
        let (mut app, folder) = library("behind");
        for file in [&FIRST, &BETTER] {
            std::fs::write(folder.join("models").join(file.name), b"x").unwrap_or_else(|e| panic!("{e}"));
        }
        app.settings.listen_quick = true;
        app.check_model();
        assert!(app.reads_with_better());
        // Twelve seconds of sound read in ten: the next phrase is waiting before this one is read.
        for _ in 0..3 {
            app.note_reading(12.0, 10.0);
        }
        assert!(app.settings.listen_better_behind);
        assert!(app.model_path().is_some_and(|path| path.ends_with(FIRST.name)));
        assert_eq!(app.better, Better::No, "it is not offered again");
        let _ = std::fs::remove_dir_all(folder);
    }
}
