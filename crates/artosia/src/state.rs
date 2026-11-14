//! Everything the operator window knows, and what each action does to it.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use artosia_core::bible::{Bible, Reference};
use artosia_core::deck::{Announcement, Deck, DeckSource, Fill, Person, Slide, Today, announcement_pages, passage_deck, tag_group};
use artosia_core::plan::{Plan, PlanEntry, PlanItem};
use artosia_core::search::SongIndex;
use artosia_core::song::Song;
use artosia_core::store::Store;
use artosia_remote::{RemoteCommand, RemoteServer};
use artosia_slides::frame::{Countdown, OutputState, StageChip, StageInfo};
use artosia_slides::look::Look;
use rune::{Image, ImageCache, TextEdit, Waker};
use serde::{Deserialize, Serialize};

use crate::data;
use crate::learning::Learning;
use crate::link::Link;
use crate::palette::Palette;
use crate::strings as t;

/// The deck in the Preview pane: what Enter shows, and what Next reaches when the live deck ends.
pub struct OpenDeck {
    pub deck: Deck,
    /// The plan item it was opened from, if any.
    pub item: Option<u64>,
    /// The slide Enter will show: the selector, drawn as a ring.
    pub cursor: usize,
    /// Artosia put this deck here as what comes next. A deck a person opened is never replaced.
    pub queued: bool,
    /// The selector was moved by hand, so it stays put while the live slide moves.
    pub armed: bool,
}

impl OpenDeck {
    /// A deck a person opened.
    pub fn new(deck: Deck, item: Option<u64>, cursor: usize) -> OpenDeck {
        OpenDeck { deck, item, cursor, queued: false, armed: false }
    }
}

/// A plan row on its way to a new place under the pointer.
/// How the tempo read from the sound is counted. A slow song's eighth notes look exactly like a
/// faster song's beats to the tracker (`crates/artosia-listen/README.md`, "Measured on real
/// music"), so a person can say which it is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TempoCount {
    Half,
    #[default]
    AsHeard,
    Double,
}

impl TempoCount {
    /// What the reading is multiplied by.
    pub fn times(self) -> f32 {
        match self {
            TempoCount::Half => 0.5,
            TempoCount::AsHeard => 1.0,
            TempoCount::Double => 2.0,
        }
    }
}

/// A tempo report waiting to be taken, with when it arrived (see `App::listen_tempo_slot`).
pub type TempoSlot = Arc<Mutex<Option<(Option<artosia_listen::Tempo>, std::time::Instant)>>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanDrag {
    pub id: u64,
    pub from: usize,
    pub to: usize,
    /// How far the pointer has carried the row.
    pub dy: f32,
}

/// The deck on the projector.
pub struct LiveDeck {
    pub deck: Deck,
    pub item: Option<u64>,
    pub index: usize,
}

/// Whether Artosia's own window is light or dark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Appearance {
    /// As the computer is set.
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub look: Look,
    pub verses_per_slide: usize,
    /// The church's CCLI licence number, printed with song credits.
    pub licence: String,
    /// A picture shown alone by the Logo button.
    pub logo: String,
    pub bible: String,
    pub second_bible: String,
    pub plan: i64,
    /// The first-run card has been dismissed.
    pub welcomed: bool,
    /// The first-start tour has been finished or skipped.
    pub toured: bool,
    /// The first run was begun and not finished or skipped: it opens again at the next start.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub first_run_open: bool,
    /// The country or region of the church as two capital letters, asked once by the first run;
    /// empty for a church that was never asked. It only orders the Bibles on offer.
    pub country: String,
    pub remote_on: bool,
    /// The screen the projector uses; None lets the output process choose.
    pub audience_screen: Option<usize>,
    pub stage_screen: Option<usize>,
    pub stage_window: bool,
    /// With one screen only: the projector is showing on it.
    pub projector_here: bool,
    /// The projector is switched off.
    pub projector_off: bool,
    pub appearance: Appearance,
    /// The interface language's code (`en`, `id`, ...); empty in a file saved before it was one.
    pub language: String,
    /// What older files said instead: Indonesian or not. Read, never written.
    #[serde(rename = "indonesian", skip_serializing)]
    pub legacy_indonesian: bool,
    /// The microphone Artosia listens on; empty for the computer's own choice.
    pub listen_device: String,
    /// The language it listens for: "id" or "en".
    pub listen_language: String,
    /// The red Live marker pulses gently on the beat the listener hears. Off by default.
    pub beat_blink: bool,
    /// Heard phrases and what Artosia made of them are kept in a folder. Off until a person
    /// switches it on: a service holds people's voices.
    pub keep_recordings: bool,
    /// The folder a person picked for them; empty keeps them beside the library.
    pub recordings_folder: String,
    /// This PC read the first speech model quickly enough to be offered the better one.
    pub listen_quick: bool,
    /// The better speech model fell behind on this PC, so the first one is used.
    pub listen_better_behind: bool,
    /// The streaming service's address, and the key it gave this church. Kept on this computer only.
    pub stream_address: String,
    pub stream_key: String,
    /// Keep a recording of the service in the Videos folder.
    pub stream_record: bool,
    /// Corner pin and colour for the projector picture.
    pub warp: artosia_slides::frame::Warp,
    /// The search learns from what is picked and sung, and orders equal matches by it.
    pub learn: bool,
    /// A letter jumps to the section it names in the live song (C the chorus, V a verse).
    /// Off, every letter starts a search. WCAG 2.1.4 asks that single-letter keys can be
    /// turned off.
    #[serde(default = "yes")]
    pub letter_keys: bool,
    /// The languages the church usually shows together on a slide, in the order they are drawn
    /// (BCP 47 codes). A plan may have its own list; empty shows each song's first language.
    pub song_languages: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            look: Look::default(),
            verses_per_slide: 1,
            licence: String::new(),
            logo: String::new(),
            bible: "KJV".to_string(),
            second_bible: String::new(),
            plan: 0,
            welcomed: false,
            toured: false,
            first_run_open: false,
            country: String::new(),
            remote_on: false,
            audience_screen: None,
            stage_screen: None,
            stage_window: false,
            projector_here: false,
            projector_off: false,
            appearance: Appearance::Auto,
            language: String::new(),
            legacy_indonesian: false,
            listen_device: String::new(),
            listen_language: "id".to_string(),
            beat_blink: false,
            keep_recordings: false,
            recordings_folder: String::new(),
            listen_quick: false,
            listen_better_behind: false,
            stream_address: String::new(),
            stream_key: String::new(),
            stream_record: false,
            warp: artosia_slides::frame::Warp::default(),
            learn: true,
            letter_keys: true,
            song_languages: Vec::new(),
        }
    }
}

/// A translation Artosia can load.
#[derive(Clone, Debug, PartialEq)]
pub struct BibleFile {
    /// The module's code, its file name in capitals: what settings and plans keep.
    pub code: String,
    /// What people call it, drawn in the picker and under a verse: "TB". The code when nothing
    /// says another (`App::list_bibles`).
    pub short: String,
    pub name: String,
    /// None for the translation packed into the program.
    pub path: Option<PathBuf>,
}

fn yes() -> bool {
    true
}

/// The key that names a section's kind: V, P, C, B, I, E, and X for any other.
/// The letter drawn beside a section in the lists: the language's own letter for a chorus,
/// V beside a verse only where the word for a verse begins with it, the usual letter otherwise.
pub fn shown_letter(tag: artosia_core::song::Tag) -> Option<char> {
    use artosia_core::song::SectionKind;
    match tag.kind {
        SectionKind::Chorus => Some(t::chorus_key()),
        SectionKind::Verse => t::verse_key_shown().then_some('V'),
        _ => Some(marker_letter(tag)),
    }
}

pub fn marker_letter(tag: artosia_core::song::Tag) -> char {
    tag.short().chars().next().unwrap_or('X')
}

pub enum Undo {
    PlanItem { at: usize, item: PlanItem },
    Song(Box<Song>),
    Announcement(Announcement),
    /// An editor closed with unsaved changes.
    Editor(Box<Editor>),
    /// The search history was forgotten.
    History(Box<artosia_core::learn::History>),
    /// A whole plan was deleted.
    Plan(Box<Plan>),
    /// A plan went back to the usual look; this was its own, and `plan` is the plan it belonged to.
    PlanLook { plan: i64, look: serde_json::Value },
    /// A plan went back to the church's usual languages; this was its own list.
    PlanLanguages { plan: i64, list: Vec<String> },
    /// A language's words were taken out of the song in the editor, from this place in its list.
    EditorLanguage { at: usize, entry: (String, String) },
    /// Every recording was deleted; they wait beside the folder until the toast leaves.
    Recordings,
}

pub struct Toast {
    pub text: String,
    pub undo: Option<Undo>,
    /// App clock seconds at which it leaves.
    pub until: f64,
}

/// A whole song offered under the title of a new one, as an editor offers a completion.
#[derive(Clone, Debug, PartialEq)]
pub enum Suggestion {
    /// The library already has it, at this place in `songs`.
    Library(usize),
    /// Found on the web.
    Web(artosia_lyrics::Found),
}

