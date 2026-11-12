//! The first run's window: a card of eight pages, one question on each, drawn from what
//! `crate::onboard` says. The state machine decides; this file draws it, hands it the presses
//! and keys, and carries out what comes back (`App::onboard_perform`).
//!
//! The look is the owner's pick from four mocks, the one that copies Windows setup: a tinted
//! panel on the left with the page's picture and its dots, the question in large type on the
//! right, the answer already ringed, and Yes under it (`docs/operator.md`, "The first run").
//!
//! While it is open it owns the keyboard (`App::onboard_key`), as the tour does, so no key
//! reaches the projector: Enter presses the primary button, Escape goes Back, arrows move in
//! the list, typing filters the countries, Tab moves between the list and the buttons.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use artosia_core::song::Song;
use rune::{Cx, El, GrowthSpec, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar, TextInput};

use crate::bibles::{Catalogue, Row};
use crate::onboard::{Action, BibleJob, Effect, Facts, Finish, JobRow, Onboard, Page, SongRow, Songs, SongsJob, Summary};
use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::bibles::{copyright_line, extent, installed};
use crate::ui::widgets::{btn, state_pill};

/// The card at the most it grows to, and the room the window keeps round it.
const CARD_W: f32 = 1100.0;
const CARD_H: f32 = 640.0;
const MARGIN: f32 = 24.0;
/// A row of the language and country lists, and a row of the Bible list (headings included).
const ROW: f32 = 48.0;
const BIBLE_ROW: f32 = 44.0;
/// A line of the getting-ready page, and a card of the songs page.
const JOB: f32 = 56.0;
/// Songs go into the library this many at a time, one batch a frame, so the window keeps drawing.
const BATCH: usize = 40;

/// What the reading thread found, handed over to the window.
struct Read {
    songs: Vec<Song>,
    failed: usize,
}

/// The first run's work that outlives one press: songs read in the background and going into
/// the library a batch at a time, the Bible that was asked for, and what the next frame does.
#[derive(Default)]
pub struct Work {
    pending: Vec<Song>,
    summary: Summary,
    total: usize,
    read: Arc<Mutex<Option<Read>>>,
    /// The translation the first run asked for: used when it arrives even if the run was closed meanwhile.
    bible: Option<String>,
    /// The folder picker was asked for.
    want_folder: bool,
    /// The row to bring into view and the control to focus, once the page is on screen.
    scroll: Option<String>,
    focus: Option<&'static str>,
    /// The screens last told to the machine, so a poll tells it again only on a change.
    screens: (usize, Option<usize>),
}

/// Reads every song of a folder: EasiSlides' own data when it holds that, else OpenLyrics files.
fn read_songs(folder: &Path) -> Read {
    if !artosia_core::import::easislides_song_files(folder).is_empty() {
        let mut songs = Vec::new();
        let mut failed = 0;
        for file in artosia_core::import::easislides_song_files(folder) {
            match artosia_core::import::read_easislides_songs(&file) {
                Ok(found) => songs.extend(found),
                Err(_) => failed += 1,
            }
        }
        return Read { songs, failed };
    }
    let (songs, failed) = artosia_core::openlyrics::read_folder(folder);
    Read { songs, failed }
}

impl App {
    /// The first run for a new church, at the start of a real window.
    pub fn start_first_run(&mut self) {
        // One left unfinished keeps what was already answered.
        let fresh = !self.settings.first_run_open;
        self.settings.first_run_open = true;
        self.touch_settings();
        self.begin_onboarding(fresh);
    }

    /// The first run again, asked for from Settings or the search.
    pub fn start_onboarding(&mut self) {
        self.begin_onboarding(false);
    }

    fn begin_onboarding(&mut self, fresh: bool) {
        self.sheet = Sheet::None;
        self.close_palette();
        self.tour = None;
        let offered: Vec<String> = crate::languages::listed().map(|(_, language)| language.code.to_string()).collect();
        let codes: Vec<&str> = offered.iter().map(String::as_str).collect();
        // A language named on the command line stands, and so does the church's own when it comes back.
        let language = if fresh && crate::data::flag_value("--language").is_none() { crate::onboard::default_language(&codes, &artosia_locale::languages()) } else { self.settings.language.clone() };
        let country = if !fresh && !self.settings.country.is_empty() { self.settings.country.clone() } else { crate::onboard::default_country(artosia_locale::region().as_deref()) };
        let easislides = self.easislides.as_ref().map(|folder| artosia_core::import::easislides_song_count(folder)).filter(|count| *count > 0);
        let has_songs = !self.songs.is_empty();
        let onboard = Onboard::new(Facts { offered, language, country, easislides, has_songs });
        let effects = onboard.start();
        self.onboard = Some(onboard);
        self.onboard_work.screens = (usize::MAX, None);
        self.onboard_perform(effects);
        self.onboard_settle(None);
    }

    /// Runs one change of the machine and what it asks for, then settles focus and scroll.
    fn onboard_do(&mut self, change: impl FnOnce(&mut Onboard) -> Vec<Effect>) {
        let Some(onboard) = self.onboard.as_mut() else { return };
        let was = onboard.page;
        let effects = change(onboard);
        self.onboard_perform(effects);
        self.onboard_settle(Some(was));
    }

    /// After a page changes: the list or button that takes the keys, and the picked row in view.
    fn onboard_settle(&mut self, was: Option<Page>) {
        let Some(onboard) = self.onboard.as_ref() else { return };
        if was == Some(onboard.page) {
            return;
        }
        let (focus, scroll) = match onboard.page {
            Page::Language => ("onboard-list", Some(row_to_reveal(crate::languages::listed().map(|(_, language)| language.code), &onboard.language))),
            Page::Country => ("onboard-filter", Some(row_to_reveal(onboard.visible_countries().iter().map(|(code, _)| *code), &onboard.country))),
            // The Bible picked for the person is the list's first row, under its language's
            // heading: only a Bible the person chose is brought into view.
            Page::Bible => ("onboard-list", onboard.bible.as_ref().filter(|_| onboard.bible_chosen).map(|(id, _)| format!("bible-{id}"))),
            Page::Songs => ("onboard-list", None),
            Page::Sing => ("onboard-filter", None),
            Page::Screens | Page::Ready | Page::Done => ("onboard-primary", None),
        };
        self.onboard_work.focus = Some(focus);
        self.onboard_work.scroll = scroll;
    }

