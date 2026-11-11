//! The speech model: the one file the listener needs to turn sound into words. A PC that has
//! none is offered it in the Listen dialog, and only a press fetches it.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::state::{App, Arrived};

pub const FILE: &str = "ggml-base-q5_1.bin";
/// Where whisper.cpp's author publishes it, with the size and SHA-256 Hugging Face lists for
/// the file (`/api/models/ggerganov/whisper.cpp/tree/main`, read 2026-11-11).
const URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin";
const SIZE: u64 = 59_707_625;
const SHA256: &str = "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898";

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

impl App {
    pub fn model_path(&self) -> Option<PathBuf> {
        self.folder.as_ref().map(|folder| folder.join("models").join(FILE))
    }

    /// Whether this build can turn sound into words at all.
    pub fn speech_built() -> bool {
        cfg!(feature = "speech")
    }

    /// Looks whether the model is on this PC. A download in progress is left alone.
    pub fn check_model(&mut self) {
        if self.model == Model::Downloading {
            return;
        }
        self.model = match self.model_path() {
            Some(path) if path.is_file() => Model::Ready,
            Some(_) => Model::Missing,
            None => Model::Unknown,
        };
    }

    /// Fetches the model on a worker. Only the button in the Listen dialog calls this.
    pub fn download_model(&mut self) {
        let Some(path) = self.model_path() else { return };
        if self.model == Model::Downloading {
            return;
        }
        self.model = Model::Downloading;
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = fetch(&path);
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Model(result));
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
}

/// Downloads to a side file and keeps it only when it is the file Hugging Face lists.
fn fetch(path: &Path) -> Result<(), String> {
    let folder = path.parent().ok_or_else(|| "no folder".to_string())?;
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let part = path.with_extension("part");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_global(Some(Duration::from_secs(1800)))
        .user_agent("Artosia (free worship presentation app)")
        .build()
        .into();
    let mut response = agent.get(URL).call().map_err(|error| error.to_string())?;
    let mut file = std::fs::File::create(&part).map_err(|error| error.to_string())?;
    // A reader has ureq's 10 MB limit unless it is raised.
    std::io::copy(&mut response.body_mut().with_config().limit(SIZE + 1024).reader(), &mut file).map_err(|error| error.to_string())?;
    drop(file);
    let right = std::fs::metadata(&part).map(|meta| meta.len()).ok() == Some(SIZE) && artosia_update::manifest::sha256_hex_of_file(&part).ok().as_deref() == Some(SHA256);
    if !right {
        let _ = std::fs::remove_file(&part);
        return Err("the file that arrived is not the speech model".to_string());
    }
    std::fs::rename(&part, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practice_has_no_place_for_a_model_and_a_library_says_missing() {
        let mut app = App::practice();
        app.check_model();
        assert_eq!(app.model, Model::Unknown);
        let folder = std::env::temp_dir().join(format!("artosia-model-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(folder.join("models"));
        app.folder = Some(folder.clone());
        app.check_model();
        assert_eq!(app.model, Model::Missing);
        std::fs::write(folder.join("models").join(FILE), b"x").expect("writes");
        app.check_model();
        assert_eq!(app.model, Model::Ready);
        app.model_arrived(Err("offline".into()));
        assert_eq!(app.model, Model::Failed("offline".into()));
        let _ = std::fs::remove_dir_all(folder);
    }
}