#[derive(Default)]
pub struct Editor {
    pub id: i64,
    pub title: TextEdit,
    pub author: TextEdit,
    pub copyright: TextEdit,
    pub ccli: TextEdit,
    pub key: TextEdit,
    pub order: TextEdit,
    /// The words of the language that shows in the field (`at`); the others wait in `parked`.
    pub lyrics: TextEdit,
    /// The code of each language the song has, the first language first; empty for one nobody named.
    pub codes: Vec<String>,
    /// The words of each language as they were when it last left the field. The one `at` is stale.
    pub parked: Vec<String>,
    /// The language in the field.
    pub at: usize,
    /// The chip whose menu is open.
    pub menu: Option<usize>,
    pub suggestions: Vec<Suggestion>,
    /// The suggestion Tab or Enter takes.
    pub suggestion: usize,
    /// An arrow key has chosen one of the offered songs, so Enter takes it.
    pub suggestion_armed: bool,
    /// The title the web was last asked about.
    pub asked: String,
    /// When to ask the web about the title as it now stands, on the app clock.
    pub ask_at: Option<f64>,
    /// A song's words are on their way from the web.
    pub fetching: bool,
    /// Where filled-in words came from: the source's name and the page.
    pub credit: Option<(String, String)>,
    /// The strip proposing section marks was dismissed for this song.
    pub marks_hidden: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    None,
    /// What the projector draws: background, words, logo.
    Looks,
    /// The program's own settings.
    Settings,
    Screens,
    Import,
    Remote,
    Keys,
    Listen,
    Stream,
    Bibles,
    /// Which languages a slide shows together; from a plan's menu it edits the plan's own list.
    Languages,
    /// The backdrop browser, until the Look dialog holds it as its Background panel.
    Backdrops,
}

/// Something the room was heard to say or sing, offered for one click.
#[derive(Clone, Debug, PartialEq)]
pub enum HeardHit {
    Song(usize),
    Passage(Reference, String),
}

/// The announcement editor: the one being written, and whether it was changed.
#[derive(Default)]
pub struct Notices {
    /// 0 for an announcement not saved yet.
    pub id: i64,
    pub title: TextEdit,
    pub body: TextEdit,
    pub seconds: u32,
    /// A picture shown with the words; empty for none.
    pub image: String,
    pub fill: Fill,
    /// The reference of a passage card, as typed.
    pub passage: TextEdit,
}

/// Results a worker thread hands back to the window.
pub enum Arrived {
    Bible(String, Result<Bible, String>),
    /// Songs the web found for a title.
    Lyrics(String, Result<Vec<artosia_lyrics::Found>, String>),
    /// The words of one of them.
    LyricsText(Result<artosia_lyrics::Fetched, String>),
    /// Words heard in the room, or why listening cannot work.
    Heard(artosia_listen::Heard),
    /// A check for a newer Artosia: the version now waiting, none, or why it failed.
    Update(Result<Option<String>, String>),
    /// The speech model's download ended.
    Model(Result<(), String>),
    /// The better speech model's download ended.
    BetterModel(Result<(), String>),
    /// The recordings were packed into one file: its name, or that it could not be written.
    RecordingsSaved(Result<String, ()>),
    /// The firewall step of one press of the Phone switch (the number of the press) and where it got to.
    Firewall(u64, crate::firewall::Progress),
}

pub struct App {
    pub folder: Option<PathBuf>,
    pub store: Store,
    /// Why the library could not be opened on disk, when it could not.
    pub store_problem: Option<String>,
    pub songs: Vec<Song>,
    pub index: SongIndex,
    pub bible_files: Vec<BibleFile>,
    pub bibles: HashMap<String, Arc<Bible>>,
    /// Codes being read by a worker.
    pub bibles_loading: Vec<String>,
    /// The Bibles dialog: files being converted and eBible's list.
    pub desk: crate::bibles::Desk,
    /// The backdrop browser: ready pictures and videos to put behind the words.
    pub backdrops: crate::backdrops::Desk,
    pub announcements: Vec<Announcement>,
    /// Everyone whose birthday the church keeps, for announcements that fill themselves.
    pub people: Vec<Person>,
    pub plan: Plan,
    pub settings: Settings,

    pub open: Option<OpenDeck>,
    pub live: Option<LiveDeck>,
    pub black: bool,
    pub text_hidden: bool,
    pub logo_on: bool,
    pub message: TextEdit,
    pub message_on: bool,
    pub stage_message: TextEdit,
    pub stage_message_on: bool,
    pub timer_minutes: TextEdit,
    pub countdown: Option<Countdown>,

    pub palette: Palette,
    /// What the search learns from use (`learning.rs`).
    pub learn: Learning,
    pub editor: Option<Editor>,
    /// Where an editor field writes once the editor has closed under it.
    pub spare_edit: TextEdit,
    pub sheet: Sheet,
    /// The song being written was started from the plan's Add menu, so saving it adds it.
    pub plan_new_song: bool,
    /// The part of the Design dialog that shows: 0 background, 1 words, 2 logo.
    pub design_part: usize,
    /// The tour's stop, while the tour is open (`ui/tour.rs`).
    pub tour: Option<usize>,
    /// The language drop-down of Settings is open.
    pub language_open: bool,
    /// One language is showing alone, chosen from the Live pane or the search. Never saved.
    pub only_language: Option<String>,
    /// Languages switched off in the dialog stay in their rows until it closes.
    pub languages_off: Vec<String>,
    /// The languages dialog was opened from a plan, so it holds the two tiles.
    pub languages_from_plan: bool,
    /// The list of languages to pick one from, while it is open.
    pub language_pick: Option<crate::song_languages::Pick>,
    /// The shape of the projector's screen, in pixels, once it is known: the slides split for it.
    pub projector_shape: Option<(u32, u32)>,
    /// The first run, while it is open (`onboard.rs`, `ui/onboard.rs`).
    pub onboard: Option<crate::onboard::Onboard>,
    /// This start is a new church's: the first run opens by itself.
    pub first_run: bool,
    /// The clock when Enter last pressed the first run's primary button, as `tour_entered_at` does.
    pub onboard_entered_at: f64,
    /// The first run's work that outlives one press.
    pub onboard_work: crate::ui::onboard::Work,
    /// The tour has just opened and its Next button should take keyboard focus.
    pub tour_focus: bool,
    /// The clock when Enter last moved the tour. If Rune also presses the focused button for
    /// the same Enter, that press is the same step and is dropped.
    pub tour_entered_at: f64,
    pub notices: Option<Notices>,
    /// The announcements are advancing by themselves on the projector.
    pub looping: bool,
    /// The last plan item that was live. An item shown from the search has no place in the
    /// plan, and Next goes on from here once it ends.
    pub resume: Option<u64>,
    /// A slide of the live deck picked while Shift is held; it shows when Shift is let go.
    pub live_pick: Option<usize>,
    /// Shift moved the Preview ring while nothing was live; letting go shows that slide.
    pub preview_pick: bool,
    pub plan_drag: Option<PlanDrag>,
    /// A plan row was just dropped, so the press that ends the same gesture opens nothing.
    pub plan_dropped: bool,
    /// Ctrl or Shift is down. The arrows mean something else with each, and the hint bar and
    /// the panes say which while it is held.
    pub held_ctrl: bool,
    pub held_shift: bool,
    /// What was on the projector before, newest last, so a volunteer who went too far can
    /// see where they were and bring it back into Preview.
    /// Whether the speech model is on this PC (`speech_model.rs`).
    pub model: crate::speech_model::Model,
    /// The better speech model's row in the Listen dialog.
    pub better: crate::speech_model::Better,
    /// How many phrases were read since the listener started, and the share of their own
    /// length the readings took, newer ones weighing more.
    pub read_speed: (u32, f32),
    pub history: Vec<LiveDeck>,
    pub history_menu: bool,
    /// The Live switch is off: the Live list keeps what it holds and the projector shows none
    /// of it. Every start begins on, so a switch left off last week is not a trap this week.
    pub off_air: bool,
    /// The plan's Add menu is open.
    pub add_menu: bool,
    /// The slide the loop is waiting on, and when it moves on, on the app clock.
    pub loop_at: (usize, f64),
    /// How many slide tiles fit in a row, for the arrow keys. The view writes it.
    /// Streaming or recording while this exists.
    pub stream: Option<crate::stream::Stream>,
    pub stream_address: TextEdit,
    /// The address of a camera on the network, as typed in Design.
    pub camera_address: TextEdit,
    pub stream_key: TextEdit,
    pub stream_problem: Option<String>,
    /// Listening to the room while this exists.
    pub listener: Option<artosia_listen::Listener>,
    /// How loud the room is, 0 to 1, as the bits of an f32 the listening thread writes.
    pub listen_level: Arc<std::sync::atomic::AtomicU32>,
    /// The level meter or the beat is on screen, so the listening thread should wake the window.
    pub meter_shown: Arc<std::sync::atomic::AtomicBool>,
    /// The newest tempo report the listening thread has not handed over yet, with its arrival time.
    /// The outer `Option` is "nothing new"; the inner one is "no steady pulse".
    pub listen_tempo_slot: TempoSlot,
    /// The beat as of the last report, with the app clock at which it was reported.
    pub tempo: Option<(artosia_listen::Tempo, f64)>,
    /// How the reading is counted now, and the tempo a person said the song has by pressing
    /// Half or Double. The tracker flips between a tempo and its double within one song, so the
    /// count follows the reading to keep that tempo, and ends when no count reaches it: that is
    /// another song.
    pub tempo_count: TempoCount,
    pub tempo_wanted: Option<f32>,
    /// What is kept of what the listener hears, when a person switched that on.
    pub recordings: crate::recordings::Recordings,
    /// The Listen dialog shows its second part, the recordings.
    pub listen_part: usize,
    /// The app clock of the last beat reported and whether it is an off beat of the halved
    /// count, carried from report to report so the slower pulse does not jump.
    pub tempo_half: Option<(f64, bool)>,
    /// A headless scene shows the Listen dialog as it looks while listening.
    pub demo_listening: bool,
    /// The system asks for reduced motion; the view writes it, since only a view can read it.
    pub motion_reduced: std::sync::atomic::AtomicBool,
    pub listen_problem: Option<String>,
    /// The last few things heard, newest last.
    pub heard: Vec<String>,
    pub heard_hits: Vec<HeardHit>,
    /// What the listener's phrases matched, for the suggestion strip (`suggest.rs`).
    pub hearing: crate::suggest::Hearing,
    pub microphones: Vec<artosia_listen::Device>,
    /// The corner of the projector picture the nudge buttons move: 0 top left, clockwise.
    pub warp_corner: usize,
    /// A headless scene is showing the Screens dialog as it looks with a projector plugged in.
    pub demo_screens: bool,
    /// This version has told the updater it started well.
    pub confirmed: bool,
    pub licence_edit: TextEdit,
    /// The Look dialog's hex fields and font picker.
    pub look_edit: crate::ui::look::LookEdit,
    pub toast: Option<Toast>,
    pub plan_menu: Option<u64>,
    pub more_menu: bool,
    /// The list of plans, the rename dialog, and where Next starts after a switch (`plans.rs`).
    pub switcher: crate::plans::Switcher,
    pub easislides: Option<PathBuf>,

