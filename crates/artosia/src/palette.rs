//! The search box: one field that finds songs, Bible passages, announcements and commands, and
//! that is also the library when nothing is typed.
//!
//! The ranking follows the owner's house-kit palette (`artosia_core::palette`): every part of
//! the app contributes documents, the order is a cascade of single rules, a document answers
//! to its words in English and Indonesian, and Tab attaches the picked song so the next words
//! search inside it.

use std::collections::HashSet;
use std::ops::Range;

use artosia_core::bible::{Reference, english_name};
use artosia_core::deck::{DeckSource, Fill};
use artosia_core::palette::{self as kit, Doc, LABEL};
use artosia_core::plan::PlanEntry;
use artosia_core::search::LineHit;
use artosia_core::song::{SectionKind, Tag};
use artosia_slides::look::presets;
use rune::TextEdit;

use crate::state::{App, Sheet, section_starts};
use crate::strings as t;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    #[default]
    All,
    Songs,
    Bible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    NewSong,
    PasteSong,
    Announcements,
    Looks,
    Backdrops,
    Settings,
    Screens,
    Import,
    ImportFolder,
    Remote,
    Heading,
    NewPlan,
    PlanDuplicate,
    PlanRename,
    PlanDelete,
    StartHere,
    Earlier,
    NewAnnouncement,
    Keys,
    StartService,
    Next,
    Previous,
    Black,
    ClearText,
    Logo,
    StopCountdown,
    HideMessage,
    EditSong,
    Undo,
    LoopAnnouncements,
    ImportPptx,
    ImportBirthdays,
    CheckUpdate,
    Listen,
    Stream,
    BibleFile,
    Bibles,
    Tour,
    FirstRun,
}