    fn onboard_perform(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::SetLanguage(code) => self.set_language(&code),
                Effect::SaveCountry(country) => {
                    self.settings.country = country.clone();
                    self.touch_settings();
                    self.desk.set_country(&country);
                    // The list is arranged again for the country; a person who never picked gets its default.
                    if let (Some(onboard), Catalogue::Ready(list)) = (self.onboard.as_mut(), &self.desk.catalogue) {
                        onboard.retarget(list);
                    }
                    self.desk.arrange();
                    self.onboard_sync_pick();
                }
                Effect::SaveSongLanguages(list) => {
                    self.settings.song_languages = list;
                    self.touch_settings();
                    self.refresh_decks();
                }
                Effect::LoadCatalogue => {
                    if !rune::is_headless() {
                        self.start_catalogue();
                    }
                }
                Effect::Download(id) => self.onboard_download(&id),
                Effect::ImportEasislides => {
                    if let Some(folder) = self.easislides.clone() {
                        self.onboard_read(folder);
                    }
                }
                Effect::ImportFolder(folder) => self.onboard_read(folder),
                Effect::ChooseFolder => self.onboard_work.want_folder = true,
                Effect::ShowTestPicture(screen) => {
                    // The numbers the Screens dialog's Identify button draws, on the screen the projector would take.
                    let mut options = self.output_options();
                    options.audience = Some(screen);
                    options.off = false;
                    self.link.place(options);
                    self.identify = true;
                    self.link.send(artosia_slides::frame::ToOutput::Identify(true));
                }
                Effect::HideTestPicture => {
                    self.identify = false;
                    self.link.send(artosia_slides::frame::ToOutput::Identify(false));
                    self.link.place(self.output_options());
                }
                Effect::UseScreen(screen) => {
                    let mut options = self.output_options();
                    options.audience = Some(screen);
                    options.off = false;
                    self.settings_screens(options);
                }
                Effect::Finish(finish) => self.finish_onboarding(finish),
            }
        }
    }

    /// Starts the Bible's download, or uses it at once when it is already installed.
    fn onboard_download(&mut self, id: &str) {
        let code = crate::bibles::installed_code(id);
        if self.bible_files.iter().any(|file| file.code == code) {
            self.set_bible(&code);
            if let Some(onboard) = self.onboard.as_mut() {
                onboard.bible_done();
            }
            return;
        }
        self.onboard_work.bible = Some(id.to_string());
        self.download_bible(id);
        if self.desk.downloading.is_none() {
            // A practice library has no folder to put it in.
            self.onboard_work.bible = None;
            if let Some(onboard) = self.onboard.as_mut() {
                onboard.bible_failed(t::BIBLES_PRACTICE().to_string());
            }
        }
    }

    /// Reads the songs of a folder on a thread; `onboard_batch` takes them in from there.
    fn onboard_read(&mut self, folder: PathBuf) {
        let (slot, waker) = (self.onboard_work.read.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let read = read_songs(&folder);
            if let Ok(mut slot) = slot.lock() {
                *slot = Some(read);
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// The Bible the first run asked for arrived: it is the window's Bible from now on.
    pub(crate) fn onboard_bible_arrived(&mut self, id: &str, code: &str) {
        if self.onboard_work.bible.as_deref() != Some(id) {
            return;
        }
        self.onboard_work.bible = None;
        self.set_bible(code);
        if let Some(onboard) = self.onboard.as_mut() {
            onboard.bible_done();
        }
    }

    /// The download the first run asked for failed. True when the run shows the reason, so no toast does.
    pub(crate) fn onboard_bible_failed(&mut self, id: &str, why: &str) -> bool {
        if self.onboard_work.bible.as_deref() != Some(id) {
            return false;
        }
        self.onboard_work.bible = None;
        match self.onboard.as_mut() {
            Some(onboard) => {
                onboard.bible_failed(why.to_string());
                true
            }
            None => false,
        }
    }

    /// Puts a Bible row's translation among the first run's picks and shows its copyright line.
    fn onboard_sync_pick(&mut self) {
        let Some(id) = self.onboard.as_ref().and_then(|onboard| onboard.bible.as_ref().map(|(id, _)| id.clone())) else { return };
        if self.desk.picked.as_deref() != Some(id.as_str()) {
            self.pick_bible(&id);
        }
    }

    /// The first run closes, by Skip setup or by one of the last page's cards.
    fn finish_onboarding(&mut self, finish: Finish) {
        self.onboard = None;
        self.onboard_work.focus = None;
        self.onboard_work.scroll = None;
        if std::mem::take(&mut self.settings.first_run_open) {
            self.touch_settings();
        }
        // The tour is offered on the last page, so it does not start again by itself.
        if !self.settings.toured {
            self.settings.toured = true;
            self.touch_settings();
        }
        match finish {
            Finish::Skip => {}
            Finish::Tour => self.start_tour(),
            Finish::Plan => {
                if let Some((id, _)) = self.plan_start() {
                    self.open_plan_item(id);
                }
            }
            Finish::NewSong => self.edit_new_song(Song::default()),
        }
    }

    // ---- What a person did ----

    pub fn onboard_press(&mut self, action: Action) {
        self.onboard_do(|onboard| onboard.press(action));
    }

    /// The primary button's own press. Dropped when the Enter that caused it already pressed it.
    pub fn onboard_press_primary(&mut self) {
        if self.clock == self.onboard_entered_at {
            return;
        }
        if let Some(action) = self.onboard.as_ref().map(|onboard| onboard.buttons().primary) {
            self.onboard_press(action);
        }
    }

    pub fn onboard_back(&mut self) {
        self.onboard_do(Onboard::back);
    }

    pub fn onboard_skip(&mut self) {
        self.onboard_do(Onboard::skip);
    }

    fn onboard_pick_language(&mut self, code: &str) {
        self.onboard_do(|onboard| onboard.pick_language(code));
    }

    fn onboard_pick_country(&mut self, code: &str) {
        self.onboard_do(|onboard| {
            onboard.pick_country(code);
            Vec::new()
        });
    }

    fn onboard_pick_bible(&mut self, id: &str) {
        let title = self.desk.translations().iter().find(|item| item.id == id).map(|item| self.desk.title_of(item).to_string()).unwrap_or_default();
        self.onboard_do(|onboard| {
            onboard.pick_bible(id, &title);
            Vec::new()
        });
        self.onboard_sync_pick();
    }

    fn onboard_pick_songs(&mut self, row: SongRow) {
        self.onboard_do(|onboard| onboard.pick_songs(row));
    }

    fn onboard_tick(&mut self, code: &str) {
        if let Some(onboard) = self.onboard.as_mut() {
            onboard.tick_language(code);
        }
    }

    fn onboard_sing_query_changed(&mut self) {
        if let Some(onboard) = self.onboard.as_mut() {
            onboard.sing_query_changed();
            self.onboard_work.scroll = Some(format!("onboard-lang-{}", onboard.sing_cursor));
        }
    }

    fn onboard_query_changed(&mut self) {
        if let Some(onboard) = self.onboard.as_mut() {
            onboard.query_changed();
            self.onboard_work.scroll = Some(row_to_reveal(onboard.visible_countries().iter().map(|(code, _)| *code), &onboard.country));
        }
    }

    /// The translations of the Bible page in the order they are drawn, as ids and titles.
    fn onboard_bibles(&self) -> Vec<(String, String)> {
        self.desk
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::Translation(at) => self.desk.translations().get(*at).map(|item| (item.id.clone(), self.desk.title_of(item).to_string())),
                Row::Language { .. } => None,
            })
            .collect()
    }

    fn onboard_step(&mut self, delta: i32, tx: &mut Tx) {
        let bibles = if self.onboard.as_ref().is_some_and(|onboard| onboard.page == Page::Bible) { self.onboard_bibles() } else { Vec::new() };
        self.onboard_do(|onboard| onboard.step(delta, &bibles));
        self.onboard_sync_pick();
        if let Some(onboard) = self.onboard.as_ref() {
            let key = match onboard.page {
                Page::Language => Some(format!("onboard-row-{}", onboard.language)),
                Page::Country => Some(format!("onboard-row-{}", onboard.country)),
                Page::Bible => onboard.bible.as_ref().map(|(id, _)| format!("bible-{id}")),
                Page::Sing => Some(format!("onboard-lang-{}", onboard.sing_cursor)),
                _ => None,
            };
            if let Some(key) = key {
                tx.scroll_into_view(key);
            }
        }
    }

    /// A key while the first run is open. Every key is taken, so none reaches the projector.
    pub fn onboard_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool {
        let focused = tx.focused().is_some();
        match &ev.key {
            // A modifier on its own and Tab belong to the window's own focus; a ringed Shift must not be eaten.
            Key::Modifier { .. } | Key::Tab => return false,
            Key::Enter => {
                // Rune may press a focused button for the same Enter: the press that follows is the same step.
                self.onboard_entered_at = self.clock;
                if let Some(action) = self.onboard.as_ref().map(|onboard| onboard.buttons().primary) {
                    self.onboard_press(action);
                }
            }
            Key::Escape => self.onboard_back(),
            Key::Up => self.onboard_step(-1, tx),
            Key::Down => self.onboard_step(1, tx),
            Key::PageUp => self.onboard_step(-7, tx),
            Key::PageDown => self.onboard_step(7, tx),
            Key::Home => self.onboard_step(-100_000, tx),
            Key::End => self.onboard_step(100_000, tx),
            // Typing anywhere on the country page filters it, as the search does.
            Key::Text(typed) if !focused && self.onboard.as_ref().is_some_and(|onboard| onboard.page == Page::Country) && typed.chars().all(|c| !c.is_control()) && typed != " " => {
                if let Some(onboard) = self.onboard.as_mut() {
                    onboard.query.insert(typed);
                    onboard.query.focused = true;
                }
                self.onboard_query_changed();
            }
            // On the languages page the space bar ticks the language the arrows rest on, and any other typing filters.
            Key::Text(typed) if !focused && self.onboard.as_ref().is_some_and(|onboard| onboard.page == Page::Sing) && typed.chars().all(|c| !c.is_control()) => {
                if typed == " " {
                    let cursor = self.onboard.as_ref().map(|onboard| onboard.sing_cursor.clone()).unwrap_or_default();
                    if !cursor.is_empty() {
                        self.onboard_tick(&cursor);
                    }
                } else {
                    if let Some(onboard) = self.onboard.as_mut() {
                        onboard.sing_query.insert(typed);
                        onboard.sing_query.focused = true;
                    }
                    self.onboard_sing_query_changed();
                }
            }
            _ => {}
        }
        true
    }

    // ---- Between frames ----

    /// Runs every frame while the first run or its work is alive: takes what the workers and the
    /// output process said, puts songs into the library a batch at a time, and settles focus.
    pub fn poll_onboard(&mut self, tx: &mut Tx) -> bool {
        let mut changed = false;
        if let Some(read) = self.onboard_work.read.lock().ok().and_then(|mut slot| slot.take()) {
            self.onboard_work.total = read.songs.len();
            self.onboard_work.summary = Summary { failed: read.failed, ..Summary::default() };
            self.onboard_work.pending = read.songs;
            self.onboard_work.pending.reverse();
            if let Some(onboard) = self.onboard.as_mut() {
                onboard.songs_read(self.onboard_work.total);
            }
            changed = true;
        }
        if !self.onboard_work.pending.is_empty() {
            self.onboard_batch();
            tx.wake_after(std::time::Duration::from_millis(16));
            changed = true;
        }
        if let Some(onboard) = self.onboard.as_mut() {
            match &self.desk.catalogue {
                Catalogue::Ready(list) if !onboard.catalogue_ready => {
                    onboard.retarget(list);
                    changed = true;
                }
                Catalogue::Failed(why) if onboard.catalogue_failed.as_deref() != Some(why.as_str()) => {
                    onboard.catalogue_failed(why.clone());
                    changed = true;
                }
                _ => {}
            }
            if matches!(onboard.bible_job, BibleJob::Waiting | BibleJob::Downloading(_)) && self.desk.downloading.is_some() {
                let percent = self.desk.progress.load(std::sync::atomic::Ordering::Relaxed);
                let shown = (percent > 0).then_some(percent.min(100) as u8);
                if onboard.bible_job != BibleJob::Downloading(shown) {
                    onboard.bible_progress(shown);
                    changed = true;
                }
            }
        }
        self.onboard_sync_pick();
        // What the output process says about the screens: the test picture goes on the one the projector would take.
        if self.onboard.is_some() && !rune::is_headless() {
            let status = self.link.status();
            if self.onboard_work.screens != (status.displays.len(), status.audience) {
                self.onboard_work.screens = (status.displays.len(), status.audience);
                let screens = status.displays.iter().map(|display| crate::onboard::Screen { width: display.width, height: display.height, primary: display.primary }).collect();
                let effects = self.onboard.as_mut().map(|onboard| onboard.screens_seen(screens, status.audience)).unwrap_or_default();
                self.onboard_perform(effects);
                changed = true;
            }
        }
        if self.onboard.is_some() {
            if let Some(focus) = self.onboard_work.focus.take() {
                tx.focus(focus, false);
            }
            if let Some(key) = self.onboard_work.scroll.take() {
                tx.scroll_into_view(key);
            }
            if std::mem::take(&mut self.onboard_work.want_folder) {
                let options = rune::FileDialogOptions { title: t::ONBOARD_SONGS_FOLDER().to_string(), ..Default::default() };
                tx.open_folder(options, |s: &mut App, result, _| match result {
                    Ok(Some(folder)) => s.onboard_do(|onboard| {
                        onboard.folder_chosen(Some(folder));
                        Vec::new()
                    }),
                    Ok(None) => {}
                    Err(error) => s.say(error.to_string(), None),
                });
            }
        }
        changed
    }

    /// Puts the next batch of songs into the library and tells the machine how far it is.
    pub(crate) fn onboard_batch(&mut self) {
        let take = self.onboard_work.pending.len().min(BATCH);
        let at = self.onboard_work.pending.len() - take;
        let mut batch: Vec<Song> = self.onboard_work.pending.split_off(at);
        batch.reverse();
        let done = self.store.import_songs(&mut batch).unwrap_or_default();
        let summary = &mut self.onboard_work.summary;
        (summary.added, summary.updated, summary.kept) = (summary.added + done.added, summary.updated + done.updated, summary.kept + done.kept);
        let (left, total, summary) = (self.onboard_work.pending.len(), self.onboard_work.total, self.onboard_work.summary);
        if left == 0 {
            self.reload_songs();
            self.settings.welcomed = true;
            self.touch_settings();
        }
        if let Some(onboard) = self.onboard.as_mut() {
            if left == 0 {
                onboard.songs_done(summary);
            } else {
                onboard.songs_progress(total - left, total);
            }
        }
    }
}

