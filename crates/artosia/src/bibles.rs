//! Getting Bibles in: a file the church holds, and eBible's free list. Conversion and the
//! network run on worker threads; their answers wait in `events` until the next frame.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use artosia_core::bible_file::{self, ImportError, Installed};
use rune::Tx;

use crate::state::{App, Sheet};
use crate::strings as t;

pub enum Event {
    Installed(Result<Installed, ImportError>),
}

#[derive(Default)]
pub struct Desk {
    /// A handler asked for a Bible file to choose; the next frame opens the dialog.
    pub want_file: bool,
    /// A file is being converted.
    pub busy: bool,
    pub events: Arc<Mutex<Vec<Event>>>,
}

impl App {
    pub fn open_bibles(&mut self) {
        self.sheet = Sheet::Bibles;
    }

    fn bibles_folder(&self) -> Option<PathBuf> {
        self.folder.as_ref().map(|folder| folder.join("bibles"))
    }

    /// Converts a Bible file into the library on a worker and reports in a toast.
    pub fn add_bible_file(&mut self, path: &Path) {
        let Some(target) = self.bibles_folder() else {
            self.say(t::BIBLES_PRACTICE(), None);
            return;
        };
        if self.desk.busy {
            return;
        }
        self.desk.busy = true;
        let (events, waker) = (self.desk.events.clone(), self.waker.clone());
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let result = bible_file::install_path(&path, &target);
            if let Ok(mut events) = events.lock() {
                events.push(Event::Installed(result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// A Bible reached the library: list it again and read it fresh if it was open.
    fn bible_installed(&mut self, installed: Installed) {
        self.list_bibles();
        self.bibles.remove(&installed.code);
        self.bibles_loading.retain(|code| *code != installed.code);
        if self.settings.bible == installed.code || self.settings.second_bible == installed.code {
            self.want_bible(&installed.code);
        }
        self.say(t::bible_added(&installed.name, installed.verses), None);
    }

    /// Runs before every frame: opens the file dialog when asked and takes what the workers did.
    pub fn poll_desk(&mut self, tx: &mut Tx) -> bool {
        let mut changed = false;
        if self.desk.want_file {
            self.desk.want_file = false;
            let options = rune::FileDialogOptions { title: t::BIBLES_CHOOSE_FILE().to_string(), ..Default::default() }
                .filter(t::BIBLES_FILE_HINT(), &["sqlite3", "mybible", "xml", "zip", "usfm", "sfm"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.add_bible_file(&path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        let arrived: Vec<Event> = self.desk.events.lock().map(|mut events| std::mem::take(&mut *events)).unwrap_or_default();
        for event in arrived {
            changed = true;
            match event {
                Event::Installed(result) => {
                    self.desk.busy = false;
                    match result {
                        Ok(installed) => self.bible_installed(installed),
                        Err(ImportError::Unsupported) => self.say(t::BIBLES_FILE_UNSUPPORTED(), None),
                        Err(ImportError::Empty) => self.say(t::BIBLES_FILE_EMPTY(), None),
                        Err(other) => self.say(t::bibles_file_failed(&other.to_string()), None),
                    }
                }
            }
        }
        changed
    }
}