impl Command {
    pub const ALL: [Command; 39] = [
        Command::StartService,
        Command::Next,
        Command::Previous,
        Command::Black,
        Command::ClearText,
        Command::Logo,
        Command::StopCountdown,
        Command::HideMessage,
        Command::LoopAnnouncements,
        Command::Undo,
        Command::NewSong,
        Command::PasteSong,
        Command::EditSong,
        Command::Announcements,
        Command::NewAnnouncement,
        Command::Heading,
        Command::StartHere,
        Command::Earlier,
        Command::NewPlan,
        Command::PlanDuplicate,
        Command::PlanRename,
        Command::PlanDelete,
        Command::Looks,
        Command::Backdrops,
        Command::Settings,
        Command::Screens,
        Command::Remote,
        Command::Listen,
        Command::Stream,
        Command::Bibles,
        Command::BibleFile,
        Command::ImportFolder,
        Command::ImportPptx,
        Command::ImportBirthdays,
        Command::Import,
        Command::Keys,
        Command::Tour,
        Command::FirstRun,
        Command::CheckUpdate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Command::NewSong => t::CMD_NEW_SONG(),
            Command::PasteSong => t::CMD_PASTE_SONG(),
            Command::Announcements => t::CMD_ANNOUNCEMENTS(),
            Command::Looks => t::CMD_LOOKS(),
            Command::Settings => t::CMD_SETTINGS(),
            Command::Backdrops => t::BACKDROPS_TITLE(),
            Command::Screens => t::CMD_SCREENS(),
            Command::Import => t::CMD_IMPORT(),
            Command::ImportFolder => t::CMD_IMPORT_FOLDER(),
            Command::Remote => t::CMD_REMOTE(),
            Command::Heading => t::CMD_HEADING(),
            Command::NewPlan => t::PLAN_NEW(),
            Command::PlanDuplicate => t::PLAN_DUPLICATE(),
            Command::PlanRename => t::PLAN_RENAME(),
            Command::PlanDelete => t::PLAN_DELETE(),
            Command::StartHere => t::CMD_START_HERE(),
            Command::Earlier => t::CMD_EARLIER(),
            Command::NewAnnouncement => t::ANNOUNCEMENT_NEW(),
            Command::Keys => t::CMD_KEYS(),
            Command::Tour => t::CMD_TOUR(),
            Command::FirstRun => t::CMD_FIRST_RUN(),
            Command::StartService => t::CMD_START(),
            Command::Next => t::TIP_NEXT(),
            Command::Previous => t::CMD_PREVIOUS(),
            Command::Black => t::BTN_BLACK(),
            Command::ClearText => t::BTN_CLEAR_TEXT(),
            Command::Logo => t::BTN_LOGO(),
            Command::StopCountdown => t::CMD_STOP_COUNTDOWN(),
            Command::HideMessage => t::CMD_HIDE_MESSAGE(),
            Command::EditSong => t::CMD_EDIT_SONG(),
            Command::Undo => t::BTN_UNDO(),
            Command::LoopAnnouncements => t::CMD_LOOP(),
            Command::ImportPptx => t::CMD_IMPORT_PPTX(),
            Command::ImportBirthdays => t::CMD_IMPORT_BIRTHDAYS(),
            Command::CheckUpdate => t::CMD_UPDATE(),
            Command::Listen => t::CMD_LISTEN(),
            Command::Stream => t::CMD_STREAM(),
            Command::BibleFile => t::CMD_BIBLE_FILE(),
            Command::Bibles => t::CMD_BIBLES(),
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Command::NewSong => t::CMD_NEW_SONG_DETAIL(),
            Command::PasteSong => t::CMD_PASTE_SONG_DETAIL(),
            Command::Announcements => t::CMD_ANNOUNCEMENTS_DETAIL(),
            Command::Looks => t::CMD_LOOKS_DETAIL(),
            Command::Settings => t::CMD_SETTINGS_DETAIL(),
            Command::Backdrops => t::CMD_BACKDROPS_DETAIL(),
            Command::Screens => t::CMD_SCREENS_DETAIL(),
            Command::Import => t::CMD_IMPORT_DETAIL(),
            Command::ImportFolder => t::CMD_IMPORT_FOLDER_DETAIL(),
            Command::Remote => t::CMD_REMOTE_DETAIL(),
            Command::Heading => t::CMD_HEADING_DETAIL(),
            Command::NewPlan => t::PLAN_NEW_DETAIL(),
            Command::PlanDuplicate => t::CMD_PLAN_DUPLICATE_DETAIL(),
            Command::PlanRename => t::CMD_PLAN_RENAME_DETAIL(),
            Command::PlanDelete => t::CMD_PLAN_DELETE_DETAIL(),
            Command::StartHere => t::CMD_START_HERE_DETAIL(),
            Command::Earlier => t::CMD_EARLIER_DETAIL(),
            Command::NewAnnouncement => t::CMD_NEW_ANNOUNCEMENT_DETAIL(),
            Command::Keys => t::CMD_KEYS_DETAIL(),
            Command::Tour => t::CMD_TOUR_DETAIL(),
            Command::FirstRun => t::CMD_FIRST_RUN_DETAIL(),
            Command::StartService => t::CMD_START_DETAIL(),
            Command::Next => t::CMD_NEXT_DETAIL(),
            Command::Previous => t::CMD_PREVIOUS_DETAIL(),
            Command::Black => t::CMD_BLACK_DETAIL(),
            Command::ClearText => t::CMD_CLEAR_DETAIL(),
            Command::Logo => t::CMD_LOGO_DETAIL(),
            Command::StopCountdown => t::CMD_STOP_COUNTDOWN_DETAIL(),
            Command::HideMessage => t::CMD_HIDE_MESSAGE_DETAIL(),
            Command::EditSong => t::CMD_EDIT_SONG_DETAIL(),
            Command::Undo => t::CMD_UNDO_DETAIL(),
            Command::LoopAnnouncements => t::CMD_LOOP_DETAIL(),
            Command::ImportPptx => t::CMD_IMPORT_PPTX_DETAIL(),
            Command::ImportBirthdays => t::CMD_IMPORT_BIRTHDAYS_DETAIL(),
            Command::CheckUpdate => t::CMD_UPDATE_DETAIL(),
            Command::Listen => t::CMD_LISTEN_DETAIL(),
            Command::Stream => t::CMD_STREAM_DETAIL(),
            Command::BibleFile => t::CMD_BIBLE_FILE_DETAIL(),
            Command::Bibles => t::CMD_BIBLES_FIND_DETAIL(),
        }
    }

    /// Other words that find the command, in English and Indonesian. They never show.
    fn aliases(self) -> &'static [&'static str] {
        match self {
            Command::NewSong => &["add song", "create song", "lagu baru", "tambah lagu", "buat lagu"],
            Command::PasteSong => &["paste", "clipboard", "tempel lirik", "tempel lagu"],
            Command::Announcements => &["notices", "edit announcements", "pengumuman", "warta"],
            Command::Looks => &[
                "look", "theme", "background", "font", "text size", "colour", "color", "text colour", "text color", "yellow", "white", "outline", "shadow", "words", "lines", "position", "tampilan", "desain", "tema", "latar", "ukuran teks", "warna", "warna teks", "kuning", "putih",
                "huruf", "garis tepi", "baris", "posisi teks",
            ],
            Command::Backdrops => &["backdrop", "backgrounds", "stock", "pictures", "videos", "wallpaper", "nature", "latar", "gambar", "video", "pemandangan"],
            Command::Settings => &["settings", "preferences", "options", "language", "dark", "light", "pengaturan", "setelan", "bahasa"],
            Command::Screens => &["projector", "monitor", "display", "stage", "layar", "proyektor"],
            Command::Import => &["easislides", "impor", "pindahkan"],
            Command::ImportFolder => &["openlyrics", "impor lagu", "folder lagu"],
            Command::Remote => &["mobile", "qr", "hp", "ponsel", "telepon", "remote hp"],
            Command::Heading => &["divider", "section", "judul bagian", "pembatas"],
            Command::PlanDuplicate => &["copy plan", "duplicate plan", "clone", "same songs", "copy service", "salin rundown", "gandakan", "duplikat", "sama dengan"],
            Command::PlanRename => &["rename plan", "name", "day", "time", "hour", "change time", "ganti nama", "hari", "jam", "ganti jam", "ganti hari"],
            Command::PlanDelete => &["delete plan", "remove plan", "hapus rundown", "buang rundown"],
            Command::Earlier => &["back", "go back", "earlier", "shown before", "previous song", "last song", "kembali", "balik", "tadi", "lagu tadi", "sebelumnya"],
            Command::StartHere => &["start at", "start from", "begin at", "medley", "start at chorus", "start here", "mulai dari", "mulai dari reff", "medley", "sambung"],
            Command::NewAnnouncement => &["new announcement", "notice", "verse of the week", "birthday", "welcome", "schedule", "offering", "pengumuman baru", "warta", "ayat minggu ini", "ayat hafalan", "ulang tahun", "selamat datang", "jadwal", "persembahan"],
            Command::NewPlan => &["new service", "reset plan", "rencana baru", "susunan baru", "ibadah baru"],
            Command::Keys => &["shortcuts", "help", "hotkeys", "pintasan", "tombol", "bantuan"],
            Command::Tour => &["tutorial", "help", "guide", "walkthrough", "intro", "onboarding", "panduan", "bantuan", "tur", "pengenalan", "cara pakai"],
            Command::FirstRun => &["setup", "set up", "first run", "first start", "welcome", "onboarding", "again", "language", "country", "region"],
            Command::StartService => &["start live", "go live", "begin", "mulai", "mulai ibadah", "tayangkan"],
            Command::Next => &["forward", "lanjut", "berikutnya", "maju"],
            Command::Previous => &["back", "sebelumnya", "kembali", "mundur"],
            Command::Black => &["blank", "screen off", "hitam", "gelap", "layar hitam", "matikan layar"],
            Command::ClearText => &["hide words", "hide lyrics", "sembunyikan teks", "kosongkan teks", "hapus teks"],
            Command::Logo => &["show logo", "tampilkan logo"],
            Command::StopCountdown => &["stop timer", "cancel countdown", "hentikan hitung mundur", "batal hitung mundur"],
            Command::HideMessage => &["clear message", "sembunyikan pesan", "hapus pesan"],
            Command::EditSong => &["edit", "fix lyrics", "ubah lagu", "sunting lagu", "perbaiki lirik"],
            Command::Undo => &["revert", "batalkan", "urungkan"],
            Command::LoopAnnouncements => &["auto advance", "slideshow", "putar pengumuman", "ulang pengumuman", "otomatis"],
            Command::ImportPptx => &["powerpoint", "ppt", "pptx", "slides", "warta", "impor powerpoint"],
            Command::ImportBirthdays => &["birthdays", "birthday", "ulang tahun", "ultah", "hut"],
            Command::CheckUpdate => &["update", "upgrade", "new version", "version", "perbarui", "pembaruan", "versi baru"],
            Command::Listen => &["microphone", "mic", "hear", "speech", "dengar", "dengarkan", "mikrofon", "suara"],
            Command::Stream => &["live stream", "broadcast", "youtube", "facebook", "record", "recording", "siaran", "siaran langsung", "rekam", "streaming"],
            Command::BibleFile => &["bible file", "import bible", "mybible", "usfm", "usfx", "tb", "alkitab", "berkas alkitab", "impor alkitab", "tambah alkitab"],
            Command::Bibles => &["download bible", "translations", "ebible", "alkitab", "terjemahan", "unduh alkitab", "versi alkitab", "more bibles"],
        }
    }

    /// Whether the command can do anything now. One that cannot is left out of the results.
    fn usable(self, app: &App) -> bool {
        self.usable_in(app, crate::store::packaged())
    }

    /// `usable` for a program that is, or is not, the Microsoft Store's: the Store updates it, so
    /// it never offers to check for a new version itself.
    fn usable_in(self, app: &App, packaged: bool) -> bool {
        match self {
            Command::CheckUpdate => !packaged,
            Command::Import => app.easislides.is_some(),
            Command::StartService => app.plan.items.iter().any(|item| !item.entry.is_heading()),
            Command::Next | Command::Previous => app.live.is_some() || app.open.is_some(),
            Command::Logo => !app.settings.logo.is_empty(),
            Command::StopCountdown => app.countdown.is_some(),
            Command::HideMessage => app.message_on || app.stage_message_on,
            Command::EditSong => matches!(app.open.as_ref().map(|open| &open.deck.source), Some(DeckSource::Song(_))),
            Command::Undo => app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()),
            Command::LoopAnnouncements => !app.announcements.is_empty(),
            // Offered whenever the plan holds a song: a person who types "start at chorus" must find
            // it, and is told what to open first when Preview holds something else.
            Command::Earlier => !app.history.is_empty(),
            Command::StartHere => app.plan.items.iter().any(|item| matches!(item.entry, artosia_core::plan::PlanEntry::Song { .. })),
            _ => true,
        }
    }
}