// ---------------------------------------------------------------- the view

fn language_name(code: &str) -> String {
    crate::languages::index_of(code).map(|at| crate::languages::LANGUAGES[at].name.to_string()).unwrap_or_else(|| code.to_string())
}

fn action_label(action: Action) -> &'static str {
    match action {
        Action::Yes => t::ONBOARD_YES(),
        Action::Next => t::ONBOARD_NEXT(),
        Action::SkipForNow => t::ONBOARD_SKIP_FOR_NOW(),
        Action::TryAgain => t::BIBLES_RETRY(),
        Action::Works => t::ONBOARD_WORKS(),
        Action::TryOther => t::ONBOARD_OTHER_SCREEN(),
        Action::TakeTour => t::CMD_TOUR(),
        Action::OpenPlan => t::ONBOARD_OPEN_PLAN(),
        Action::NewSong => t::CMD_NEW_SONG(),
    }
}

fn glyph(page: Page) -> &'static rune::Icon {
    match page {
        Page::Language | Page::Country => &icons::GLOBE,
        Page::Bible => &icons::BOOK,
        Page::Songs => &icons::SONG,
        Page::Sing => &icons::GLOBE,
        Page::Screens => &icons::SCREEN,
        Page::Ready => &icons::IMPORT,
        Page::Done => &icons::CHECK,
    }
}

