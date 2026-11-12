//! The first run, as a state machine with no window: seven pages, one question on each, every
//! one already answered from what Windows knows. The window (`ui/onboard.rs`) draws what this
//! says and sends it what a person pressed; what a press must start or store comes back as
//! `Effect`s for the window to carry out (`docs/operator.md`, "The first run").
//!
//! Pages: language, country, Bible, songs, screens, getting ready, done. Nothing here blocks a
//! person: every page can be left with a button, Skip setup closes from any of them, and a
//! failed download or import is a line on the getting-ready page.

use std::path::PathBuf;

use artosia_core::countries;
use artosia_core::ebible::{self, Translation};
use artosia_core::text::normalize;
use rune::TextEdit;

/// The country of a church whose computer does not say where it is: the church Artosia was built for.
pub const FALLBACK_COUNTRY: &str = "ID";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Page {
    Language,
    Country,
    Bible,
    Songs,
    Screens,
    Ready,
    Done,
}

impl Page {
    pub const ALL: [Page; 7] = [Page::Language, Page::Country, Page::Bible, Page::Songs, Page::Screens, Page::Ready, Page::Done];

    /// Its place from 1, for the dots.
    pub fn number(self) -> usize {
        Page::ALL.iter().position(|page| *page == self).map_or(1, |at| at + 1)
    }

    fn before(self) -> Page {
        Page::ALL[self.number().saturating_sub(2)]
    }
}

/// Where the church's songs are.
#[derive(Clone, Debug, PartialEq)]
pub enum Songs {
    /// EasiSlides' own folder on this computer.
    Easislides,
    /// A folder a person chose: EasiSlides' data or OpenLyrics files.
    Folder(PathBuf),
    Nowhere,
}

/// One of the three rows of the songs page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SongRow {
    Easislides,
    Folder,
    Nowhere,
}

/// The Bible's job on the getting-ready page.
#[derive(Clone, Debug, PartialEq)]
pub enum BibleJob {
    /// Nothing started: the page was not passed.
    Idle,
    /// Asked for, not yet running.
    Waiting,
    /// Running; the percent when the server says how long the file is.
    Downloading(Option<u8>),
    Done,
    /// The reason is `why.rs`'s string.
    Failed(String),
    /// The person went on without one.
    Skipped,
}

/// What an import did, as `import_summary` counts it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub added: usize,
    pub updated: usize,
    pub kept: usize,
    /// Files that could not be read.
    pub failed: usize,
}

/// The songs' job on the getting-ready page.
#[derive(Clone, Debug, PartialEq)]
pub enum SongsJob {
    /// No import was asked for.
    Idle,
    /// The files are being read.
    Reading,
    Importing { done: usize, of: usize },
    Done(Summary),
}

/// One row of the getting-ready page.
#[derive(Clone, Debug, PartialEq)]
pub enum JobRow {
    /// The language and the country, answered on the first two pages.
    LanguageAndCountry,
    Bible { title: String, job: BibleJob },
    Songs(SongsJob),
}

/// What a button does. Each page offers some of these (`Onboard::buttons`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Yes,
    Next,
    SkipForNow,
    TryAgain,
    /// The room sees the test picture.
    Works,
    /// Moves the test picture to the next screen.
    TryOther,
    TakeTour,
    OpenPlan,
    NewSong,
}

/// The buttons of a page besides Back and Skip setup, which every page has (Back from the second).
#[derive(Clone, Debug, PartialEq)]
pub struct Buttons {
    pub primary: Action,
    /// False while the page has nothing to go on with yet (the list of Bibles is loading).
    pub primary_enabled: bool,
    pub others: Vec<Action>,
}

/// How a first run closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    /// Skip setup: what was answered stays, the rest is left.
    Skip,
    Tour,
    Plan,
    NewSong,
}

/// What the window must do because of a press.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    /// The window reads in this language from now on, and it is saved.
    SetLanguage(String),
    /// Saved in settings as two capital letters.
    SaveCountry(String),
    /// Ask eBible and Beblia for their lists.
    LoadCatalogue,
    /// Download and install this translation in the background, then use it.
    Download(String),
    ImportEasislides,
    ImportFolder(PathBuf),
    /// Open the folder picker.
    ChooseFolder,
    /// Put the test picture on this screen (the numbers the Screens dialog's Identify draws).
    ShowTestPicture(usize),
    HideTestPicture,
    /// The projector is this screen from now on, and it is saved.
    UseScreen(usize),
    Finish(Finish),
}

/// One screen as the output process listed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    /// The screen Artosia's own window is on.
    pub primary: bool,
}

/// The three plain rows of the screens page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRows {
    /// Screen numbers from 1.
    pub window: usize,
    pub picture: usize,
    pub width: u32,
    pub height: u32,
}

/// What the machine is told at the start.
pub struct Facts {
    /// The codes of the languages the window offers.
    pub offered: Vec<String>,
    pub language: String,
    pub country: String,
    /// The songs EasiSlides holds on this computer, when it is there.
    pub easislides: Option<usize>,
    /// The library already holds songs: the first run was opened again, so nothing is brought
    /// in unless the person asks for it.
    pub has_songs: bool,
}