/// Why a song is where it is, from what the search learned. Each is one plain count.
#[derive(Clone, Debug, PartialEq)]
pub enum Reason {
    /// Picked this many times for the text typed, the last time at this unix second.
    Habit { typed: String, picks: u32, last: i64 },
    /// Came right after the live song in this many past services.
    Follows { count: u32, last: i64 },
    /// Sung on this day of the week in this many of the last `of` services.
    Weekday { count: u32, of: u32 },
    /// Picked from the list with nothing typed.
    Recent { picks: u32, last: i64 },
}

/// A heading above a group of suggested songs. It is never picked.
#[derive(Clone, Debug, PartialEq)]
pub enum Group {
    /// After the live song, named by its title.
    Follows(String),
    Weekday,
    Recent,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Song { song: usize, title_ranges: Vec<Range<usize>>, line: Option<LineHit>, planned: bool, reason: Option<Reason> },
    Group(Group),
    /// A section of the attached song, at its place in the sung order.
    Section { song: usize, slide: usize, tag: Tag, label: String, text: String },
    /// A reference read from the query, or a book or chapter to step into.
    Passage { reference: Reference, title: String, text: String, ranges: Vec<Range<usize>>, step: Option<String> },
    /// Something in today's plan that is not a song.
    PlanItem { id: u64, title: String },
    Announcement { index: usize, title: String, body: String },
    /// Another of the church's plans, which opens in place of this one.
    Plan { id: i64, name: String },
    /// A look Artosia ships, by its place in the presets.
    Look { index: usize, name: String },
    Bible { code: String, name: String },
    Command(Command),
    Countdown(u32),
    Message(String),
    /// A message for the musicians, on the stage screen only.
    StageMessage(String),
    /// Switch the window to the language with this code.
    Language(&'static str),
}

/// What Enter does to the picked result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Open,
    ShowNow,
    AddToPlan,
}

#[derive(Default)]
pub struct Palette {
    pub open: bool,
    pub query: TextEdit,
    pub scope: Scope,
    /// The song the search is inside: its sections are the results.
    pub chip: Option<usize>,
    pub hits: Vec<Hit>,
    pub picked: usize,
    /// How many songs matched before the list was cut, for the count line.
    pub total: usize,
    /// Opened from the plan's Add button: Enter and a click put the row in the plan, and the
    /// search stays open for the next one.
    pub adding: bool,
}

const SONG_LIMIT: usize = 40;
/// The most commands and other named things that may go above the songs.
const NAMED_LIMIT: usize = 4;

fn verse_title(app: &App, reference: &Reference) -> String {
    app.bible().map(|bible| bible.format(reference)).unwrap_or_else(|| english_name(reference.book).to_string())
}

fn passage_text(app: &App, reference: &Reference, limit: usize) -> String {
    let Some(bible) = app.bible() else { return String::new() };
    let mut out = String::new();
    for verse in bible.verses(reference) {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&verse.text);
        if out.len() > limit {
            break;
        }
    }
    out
}

/// Words that find a section besides its name: what Indonesian song sheets call it.
fn section_aliases(tag: Tag) -> Vec<String> {
    let words: &[&str] = match tag.kind {
        SectionKind::Verse => &["bait", "ayat", "stanza"],
        SectionKind::Chorus => &["reff", "ref", "refrain", "koor"],
        SectionKind::PreChorus => &["pre", "prechorus", "pre reff"],
        SectionKind::Bridge => &["jembatan"],
        SectionKind::Intro => &["pembuka", "opening"],
        SectionKind::Ending => &["end", "coda", "penutup", "akhir", "outro"],
        SectionKind::Other => &["bagian"],
    };
    let mut out: Vec<String> = words.iter().map(|word| format!("{word} {}", tag.number)).collect();
    out.push(tag.short());
    out
}

impl App {
    pub fn open_palette(&mut self, scope: Scope, seed: &str) {
        self.palette.open = true;
        self.palette.adding = false;
        self.palette.scope = scope;
        self.palette.chip = None;
        self.palette.query = TextEdit::new(seed);
        self.palette.query.focused = true;
        self.palette.query.set_caret(seed.len());
        self.palette.picked = 0;
        self.plan_menu = None;
        self.more_menu = false;
        self.refresh_plan_rows();
        self.want_bible(&self.settings.bible.clone());
        self.search();
    }

    /// The search as the plan's Add button opens it (owner, 2026-11-11: "Pressing add to plan
    /// only opens the palette?").
    pub fn open_palette_adding(&mut self, scope: Scope) {
        self.open_palette(scope, "");
        self.palette.adding = true;
        self.add_menu = false;
    }

    pub fn close_palette(&mut self) {
        self.palette.open = false;
        self.palette.adding = false;
        self.palette.chip = None;
        self.palette.query.focused = false;
    }

    fn pick_into_plan(&mut self, hit: Hit) {
        match hit {
            Hit::Song { song, .. } | Hit::Section { song, .. } => self.add_song_to_plan(song),
            Hit::Passage { reference, .. } => self.add_passage_to_plan(&reference),
            Hit::Announcement { .. } => self.add_announcements_to_plan(),
            _ => {}
        }
    }

    /// Escape closes the search at once; from inside a song it first goes back out to the
    /// whole search. It used to empty the box first, and a tester who had opened the search by
    /// a slip of a letter key pressed it once and was still inside.
    pub fn palette_escape(&mut self) {
        if self.palette.chip.is_none() {
            self.close_palette();
            return;
        }
        self.palette.chip = None;
        self.set_query("");
    }

    pub fn set_query(&mut self, text: &str) {
        self.palette.query = TextEdit::new(text);
        self.palette.query.focused = true;
        self.palette.query.set_caret(text.len());
        self.palette.picked = 0;
        self.search();
    }

