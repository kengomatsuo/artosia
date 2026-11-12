//! Getting Bibles in: a file the church holds, and eBible's free list. Conversion and the
//! network run on worker threads; their answers wait in `events` until the next frame.
//! Nothing is downloaded until a volunteer presses the button for that translation.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use artosia_core::bible_file::{self, Details, ImportError, Installed};
use artosia_core::beblia;
use artosia_core::ebible::{self, Source, Translation};
use rune::{TextEdit, Tx};

use crate::state::{App, Sheet};
use crate::strings as t;

/// How long a saved copy of eBible's list stands in for a fresh one.
const CATALOGUE_FRESH: Duration = Duration::from_secs(7 * 24 * 3600);
const CATALOGUE_FILE: &str = "ebible-translations.csv";
const BEBLIA_FILE: &str = "beblia-files.json";

pub enum Event {
    Installed(Result<Installed, ImportError>),
    Catalogue(Result<Vec<Translation>, String>),
    Downloaded(Result<Installed, String>),
    /// What a Beblia file says about itself, by the translation's id.
    Notice(String, Result<(String, String), String>),
}

/// What a Beblia file says about itself: the list holds only file names, so the first kilobytes
/// are read when the row is picked.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    Loading,
    /// The translation's own name and its copyright line, either of them possibly empty.
    Known { name: String, copyright: String },
    Failed,
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
    /// What picked Beblia files said about themselves, by id.
    pub notices: HashMap<String, Notice>,
    /// The languages of the country the list is arranged for, first in its order (`ebible::ranking`).
    pub ranking: Vec<String>,
    /// How much of the running download has arrived, in percent; 0 until the server says how long the file is.
    pub progress: Arc<AtomicU32>,
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
            notices: HashMap::new(),
            ranking: ebible::ranking(crate::onboard::FALLBACK_COUNTRY),
            progress: Arc::default(),
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

    /// The name to show: the file's own once it was read, else the one the list derived.
    pub fn title_of<'a>(&'a self, item: &'a Translation) -> &'a str {
        match self.notices.get(&item.id) {
            Some(Notice::Known { name, .. }) if !name.is_empty() => name,
            _ => &item.title,
        }
    }

    /// Arranges the list for a church in `country` (two capital letters): its languages come first.
    /// A library that never said where the church is keeps the order it always had, Indonesia's.
    pub fn set_country(&mut self, country: &str) {
        let country = if country.is_empty() { crate::onboard::FALLBACK_COUNTRY } else { country };
        self.ranking = ebible::ranking(country);
        self.rows_for = None;
    }

    /// Arranges the list for what is typed. Cheap to call every frame: it works only on a change.
    pub fn arrange(&mut self) -> bool {
        let query = self.query.text.trim().to_string();
        if self.rows_for.as_ref() == Some(&query) && !self.rows.is_empty() {
            return false;
        }
        let list = self.translations();
        let mut rows = Vec::new();
        for group in ebible::grouped(list, &query, &self.ranking) {
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
        .user_agent(concat!("Artosia/", env!("CARGO_PKG_VERSION"), " (free worship presentation app)"))
        .build()
        .into()
}

fn net_error(error: ureq::Error) -> String {
    crate::why::net(error)
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).call().map_err(net_error)?;
    response.body_mut().with_config().limit(limit).read_to_vec().map_err(net_error)
}

/// A download that reports how much has arrived, in percent, when the server says how long it is.
fn get_with_progress(url: &str, limit: u64, progress: &AtomicU32) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).call().map_err(net_error)?;
    let length = response.body().content_length().filter(|length| *length > 0);
    progress.store(0, Ordering::Relaxed);
    let mut reader = response.body_mut().with_config().limit(limit).reader();
    let mut bytes = Vec::with_capacity(length.map_or(0, |length| length.min(limit) as usize));
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut chunk).map_err(|error| net_error(ureq::Error::Io(error)))?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if let Some(length) = length {
            progress.store((bytes.len() as u64 * 100 / length).min(100) as u32, Ordering::Relaxed);
        }
    }
}