fn radio(k: &Kit, on: bool) -> El {
    let ring = el().size(16.0, 16.0).radius(8.0).center().a11y_hidden();
    if on { ring.border(5.0, theme::accent()) } else { ring.border(1.5, k.c.border_strong) }
}

fn muted(k: &Kit, words: impl Into<String>) -> El {
    crate::ui::widgets::wrapped(words.into()).font(type_::SMALL).color(k.c.text_muted).wrap()
}

fn body(k: &Kit, words: impl Into<String>) -> El {
    crate::ui::widgets::wrapped(words.into()).font(type_::BODY).color(k.c.text_base).wrap()
}

/// One row of a list: the picked one filled with the accent and ringed, the others bare.
fn option_row(k: &Kit, key: String, label: String, on: bool, height: f32, press: impl Fn(&mut App) + 'static) -> El {
    let mut line = row().key(key).role("option").label(label).w_full().h(height).px(space::S4).gap(space::S3).center_y().radius(k.radius.md).hover(k.c.hover).cursor(rune::Cursor::Pointer);
    if on {
        line = line.fill(theme::accent().alpha(0.18)).border(1.0, theme::accent());
    }
    line.on_press(press)
}

fn row_text(k: &Kit, words: String, on: bool) -> El {
    text(words).font(rune::TextStyle::new(15.0, if on { 600.0 } else { 400.0 }).lh(20.0)).color(k.c.text_base).truncate().grow(1.0).min_w(0.0)
}

/// The row to bring into view so that the picked one sits clear of the list's fading edge:
/// the row three places past it, or the last one.
fn row_to_reveal<'a>(codes: impl Iterator<Item = &'a str>, picked: &str) -> String {
    let codes: Vec<&str> = codes.collect();
    let past = codes.iter().position(|code| *code == picked).map(|at| codes[(at + 3).min(codes.len() - 1)]);
    format!("onboard-row-{}", past.unwrap_or(picked))
}

/// The scrolling box of a long list. It is one Tab stop; arrows move the pick inside it.
fn list_box(k: &Kit, inside: El) -> El {
    col().key("onboard-list").role("listbox").focusable(true).w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(inside).scroll_bar(k)
}

fn languages_page(o: &Onboard, k: &Kit) -> El {
    let mut rows = col().w_full().gap(2.0);
    for code in &o.offered {
        let (on, pick) = (*code == o.language, code.clone());
        rows = rows.child(option_row(k, format!("onboard-row-{code}"), language_name(code), on, ROW, move |s: &mut App| s.onboard_pick_language(&pick)).child(row_text(k, language_name(code), on)));
    }
    list_box(k, rows)
}

fn countries_page(o: &Onboard, k: &Kit) -> El {
    let field = TextInput::new(&o.query)
        .width(None)
        .height(38.0)
        .label(t::ONBOARD_COUNTRY_FILTER())
        .placeholder(t::ONBOARD_COUNTRY_FILTER())
        .leading_icon(&rune_kit::glyphs::SEARCH)
        .on_edit(|s: &mut App| match s.onboard.as_mut() {
            Some(onboard) => &mut onboard.query,
            None => &mut s.spare_edit,
        })
        .on_change(|s: &mut App, _: &mut Tx| s.onboard_query_changed())
        .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| match ev.key {
            Key::Down | Key::Up | Key::PageDown | Key::PageUp | Key::Enter | Key::Escape => s.onboard_key(ev, tx),
            _ => false,
        })
        .build(k)
        .key("onboard-filter");
    let visible = o.visible_countries();
    let inside = if visible.is_empty() {
        col().w_full().h(ROW).center_y().px(space::S4).child(crate::ui::widgets::wrapped(t::nothing_found(o.query.text.trim())).font(type_::BODY).color(k.c.text_muted).wrap())
    } else {
        let count = visible.len();
        col().w_full().growth(GrowthSpec::default()).virtual_rows(
            count,
            ROW + 2.0,
            |s: &App, at| match s.onboard.as_ref().and_then(|onboard| onboard.visible_countries().get(at).map(|(code, _)| *code)) {
                Some(code) => format!("onboard-row-{code}").into(),
                None => format!("onboard-gap-{at}").into(),
            },
            |s: &App, at| {
                let Some(o) = s.onboard.as_ref() else { return el() };
                let Some((code, name)) = o.visible_countries().get(at).copied() else { return el() };
                let (on, pick) = (code == o.country, code.to_string());
                col().w_full().h(ROW + 2.0).child(option_row(theme::kit(), format!("onboard-row-{code}"), name.to_string(), on, ROW, move |s: &mut App| s.onboard_pick_country(&pick)).child(row_text(theme::kit(), name.to_string(), on)))
            },
        )
    };
    col().w_full().grow(1.0).min_h(0.0).gap(space::S3).child(field).child(list_box(k, inside))
}

/// A square tick box: the languages of this page are many, so each is ticked on its own.
fn tick_box(k: &Kit, on: bool) -> El {
    let square = el().size(18.0, 18.0).radius(4.0).center().a11y_hidden();
    if on { square.fill(theme::accent()).child(icon(&icons::CHECK, 14.0).color(rune::Color::WHITE)) } else { square.border(1.5, k.c.border_strong) }
}

/// Which languages the church sings in: a filter and a list with ticks, the country's languages
/// first. The ticked ones are shown together on a slide, in the order they were ticked.
fn sing_page(o: &Onboard, k: &Kit) -> El {
    let field = TextInput::new(&o.sing_query)
        .width(None)
        .height(38.0)
        .label(t::LANGUAGE_FILTER())
        .placeholder(t::LANGUAGE_FILTER())
        .leading_icon(&rune_kit::glyphs::SEARCH)
        .on_edit(|s: &mut App| match s.onboard.as_mut() {
            Some(onboard) => &mut onboard.sing_query,
            None => &mut s.spare_edit,
        })
        .on_change(|s: &mut App, _: &mut Tx| s.onboard_sing_query_changed())
        .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| match ev.key {
            Key::Down | Key::Up | Key::PageDown | Key::PageUp | Key::Enter | Key::Escape => s.onboard_key(ev, tx),
            _ => false,
        })
        .build(k)
        .key("onboard-filter");
    let visible = o.visible_languages();
    let inside = if visible.is_empty() {
        col().w_full().h(ROW).center_y().px(space::S4).child(crate::ui::widgets::wrapped(t::nothing_found(o.sing_query.text.trim())).font(type_::BODY).color(k.c.text_muted).wrap())
    } else {
        col().w_full().growth(GrowthSpec::default()).virtual_rows(
            visible.len(),
            ROW + 2.0,
            |s: &App, at| match s.onboard.as_ref().and_then(|onboard| onboard.visible_languages().get(at).map(|(code, _)| code.clone())) {
                Some(code) => format!("onboard-lang-{code}").into(),
                None => format!("onboard-gap-{at}").into(),
            },
            |s: &App, at| {
                let Some(o) = s.onboard.as_ref() else { return el() };
                let Some((code, name)) = o.visible_languages().get(at).cloned() else { return el() };
                let (on, here, pick) = (o.sing.contains(&code), o.sing_cursor == code, code.clone());
                let k = theme::kit();
                let mut line = row().key(format!("onboard-lang-{code}")).role("option").label(name.clone()).w_full().h(ROW).px(space::S4).gap(space::S3).center_y().radius(k.radius.md).hover(k.c.hover).cursor(rune::Cursor::Pointer);
                if on {
                    line = line.fill(theme::accent().alpha(0.18));
                }
                if here {
                    line = line.border(1.0, theme::accent());
                }
                col().w_full().h(ROW + 2.0).child(line.child(tick_box(k, on)).child(row_text(k, name, on)).on_press(move |s: &mut App| s.onboard_tick(&pick)))
            },
        )
    };
    col().w_full().grow(1.0).min_h(0.0).gap(space::S3).child(field).child(list_box(k, inside))
}