    pub fn set_scope(&mut self, scope: Scope) {
        self.palette.scope = scope;
        self.palette.picked = 0;
        self.search();
    }

    /// Tab: steps into the picked song or book; with nothing to step into, changes the scope.
    pub fn palette_tab(&mut self) {
        match self.palette.hits.get(self.palette.picked).cloned() {
            Some(Hit::Song { song, line, .. }) => {
                self.palette.chip = Some(song);
                self.set_query("");
                // The section the search found stays picked.
                if let Some(tag) = line.map(|line| line.tag) {
                    let at = self.palette.hits.iter().position(|hit| matches!(hit, Hit::Section { tag: own, .. } if *own == tag));
                    self.palette.picked = at.unwrap_or(0);
                }
            }
            Some(Hit::Passage { step: Some(_), .. }) => self.pick(Pick::Open),
            _ if self.palette.chip.is_some() => {}
            _ => {
                let next = match self.palette.scope {
                    Scope::All => Scope::Songs,
                    Scope::Songs => Scope::Bible,
                    Scope::Bible => Scope::All,
                };
                self.set_scope(next);
            }
        }
    }

    /// Backspace in an empty search lets go of the attached song. False when there is none.
    pub fn palette_detach(&mut self) -> bool {
        if self.palette.chip.is_none() || !self.palette.query.text.is_empty() {
            return false;
        }
        self.palette.chip = None;
        self.set_query("");
        true
    }

    /// Recomputes the results for the current query and scope.
    pub fn search(&mut self) {
        let query = self.palette.query.text.trim().to_string();
        self.palette.total = 0;
        self.read_starts();
        let hits = match self.palette.chip {
            Some(song) => self.section_hits(song, &query),
            None if query.is_empty() => self.browse_hits(),
            None => self.query_hits(&query),
        };
        self.palette.hits = hits;
        self.palette.picked = self.palette.picked.min(self.palette.hits.len().saturating_sub(1));
        // A heading cannot be picked: Enter takes the first row under it.
        while matches!(self.palette.hits.get(self.palette.picked), Some(Hit::Group(_))) && self.palette.picked + 1 < self.palette.hits.len() {
            self.palette.picked += 1;
        }
    }

    fn browse_hits(&mut self) -> Vec<Hit> {
        match self.palette.scope {
            Scope::Bible => self.book_hits(),
            scope => {
                let mut hits = Vec::new();
                if scope == Scope::All {
                    // The two ways to add a song lead the list; other commands answer to typing.
                    hits.extend([Hit::Command(Command::NewSong), Hit::Command(Command::PasteSong)]);
                    hits.extend(self.suggestion_rows());
                }
                self.palette.total = self.songs.len();
                let planned = self.planned_songs();
                hits.extend((0..self.songs.len()).map(|song| Hit::Song { song, title_ranges: Vec::new(), line: None, planned: planned.contains(&song), reason: None }));
                hits
            }
        }
    }

    /// The songs in today's plan, as places in the library.
    pub(crate) fn planned_songs(&self) -> HashSet<usize> {
        self.plan
            .items
            .iter()
            .filter_map(|item| match &item.entry {
                PlanEntry::Song { song_id, .. } => self.song_position(*song_id),
                _ => None,
            })
            .collect()
    }

    /// The order of a typed search, top to bottom: a countdown or message typed out, a Bible
    /// reference, songs in today's plan, commands and other things found by name, every other
    /// song, what was found by another word, then Bible text.
    fn query_hits(&mut self, query: &str) -> Vec<Hit> {
        let scope = self.palette.scope;
        let mut hits: Vec<Hit> = Vec::new();
        if scope == Scope::All {
            hits.extend(self.typed_hits(query));
        }
        let passages = if scope == Scope::Songs { Vec::new() } else { self.passage_hits(query) };
        // A typed reference is nearly always what was meant, so it goes first.
        let reference_first = passages.iter().any(|hit| matches!(hit, Hit::Passage { ranges, .. } if ranges.is_empty()));
        let (mut first_songs, mut songs) = (Vec::new(), Vec::new());
        if scope != Scope::Bible {
            let planned = self.planned_songs();
            // What was picked for this text lately only orders songs that tied on everything else.
            let habits = self.habit_weights(query);
            let weight = |song: usize| habits.get(&song).map_or(0, |habit| habit.weight());
            let found = self.index.search_ranked(query, SONG_LIMIT, &|song| planned.contains(&song), &weight);
            self.palette.total = found.len();
            let typed = artosia_core::learn::fold_typed(query);
            for hit in found {
                let leads = hit.preferred && hit.complete;
                let reason = habits.get(&hit.song).filter(|_| hit.habit > 0).map(|habit| Reason::Habit { typed: typed.clone(), picks: habit.picks, last: habit.last });
                let row = Hit::Song { song: hit.song, title_ranges: hit.title_ranges, line: hit.line, planned: hit.preferred, reason };
                if leads { first_songs.push(row) } else { songs.push(row) }
            }
        }
        let (mut leading, mut named, mut others) = (Vec::new(), Vec::new(), Vec::new());
        // A word that is a whole word of a command's or a plan's name was meant for it.
        let mut names_a_thing = false;
        if scope == Scope::All {
            let found = self.documents();
            let docs: Vec<Doc> = found.iter().map(|(doc, _)| doc.clone()).collect();
            let typed = artosia_core::search::tokens(query).len();
            for hit in kit::search(&docs, query, &|doc| doc.group == "plan", 24) {
                // Every typed word is a whole word of the name itself.
                let whole = hit.typos == 0 && hit.infix == 0 && hit.weight >= LABEL && usize::from(hit.exact) == typed;
                names_a_thing |= whole;
                // The name and nothing else was typed: "today" for the plan Today. It leads
                // even a planned song that only holds the word in a line.
                let is_name = whole && hit.position == 0 && artosia_core::text::words(&docs[hit.doc].display).len() == typed;
                // Two or more words standing together in a name count wherever they stand in it:
                // "new plan" finds "Start a new plan" before songs that hold one of the words.
                let by_name = hit.typos == 0 && hit.infix == 0 && hit.weight >= LABEL && (hit.position == 0 || (typed >= 2 && whole && hit.proximity == 0));
                let row = found[hit.doc].1.clone();
                if is_name && leading.len() < NAMED_LIMIT {
                    leading.push(row)
                } else if by_name && named.len() < NAMED_LIMIT {
                    named.push(row)
                } else {
                    others.push(row)
                }
            }
        }
        // A book of the Bible guessed from a word that names a command or a plan does not go
        // first: "plan" was read as a book and filled the list with its chapters.
        let reference_first = reference_first && !names_a_thing;
        if reference_first {
            hits.extend(passages.iter().cloned());
        }
        hits.extend(leading);
        hits.extend(first_songs);
        hits.extend(named);
        hits.extend(songs);
        hits.extend(others);
        if !reference_first {
            hits.extend(passages);
        }
        hits
    }