    pub link: Link,
    pub remote: Option<RemoteServer>,
    pub remote_pin: String,
    pub remote_problem: Option<String>,
    /// Where the firewall rule for the phone remote stands, and the press it answers: a late
    /// answer for an earlier press is dropped.
    pub firewall: crate::firewall::Standing,
    pub remote_round: u64,
    /// What the phone last searched for, and what was found.
    pub remote_hits: (String, Vec<artosia_remote::RemoteHit>),
    pub remote_qr: Option<rune::Pixels>,
    /// The screens are showing their numbers.
    pub identify: bool,
    /// A handler asked for the clipboard; the next frame requests it.
    pub want_paste: bool,
    /// A handler asked for a folder of songs to import; the next frame opens the dialog.
    pub want_folder: bool,
    /// The search asked for the file picker, for a zip of songs.
    pub want_zip: bool,
    /// A handler asked for a PowerPoint, a birthday list or an announcement picture to choose.
    pub want_pptx: bool,
    /// UIOVER1 mocks: a direction (a, b or c) and a page (1 to 7) drawn instead of the window, for `--scene mock-*` only.
    pub mock: Option<(char, u8)>,
    pub want_birthdays: bool,
    pub want_notice_image: bool,
    pub images: Option<ImageCache>,
    pub picture: Option<(String, Image)>,
    pub logo: Option<(String, Image)>,
    /// The pictures announcements carry, by path, for the previews.
    pub slide_pictures: HashMap<String, Image>,
    pub waker: Option<Waker>,
    pub inbox: Arc<Mutex<Vec<Arrived>>>,
    pub remote_tx: Sender<RemoteCommand>,
    pub remote_rx: Receiver<RemoteCommand>,
    /// Seconds since the Unix epoch, refreshed every frame.
    pub now: f64,
    /// The app clock, for toasts.
    pub clock: f64,
    pub sent: Option<OutputState>,
    pub plan_dirty: bool,
    pub settings_dirty: bool,
}

pub fn unix_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// The first slide of each section of a deck in sung order: where the number keys jump to,
/// and the numbers the lists show. A section split over several slides counts once; a chorus
/// sung twice counts twice. A deck with no sections, such as a passage, counts every slide.
pub fn section_starts(deck: &Deck) -> Vec<usize> {
    if deck.slides.iter().all(|slide| slide.tag.is_none()) {
        return (0..deck.slides.len()).collect();
    }
    let mut starts = Vec::new();
    let mut last = None;
    for (index, slide) in deck.slides.iter().enumerate() {
        if slide.tag.is_some() && slide.tag != last {
            starts.push(index);
        }
        last = slide.tag;
    }
    starts
}

impl App {
    pub fn load() -> App {
        App::load_in(data::folder())
    }

    /// The practice library in memory, as a headless run and the tests use it.
    #[cfg(test)]
    pub fn practice() -> App {
        App::load_in(None)
    }

    fn load_in(folder: Option<PathBuf>) -> App {
        let asked = data::flag_value("--language").filter(|code| crate::languages::index_of(code).is_some()).or_else(|| data::has_flag("--indonesian").then(|| "id".to_string()));
        t::set_language(asked.as_deref().unwrap_or("en"));
        let (store, store_problem) = data::open_store(folder.as_ref());
        let practice = folder.is_none();
        if practice {
            for mut song in data::practice_songs() {
                let _ = store.save_song(&mut song);
            }
            for mut item in data::practice_announcements() {
                let _ = store.save_announcement(&mut item);
            }
        }
        let songs = store.songs().unwrap_or_default();
        let index = SongIndex::build(&songs);
        // A library that never saved settings is a new church's: the first run opens (`onboard.rs`).
        let stored: Option<Settings> = store.setting("settings");
        let first_run = crate::onboard::opens_at_start(folder.is_some(), stored.is_some(), stored.as_ref().is_some_and(|settings| settings.first_run_open), rune::is_headless());
        let mut settings: Settings = stored.unwrap_or_default();
        // The practice library's song in three languages shows all three, so the feature can be seen without setting anything up.
        if practice {
            settings.song_languages = vec!["id".to_string(), "en".to_string(), "zh-Hans".to_string()];
        }
        let announcements = store.announcements().unwrap_or_default();
        let plans = store.plans().unwrap_or_default();
        let plan = plans
            .iter()
            .find(|plan| plan.id == settings.plan)
            .or(plans.first())
            .cloned()
            .unwrap_or_else(|| Plan::named(""));
        let (remote_tx, remote_rx) = channel();
        let licence_edit = TextEdit::new(settings.licence.clone());
        let first_look = plan.look.clone().and_then(|own| serde_json::from_value(own).ok()).unwrap_or_else(|| settings.look.clone());
        let look_edit = crate::ui::look::LookEdit::new(&first_look);
        let mut app = App {
            folder,
            store,
            store_problem,
            songs,
            index,
            bible_files: Vec::new(),
            bibles: HashMap::new(),
            bibles_loading: Vec::new(),
            desk: crate::bibles::Desk::default(),
            backdrops: crate::backdrops::Desk::default(),
            announcements,
            people: Vec::new(),
            plan,
            settings,
            open: None,
            live: None,
            black: false,
            text_hidden: false,
            logo_on: false,
            message: TextEdit::new(""),
            message_on: false,
            stage_message: TextEdit::new(""),
            stage_message_on: false,
            timer_minutes: TextEdit::new("5"),
            countdown: None,
            palette: Palette::default(),
            learn: Learning::default(),
            editor: None,
            spare_edit: TextEdit::new(""),
            sheet: Sheet::None,
            plan_new_song: false,
            design_part: 0,
            tour: None,
            onboard: None,
            language_open: false,
            only_language: None,
            languages_off: Vec::new(),
            languages_from_plan: false,
            language_pick: None,
            projector_shape: None,
            first_run,
            onboard_entered_at: -1.0,
            onboard_work: Default::default(),
            tour_entered_at: -1.0,
            tour_focus: false,
            notices: None,
            looping: false,
            resume: None,
            live_pick: None,
            preview_pick: false,
            plan_drag: None,
            plan_dropped: false,
            held_ctrl: false,
            held_shift: false,
            model: crate::speech_model::Model::Unknown,
            better: crate::speech_model::Better::No,
            read_speed: (0, 0.0),
            history: Vec::new(),
            history_menu: false,
            off_air: false,
            add_menu: false,
            loop_at: (0, 0.0),
            stream: None,
            stream_address: TextEdit::new(""),
            camera_address: TextEdit::new(""),
            stream_key: TextEdit::new("").secure(),
            stream_problem: None,
            listener: None,
            listen_level: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            meter_shown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            listen_tempo_slot: Arc::new(Mutex::new(None)),
            tempo_count: TempoCount::AsHeard,
            tempo_wanted: None,
            recordings: crate::recordings::Recordings::at_start(),
            listen_part: 0,
            tempo_half: None,
            tempo: None,
            demo_listening: false,
            motion_reduced: std::sync::atomic::AtomicBool::new(false),
            listen_problem: None,
            heard: Vec::new(),
            heard_hits: Vec::new(),
            hearing: Default::default(),
            microphones: Vec::new(),
            warp_corner: 0,
            demo_screens: false,
            confirmed: false,
            licence_edit,
            look_edit,
            toast: None,
            plan_menu: None,
            more_menu: false,
            switcher: Default::default(),
            easislides: artosia_core::import::easislides_folder(),
            link: Link::default(),
            remote: None,
            remote_pin: String::new(),
            remote_problem: None,
            firewall: Default::default(),
            remote_round: 0,
            remote_hits: (String::new(), Vec::new()),
            remote_qr: None,
            identify: false,
            want_paste: false,
            want_folder: false,
            want_zip: false,
            want_pptx: false,
            mock: None,
            want_birthdays: false,
            want_notice_image: false,
            images: None,
            picture: None,
            logo: None,
            slide_pictures: HashMap::new(),
            waker: None,
            inbox: Arc::new(Mutex::new(Vec::new())),
            remote_tx,
            remote_rx,
            now: unix_now(),
            clock: 0.0,
            sent: None,
            plan_dirty: false,
            settings_dirty: false,
        };
        app.settings.language = asked.unwrap_or_else(|| crate::languages::from_settings(&app.settings.language, app.settings.legacy_indonesian).to_string());
        t::set_language(&app.settings.language);
        app.people = app.store.people().unwrap_or_default();
        app.stream_address.replace_text(app.settings.stream_address.clone());
        app.stream_key.replace_text(app.settings.stream_key.clone());
        app.sync_camera_address();
        app.list_bibles();
        app.desk.set_country(&app.settings.country);
        app.start_learning();
        if practice {
            app.seed_practice_plan();
        }
        // The plans are listed under the plan from the first frame, and a real start opens the
        // plan of the service at hand.
        app.refresh_plan_rows();
        if !rune::is_headless() && !practice {
            app.open_plan_for_now();
        }
        if data::has_flag("--import-easislides") {
            app.import_easislides();
            app.toast = None;
        }
        if let Some(folder) = data::flag_value("--import-openlyrics") {
            // The toast stays: it is the only report of what the import did.
            app.import_openlyrics(std::path::Path::new(&folder));
        }
        // `--import-pptx <file>` and `--import-birthdays <file>` bring a church's own files in at start.
        if let Some(file) = data::flag_value("--import-pptx") {
            app.import_pptx(std::path::Path::new(&file));
        }
        if let Some(file) = data::flag_value("--import-birthdays") {
            app.import_birthdays(std::path::Path::new(&file));
        }
        // A headless run has no later frame to wait for, so it reads the Bible before drawing.
        if rune::is_headless() {
            let code = app.settings.bible.clone();
            let file = app.bible_files.iter().find(|file| file.code == code).cloned();
            let loaded = match file.as_ref().and_then(|file| file.path.as_ref()) {
                Some(path) => Bible::open(path).ok().map(|mut bible| {
                    if let Some(file) = &file {
                        bible.short = file.short.clone();
                    }
                    bible
                }),
                None => artosia_core::import::bundled_bible().ok(),
            };
            if let Some(bible) = loaded {
                app.bibles.insert(code, Arc::new(bible));
            }
        }
        // `--background <file>` sets a picture or video behind the text, for checking it by eye.
        if let Some(file) = data::flag_value("--background") {
            app.set_background_file(std::path::Path::new(&file));
            app.toast = None;
        }
        // `--light` and `--dark` set the window's theme for this run, for checking both by eye.
        if data::has_flag("--light") {
            app.settings.appearance = Appearance::Light;
        } else if data::has_flag("--dark") {
            app.settings.appearance = Appearance::Dark;
        }
        // `--query <text>` opens the search with that text, for checking a search by eye.
        if let Some(query) = data::flag_value("--query") {
            app.open_palette(crate::palette::Scope::All, &query);
        }
        app
    }