fn bible_page(s: &App, o: &Onboard, k: &Kit) -> El {
    let inside = match &s.desk.catalogue {
        _ if o.catalogue_failed.is_some() => {
            let why = o.catalogue_failed.clone().unwrap_or_default();
            return col()
                .key("onboard-offline")
                .w_full()
                .grow(1.0)
                .center()
                .gap(space::S3)
                .child(text(t::BIBLES_LOAD_FAILED()).font(type_::TITLE).color(theme::warn_ink()))
                .child(crate::ui::widgets::wrapped(t::as_sentence(&why)).font(type_::BODY).color(k.c.text_muted).wrap());
        }
        Catalogue::Idle | Catalogue::Loading | Catalogue::Failed(_) => {
            let mut lines = col().key("onboard-skeleton").w_full().child(row().w_full().h(BIBLE_ROW).px(space::S3).center_y().child(text(t::BIBLES_LOADING()).font(type_::SMALL).color(k.c.text_muted)));
            for (at, width) in [180.0f32, 120.0, 260.0, 210.0, 150.0].into_iter().enumerate() {
                lines = lines.child(row().key(format!("onboard-bar-{at}")).w_full().h(BIBLE_ROW).px(space::S3).center_y().a11y_hidden().child(el().w(width).h(12.0).radius(4.0).fill(k.c.layer_04)));
            }
            return col().w_full().grow(1.0).min_h(0.0).child(lines);
        }
        Catalogue::Ready(_) => col().key("onboard-bibles").w_full().growth(GrowthSpec::default()).virtual_rows(
            s.desk.rows.len(),
            BIBLE_ROW,
            |s: &App, at| match s.desk.rows.get(at) {
                Some(Row::Language { name, .. }) => format!("onboard-language-{name}").into(),
                Some(Row::Translation(index)) => s.desk.translations().get(*index).map(|item| format!("bible-{}", item.id)).unwrap_or_default().into(),
                None => format!("onboard-row-{at}").into(),
            },
            |s: &App, at| bible_line(s, theme::kit(), at),
        ),
    };
    // The picked Bible's own copyright line shows before anything downloads (docs/bibles.md).
    let notice = s.desk.picked_translation().map(|item| copyright_line(s, item)).unwrap_or_default();
    let notice = col().w_full().h(56.0).gap(2.0).child(text(t::BIBLES_COPYRIGHT()).font(type_::SMALL_STRONG).color(k.c.text_muted)).child(text(notice).font(type_::SMALL).color(k.c.text_base).max_lines(2));
    col().w_full().grow(1.0).min_h(0.0).gap(space::S3).child(list_box(k, inside)).child(notice)
}

fn bible_line(s: &App, k: &Kit, at: usize) -> El {
    match s.desk.rows.get(at) {
        Some(Row::Language { name, local }) => {
            let mut heading = row().key(format!("onboard-language-{name}")).w_full().h(BIBLE_ROW).px(space::S3).gap(space::S2).center_y().child(text(t::language_word(name)).font(type_::BODY_STRONG).color(k.c.text_base).truncate());
            if !local.is_empty() {
                heading = heading.child(text(local.clone()).font(type_::SMALL).color(k.c.text_muted).truncate());
            }
            heading
        }
        Some(Row::Translation(index)) => {
            let Some(item) = s.desk.translations().get(*index) else { return el() };
            let id = item.id.clone();
            let on = s.onboard.as_ref().and_then(|o| o.bible.as_ref()).is_some_and(|(picked, _)| *picked == item.id);
            let title = s.desk.title_of(item).to_string();
            let mut line = row()
                .key(format!("bible-{}", item.id))
                .role("option")
                .label(title.clone())
                .w_full()
                .h(BIBLE_ROW)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .hover(k.c.hover)
                .cursor(rune::Cursor::Pointer);
            if on {
                line = line.fill(theme::accent().alpha(0.12)).border(1.0, theme::accent());
            }
            line = line.child(radio(k, on)).child(text(title).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0).min_w(0.0)).child(muted(k, extent(item)));
            if installed(s, &item.id) {
                line = line.child(state_pill(t::BIBLES_INSTALLED(), k.c.layer_03, k.c.text_base));
            }
            line.on_press(move |s: &mut App| s.onboard_pick_bible(&id))
        }
        None => el(),
    }
}

fn songs_page(o: &Onboard, k: &Kit) -> El {
    let card = |row_kind: SongRow, key: &'static str, title: &str, hint: String| {
        let on = o.song_row_picked(row_kind);
        let mut card = row()
            .key(key)
            .role("option")
            .label(title.to_string())
            .w_full()
            .min_h(64.0)
            .px(space::S4)
            .gap(space::S3)
            .center_y()
            .radius(k.radius.lg)
            .hover(k.c.hover)
            .cursor(rune::Cursor::Pointer)
            .border(if on { 2.0 } else { 1.0 }, if on { theme::accent() } else { k.c.border_base });
        if on {
            card = card.fill(theme::accent().alpha(0.12));
        }
        card.child(radio(k, on))
            .child(col().grow(1.0).min_w(0.0).child(text(title.to_string()).font(type_::TITLE).color(k.c.text_base).truncate()).child(text(hint).font(type_::SMALL).color(k.c.text_muted).truncate_middle()))
            .on_press(move |s: &mut App| s.onboard_pick_songs(row_kind))
    };
    let mut rows = col().key("onboard-list").role("listbox").focusable(true).w_full().gap(space::S3);
    if let Some(count) = o.easislides {
        rows = rows.child(card(SongRow::Easislides, "onboard-songs-easislides", t::ONBOARD_SONGS_EASI(), t::onboard_songs_found(count)));
    }
    let folder_hint = match &o.songs {
        Songs::Folder(path) => path.display().to_string(),
        _ => t::ONBOARD_SONGS_FOLDER_HINT().to_string(),
    };
    rows = rows.child(card(SongRow::Folder, "onboard-songs-folder", t::ONBOARD_SONGS_FOLDER(), folder_hint));
    rows = rows.child(card(SongRow::Nowhere, "onboard-songs-nowhere", t::ONBOARD_SONGS_NONE(), t::ONBOARD_SONGS_NONE_HINT().to_string()));
    if o.locked() { rows.layer_opacity(0.6) } else { rows }
}