    /// A command typed with what it takes: "countdown 5", "message the car park is full".
    fn typed_hits(&self, query: &str) -> Vec<Hit> {
        let timers = [Doc::new("countdown", "action", t::COUNTDOWN()).aliases(&["timer", "count down", "hitung mundur", "waktu mundur"])];
        let is_timer = |words: &str| !kit::search(&timers, words, &|_| false, 1).is_empty();
        // Anything else that ends in a number goes on: a message may end in a room number.
        if let Some((head, minutes)) = kit::trailing_number(query)
            && minutes > 0
            && minutes <= 600
            && is_timer(&head)
        {
            return vec![Hit::Countdown(minutes)];
        }
        if is_timer(query) && self.countdown.is_none() {
            return vec![Hit::Countdown(5), Hit::Countdown(10), Hit::Countdown(15)];
        }
        let lower = query.to_lowercase();
        for verb in ["stage ", "band ", "panggung ", "pemusik "] {
            if let Some(rest) = lower.strip_prefix(verb) {
                let text = query[query.len() - rest.len()..].trim();
                if !text.is_empty() {
                    return vec![Hit::StageMessage(text.to_string())];
                }
            }
        }
        for verb in ["message ", "msg ", "say ", "pesan ", "umumkan "] {
            if let Some(rest) = lower.strip_prefix(verb) {
                let text = query[query.len() - rest.len()..].trim();
                if !text.is_empty() {
                    return vec![Hit::Message(text.to_string())];
                }
            }
        }
        Vec::new()
    }

    /// Everything that is not a song or a verse, each with the row it becomes.
    fn documents(&self) -> Vec<(Doc, Hit)> {
        let mut out: Vec<(Doc, Hit)> = Vec::new();
        for item in &self.plan.items {
            let title = self.entry_title(&item.entry);
            match &item.entry {
                PlanEntry::Passage { .. } | PlanEntry::Announcements => {
                    let doc = Doc::new(format!("plan:{}", item.id), "plan", title.clone()).detail(t::IN_PLAN()).aliases(&["plan", "today", "rencana", "hari ini"]);
                    out.push((doc, Hit::PlanItem { id: item.id, title }));
                }
                // Songs in the plan are found as songs, ahead of the rest of the library.
                PlanEntry::Song { .. } | PlanEntry::Heading { .. } => {}
            }
        }
        // The church's other plans, found by name, by day and time, and by the word for plans
        // itself, which lists them all; the open one is already open.
        for row in self.switcher.rows.iter().filter(|row| row.id != self.plan.id) {
            let name = row.label();
            let slot = crate::plans::slot_words(row.day, row.time).unwrap_or_default();
            let doc = Doc::new(format!("planfile:{}", row.id), "plans", name.clone()).detail(t::KIND_PLAN()).aliases(&["plan", "plans", "service", "services", "other service", "rundown", "semua rundown", "rencana", "acara", "ibadah", "ibadah lain", slot.as_str()]);
            out.push((doc, Hit::Plan { id: row.id, name }));
        }
        for command in Command::ALL {
            if command.usable(self) {
                out.push((Doc::new(format!("command:{command:?}"), "action", command.label()).detail(command.detail()).aliases(command.aliases()), Hit::Command(command)));
            }
        }
        for (index, item) in self.announcements.iter().enumerate() {
            // A card that fills itself has no body of its own: it says what it holds today.
            let body = match item.fill {
                Fill::Nothing => item.body.clone(),
                Fill::Passage => item.passage.clone(),
                _ => t::names_today(artosia_core::deck::birthdays(&self.people, item.fill, &self.today()).len()),
            };
            let mut doc = Doc::new(format!("announcement:{}", item.id), "announcement", item.title.clone())
                .detail(body.clone())
                .aliases(&["announcement", "show announcement", "notice", "pengumuman", "tampilkan pengumuman", "warta"]);
            doc = match item.fill {
                Fill::BirthdaysWeek | Fill::BirthdaysMonth => doc.aliases(&["birthday", "birthdays", "show birthdays", "ulang tahun", "ultah", "tampilkan ulang tahun"]),
                Fill::Passage => doc.aliases(&["verse of the week", "bible verse", "verse", "ayat", "ayat minggu ini"]),
                Fill::Nothing => doc,
            };
            out.push((doc, Hit::Announcement { index, title: item.title.clone(), body }));
        }
        for (index, look) in presets().into_iter().enumerate() {
            let doc = Doc::new(format!("look:{}", look.name), "look", t::look_label(&look.name)).aliases(&["look", "theme", "background", "tampilan", "tema", "latar"]).aliases(&[look.name.as_str()]);
            out.push((doc, Hit::Look { index, name: t::look_label(&look.name) }));
        }
        for (_, language) in crate::languages::listed() {
            let doc = Doc::new(format!("language:{}", language.code), "language", language.name).aliases(&["language", "bahasa"]).aliases(language.aliases);
            out.push((doc, Hit::Language(language.code)));
        }
        if self.bible_files.len() > 1 {
            for file in &self.bible_files {
                let doc = Doc::new(format!("bible:{}", file.code), "bible", file.name.clone())
                    .aliases(&[file.code.as_str(), "bible", "translation", "version", "alkitab", "terjemahan", "versi"]);
                out.push((doc, Hit::Bible { code: file.code.clone(), name: file.name.clone() }));
            }
        }
        out
    }

    /// The attached song's sections in sung order, or those the typed words find.
    fn section_hits(&self, song: usize, query: &str) -> Vec<Hit> {
        let Some(found) = self.songs.get(song) else { return Vec::new() };
        let deck = self.deck_of_song(found);
        let rows: Vec<Hit> = section_starts(&deck)
            .into_iter()
            .filter_map(|slide| {
                let first = deck.slides.get(slide)?;
                let tag = first.tag?;
                // A section split over several slides reads as one.
                let text: Vec<&str> = deck.slides[slide..].iter().take_while(|other| other.tag == Some(tag)).map(|other| other.text.as_str()).collect();
                Some(Hit::Section { song, slide, tag, label: t::section_label(tag), text: text.join("\n") })
            })
            .collect();
        if query.is_empty() {
            return rows;
        }
        let docs: Vec<Doc> = rows
            .iter()
            .enumerate()
            .map(|(at, row)| match row {
                Hit::Section { tag, label, text, .. } => {
                    let aliases = section_aliases(*tag);
                    let aliases: Vec<&str> = aliases.iter().map(String::as_str).collect();
                    Doc::new(format!("section:{at}"), "section", label.clone()).detail(text.clone()).aliases(&aliases)
                }
                _ => Doc::default(),
            })
            .collect();
        kit::search(&docs, query, &|_| false, rows.len()).into_iter().map(|hit| rows[hit.doc].clone()).collect()
    }

