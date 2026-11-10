//! Getting Bibles in: a file the church holds, and eBible's free list. Conversion and the
//! network run on worker threads; their answers wait in `events` until the next frame.
//! Nothing is downloaded until a volunteer presses the button for that translation.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use artosia_core::bible_file::{self, Details, ImportError, Installed};
use artosia_core::ebible::{self, Translation};
use rune::{TextEdit, Tx};

use crate::state::{App, Sheet};
use crate::strings as t;

/// How long a saved copy of eBible's list stands in for a fresh one.
const CATALOGUE_FRESH: Duration = Duration::from_secs(7 * 24 * 3600);
const CATALOGUE_FILE: &str = "ebible-translations.csv";

pub enum Event {
    Installed(Result<Installed, ImportError>),
    Catalogue(Result<Vec<Translation>, String>),
    Downloaded(Result<Installed, String>),
}

/// eBible's list as the dialog has it.
pub enum Catalogue {
    /// Not asked for yet.
    Idle,
    Loading,
    Ready(Vec<Translation>),
    Failed(String),
}

/// One line of the list: a language, or a translation of the language above.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    Language { name: String, local: String },
    Translation(usize),
}

pub struct Desk {
    /// A handler asked for a Bible file to choose; the next frame opens the dialog.
    pub want_file: bool,
    /// A file is being converted.
    pub busy: bool,
    pub events: Arc<Mutex<Vec<Event>>>,
    pub catalogue: Catalogue,
    pub query: TextEdit,
    /// The translation whose copyright line is showing, by eBible's id.
    pub picked: Option<String>,
    /// The list as arranged for `rows_for`.
    pub rows: Vec<Row>,
    pub rows_for: Option<String>,
    /// The translation being downloaded, by id.
    pub downloading: Option<String>,
}

impl Default for Desk {
    fn default() -> Self {
        Desk {
            want_file: false,
            busy: false,
            events: Arc::default(),
            catalogue: Catalogue::Idle,
            query: TextEdit::new(""),
            picked: None,
            rows: Vec::new(),
            rows_for: None,
            downloading: None,
        }
    }
}

impl Desk {
    pub fn translations(&self) -> &[Translation] {
        match &self.catalogue {
            Catalogue::Ready(list) => list,
            _ => &[],
        }
    }

    pub fn picked_translation(&self) -> Option<&Translation> {
        let id = self.picked.as_ref()?;
        self.translations().iter().find(|item| item.id == *id)
    }

    /// Arranges the list for what is typed. Cheap to call every frame: it works only on a change.
    pub fn arrange(&mut self) -> bool {
        let query = self.query.text.trim().to_string();
        if self.rows_for.as_ref() == Some(&query) && !self.rows.is_empty() {
            return false;
        }
        let list = self.translations();
        let mut rows = Vec::new();
        for group in ebible::grouped(list, &query) {
            rows.push(Row::Language { name: group.language.clone(), local: if group.local == group.language { String::new() } else { group.local.clone() } });
            rows.extend(group.items.into_iter().map(Row::Translation));
        }
        self.rows = rows;
        self.rows_for = Some(query);
        true
    }
}

/// The file name an installed translation gets, and the code the Look dialog shows for it.
pub fn installed_code(id: &str) -> String {
    bible_file::file_code(id).to_uppercase()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_global(Some(Duration::from_secs(120)))
        .user_agent("Artosia (free worship presentation app)")
        .build()
        .into()
}

fn net_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => format!("the server answered {code}"),
        ureq::Error::Timeout(_) => "it took too long".to_string(),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => "there is no connection".to_string(),
        other => other.to_string(),
    }
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).call().map_err(net_error)?;
    response.body_mut().with_config().limit(limit).read_to_vec().map_err(net_error)
}