fn screens_page(o: &Onboard, k: &Kit) -> El {
    let Some(rows) = o.screen_rows() else { return el() };
    let line = |title: &str, value: String| row().w_full().h(48.0).gap(space::S3).center_y().child(icon(&icons::CHECK, 18.0).color(theme::accent()).a11y_hidden()).child(body(k, title)).child(spacer()).child(text(value).font(type_::BODY).color(k.c.text_muted));
    col().w_full().padding(space::S4).radius(k.radius.lg).fill(k.c.layer_03).child(line(t::ONBOARD_ROW_WINDOW(), t::screen_name(rows.window))).child(line(t::ONBOARD_ROW_PICTURE(), t::screen_name(rows.picture))).child(line(t::ONBOARD_ROW_SIZE(), t::screen_detail(rows.width, rows.height, 0.0)))
}

fn job_line(k: &Kit, title: String, sub: Option<String>, state: String, mark: Mark) -> El {
    let (fill, inside): (rune::Color, Option<El>) = match mark {
        Mark::Done => (theme::accent(), Some(icon(&icons::CHECK, 16.0).color(rune::Color::WHITE).a11y_hidden())),
        Mark::Failed => (theme::warn(), Some(icon(&icons::CLOSE, 14.0).color(rune::Color::BLACK).a11y_hidden())),
        Mark::Open => (k.c.layer_04, None),
    };
    let mut words = col().grow(1.0).min_w(0.0).child(text(title).font(type_::BODY).color(k.c.text_base).truncate());
    if let Some(sub) = sub {
        words = words.child(text(sub).font(type_::SMALL).color(if mark == Mark::Failed { theme::warn_ink() } else { k.c.text_muted }).truncate());
    }
    row()
        .w_full()
        .h(JOB)
        .gap(space::S4)
        .center_y()
        .child(el().size(28.0, 28.0).radius(14.0).center().fill(fill).children(inside))
        .child(words)
        .child(text(state).font(type_::BODY).color(if mark == Mark::Failed { theme::warn_ink() } else if mark == Mark::Done { k.c.text_base } else { k.c.text_muted }))
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Open,
    Done,
    Failed,
}

fn ready_page(o: &Onboard, k: &Kit) -> El {
    let mut list = col().w_full();
    let mut failed: Option<String> = None;
    for job in o.jobs() {
        list = list.child(match job {
            JobRow::LanguageAndCountry => job_line(k, t::ONBOARD_JOB_LANGUAGE().to_string(), None, t::ONBOARD_STATE_DONE().to_string(), Mark::Done),
            JobRow::Bible { title, job } => match job {
                BibleJob::Done => job_line(k, title, None, t::ONBOARD_STATE_DONE().to_string(), Mark::Done),
                BibleJob::Failed(why) => {
                    failed = Some(why.clone());
                    job_line(k, title, Some(t::as_sentence(&why)), t::ONBOARD_STATE_FAILED().to_string(), Mark::Failed)
                }
                BibleJob::Downloading(Some(percent)) => job_line(k, title, None, t::onboard_percent(percent), Mark::Open),
                BibleJob::Downloading(None) => job_line(k, title, None, t::BIBLES_DOWNLOADING().to_string(), Mark::Open),
                BibleJob::Idle | BibleJob::Waiting | BibleJob::Skipped => job_line(k, title, None, t::ONBOARD_STATE_WAITING().to_string(), Mark::Open),
            },
            JobRow::Songs(job) => match job {
                SongsJob::Done(summary) => {
                    let said = t::import_summary(summary.added, summary.updated, summary.kept);
                    let said = if summary.failed > 0 { format!("{said}. {}", t::files_unreadable(summary.failed)) } else { said };
                    job_line(k, t::ONBOARD_JOB_SONGS().to_string(), Some(said), t::ONBOARD_STATE_DONE().to_string(), Mark::Done)
                }
                SongsJob::Importing { done, of } => job_line(k, t::ONBOARD_JOB_SONGS().to_string(), None, t::onboard_progress(done, of), Mark::Open),
                SongsJob::Idle | SongsJob::Reading => job_line(k, t::ONBOARD_JOB_SONGS().to_string(), None, t::ONBOARD_STATE_WAITING().to_string(), Mark::Open),
            },
        });
    }
    let mut page = col().w_full().gap(space::S4).child(list);
    if failed.is_some() {
        page = page.child(crate::ui::widgets::wrapped(t::ONBOARD_BIBLE_FAILED()).font(type_::BODY).color(k.c.text_muted).wrap());
    }
    page
}

fn done_page(k: &Kit) -> El {
    let card = |n: &str, title: &str, action: Action, primary: bool| {
        let button = btn(action_label(action)).focusable();
        let button = if primary { button.primary() } else { button };
        let key = format!("onboard-card-{n}");
        let press = move |s: &mut App, _: &mut Tx| if primary { s.onboard_press_primary() } else { s.onboard_press(action) };
        let button = button.build(k, press);
        col()
            .key(key)
            .grow(1.0)
            
            .min_w(0.0)
            .h(220.0)
            .padding(space::S4)
            .gap(space::S3)
            .radius(k.radius.lg)
            .fill(k.c.layer_03)
            .child(el().size(36.0, 36.0).radius(18.0).fill(theme::accent()).center().child(text(n.to_string()).font(type_::TITLE).color(rune::Color::WHITE)))
            .child(crate::ui::widgets::wrapped(title.to_string()).font(type_::TITLE).color(k.c.text_base).wrap())
            .child(spacer())
            .child(row().child(if primary { button.key("onboard-primary") } else { button }))
    };
    row().w_full().gap(space::S3).child(card("1", t::ONBOARD_CARD_TOUR(), Action::TakeTour, true)).child(card("2", t::ONBOARD_CARD_PLAN(), Action::OpenPlan, false)).child(card("3", t::ONBOARD_CARD_SONG(), Action::NewSong, false))
}

fn dots(k: &Kit, page: Page) -> El {
    let mut dots = row().gap(space::S2).center_y().a11y_hidden();
    for at in 1..=Page::ALL.len() {
        let on = at <= page.number();
        dots = dots.child(el().size(if at == page.number() { 22.0 } else { 8.0 }, 8.0).radius(4.0).fill(if on { theme::accent() } else { k.c.layer_04 }));
    }
    dots
}