    /// Every book as a step into its chapters, for people who would rather click than type.
    fn book_hits(&self) -> Vec<Hit> {
        let Some(bible) = self.bible() else { return Vec::new() };
        bible
            .books()
            .iter()
            .map(|book| Hit::Passage {
                reference: Reference { book: book.number, chapter: 1, verse: 1, end_chapter: 1, end_verse: book.chapters.first().copied().unwrap_or(1) },
                title: book.name.clone(),
                text: t::chapters(book.chapters.len()),
                ranges: Vec::new(),
                step: Some(format!("{} ", book.name)),
            })
            .collect()
    }

    fn passage_hits(&self, query: &str) -> Vec<Hit> {
        let Some(bible) = self.bible() else { return Vec::new() };
        let mut hits = Vec::new();
        let guesses = bible.guess(query);
        if let Some(best) = guesses.first() {
            let book = bible.book(best.reference.book);
            if best.book_only {
                // A bare book lists its chapters to step into.
                let chapters = book.map(|book| book.chapters.len()).unwrap_or(0);
                for chapter in 1..=chapters as u16 {
                    let last = book.and_then(|book| book.chapters.get(usize::from(chapter) - 1).copied()).unwrap_or(1);
                    let reference = Reference { book: best.reference.book, chapter, verse: 1, end_chapter: chapter, end_verse: last };
                    hits.push(Hit::Passage {
                        reference,
                        title: bible.format(&reference),
                        text: passage_text(self, &Reference::single(reference.book, chapter, 1), 120),
                        ranges: Vec::new(),
                        step: Some(format!("{} {chapter}:", book.map(|b| b.name.as_str()).unwrap_or(""))),
                    });
                }
            } else if best.whole_chapter {
                hits.push(Hit::Passage {
                    reference: best.reference,
                    title: bible.format(&best.reference),
                    text: t::verses(usize::from(best.reference.end_verse)),
                    ranges: Vec::new(),
                    step: None,
                });
                for verse in bible.verses(&best.reference) {
                    let reference = Reference::single(verse.book, verse.chapter, verse.verse);
                    hits.push(Hit::Passage { reference, title: bible.format(&reference), text: verse.text.to_string(), ranges: Vec::new(), step: None });
                }
            } else {
                hits.push(Hit::Passage {
                    reference: best.reference,
                    title: bible.format(&best.reference),
                    text: passage_text(self, &best.reference, 240),
                    ranges: Vec::new(),
                    step: None,
                });
            }
            // Other books the name could mean, such as Joshua for "jo".
            for other in guesses.iter().skip(1).take(3) {
                hits.push(Hit::Passage {
                    reference: other.reference,
                    title: bible.format(&other.reference),
                    text: passage_text(self, &Reference::single(other.reference.book, other.reference.chapter, other.reference.verse), 120),
                    ranges: Vec::new(),
                    step: None,
                });
            }
            return hits;
        }
        if query.chars().count() < 3 {
            return hits;
        }
        for found in bible.search(query, 24) {
            if self.palette.scope == Scope::All && !found.complete {
                continue;
            }
            let verse = &bible.all_verses()[found.index];
            let reference = Reference::single(verse.book, verse.chapter, verse.verse);
            hits.push(Hit::Passage { reference, title: bible.format(&reference), text: verse.text.to_string(), ranges: found.ranges, step: None });
        }
        hits
    }

    /// Whether every result is a book or chapter to step into, which draws as a grid.
    pub fn palette_is_grid(&self) -> bool {
        !self.palette.hits.is_empty() && self.palette.hits.iter().all(|hit| matches!(hit, Hit::Passage { step: Some(_), .. }))
    }

    pub fn move_pick(&mut self, by: isize) {
        let count = self.palette.hits.len();
        if count == 0 {
            return;
        }
        let mut next = (self.palette.picked as isize + by).rem_euclid(count as isize);
        // Headings are stepped over, in the direction of travel.
        for _ in 0..count {
            if !matches!(self.palette.hits[next as usize], Hit::Group(_)) {
                break;
            }
            next = (next + by.signum()).rem_euclid(count as isize);
        }
        self.palette.picked = next as usize;
    }

    /// Acts on the picked result.
    pub fn pick(&mut self, how: Pick) {
        let Some(hit) = self.palette.hits.get(self.palette.picked).cloned() else { return };
        // While adding, the main choice of a row that can be planned is to plan it.
        let joins = matches!(hit, Hit::Song { .. } | Hit::Section { .. } | Hit::Announcement { .. } | Hit::Passage { step: None, .. });
        let how = if self.palette.adding && how == Pick::Open && joins { Pick::AddToPlan } else { how };
        self.log_pick(&hit, how);
        if self.palette.adding && how == Pick::AddToPlan && joins {
            // The field empties for the next one.
            self.pick_into_plan(hit);
            self.palette.query = TextEdit::new("");
            self.palette.query.focused = true;
            self.palette.chip = None;
            self.palette.picked = 0;
            self.search();
            return;
        }
        match hit {
            Hit::Group(_) => {}
            Hit::Song { song, line, .. } => {
                if how == Pick::AddToPlan {
                    self.add_song_to_plan(song);
                    return;
                }
                self.open_song(song, line.map(|line| line.section));
                self.close_palette();
                if how == Pick::ShowNow {
                    let cursor = self.open.as_ref().map(|open| open.cursor).unwrap_or(0);
                    self.show(cursor);
                }
            }
            Hit::Section { song, slide, .. } => {
                if how == Pick::AddToPlan {
                    self.add_song_to_plan(song);
                    return;
                }
                self.open_song(song, None);
                self.close_palette();
                // Opening the live song keeps the cursor on the live slide; a picked section moves it.
                if let Some(open) = self.open.as_mut() {
                    open.cursor = slide.min(open.deck.slides.len().saturating_sub(1));
                }
                if how == Pick::ShowNow {
                    self.show(slide);
                }
            }
            Hit::Passage { reference, step, .. } => {
                if let (Some(step), Pick::Open) = (step, how) {
                    // Stepping into a book or chapter refines the query instead of opening it.
                    self.set_query(&step);
                    return;
                }
                if how == Pick::AddToPlan {
                    self.add_passage_to_plan(&reference);
                    return;
                }
                self.open_passage(&reference);
                self.close_palette();
                if how == Pick::ShowNow {
                    self.show(0);
                }
            }
            Hit::PlanItem { id, .. } => {
                self.open_plan_item(id);
                self.close_palette();
                if how == Pick::ShowNow {
                    self.show_start_of(id);
                }
            }
            Hit::Plan { id, .. } => {
                self.close_palette();
                self.switch_plan(id);
            }
            Hit::Announcement { index, .. } => {
                if how == Pick::AddToPlan {
                    self.add_announcements_to_plan();
                    return;
                }
                // An announcement that fills itself may have no slide today, or several.
                let Some(slide) = self.announcement_slide_index(index) else {
                    self.say(t::NOTICE_NOTHING_TO_SHOW(), None);
                    return;
                };
                self.open_announcements();
                self.close_palette();
                if let Some(open) = self.open.as_mut() {
                    open.cursor = slide.min(open.deck.slides.len().saturating_sub(1));
                }
                if how == Pick::ShowNow {
                    self.show(slide);
                }
            }
            Hit::Look { index, .. } => {
                self.close_palette();
                if let Some(preset) = presets().into_iter().nth(index) {
                    let chosen = self.look().with_preset(preset);
                    self.set_look(chosen);
                }
            }
            Hit::Bible { code, .. } => {
                self.close_palette();
                self.set_bible(&code);
            }
            Hit::Command(command) => {
                self.close_palette();
                self.run(command);
            }
            Hit::Countdown(minutes) => {
                self.close_palette();
                self.timer_minutes.replace_text(minutes.to_string());
                self.start_countdown();
            }
            Hit::Language(code) => {
                self.close_palette();
                self.set_language(code);
            }
            Hit::Message(text) => {
                self.close_palette();
                self.message.replace_text(text);
                self.message_on = true;
            }
            Hit::StageMessage(text) => {
                self.close_palette();
                self.stage_message.replace_text(text);
                self.stage_message_on = true;
            }
        }
    }