/// The first bytes of a file, for reading what it says about itself before it is downloaded.
fn get_head(url: &str, bytes: u64) -> Result<Vec<u8>, String> {
    let mut response = agent().get(url).header("Range", format!("bytes=0-{}", bytes - 1)).call().map_err(net_error)?;
    let mut head = Vec::new();
    // A server that ignores the range sends the whole file; only the start is taken.
    response.body_mut().as_reader().take(bytes).read_to_end(&mut head).map_err(|error| error.to_string())?;
    Ok(head)
}

/// A list's bytes: the saved copy when it is under a week old, else a fresh download (saved),
/// else the old saved copy when the network fails, so a church with a poor connection keeps its list.
fn cached_bytes(cache: &Option<PathBuf>, url: &str, limit: u64, usable: &dyn Fn(&[u8]) -> bool) -> Result<Vec<u8>, String> {
    let saved = cache.as_ref().and_then(|path| std::fs::read(path).ok().map(|bytes| (path.clone(), bytes)));
    let age = |path: &Path| std::fs::metadata(path).and_then(|meta| meta.modified()).ok().and_then(|at| SystemTime::now().duration_since(at).ok());
    if let Some((path, bytes)) = &saved
        && age(path).is_some_and(|age| age < CATALOGUE_FRESH)
        && usable(bytes)
    {
        return Ok(bytes.clone());
    }
    match get(url, limit) {
        Ok(bytes) => {
            if !usable(&bytes) {
                return Err(t::WHY_UNREADABLE().to_string());
            }
            if let Some(path) = cache {
                let _ = std::fs::write(path, &bytes);
            }
            Ok(bytes)
        }
        Err(error) => match saved.map(|(_, bytes)| bytes).filter(|bytes| usable(bytes)) {
            Some(bytes) => Ok(bytes),
            None => Err(error),
        },
    }
}

/// eBible's list.
fn load_ebible(cache: Option<PathBuf>) -> Result<Vec<Translation>, String> {
    let parse = |bytes: &[u8]| ebible::parse_catalogue(&String::from_utf8_lossy(bytes));
    cached_bytes(&cache, ebible::CATALOGUE_URL, 8 * 1024 * 1024, &|bytes| !parse(bytes).is_empty()).map(|bytes| parse(&bytes))
}

/// The Beblia repository's files.
fn load_beblia(cache: Option<PathBuf>) -> Result<Vec<Translation>, String> {
    let parse = |bytes: &[u8]| beblia::parse_tree(&String::from_utf8_lossy(bytes));
    cached_bytes(&cache, beblia::TREE_URL, 4 * 1024 * 1024, &|bytes| !parse(bytes).is_empty()).map(|bytes| parse(&bytes))
}

/// The short codes of eBible's translations as this library last saved its list, each with the
/// code its module would have: what an eBible Bible installed before 0.1.18 is called, since
/// its module does not say. Empty when the list was never saved.
pub fn saved_short_codes(folder: &Path) -> Vec<(String, String)> {
    let Ok(bytes) = std::fs::read(folder.join(CATALOGUE_FILE)) else { return Vec::new() };
    ebible::parse_catalogue(&String::from_utf8_lossy(&bytes)).into_iter().filter(|item| !item.code.is_empty()).map(|item| (bible_file::file_code(&item.id).to_uppercase(), item.code)).collect()
}