pub(crate) fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) {
    let Some(o) = &s.onboard else { return };
    let [window_w, window_h] = cx.layout_size("window").unwrap_or([1366.0, 768.0]);
    let (card_w, card_h) = (CARD_W.min(window_w - 2.0 * MARGIN), CARD_H.min(window_h - 2.0 * MARGIN));
    // A window at its smallest keeps the card whole by giving up padding, never by clipping.
    let pad = if card_h < 600.0 { 32.0 } else { 48.0 };
    let buttons = o.buttons();
    let (title, hint) = match o.page {
        Page::Language => (t::ONBOARD_LANGUAGE_TITLE(), None),
        Page::Country => (t::ONBOARD_COUNTRY_TITLE(), Some(t::ONBOARD_COUNTRY_HINT())),
        Page::Bible => (t::ONBOARD_BIBLE_TITLE(), Some(t::ONBOARD_BIBLE_HINT())),
        Page::Songs => (t::ONBOARD_SONGS_TITLE(), None),
        Page::Sing => (t::ONBOARD_SING_TITLE(), Some(t::ONBOARD_SING_HINT())),
        Page::Screens if o.two_screens() || o.shown.is_some() => (t::ONBOARD_SCREENS_TITLE(), Some(t::ONBOARD_SCREENS_HINT())),
        Page::Screens => (t::ONBOARD_NO_PROJECTOR(), Some(t::ONBOARD_NO_PROJECTOR_HINT())),
        Page::Ready => (t::ONBOARD_READY_TITLE(), Some(t::ONBOARD_READY_HINT())),
        Page::Done => (t::ONBOARD_DONE_TITLE(), Some(t::ONBOARD_DONE_HINT())),
    };
    let content = match o.page {
        Page::Language => languages_page(o, k),
        Page::Country => countries_page(o, k),
        Page::Bible => bible_page(s, o, k),
        Page::Songs => songs_page(o, k),
        Page::Sing => sing_page(o, k),
        Page::Screens => screens_page(o, k),
        Page::Ready => ready_page(o, k),
        Page::Done => done_page(k),
    };
    let mut words = col().w_full().gap(space::S2).child(crate::ui::widgets::wrapped(title.to_string()).font(rune::TextStyle::new(30.0, 600.0).lh(38.0)).color(k.c.text_base).wrap());
    if let Some(hint) = hint {
        words = words.child(crate::ui::widgets::wrapped(hint.to_string()).font(rune::TextStyle::new(15.0, 400.0).lh(22.0)).color(k.c.text_muted).wrap());
    }
    // Back at the left, then the quiet buttons, then the one filled one, always at the right.
    let mut foot = row().w_full().gap(space::S3).center_y();
    if o.page != Page::Language {
        foot = foot.child(btn(t::ONBOARD_BACK()).icon(&icons::ARROW_LEFT).focusable().build(k, |s, _| s.onboard_back()).key("onboard-back"));
    }
    foot = foot.child(spacer());
    if o.page != Page::Done {
        for action in &buttons.others {
            let action = *action;
            foot = foot.child(btn(action_label(action)).ghost().focusable().build(k, move |s, _| s.onboard_press(action)).key("onboard-other"));
        }
        let primary = btn(action_label(buttons.primary)).primary().focusable().disabled(!buttons.primary_enabled).grow().build(k, |s, _| s.onboard_press_primary()).key("onboard-primary");
        foot = foot.child(row().min_w(140.0).child(primary));
    }
    let panel = col()
        .w((card_w * 0.345).round().clamp(240.0, 380.0))
        .h_full()
        .padding(pad)
        .gap(space::S5)
        .fill(theme::accent().alpha(0.12))
        .child(row().gap(space::S2).center_y().child(icon(&icons::LOGO, 22.0).color(k.c.text_base).a11y_hidden()).child(text(t::APP_NAME()).font(type_::HEADING).color(k.c.text_base)))
        .child(spacer())
        .child(icon(glyph(o.page), 96.0).color(theme::accent()).a11y_hidden())
        .child(spacer())
        .child(dots(k, o.page));
    let card = row()
        .key("onboard-card")
        .role("dialog")
        .label(t::CMD_FIRST_RUN())
        .w(card_w)
        .h(card_h)
        .radius(12.0)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .clip_rounded(12.0)
        .child(panel)
        .child(col().grow(1.0).min_w(0.0).h_full().padding(pad).gap(space::S5).child(words).child(col().w_full().grow(1.0).min_h(0.0).child(content)).child(foot))
        // Skip setup is quiet and at the top right of every page: it keeps what was answered and closes.
        .child(row().pin(None, Some(space::S4), Some(space::S4), None).child(btn(t::ONBOARD_SKIP()).ghost().focusable().build(k, |s, _| s.onboard_skip()).key("onboard-skip")))
        .modal(false, |s: &mut App| s.onboard_back());
    layer.push(el().key("onboard-backdrop").absolute(0.0, 0.0).w_full().h_full().fill(k.c.bg_base).a11y_hidden());
    layer.push(col().key("onboard-centre").absolute(0.0, 0.0).w_full().h_full().center().child(card));
}

// ---------------------------------------------------------------- scenes