    pub fn run(&mut self, command: Command) {
        match command {
            Command::NewSong => self.edit_new_song(artosia_core::song::Song::default()),
            Command::PasteSong => self.want_paste = true,
            Command::Announcements => self.edit_announcements(),
            Command::Looks => {
                // The part the typed word lives in opens first: "yellow" and "font" are about the words.
                let typed = self.palette.query.text.to_lowercase();
                let about = |words: &[&str]| words.iter().any(|word| typed.contains(word));
                if about(&["logo"]) {
                    self.design_part = 2;
                } else if about(&["font", "huruf", "colo", "warna", "yellow", "kuning", "white", "putih", "outline", "garis", "shadow", "size", "ukuran", "word", "text", "teks", "line", "baris", "posi"]) {
                    self.design_part = 1;
                } else if about(&["background", "latar", "picture", "gambar", "video", "camera", "kamera"]) {
                    self.design_part = 0;
                }
                self.sheet = Sheet::Looks;
            }
            Command::Settings => self.sheet = Sheet::Settings,
            Command::Backdrops => self.open_backdrops(),
            Command::Screens => self.sheet = Sheet::Screens,
            Command::Import => self.sheet = Sheet::Import,
            Command::ImportFolder => self.want_folder = true,
            Command::Remote => self.sheet = Sheet::Remote,
            Command::Heading => self.add_heading(),
            Command::NewPlan => self.new_plan(),
            Command::PlanDuplicate => self.begin_copy(self.plan.id),
            Command::PlanRename => self.begin_edit(self.plan.id),
            Command::PlanDelete => self.delete_plan(),
            Command::Earlier => drop(self.open_earlier()),
            Command::StartHere => match self.start_control() {
                Some((_, _, state)) if state != crate::starts::StartState::Beginning => self.toggle_start_here(),
                _ => self.say(t::START_HERE_HOW(), None),
            },
            Command::NewAnnouncement => {
                self.edit_announcements();
                self.new_notice("", "");
            }
            Command::Keys => self.sheet = Sheet::Keys,
            Command::Tour => self.start_tour(),
            Command::FirstRun => self.start_onboarding(),
            Command::StartService => self.start_service(),
            Command::Next => self.next(),
            Command::Previous => self.previous(),
            Command::Black => self.toggle_black(),
            Command::ClearText => self.toggle_text(),
            Command::Logo => self.toggle_logo(),
            Command::StopCountdown => self.countdown = None,
            Command::HideMessage => {
                self.message_on = false;
                self.stage_message_on = false;
            }
            Command::EditSong => self.edit_open_song(),
            Command::Undo => self.undo(),
            Command::LoopAnnouncements => {
                self.open_announcements();
                self.show(0);
                self.looping = true;
            }
            Command::ImportPptx => self.want_pptx = true,
            Command::ImportBirthdays => self.want_birthdays = true,
            Command::CheckUpdate => crate::update::check(self.inbox.clone(), self.waker.clone(), true),
            Command::Listen => self.open_listen(),
            Command::Stream => self.open_stream(),
            Command::BibleFile => self.desk.want_file = true,
            Command::Bibles => self.open_bibles(),
        }
    }