/// eBible's list: the saved copy when it is under a week old, else a fresh one (saved), else
/// the old saved copy when the network fails, so a church with a poor connection keeps its list.
fn load_catalogue(cache: Option<PathBuf>) -> Result<Vec<Translation>, String> {
    let saved = cache.as_ref().and_then(|path| std::fs::read(path).ok().map(|bytes| (path.clone(), bytes)));
    let age = |path: &Path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok().and_then(|at| SystemTime::now().duration_since(at).ok());
    let parse = |bytes: &[u8]| ebible::parse_catalogue(&String::from_utf8_lossy(bytes));
    if let Some((path, bytes)) = &saved
        && age(path).is_some_and(|age| age < CATALOGUE_FRESH)
    {
        let list = parse(bytes);
        if !list.is_empty() {
            return Ok(list);
        }
    }
    match get(ebible::CATALOGUE_URL, 8 * 1024 * 1024) {
        Ok(bytes) => {
            let list = parse(&bytes);
            if list.is_empty() {
                return Err("the list was not in the shape expected".to_string());
            }
            if let Some(path) = &cache {
                let _ = std::fs::write(path, &bytes);
            }
            Ok(list)
        }
        Err(error) => match saved.map(|(_, bytes)| parse(&bytes)).filter(|list| !list.is_empty()) {
            Some(list) => Ok(list),
            None => Err(error),
        },
    }
}

/// Made-up translations for a headless render, so no real Bible's name stands in a screenshot.
fn demo_list() -> Vec<Translation> {
    let item = |id: &str, code: &str, local: &str, english: &str, title: &str, copyright: &str, old: u16, new: u16| Translation {
        id: id.into(),
        language_code: code.into(),
        language: local.into(),
        language_english: english.into(),
        title: title.into(),
        description: String::new(),
        copyright: copyright.into(),
        old_books: old,
        new_books: new,
    };
    vec![
        item("demoa", "ind", "Bahasa Contoh", "Indonesian", "Alkitab Contoh Lengkap", "Made up for a screenshot. Free to copy with this notice kept, not for sale. Second sentence that runs long enough to need a second line.", 39, 27),
        item("demob", "ind", "Bahasa Contoh", "Indonesian", "Perjanjian Baru Contoh", "Made up for a screenshot, 2020.", 0, 27),
        item("democ", "mkn", "Contoh Kupang", "Kupang Malay", "Kitab Contoh Kupang", "Made up for a screenshot.", 4, 27),
        item("demod", "aaa", "Aaa", "Afar", "Afar made up Bible", "Made up for a screenshot.", 39, 27),
        item("demoe", "bbb", "Bbb", "Basque", "Basque made up Bible", "Made up for a screenshot.", 0, 4),
        item("demof", "ccc", "Ccc", "Czech", "Czech made up Bible", "Made up for a screenshot.", 39, 27),
        item("demog", "ccc", "Ccc", "Czech", "Czech second made up Bible", "Made up for a screenshot.", 0, 27),
        item("demoh", "ddd", "Ddd", "Danish", "Danish made up Bible", "Made up for a screenshot.", 39, 27),
    ]
}

impl App {
    /// Puts the Bibles dialog in a state for a headless render: "ready", "picked", "loading",
    /// "failed" or "empty".
    pub fn demo_bibles(&mut self, state: &str) {
        self.sheet = Sheet::Bibles;
        match state {
            "loading" => self.desk.catalogue = Catalogue::Loading,
            "failed" => self.desk.catalogue = Catalogue::Failed("there is no connection".to_string()),
            _ => {
                self.desk.catalogue = Catalogue::Ready(demo_list());
                match state {
                    "picked" => self.desk.picked = Some("demoa".to_string()),
                    "empty" => self.desk.query.replace_text("zzzqq"),
                    _ => {}
                }
                self.desk.arrange();
            }
        }
    }

    pub fn open_bibles(&mut self) {
        self.sheet = Sheet::Bibles;
        if matches!(self.desk.catalogue, Catalogue::Idle | Catalogue::Failed(_)) && !rune::is_headless() {
            self.start_catalogue();
        }
    }

    fn bibles_folder(&self) -> Option<PathBuf> {
        self.folder.as_ref().map(|folder| folder.join("bibles"))
    }