impl App {
    /// Puts the first run on a page with made-up answers, for a headless render
    /// (`--scene onboard-<page>`): the made-up Bibles of the Bibles dialog's demo, no real song.
    pub fn demo_onboard(&mut self, which: &str) {
        let offered: Vec<String> = crate::languages::listed().map(|(_, language)| language.code.to_string()).collect();
        let easislides = (which != "songs-none").then_some(961);
        let mut onboard = Onboard::new(Facts { offered, language: t::language_code().to_string(), country: "ID".to_string(), easislides, has_songs: false });
        let screen = |width: u32, height: u32, primary: bool| crate::onboard::Screen { width, height, primary };
        let two = || vec![screen(1920, 1080, true), screen(1280, 720, false)];
        self.demo_catalogue();
        match which {
            "country" => onboard.page = Page::Country,
            "bible" | "bible-offline" => {
                onboard.page = Page::Bible;
                if which == "bible" {
                    if let Catalogue::Ready(list) = &self.desk.catalogue {
                        onboard.retarget(list);
                    }
                } else {
                    onboard.catalogue_failed(t::WHY_OFFLINE().to_string());
                    self.desk.catalogue = Catalogue::Failed(t::WHY_OFFLINE().to_string());
                }
            }
            "songs" => onboard.page = Page::Songs,
            "sing" => {
                onboard.page = Page::Sing;
                // The window's own language is ticked already; two more are ticked by hand.
                onboard.tick_language("en");
                onboard.tick_language("su");
                onboard.sing_cursor = "jv".to_string();
            }
            "screens" => {
                onboard.page = Page::Screens;
                onboard.screens = two();
                onboard.shown = Some(1);
            }
            "one-screen" => {
                onboard.page = Page::Screens;
                onboard.screens = vec![screen(1920, 1080, true)];
            }
            "ready" | "ready-failed" => {
                onboard.page = Page::Ready;
                onboard.bible = Some(("demoa".to_string(), "Alkitab Contoh Lengkap".to_string()));
                if which == "ready" {
                    onboard.bible_job = BibleJob::Downloading(Some(62));
                    onboard.songs_job = SongsJob::Importing { done: 412, of: 961 };
                } else {
                    onboard.bible_job = BibleJob::Failed(t::WHY_BROKEN().to_string());
                    onboard.songs_job = SongsJob::Done(Summary { added: 950, updated: 3, kept: 8, failed: 0 });
                }
            }
            "done" => onboard.page = Page::Done,
            _ => {}
        }
        self.desk.set_country("ID");
        self.desk.arrange();
        if which.starts_with("bible") && which != "bible-offline" {
            self.desk.picked = Some("demoa".to_string());
        }
        self.onboard_work.scroll = Some(match onboard.page {
            Page::Sing => String::new(),
            Page::Country => row_to_reveal(onboard.visible_countries().iter().map(|(code, _)| *code), &onboard.country),
            _ => String::new(),
        });
        self.onboard = Some(onboard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key) -> KeyEvent {
        KeyEvent { key, mods: rune::Mods::default() }
    }

    fn open() -> App {
        let mut app = App::practice();
        app.start_onboarding();
        app
    }

    fn page(app: &App) -> Page {
        app.onboard.as_ref().map(|onboard| onboard.page).unwrap_or(Page::Done)
    }

    #[test]
    fn picking_a_language_changes_the_window_at_once_and_is_saved() {
        let mut app = open();
        app.onboard_pick_language("id");
        assert_eq!(t::language_code(), "id");
        assert_eq!(app.settings.language, "id");
        app.onboard_pick_language("en");
        assert_eq!((t::language_code(), app.settings.language.as_str()), ("en", "en"));
    }

    #[test]
    fn yes_on_the_country_page_keeps_two_letters_and_puts_its_languages_first() {
        let mut app = open();
        app.onboard_press(Action::Yes);
        app.onboard_pick_country("BR");
        app.onboard_press(Action::Yes);
        assert_eq!(page(&app), Page::Bible);
        assert_eq!(app.settings.country, "BR");
        assert_eq!(app.desk.ranking.first().map(String::as_str), Some("por"));
    }

    #[test]
    fn skip_setup_closes_keeps_the_answers_and_marks_the_tour_as_offered() {
        let mut app = open();
        assert!(!app.settings.toured);
        app.onboard_press(Action::Yes);
        app.onboard_pick_country("JP");
        app.onboard_press(Action::Yes);
        app.onboard_skip();
        assert!(app.onboard.is_none());
        assert_eq!(app.settings.country, "JP");
        assert!(app.settings.toured, "the tour was on offer, so it does not start by itself next time");
        assert!(app.tour.is_none());
    }

    #[test]
    fn the_cards_of_the_last_page_close_it_and_the_tour_card_starts_the_tour() {
        let mut app = open();
        if let Some(onboard) = app.onboard.as_mut() {
            onboard.page = Page::Done;
        }
        app.onboard_press_primary();
        assert!(app.onboard.is_none());
        assert_eq!(app.tour, Some(0));
        let mut again = open();
        if let Some(onboard) = again.onboard.as_mut() {
            onboard.page = Page::Done;
        }
        again.onboard_press(Action::NewSong);
        assert!(again.onboard.is_none() && again.editor.is_some() && again.tour.is_none());
    }

    #[test]
    fn the_command_and_settings_open_it_again() {
        let mut app = App::practice();
        assert!(app.onboard.is_none());
        app.sheet = Sheet::Settings;
        app.run(crate::palette::Command::FirstRun);
        assert_eq!(page(&app), Page::Language);
        assert_eq!(app.sheet, Sheet::None);
        assert!(crate::palette::Command::ALL.contains(&crate::palette::Command::FirstRun));
    }

    #[test]
    fn it_owns_the_keyboard_and_space_never_reaches_the_projector() {
        let mut app = open();
        let mut tx = Tx::default();
        let live = app.live.as_ref().map(|live| live.index);
        for typed in [" ", "x", "1"] {
            assert!(app.onboard_key(&key(Key::Text(typed.into())), &mut tx), "{typed:?}");
        }
        for pressed in [Key::Right, Key::Left, Key::Backspace, Key::Delete, Key::Function(6)] {
            assert!(app.onboard_key(&key(pressed), &mut tx));
        }
        assert_eq!(app.live.as_ref().map(|live| live.index), live);
        assert!(!app.black && !app.off_air);
        // Enter is Yes, Escape is Back, an arrow moves the list.
        assert!(app.onboard_key(&key(Key::Enter), &mut tx));
        assert_eq!(page(&app), Page::Country);
        assert!(app.onboard_key(&key(Key::Escape), &mut tx));
        assert_eq!(page(&app), Page::Language);
        app.onboard_key(&key(Key::Down), &mut tx);
        assert_eq!(app.onboard.as_ref().map(|onboard| onboard.language.clone()).as_deref(), Some("id"));
        // Tab is the window's own focus move.
        assert!(!app.onboard_key(&key(Key::Tab), &mut tx));
    }

    #[test]
    fn typing_on_the_country_page_filters_the_list() {
        let mut app = open();
        let mut tx = Tx::default();
        app.onboard_press(Action::Yes);
        for letter in ["j", "a", "p"] {
            app.onboard_key(&key(Key::Text(letter.into())), &mut tx);
        }
        let onboard = app.onboard.as_ref();
        assert_eq!(onboard.map(|onboard| onboard.query.text.clone()).as_deref(), Some("jap"));
        assert_eq!(onboard.map(|onboard| onboard.country.clone()).as_deref(), Some("JP"));
        assert_eq!(onboard.map(|onboard| onboard.visible_countries().len()), Some(1));
    }

    #[test]
    fn a_bible_asked_for_on_a_library_with_no_folder_fails_into_a_line_and_blocks_nothing() {
        let mut app = open();
        app.demo_catalogue();
        app.desk.arrange();
        if let (Some(onboard), Catalogue::Ready(list)) = (app.onboard.as_mut(), &app.desk.catalogue) {
            onboard.retarget(list);
            onboard.page = Page::Bible;
        }
        app.onboard_press(Action::Next);
        assert_eq!(page(&app), Page::Songs);
        assert!(matches!(app.onboard.as_ref().map(|onboard| onboard.bible_job.clone()), Some(BibleJob::Failed(_))));
    }

    /// Songs go in a hundred a frame. A made-up library of the size of a church's own: the time
    /// of one batch is what a frame pays (`BATCH`), printed so it can be read.
    #[test]
    fn songs_go_into_the_library_in_batches_and_one_batch_is_cheap() {
        let mut app = open();
        let songs: Vec<Song> = (0..961)
            .map(|n| Song { title: format!("Made up song {n}"), author: "Test".into(), lyrics: format!("[1]\nMade up line one of {n}\nMade up line two of {n}\n[chorus]\nMade up chorus {n}\n"), ..Song::default() })
            .collect();
        let before = app.songs.len();
        app.onboard_work.total = songs.len();
        app.onboard_work.pending = songs;
        app.onboard_work.pending.reverse();
        if let Some(onboard) = app.onboard.as_mut() {
            onboard.songs_read(961);
        }
        let mut worst = std::time::Duration::ZERO;
        let mut batches = 0;
        while !app.onboard_work.pending.is_empty() {
            let started = std::time::Instant::now();
            app.onboard_batch();
            worst = worst.max(started.elapsed());
            batches += 1;
            let job = app.onboard.as_ref().map(|onboard| onboard.songs_job.clone());
            if !app.onboard_work.pending.is_empty() {
                assert!(matches!(job, Some(SongsJob::Importing { .. })), "{job:?}");
            }
        }
        println!("onboarding import: {batches} batches of {BATCH}, the slowest {worst:?}");
        assert_eq!(batches, 961_usize.div_ceil(BATCH));
        assert_eq!(app.songs.len(), before + 961);
        assert_eq!(app.onboard.as_ref().map(|onboard| onboard.songs_job.clone()), Some(SongsJob::Done(Summary { added: 961, ..Summary::default() })));
        assert!(worst < std::time::Duration::from_millis(250), "{worst:?}");
    }

    #[test]
    fn a_folder_with_no_songs_reads_as_nothing_and_no_failures() {
        let folder = std::env::temp_dir().join(format!("artosia-onboard-read-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|error| panic!("{error}"));
        let read = read_songs(&folder);
        assert_eq!((read.songs.len(), read.failed), (0, 0));
        let _ = std::fs::remove_dir_all(&folder);
    }
}