pub struct Onboard {
    pub page: Page,
    pub offered: Vec<String>,
    pub language: String,
    pub country: String,
    /// What is typed to find a country.
    pub query: TextEdit,
    /// The translation picked on the Bible page, by id, and its title for the getting-ready page.
    pub bible: Option<(String, String)>,
    /// A person picked it; Artosia's own default no longer moves it.
    bible_chosen: bool,
    pub songs: Songs,
    pub easislides: Option<usize>,
    pub bible_job: BibleJob,
    pub songs_job: SongsJob,
    /// The list of Bibles could not be had, with the reason.
    pub catalogue_failed: Option<String>,
    /// The list of Bibles has arrived.
    pub catalogue_ready: bool,
    pub screens: Vec<Screen>,
    /// The screen the test picture is on.
    pub shown: Option<usize>,
    /// The screen the output process says the projector is on.
    audience: Option<usize>,
}

/// The language the window starts in: the first of Windows' languages that Artosia offers,
/// English when none fits.
pub fn default_language(offered: &[&str], wanted: &[String]) -> String {
    artosia_locale::pick_first(wanted, offered).unwrap_or("en").to_string()
}

/// The country the page starts on: the one Windows is set to when it is a country Artosia knows,
/// else Indonesia.
pub fn default_country(region: Option<&str>) -> String {
    region.map(str::to_ascii_uppercase).filter(|code| countries::name(code, "en").is_some()).unwrap_or_else(|| FALLBACK_COUNTRY.to_string())
}

/// Whether the first run opens by itself at this start: a real window on a library that had no
/// settings before this start, or whose first run was left unfinished. A church that already
/// uses Artosia has settings and no such mark, so it never sees it.
pub fn opens_at_start(has_library: bool, had_settings: bool, unfinished: bool, headless: bool) -> bool {
    has_library && !headless && (!had_settings || unfinished)
}

/// The translation the Bible page starts on: the first whole Bible of the first language of the
/// country that has one; failing that, the first translation of the first language.
pub fn default_bible(list: &[Translation], country: &str) -> Option<(String, String)> {
    let groups = ebible::grouped(list, "", &ebible::ranking(country));
    let whole = groups.iter().flat_map(|group| group.items.iter()).find(|at| list[**at].whole_bible());
    let first = groups.first().and_then(|group| group.items.first());
    whole.or(first).map(|at| (list[*at].id.clone(), list[*at].title.clone()))
}

/// The id after `current` by `delta` places, held at both ends. With nothing current, the first.
fn stepped(ids: &[String], current: Option<&str>, delta: i32) -> Option<String> {
    let at = current.and_then(|id| ids.iter().position(|each| each == id));
    let to = match at {
        Some(at) => (at as i64 + i64::from(delta)).clamp(0, ids.len() as i64 - 1) as usize,
        None => 0,
    };
    ids.get(to).cloned()
}

impl Onboard {
    pub fn new(facts: Facts) -> Onboard {
        Onboard {
            page: Page::Language,
            offered: facts.offered,
            language: facts.language,
            country: facts.country,
            query: TextEdit::new(""),
            bible: None,
            bible_chosen: false,
            songs: if facts.easislides.is_some() && !facts.has_songs { Songs::Easislides } else { Songs::Nowhere },
            easislides: facts.easislides,
            bible_job: BibleJob::Idle,
            songs_job: SongsJob::Idle,
            catalogue_failed: None,
            catalogue_ready: false,
            screens: Vec::new(),
            shown: None,
            audience: None,
        }
    }

    /// What the window does as the first run opens: the language Windows asked for, and the list of Bibles.
    pub fn start(&self) -> Vec<Effect> {
        vec![Effect::SetLanguage(self.language.clone()), Effect::LoadCatalogue]
    }

    // ---- What a page shows ----

    /// The countries the filter leaves, by name in the window's language.
    pub fn visible_countries(&self) -> Vec<(&'static str, &'static str)> {
        let words: Vec<String> = normalize(&self.query.text).split_whitespace().map(str::to_string).collect();
        countries::names(&self.language).into_iter().filter(|(_, name)| {
            let hay = normalize(name);
            words.iter().all(|word| hay.contains(word.as_str()))
        }).collect()
    }

    /// The rows of the songs page, in order: EasiSlides only where it was found.
    pub fn song_rows(&self) -> Vec<SongRow> {
        let mut rows = Vec::new();
        if self.easislides.is_some() {
            rows.push(SongRow::Easislides);
        }
        rows.push(SongRow::Folder);
        rows.push(SongRow::Nowhere);
        rows
    }

    pub fn song_row_picked(&self, row: SongRow) -> bool {
        matches!((&self.songs, row), (Songs::Easislides, SongRow::Easislides) | (Songs::Folder(_), SongRow::Folder) | (Songs::Nowhere, SongRow::Nowhere))
    }

    /// Whether the page's own pick can still change: a Bible or an import that was started stays as asked.
    pub fn locked(&self) -> bool {
        match self.page {
            Page::Bible => !matches!(self.bible_job, BibleJob::Idle),
            Page::Songs => !matches!(self.songs_job, SongsJob::Idle),
            _ => false,
        }
    }

    /// The buttons of the page, other than Back and Skip setup.
    pub fn buttons(&self) -> Buttons {
        let (primary, primary_enabled, others) = match self.page {
            Page::Language | Page::Country => (Action::Yes, true, vec![]),
            Page::Bible if self.catalogue_failed.is_some() => (Action::TryAgain, true, vec![Action::SkipForNow]),
            Page::Bible => (Action::Next, self.bible.is_some() || self.locked(), vec![Action::SkipForNow]),
            Page::Songs | Page::Ready => (Action::Next, true, vec![]),
            Page::Screens if self.two_screens() => (Action::Works, true, vec![Action::TryOther]),
            Page::Screens => (Action::Next, true, vec![]),
            Page::Done => (Action::TakeTour, true, vec![Action::OpenPlan, Action::NewSong]),
        };
        Buttons { primary, primary_enabled, others }
    }