    /// Asks eBible for its list on a worker; the dialog shows a skeleton meanwhile.
    pub fn start_catalogue(&mut self) {
        self.desk.catalogue = Catalogue::Loading;
        let cache = self.folder.as_ref().map(|folder| folder.join(CATALOGUE_FILE));
        let (events, waker) = (self.desk.events.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = load_catalogue(cache);
            if let Ok(mut events) = events.lock() {
                events.push(Event::Catalogue(result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
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
        let path = path.to_path_buf();
        let (events, waker) = (self.desk.events.clone(), self.waker.clone());
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

    /// Downloads one translation from eBible and installs it. Only a button press calls this.
    pub fn download_bible(&mut self, id: &str) {
        let Some(item) = self.desk.translations().iter().find(|item| item.id == id).cloned() else { return };
        let Some(target) = self.bibles_folder() else {
            self.say(t::BIBLES_PRACTICE(), None);
            return;
        };
        if self.desk.downloading.is_some() {
            return;
        }
        self.desk.downloading = Some(id.to_string());
        let (events, waker) = (self.desk.events.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = get(&ebible::download_url(&item.id), 64 * 1024 * 1024).and_then(|bytes| {
                let details = Details { code: item.id.clone(), name: item.title.clone(), language: item.language_english.clone(), copyright: item.copyright.clone() };
                bible_file::install_bytes(&format!("{}_usfx.zip", item.id), &bytes, &target, &details).map_err(|error| error.to_string())
            });
            if let Ok(mut events) = events.lock() {
                events.push(Event::Downloaded(result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// Makes an installed translation the one the slides use.
    pub fn use_bible(&mut self, code: &str) {
        self.set_bible(code);
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
                Event::Catalogue(Ok(list)) => {
                    self.desk.catalogue = Catalogue::Ready(list);
                    self.desk.rows_for = None;
                }
                Event::Catalogue(Err(why)) => self.desk.catalogue = Catalogue::Failed(why),
                Event::Downloaded(result) => {
                    self.desk.downloading = None;
                    match result {
                        Ok(installed) => self.bible_installed(installed),
                        Err(why) => self.say(t::bibles_download_failed(&why), None),
                    }
                }
            }
        }
        if self.sheet == Sheet::Bibles {
            changed |= self.desk.arrange();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &str = "languageCode,translationId,languageName,languageNameInEnglish,title,description,Redistributable,Copyright,OTbooks,NTbooks,downloadable\n\
        ind,indmade,Bahasa Buatan,Indonesian,Alkitab Buatan,x,True,Made up line,0,27,True\n\
        xyz,xyz1,Zed,Zedish,The Zed Bible,x,True,Made up line,39,27,True\n";

    #[test]
    fn a_fresh_saved_list_is_used_without_the_network() {
        let folder = std::env::temp_dir().join(format!("artosia-desk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
        let cache = folder.join(CATALOGUE_FILE);
        std::fs::write(&cache, CSV).unwrap_or_else(|e| panic!("{e}"));
        // A file written a moment ago is fresh, so the network is never asked.
        let list = load_catalogue(Some(cache)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(list.len(), 2);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_list_is_arranged_for_what_is_typed() {
        let mut desk = Desk { catalogue: Catalogue::Ready(ebible::parse_catalogue(CSV)), ..Desk::default() };
        assert!(desk.arrange());
        assert_eq!(desk.rows.len(), 4);
        assert!(!desk.arrange());
        desk.query.replace_text("zed");
        assert!(desk.arrange());
        assert_eq!(desk.rows.len(), 2);
        desk.query.replace_text("nothing");
        desk.arrange();
        assert!(desk.rows.is_empty());
    }

    /// Reaches eBible for real: `cargo test -p artosia live_ -- --ignored`.
    #[test]
    #[ignore = "uses the network"]
    fn live_catalogue_is_fetched_and_read() {
        let list = load_catalogue(None).unwrap_or_else(|e| panic!("{e}"));
        assert!(list.len() > 1000);
        assert!(list.iter().any(|item| item.language_code == "ind"));
    }

    #[test]
    fn an_installed_code_matches_what_the_reader_lists() {
        assert_eq!(installed_code("indags"), "INDAGS");
        assert_eq!(installed_code("zlmKSZI"), "ZLMKSZI");
    }
}