    fn seed_practice_plan(&mut self) {
        if !self.plan.items.is_empty() {
            return;
        }
        self.plan.push(PlanEntry::Announcements);
        self.plan.push(PlanEntry::Heading { title: t::PLAN_HEADING_WORSHIP().to_string() });
        for song in self.songs.iter().take(3) {
            self.plan.push(PlanEntry::song(song.id, song.title.clone()));
        }
        self.plan.push(PlanEntry::Heading { title: t::PLAN_HEADING_WORD().to_string() });
        let reference = Reference { book: 43, chapter: 3, verse: 16, end_chapter: 3, end_verse: 17 };
        self.plan.push(PlanEntry::Passage { reference, bible: "KJV".into(), second: None, title: "John 3:16-17".into() });
    }

    /// Lists the translations on disk beside the one packed in.
    pub fn list_bibles(&mut self) {
        let mut files = vec![BibleFile { code: "KJV".into(), short: "KJV".into(), name: "King James Version".into(), path: None }];
        if let Some(folder) = &self.folder {
            // eBible's saved list is read once, and only for a module that does not say its own code.
            let mut saved: Option<Vec<(String, String)>> = None;
            for path in artosia_core::import::bible_files(&folder.join("bibles")) {
                if let Ok((code, name, _)) = artosia_core::bible::describe(&path)
                    && !files.iter().any(|file| file.code == code)
                {
                    let mut short = artosia_core::bible::short_code(&path);
                    if short.is_empty() {
                        let saved = saved.get_or_insert_with(|| crate::bibles::saved_short_codes(folder));
                        short = saved.iter().find(|(module, _)| *module == code).map(|(_, short)| short.clone()).unwrap_or_default();
                    }
                    if short.is_empty() {
                        short = code.clone();
                    }
                    files.push(BibleFile { code, short, name, path: Some(path) });
                }
            }
        }
        // Two Bibles that would be called the same are told apart by their own codes.
        let shorts: Vec<String> = files.iter().map(|file| file.short.to_uppercase()).collect();
        for (at, file) in files.iter_mut().enumerate() {
            if shorts.iter().enumerate().any(|(other, short)| other != at && *short == shorts[at]) {
                file.short = file.code.clone();
            }
        }
        self.bible_files = files;
        if !self.bible_files.iter().any(|file| file.code == self.settings.bible) {
            self.settings.bible = "KJV".into();
        }
        if !self.bible_files.iter().any(|file| file.code == self.settings.second_bible) {
            self.settings.second_bible.clear();
        }
    }