/// Both sources in one list. When one of them cannot be reached the other still shows, and the
/// dialog fails only when neither can.
fn load_catalogue(folder: Option<PathBuf>) -> Result<Vec<Translation>, String> {
    let ebible = load_ebible(folder.as_ref().map(|folder| folder.join(CATALOGUE_FILE)));
    let beblia = load_beblia(folder.as_ref().map(|folder| folder.join(BEBLIA_FILE)));
    match (ebible, beblia) {
        (Ok(mut list), Ok(more)) => {
            list.extend(more);
            Ok(list)
        }
        (Ok(list), Err(_)) | (Err(_), Ok(list)) => Ok(list),
        (Err(why), Err(_)) => Err(why),
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
        ..Translation::default()
    };
    let file = |id: &str, language: &str, title: &str, code: &str, size: u64| Translation {
        source: Source::Beblia,
        file: format!("{id}Bible.xml"),
        size,
        id: id.into(),
        language_code: if language == "Indonesian" { "ind" } else { "bbb" }.into(),
        language: language.into(),
        language_english: language.into(),
        title: title.into(),
        code: code.into(),
        ..Translation::default()
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
        file("DemoIndonesianOne", "Indonesian", "Indonesian (Alkitab Terjemahan Contoh)", "TC", 5_400_000),
        file("DemoIndonesianTwo", "Indonesian", "Indonesian ABC (Alkitab Buatan Contoh)", "ABC", 1_450_000),
        file("DemoBasqueOne", "Basque", "Basque Beblia made up", "", 3_900_000),
    ]
}

impl App {
    /// Puts the Bibles dialog in a state for a headless render: "ready", "picked", "loading",
    /// "failed" or "empty".
    pub fn demo_bibles(&mut self, state: &str) {
        self.sheet = Sheet::Bibles;
        match state {
            "loading" => self.desk.catalogue = Catalogue::Loading,
            "failed" => self.desk.catalogue = Catalogue::Failed(t::WHY_OFFLINE().to_string()),
            _ => {
                self.desk.catalogue = Catalogue::Ready(demo_list());
                match state {
                    "picked" => self.desk.picked = Some("demoa".to_string()),
                    "picked-file" => {
                        self.desk.picked = Some("DemoIndonesianOne".to_string());
                        self.desk.notices.insert(
                            "DemoIndonesianOne".to_string(),
                            Notice::Known { name: "Alkitab Contoh (made up for a screenshot)".to_string(), copyright: "Made up for a screenshot. Used with permission, all rights reserved.".to_string() },
                        );
                    }
                    "picked-silent" => {
                        self.desk.picked = Some("DemoIndonesianTwo".to_string());
                        self.desk.notices.insert("DemoIndonesianTwo".to_string(), Notice::Known { name: String::new(), copyright: String::new() });
                    }
                    "picked-reading" => {
                        self.desk.picked = Some("DemoBasqueOne".to_string());
                        self.desk.notices.insert("DemoBasqueOne".to_string(), Notice::Loading);
                    }
                    "empty" => self.desk.query.replace_text("zzzqq"),
                    _ => {}
                }
                self.desk.arrange();
            }
        }
    }

    /// Fills the list with the made-up translations of the headless renders.
    pub fn demo_catalogue(&mut self) {
        self.desk.catalogue = Catalogue::Ready(demo_list());
        self.desk.rows_for = None;
    }

