//! Plan edits, the song editor, importing, announcements and the work done between frames.

use std::sync::Arc;

use artosia_core::bible::Reference;
use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, DeckSource, Fill, NoticeStyle, Person, Template};
use artosia_core::window::{Range, Window};
use artosia_core::plan::PlanEntry;
use artosia_core::marks;
use artosia_core::song::{SectionKind, Song, Tag, parse_order, song_from_paste, tidy_lyrics, write_order};
use artosia_remote::{RemoteCommand, RemoteConfig, RemoteHit, RemotePlanItem, RemoteSection, RemoteServer, RemoteSlide, RemoteState};
use artosia_slides::frame::ToOutput;
use artosia_slides::look::{Background, Look};
use rune::{TextEdit, Tx};

use crate::link::OutputOptions;
use crate::firewall::{self, Standing};
use crate::state::{App, Arrived, Edge, Editor, HeardHit, Notices, OpenDeck, Sheet, Suggestion, Tab, TempoCount, Undo, entry_matches, section_starts, unix_now};
use crate::strings as t;

/// The phone remote's port, for the server and for its firewall rule.
const REMOTE_PORT: u16 = 4780;

/// Seconds between frames while something pulses on the beat: gentle enough for a slow PC.
const BEAT_FRAME: f64 = 0.05;
/// A tempo report older than this is forgotten: the listener has stopped or the pulse is gone.
const TEMPO_STALE: f64 = 2.5;
/// A reading within this share of the one a person corrected is still the same song.
const TEMPO_SAME: f32 = 0.06;

/// A QR code as a picture, eight pixels to a module, black on white.
fn qr_picture(text: &str) -> Option<rune::Pixels> {
    let modules = artosia_remote::qr_modules(text)?;
    let (count, unit) = (modules.len(), 8usize);
    let side = count * unit;
    let mut rgba = vec![255u8; side * side * 4];
    for (y, line) in modules.iter().enumerate() {
        for (x, dark) in line.iter().enumerate() {
            if !*dark {
                continue;
            }
            for py in 0..unit {
                for px in 0..unit {
                    let at = ((y * unit + py) * side + x * unit + px) * 4;
                    rgba[at..at + 3].fill(0);
                }
            }
        }
    }
    let picture = rune::Pixels::new();
    picture.set(side as u32, side as u32, rgba);
    Some(picture)
}

impl App {
    // ---- Plan ----

    fn plan_has(&self, source: &DeckSource) -> bool {
        self.plan.items.iter().any(|item| entry_matches(&item.entry, source))
    }

    pub fn open_in_plan(&self) -> bool {
        self.open.as_ref().is_some_and(|open| {
            // The plan holds the announcements as one item, so a single card has nothing to add.
            let card_of_plan = matches!(open.deck.source, DeckSource::Announcement(_)) && self.plan.items.iter().any(|item| matches!(item.entry, PlanEntry::Announcements));
            open.item.is_some() || card_of_plan || self.plan_has(&open.deck.source)
        })
    }

    pub fn add_song_to_plan(&mut self, at: usize) {
        let Some(song) = self.songs.get(at) else { return };
        let (id, title) = (song.id, song.title.clone());
        self.queue_entry(PlanEntry::song(id, title.clone()));
        self.requeue();
        self.say(t::added_to_plan(&title), None);
    }

    pub fn add_passage_to_plan(&mut self, reference: &Reference) {
        let title = self.passage_title(reference);
        let second = if self.settings.second_bible.is_empty() { None } else { Some(self.settings.second_bible.clone()) };
        self.queue_entry(PlanEntry::Passage { reference: *reference, bible: self.settings.bible.clone(), second, title: title.clone() });
        self.requeue();
        self.say(t::added_to_plan(&title), None);
    }

    /// Adds the open deck to the plan and ties the two together.
    pub fn add_open_to_plan(&mut self) {
        let Some(open) = self.open.as_ref() else { return };
        let entry = match &open.deck.source {
            DeckSource::Song(id) => PlanEntry::song(*id, open.deck.title.clone()),
            DeckSource::Passage { reference, bible, second } => {
                PlanEntry::Passage { reference: *reference, bible: bible.clone(), second: second.clone(), title: open.deck.title.clone() }
            }
            DeckSource::Announcements | DeckSource::Announcement(_) => PlanEntry::Announcements,
            // A rundown item is opened from the plan and is already in it.
            DeckSource::Media(_) => return,
        };
        let was_live = self.open_is_live();
        let title = open.deck.title.clone();
        let id = self.queue_entry(entry);
        if let Some(open) = self.open.as_mut() {
            open.item = Some(id);
        }
        if was_live
            && let Some(live) = self.live.as_mut() {
                live.item = Some(id);
            }
        self.requeue();
        self.say(t::added_to_plan(&title), None);
    }

    /// Adds an entry to the plan right after what is live, so it is the next one. With
    /// nothing from the plan live yet it goes to the end, the order a plan is written in.
    pub(crate) fn queue_entry(&mut self, entry: PlanEntry) -> u64 {
        let after = self.live.as_ref().and_then(|live| live.item).or(self.resume).and_then(|id| self.plan.position(id));
        let id = match after {
            Some(at) => self.plan.insert(at + 1, entry),
            None => self.plan.push(entry),
        };
        self.touch_plan();
        id
    }

    /// Lets go of a dragged plan row at the place it was carried to.
    pub fn drop_plan_row(&mut self, cancelled: bool) {
        let Some(held) = self.plan_drag.take() else { return };
        self.plan_dropped = true;
        if !cancelled && held.to != held.from {
            self.plan.move_to(held.id, held.to);
            self.touch_plan();
            self.requeue();
        }
    }

    pub fn add_heading(&mut self) {
        self.plan.push(PlanEntry::Heading { title: t::PLAN_HEADING_NEW().to_string() });
        self.touch_plan();
    }

    pub fn remove_plan_item(&mut self, id: u64) {
        if let Some((at, item)) = self.plan.remove(id) {
            let title = self.entry_title(&item.entry);
            for holder in [self.open.as_mut().map(|open| &mut open.item), self.live.as_mut().map(|live| &mut live.item)].into_iter().flatten() {
                if *holder == Some(id) {
                    *holder = None;
                }
            }
            self.plan_menu = None;
            self.touch_plan();
            self.say(t::removed_from_plan(&title), Some(Undo::PlanItem { at, item }));
        }
    }

    pub fn shift_plan_item(&mut self, id: u64, by: isize) {
        if self.plan.shift(id, by) {
            self.touch_plan();
            self.requeue();
        }
    }

    /// Starts another plan beside the open one, which is kept (`plans.rs`).
    pub fn new_plan(&mut self) {
        self.begin_new_plan();
    }

    pub(crate) fn save_plan_now(&mut self) {
        if self.store.save_plan(&mut self.plan).is_ok() && self.settings.plan != self.plan.id {
            self.settings.plan = self.plan.id;
            self.settings_dirty = true;
        }
        self.plan_dirty = false;
    }

    pub fn undo(&mut self) {
        let Some(toast) = self.toast.take() else { return };
        match toast.undo {
            Some(Undo::PlanItem { at, item }) => {
                self.plan.items.insert(at.min(self.plan.items.len()), item);
                self.touch_plan();
            }
            Some(Undo::Song(song)) => {
                if self.store.restore_song(&song).is_ok() {
                    self.reload_songs();
                }
            }
            Some(Undo::Editor(editor)) => {
                let mut editor = *editor;
                editor.lyrics.focused = true;
                self.editor = Some(editor);
            }
            Some(Undo::History(history)) => {
                let _ = self.store.restore_history(*history);
            }
            Some(Undo::Announcement(mut item)) => {
                item.id = 0;
                if self.store.save_announcement(&mut item).is_ok() {
                    self.reload_announcements();
                }
            }
            Some(Undo::Plan(plan)) => self.restore_plan(*plan),
            Some(Undo::PlanLook { plan, look }) => self.restore_plan_look(plan, look),
            Some(Undo::PlanLanguages { plan, list }) => self.restore_plan_languages(plan, list),
            Some(Undo::EditorLanguage { at, entry }) => self.restore_editor_language(at, entry),
            Some(Undo::Recordings) => self.restore_recordings(),
            Some(Undo::Event(event)) => self.restore_event(event),
            Some(Undo::Series { before, added }) => self.restore_series(before, added),
            Some(Undo::EventBack(event) | Undo::Moved { before: event, .. }) => self.put_back_event(event),
            Some(Undo::Calendar(calendar, events)) => self.restore_calendar(calendar, events),
            Some(Undo::Person(person)) => self.restore_person(person),
            Some(Undo::People(people)) => self.restore_people(people),
            Some(Undo::Gift(gift)) => self.restore_gift(gift),
            Some(Undo::Fund(fund, gifts)) => self.restore_fund(fund, gifts),
            None => {}
        }
    }

    // ---- Song editor ----

    fn editor_for(song: &Song) -> Editor {
        let mut editor = Editor {
            id: song.id,
            title: TextEdit::new(song.title.clone()),
            author: TextEdit::new(song.author.clone()),
            copyright: TextEdit::new(song.copyright.clone()),
            ccli: TextEdit::new(song.ccli.clone()),
            key: TextEdit::new(song.key.clone()),
            order: TextEdit::new(song.order.clone()),
            lyrics: TextEdit::new(song.lyrics.clone()),
            ..Editor::default()
        };
        (editor.codes, editor.parked) = Editor::languages_of(song.owned_texts());
        if song.title.is_empty() {
            editor.title.focused = true;
        } else {
            editor.lyrics.focused = true;
        }
        editor
    }

    pub fn edit_new_song(&mut self, song: Song) {
        self.editor = Some(App::editor_for(&song));
        self.notices = None;
        self.tab = Tab::Plan;
        self.close_palette();
        self.sheet = Sheet::None;
        self.title_changed();
    }

    /// Ctrl E: opens what is in Preview for editing. A song opens its editor, and the
    /// announcements open theirs; a Bible passage has nothing to edit.
    /// The streaming dialog, with the caret in the address while there is none to stream to.
    pub fn open_stream(&mut self) {
        self.sheet = Sheet::Stream;
        self.stream_address.focused = self.stream.is_none() && self.stream_address.text.trim().is_empty();
    }

    pub fn edit_open(&mut self) {
        match self.open.as_ref().map(|open| &open.deck.source) {
            Some(DeckSource::Song(_)) => self.edit_open_song(),
            Some(DeckSource::Announcements | DeckSource::Announcement(_)) => self.edit_announcements(),
            _ => {}
        }
    }

    pub fn edit_open_song(&mut self) {
        let Some(DeckSource::Song(id)) = self.open.as_ref().map(|open| open.deck.source.clone()) else { return };
        if let Some(at) = self.song_position(id) {
            self.editor = Some(App::editor_for(&self.songs[at]));
            self.notices = None;
            self.tab = Tab::Plan;
        }
    }

    // ---- Whole songs suggested for a new one ----