    /// Starts reading a translation on a worker unless it is loaded or on its way.
    pub fn want_bible(&mut self, code: &str) {
        if code.is_empty() || self.bibles.contains_key(code) || self.bibles_loading.iter().any(|c| c == code) {
            return;
        }
        let Some(file) = self.bible_files.iter().find(|file| file.code == code).cloned() else { return };
        self.bibles_loading.push(code.to_string());
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let result = match &file.path {
                // The caption under a verse calls it what the picker does.
                Some(path) => Bible::open(path).map(|mut bible| {
                    bible.short = file.short.clone();
                    bible
                }),
                None => artosia_core::import::bundled_bible(),
            };
            let result = result.map_err(|error| error.to_string());
            if let Ok(bible) = &result {
                bible.warm();
            }
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Bible(file.code.clone(), result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    pub fn bible(&self) -> Option<&Arc<Bible>> {
        self.bibles.get(&self.settings.bible)
    }

    pub fn second_bible(&self) -> Option<&Arc<Bible>> {
        if self.settings.second_bible.is_empty() { None } else { self.bibles.get(&self.settings.second_bible) }
    }

    pub fn bible_is_loading(&self) -> bool {
        self.bible().is_none() && self.bibles_loading.contains(&self.settings.bible)
    }

    pub fn song_position(&self, id: i64) -> Option<usize> {
        self.songs.iter().position(|song| song.id == id)
    }

    pub fn deck_of_song(&self, song: &Song) -> Deck {
        let mut deck = artosia_slides::voices::song_deck(song, self.look().max_lines, &self.settings.licence, &self.song_languages(), &self.look(), self.slide_shape());
        // The library names sections in English; a volunteer reads them in the window's language.
        for slide in &mut deck.slides {
            if let Some(tag) = slide.tag {
                slide.label = t::section_label(tag);
            }
        }
        deck
    }

    /// The plan's name as its column shows it. A plan nobody renamed reads in the window's language.
    pub fn plan_title(&self) -> String {
        crate::plans::plan_words(&self.plan.name, self.plan.day, self.plan.time)
    }

    /// A plan entry's title as the plan shows it.
    pub fn entry_title(&self, entry: &PlanEntry) -> String {
        match entry {
            PlanEntry::Announcements => t::CMD_ANNOUNCEMENTS().to_string(),
            other => other.title().to_string(),
        }
    }

    pub fn deck_of_passage(&self, reference: &Reference) -> Option<Deck> {
        let bible = self.bible()?;
        Some(passage_deck(bible, reference, self.second_bible().map(Arc::as_ref), self.settings.verses_per_slide))
    }

    /// The deck a plan entry stands for; None for a heading, a deleted song or a Bible still loading.
    pub fn deck_for(&self, entry: &PlanEntry) -> Option<Deck> {
        match entry {
            PlanEntry::Song { song_id, .. } => self.song_position(*song_id).map(|at| self.deck_of_song(&self.songs[at])),
            PlanEntry::Passage { reference, bible, second, .. } => {
                let first = self.bibles.get(bible).or(self.bible())?;
                let other = second.as_ref().and_then(|code| self.bibles.get(code));
                Some(passage_deck(first, reference, other.map(Arc::as_ref), self.settings.verses_per_slide))
            }
            PlanEntry::Announcements => Some(self.announcements_deck()),
            PlanEntry::Heading { .. } => None,
        }
    }

    /// Whether a plan entry can no longer be shown, and so needs a warning in the list.
    pub fn entry_is_missing(&self, entry: &PlanEntry) -> bool {
        matches!(entry, PlanEntry::Song { song_id, .. } if self.song_position(*song_id).is_none())
    }

    fn plan_item_of(&self, source: &DeckSource) -> Option<u64> {
        self.plan.items.iter().find(|item| entry_matches(&item.entry, source)).map(|item| item.id)
    }

    /// A passage to show as an example, in the book names of the Bible in use: "Psalms 23:1"
    /// under a KJV, "Mazmur 23:1" under an Indonesian Bible, and the window's language when
    /// there is no Bible yet.
    pub fn example_passage(&self, verses: &str) -> String {
        let book = self.bible().and_then(|bible| bible.book(19).map(|book| book.name.clone())).unwrap_or_else(|| artosia_core::bible::book_name(19, t::language_code()).to_string());
        format!("{book} {verses}")
    }

    /// Something with a field of its own has just opened: a dialog, the box that names a plan,
    /// an editor. Whoever closes the search then must not drop the keyboard focus.
    pub fn wants_focus(&self) -> bool {
        self.sheet != Sheet::None || self.switcher.editing.is_some() || self.editor.is_some() || self.notices.is_some()
    }

    /// Today's date, for announcements that fill themselves with this week's birthdays.
    pub fn today(&self) -> Today<'static> {
        let time = rune::LocalTime::at((self.now * 1000.0) as i64);
        Today { month: time.month.clamp(1, 12) as u8, day: time.day.clamp(1, 31) as u8, months: t::MONTHS(), date: t::day_month }
    }

    /// Every announcement slide in order, each with the place of its announcement in the list.
    pub fn announcement_pages(&self) -> Vec<(usize, Slide)> {
        announcement_pages(&self.announcements, &self.people, &self.today(), self.bible().map(|bible| &**bible), self.settings.verses_per_slide)
    }

    pub fn announcements_deck(&self) -> Deck {
        let mut deck = artosia_core::deck::announcements_deck(&self.announcements, &self.people, &self.today(), self.bible().map(|bible| &**bible), self.settings.verses_per_slide);
        deck.title = t::CMD_ANNOUNCEMENTS().to_string();
        deck
    }

    /// The card a slide of the announcements came from. A card may give several slides or none.
    pub fn card_at_slide(&self, slide: usize) -> Option<&Announcement> {
        let pages = announcement_pages(&self.announcements, &self.people, &self.today(), self.bible().map(|bible| &**bible), self.settings.verses_per_slide);
        pages.get(slide).and_then(|(at, _)| self.announcements.get(*at))
    }

    /// The slides one card gives today: none for a birthday card with nobody, or a passage card
    /// the Bible cannot read.
    pub fn card_slides(&self, card: &Announcement) -> Vec<Slide> {
        announcement_pages(std::slice::from_ref(card), &self.people, &self.today(), self.bible().map(|bible| &**bible), self.settings.verses_per_slide).into_iter().map(|(_, slide)| slide).collect()
    }

    /// One saved card as a deck of its own, so it can be shown without the rest; None when it has
    /// nothing to show today.
    pub fn card_deck(&self, id: i64) -> Option<Deck> {
        let card = self.announcements.iter().find(|card| card.id == id)?;
        let slides = self.card_slides(card);
        if slides.is_empty() {
            return None;
        }
        let title = if card.title.is_empty() { slides[0].label.clone() } else { card.title.clone() };
        Some(Deck { title, subtitle: String::new(), source: DeckSource::Announcement(id), slides })
    }

    /// The deck the number keys act on: what is live, else what is open.
    pub fn target_deck(&self) -> Option<&Deck> {
        self.live.as_ref().map(|live| &live.deck).or(self.open.as_ref().map(|open| &open.deck))
    }

    /// Shows the nth section of the live song, counted from 0 in sung order. With nothing
    /// live it starts the open song there.
    pub fn jump_to_section(&mut self, nth: usize) {
        let Some(slide) = self.target_deck().and_then(|deck| section_starts(deck).get(nth).copied()) else { return };
        // With nothing live the ring moves: only Show, Enter and Next start the projector.
        if self.live.is_some() { self.go_to(slide) } else { self.select_at(slide) }
    }

    /// Ctrl and a digit: the ring in Preview goes to that section; the projector stays.
    pub fn select_section(&mut self, nth: usize) {
        let Some(slide) = self.open.as_ref().and_then(|open| section_starts(&open.deck).get(nth).copied()) else { return };
        self.select_at(slide);
    }

    /// The letters that jump somewhere in the deck the keys act on.
    pub fn marker_letters(&self) -> Vec<char> {
        let Some(deck) = self.target_deck() else { return Vec::new() };
        let mut letters: Vec<char> = deck.slides.iter().filter_map(|slide| slide.tag).map(marker_letter).collect();
        // R is the chorus too: a church that says "reff" reaches for R, and no kind of section owns that letter.
        if letters.contains(&'C') {
            letters.push('R');
            letters.push(t::chorus_key());
        }
        letters.sort_unstable();
        letters.dedup();
        letters
    }

    /// A letter key: the next section of that kind after the live slide, going round to the
    /// first (owner, 2026-11-11: "C stands for chorus, and a keyboard shortcut for every named
    /// marker"). False when the deck has no such section.
    pub fn jump_to_marker(&mut self, letter: char) -> bool {
        let Some(deck) = self.target_deck() else { return false };
        let letter = if letter == 'R' || letter == t::chorus_key() { 'C' } else { letter };
        let from = self.live.as_ref().map(|live| live.index).or(self.open.as_ref().map(|open| open.cursor)).unwrap_or(0);
        let starts: Vec<usize> = section_starts(deck).into_iter().filter(|&at| deck.slides[at].tag.is_some_and(|tag| marker_letter(tag) == letter)).collect();
        let Some(&to) = starts.iter().find(|&&at| at > from).or(starts.first()) else { return false };
        if self.live.is_some() { self.go_to(to) } else { self.select_at(to) }
        true
    }

    /// Keeps what is leaving the projector, when another item takes its place.
    fn remember_live(&mut self, next: &DeckSource) {
        let Some(live) = &self.live else { return };
        if &live.deck.source == next {
            return;
        }
        let kept = LiveDeck { deck: live.deck.clone(), item: live.item, index: live.index };
        self.history.retain(|shown| shown.deck.source != kept.deck.source);
        self.history.push(kept);
        let extra = self.history.len().saturating_sub(8);
        self.history.drain(..extra);
    }

    /// Brings something shown earlier back into Preview, with the ring where it was left.
    pub fn open_shown(&mut self, at: usize) {
        let Some(shown) = self.history.get(at) else { return };
        let (deck, item, index) = (shown.deck.clone(), shown.item, shown.index);
        self.open_deck(deck, item, index);
        if let Some(open) = self.open.as_mut() {
            open.cursor = index.min(open.deck.slides.len().saturating_sub(1));
            open.armed = true;
        }
        self.history_menu = false;
    }

    /// Backspace, the key that goes back in Windows' own file windows: what was on the projector
    /// before this comes into Preview with the ring where it was left. Pressed again it is the
    /// one before that, and after the oldest the newest again. False with nothing shown before.
    pub fn open_earlier(&mut self) -> bool {
        let Some(newest) = self.history.len().checked_sub(1) else { return false };
        let held = self.open.as_ref().and_then(|open| self.history.iter().position(|shown| shown.deck.source == open.deck.source));
        self.open_shown(match held {
            Some(at) if at > 0 => at - 1,
            _ => newest,
        });
        true
    }

    /// Moves the selector in the Preview pane and leaves the projector alone. Past either end
    /// of the deck it goes on into the plan item before or after. True when it moved.
    pub fn select(&mut self, by: isize) -> bool {
        let Some(open) = self.open.as_mut() else {
            self.walk_plan(by > 0);
            return self.open.is_some();
        };
        let last = open.deck.slides.len().saturating_sub(1) as isize;
        let wanted = open.cursor as isize + by;
        if (0..=last).contains(&wanted) {
            open.cursor = wanted as usize;
            open.armed = true;
            open.queued = false;
            return true;
        }
        let was = open.item;
        self.walk_plan(by > 0);
        let Some(open) = self.open.as_mut().filter(|open| open.item != was) else { return false };
        if by < 0 {
            open.cursor = open.deck.slides.len().saturating_sub(1);
        }
        open.armed = true;
        true
    }

    /// A click on a slide in Preview: the ring goes there and the projector is left alone.
    /// Preview is the safe side of the window; only Show, Enter and the Live pane put
    /// something on the projector (owner, 2026-11-11: "Why does clicking it show it on the live?").
    pub fn select_at(&mut self, index: usize) {
        let Some(open) = self.open.as_mut() else { return };
        if index < open.deck.slides.len() {
            open.cursor = index;
            open.armed = true;
            open.queued = false;
        }
    }

    /// Whether the room sees what the Live list holds.
    pub fn on_air(&self) -> bool {
        !self.off_air && !self.settings.projector_off
    }

    /// The Live switch. Show moves Preview into the Live list; this decides whether the
    /// projector shows it (owner, 2026-11-11: "Show should not be same as live, it should only
    /// move to live view but not force live on"). EasyWorship does the same: "To turn off the
    /// live output, click the LIVE button". Off, the projector's window is gone and the list
    /// keeps its place; on, it comes back where the Screens dialog put it.
    pub fn toggle_live(&mut self) {
        if self.on_air() {
            self.off_air = true;
        } else {
            self.off_air = false;
            if self.settings.projector_off {
                self.settings.projector_off = false;
                self.touch_settings();
            }
        }
        self.link.place(self.output_options());
    }

    /// The title of the plan item before or after the one in Preview.
    pub fn walk_title(&self, forward: bool) -> Option<String> {
        let id = self.walk_target(forward)?;
        self.plan.items.iter().find(|item| item.id == id).map(|item| self.entry_title(&item.entry))
    }

    /// Shift and an arrow: moves a ring through the live deck and leaves the projector alone.
    /// Letting Shift go shows the slide it is on, so a far slide is reached without the ones
    /// between passing over the room's screen (owner, 2026-11-11: "when release the shift/ctrl
    /// it activates"). With nothing live the ring is the Preview one, and letting go starts there.
    pub fn pick_slide(&mut self, by: isize) {
        match &self.live {
            Some(live) => {
                let last = live.deck.slides.len().saturating_sub(1) as isize;
                let from = self.live_pick.unwrap_or(live.index) as isize;
                self.live_pick = Some((from + by).clamp(0, last.max(0)) as usize);
            }
            None => {
                self.select(by);
                self.preview_pick = self.open.is_some();
            }
        }
    }

    /// Shift with Left or Right: the pick goes to the first or the last slide.
    pub fn pick_slide_edge(&mut self, last: bool) {
        match &self.live {
            Some(live) => self.live_pick = Some(if last { live.deck.slides.len().saturating_sub(1) } else { 0 }),
            None => {
                self.select_edge(last);
                self.preview_pick = self.open.is_some();
            }
        }
    }

    /// Shift was let go: the picked slide goes to the projector. True when something was picked.
    pub fn release_pick(&mut self) -> bool {
        let preview = std::mem::take(&mut self.preview_pick);
        match self.live_pick.take() {
            // The slide already showing is left as it is; going to it would lift Black.
            Some(pick) => {
                if self.live.as_ref().is_some_and(|live| live.index != pick) {
                    self.go_to(pick);
                }
                true
            }
            None if preview && self.live.is_none() => {
                let cursor = self.open.as_ref().map(|open| open.cursor).unwrap_or(0);
                self.show(cursor);
                true
            }
            None => false,
        }
    }

    /// Esc while Shift is held: nothing is shown.
    pub fn cancel_pick(&mut self) -> bool {
        let picking = self.live_pick.is_some() || self.preview_pick;
        self.live_pick = None;
        self.preview_pick = false;
        picking
    }

    /// Puts the selector on the first or the last slide of the Preview deck.
    pub fn select_edge(&mut self, last: bool) -> bool {
        let Some(open) = self.open.as_mut() else { return false };
        let to = if last { open.deck.slides.len().saturating_sub(1) } else { 0 };
        let moved = to != open.cursor;
        open.cursor = to;
        open.armed = true;
        open.queued = false;
        moved
    }

    /// Whether Enter would show something other than the live slide: another deck, or a slide
    /// of the live deck picked by hand.
    pub fn preview_waits(&self) -> bool {
        match (&self.open, &self.live) {
            (Some(open), Some(live)) => !open.deck.slides.is_empty() && (!self.open_is_live() || (open.armed && open.cursor != live.index)),
            (Some(open), None) => !open.deck.slides.is_empty(),
            _ => false,
        }
    }

    /// Fills the Preview pane with what comes after the live item, so Enter, or Next at the end
    /// of the live deck, goes on with the plan. With nothing after it, Preview holds the live deck.
    fn queue_next(&mut self) {
        let Some(live) = &self.live else { return };
        if let Some(id) = live.item {
            self.resume = Some(id);
        }
        let queued = match self.neighbour(true) {
            Some((id, deck)) => OpenDeck { cursor: self.start_cursor(id, &deck), deck, item: Some(id), queued: true, armed: false },
            None => OpenDeck { deck: live.deck.clone(), item: live.item, cursor: live.index, queued: true, armed: false },
        };
        self.open = Some(queued);
    }

    /// Fills the Preview pane again after the plan changed under a deck Artosia queued.
    pub fn requeue(&mut self) {
        if self.live.is_some() && self.open.as_ref().is_none_or(|open| open.queued) {
            self.queue_next();
        }
    }

    /// The plan item before or after the one in Preview.
    pub fn walk_target(&self, forward: bool) -> Option<u64> {
        let from = self.open.as_ref().and_then(|open| open.item).or(self.live.as_ref().and_then(|live| live.item)).or(self.resume);
        match from {
            Some(id) if forward => self.plan.next_showable(id).map(|item| item.id),
            Some(id) => self.plan.previous_showable(id).map(|item| item.id),
            // With no place in the plan yet, forward starts it and there is nothing before.
            None if forward => self.plan.items.iter().find(|item| !item.entry.is_heading()).map(|item| item.id),
            None => None,
        }
    }

    /// Opens the plan item before or after the open one, without showing it.
    pub fn walk_plan(&mut self, forward: bool) {
        if let Some(id) = self.walk_target(forward) {
            self.open_plan_item(id);
        }
    }

    /// Puts the live deck in the Preview pane too, so the selector can pick another part of it.
    pub fn open_live(&mut self) {
        if let Some(live) = &self.live {
            self.open = Some(OpenDeck::new(live.deck.clone(), live.item, live.index));
        }
    }

    /// Shows the first slide of the first thing in the plan.
    pub fn start_service(&mut self) {
        let first = self.plan.items.iter().filter(|item| !item.entry.is_heading()).find(|item| self.deck_for(&item.entry).is_some_and(|deck| !deck.slides.is_empty()));
        match first.map(|item| item.id) {
            Some(id) => {
                self.open_plan_item(id);
                self.show_start_of(id);
            }
            None => self.say(t::PLAN_EMPTY(), None),
        }
    }

    /// Moves the announcement loop on when the live slide's time is up. Returns the seconds
    /// until it next has work, or None when no loop runs.
    pub fn loop_step(&mut self, clock: f64) -> Option<f64> {
        let on_air = self.live.as_ref().is_some_and(|live| live.deck.source == DeckSource::Announcements && !live.deck.slides.is_empty());
        if !self.looping || !on_air {
            self.looping = false;
            return None;
        }
        // A birthday announcement can run over several slides, so each slide's time is looked up.
        let own: Vec<u32> = self.announcement_pages().into_iter().map(|(at, _)| self.announcements.get(at).map(|item| item.seconds).unwrap_or(10)).collect();
        let seconds = |index: usize| f64::from(own.get(index).copied().unwrap_or(10).clamp(2, 600));
        let live = self.live.as_mut()?;
        if self.loop_at.0 != live.index || self.loop_at.1 <= 0.0 {
            // A slide shown by hand gets its full time.
            self.loop_at = (live.index, clock + seconds(live.index));
        } else if clock >= self.loop_at.1 {
            live.index = (live.index + 1) % live.deck.slides.len();
            self.loop_at = (live.index, clock + seconds(live.index));
            self.follow_live();
        }
        Some((self.loop_at.1 - clock).max(0.05))
    }

    pub fn open_deck(&mut self, deck: Deck, item: Option<u64>, cursor: usize) {
        let item = item.or_else(|| self.plan_item_of(&deck.source));
        let last = deck.slides.len().saturating_sub(1);
        // Opening what is already live keeps the cursor on the live slide.
        let cursor = match &self.live {
            Some(live) if live.deck.source == deck.source && live.item == item => live.index.min(last),
            _ => cursor.min(last),
        };
        self.open = Some(OpenDeck::new(deck, item, cursor));
        self.editor = None;
        self.notices = None;
    }

    pub fn open_song(&mut self, at: usize, section: Option<usize>) {
        let Some(song) = self.songs.get(at) else { return };
        let deck = self.deck_of_song(song);
        let cursor = section
            .and_then(|section| song.sections().get(section).map(|s| s.tag))
            .and_then(|tag| deck.slides.iter().position(|slide| slide.tag == Some(tag)))
            .unwrap_or(0);
        self.open_deck(deck, None, cursor);
    }

    pub fn open_passage(&mut self, reference: &Reference) {
        if let Some(deck) = self.deck_of_passage(reference) {
            self.open_deck(deck, None, 0);
        }
    }

    pub fn open_plan_item(&mut self, id: u64) {
        let Some(item) = self.plan.items.iter().find(|item| item.id == id) else { return };
        if let Some(deck) = self.deck_for(&item.entry) {
            // A song marked to begin at its chorus opens with the ring there.
            let cursor = self.start_cursor(id, &deck);
            self.open_deck(deck, Some(id), cursor);
        }
    }

    pub fn open_is_live(&self) -> bool {
        match (&self.open, &self.live) {
            (Some(open), Some(live)) => open.deck.source == live.deck.source && open.item == live.item,
            _ => false,
        }
    }

    /// Puts a slide of the Preview deck on the projector, and queues what comes after it.
    /// Anything sent to the screen shows: black, logo and cleared text all lift.
    pub fn show(&mut self, index: usize) {
        let Some(open) = self.open.as_mut() else { return };
        if open.deck.slides.is_empty() {
            return;
        }
        let index = index.min(open.deck.slides.len() - 1);
        open.cursor = index;
        open.armed = false;
        let (source, tag, first) = (open.deck.source.clone(), open.deck.slides[index].tag, open.deck.slides[0].tag);
        let before = self.live.as_ref().map(|live| live.deck.source.clone());
        let (deck, item) = (open.deck.clone(), open.item);
        self.remember_live(&source);
        self.live = Some(LiveDeck { deck, item, index });
        // Another slide of what is already live is not a new song going up.
        if before.as_ref() != Some(&source) {
            self.log_live(&source, tag, first, index);
        }
        self.reveal();
        self.queue_next();
    }

    fn reveal(&mut self) {
        self.black = false;
        self.text_hidden = false;
        self.logo_on = false;
    }

    fn follow_live(&mut self) {
        let Some(live) = &self.live else { return };
        match self.open.as_mut() {
            Some(open) if !open.armed && open.deck.source == live.deck.source && open.item == live.item => open.cursor = live.index,
            _ => {}
        }
    }

    /// The plan item after the live one, with its deck.
    fn neighbour(&self, forward: bool) -> Option<(u64, Deck)> {
        let live = self.live.as_ref()?;
        let mut id = match live.item.or(if forward { self.resume } else { None }) {
            Some(id) => id,
            // Right after another plan was opened, what is live belongs to none of its items and
            // Next goes on from the top of the new plan.
            None if forward && self.switcher.fresh => return self.plan_start(),
            None => return None,
        };
        // Skips entries with nothing to show, such as a deleted song.
        for _ in 0..self.plan.items.len() {
            let item = if forward { self.plan.next_showable(id) } else { self.plan.previous_showable(id) }?;
            match self.deck_for(&item.entry) {
                Some(deck) if !deck.slides.is_empty() => return Some((item.id, deck)),
                _ => id = item.id,
            }
        }
        None
    }

    /// The title of what Next will reach once the live deck ends.
    pub fn next_item_title(&self) -> Option<String> {
        self.neighbour(true).map(|(_, deck)| deck.title)
    }

    pub fn next(&mut self) {
        let Some(live) = self.live.as_mut() else {
            // Nothing is live yet: Next starts the Preview deck at its selector, or the plan.
            match self.open.as_ref().map(|open| open.cursor) {
                Some(cursor) => self.show(cursor),
                None => self.start_service(),
            }
            return;
        };
        if live.index + 1 < live.deck.slides.len() {
            live.index += 1;
            self.reveal();
            self.follow_live();
        } else if self.preview_waits() {
            // The live deck is over: what waits in Preview goes on, at its selector.
            let cursor = self.open.as_ref().map(|open| open.cursor).unwrap_or(0);
            self.show(cursor);
        } else if let Some((id, deck)) = self.neighbour(true) {
            let cursor = self.start_cursor(id, &deck);
            self.open = Some(OpenDeck::new(deck, Some(id), cursor));
            self.show(cursor);
        }
    }

    pub fn previous(&mut self) {
        let Some(live) = self.live.as_mut() else { return };
        if live.index > 0 {
            live.index -= 1;
            self.reveal();
            self.follow_live();
        } else if let Some((id, deck)) = self.neighbour(false) {
            let index = deck.slides.len().saturating_sub(1);
            self.remember_live(&deck.source);
            self.live = Some(LiveDeck { deck, item: Some(id), index });
            self.reveal();
            self.requeue();
        }
    }

    /// The first or the last slide of the live deck.
    pub fn go_to_edge(&mut self, last: bool) {
        let end = self.live.as_ref().map(|live| live.deck.slides.len().saturating_sub(1)).unwrap_or(0);
        self.go_to(if last { end } else { 0 });
    }

    pub fn go_to(&mut self, index: usize) {
        if let Some(live) = self.live.as_mut()
            && index < live.deck.slides.len() {
                live.index = index;
                self.reveal();
                self.follow_live();
            }
    }

    /// Enter in the main window: show the slide with the selector, else go on.
    pub fn enter(&mut self) {
        if self.preview_waits() {
            let cursor = self.open.as_ref().map(|open| open.cursor).unwrap_or(0);
            self.show(cursor);
        } else {
            self.next();
        }
    }

    pub fn toggle_black(&mut self) {
        self.black = !self.black;
    }

    pub fn toggle_text(&mut self) {
        self.text_hidden = !self.text_hidden;
    }

    pub fn toggle_logo(&mut self) {
        if !self.settings.logo.is_empty() {
            self.logo_on = !self.logo_on;
        }
    }

    pub fn start_countdown(&mut self) {
        let minutes: f64 = self.timer_minutes.text.trim().replace(',', ".").parse().unwrap_or(5.0);
        let minutes = minutes.clamp(0.1, 600.0);
        self.countdown = Some(Countdown { label: t::COUNTDOWN_LABEL().to_string(), ends_at: self.now + minutes * 60.0 });
    }

    /// What the projector and the stage screen should show now.
    pub fn output_state(&self) -> OutputState {
        let slide = self.live.as_ref().and_then(|live| live.deck.slides.get(live.index).cloned());
        let stage = match &self.live {
            Some(live) => StageInfo {
                title: live.deck.title.clone(),
                next: live.deck.slides.get(live.index + 1).cloned(),
                next_title: if live.index + 1 >= live.deck.slides.len() { self.next_item_title().unwrap_or_default() } else { String::new() },
                index: live.index,
                count: live.deck.slides.len(),
                chips: live
                    .deck
                    .slides
                    .iter()
                    .enumerate()
                    .filter(|(_, slide)| slide.tag.is_some())
                    .map(|(i, slide)| StageChip {
                        text: slide.short.clone(),
                        group: slide.tag.map(tag_group).unwrap_or(0),
                        active: i == live.index,
                    })
                    .collect(),
                message: if self.stage_message_on { self.stage_message.text.clone() } else { String::new() },
                words: stage_words(),
            },
            None => StageInfo {
                message: if self.stage_message_on { self.stage_message.text.clone() } else { String::new() },
                words: stage_words(),
                ..StageInfo::default()
            },
        };
        OutputState {
            slide,
            look: self.look(),
            black: self.black,
            text_hidden: self.text_hidden,
            logo: self.settings.logo.clone(),
            logo_on: self.logo_on,
            message: if self.message_on { self.message.text.clone() } else { String::new() },
            countdown: self.countdown.clone(),
            stage,
            warp: self.settings.warp,
        }
    }

    /// The same state with another slide in place, for a slide tile.
    pub fn preview_state(&self, slide: Option<artosia_core::deck::Slide>) -> OutputState {
        OutputState { slide, look: self.look(), ..OutputState::default() }
    }

    pub fn say(&mut self, text: impl Into<String>, undo: Option<Undo>) {
        // A toast that can be undone stays long enough to read and reach.
        let stay = if undo.is_some() { 8.0 } else { 4.0 };
        self.toast = Some(Toast { text: text.into(), undo, until: self.clock + stay });
    }

    pub fn output_options(&self) -> crate::link::OutputOptions {
        crate::link::OutputOptions {
            audience: self.settings.audience_screen,
            stage: self.settings.stage_screen,
            stage_window: self.settings.stage_window,
            here: self.settings.projector_here,
            off: self.settings.projector_off || self.off_air,
        }
    }

    /// Rebuilds the open and live decks after songs, announcements or the look changed.
    pub fn refresh_decks(&mut self) {
        let rebuilt = |app: &App, source: &DeckSource| -> Option<Deck> {
            match source {
                DeckSource::Song(id) => app.song_position(*id).map(|at| app.deck_of_song(&app.songs[at])),
                DeckSource::Passage { reference, .. } => app.deck_of_passage(reference),
                DeckSource::Announcements => Some(app.announcements_deck()),
                DeckSource::Announcement(id) => app.card_deck(*id),
            }
        };
        if let Some(source) = self.open.as_ref().map(|open| open.deck.source.clone()) {
            match rebuilt(self, &source) {
                Some(deck) => {
                    if let Some(open) = self.open.as_mut() {
                        open.cursor = crate::song_languages::same_place(&open.deck, open.cursor, &deck);
                        open.deck = deck;
                    }
                }
                None => self.open = None,
            }
        }
        if let Some(source) = self.live.as_ref().map(|live| live.deck.source.clone())
            && let Some(deck) = rebuilt(self, &source)
                && let Some(live) = self.live.as_mut() {
                    live.index = crate::song_languages::same_place(&live.deck, live.index, &deck);
                    live.deck = deck;
                }
    }

    pub fn reload_songs(&mut self) {
        self.songs = self.store.songs().unwrap_or_default();
        self.index = SongIndex::build(&self.songs);
        self.refresh_decks();
    }

    pub fn touch_settings(&mut self) {
        self.settings_dirty = true;
    }

    pub fn touch_plan(&mut self) {
        self.plan_dirty = true;
    }
}

/// The few words the stage monitor writes itself, in the window's language.
fn stage_words() -> artosia_slides::frame::StageWords {
    artosia_slides::frame::StageWords {
        clock: t::STAGE_CLOCK().to_string(),
        timer: t::STAGE_TIMER().to_string(),
        black: t::STAGE_BLACK().to_string(),
        nothing: t::STAGE_NOTHING().to_string(),
        next: t::stage_next("{label}"),
        place: t::stage_place("{index}", "{count}"),
    }
}

/// Whether a plan entry and a deck are the same thing.
pub fn entry_matches(entry: &PlanEntry, source: &DeckSource) -> bool {
    match (entry, source) {
        (PlanEntry::Song { song_id, .. }, DeckSource::Song(id)) => song_id == id,
        (PlanEntry::Passage { reference, .. }, DeckSource::Passage { reference: other, .. }) => reference == other,
        (PlanEntry::Announcements, DeckSource::Announcements) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nth song of the practice plan.
    fn song_item(app: &App, nth: usize) -> u64 {
        app.plan.items.iter().filter(|item| matches!(item.entry, PlanEntry::Song { .. })).nth(nth).map(|item| item.id).expect("the practice plan has songs")
    }

    fn live_at(app: &App) -> Option<(Option<u64>, usize)> {
        app.live.as_ref().map(|live| (live.item, live.index))
    }

    #[test]
    fn a_letter_goes_to_the_next_section_of_its_kind() {
        let mut app = App::practice();
        let first = song_item(&app, 0);
        app.open_plan_item(first);
        // Nothing is live: a letter and a digit move the ring and start nothing.
        assert!(app.jump_to_marker('C'));
        assert!(app.live.is_none());
        let chorus = app.open.as_ref().map(|open| open.cursor).unwrap_or(0);
        assert!(chorus > 0);
        app.jump_to_section(0);
        assert_eq!((app.live.is_none(), app.open.as_ref().map(|open| open.cursor)), (true, Some(0)));
        // Live: the letter moves the projector, and a kind the song lacks does nothing.
        app.show(0);
        assert!(app.jump_to_marker('C'));
        assert_eq!(live_at(&app), Some((Some(first), chorus)));
        assert!(!app.jump_to_marker('Q'));
        assert_eq!(live_at(&app), Some((Some(first), chorus)));
        // Past the last of its kind it goes round to the first.
        assert!(app.jump_to_marker('V'));
        assert_eq!(live_at(&app), Some((Some(first), 0)));
        assert!(app.marker_letters().contains(&'C'));
    }

    #[test]
    fn what_left_the_projector_can_be_brought_back_into_preview() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(1);
        // Another slide of the same song is not something that was left.
        app.go_to(0);
        assert!(app.history.is_empty());
        app.go_to(1);
        app.open_plan_item(second);
        app.show(0);
        assert_eq!(app.history.iter().map(|shown| (shown.item, shown.index)).collect::<Vec<_>>(), vec![(Some(first), 1)]);
        // Back into Preview where it was left; the projector stays on the second song.
        app.open_shown(0);
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(first), 1)));
        assert_eq!(live_at(&app), Some((Some(second), 0)));
        assert!(app.preview_waits());
    }

    #[test]
    fn backspace_steps_back_through_what_was_shown() {
        let mut app = App::practice();
        assert!(!app.open_earlier(), "nothing was shown before");
        let (first, second, third) = (song_item(&app, 0), song_item(&app, 1), song_item(&app, 2));
        app.open_plan_item(first);
        app.show(1);
        app.open_plan_item(second);
        app.show(0);
        app.open_plan_item(third);
        app.show(0);
        // The newest first, then the one before it, then round to the newest again.
        assert!(app.open_earlier());
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(second), 0)));
        assert!(app.open_earlier());
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(first), 1)));
        assert!(app.open_earlier());
        assert_eq!(app.open.as_ref().map(|open| open.item), Some(Some(second)));
        assert_eq!(live_at(&app), Some((Some(third), 0)), "the projector did not move");
    }

    #[test]
    fn a_click_in_preview_moves_the_ring_and_leaves_the_projector() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(0);
        app.open_plan_item(second);
        app.select_at(1);
        assert_eq!(live_at(&app), Some((Some(first), 0)), "the projector did not move");
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(second), 1)));
        // A slide the deck does not have is no place for the ring.
        app.select_at(400);
        assert_eq!(app.open.as_ref().map(|open| open.cursor), Some(1));
        // Show sends it, at the ring.
        assert!(app.preview_waits());
        app.enter();
        assert_eq!(live_at(&app), Some((Some(second), 1)));
    }

    #[test]
    fn the_live_switch_keeps_the_list_and_takes_the_projector_away() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(1);
        assert!(app.on_air());
        app.toggle_live();
        assert!(!app.on_air() && app.output_options().off);
        // The list keeps its place, and Show still moves Preview into it.
        assert_eq!(live_at(&app), Some((Some(first), 1)));
        app.enter();
        assert_eq!(live_at(&app), Some((Some(second), 0)));
        assert!(!app.on_air(), "Show does not switch the projector on");
        app.toggle_live();
        assert!(app.on_air() && !app.output_options().off);
        // Switched off in the Screens dialog, the Live switch turns it back on for good.
        app.settings.projector_off = true;
        app.toggle_live();
        assert!(app.on_air() && !app.settings.projector_off);
    }

    #[test]
    fn preview_names_the_plan_items_beside_it() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        let after = app.plan.items.iter().find(|item| item.id == second).map(|item| app.entry_title(&item.entry));
        assert_eq!(app.walk_title(true), after);
        assert_eq!(app.walk_target(true), Some(second));
    }

    #[test]
    fn showing_a_plan_item_queues_the_one_after_it() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(0);
        assert_eq!(live_at(&app), Some((Some(first), 0)));
        let open = app.open.as_ref().expect("Preview is filled");
        assert_eq!((open.item, open.cursor, open.queued), (Some(second), 0, true));
    }

    #[test]
    fn the_selector_moves_alone_and_the_next_song_starts_where_it_was_put() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(0);
        // Shift and Down: the ring moves in the song that waits; the projector stays.
        assert!(app.select(1));
        assert_eq!(live_at(&app), Some((Some(first), 0)));
        assert_eq!(app.open.as_ref().map(|open| (open.item, open.cursor)), Some((Some(second), 1)));
        // Next runs through the live song, and at its end goes on at the ring: a medley.
        let slides = app.live.as_ref().map(|live| live.deck.slides.len()).unwrap_or(0);
        for _ in 1..slides {
            app.next();
        }
        assert_eq!(live_at(&app), Some((Some(first), slides - 1)));
        app.next();
        assert_eq!(live_at(&app), Some((Some(second), 1)));
    }

    #[test]
    fn a_song_from_the_search_keeps_the_place_in_the_plan() {
        let mut app = App::practice();
        let (first, second) = (song_item(&app, 0), song_item(&app, 1));
        app.open_plan_item(first);
        app.show(0);
        let planned: Vec<i64> = app.plan.items.iter().filter_map(|item| if let PlanEntry::Song { song_id, .. } = item.entry { Some(song_id) } else { None }).collect();
        let extra = app.songs.iter().position(|song| !planned.contains(&song.id)).expect("the practice library has a song outside the plan");
        app.open_song(extra, None);
        app.show(0);
        assert_eq!(app.live.as_ref().and_then(|live| live.item), None);
        assert_eq!(app.open.as_ref().and_then(|open| open.item), Some(second), "Preview goes on with the plan");
        // Added now, it is the next one, ahead of what was already planned.
        app.add_song_to_plan(extra);
        let at = |id: u64| app.plan.position(id);
        let added = app.plan.items.iter().find(|item| matches!(item.entry, PlanEntry::Song { song_id, .. } if song_id == app.songs[extra].id)).map(|item| item.id);
        assert_eq!(added.and_then(at), at(first).map(|place| place + 1));
    }

    #[test]
    fn shift_picks_in_the_live_song_and_letting_go_shows_it() {
        let mut app = App::practice();
        let first = song_item(&app, 0);
        app.open_plan_item(first);
        app.show(0);
        app.toggle_black();
        // Held: the pick moves, the projector and Black stay.
        app.pick_slide(1);
        assert_eq!((live_at(&app), app.live_pick, app.black), (Some((Some(first), 0)), Some(1), true));
        // Past the end it stays on the last slide of this song.
        app.pick_slide(40);
        let last = app.live.as_ref().map(|live| live.deck.slides.len() - 1).unwrap_or(0);
        assert_eq!(app.live_pick, Some(last));
        app.pick_slide_edge(false);
        app.pick_slide(1);
        // Let go: the picked slide shows.
        assert!(app.release_pick());
        assert_eq!((live_at(&app), app.live_pick, app.black), (Some((Some(first), 1)), None, false));
        // Esc while held shows nothing.
        app.pick_slide(-1);
        assert!(app.cancel_pick());
        assert!(!app.release_pick());
        assert_eq!(live_at(&app), Some((Some(first), 1)));
    }

    #[test]
    fn the_live_song_can_be_armed_for_another_of_its_own_parts() {
        let mut app = App::practice();
        // A plan that ends on its first song.
        let last = song_item(&app, 0);
        let keep = app.plan.position(last).map(|at| at + 1).unwrap_or(0);
        app.plan.items.truncate(keep);
        app.open_plan_item(last);
        app.show(0);
        // Nothing follows, so Preview holds the live deck and Enter is Next.
        assert!(app.open_is_live() && !app.preview_waits());
        assert!(app.select(1));
        assert!(app.preview_waits());
        app.go_to(0);
        assert_eq!(app.open.as_ref().map(|open| open.cursor), Some(1), "an armed ring stays put");
        app.enter();
        assert_eq!(live_at(&app), Some((Some(last), 1)));
    }
}