    pub fn open_bibles(&mut self) {
        self.sheet = Sheet::Bibles;
        // The first run's Bible page types into the same filter: the dialog opens on the whole list.
        if !self.desk.query.text.is_empty() {
            self.desk.query.replace_text("");
            self.desk.rows_for = None;
        }
        // A person opens this to look for a Bible: the filter takes what is typed at once.
        self.desk.query.focused = true;
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
        let cache = self.folder.clone();
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

    /// Picks a row. A Beblia file's own name and copyright line are read first, from the start of
    /// the file only (a few kilobytes); nothing is downloaded.
    pub fn pick_bible(&mut self, id: &str) {
        self.desk.picked = Some(id.to_string());
        let Some(item) = self.desk.translations().iter().find(|item| item.id == id).cloned() else { return };
        if item.source != Source::Beblia || matches!(self.desk.notices.get(id), Some(Notice::Loading | Notice::Known { .. })) || rune::is_headless() {
            return;
        }
        self.desk.notices.insert(id.to_string(), Notice::Loading);
        let (events, waker) = (self.desk.events.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = get_head(&beblia::download_url(&item.file), beblia::HEADER_BYTES).and_then(|head| {
                bible_file::beblia_header(&String::from_utf8_lossy(&head)).ok_or_else(|| t::WHY_NOT_BIBLE().to_string())
            });
            if let Ok(mut events) = events.lock() {
                events.push(Event::Notice(item.id.clone(), result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// Downloads one translation and installs it. Only a button press calls this.
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
        let (events, waker, progress) = (self.desk.events.clone(), self.waker.clone(), self.desk.progress.clone());
        std::thread::spawn(move || {
            let result = match item.source {
                Source::Ebible => get_with_progress(&ebible::download_url(&item.id), 64 * 1024 * 1024, &progress).and_then(|bytes| {
                    let details = Details { code: item.id.clone(), name: item.title.clone(), language: item.language_english.clone(), copyright: item.copyright.clone(), short: item.code.clone() };
                    bible_file::install_bytes(&format!("{}_usfx.zip", item.id), &bytes, &target, &details).map_err(|error| error.to_string())
                }),
                // The name and the copyright line are the file's own: the details leave them empty.
                Source::Beblia => get_with_progress(&beblia::download_url(&item.file), 64 * 1024 * 1024, &progress).and_then(|bytes| {
                    let details = Details { code: item.id.clone(), language: item.language_english.clone(), short: item.code.clone(), ..Details::default() };
                    bible_file::install_bytes(&item.file, &bytes, &target, &details).map_err(|error| error.to_string())
                }),
            };
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
        if self.onboard.is_none() {
            self.say(t::bible_added(&installed.name, installed.verses), None);
        }
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
                        Err(other) => self.say(t::bibles_file_failed(&crate::why::bible_file(&other)), None),
                    }
                }
                Event::Catalogue(Ok(list)) => {
                    self.desk.catalogue = Catalogue::Ready(list);
                    self.desk.rows_for = None;
                }
                Event::Catalogue(Err(why)) => self.desk.catalogue = Catalogue::Failed(why),
                Event::Notice(id, Ok((name, copyright))) => {
                    self.desk.notices.insert(id, Notice::Known { name, copyright });
                }
                Event::Notice(id, Err(_)) => {
                    self.desk.notices.insert(id, Notice::Failed);
                }
                Event::Downloaded(result) => {
                    let id = self.desk.downloading.take();
                    match result {
                        Ok(installed) => {
                            let code = installed.code.clone();
                            self.bible_installed(installed);
                            // The Bible the first run asked for becomes the window's own.
                            if let Some(id) = id {
                                self.onboard_bible_arrived(&id, &code);
                            }
                        }
                        Err(why) => {
                            // The first run shows its own reason on its getting-ready page.
                            let shown = id.as_deref().is_some_and(|id| self.onboard_bible_failed(id, &why));
                            if !shown {
                                self.say(t::bibles_download_failed(&why), None);
                            }
                        }
                    }
                }
            }
        }
        if self.sheet == Sheet::Bibles || self.onboard.is_some() {
            changed |= self.desk.arrange();
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole road a press of Download takes, against the real ebible.org: the list is
    /// fetched, one small translation is downloaded, converted and listed. Run by hand, since
    /// it needs the internet: `cargo test -p artosia a_real_download -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn a_real_download_lands_in_the_library() {
        let folder = std::env::temp_dir().join(format!("artosia-bible-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a scratch library");
        // SAFETY: this test is run alone by name; nothing else reads the environment meanwhile.
        unsafe { std::env::set_var("ARTOSIA_DATA", &folder) };
        let mut app = App::load();
        let mut tx = Tx::default();
        let mut wait = |app: &mut App, done: &dyn Fn(&App) -> bool| {
            for _ in 0..600 {
                app.poll_desk(&mut tx);
                if done(app) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            false
        };
        app.start_catalogue();
        assert!(wait(&mut app, &|app| !matches!(app.desk.catalogue, Catalogue::Loading)), "the list never arrived");
        let listed = app.desk.translations().len();
        assert!(listed > 500, "only {listed} translations listed");
        // A small Indonesian-language translation, 473 KB on 2026-11-11.
        app.download_bible("indags");
        assert!(app.desk.downloading.is_some(), "the download did not start");
        assert!(wait(&mut app, &|app| app.desk.downloading.is_none()), "the download never ended");
        let code = installed_code("indags");
        assert!(app.bible_files.iter().any(|file| file.code == code), "{code} is not in the Bible list: {:?}", app.toast.as_ref().map(|toast| toast.text.clone()));
        println!("{listed} translations listed; {code} installed in {}", folder.join("bibles").display());
        let _ = std::fs::remove_dir_all(&folder);
    }

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
        let list = load_ebible(Some(cache)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(list.len(), 2);
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// What an installed Bible is called: its module's own short code, else the code of the
    /// repository file it came from, else eBible's saved list, else its own code; two that
    /// would be called the same keep their own codes.
    #[test]
    fn an_installed_bible_is_called_by_its_short_code() {
        let folder = std::env::temp_dir().join(format!("artosia-desk-short-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let bibles = folder.join("bibles");
        std::fs::create_dir_all(&bibles).unwrap_or_else(|e| panic!("{e}"));
        let xml = r#"<bible translation="Made up for a test"><testament name="New"><book number="43"><chapter number="3"><verse number="16">Made up words for a test.</verse></chapter></book></testament></bible>"#;
        let install = |code: &str, short: &str| {
            let details = Details { code: code.into(), short: short.into(), ..Details::default() };
            bible_file::install_bytes("MadeBible.xml", xml.as_bytes(), &bibles, &details).unwrap_or_else(|e| panic!("{e}"));
        };
        // The repository's file of the Terjemahan Baru and an eBible translation, as installed before 0.1.18.
        install("Indonesian", "");
        install("indmade", "");
        install("zedone", "ZED");
        install("zedtwo", "ZED");
        install("ownfile", "");
        std::fs::write(folder.join(CATALOGUE_FILE), CSV).unwrap_or_else(|e| panic!("{e}"));
        let mut app = App::practice();
        app.folder = Some(folder.clone());
        app.list_bibles();
        let short = |code: &str| app.bible_files.iter().find(|file| file.code == code).map(|file| file.short.clone()).unwrap_or_default();
        assert_eq!(short("KJV"), "KJV");
        assert_eq!(short("INDONESIAN"), "TB");
        assert_eq!(short("INDMADE"), "MADE");
        assert_eq!((short("ZEDONE"), short("ZEDTWO")), ("ZEDONE".to_string(), "ZEDTWO".to_string()));
        assert_eq!(short("OWNFILE"), "OWNFILE");
        // The caption under a verse says the same, for a Bible loaded as `want_bible` loads it.
        let caption = |code: &str| {
            let file = app.bible_files.iter().find(|file| file.code == code).cloned().unwrap_or_else(|| panic!("no {code}"));
            let mut bible = artosia_core::bible::Bible::open(&file.path.clone().unwrap_or_default()).unwrap_or_else(|e| panic!("{e}"));
            bible.short = file.short.clone();
            let reference = artosia_core::bible::Reference { book: 43, chapter: 3, verse: 16, end_chapter: 3, end_verse: 16 };
            artosia_core::deck::passage_deck(&bible, &reference, None, 1).slides[0].caption.clone()
        };
        assert!(caption("INDONESIAN").ends_with(" TB"), "{}", caption("INDONESIAN"));
        assert!(caption("INDMADE").ends_with(" MADE"), "{}", caption("INDMADE"));
        let path = app.bible_files.iter().find(|file| file.code == "INDONESIAN").and_then(|file| file.path.clone()).unwrap_or_else(|| panic!("no file"));
        assert_eq!(artosia_core::bible::Bible::open(&path).unwrap_or_else(|e| panic!("{e}")).short, "TB");
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

    const TREE: &str = r#"{"tree":[{"path":"IndonesianMadeBible.xml","type":"blob","size":1000},{"path":"README.md","type":"blob","size":3}]}"#;

    #[test]
    fn a_fresh_saved_beblia_list_is_used_without_the_network() {
        let folder = std::env::temp_dir().join(format!("artosia-desk-beblia-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
        let cache = folder.join(BEBLIA_FILE);
        std::fs::write(&cache, TREE).unwrap_or_else(|e| panic!("{e}"));
        let list = load_beblia(Some(cache)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((list.len(), list[0].title.as_str(), list[0].source), (1, "Indonesian Made", Source::Beblia));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn both_sources_share_one_arranged_list_with_indonesian_first() {
        let mut list = ebible::parse_catalogue(CSV);
        list.extend(beblia::parse_tree(TREE));
        let mut desk = Desk { catalogue: Catalogue::Ready(list), ..Desk::default() };
        desk.arrange();
        // Indonesian, its eBible row, its Beblia row, then Zedish.
        assert!(matches!(&desk.rows[0], Row::Language { name, .. } if name == "Indonesian"));
        let sources: Vec<Source> = desk.rows.iter().filter_map(|row| if let Row::Translation(at) = row { Some(desk.translations()[*at].source) } else { None }).collect();
        assert_eq!(sources, [Source::Ebible, Source::Beblia, Source::Ebible]);
    }

    #[test]
    fn a_files_own_name_replaces_the_derived_one() {
        let mut desk = Desk { catalogue: Catalogue::Ready(beblia::parse_tree(TREE)), ..Desk::default() };
        let item = desk.translations()[0].clone();
        assert_eq!(desk.title_of(&item), "Indonesian Made");
        desk.notices.insert(item.id.clone(), Notice::Known { name: "Made Up Name".into(), copyright: String::new() });
        assert_eq!(desk.title_of(&item), "Made Up Name");
        desk.notices.insert(item.id.clone(), Notice::Known { name: String::new(), copyright: String::new() });
        assert_eq!(desk.title_of(&item), "Indonesian Made");
    }

    /// A real Indonesian file from the Beblia repository, downloaded into a temporary library
    /// and counted, never printed: `cargo test -p artosia a_real_beblia -- --ignored --nocapture`.
    #[test]
    #[ignore = "uses the network"]
    fn a_real_beblia_download_lands_in_the_library() {
        let folder = std::env::temp_dir().join(format!("artosia-beblia-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let list = load_beblia(None).unwrap_or_else(|e| panic!("{e}"));
        let indonesian: Vec<&Translation> = list.iter().filter(|item| item.language_english == "Indonesian").collect();
        assert!(list.len() > 500 && indonesian.len() >= 5, "{} files, {} Indonesian", list.len(), indonesian.len());
        // The smallest Indonesian file keeps the test quick.
        let item = indonesian.iter().min_by_key(|item| item.size).copied().unwrap_or_else(|| panic!("no Indonesian file"));
        let head = get_head(&beblia::download_url(&item.file), beblia::HEADER_BYTES).unwrap_or_else(|e| panic!("{e}"));
        let (name, copyright) = bible_file::beblia_header(&String::from_utf8_lossy(&head)).unwrap_or_else(|| panic!("no header"));
        assert!(!name.is_empty() && head.len() as u64 <= beblia::HEADER_BYTES);
        let bytes = get(&beblia::download_url(&item.file), 64 * 1024 * 1024).unwrap_or_else(|e| panic!("{e}"));
        let bibles = folder.join("bibles");
        let details = Details { code: item.id.clone(), language: item.language_english.clone(), ..Details::default() };
        let installed = bible_file::install_bytes(&item.file, &bytes, &bibles, &details).unwrap_or_else(|e| panic!("{e}"));
        let bible = artosia_core::bible::Bible::open(&bibles.join(format!("{}.sqlite3", bible_file::file_code(&item.id)))).unwrap_or_else(|e| panic!("{e}"));
        let chapters: usize = bible.books().iter().map(|book| book.chapters.len()).sum();
        assert!(!bible.books().is_empty() && chapters > 0 && installed.verses > 0);
        assert_eq!(installed.name, name);
        println!(
            "{} files listed, {} Indonesian; {} installed: {} books, {chapters} chapters, {} verses; the file states a copyright line: {}",
            list.len(),
            indonesian.len(),
            item.file,
            bible.books().len(),
            installed.verses,
            !copyright.is_empty()
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_installed_code_matches_what_the_reader_lists() {
        assert_eq!(installed_code("indags"), "INDAGS");
        assert_eq!(installed_code("zlmKSZI"), "ZLMKSZI");
    }
}