    /// Runs when a new song's title changes: lists the songs the library already has under
    /// that name at once, and asks the web a moment after the typing stops.
    pub fn title_changed(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.suggestion_armed = false;
        }
        let Some(editor) = self.editor.as_ref().filter(|editor| editor.id == 0 && editor.lyrics.text.trim().is_empty()) else {
            if let Some(editor) = self.editor.as_mut() {
                editor.suggestions.clear();
                editor.ask_at = None;
            }
            return;
        };
        let title = editor.title.text.trim().to_string();
        let mut suggestions: Vec<Suggestion> = Vec::new();
        if title.chars().count() >= 3 {
            suggestions.extend(self.index.search(&title, 3).into_iter().filter(|hit| hit.complete && hit.in_title).map(|hit| Suggestion::Library(hit.song)));
        }
        let clock = self.clock;
        let Some(editor) = self.editor.as_mut() else { return };
        // What the web already answered for these exact words stays under the library's songs.
        let kept: Vec<Suggestion> = if editor.asked == title { editor.suggestions.iter().filter(|s| matches!(s, Suggestion::Web(_))).cloned().collect() } else { Vec::new() };
        suggestions.extend(kept);
        editor.suggestions = suggestions;
        editor.suggestion = 0;
        editor.ask_at = (title.chars().count() >= 4 && editor.asked != title).then_some(clock + 0.4);
    }

    /// Asks the web for songs with the title as typed, on a worker.
    fn ask_web(&mut self) {
        let Some(editor) = self.editor.as_mut() else { return };
        editor.ask_at = None;
        let title = editor.title.text.trim().to_string();
        editor.asked = title.clone();
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let found = artosia_lyrics::search(&title).map_err(|error| crate::why::lyrics(&error));
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Lyrics(title, found));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    /// Down and Up in the title choose among the songs offered under it. Nothing is chosen
    /// until the first press, so Enter and Tab stay the title's own keys: a tester typed a
    /// title, pressed Tab for the next field, and was taken to another song (2026-11-11).
    pub fn move_suggestion(&mut self, by: isize) {
        let Some(editor) = self.editor.as_mut().filter(|editor| !editor.suggestions.is_empty()) else { return };
        let count = editor.suggestions.len() as isize;
        if !editor.suggestion_armed {
            editor.suggestion_armed = true;
            editor.suggestion = if by < 0 { (count - 1) as usize } else { 0 };
            return;
        }
        editor.suggestion = (editor.suggestion as isize + by).rem_euclid(count) as usize;
    }

    /// Takes the picked suggestion: opens the library's song, or fills the editor from the web.
    /// False when there was nothing to take.
    pub fn take_suggestion(&mut self, at: Option<usize>) -> bool {
        let Some(editor) = self.editor.as_mut() else { return false };
        let Some(picked) = editor.suggestions.get(at.unwrap_or(editor.suggestion)).cloned() else { return false };
        match picked {
            Suggestion::Library(song) => {
                self.editor = None;
                self.open_song(song, None);
            }
            Suggestion::Web(found) => {
                editor.fetching = true;
                editor.suggestions.clear();
                editor.ask_at = None;
                let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
                std::thread::spawn(move || {
                    let fetched = artosia_lyrics::fetch(&found).map_err(|error| crate::why::lyrics(&error));
                    if let Ok(mut inbox) = inbox.lock() {
                        inbox.push(Arrived::LyricsText(fetched));
                    }
                    if let Some(wake) = waker {
                        wake();
                    }
                });
            }
        }
        true
    }

    pub fn dismiss_suggestions(&mut self) -> bool {
        if let Some(editor) = self.editor.as_mut() {
            editor.suggestion_armed = false;
        }
        match self.editor.as_mut().filter(|editor| !editor.suggestions.is_empty()) {
            Some(editor) => {
                editor.suggestions.clear();
                editor.ask_at = None;
                true
            }
            None => false,
        }
    }

    /// Puts fetched words into the editor as a whole song.
    fn fill_editor(&mut self, fetched: artosia_lyrics::Fetched) {
        let Some(editor) = self.editor.as_mut() else { return };
        editor.fetching = false;
        editor.title.replace_text(fetched.title.clone());
        editor.asked = fetched.title;
        editor.author.replace_text(fetched.artist);
        editor.lyrics.replace_text(tidy_lyrics(&fetched.text));
        editor.credit = Some((fetched.source, fetched.url));
        editor.title.focused = false;
        editor.lyrics.focused = true;
    }

    /// Lyrics pasted from anywhere become a new song, ready to check and save.
    pub fn song_from_clipboard(&mut self, text: &str) {
        if text.trim().is_empty() {
            self.say(t::PASTE_EMPTY(), None);
            return;
        }
        self.edit_new_song(song_from_paste(text));
    }

    /// The song the editor holds, as it would be saved.
    pub fn editor_song(&self) -> Option<Song> {
        let editor = self.editor.as_ref()?;
        let existing = self.song_position(editor.id).map(|at| self.songs[at].clone()).unwrap_or_default();
        let mut song = Song {
            id: editor.id,
            title: editor.title.text.trim().to_string(),
            author: editor.author.text.trim().to_string(),
            copyright: editor.copyright.text.trim().to_string(),
            ccli: editor.ccli.text.trim().to_string(),
            key: editor.key.text.trim().to_string(),
            order: editor.order.text.trim().to_string(),
            ..existing
        };
        // Every language, the one in the field as it is typed now; nothing is ever written for one that was not typed.
        song.set_texts(&editor.texts());
        Some(song)
    }

    pub fn save_editor(&mut self) {
        let Some(mut song) = self.editor_song() else { return };
        if song.title.is_empty() {
            song.title = song.first_line().unwrap_or_else(|| t::SONG_UNTITLED().to_string());
        }
        // A change made here to an imported song must outlive the next import of its library.
        let before = self.song_position(song.id).map(|at| &self.songs[at]);
        if !song.source.is_empty() && before.is_some_and(|before| *before != song) {
            song.edited = true;
        }
        match self.store.save_song(&mut song) {
            Ok(()) => {
                let id = song.id;
                self.editor = None;
                self.reload_songs();
                for item in &mut self.plan.items {
                    if let PlanEntry::Song { song_id, title, .. } = &mut item.entry
                        && *song_id == id && *title != song.title {
                            *title = song.title.clone();
                            self.plan_dirty = true;
                        }
                }
                if let Some(at) = self.song_position(id) {
                    self.open_song(at, None);
                    if std::mem::take(&mut self.plan_new_song) {
                        self.add_song_to_plan(at);
                    }
                }
                self.say(t::SONG_SAVED(), None);
            }
            Err(error) => self.say(t::could_not_save(&error.to_string()), None),
        }
    }

    /// Closes the editor. Unsaved changes are kept in a toast for eight seconds, so a stray
    /// Escape never costs a volunteer the words they typed.
    pub fn close_editor(&mut self) {
        let changed = match (self.editor_song(), self.editor.as_ref()) {
            (Some(song), Some(editor)) => match self.song_position(editor.id) {
                Some(at) => self.songs[at] != song,
                None => !song.title.is_empty() || !song.lyrics.trim().is_empty(),
            },
            _ => false,
        };
        if let (true, Some(editor)) = (changed, self.editor.take()) {
            self.say(t::EDITOR_CLOSED(), Some(Undo::Editor(Box::new(editor))));
        }
        self.editor = None;
    }

    /// Starts a section at the caret. A verse takes the next free number, and a second chorus
    /// or bridge is numbered when the song already has one.
    pub fn insert_section(&mut self, kind: SectionKind) {
        let Some(song) = self.editor_song() else { return };
        let taken = song.sections().iter().filter(|section| section.tag.kind == kind).map(|section| section.tag.number).max().unwrap_or(0);
        let once = matches!(kind, SectionKind::Intro | SectionKind::Ending);
        let tag = Tag::new(kind, if once { 1 } else { taken + 1 });
        self.insert_marker(&tag.marker());
    }

    /// The order the editor's song is sung in, as it stands: what is typed, else as written.
    pub fn editor_order(&self) -> Vec<Tag> {
        let Some(song) = self.editor_song() else { return Vec::new() };
        song.arranged().iter().map(|section| section.tag).collect()
    }

    fn set_order(&mut self, tags: &[Tag]) {
        if let Some(editor) = self.editor.as_mut() {
            editor.order.replace_text(write_order(tags));
        }
    }

    pub fn order_add(&mut self, tag: Tag) {
        let mut order = self.editor_order();
        order.push(tag);
        self.set_order(&order);
    }

    pub fn order_remove(&mut self, at: usize) {
        let mut order = self.editor_order();
        if at < order.len() {
            order.remove(at);
            self.set_order(&order);
        }
    }

    /// Sings the chorus after every verse, as most hymns go.
    pub fn order_chorus_each_verse(&mut self) {
        if let Some(order) = self.editor_song().and_then(|song| song.order_with_repeated_chorus()) {
            let tags = parse_order(&order);
            self.set_order(&tags);
        }
    }

    pub fn order_as_written(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.order.replace_text(String::new());
        }
    }

    /// Cleans the words in the editor: headings become markers and chord lines go.
    pub fn tidy_editor(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            let tidy = tidy_lyrics(&editor.lyrics.text);
            if tidy.trim() != editor.lyrics.text.trim() {
                editor.lyrics.replace_text(tidy);
            }
            editor.lyrics.focused = true;
        }
    }

    /// What the words would get from `marks::propose`, while some stanza has no mark and the
    /// strip was not dismissed for this song.
    pub fn marks_offer(&self) -> Option<marks::Proposal> {
        let editor = self.editor.as_ref().filter(|editor| !editor.marks_hidden)?;
        let proposal = marks::propose(&editor.lyrics.text);
        (proposal.pending() > 0).then_some(proposal)
    }

    pub fn hide_marks(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.marks_hidden = true;
            editor.lyrics.focused = true;
        }
    }

    /// Writes every proposed marker line into the words, as one edit that Ctrl Z undoes and
    /// with the caret before the same letter.
    pub fn apply_marks(&mut self) {
        let Some(proposal) = self.editor.as_ref().map(|editor| marks::propose(&editor.lyrics.text)) else { return };
        self.write_marks(&proposal.changes());
    }

    /// Gives the stanza that starts at byte `start` the kind a person picked for it.
    pub fn mark_stanza(&mut self, start: usize, kind: SectionKind) {
        let Some(editor) = self.editor.as_ref() else { return };
        let proposal = marks::propose(&editor.lyrics.text);
        let Some(at) = proposal.stanzas.iter().position(|stanza| stanza.start == start) else { return };
        let tag = proposal.tag_as(at, kind);
        // A stanza already marked is left to the person's own hands.
        if proposal.stanzas[at].is_unmarked() {
            self.write_marks(&proposal.change_for(at, tag).into_iter().collect::<Vec<_>>());
        }
    }

    fn write_marks(&mut self, changes: &[marks::Change]) {
        let Some(editor) = self.editor.as_mut() else { return };
        if changes.is_empty() {
            return;
        }
        let (from, to) = editor.lyrics.range();
        let written = marks::apply(&editor.lyrics.text, changes);
        let len = editor.lyrics.text.len();
        editor.lyrics.undo_group(|lyrics| {
            lyrics.set_selection(0, len);
            lyrics.insert(&written);
        });
        let (from, to) = (marks::caret_after(changes, from), marks::caret_after(changes, to));
        editor.lyrics.set_selection(from, to);
        editor.lyrics.focused = true;
    }

    pub fn insert_marker(&mut self, marker: &str) {
        let Some(editor) = self.editor.as_mut() else { return };
        let at = editor.lyrics.range().0.min(editor.lyrics.text.len());
        // A heading stands after an empty line, as stanzas are written: under a line of words
        // it brings that empty line itself.
        let before = &editor.lyrics.text[..at];
        let text = if before.is_empty() || before.ends_with("\n\n") {
            format!("{marker}\n")
        } else if before.ends_with('\n') {
            format!("\n{marker}\n")
        } else {
            format!("\n\n{marker}\n")
        };
        editor.lyrics.insert(&text);
        editor.lyrics.focused = true;
    }

    pub fn delete_song(&mut self, id: i64) {
        let Some(at) = self.song_position(id) else { return };
        let song = self.songs[at].clone();
        if self.store.delete_song(id).is_ok() {
            self.editor = None;
            self.sheet = Sheet::None;
            if matches!(self.open.as_ref().map(|open| &open.deck.source), Some(DeckSource::Song(open)) if *open == id) {
                self.open = None;
            }
            self.reload_songs();
            self.say(t::song_deleted(&song.title), Some(Undo::Song(Box::new(song))));
        }
    }

    // ---- Import ----

    /// Brings songs and Bibles over from the EasiSlides folder found on this computer.
    pub fn import_easislides(&mut self) {
        let Some(folder) = self.easislides.clone() else { return };
        let mut songs: Vec<Song> = Vec::new();
        let mut failed = 0usize;
        for file in artosia_core::import::easislides_song_files(&folder) {
            match artosia_core::import::read_easislides_songs(&file) {
                Ok(found) => songs.extend(found),
                Err(_) => failed += 1,
            }
        }
        let added = self.store.import_songs(&mut songs).unwrap_or_default().added;
        let mut bibles = 0usize;
        if let Some(own) = self.folder.clone() {
            let target = own.join("bibles");
            for file in artosia_core::import::easislides_bible_files(&folder) {
                let Some(name) = file.file_name() else { continue };
                let to = target.join(name);
                if !to.exists() && std::fs::copy(&file, &to).is_ok() {
                    bibles += 1;
                }
            }
        }
        self.list_bibles();
        self.reload_songs();
        self.settings.welcomed = true;
        self.touch_settings();
        self.sheet = Sheet::None;
        if failed > 0 {
            self.say(t::import_partial(added, failed), None);
        } else {
            self.say(t::import_done(added, bibles), None);
        }
    }

    /// Adds every OpenLyrics song in a folder or in a zip file, skipping songs the library already has.
    pub fn import_openlyrics(&mut self, picked: &std::path::Path) {
        let zip = picked.is_file();
        let nothing = if zip { t::IMPORT_ZIP_EMPTY() } else { t::IMPORT_FOLDER_EMPTY() };
        let Ok(folder) = artosia_core::import::as_folder(picked) else {
            self.say(nothing, None);
            return;
        };
        let (mut songs, failed) = artosia_core::openlyrics::read_folder(&folder);
        if zip {
            let _ = std::fs::remove_dir_all(&folder);
        }
        if songs.is_empty() && failed == 0 {
            self.say(nothing, None);
            return;
        }
        let done = self.store.import_songs(&mut songs).unwrap_or_default();
        self.reload_songs();
        self.settings.welcomed = true;
        self.touch_settings();
        // A file that failed never hides what the rest of the import did.
        let summary = t::import_summary(done.added, done.updated, done.kept);
        if failed > 0 {
            self.say(format!("{summary}. {}", t::files_unreadable(failed)), None);
        } else {
            self.say(summary, None);
        }
    }

    // ---- Announcements ----

    pub(crate) fn notices_for(item: Option<&Announcement>) -> Notices {
        let mut form = match item {
            Some(item) => Notices {
                id: item.id,
                title: TextEdit::new(item.title.clone()),
                body: TextEdit::new(item.body.clone()),
                seconds: item.seconds,
                image: item.image.clone(),
                fill: item.fill,
                passage: TextEdit::new(item.passage.clone()),
                style: item.style,
                title_colour: item.title_colour,
                calendars: item.calendars.clone(),
                window: item.window,
                pick: None,
            },
            None => Notices { seconds: 10, ..Notices::default() },
        };
        form.title.focused = true;
        form
    }

    /// Opens the announcement editor on the first announcement, or on a blank one.
    pub fn edit_announcements(&mut self) {
        // A card opened on its own is the one to edit; with all of them open, the card of the
        // ringed slide is (a tester had the ring on the second card and got the first).
        let opened = match self.open.as_ref().map(|open| (&open.deck.source, open.cursor)) {
            Some((DeckSource::Announcement(id), _)) => self.announcements.iter().find(|card| card.id == *id),
            Some((DeckSource::Announcements, ring)) => self.card_at_slide(ring),
            _ => None,
        };
        self.notices = Some(App::notices_for(opened.or(self.announcements.first())));
        self.sheet = Sheet::None;
        self.tab = Tab::Notices;
        self.close_palette();
    }

    /// Opens one of the program's windows. The announcement editor lives in its own tab and is
    /// saved and closed on the way out, so every other tab's keys work as they always did.
    pub fn open_tab(&mut self, tab: Tab) {
        if self.notices.is_some() && tab != Tab::Notices {
            self.close_announcements();
        }
        self.tab = tab;
        self.sheet = Sheet::None;
        self.close_palette();
        if tab == Tab::Notices && self.notices.is_none() {
            self.edit_announcements();
        }
    }

    /// The announcement in the form, as it would be saved; None while it has no words.
    pub fn notice_draft(&self) -> Option<Announcement> {
        let form = self.notices.as_ref()?;
        let (title, body) = (form.title.text.trim().to_string(), form.body.text.trim().to_string());
        let has_words = !title.is_empty() || !body.is_empty() || !form.image.is_empty() || form.fill != Fill::Nothing;
        let passage = if form.fill == Fill::Passage { form.passage.text.trim().to_string() } else { String::new() };
        let calendars = if form.fill == Fill::Schedule { form.calendars.clone() } else { Vec::new() };
        // Only a birthday card has a part that stands out; a card with no window reads the default one.
        let window = match form.fill {
            Fill::Schedule => Window { highlight: None, ..form.window },
            Fill::Birthdays => form.window,
            _ => Window::default(),
        };
        has_words.then_some(Announcement { id: form.id, title, body, seconds: form.seconds.clamp(2, 600), image: form.image.clone(), fill: form.fill, passage, style: form.style, title_colour: form.title_colour, calendars, window })
    }

    pub(crate) fn reload_announcements(&mut self) {
        self.announcements = self.store.announcements().unwrap_or_default();
        self.refresh_decks();
    }

    /// Saves the announcement in the form when it has words and differs from what is stored.
    pub fn save_notice(&mut self) {
        let Some(mut draft) = self.notice_draft() else { return };
        if self.announcements.contains(&draft) {
            return;
        }
        if self.store.save_announcement(&mut draft).is_ok() {
            if let Some(form) = self.notices.as_mut() {
                form.id = draft.id;
            }
            self.reload_announcements();
            self.say(t::NOTICE_SAVED(), None);
        }
    }

    /// The card before or after the one in the form, from anywhere in the editor.
    pub fn step_notice(&mut self, by: isize) {
        let Some(form) = self.notices.as_ref() else { return };
        let at = self.announcements.iter().position(|card| card.id == form.id);
        let to = match at {
            Some(at) => at as isize + by,
            // A card not saved yet stands after the last one.
            None => self.announcements.len() as isize - 1,
        };
        if let Some(card) = usize::try_from(to).ok().and_then(|to| self.announcements.get(to)) {
            let id = card.id;
            self.pick_notice(id);
        }
    }

    /// Puts another announcement in the form; the one being written is saved first.
    pub fn pick_notice(&mut self, id: i64) {
        self.save_notice();
        let found = self.announcements.iter().find(|item| item.id == id);
        self.notices = Some(App::notices_for(found));
        self.preview_card(id);
    }

    /// Puts one saved card in Preview, behind the editor, so closing the editor leaves it there.
    fn preview_card(&mut self, id: i64) {
        if let Some(deck) = self.card_deck(id) {
            self.open = Some(OpenDeck::new(deck, None, 0));
        }
    }

    /// Puts the card in the form on the projector by itself; the rest of the announcements stay out.
    pub fn show_form_notice_now(&mut self) {
        self.save_notice();
        let id = self.notices.as_ref().map(|form| form.id).unwrap_or(0);
        self.show_notice_now(id);
    }

    /// Puts one saved card on the projector by itself, or says it has nothing to show today.
    pub fn show_notice_now(&mut self, id: i64) {
        match self.card_deck(id) {
            Some(deck) => {
                self.open = Some(OpenDeck::new(deck, None, 0));
                self.show(0);
            }
            None => self.say(t::NOTICE_NOTHING_TO_SHOW(), None),
        }
    }

    /// Starts a new announcement, blank or from a ready-made one.
    pub fn new_notice(&mut self, title: &str, body: &str) {
        self.save_notice();
        let mut form = App::notices_for(None);
        form.title = TextEdit::new(title);
        form.body = TextEdit::new(body);
        form.title.focused = true;
        let end = form.title.text.len();
        form.title.set_selection(0, end);
        self.notices = Some(form);
    }

    pub fn set_notice_seconds(&mut self, seconds: u32) {
        if let Some(form) = self.notices.as_mut() {
            form.seconds = seconds;
        }
        self.save_notice();
    }

    /// Makes the card in the form a card of `template`: only where its words come from changes.
    pub fn set_notice_template(&mut self, template: Template) {
        if let Some(form) = self.notices.as_mut()
            && form.fill.template() != template
        {
            form.fill = template.fill();
        }
        self.save_notice();
    }

    pub fn set_notice_style(&mut self, style: NoticeStyle) {
        if let Some(form) = self.notices.as_mut() {
            form.style = Some(style);
        }
        self.save_notice();
    }

    /// The days the card in the form reads. Custom starts from the days it read until then.
    pub fn set_notice_range(&mut self, range: Range) {
        let today = self.today();
        if let Some(form) = self.notices.as_mut() {
            form.window.range = match range {
                Range::Custom(..) => {
                    let shown = today.shown(&form.window);
                    Range::Custom(shown.from, shown.to)
                }
                other => other,
            };
            form.pick = None;
        }
        self.save_notice();
    }

    /// The part of the days that stands out; Custom starts on the first week of the days read.
    pub fn set_notice_highlight(&mut self, highlight: Option<Range>) {
        let today = self.today();
        if let Some(form) = self.notices.as_mut() {
            form.window.highlight = match highlight {
                Some(Range::Custom(..)) => {
                    let shown = today.shown(&form.window);
                    Some(Range::Custom(shown.from, shown.to.min(shown.from.plus_days(6))))
                }
                other => other,
            };
            form.pick = None;
        }
        self.save_notice();
    }

    /// Opens the month picker under one date of the window, or closes it when it is that one's.
    pub fn notice_open_pick(&mut self, edge: Edge) {
        let today = self.today();
        let Some(form) = self.notices.as_mut() else { return };
        if form.pick.is_some_and(|(open, _)| open == edge) {
            form.pick = None;
            return;
        }
        let shown = today.shown(&form.window);
        let own = match (edge, form.window.range, form.window.highlight) {
            (Edge::RangeFrom, ..) => shown.from,
            (Edge::RangeTo, ..) => shown.to,
            (Edge::HighlightFrom, _, Some(Range::Custom(from, to))) => from.min(to),
            (Edge::HighlightTo, _, Some(Range::Custom(from, to))) => from.max(to),
            _ => shown.from,
        };
        form.pick = Some((edge, own.first_of_month()));
    }

    /// The open month picker shows the month `by` months on.
    pub fn notice_pick_month(&mut self, by: i32) {
        if let Some((_, shown)) = self.notices.as_mut().and_then(|form| form.pick.as_mut()) {
            *shown = shown.plus_months(by);
        }
    }

    /// Sets the date the open picker is for; the other end of the days follows when they would cross.
    pub fn notice_pick_date(&mut self, date: Date) {
        let Some(form) = self.notices.as_mut() else { return };
        let Some((edge, _)) = form.pick.take() else { return };
        let ends = |range: Range, from: bool| match range {
            Range::Custom(a, b) => {
                let (a, b) = (a.min(b), a.max(b));
                Range::Custom(if from { date } else { a.min(date) }, if from { b.max(date) } else { date })
            }
            other => other,
        };
        match edge {
            Edge::RangeFrom | Edge::RangeTo => form.window.range = ends(form.window.range, edge == Edge::RangeFrom),
            Edge::HighlightFrom | Edge::HighlightTo => form.window.highlight = form.window.highlight.map(|range| ends(range, edge == Edge::HighlightFrom)),
        }
        self.save_notice();
    }

    /// Takes a calendar in or out of those a schedule card reads. The last one stays: a card
    /// naming none would read them all.
    pub fn toggle_notice_calendar(&mut self, id: i64) {
        let all: Vec<i64> = self.calendars.iter().map(|calendar| calendar.id).collect();
        if let Some(form) = self.notices.as_mut() {
            let mut chosen = if form.calendars.is_empty() { all.clone() } else { form.calendars.clone() };
            match chosen.iter().position(|own| *own == id) {
                Some(at) if chosen.len() > 1 => {
                    chosen.remove(at);
                }
                Some(_) => {}
                None => chosen.push(id),
            }
            form.calendars = if chosen.len() >= all.len() && all.iter().all(|own| chosen.contains(own)) { Vec::new() } else { chosen };
        }
        self.save_notice();
    }

    /// Sets or clears the picture of the announcement in the form.
    pub fn set_notice_image(&mut self, path: Option<&std::path::Path>) {
        if let Some(form) = self.notices.as_mut() {
            form.image = path.map(|path| path.to_string_lossy().to_string()).unwrap_or_default();
        }
        self.save_notice();
    }

    /// The slide an announcement starts on in the deck; None when it shows nothing today.
    pub fn announcement_slide_index(&self, announcement: usize) -> Option<usize> {
        self.announcement_pages().iter().position(|(at, _)| *at == announcement)
    }

    /// Where Artosia keeps pictures it made itself, such as slides brought in from PowerPoint.
    pub(crate) fn media_folder(&self) -> std::path::PathBuf {
        let base = self.folder.clone().unwrap_or_else(|| std::env::temp_dir().join("artosia-practice"));
        base.join("media")
    }

    /// Brings a PowerPoint in as announcements at once, as a start from a script wants.
    pub fn import_pptx(&mut self, path: &std::path::Path) {
        if let Some((added, lost)) = self.redraw_cards(path) {
            self.say(t::pptx_imported(added, lost), None);
        }
    }

    /// Redraws a PowerPoint as announcements: one per slide, tables as columns, pictures kept.
    /// The cards made and the pictures that did not come across; None when the file would not read.
    pub(crate) fn redraw_cards(&mut self, path: &std::path::Path) -> Option<(usize, usize)> {
        let deck = match artosia_office::Pptx::open(path) {
            Ok(deck) => deck,
            Err(_) => {
                self.say(t::could_not_read(t::WHY_UNREADABLE()), None);
                return None;
            }
        };
        self.save_notice();
        let folder = self.media_folder();
        let stem = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_else(|| "slides".to_string());
        let (mut added, mut lost) = (0usize, 0usize);
        for draft in artosia_office::to_announcements(&deck) {
            let mut image = String::new();
            if let Some(picture) = &draft.picture {
                // Windows metafiles and the like are redrawn as PNG, which every screen can show.
                match picture.drawable(1920) {
                    Ok((format, bytes)) => {
                        let extension = if matches!(format, artosia_office::ImageFormat::Jpeg) { "jpg" } else { "png" };
                        let file = folder.join(format!("{stem}-{}.{extension}", draft.slide));
                        if std::fs::create_dir_all(&folder).is_ok() && std::fs::write(&file, bytes).is_ok() {
                            image = file.to_string_lossy().to_string();
                        } else {
                            lost += 1;
                        }
                    }
                    Err(_) => lost += 1,
                }
            }
            lost += draft.dropped_pictures;
            if draft.title.trim().is_empty() && draft.body.trim().is_empty() && image.is_empty() {
                continue;
            }
            let mut item = Announcement { id: 0, title: draft.title, body: draft.body, seconds: 10, image, fill: Fill::Nothing, passage: String::new(), ..Announcement::default() };
            if self.store.save_announcement(&mut item).is_ok() {
                added += 1;
            }
        }
        self.show_new_cards();
        Some((added, lost))
    }

    pub fn duplicate_notice(&mut self, id: i64) {
        self.save_notice();
        let Some(mut copy) = self.announcements.iter().find(|item| item.id == id).cloned() else { return };
        copy.id = 0;
        if self.store.save_announcement(&mut copy).is_ok() {
            // The copy sits right under the one it was made from.
            let mut ids: Vec<i64> = self.announcements.iter().map(|item| item.id).collect();
            let at = ids.iter().position(|own| *own == id).map(|at| at + 1).unwrap_or(ids.len());
            ids.insert(at, copy.id);
            let _ = self.store.order_announcements(&ids);
            self.reload_announcements();
            self.notices = Some(App::notices_for(self.announcements.iter().find(|item| item.id == copy.id)));
        }
    }

    pub fn move_notice(&mut self, id: i64, by: isize) {
        self.save_notice();
        let mut ids: Vec<i64> = self.announcements.iter().map(|item| item.id).collect();
        let Some(at) = ids.iter().position(|own| *own == id) else { return };
        let to = at as isize + by;
        if to < 0 || to as usize >= ids.len() {
            return;
        }
        ids.swap(at, to as usize);
        if self.store.order_announcements(&ids).is_ok() {
            self.reload_announcements();
        }
    }

    pub fn delete_announcement(&mut self, id: i64) {
        let Some(at) = self.announcements.iter().position(|item| item.id == id) else { return };
        let item = self.announcements[at].clone();
        if self.store.delete_announcement(id).is_ok() {
            self.reload_announcements();
            if self.notices.as_ref().is_some_and(|form| form.id == id) {
                // The form moves to the neighbour, so Delete can be pressed again.
                let next = self.announcements.get(at).or(self.announcements.last());
                self.notices = Some(App::notices_for(next));
            }
            self.say(t::announcement_deleted(&item.title), Some(Undo::Announcement(item)));
        }
    }

    /// Leaves the announcement editor; what was being written is saved.
    pub fn close_announcements(&mut self) {
        self.save_notice();
        // The Templates tab applies a style to the card that was open last.
        if let Some(form) = self.notices.as_ref() {
            self.templates.card = form.id;
        }
        self.notices = None;
        if self.tab == Tab::Notices {
            self.tab = Tab::Plan;
        }
    }

    pub fn open_announcements(&mut self) {
        self.save_notice();
        let deck = self.announcements_deck();
        self.open_deck(deck, None, 0);
        self.sheet = Sheet::None;
    }

    /// Fills the practice library for a headless render of the announcement editor: "passage",
    /// "passage-no-bible" or "birthdays". The names are made up and fall on today's date.
    pub fn demo_notices(&mut self, kind: &str) {
        let today = self.today();
        let person = |name: &str, day: u8| Person { name: name.into(), day: day.clamp(1, 28), month: today.month, ..Person::default() };
        self.people = vec![person("Sample Alpha", today.day), person("Sample Beta", today.day.saturating_add(2)), person("Sample Gamma", today.day.saturating_add(3))];
        let mut cards = vec![
            // The cards take their titles from the templates, so a render reads in the window's language.
            Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[3].0.into(), fill: Fill::Passage, passage: "Psalm 23:1-3".into(), seconds: 12, ..Announcement::default() },
            Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[5].0.into(), fill: Fill::Birthdays, seconds: 12, ..Announcement::default() },
        ];
        if kind == "birthdays" {
            cards.reverse();
        }
        if kind == "offerings" || kind == "transfer" {
            // The offering card first, so a render shows one of these cards.
            cards.push(Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[4].0.into(), fill: if kind == "offerings" { Fill::Offerings } else { Fill::Transfer }, seconds: 12, ..Announcement::default() });
        }
        if kind == "schedule" {
            cards.push(Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[crate::calendar::SCHEDULE_TEMPLATE].0.into(), fill: Fill::Schedule, seconds: 12, ..Announcement::default() });
        }
        for card in &mut cards {
            let _ = self.store.save_announcement(card);
        }
        if kind == "passage-no-bible" {
            // A code no file has, so nothing starts loading behind the render.
            self.bibles.clear();
            self.settings.bible = "none".into();
        }
        self.reload_announcements();
        // The editor opens on the card the scene is about.
        let wanted = match kind {
            "birthdays" => Fill::Birthdays,
            "schedule" => Fill::Schedule,
            "offerings" => Fill::Offerings,
            "transfer" => Fill::Transfer,
            _ => Fill::Passage,
        };
        let id = self.announcements.iter().find(|card| card.fill == wanted).map(|card| card.id).unwrap_or(0);
        self.edit_announcements();
        self.pick_notice(id);
    }

    /// Fills the practice library for a render of the birthday card of this month with the past week
    /// highlighted: thirty made-up names spread over the days of the month.
    pub fn demo_birthdays_highlight(&mut self, style: Option<NoticeStyle>) {
        let today = self.today().date_of();
        let days = usize::from(Date::month_days(today.year, today.month));
        let first = ["Alfa", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Teta", "Iota", "Kapa"];
        self.people = (0..30)
            .map(|at| {
                let last = (at * 3 + at / 10 + 1) % 10;
                let last = if last == at % 10 { (last + 1) % 10 } else { last };
                Person { name: format!("{} {}", first[at % 10], first[last]), day: (1 + at * days / 30) as u8, month: today.month, ..Person::default() }
            })
            .collect();
        let mut card = Announcement {
            title: t::ANNOUNCEMENT_TEMPLATES()[5].0.into(),
            fill: Fill::Birthdays,
            seconds: 12,
            style,
            window: Window { range: Range::Month(0), highlight: Some(Range::Past(7)) },
            ..Announcement::default()
        };
        let _ = self.store.save_announcement(&mut card);
        self.reload_announcements();
        self.edit_announcements();
        self.pick_notice(card.id);
    }

    /// Whether the announcements are a row of today's plan already.
    pub fn announcements_in_plan(&self) -> bool {
        self.plan_has(&DeckSource::Announcements)
    }

    pub fn add_announcements_to_plan(&mut self) {
        if !self.plan_has(&DeckSource::Announcements) {
            self.plan.push(PlanEntry::Announcements);
            self.touch_plan();
        }
        self.say(t::added_to_plan(t::CMD_ANNOUNCEMENTS()), None);
    }

    /// Starts or stops the announcements advancing by themselves on the projector.
    pub fn toggle_loop(&mut self) {
        if self.looping {
            self.looping = false;
            return;
        }
        let on_air = self.live.as_ref().is_some_and(|live| live.deck.source == DeckSource::Announcements);
        if !on_air {
            self.open_announcements();
            let cursor = self.open.as_ref().map(|open| open.cursor).unwrap_or(0);
            self.show(cursor);
        }
        self.looping = self.live.as_ref().is_some_and(|live| live.deck.source == DeckSource::Announcements);
        self.loop_at = (0, 0.0);
    }

    // ---- Streaming ----

    /// Starts sending the projector's picture to the address in the dialog, or stops it.
    pub fn toggle_stream(&mut self) {
        if let Some(stream) = self.stream.as_mut() {
            stream.stop();
            return;
        }
        self.settings.stream_address = self.stream_address.text.trim().to_string();
        self.settings.stream_key = self.stream_key.text.trim().to_string();
        self.touch_settings();
        let (address, key) = (self.settings.stream_address.clone(), self.settings.stream_key.clone());
        // A service gives an address and a key; the two joined are where the stream goes.
        let target = match (address.is_empty(), key.is_empty()) {
            (true, _) => String::new(),
            (false, true) => address,
            (false, false) => format!("{}/{}", address.trim_end_matches('/'), key),
        };
        let record = self.settings.stream_record.then(|| {
            let folder = crate::data::folder().map(|folder| folder.join("recordings")).unwrap_or_else(std::env::temp_dir);
            let _ = std::fs::create_dir_all(&folder);
            let time = rune::LocalTime::at((self.now * 1000.0) as i64);
            folder.join(format!("service-{:04}-{:02}-{:02}-{:02}{:02}.mp4", time.year, time.month, time.day, time.hour, time.minute))
        });
        if target.is_empty() && record.is_none() {
            self.stream_problem = Some(t::STREAM_NOTHING().to_string());
            return;
        }
        match crate::stream::Stream::start(crate::stream::StreamOptions { address: target, record }, self.waker.clone()) {
            Ok(stream) => {
                self.stream = Some(stream);
                self.stream_problem = None;
            }
            Err(problem) => self.stream_problem = Some(problem),
        }
    }

    // ---- Listening ----

    /// Opens the listening dialog with the microphones this computer has now. A headless run lists
    /// made-up ones, so a recorded clip or a render never names this computer's devices.
    pub fn open_listen(&mut self) {
        self.microphones = if rune::is_headless() { demo_microphones() } else { artosia_listen::devices() };
        self.check_model();
        self.refresh_recordings();
        self.listen_part = 0;
        self.sheet = Sheet::Listen;
    }

    /// Starts or stops listening to the room. Words that are heard become suggestions; nothing
    /// is ever shown on the projector because of them.
    pub fn set_listening(&mut self, on: bool) {
        if let Some(listener) = self.listener.take() {
            // Stopping waits for a recognition in progress, so it happens off the window's thread.
            std::thread::spawn(move || drop(listener));
        }
        self.end_sitting();
        self.listen_problem = None;
        self.hearing.clear();
        self.listen_level.store(0, std::sync::atomic::Ordering::Relaxed);
        self.tempo = None;
        (self.tempo_count, self.tempo_wanted, self.tempo_half) = (TempoCount::AsHeard, None, None);
        if let Ok(mut slot) = self.listen_tempo_slot.lock() {
            *slot = None;
        }
        if !on {
            return;
        }
        self.read_speed = (0, 0.0);
        // Practice has no library folder and so no model: the level and the tempo still work.
        let model = self.model_path().unwrap_or_else(|| std::env::temp_dir().join("no-speech-model.bin"));
        let device = Some(self.settings.listen_device.clone()).filter(|name| !name.is_empty());
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        let (level, shown) = (self.listen_level.clone(), self.meter_shown.clone());
        let tempo_slot = self.listen_tempo_slot.clone();
        // With recordings switched on, a sitting opens and the listener writes its phrases there.
        let keep = self.begin_sitting();
        let started = artosia_listen::Listener::start_keeping(device.as_deref(), &model, &self.settings.listen_language, keep.as_deref(), move |heard| {
            let wake = match heard {
                artosia_listen::Heard::Level(now) => {
                    level.store(now.clamp(0.0, 1.0).to_bits(), std::sync::atomic::Ordering::Relaxed);
                    // The meter only needs drawing while someone can see it.
                    shown.load(std::sync::atomic::Ordering::Relaxed)
                }
                artosia_listen::Heard::Tempo(tempo) => {
                    // Only the newest report matters, so it is a slot and not a queue.
                    if let Ok(mut slot) = tempo_slot.lock() {
                        *slot = Some((tempo, std::time::Instant::now()));
                    }
                    shown.load(std::sync::atomic::Ordering::Relaxed)
                }
                other => {
                    if let Ok(mut inbox) = inbox.lock() {
                        inbox.push(Arrived::Heard(other));
                    }
                    true
                }
            };
            if let (true, Some(wake)) = (wake, &waker) {
                wake();
            }
        });
        match started {
            Ok(listener) => self.listener = Some(listener),
            Err(error) => {
                self.end_sitting();
                self.listen_problem = Some(crate::why::listen_error(&error));
            }
        }
    }

    /// Takes the newest tempo report from the listening thread. True when it changed what shows.
    fn take_tempo(&mut self, clock: f64) -> bool {
        let fresh = self.listen_tempo_slot.lock().ok().and_then(|mut slot| slot.take());
        let Some((tempo, arrived)) = fresh else {
            return false;
        };
        let before = self.tempo_shown().map(|bpm| bpm.round() as i32);
        self.tempo = tempo.map(|tempo| (tempo, clock - arrived.elapsed().as_secs_f64()));
        if let Some((tempo, at)) = self.tempo {
            // A person said what the tempo is. The tracker changes its own count within a song
            // (a ballad read 60, then 120, then 60), so the count that reaches that tempo is
            // taken anew for every reading; when none does, it is another song.
            if let Some(wanted) = self.tempo_wanted {
                let reach = |count: TempoCount| (tempo.bpm * count.times() - wanted).abs();
                let nearest = [TempoCount::Half, TempoCount::AsHeard, TempoCount::Double].into_iter().min_by(|a, b| reach(*a).total_cmp(&reach(*b))).unwrap_or(TempoCount::AsHeard);
                if reach(nearest) <= TEMPO_SAME * wanted {
                    self.tempo_count = nearest;
                } else {
                    (self.tempo_count, self.tempo_wanted) = (TempoCount::AsHeard, None);
                }
            }
            // Which of every two beats the halved count falls on.
            let last_beat = at - f64::from(tempo.since_beat);
            let odd = self.tempo_half.is_some_and(|(before, odd)| odd ^ ((((last_beat - before) / f64::from(tempo.period)).round() as i64).rem_euclid(2) == 1));
            self.tempo_half = Some((last_beat, odd));
        }
        before != self.tempo_shown().map(|bpm| bpm.round() as i32)
    }

    /// Counts the tempo at half or double what the sound gives, and keeps the tempo that makes
    /// until the song changes. As heard lets the reading stand again.
    pub fn set_tempo_count(&mut self, count: TempoCount) {
        self.tempo_count = count;
        self.tempo_wanted = match (count, self.tempo) {
            (TempoCount::AsHeard, _) | (_, None) => None,
            (_, Some((tempo, _))) => Some(tempo.bpm * count.times()),
        };
        self.recording_count();
    }

    /// The tempo as it is counted: the tracker's reading, halved or doubled by a press.
    pub fn tempo_shown(&self) -> Option<f32> {
        self.tempo.map(|(tempo, _)| tempo.bpm * self.tempo_count.times())
    }

    /// The beat as it is `now` (the app clock), counted as a person said, or none when there is
    /// no recent steady pulse.
    pub fn beat_phase(&self, now: f64) -> Option<f32> {
        let (tempo, at) = self.tempo?;
        let after = now - at;
        if after >= TEMPO_STALE {
            return None;
        }
        let phase = tempo.phase_after(after as f32);
        Some(match self.tempo_count {
            TempoCount::AsHeard => phase,
            TempoCount::Double => (phase * 2.0).rem_euclid(1.0),
            TempoCount::Half => {
                let beats = ((tempo.since_beat + after as f32) / tempo.period).floor() as i64;
                let odd = self.tempo_half.is_some_and(|(_, odd)| odd) ^ (beats.rem_euclid(2) == 1);
                (phase + f32::from(u8::from(odd))) / 2.0
            }
        })
    }

    /// Something is pulsing on the beat and the window should redraw often.
    fn beat_moving(&self) -> bool {
        let shown = self.sheet == Sheet::Listen || self.settings.beat_blink;
        shown && !self.motion_reduced.load(std::sync::atomic::Ordering::Relaxed) && self.beat_phase(self.clock).is_some()
    }

    /// How much of its full strength the red Live marker is drawn with at app clock `now`: 1.0 on
    /// the beat, down to 0.8 between beats, so it breathes and never flashes. 1.0 when the switch
    /// is off, the system asks for reduced motion, or the beat is not known.
    pub fn beat_glow(&self, now: f64) -> f32 {
        if !self.settings.beat_blink || self.motion_reduced.load(std::sync::atomic::Ordering::Relaxed) {
            return 1.0;
        }
        match self.beat_phase(now) {
            Some(phase) => 0.8 + 0.2 * beat_pulse(phase),
            None => 1.0,
        }
    }

    /// Turns a sentence that was heard into what it may have named: a passage, or songs that
    /// hold its words.
    pub(crate) fn heard_final(&mut self, text: String) {
        self.note_heard(&text);
        let mut hits: Vec<HeardHit> = Vec::new();
        if let Some(bible) = self.bible()
            && let Some(guess) = bible.guess(&text).first() {
                hits.push(HeardHit::Passage(guess.reference, bible.format(&guess.reference)));
            }
        hits.extend(self.index.search(&text, 3).into_iter().filter(|hit| hit.complete).map(|hit| HeardHit::Song(hit.song)));
        if !hits.is_empty() {
            self.heard_hits = hits;
        }
        self.heard.push(text);
        let extra = self.heard.len().saturating_sub(3);
        self.heard.drain(..extra);
    }

    /// F4: puts the first suggestion in the Preview pane, where Enter shows it: what the listener
    /// heard, else what usually follows the live song. It never goes to the projector by itself:
    /// a misheard word must not reach the room.
    pub fn open_last_heard(&mut self) {
        if let Some(first) = crate::suggest::suggestions(self).into_iter().next() {
            self.take_offer(&first);
            return;
        }
        match self.heard_hits.first().cloned() {
            Some(hit) => self.open_heard(&hit),
            None => self.say(t::HEARD_NOTHING(), None),
        }
    }

    pub fn open_heard(&mut self, hit: &HeardHit) {
        match hit {
            HeardHit::Song(song) => self.open_song(*song, None),
            HeardHit::Passage(reference, _) => self.open_passage(reference),
        }
        self.sheet = Sheet::None;
    }

    // ---- Look ----

    pub fn set_look(&mut self, look: Look) {
        let lines = self.look().max_lines;
        let changed_lines = look.max_lines != lines;
        if let Background::Camera(name) = &look.background
            && name.contains("://")
            && self.camera_address.text != *name
        {
            self.camera_address.replace_text(name.clone());
        }
        if self.plan.look.is_some() {
            self.plan.look = serde_json::to_value(&look).ok();
            self.touch_plan();
        } else {
            self.settings.look = look;
            self.touch_settings();
        }
        if changed_lines {
            self.refresh_decks();
        }
    }

    pub fn set_background_file(&mut self, path: &std::path::Path) {
        let text = path.to_string_lossy().to_string();
        let Some(background) = crate::run::background_of(&text) else {
            self.say(t::BACKGROUND_UNKNOWN(), None);
            return;
        };
        let mut look = self.look();
        look.background = background;
        look.shadow = true;
        look.name = t::LOOK_CUSTOM_ID.to_string();
        self.set_look(look);
    }

    pub fn set_bible(&mut self, code: &str) {
        self.settings.bible = code.to_string();
        self.want_bible(code);
        self.touch_settings();
        self.refresh_decks();
        if self.palette.open {
            self.search();
        }
    }

    pub fn set_second_bible(&mut self, code: &str) {
        self.settings.second_bible = code.to_string();
        self.want_bible(code);
        self.touch_settings();
        self.refresh_decks();
    }

    // ---- Remote ----

    /// The person's press of the Phone switch. Switched on, a missing firewall rule is asked
    /// for first (`firewall.rs`) and the remote starts when that has answered.
    pub fn set_remote(&mut self, on: bool) {
        self.settings.remote_on = on;
        self.touch_settings();
        if !on {
            self.remote_round += 1;
            self.firewall = Standing::Idle;
            self.remote = None;
            return;
        }
        if self.remote.is_some() || self.firewall.waiting() {
            return;
        }
        self.remote_pin = artosia_remote::new_pin();
        self.remote_problem = None;
        self.remote_round += 1;
        if self.firewall_applies() {
            self.firewall = Standing::Checking;
            firewall::spawn(self.inbox.clone(), self.waker.clone(), self.remote_round, REMOTE_PORT, true);
        } else {
            self.start_remote();
        }
    }

    /// The remote was left on: start it as it always was, and only look whether its firewall
    /// rule is there. A Windows box nobody asked for while the room is being set up is worse
    /// than a warning line in the Phone dialog.
    pub fn resume_remote(&mut self) {
        self.remote_pin = artosia_remote::new_pin();
        self.start_remote();
        if self.firewall_applies() {
            self.remote_round += 1;
            firewall::spawn(self.inbox.clone(), self.waker.clone(), self.remote_round, REMOTE_PORT, false);
        }
    }

    /// Windows only, in a real window on a real library (so never practising or headless), and
    /// the remote listens on the network.
    fn firewall_applies(&self) -> bool {
        firewall::applies(cfg!(windows) && !cfg!(test), self.folder.is_some() && !rune::is_headless(), false)
    }

    /// What the firewall step reported for a press of the switch.
    fn firewall_answered(&mut self, round: u64, progress: firewall::Progress) {
        if round != self.remote_round {
            return;
        }
        match progress {
            firewall::Progress::Asking => self.firewall = Standing::Asking,
            firewall::Progress::Done(outcome) => {
                let waiting = self.firewall.waiting();
                self.firewall = if outcome == firewall::Outcome::Refused { Standing::Refused } else { Standing::Idle };
                // The remote starts whether or not the rule was added.
                if waiting && self.settings.remote_on && self.remote.is_none() {
                    self.start_remote();
                }
            }
        }
    }

    fn start_remote(&mut self) {
        let (tx, waker) = (self.remote_tx.clone(), self.waker.clone());
        let started = RemoteServer::start(RemoteConfig { port: REMOTE_PORT, pin: self.remote_pin.clone(), local_only: false }, move |command| {
            let _ = tx.send(command);
            if let Some(wake) = &waker {
                wake();
            }
        });
        match started {
            Ok(server) => {
                self.remote = Some(server);
                self.remote_problem = None;
                self.remote_qr = self.remote_url().and_then(|url| qr_picture(&url));
            }
            Err(error) => self.remote_problem = Some(error.to_string()),
        }
    }

    pub fn remote_url(&self) -> Option<String> {
        let server = self.remote.as_ref()?;
        server.urls().first().map(|url| format!("{url}?pin={}", self.remote_pin))
    }

    fn remote_state(&self) -> RemoteState {
        let live_item = self.live.as_ref().and_then(|live| live.item);
        let plan = self
            .plan
            .items
            .iter()
            .map(|item| RemotePlanItem {
                id: item.id,
                title: self.entry_title(&item.entry),
                kind: match item.entry {
                    PlanEntry::Song { .. } => "song",
                    PlanEntry::Passage { .. } => "passage",
                    PlanEntry::Announcements => "announcements",
                    PlanEntry::Heading { .. } => "heading",
                    PlanEntry::Video { .. } => "video",
                    PlanEntry::Audio { .. } => "audio",
                    PlanEntry::Scene { .. } => "scene",
                }
                .to_string(),
                live: live_item == Some(item.id),
            })
            .collect();
        let mut state = RemoteState {
            text_hidden: self.text_hidden,
            black: self.black,
            off: !self.on_air(),
            logo_on: self.logo_on,
            logo_ready: !self.settings.logo.is_empty(),
            plan,
            hits_for: self.remote_hits.0.clone(),
            hits: self.remote_hits.1.clone(),
            message: if self.message_on { self.message.text.clone() } else { String::new() },
            countdown_ends_at: self.countdown.as_ref().map(|timer| timer.ends_at),
            ..RemoteState::default()
        };
        if let Some(live) = &self.live {
            state.item_title = live.deck.title.clone();
            state.slides = live.deck.slides.iter().map(|slide| RemoteSlide { label: slide.label.clone(), text: slide.text.clone(), more: slide.more.iter().map(|voice| voice.text.clone()).collect() }).collect();
            state.live_index = Some(live.index);
            state.next_title = self.next_item_title().unwrap_or_default();
            // One entry each time a section is sung, so a repeated chorus appears twice.
            for index in section_starts(&live.deck) {
                let slide = &live.deck.slides[index];
                if let Some(tag) = slide.tag {
                    state.sections.push(RemoteSection { label: slide.label.clone(), short: slide.short.clone(), group: artosia_core::deck::tag_group(tag), slide: index });
                }
            }
        }
        state
    }

    /// Answers a search typed on a phone with a few songs and passages.
    fn remote_search(&mut self, query: &str) {
        let mut hits: Vec<RemoteHit> = Vec::new();
        if let Some(bible) = self.bible() {
            for guess in bible.guess(query).iter().take(2) {
                let r = guess.reference;
                let text = bible.verses(&r).first().map(|verse| verse.text.to_string()).unwrap_or_default();
                hits.push(RemoteHit {
                    id: format!("ref:{}:{}:{}:{}:{}", r.book, r.chapter, r.verse, r.end_chapter, r.end_verse),
                    title: bible.format(&r),
                    detail: text,
                    kind: "passage".to_string(),
                });
            }
        }
        let planned: Vec<usize> = self
            .plan
            .items
            .iter()
            .filter_map(|item| match &item.entry {
                PlanEntry::Song { song_id, .. } => self.song_position(*song_id),
                _ => None,
            })
            .collect();
        for found in self.index.search_with(query, 12, &|song| planned.contains(&song)) {
            let Some(song) = self.songs.get(found.song) else { continue };
            hits.push(RemoteHit {
                id: format!("song:{}", song.id),
                title: song.title.clone(),
                detail: found.line.map(|line| line.text).or_else(|| song.first_line()).unwrap_or_default(),
                kind: "song".to_string(),
            });
        }
        self.remote_hits = (query.to_string(), hits);
    }

    /// Opens what a phone's search result stands for. False when it no longer exists.
    fn open_remote_hit(&mut self, id: &str) -> bool {
        if let Some(song) = id.strip_prefix("song:").and_then(|id| id.parse::<i64>().ok()) {
            let Some(at) = self.song_position(song) else { return false };
            self.open_song(at, None);
            return true;
        }
        let Some(numbers) = id.strip_prefix("ref:") else { return false };
        let parts: Vec<u16> = numbers.split(':').filter_map(|part| part.parse().ok()).collect();
        let [book, chapter, verse, end_chapter, end_verse] = parts[..] else { return false };
        let reference = Reference { book: book as u8, chapter, verse, end_chapter, end_verse };
        self.open_passage(&reference);
        self.open.as_ref().is_some_and(|open| matches!(&open.deck.source, DeckSource::Passage { reference: shown, .. } if *shown == reference))
    }

    // ---- Screens ----

    /// Saves where the projector and the stage view go and tells the output, which moves
    /// its windows without restarting.
    pub fn settings_screens(&mut self, options: OutputOptions) {
        self.settings.audience_screen = options.audience;
        self.settings.stage_screen = options.stage;
        self.settings.stage_window = options.stage_window;
        self.settings.projector_here = options.here;
        // The Live switch is for this sitting only; it is never saved as the screen's setting.
        if !self.off_air {
            self.settings.projector_off = options.off;
        }
        self.touch_settings();
        self.identify = false;
        self.link.place(options);
    }

    /// Turns the projector on or off, wherever it is.
    pub fn set_projector(&mut self, on: bool) {
        let mut options = self.output_options();
        options.off = !on;
        self.settings_screens(options);
    }

    // ---- The picture on the wall ----

    /// Moves the picked corner of the projector picture by a step, as fractions of the screen.
    /// A projector that sits off to one side throws a slanted picture; this squares it up.
    pub fn nudge_corner(&mut self, dx: f32, dy: f32) {
        let corner = &mut self.settings.warp.corners[self.warp_corner.min(3)];
        corner[0] = (corner[0] + dx).clamp(-0.25, 1.25);
        corner[1] = (corner[1] + dy).clamp(-0.25, 1.25);
        self.touch_settings();
    }

    /// Changes brightness (0), contrast (1) or colour strength (2) of the projector picture.
    pub fn nudge_colour(&mut self, which: usize, by: f32) {
        let warp = &mut self.settings.warp;
        match which {
            0 => warp.brightness = ((warp.brightness + by) * 100.0).round().clamp(-50.0, 50.0) / 100.0,
            1 => warp.contrast = ((warp.contrast + by) * 100.0).round().clamp(50.0, 200.0) / 100.0,
            _ => warp.saturation = ((warp.saturation + by) * 100.0).round().clamp(0.0, 200.0) / 100.0,
        }
        self.touch_settings();
    }

    pub fn reset_warp(&mut self) {
        self.settings.warp = artosia_slides::frame::Warp::default();
        self.touch_settings();
    }

    /// Switches the window to the language with this code, section names on the slides included.
    pub fn set_language(&mut self, code: &str) {
        // The practice announcements nobody has changed follow the window's language.
        let was = crate::data::practice_announcements();
        let untouched = self.folder.is_none() && self.announcements.len() == was.len() && self.announcements.iter().zip(&was).all(|(card, was)| card.title == was.title && card.body == was.body);
        if !t::set_language(code) {
            return;
        }
        if untouched {
            for (mut card, fresh) in self.announcements.clone().into_iter().zip(crate::data::practice_announcements()) {
                (card.title, card.body) = (fresh.title, fresh.body);
                let _ = self.store.save_announcement(&mut card);
            }
            self.reload_announcements();
        }
        self.settings.language = code.to_string();
        self.touch_settings();
        self.refresh_decks();
        if self.palette.open {
            self.search();
        }
    }

    /// The network camera's field shows the address the look holds.
    pub(crate) fn sync_camera_address(&mut self) {
        if let Background::Camera(name) = &self.look().background
            && name.contains("://")
        {
            self.camera_address.replace_text(name.clone());
        }
    }

    /// Puts the camera at the typed address behind the words. What is typed has to be an
    /// address: a word alone would be looked for among this computer's cameras.
    pub fn use_camera_address(&mut self) {
        let address = self.camera_address.text.trim().to_string();
        if address.contains("://") {
            self.set_camera(&address);
        }
    }

    /// Puts a camera behind the words, as a video background is.
    pub fn set_camera(&mut self, name: &str) {
        let mut look = self.look();
        look.background = Background::Camera(name.to_string());
        look.shadow = true;
        look.name = t::LOOK_CUSTOM_ID.to_string();
        self.set_look(look);
    }

    // ---- Passages ----

    /// Adds the verse after a passage to it, as a speaker reading on does.
    pub fn extend_passage(&mut self) {
        let Some(open) = self.open.as_ref() else { return };
        let DeckSource::Passage { reference, .. } = &open.deck.source else { return };
        let Some(longer) = self.bible().and_then(|bible| bible.extended(reference)) else { return };
        let (item, old) = (open.item, open.deck.source.clone());
        let Some(deck) = self.deck_of_passage(&longer) else { return };
        if let Some(live) = self.live.as_mut()
            && live.deck.source == old && live.item == item {
                live.deck = deck.clone();
            }
        if let Some(id) = item {
            let title = deck.title.clone();
            if let Some(PlanEntry::Passage { reference, title: shown, .. }) = self.plan.items.iter_mut().find(|i| i.id == id).map(|i| &mut i.entry) {
                *reference = longer;
                *shown = title;
                self.plan_dirty = true;
            }
        }
        if let Some(open) = self.open.as_mut() {
            open.deck = deck;
        }
    }

    // ---- Between frames ----

    /// Runs before every frame: takes what workers finished, keeps the screens and the phone
    /// remote in step, and saves what changed. True when something on screen changed.
    pub fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        self.clock = clock;
        let now = unix_now();
        if now.floor() != self.now.floor() {
            // The clock and any countdown tick once a second.
            changed = self.countdown.is_some() || (now / 60.0).floor() != (self.now / 60.0).floor();
        }
        self.now = now;
        changed |= self.hearing_aged();
        let mut wake = if self.countdown.is_some() { 0.25 } else { 1.0 };
        let was = self.live.as_ref().map(|live| live.index);
        if let Some(until) = self.loop_step(clock) {
            wake = until.min(wake);
            changed |= was != self.live.as_ref().map(|live| live.index);
        }
        self.meter_shown.store(self.listener.is_some() && (self.sheet == Sheet::Listen || self.settings.beat_blink), std::sync::atomic::Ordering::Relaxed);
        changed |= self.take_tempo(clock);
        changed |= self.watch_pptx(&mut wake);
        changed |= crate::ui::tour::reveal_ring(self.lesson.is_some(), tx);
        self.pump_web(&mut wake);
        self.note_recordings();
        self.purge_recordings();
        if self.beat_moving() {
            // The dot and the Live marker pulse, which needs a frame every so often.
            changed = true;
            wake = wake.min(BEAT_FRAME);
        }
        if let Some(at) = self.editor.as_ref().and_then(|editor| editor.ask_at) {
            if clock >= at { self.ask_web() } else { wake = (at - clock).max(0.05).min(wake) }
        }
        tx.wake_after(std::time::Duration::from_secs_f64(wake));

        let arrived: Vec<Arrived> = self.inbox.lock().map(|mut inbox| std::mem::take(&mut *inbox)).unwrap_or_default();
        for item in arrived {
            changed = true;
            match item {
                Arrived::Bible(code, Ok(bible)) => {
                    self.bibles_loading.retain(|c| *c != code);
                    self.bibles.insert(code, Arc::new(bible));
                    self.refresh_decks();
                    if self.palette.open {
                        self.search();
                    }
                }
                Arrived::Bible(code, Err(problem)) => {
                    self.bibles_loading.retain(|c| *c != code);
                    self.say(t::bible_failed(&code, &problem), None);
                }
                Arrived::Lyrics(title, found) => {
                    // An answer for words that have since changed is dropped; being offline is not news.
                    if let Some(editor) = self.editor.as_mut().filter(|editor| editor.id == 0 && editor.asked == title && editor.title.text.trim() == title) {
                        editor.suggestions.retain(|suggestion| matches!(suggestion, Suggestion::Library(_)));
                        editor.suggestions.extend(found.unwrap_or_default().into_iter().take(5).map(Suggestion::Web));
                        editor.suggestion = editor.suggestion.min(editor.suggestions.len().saturating_sub(1));
                    }
                }
                Arrived::Heard(artosia_listen::Heard::Final(text)) => self.heard_final(text),
                Arrived::Heard(artosia_listen::Heard::Kept { n, seconds }) => self.recording_kept(n, seconds),
                Arrived::Heard(artosia_listen::Heard::Read { n, text }) => self.recording_read(n, text),
                Arrived::Heard(artosia_listen::Heard::Speed { sound, took }) => self.note_reading(sound, took),
                Arrived::Heard(artosia_listen::Heard::Behind) => self.readings_behind(),
                Arrived::Heard(artosia_listen::Heard::Problem(artosia_listen::Problem::Keep(_))) => {
                    self.recordings.failed = true;
                    self.end_sitting();
                }
                Arrived::Heard(artosia_listen::Heard::Problem(problem)) => {
                    // A model that would not load is offered again by the row that fetches it.
                    if matches!(problem, artosia_listen::Problem::Model(_)) {
                        self.model = crate::speech_model::Model::Missing;
                    }
                    if let Some(words) = crate::why::listen_problem(&problem, App::speech_built()) {
                        self.listen_problem = Some(words);
                    }
                }
                Arrived::Heard(_) => {}
                Arrived::Pptx(round, made) => self.pptx_arrived(round, made),
                Arrived::WebIndex { index, done } => self.web_index_arrived(index, done),
                Arrived::WebFound(round, query, browse) => self.web_found_arrived(round, query, browse),
                Arrived::WebWords(round, url, fetched) => self.web_words_arrived(round, url, fetched),
                Arrived::Model(result) => self.model_arrived(result),
                Arrived::BetterModel(result) => self.better_arrived(result),
                Arrived::RecordingsSaved(saved) => self.recordings_saved(saved),
                Arrived::Firewall(round, progress) => self.firewall_answered(round, progress),
                Arrived::Update(Ok(Some(version))) => self.say(t::update_ready(&version), None),
                Arrived::Update(Ok(None)) => self.say(t::update_none(crate::update::VERSION), None),
                Arrived::Update(Err(problem)) => self.say(problem, None),
                Arrived::LyricsText(Ok(fetched)) => self.fill_editor(fetched),
                Arrived::LyricsText(Err(problem)) => {
                    if let Some(editor) = self.editor.as_mut() {
                        editor.fetching = false;
                    }
                    self.say(problem, None);
                }
            }
        }
        changed |= self.poll_media();
        if self.link.take_hidden_here() {
            // Someone pressed Escape on the projector shown on this screen.
            self.settings.projector_here = false;
            self.touch_settings();
            changed = true;
        }
        while let Ok(command) = self.remote_rx.try_recv() {
            self.remote_command(command);
            changed = true;
        }
        if self.countdown.as_ref().is_some_and(|timer| timer.ends_at + 2.0 < self.now) {
            self.countdown = None;
            changed = true;
        }
        if self.toast.as_ref().is_some_and(|toast| toast.until <= clock) {
            self.toast = None;
            changed = true;
        }
        if self.members.on != self.civil_today() {
            self.refresh_members();
            changed = true;
        }
        if self.stream.as_mut().is_some_and(|stream| stream.finished()) {
            // The helper has gone: by Stop, or by itself, in which case its last words stay up.
            let last = self.stream.take().and_then(|stream| stream.status().problem);
            self.stream_problem = last;
            changed = true;
        }
        if !self.confirmed && clock > 3.0 {
            // The window has been up for a few seconds: this version works and is kept.
            self.confirmed = true;
            crate::update::confirm();
            // Only now is a newer version looked for. Confirming a version that was just swapped
            // in empties the folder a download fills, so a check made at the start lost its
            // download and the next version needed one more start (0.1.17 with 0.1.20 out).
            if self.folder.is_some() {
                crate::update::check(self.inbox.clone(), self.waker.clone(), false);
            }
        }
        if self.want_paste {
            self.want_paste = false;
            tx.request_paste();
        }
        if self.want_pptx && rune::is_headless() {
            self.want_pptx = false;
            self.demo_pptx("made");
            changed = true;
        }
        if self.want_pptx {
            self.want_pptx = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_PPTX().to_string(), ..Default::default() }.filter(t::CMD_IMPORT_PPTX(), &["pptx"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.open_pptx_import(&path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if self.want_birthdays {
            self.want_birthdays = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_BIRTHDAYS().to_string(), ..Default::default() }.filter(t::CMD_IMPORT_BIRTHDAYS(), &["docx", "pptx", "xlsx", "csv"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.open_import(&path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if let Some(pick) = self.run.pick.take() {
            let (title, kinds) = pick.dialog();
            let options = rune::FileDialogOptions { title: title.to_string(), ..Default::default() }.filter(title, kinds);
            tx.open_file(options, move |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.picked(pick, &path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if self.want_notice_image {
            self.want_notice_image = false;
            let options = rune::FileDialogOptions { title: t::LOOK_CHOOSE_LOGO().to_string(), ..Default::default() }.filter(t::LOOK_CHOOSE_LOGO(), &["png", "jpg", "jpeg", "webp", "bmp"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.set_notice_image(Some(&path)),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if self.want_folder {
            self.want_folder = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_FOLDER().to_string(), ..Default::default() };
            tx.open_folder(options, |s: &mut App, result, _| match result {
                Ok(Some(folder)) => s.import_openlyrics(&folder),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if self.want_zip {
            self.want_zip = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_ZIP().to_string(), ..Default::default() }.filter("Zip", &["zip"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(zip)) => s.import_openlyrics(&zip),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        changed |= self.poll_desk(tx);
        changed |= self.poll_backdrops();
        changed |= self.poll_onboard(tx);
        changed |= self.track_projector_shape();

        let state = self.output_state();
        if self.sent.as_ref() != Some(&state) {
            self.link.send(ToOutput::State(Box::new(state.clone())));
            self.sent = Some(state);
        }
        if let Some(remote) = &self.remote {
            remote.set_state(self.remote_state());
        }
        if self.plan_dirty {
            self.save_plan_now();
        }
        if self.calendar.dirty {
            self.save_calendar_colours();
        }
        if self.giving.dirty {
            self.save_fund_colours();
        }
        if self.settings_dirty {
            self.settings_dirty = false;
            let _ = self.store.set_setting("settings", &self.settings);
        }
        self.load_pictures();
        changed
    }

    fn remote_command(&mut self, command: RemoteCommand) {
        match command {
            RemoteCommand::Next => self.next(),
            RemoteCommand::Previous => self.previous(),
            RemoteCommand::GoTo(index) => self.go_to(index),
            RemoteCommand::ToggleText => self.toggle_text(),
            RemoteCommand::ToggleBlack => self.toggle_black(),
            RemoteCommand::ToggleLive => self.toggle_live(),
            RemoteCommand::ToggleLogo => self.toggle_logo(),
            RemoteCommand::ShowPlanItem(id) => {
                self.open_plan_item(id);
                self.show_start_of(id);
            }
            RemoteCommand::Search(query) => self.remote_search(&query),
            RemoteCommand::ShowHit(id) => {
                if self.open_remote_hit(&id) {
                    self.show(0);
                }
            }
            RemoteCommand::AddHit(id) => {
                if self.open_remote_hit(&id) {
                    self.add_open_to_plan();
                }
            }
            RemoteCommand::Message(text) => {
                self.message_on = !text.trim().is_empty();
                self.message.replace_text(text);
            }
            RemoteCommand::Countdown(0) => self.countdown = None,
            RemoteCommand::Countdown(minutes) => {
                self.timer_minutes.replace_text(minutes.to_string());
                self.start_countdown();
            }
        }
    }

    /// Decodes the background picture and the logo for the previews when their paths change.
    fn load_pictures(&mut self) {
        let Some(images) = &self.images else { return };
        let wanted = match &self.look().background {
            Background::Image(path) => Some(path.clone()),
            _ => None,
        };
        if self.picture.as_ref().map(|(path, _)| path) != wanted.as_ref() {
            self.picture = wanted.map(|path| {
                let image = images.file(&path, 640.0, 1.0);
                (path, image)
            });
        }
        let logo = Some(self.settings.logo.clone()).filter(|path| !path.is_empty());
        if self.logo.as_ref().map(|(path, _)| path) != logo.as_ref() {
            self.logo = logo.map(|path| {
                let image = images.file(&path, 640.0, 1.0);
                (path, image)
            });
        }
        // Announcement pictures: those in the list and the one in the form.
        let mut wanted: Vec<String> = self.announcements.iter().map(|item| item.image.clone()).filter(|path| !path.is_empty()).collect();
        wanted.extend(self.notices.as_ref().map(|form| form.image.clone()).filter(|path| !path.is_empty()));
        self.slide_pictures.retain(|path, _| wanted.contains(path));
        for path in wanted {
            let image = || images.file(&path, 960.0, 1.0);
            self.slide_pictures.entry(path.clone()).or_insert_with(image);
        }
    }
}

/// 1 on the beat, 0 half a beat later, smooth all the way round.
pub fn beat_pulse(phase: f32) -> f32 {
    let wave = 0.5 + 0.5 * (std::f32::consts::TAU * phase).cos();
    wave.powf(1.5)
}

#[cfg(test)]
mod notice_tests {
    use super::*;
    use artosia_core::bible::{Bible, Verse};

    /// A made-up Bible: three verses at Psalms 23, with a book name of its own.
    fn made_up(words: &str) -> Arc<Bible> {
        let verse = |v: u16| Verse { book: 19, chapter: 23, verse: v, text: format!("{words} {v}").into() };
        let names = vec![(19, "Mazmur".to_string(), "Mzm".to_string())];
        Arc::new(Bible::from_parts("TST".into(), None, String::new(), names, vec![verse(1), verse(2), verse(3)]).unwrap_or_else(|_| unreachable!()))
    }

    fn app_with(card: Announcement) -> (App, i64) {
        let mut app = App::practice();
        app.bibles.clear();
        app.bibles.insert("TST".into(), made_up("Made up words"));
        app.settings.bible = "TST".into();
        app.settings.verses_per_slide = 3;
        let mut card = card;
        assert!(app.store.save_announcement(&mut card).is_ok());
        app.reload_announcements();
        (app, card.id)
    }

    fn passage_card(reference: &str) -> Announcement {
        Announcement { title: "Verse of the week".into(), fill: Fill::Passage, passage: reference.into(), seconds: 10, ..Announcement::default() }
    }

    #[test]
    fn practice_announcements_nobody_changed_follow_the_window_s_language() {
        let titles = |app: &App| app.announcements.iter().map(|card| card.title.clone()).collect::<Vec<_>>();
        let fresh = || crate::data::practice_announcements().into_iter().map(|card| card.title).collect::<Vec<_>>();
        let mut app = App::practice();
        let english = titles(&app);
        app.set_language("id");
        assert_eq!(titles(&app), fresh());
        assert_ne!(titles(&app), english, "the Indonesian table has its own practice cards");
        app.set_language("en");
        assert_eq!(titles(&app), english);
        // A card a person rewrote is theirs, and the set stays as it is.
        let mut card = app.announcements[0].clone();
        card.title = "Our own welcome".into();
        assert!(app.store.save_announcement(&mut card).is_ok());
        app.reload_announcements();
        let kept = titles(&app);
        app.set_language("id");
        assert_eq!(titles(&app), kept);
    }

    #[test]
    fn editing_the_announcements_opens_the_card_of_the_ringed_slide() {
        let mut app = App::practice();
        app.open_announcements();
        let second = app.announcements[1].id;
        app.select_at(1);
        app.edit_announcements();
        assert_eq!(app.notices.as_ref().map(|form| form.id), Some(second));
    }

    #[test]
    fn a_heading_started_under_words_brings_its_empty_line() {
        let mut app = App::practice();
        app.edit_new_song(artosia_core::song::Song::default());
        let lyrics = |app: &App| app.editor.as_ref().map(|editor| editor.lyrics.text.clone()).unwrap_or_default();
        app.insert_marker("[1]");
        assert_eq!(lyrics(&app), "[1]\n");
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.insert("Morning light on the hill\n");
        }
        app.insert_marker("[chorus]");
        assert_eq!(lyrics(&app), "[1]\nMorning light on the hill\n\n[chorus]\n");
        // In the middle of a line the heading still starts a stanza of its own.
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.insert("Quiet road below");
        }
        app.insert_marker("[2]");
        assert!(lyrics(&app).ends_with("Quiet road below\n\n[2]\n"), "{:?}", lyrics(&app));
    }

    #[test]
    fn a_passage_card_fills_from_the_installed_bible() {
        let (mut app, id) = app_with(passage_card("Mazmur 23:1-3"));
        let deck = app.card_deck(id).unwrap_or_else(|| panic!("the card has no deck"));
        assert_eq!(deck.slides.len(), 1);
        assert_eq!(deck.slides[0].text, "1 Made up words 1 2 Made up words 2 3 Made up words 3");
        assert_eq!(deck.slides[0].heading, "Verse of the week");
        assert_eq!(deck.slides[0].caption, "Mazmur 23:1-3 TST");
        // The Bible changes: the same card reads the new words.
        app.bibles.insert("TST".into(), made_up("Other words"));
        assert!(app.card_deck(id).is_some_and(|deck| deck.slides[0].text.starts_with("1 Other words 1")));
    }

    #[test]
    fn a_passage_card_without_a_bible_says_so_and_shows_nothing() {
        let (mut app, id) = app_with(passage_card("Mazmur 23:1"));
        app.bibles.clear();
        assert!(app.card_deck(id).is_none());
        app.show_notice_now(id);
        assert!(app.live.is_none());
        assert_eq!(app.toast.as_ref().map(|toast| toast.text.as_str()), Some(t::NOTICE_NOTHING_TO_SHOW()));
        let card = app.announcements.iter().find(|card| card.id == id).cloned().unwrap_or_default();
        assert_eq!(artosia_core::deck::passage_state(&card, app.bible().map(|bible| &**bible)), artosia_core::deck::PassageState::NoBible);
    }

    #[test]
    fn show_now_puts_one_card_up_and_leaves_the_others_out() {
        let (mut app, id) = app_with(passage_card("Mazmur 23:2"));
        assert!(app.announcements.len() > 1, "the practice library has other cards");
        app.show_notice_now(id);
        let live = app.live.as_ref().unwrap_or_else(|| panic!("nothing went live"));
        assert_eq!(live.deck.source, DeckSource::Announcement(id));
        assert_eq!(live.deck.slides.len(), 1);
        assert_eq!(live.deck.slides[0].text, "Made up words 2");
        // A plain card by itself too, never the whole list.
        let other = app.announcements.iter().find(|card| card.id != id).map(|card| card.id).unwrap_or_default();
        app.show_notice_now(other);
        let live = app.live.as_ref().unwrap_or_else(|| panic!("nothing went live"));
        assert_eq!(live.deck.source, DeckSource::Announcement(other));
        assert!(live.deck.slides.iter().all(|slide| !slide.text.contains("Made up words")));
        assert!(app.announcements.len() > live.deck.slides.len());
    }

    #[test]
    fn a_row_press_opens_the_card_in_preview_and_keeps_the_editor() {
        let (mut app, id) = app_with(passage_card("Mazmur 23:1"));
        app.edit_announcements();
        app.pick_notice(id);
        assert!(app.notices.as_ref().is_some_and(|form| form.id == id));
        assert_eq!(app.open.as_ref().map(|open| open.deck.source.clone()), Some(DeckSource::Announcement(id)));
    }

    #[test]
    fn the_panel_sets_the_range_and_the_highlight_and_a_custom_date_keeps_its_ends_in_order() {
        let mut app = App::practice();
        app.demo_birthdays_highlight(None);
        let id = app.notices.as_ref().map(|form| form.id).unwrap_or_default();
        let saved = |app: &App| app.announcements.iter().find(|card| card.id == id).map(|card| card.window).unwrap_or_default();
        assert_eq!(saved(&app), Window { range: Range::Month(0), highlight: Some(Range::Past(7)) });
        // A quick range replaces the days and keeps the highlight, which the card then cuts to fit.
        app.set_notice_range(Range::Week(1));
        assert_eq!(saved(&app), Window { range: Range::Week(1), highlight: Some(Range::Past(7)) });
        assert!(app.today().shown(&saved(&app)).clamped);
        // Custom starts from the days the card read, and a picked first date later than the last pulls the last along.
        app.set_notice_range(Range::Custom(Date { year: 1970, month: 1, day: 1 }, Date { year: 1970, month: 1, day: 1 }));
        let shown = app.today().shown(&saved(&app));
        assert_eq!(saved(&app).range, Range::Custom(shown.from, shown.to));
        app.notice_open_pick(Edge::RangeFrom);
        assert_eq!(app.notices.as_ref().and_then(|form| form.pick).map(|(edge, month)| (edge, month.day)), Some((Edge::RangeFrom, 1)));
        let later = shown.to.plus_days(5);
        app.notice_pick_date(later);
        assert_eq!(saved(&app).range, Range::Custom(later, later));
        assert!(app.notices.as_ref().is_some_and(|form| form.pick.is_none()));
        // The highlight's own custom dates are set the same way and stay inside nothing until they are cut.
        app.set_notice_highlight(Some(Range::Custom(Date { year: 1970, month: 1, day: 1 }, Date { year: 1970, month: 1, day: 1 })));
        app.notice_open_pick(Edge::HighlightTo);
        app.notice_pick_date(later);
        assert_eq!(saved(&app).highlight, Some(Range::Custom(later, later)));
        // None clears it; a schedule card never keeps one.
        app.set_notice_highlight(None);
        assert_eq!(saved(&app).highlight, None);
    }

    #[test]
    fn a_schedule_card_of_next_week_lists_next_weeks_events_and_a_monday_week_moves_them() {
        let mut app = App::practice();
        app.demo_notices("schedule");
        let id = app.notices.as_ref().map(|form| form.id).unwrap_or_default();
        app.set_notice_range(Range::Week(1));
        let card = app.announcements.iter().find(|card| card.id == id).cloned().unwrap_or_default();
        let shown = app.today().shown(&card.window);
        assert_eq!((shown.from.weekday(), shown.to.weekday()), (0, 6), "Sunday to Saturday by default");
        let slides = app.card_slides(&card);
        let listed: usize = slides.iter().map(|slide| slide.text.lines().count()).sum();
        assert_eq!(listed, artosia_core::deck::window_events(&app.events, &app.today(), &card.window, &[]).len());
        assert!(listed >= 6, "a whole week of the practice calendar");
        // With Monday as the first weekday the same card reads Monday to Sunday, and the calendar grid follows.
        app.settings.first_weekday = 1;
        let monday = app.today().shown(&card.window);
        assert_eq!((monday.from.weekday(), monday.to.weekday()), (1, 0));
        app.calendar.anchor = Date { year: 2026, month: 11, day: 14 };
        assert_eq!((app.week_start().weekday(), app.month_start().weekday()), (1, 1));
    }

    #[test]
    fn a_birthday_card_holds_names_and_is_shown_at_once() {
        let mut app = App::practice();
        app.demo_notices("birthdays");
        let id = app.announcements.iter().find(|card| card.fill == Fill::Birthdays).map(|card| card.id).unwrap_or_default();
        let deck = app.card_deck(id).unwrap_or_else(|| panic!("no names in the made-up week"));
        assert!(deck.slides[0].text.contains("Sample Alpha"));
        app.show_notice_now(id);
        assert_eq!(app.live.as_ref().map(|live| live.deck.slides.len()), Some(deck.slides.len()));
    }
}

#[cfg(test)]
mod mark_tests {
    use super::*;

    const PASTED: &str = "Test Song\n\nAlpha line one\nAlpha line two\n\nBeta line one\nBeta line two\n\nAlpha line one\nAlpha line two\n\nGamma line one\nGamma line two\n";
    const WORDS: &str = "Alpha line one\nAlpha line two\n\nBeta line one\nBeta line two\n\nAlpha line one\nAlpha line two\n\nGamma line one\nGamma line two\n";

    fn pasted() -> App {
        let mut app = App::practice();
        app.song_from_clipboard(PASTED);
        app
    }

    fn lyrics(app: &App) -> String {
        app.editor.as_ref().map(|editor| editor.lyrics.text.clone()).unwrap_or_default()
    }

    #[test]
    fn a_pasted_song_with_no_marks_is_offered_marks_and_nothing_is_written() {
        let app = pasted();
        assert_eq!(lyrics(&app), WORDS);
        let offer = app.marks_offer().unwrap_or_else(|| panic!("no offer"));
        let tags: Vec<String> = offer.stanzas.iter().map(|stanza| stanza.tag.short()).collect();
        assert_eq!(tags, ["C", "V1", "C", "V2"]);
    }

    #[test]
    fn applying_the_marks_is_one_edit_and_undo_restores_the_text() {
        let mut app = pasted();
        let caret = WORDS.find("Beta line two").unwrap_or(0) + 3;
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.set_caret(caret);
        }
        app.apply_marks();
        let written = lyrics(&app);
        assert_eq!(written, "[chorus]\nAlpha line one\nAlpha line two\n\n[1]\nBeta line one\nBeta line two\n\n[chorus]\nAlpha line one\nAlpha line two\n\n[2]\nGamma line one\nGamma line two\n");
        assert!(app.marks_offer().is_none(), "a fully marked song proposes nothing");
        // The caret is before the same letter.
        let at = app.editor.as_ref().map_or(0, |editor| editor.lyrics.range().0);
        assert_eq!(&written[at..at + 4], "a li");
        // One undo gives the pasted words back.
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.undo();
        }
        assert_eq!(lyrics(&app), WORDS);
        assert!(app.marks_offer().is_some());
    }

    #[test]
    fn a_picked_kind_marks_one_stanza() {
        let mut app = pasted();
        let start = WORDS.find("Gamma").unwrap_or(0);
        app.mark_stanza(start, SectionKind::Bridge);
        assert!(lyrics(&app).ends_with("\n\n[bridge]\nGamma line one\nGamma line two\n"));
        // The others are still offered.
        assert_eq!(app.marks_offer().map(|offer| offer.pending()), Some(3));
    }

    #[test]
    fn dismissing_the_offer_keeps_it_away_for_this_song() {
        let mut app = pasted();
        app.hide_marks();
        assert!(app.marks_offer().is_none());
        assert_eq!(lyrics(&app), WORDS);
    }

    #[test]
    fn the_web_and_tidy_leave_the_stanzas_for_the_offer() {
        let mut app = pasted();
        let fetched = "Chords\nG  D\nAlpha line one  \nAlpha line two\n\n\nBeta line one\nBeta line two\n";
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.replace_text(fetched);
        }
        app.tidy_editor();
        assert_eq!(lyrics(&app), "Chords\nAlpha line one\nAlpha line two\n\nBeta line one\nBeta line two\n");
        assert!(app.marks_offer().is_some());
    }
}

#[cfg(test)]
mod tempo_tests {
    use super::*;

    fn report(app: &mut App, clock: f64, bpm: f32, since_beat: f32) {
        let tempo = artosia_listen::Tempo { bpm, confidence: 0.7, period: 60.0 / bpm, since_beat };
        if let Ok(mut slot) = app.listen_tempo_slot.lock() {
            *slot = Some((Some(tempo), std::time::Instant::now()));
        }
        app.take_tempo(clock);
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn a_press_halves_the_number_and_the_pulse() {
        let mut app = App::practice();
        report(&mut app, 100.0, 136.0, 0.0);
        assert_eq!(app.tempo_shown().map(f32::round), Some(136.0));
        app.set_tempo_count(TempoCount::Half);
        assert_eq!(app.tempo_shown().map(f32::round), Some(68.0));
        let beat = f64::from(60.0f32 / 136.0);
        let phase = |app: &App, beats: f64| app.beat_phase(100.0 + beats * beat).unwrap_or(-1.0);
        assert!(near(phase(&app, 0.01), 0.005), "{}", phase(&app, 0.01));
        assert!(near(phase(&app, 1.0), 0.5), "the heard beat between two counted ones is half way: {}", phase(&app, 1.0));
        assert!(near(phase(&app, 1.99), 0.995), "{}", phase(&app, 1.99));
    }

    #[test]
    fn the_halved_pulse_does_not_jump_when_the_next_report_comes() {
        let mut app = App::practice();
        report(&mut app, 100.0, 136.0, 0.0);
        app.set_tempo_count(TempoCount::Half);
        let beat = 60.0f32 / 136.0;
        let before = app.beat_phase(100.5).unwrap_or(-1.0);
        // Half a second later the tracker reports again: its last beat is the one after.
        report(&mut app, 100.5, 136.0, 0.5 - beat);
        let after = app.beat_phase(100.5).unwrap_or(-1.0);
        assert!(near(before, after), "{before} then {after}");
    }

    #[test]
    fn a_press_doubles_the_number_and_the_pulse() {
        let mut app = App::practice();
        report(&mut app, 100.0, 70.0, 0.0);
        app.set_tempo_count(TempoCount::Double);
        assert_eq!(app.tempo_shown().map(f32::round), Some(140.0));
        let beat = f64::from(60.0f32 / 70.0);
        assert!(near(app.beat_phase(100.0 + beat * 0.25).unwrap_or(-1.0), 0.5));
        assert!(near(app.beat_phase(100.0 + beat * 0.51).unwrap_or(-1.0), 0.02));
    }

    #[test]
    fn the_count_holds_for_the_song_and_ends_with_it() {
        let mut app = App::practice();
        report(&mut app, 100.0, 136.0, 0.0);
        app.set_tempo_count(TempoCount::Half);
        // The same song a little faster keeps the count.
        report(&mut app, 100.5, 139.0, 0.1);
        assert_eq!(app.tempo_count, TempoCount::Half);
        assert_eq!(app.tempo_shown().map(f32::round), Some(70.0));
        // The tracker changes its own count, as it did on a real ballad: the number stays.
        report(&mut app, 101.0, 68.2, 0.1);
        assert_eq!((app.tempo_count, app.tempo_shown().map(f32::round)), (TempoCount::AsHeard, Some(68.0)));
        report(&mut app, 101.5, 136.4, 0.1);
        assert_eq!((app.tempo_count, app.tempo_shown().map(f32::round)), (TempoCount::Half, Some(68.0)));
        // Another tempo is another song: the number is as heard again.
        report(&mut app, 130.0, 96.0, 0.1);
        assert_eq!(app.tempo_count, TempoCount::AsHeard);
        assert_eq!(app.tempo_shown().map(f32::round), Some(96.0));
        // Switching the listener off forgets it as well.
        app.set_tempo_count(TempoCount::Double);
        app.set_listening(false);
        assert_eq!(app.tempo_count, TempoCount::AsHeard);
    }
}

/// The microphones a headless run shows in Listen.
pub(crate) fn demo_microphones() -> Vec<artosia_listen::Device> {
    ["Microphone Array", "USB microphone", "Headset", "Mixer line in"].iter().map(|name| artosia_listen::Device { name: (*name).to_string(), id: (*name).to_string() }).collect()
}

impl App {
    /// What the Screens dialog shows: the output's own status, or two made-up screens when a
    /// headless render asks for them, so it shows the dialog a volunteer will see.
    pub fn screens_status(&self) -> crate::link::LinkStatus {
        let status = self.link.status();
        if !self.demo_screens {
            return status;
        }
        let screen = |width, height, primary, scale| artosia_slides::frame::DisplayInfo { name: String::new(), width, height, primary, scale };
        crate::link::LinkStatus { connected: true, displays: vec![screen(1920, 1080, true, 1.25), screen(1280, 720, false, 1.0)], audience: Some(1), ..status }
    }

    /// The Screens dialog lists each monitor with its job: the output runs and sees two or more.
    pub fn lists_monitors(&self) -> bool {
        let status = self.screens_status();
        (self.link.is_started() || self.demo_screens) && status.connected && status.displays.len() >= 2
    }
}