    /// A projector can be told from the window only with a second screen.
    pub fn two_screens(&self) -> bool {
        self.screens.len() >= 2 && self.target().is_some()
    }

    /// The screen that would be the projector: the one the output process uses when it is not the window's own, else the first that is not.
    fn target(&self) -> Option<usize> {
        self.audience.filter(|at| self.screens.get(*at).is_some_and(|screen| !screen.primary)).or_else(|| self.screens.iter().position(|screen| !screen.primary))
    }

    /// The three rows of the screens page; None with one screen.
    pub fn screen_rows(&self) -> Option<ScreenRows> {
        let picture = self.shown?;
        let window = self.screens.iter().position(|screen| screen.primary).unwrap_or(0);
        let screen = self.screens.get(picture)?;
        Some(ScreenRows { window: window + 1, picture: picture + 1, width: screen.width, height: screen.height })
    }

    /// The rows of the getting-ready page.
    pub fn jobs(&self) -> Vec<JobRow> {
        let mut rows = vec![JobRow::LanguageAndCountry];
        if !matches!(self.bible_job, BibleJob::Idle | BibleJob::Skipped) {
            rows.push(JobRow::Bible { title: self.bible.as_ref().map(|(_, title)| title.clone()).unwrap_or_default(), job: self.bible_job.clone() });
        }
        if self.songs_job != SongsJob::Idle {
            rows.push(JobRow::Songs(self.songs_job.clone()));
        }
        rows
    }

    // ---- What a person did ----

    pub fn pick_language(&mut self, code: &str) -> Vec<Effect> {
        if !self.offered.iter().any(|offered| offered == code) {
            return Vec::new();
        }
        self.language = code.to_string();
        vec![Effect::SetLanguage(code.to_string())]
    }

    pub fn pick_country(&mut self, code: &str) {
        if countries::name(code, "en").is_some() {
            self.country = code.to_string();
        }
    }

    /// The filter changed. A pick the filter hides gives way to the first country it shows, so Yes always means one on the page.
    pub fn query_changed(&mut self) {
        let visible = self.visible_countries();
        if !visible.is_empty() && !visible.iter().any(|(code, _)| *code == self.country) {
            self.country = visible[0].0.to_string();
        }
    }

    /// Picks a translation by hand. Nothing once a download of one has begun.
    pub fn pick_bible(&mut self, id: &str, title: &str) {
        if self.locked() {
            return;
        }
        self.bible = Some((id.to_string(), title.to_string()));
        self.bible_chosen = true;
    }

    /// The list of Bibles arrived, or the country changed: the default is the country's first whole Bible, unless a person picked.
    pub fn retarget(&mut self, list: &[Translation]) {
        self.catalogue_ready = true;
        self.catalogue_failed = None;
        if !self.bible_chosen && !self.locked() {
            self.bible = default_bible(list, &self.country);
        }
    }

    pub fn catalogue_failed(&mut self, why: String) {
        self.catalogue_ready = false;
        self.catalogue_failed = Some(why);
    }

    /// Picks a row of the songs page. The folder row opens the picker and is picked once a folder is chosen.
    pub fn pick_songs(&mut self, row: SongRow) -> Vec<Effect> {
        if self.locked() {
            return Vec::new();
        }
        match row {
            SongRow::Easislides if self.easislides.is_some() => self.songs = Songs::Easislides,
            SongRow::Easislides => {}
            SongRow::Nowhere => self.songs = Songs::Nowhere,
            SongRow::Folder => return vec![Effect::ChooseFolder],
        }
        Vec::new()
    }

    /// The picker answered. A cancelled picker leaves the pick as it was.
    pub fn folder_chosen(&mut self, folder: Option<PathBuf>) {
        if let (Some(folder), false) = (folder, self.locked()) {
            self.songs = Songs::Folder(folder);
        }
    }