    /// The title a passage shows in lists, in the active translation's book names.
    pub fn passage_title(&self, reference: &Reference) -> String {
        verse_title(self, reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::practice();
        app.open_palette(Scope::All, "");
        app
    }

    /// A command named by every typed word leads songs that hold one of the words in a line
    /// (a keys-only tester typed "new plan" and found two songs above the command).
    #[test]
    fn words_that_name_a_command_find_it_first() {
        let mut app = app();
        search(&mut app, "new plan");
        assert!(matches!(app.palette.hits.first(), Some(Hit::Command(Command::NewPlan))), "{:?}", app.palette.hits.first().map(|hit| std::mem::discriminant(hit)));
        search(&mut app, "copy plan");
        assert!(matches!(app.palette.hits.first(), Some(Hit::Command(Command::PlanDuplicate))));
    }

    /// The Store updates the program, so its edition offers no check for a new version, in the
    /// search or in Settings; the other edition offers both (`store::packaged`).
    #[test]
    fn the_update_check_is_left_out_of_the_store_edition_only() {
        let app = App::practice();
        assert!(!Command::CheckUpdate.usable_in(&app, true));
        assert!(Command::CheckUpdate.usable_in(&app, false));
        // Every other command is offered in both editions.
        for command in Command::ALL.into_iter().filter(|command| *command != Command::CheckUpdate) {
            assert_eq!(command.usable_in(&app, true), command.usable_in(&app, false), "{command:?}");
        }
        assert!(Command::FirstRun.usable_in(&app, true));
    }

    fn search(app: &mut App, query: &str) {
        app.palette.query = TextEdit::new(query);
        app.search();
    }

    /// A tester typed "yellow" to make the words yellow and got Bible verses only.
    #[test]
    fn design_is_found_by_what_is_wanted_from_it_and_opens_on_that_part() {
        let mut app = app();
        for (typed, part) in [("yellow", 1), ("outline", 1), ("warna teks", 1), ("background", 0)] {
            search(&mut app, typed);
            assert!(matches!(app.palette.hits.first(), Some(Hit::Command(Command::Looks))), "{typed}");
            app.design_part = 2;
            app.pick(Pick::Open);
            assert_eq!((app.sheet, app.design_part), (Sheet::Looks, part), "{typed}");
            app.sheet = Sheet::None;
        }
    }

    #[test]
    fn what_was_shown_before_is_a_command_once_something_was() {
        let mut app = app();
        search(&mut app, "go back");
        assert!(!app.palette.hits.iter().any(|hit| matches!(hit, Hit::Command(Command::Earlier))));
        let songs: Vec<u64> = app.plan.items.iter().filter(|item| matches!(item.entry, artosia_core::plan::PlanEntry::Song { .. })).map(|item| item.id).collect();
        app.open_plan_item(songs[0]);
        app.show(0);
        app.open_plan_item(songs[1]);
        app.show(0);
        search(&mut app, "go back");
        assert!(matches!(app.palette.hits.first(), Some(Hit::Command(Command::Earlier))));
        app.pick(Pick::Open);
        assert_eq!(app.open.as_ref().map(|open| open.item), Some(Some(songs[0])));
        assert_eq!(app.live.as_ref().map(|live| live.item), Some(Some(songs[1])));
    }

    fn first(app: &App) -> Option<&Hit> {
        app.palette.hits.first()
    }

    #[test]
    fn a_command_is_found_by_its_name_and_by_another_language() {
        let mut app = app();
        search(&mut app, "black");
        assert_eq!(first(&app), Some(&Hit::Command(Command::Black)));
        search(&mut app, "layar hitam");
        assert!(app.palette.hits.contains(&Hit::Command(Command::Black)));
        search(&mut app, "start live");
        assert_eq!(first(&app), Some(&Hit::Command(Command::StartService)));
        search(&mut app, "mulai ibadah");
        assert!(app.palette.hits.contains(&Hit::Command(Command::StartService)));
    }

    #[test]
    fn the_birthday_card_is_found_by_birthday_and_ulang_tahun() {
        let mut app = App::practice();
        app.demo_notices("birthdays");
        app.open_palette(Scope::All, "");
        let at = app.announcements.iter().position(|card| card.fill == artosia_core::deck::Fill::BirthdaysWeek).unwrap_or_default();
        // The import command also answers to "birthday", so the card is only required to be
        // among the hits there; asking to show birthdays puts it first.
        for query in ["birthday", "ulang tahun", "show birthdays", "tampilkan ulang tahun"] {
            search(&mut app, query);
            assert!(app.palette.hits.iter().any(|hit| matches!(hit, Hit::Announcement { index, .. } if *index == at)), "{query:?}: {:?}", app.palette.hits);
        }
        search(&mut app, "show birthdays");
        assert!(matches!(first(&app), Some(Hit::Announcement { index, .. }) if *index == at), "{:?}", first(&app));
        // Ctrl Enter puts it on the projector at once, on the slide that holds the names.
        app.pick(Pick::ShowNow);
        assert!(app.live.as_ref().is_some_and(|live| live.deck.slides.iter().any(|slide| slide.text.contains("Sample Alpha"))));
    }

    #[test]
    fn an_announcement_is_shown_by_asking_for_it() {
        let mut app = app();
        let title = app.announcements[1].title.clone();
        let word = title.split_whitespace().next().unwrap_or_default().to_string();
        search(&mut app, &format!("show announcement {word}"));
        assert!(matches!(first(&app), Some(Hit::Announcement { index: 1, .. })), "{:?}", first(&app));
        app.pick(Pick::ShowNow);
        assert_eq!(app.live.as_ref().map(|live| live.index), Some(1));
    }

    #[test]
    fn a_countdown_and_a_message_are_typed_out() {
        let mut app = app();
        search(&mut app, "countdown 7");
        assert_eq!(first(&app), Some(&Hit::Countdown(7)));
        search(&mut app, "hitung mundur 3");
        assert_eq!(first(&app), Some(&Hit::Countdown(3)));
        search(&mut app, "message Doors open at nine");
        assert_eq!(first(&app), Some(&Hit::Message("Doors open at nine".into())));
        app.pick(Pick::Open);
        assert!(app.message_on && app.message.text == "Doors open at nine");
        // A message that ends in a number is still a message, never a countdown.
        search(&mut app, "message Parents of room 12");
        assert_eq!(first(&app), Some(&Hit::Message("Parents of room 12".into())));
        // The stage has its own word, since the window no longer has a field for it.
        search(&mut app, "panggung satu reff lagi");
        assert_eq!(first(&app), Some(&Hit::StageMessage("satu reff lagi".into())));
        app.pick(Pick::Open);
        assert!(app.stage_message_on && app.stage_message.text == "satu reff lagi");
    }

    #[test]
    fn a_song_in_the_plan_is_the_first_song() {
        let mut app = app();
        // Two practice songs share a word; planning the later one puts it first.
        let word = "morning";
        search(&mut app, word);
        let songs: Vec<usize> = app.palette.hits.iter().filter_map(|hit| if let Hit::Song { song, .. } = hit { Some(*song) } else { None }).collect();
        assert!(songs.len() >= 2, "the practice library needs two songs with {word:?}");
        let later = songs[1];
        app.plan.items.clear();
        app.add_song_to_plan(later);
        search(&mut app, word);
        assert!(matches!(first(&app), Some(Hit::Song { song, planned: true, .. }) if *song == later));
    }

    #[test]
    fn tab_steps_into_a_song_and_finds_its_chorus() {
        let mut app = app();
        let title = app.songs[1].title.clone();
        search(&mut app, &title);
        app.palette_tab();
        assert_eq!(app.palette.chip, Some(1));
        assert!(app.palette.hits.iter().all(|hit| matches!(hit, Hit::Section { .. })));
        search(&mut app, "reff");
        assert!(matches!(first(&app), Some(Hit::Section { tag, .. }) if tag.kind == SectionKind::Chorus));
        app.pick(Pick::ShowNow);
        let live = app.live.as_ref().map(|live| (live.deck.source.clone(), live.deck.slides[live.index].tag.map(|tag| tag.kind)));
        assert_eq!(live, Some((DeckSource::Song(app.songs[1].id), Some(SectionKind::Chorus))));
    }

    #[test]
    fn escape_closes_the_search_at_once_and_first_leaves_a_song() {
        let mut app = app();
        search(&mut app, "black");
        assert!(app.palette.open);
        app.palette_escape();
        assert!(!app.palette.open, "one press leaves, whatever was typed");
        // From inside a song the first press goes back out to the whole search.
        app.open_palette(Scope::All, "");
        app.palette.chip = Some(0);
        app.palette_escape();
        assert!(app.palette.open && app.palette.chip.is_none());
        app.palette_escape();
        assert!(!app.palette.open);
    }

    #[test]
    fn a_word_that_names_a_command_is_not_read_as_a_book_of_the_bible() {
        let mut app = app();
        search(&mut app, "plan");
        assert!(!matches!(first(&app), Some(Hit::Passage { .. })), "no chapter of a guessed book leads the list");
        let command = app.palette.hits.iter().position(|hit| matches!(hit, Hit::Command(Command::NewPlan))).expect("the plan commands are found");
        let passage = app.palette.hits.iter().position(|hit| matches!(hit, Hit::Passage { .. })).unwrap_or(usize::MAX);
        assert!(command < passage, "the command stands before any passage");
    }
}