    /// An arrow key in the page's list. `bibles` are the ids and titles on the Bible page, in the order drawn.
    pub fn step(&mut self, delta: i32, bibles: &[(String, String)]) -> Vec<Effect> {
        match self.page {
            Page::Language => {
                let ids = self.offered.clone();
                match stepped(&ids, Some(&self.language), delta) {
                    Some(code) => self.pick_language(&code),
                    None => Vec::new(),
                }
            }
            Page::Country => {
                let ids: Vec<String> = self.visible_countries().iter().map(|(code, _)| (*code).to_string()).collect();
                if let Some(code) = stepped(&ids, Some(&self.country), delta) {
                    self.country = code;
                }
                Vec::new()
            }
            Page::Bible => {
                let ids: Vec<String> = bibles.iter().map(|(id, _)| id.clone()).collect();
                if let Some(id) = stepped(&ids, self.bible.as_ref().map(|(id, _)| id.as_str()), delta)
                    && let Some((id, title)) = bibles.iter().find(|(each, _)| *each == id)
                {
                    self.pick_bible(id, title);
                }
                Vec::new()
            }
            Page::Songs => {
                let rows = self.song_rows();
                let at = rows.iter().position(|row| self.song_row_picked(*row)).unwrap_or(0);
                let to = (at as i64 + i64::from(delta)).clamp(0, rows.len() as i64 - 1) as usize;
                // The folder row opens a picker: arrows rest on the rows that need none.
                match rows[to] {
                    SongRow::Folder => Vec::new(),
                    row => self.pick_songs(row),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Presses a button of the page. A button the page does not have does nothing.
    pub fn press(&mut self, action: Action) -> Vec<Effect> {
        let buttons = self.buttons();
        if action != buttons.primary && !buttons.others.contains(&action) || action == buttons.primary && !buttons.primary_enabled {
            return Vec::new();
        }
        match (self.page, action) {
            (Page::Language, Action::Yes) => self.go(Page::Country),
            (Page::Country, Action::Yes) => {
                let mut effects = vec![Effect::SaveCountry(self.country.clone())];
                effects.extend(self.go(Page::Bible));
                effects
            }
            (Page::Bible, Action::TryAgain) => {
                self.catalogue_failed = None;
                vec![Effect::LoadCatalogue]
            }
            (Page::Bible, Action::Next) => {
                let mut effects = Vec::new();
                if matches!(self.bible_job, BibleJob::Idle)
                    && let Some((id, _)) = &self.bible
                {
                    self.bible_job = BibleJob::Waiting;
                    effects.push(Effect::Download(id.clone()));
                }
                effects.extend(self.go(Page::Songs));
                effects
            }
            (Page::Bible, Action::SkipForNow) => {
                if matches!(self.bible_job, BibleJob::Idle) {
                    self.bible_job = BibleJob::Skipped;
                }
                self.go(Page::Songs)
            }
            (Page::Songs, Action::Next) => {
                let mut effects = Vec::new();
                if matches!(self.songs_job, SongsJob::Idle) {
                    match &self.songs {
                        Songs::Easislides => {
                            self.songs_job = SongsJob::Reading;
                            effects.push(Effect::ImportEasislides);
                        }
                        Songs::Folder(folder) => {
                            self.songs_job = SongsJob::Reading;
                            effects.push(Effect::ImportFolder(folder.clone()));
                        }
                        Songs::Nowhere => {}
                    }
                }
                effects.extend(self.go(Page::Screens));
                effects
            }
            (Page::Screens, Action::Works) => {
                let mut effects: Vec<Effect> = self.shown.map(Effect::UseScreen).into_iter().collect();
                effects.extend(self.go(Page::Ready));
                effects
            }
            (Page::Screens, Action::TryOther) => {
                let others: Vec<usize> = (0..self.screens.len()).filter(|at| !self.screens[*at].primary).collect();
                let now = self.shown.or_else(|| self.target());
                let next = now.and_then(|now| others.iter().find(|at| **at > now).or(others.first())).copied();
                self.shown = next;
                next.map(Effect::ShowTestPicture).into_iter().collect()
            }
            (Page::Screens, Action::Next) => self.go(Page::Ready),
            (Page::Ready, Action::Next) => self.go(Page::Done),
            (Page::Done, Action::TakeTour) => vec![Effect::Finish(Finish::Tour)],
            (Page::Done, Action::OpenPlan) => vec![Effect::Finish(Finish::Plan)],
            (Page::Done, Action::NewSong) => vec![Effect::Finish(Finish::NewSong)],
            _ => Vec::new(),
        }
    }

    /// Back, as Escape does. Nothing on the first page.
    pub fn back(&mut self) -> Vec<Effect> {
        if self.page == Page::Language {
            return Vec::new();
        }
        self.go(self.page.before())
    }

    /// Skip setup, from any page: what was answered stays, and the first run closes.
    pub fn skip(&mut self) -> Vec<Effect> {
        let mut effects = self.leave();
        effects.push(Effect::Finish(Finish::Skip));
        effects
    }

    /// Leaves the page; the test picture goes with the screens page.
    fn leave(&mut self) -> Vec<Effect> {
        if self.page == Page::Screens && self.shown.take().is_some() {
            return vec![Effect::HideTestPicture];
        }
        Vec::new()
    }

    fn go(&mut self, page: Page) -> Vec<Effect> {
        let mut effects = self.leave();
        self.page = page;
        effects.extend(self.sync_screens());
        effects
    }

    // ---- What the world said ----

    /// The output process listed its screens, and says where the projector is. On the screens
    /// page with a second screen the test picture goes on it.
    pub fn screens_seen(&mut self, screens: Vec<Screen>, audience: Option<usize>) -> Vec<Effect> {
        self.screens = screens;
        self.audience = audience;
        self.sync_screens()
    }

    fn sync_screens(&mut self) -> Vec<Effect> {
        if self.page != Page::Screens || self.shown.is_some() {
            return Vec::new();
        }
        match self.target().filter(|_| self.two_screens()) {
            Some(at) => {
                self.shown = Some(at);
                vec![Effect::ShowTestPicture(at)]
            }
            None => Vec::new(),
        }
    }

    /// The download's percent, when the server gave the length.
    pub fn bible_progress(&mut self, percent: Option<u8>) {
        if matches!(self.bible_job, BibleJob::Waiting | BibleJob::Downloading(_)) {
            self.bible_job = BibleJob::Downloading(percent);
        }
    }

    pub fn bible_done(&mut self) {
        self.bible_job = BibleJob::Done;
    }

    pub fn bible_failed(&mut self, why: String) {
        self.bible_job = BibleJob::Failed(why);
    }

    /// The files are read: `of` songs wait to go in.
    pub fn songs_read(&mut self, of: usize) {
        self.songs_job = SongsJob::Importing { done: 0, of };
    }

    pub fn songs_progress(&mut self, done: usize, of: usize) {
        self.songs_job = SongsJob::Importing { done, of };
    }

    pub fn songs_done(&mut self, summary: Summary) {
        self.songs_job = SongsJob::Done(summary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(easislides: Option<usize>) -> Facts {
        Facts { offered: vec!["en".into(), "id".into()], language: "id".into(), country: "ID".into(), easislides, has_songs: false }
    }

    fn at(page: Page, easislides: Option<usize>) -> Onboard {
        let mut onboard = Onboard::new(facts(easislides));
        onboard.page = page;
        onboard
    }

    fn screen(width: u32, primary: bool) -> Screen {
        Screen { width, height: width * 9 / 16, primary }
    }

    fn translation(id: &str, code: &str, old: u16, new: u16) -> Translation {
        Translation { id: id.into(), title: format!("Title {id}"), language_code: code.into(), language_english: code.into(), old_books: old, new_books: new, ..Translation::default() }
    }

    #[test]
    fn the_language_defaults_to_the_first_of_windows_that_is_offered() {
        let offered = ["en", "id"];
        let wanted = |tags: &[&str]| tags.iter().map(|tag| tag.to_string()).collect::<Vec<_>>();
        assert_eq!(default_language(&offered, &wanted(&["jv-ID", "id-ID", "en-US"])), "id");
        assert_eq!(default_language(&offered, &wanted(&["en-GB", "id-ID"])), "en");
        assert_eq!(default_language(&offered, &wanted(&["de-DE"])), "en", "English when none fits");
        assert_eq!(default_language(&offered, &[]), "en");
    }

    #[test]
    fn the_country_defaults_to_windows_and_else_indonesia() {
        assert_eq!(default_country(Some("BR")), "BR");
        assert_eq!(default_country(Some("br")), "BR");
        assert_eq!(default_country(None), "ID");
        assert_eq!(default_country(Some("ZZ")), "ID", "a place that is no country");
        assert_eq!(default_country(Some("001")), "ID");
    }

    #[test]
    fn it_opens_only_on_a_real_window_and_a_library_with_no_settings_file() {
        assert!(opens_at_start(true, false, false, false));
        assert!(!opens_at_start(true, true, false, false), "a church that already uses Artosia never sees it");
        assert!(opens_at_start(true, true, true, false), "a first run closed with the window comes back");
        assert!(!opens_at_start(false, false, false, false), "practice has no library");
        assert!(!opens_at_start(true, false, false, true), "never headless");
        // Opened again on a library with songs, nothing is brought in unless asked.
        let again = Onboard::new(Facts { has_songs: true, ..facts(Some(961)) });
        assert!(again.song_row_picked(SongRow::Nowhere) && again.song_rows().contains(&SongRow::Easislides));
    }

    #[test]
    fn it_starts_on_the_language_page_with_windows_answers_in_place() {
        let onboard = Onboard::new(facts(None));
        assert_eq!((onboard.page, onboard.language.as_str(), onboard.country.as_str()), (Page::Language, "id", "ID"));
        assert_eq!(onboard.start(), [Effect::SetLanguage("id".into()), Effect::LoadCatalogue]);
        assert_eq!(Page::ALL.iter().map(|page| page.number()).collect::<Vec<_>>(), [1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn picking_a_language_changes_the_window_at_once_and_only_to_an_offered_one() {
        let mut onboard = at(Page::Language, None);
        assert_eq!(onboard.pick_language("en"), [Effect::SetLanguage("en".into())]);
        assert_eq!(onboard.language, "en");
        assert!(onboard.pick_language("xx").is_empty());
        assert_eq!(onboard.language, "en");
        assert_eq!(onboard.step(1, &[]), [Effect::SetLanguage("id".into())]);
        onboard.step(1, &[]);
        assert_eq!(onboard.language, "id", "held at the end");
        assert_eq!(onboard.step(-5, &[]), [Effect::SetLanguage("en".into())]);
    }

    #[test]
    fn each_page_has_one_primary_button_and_the_others_it_should() {
        let mut onboard = at(Page::Language, Some(5));
        let of = |onboard: &Onboard| {
            let buttons = onboard.buttons();
            (buttons.primary, buttons.others)
        };
        assert_eq!(of(&onboard), (Action::Yes, vec![]));
        onboard.page = Page::Country;
        assert_eq!(of(&onboard), (Action::Yes, vec![]));
        onboard.page = Page::Bible;
        onboard.bible = Some(("a".into(), "A".into()));
        assert_eq!(of(&onboard), (Action::Next, vec![Action::SkipForNow]));
        onboard.page = Page::Songs;
        assert_eq!(of(&onboard), (Action::Next, vec![]));
        onboard.page = Page::Screens;
        assert_eq!(of(&onboard), (Action::Next, vec![]), "no second screen yet");
        onboard.page = Page::Ready;
        assert_eq!(of(&onboard), (Action::Next, vec![]));
        assert!(onboard.buttons().primary_enabled, "Next is always pressable here");
        onboard.page = Page::Done;
        assert_eq!(of(&onboard), (Action::TakeTour, vec![Action::OpenPlan, Action::NewSong]));
    }

    #[test]
    fn yes_on_the_country_page_saves_two_capital_letters_and_goes_on() {
        let mut onboard = at(Page::Country, None);
        onboard.pick_country("BR");
        onboard.pick_country("zz");
        assert_eq!(onboard.country, "BR", "a code that is no country changes nothing");
        assert_eq!(onboard.press(Action::Yes), [Effect::SaveCountry("BR".into())]);
        assert_eq!(onboard.page, Page::Bible);
    }

    #[test]
    fn typing_filters_the_countries_in_the_windows_language_and_yes_keeps_a_visible_one() {
        let mut onboard = at(Page::Country, None);
        let all = onboard.visible_countries().len();
        assert!(all > 190);
        onboard.query.replace_text("jerm");
        onboard.query_changed();
        let visible = onboard.visible_countries();
        assert_eq!(visible.iter().map(|(code, _)| *code).collect::<Vec<_>>(), ["DE"], "Jerman is Germany in Indonesian");
        assert_eq!(onboard.country, "DE", "Indonesia is hidden, so the pick moves to what shows");
        onboard.query.replace_text("zzzz");
        onboard.query_changed();
        assert!(onboard.visible_countries().is_empty());
        assert_eq!(onboard.country, "DE", "an empty list leaves the pick alone");
        onboard.query.replace_text("");
        onboard.query_changed();
        assert_eq!(onboard.visible_countries().len(), all);
    }

    #[test]
    fn arrows_move_the_country_pick_through_what_the_filter_shows() {
        let mut onboard = at(Page::Country, None);
        onboard.query.replace_text("ja");
        onboard.query_changed();
        let shown: Vec<&str> = onboard.visible_countries().iter().map(|(code, _)| *code).collect();
        assert!(shown.len() > 1);
        assert_eq!(onboard.country, shown[0], "the filter hid the old pick, so the first shown is picked");
        onboard.step(1, &[]);
        assert_eq!(onboard.country, shown[1]);
        onboard.step(-9, &[]);
        assert_eq!(onboard.country, shown[0], "held at the top");
        onboard.step(999, &[]);
        assert_eq!(onboard.country, *shown.last().unwrap_or(&""), "held at the bottom");
    }

    #[test]
    fn the_filter_costs_a_keystroke_well_under_a_frame() {
        let mut onboard = at(Page::Country, None);
        let started = std::time::Instant::now();
        let runs = 50;
        for run in 0..runs {
            onboard.query.replace_text(["i", "in", "ind", "indo"][run % 4]);
            onboard.query_changed();
        }
        let each = started.elapsed() / runs as u32;
        println!("country filter: {each:?} a keystroke over {} countries", countries::names("id").len());
        assert!(each < std::time::Duration::from_millis(8), "{each:?}");
    }

    #[test]
    fn the_bible_defaults_to_the_first_whole_bible_of_the_countrys_first_language() {
        let list = vec![
            translation("zed", "zed", 39, 27),
            translation("indnt", "ind", 0, 27),
            translation("indwhole", "ind", 39, 27),
            translation("jav", "jav", 39, 27),
        ];
        assert_eq!(default_bible(&list, "ID").map(|(id, _)| id), Some("indwhole".into()));
        // Brazil's first language has none here: the next language that has a whole Bible leads.
        let list2 = vec![translation("zed", "zed", 39, 27), translation("porNT", "por", 0, 27)];
        assert_eq!(default_bible(&list2, "BR").map(|(id, _)| id), Some("zed".into()), "none of the country's languages has a whole Bible, so the first whole Bible of the list");
        // No whole Bible at all: the first translation of the first language.
        let list3 = vec![translation("a", "aaa", 0, 27), translation("porNT", "por", 0, 27)];
        assert_eq!(default_bible(&list3, "BR").map(|(id, _)| id), Some("porNT".into()));
        assert_eq!(default_bible(&[], "ID"), None);
    }

    #[test]
    fn a_pick_by_hand_is_not_moved_by_the_default() {
        let list = vec![translation("indwhole", "ind", 39, 27), translation("other", "ind", 0, 27)];
        let mut onboard = at(Page::Bible, None);
        onboard.retarget(&list);
        assert_eq!(onboard.bible.as_ref().map(|(id, _)| id.as_str()), Some("indwhole"));
        onboard.pick_bible("other", "Title other");
        onboard.retarget(&list);
        assert_eq!(onboard.bible.as_ref().map(|(id, _)| id.as_str()), Some("other"));
        // A country answered later moves the default of a person who never picked.
        let mut fresh = at(Page::Bible, None);
        let both = vec![translation("indwhole", "ind", 39, 27), translation("porwhole", "por", 39, 27)];
        fresh.country = "BR".into();
        fresh.retarget(&both);
        assert_eq!(fresh.bible.as_ref().map(|(id, _)| id.as_str()), Some("porwhole"));
    }

    #[test]
    fn next_on_the_bible_page_starts_the_download_and_goes_on() {
        let mut onboard = at(Page::Bible, None);
        onboard.retarget(&[translation("indwhole", "ind", 39, 27)]);
        assert_eq!(onboard.press(Action::Next), [Effect::Download("indwhole".into())]);
        assert_eq!((onboard.page, &onboard.bible_job), (Page::Songs, &BibleJob::Waiting));
        // Back to the page: the pick is as asked and stays, and Next starts nothing a second time.
        onboard.back();
        assert!(onboard.locked());
        onboard.pick_bible("other", "Other");
        assert_eq!(onboard.bible.as_ref().map(|(id, _)| id.as_str()), Some("indwhole"));
        assert!(onboard.press(Action::Next).is_empty());
        assert_eq!(onboard.page, Page::Songs);
    }

    #[test]
    fn skip_for_now_goes_on_without_a_bible() {
        let mut onboard = at(Page::Bible, None);
        onboard.retarget(&[translation("indwhole", "ind", 39, 27)]);
        assert!(onboard.press(Action::SkipForNow).is_empty());
        assert_eq!((onboard.page, &onboard.bible_job), (Page::Songs, &BibleJob::Skipped));
        assert!(onboard.jobs().iter().all(|row| !matches!(row, JobRow::Bible { .. })), "a Bible nobody asked for has no row");
    }

    #[test]
    fn next_waits_while_the_list_of_bibles_loads() {
        let onboard = at(Page::Bible, None);
        assert!(!onboard.buttons().primary_enabled);
        let mut onboard = onboard;
        assert!(onboard.press(Action::Next).is_empty());
        assert_eq!(onboard.page, Page::Bible);
        // Skip for now is open at once.
        assert!(onboard.press(Action::SkipForNow).is_empty());
        assert_eq!(onboard.page, Page::Songs);
    }

    #[test]
    fn with_no_internet_the_bible_page_offers_try_again_and_skip_for_now() {
        let mut onboard = at(Page::Bible, None);
        onboard.catalogue_failed("there is no internet connection".into());
        let buttons = onboard.buttons();
        assert_eq!((buttons.primary, buttons.others.clone()), (Action::TryAgain, vec![Action::SkipForNow]));
        assert_eq!(onboard.press(Action::Next), Vec::<Effect>::new(), "Next is not on this page");
        assert_eq!(onboard.press(Action::TryAgain), [Effect::LoadCatalogue]);
        assert!(onboard.catalogue_failed.is_none(), "the page waits for the new answer");
        onboard.retarget(&[translation("indwhole", "ind", 39, 27)]);
        assert_eq!(onboard.buttons().primary, Action::Next);
    }

    #[test]
    fn a_failed_download_is_a_line_on_the_ready_page_and_blocks_nothing() {
        let mut onboard = at(Page::Bible, None);
        onboard.retarget(&[translation("indwhole", "ind", 39, 27)]);
        onboard.press(Action::Next);
        onboard.bible_progress(Some(40));
        assert_eq!(onboard.bible_job, BibleJob::Downloading(Some(40)));
        onboard.bible_failed("the connection broke on the way".into());
        onboard.page = Page::Ready;
        let rows = onboard.jobs();
        assert_eq!(rows[1], JobRow::Bible { title: "Title indwhole".into(), job: BibleJob::Failed("the connection broke on the way".into()) });
        assert!(onboard.buttons().primary_enabled);
        assert!(onboard.press(Action::Next).is_empty());
        assert_eq!(onboard.page, Page::Done);
        // A progress that arrives after the end changes nothing.
        onboard.bible_progress(Some(90));
        assert!(matches!(onboard.bible_job, BibleJob::Failed(_)));
    }

    #[test]
    fn easislides_found_is_picked_with_its_count_and_not_found_is_not_a_row() {
        let found = at(Page::Songs, Some(961));
        assert_eq!(found.song_rows(), [SongRow::Easislides, SongRow::Folder, SongRow::Nowhere]);
        assert_eq!(found.songs, Songs::Easislides);
        assert_eq!(found.easislides, Some(961));
        let none = at(Page::Songs, None);
        assert_eq!(none.song_rows(), [SongRow::Folder, SongRow::Nowhere]);
        assert_eq!(none.songs, Songs::Nowhere);
        let mut none = none;
        assert!(none.pick_songs(SongRow::Easislides).is_empty());
        assert_eq!(none.songs, Songs::Nowhere, "a row that is not there cannot be picked");
    }

    #[test]
    fn next_on_the_songs_page_starts_the_import_that_was_picked() {
        let mut onboard = at(Page::Songs, Some(961));
        assert_eq!(onboard.press(Action::Next), [Effect::ImportEasislides]);
        assert_eq!((onboard.page, &onboard.songs_job), (Page::Screens, &SongsJob::Reading));
        onboard.songs_read(961);
        onboard.songs_progress(412, 961);
        assert_eq!(onboard.jobs().last(), Some(&JobRow::Songs(SongsJob::Importing { done: 412, of: 961 })));
        let summary = Summary { added: 950, updated: 3, kept: 8, failed: 0 };
        onboard.songs_done(summary);
        assert_eq!(onboard.jobs().last(), Some(&JobRow::Songs(SongsJob::Done(summary))));

        let mut folder = at(Page::Songs, None);
        assert_eq!(folder.pick_songs(SongRow::Folder), [Effect::ChooseFolder]);
        assert_eq!(folder.songs, Songs::Nowhere, "the folder row is picked once a folder is chosen");
        folder.folder_chosen(None);
        assert_eq!(folder.songs, Songs::Nowhere, "a cancelled picker changes nothing");
        folder.folder_chosen(Some(PathBuf::from("songs")));
        assert!(folder.song_row_picked(SongRow::Folder));
        assert_eq!(folder.press(Action::Next), [Effect::ImportFolder(PathBuf::from("songs"))]);

        let mut nowhere = at(Page::Songs, None);
        assert!(nowhere.press(Action::Next).is_empty(), "nowhere yet starts nothing and has no row");
        assert!(nowhere.jobs().iter().all(|row| !matches!(row, JobRow::Songs(_))));
    }

    #[test]
    fn arrows_on_the_songs_page_rest_on_the_rows_that_need_no_picker() {
        let mut onboard = at(Page::Songs, Some(3));
        assert!(onboard.step(1, &[]).is_empty());
        assert_eq!(onboard.songs, Songs::Easislides, "the next row opens a picker, so an arrow stays");
        onboard.step(2, &[]);
        assert_eq!(onboard.songs, Songs::Nowhere);
        onboard.step(-1, &[]);
        assert_eq!(onboard.songs, Songs::Nowhere, "Folder is skipped");
    }

    #[test]
    fn with_one_screen_the_page_has_one_next_and_shows_no_picture() {
        let mut onboard = at(Page::Songs, None);
        let seen = onboard.screens_seen(vec![screen(1920, true)], None);
        assert!(seen.is_empty());
        onboard.press(Action::Next);
        assert_eq!(onboard.page, Page::Screens);
        assert!(!onboard.two_screens());
        assert_eq!(onboard.buttons().primary, Action::Next);
        assert!(onboard.screen_rows().is_none());
        assert_eq!(onboard.press(Action::Next), Vec::<Effect>::new());
        assert_eq!(onboard.page, Page::Ready);
    }

    #[test]
    fn with_two_screens_the_picture_goes_where_the_projector_would_and_yes_keeps_it() {
        let mut onboard = at(Page::Songs, None);
        onboard.screens_seen(vec![screen(1920, true), screen(1280, false)], Some(1));
        assert_eq!(onboard.press(Action::Next), [Effect::ShowTestPicture(1)]);
        assert_eq!(onboard.buttons().primary, Action::Works);
        assert_eq!(onboard.buttons().others, [Action::TryOther]);
        assert_eq!(onboard.screen_rows(), Some(ScreenRows { window: 1, picture: 2, width: 1280, height: 720 }));
        assert_eq!(onboard.press(Action::Works), [Effect::UseScreen(1), Effect::HideTestPicture]);
        assert_eq!(onboard.page, Page::Ready);
    }

    #[test]
    fn try_another_screen_moves_the_picture_on_and_round_and_never_to_the_windows_own() {
        let mut onboard = at(Page::Screens, None);
        let seen = onboard.screens_seen(vec![screen(1920, true), screen(1280, false), screen(1024, false)], None);
        assert_eq!(seen, [Effect::ShowTestPicture(1)], "the first screen that is not this one");
        assert_eq!(onboard.press(Action::TryOther), [Effect::ShowTestPicture(2)]);
        assert_eq!(onboard.press(Action::TryOther), [Effect::ShowTestPicture(1)], "round to the first again, skipping the window's own");
        assert_eq!(onboard.screen_rows().map(|rows| rows.picture), Some(2));
    }

    #[test]
    fn the_picture_waits_for_the_screens_to_be_listed_and_goes_when_the_page_is_left() {
        let mut onboard = at(Page::Screens, None);
        assert!(!onboard.two_screens());
        assert_eq!(onboard.buttons().primary, Action::Next, "until a second screen shows up");
        assert_eq!(onboard.screens_seen(vec![screen(1920, true), screen(1280, false)], None), [Effect::ShowTestPicture(1)]);
        assert!(onboard.screens_seen(vec![screen(1920, true), screen(1280, false)], Some(1)).is_empty(), "shown once");
        assert_eq!(onboard.back(), [Effect::HideTestPicture]);
        assert_eq!(onboard.page, Page::Songs);
    }

    #[test]
    fn back_goes_one_page_and_stops_at_the_first() {
        let mut onboard = at(Page::Language, None);
        assert!(onboard.back().is_empty());
        assert_eq!(onboard.page, Page::Language);
        onboard.page = Page::Done;
        for want in [Page::Ready, Page::Screens, Page::Songs, Page::Bible, Page::Country, Page::Language] {
            onboard.back();
            assert_eq!(onboard.page, want);
        }
    }

    #[test]
    fn skip_setup_on_every_page_closes_and_keeps_what_was_answered() {
        for page in Page::ALL {
            let mut onboard = at(page, Some(2));
            onboard.screens_seen(vec![screen(1920, true), screen(1280, false)], None);
            let effects = onboard.skip();
            assert_eq!(effects.last(), Some(&Effect::Finish(Finish::Skip)), "{page:?}");
            // Nothing is started by skipping: no download, no import, no country saved.
            assert!(effects.iter().all(|effect| matches!(effect, Effect::HideTestPicture | Effect::Finish(_))), "{page:?}: {effects:?}");
        }
        // A picture on the screen leaves with it.
        let mut onboard = at(Page::Screens, None);
        onboard.screens_seen(vec![screen(1920, true), screen(1280, false)], None);
        assert_eq!(onboard.skip(), [Effect::HideTestPicture, Effect::Finish(Finish::Skip)]);
        // A language picked is already in force and saved; a country not yet answered is not saved.
        let mut early = at(Page::Language, None);
        assert_eq!(early.pick_language("en"), [Effect::SetLanguage("en".into())]);
        assert_eq!(early.skip(), [Effect::Finish(Finish::Skip)]);
    }

    #[test]
    fn the_last_page_closes_the_first_run_whichever_card_is_pressed() {
        for (action, finish) in [(Action::TakeTour, Finish::Tour), (Action::OpenPlan, Finish::Plan), (Action::NewSong, Finish::NewSong)] {
            let mut onboard = at(Page::Done, None);
            assert_eq!(onboard.press(action), [Effect::Finish(finish)]);
        }
    }

    #[test]
    fn a_button_the_page_does_not_have_does_nothing() {
        let mut onboard = at(Page::Language, None);
        for action in [Action::Next, Action::TakeTour, Action::Works, Action::TryOther, Action::SkipForNow, Action::TryAgain] {
            assert!(onboard.press(action).is_empty());
            assert_eq!(onboard.page, Page::Language);
        }
    }

    #[test]
    fn the_ready_page_lists_the_jobs_that_were_asked_for() {
        let mut onboard = at(Page::Ready, Some(10));
        assert_eq!(onboard.jobs(), [JobRow::LanguageAndCountry]);
        onboard.bible = Some(("a".into(), "A".into()));
        onboard.bible_job = BibleJob::Done;
        onboard.songs_job = SongsJob::Reading;
        assert_eq!(onboard.jobs().len(), 3);
    }
}
