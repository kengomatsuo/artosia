//! Plan edits, the song editor, importing, announcements and the work done between frames.

use std::sync::Arc;

use artosia_core::bible::Reference;
use artosia_core::deck::{Announcement, DeckSource, Fill, Person};
use artosia_core::plan::{Plan, PlanEntry};
use artosia_core::song::{SectionKind, Song, Tag, parse_order, song_from_paste, tidy_lyrics, write_order};
use artosia_remote::{RemoteCommand, RemoteConfig, RemoteHit, RemotePlanItem, RemoteSection, RemoteServer, RemoteSlide, RemoteState};
use artosia_slides::frame::ToOutput;
use artosia_slides::look::{Background, Look};
use rune::{TextEdit, Tx};

use crate::link::OutputOptions;
use crate::state::{App, Arrived, Editor, HeardHit, Notices, Sheet, Suggestion, Undo, entry_matches, section_starts, unix_now};
use crate::strings as t;

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
        self.open.as_ref().is_some_and(|open| open.item.is_some() || self.plan_has(&open.deck.source))
    }

    pub fn add_song_to_plan(&mut self, at: usize) {
        let Some(song) = self.songs.get(at) else { return };
        let (id, title) = (song.id, song.title.clone());
        self.queue_entry(PlanEntry::Song { song_id: id, title: title.clone() });
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
            DeckSource::Song(id) => PlanEntry::Song { song_id: *id, title: open.deck.title.clone() },
            DeckSource::Passage { reference, bible, second } => {
                PlanEntry::Passage { reference: *reference, bible: bible.clone(), second: second.clone(), title: open.deck.title.clone() }
            }
            DeckSource::Announcements | DeckSource::Announcement(_) => PlanEntry::Announcements,
        };
        let was_live = self.open_is_live();
        let id = self.queue_entry(entry);
        if let Some(open) = self.open.as_mut() {
            open.item = Some(id);
        }
        if was_live
            && let Some(live) = self.live.as_mut() {
                live.item = Some(id);
            }
        self.requeue();
    }

    /// Adds an entry to the plan right after what is live, so it is the next one. With
    /// nothing from the plan live yet it goes to the end, the order a plan is written in.
    fn queue_entry(&mut self, entry: PlanEntry) -> u64 {
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

    pub fn new_plan(&mut self) {
        self.save_plan_now();
        self.plan = Plan::named(t::PLAN_DEFAULT_NAME());
        self.settings.plan = 0;
        self.open = None;
        self.touch_plan();
        self.touch_settings();
    }

    fn save_plan_now(&mut self) {
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
            second: TextEdit::new(song.second.clone()),
            show_second: !song.second.trim().is_empty(),
            ..Editor::default()
        };
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
        self.close_palette();
        self.sheet = Sheet::None;
        self.title_changed();
    }

    pub fn edit_open_song(&mut self) {
        let Some(DeckSource::Song(id)) = self.open.as_ref().map(|open| open.deck.source.clone()) else { return };
        if let Some(at) = self.song_position(id) {
            self.editor = Some(App::editor_for(&self.songs[at]));
            self.notices = None;
        }
    }

    // ---- Whole songs suggested for a new one ----

    /// Runs when a new song's title changes: lists the songs the library already has under
    /// that name at once, and asks the web a moment after the typing stops.
    pub fn title_changed(&mut self) {
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
            let found = artosia_lyrics::search(&title).map_err(|error| error.to_string());
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Lyrics(title, found));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    pub fn move_suggestion(&mut self, by: isize) {
        let Some(editor) = self.editor.as_mut().filter(|editor| !editor.suggestions.is_empty()) else { return };
        let count = editor.suggestions.len() as isize;
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
                    let fetched = artosia_lyrics::fetch(&found).map_err(|error| error.to_string());
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
        Some(Song {
            id: editor.id,
            title: editor.title.text.trim().to_string(),
            author: editor.author.text.trim().to_string(),
            copyright: editor.copyright.text.trim().to_string(),
            ccli: editor.ccli.text.trim().to_string(),
            key: editor.key.text.trim().to_string(),
            order: editor.order.text.trim().to_string(),
            lyrics: editor.lyrics.text.replace("\r\n", "\n"),
            second: if editor.show_second { editor.second.text.replace("\r\n", "\n") } else { String::new() },
            ..existing
        })
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
                    if let PlanEntry::Song { song_id, title } = &mut item.entry
                        && *song_id == id && *title != song.title {
                            *title = song.title.clone();
                            self.plan_dirty = true;
                        }
                }
                if let Some(at) = self.song_position(id) {
                    self.open_song(at, None);
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

    pub fn insert_marker(&mut self, marker: &str) {
        let Some(editor) = self.editor.as_mut() else { return };
        let at = editor.lyrics.range().0.min(editor.lyrics.text.len());
        let at_line_start = at == 0 || editor.lyrics.text[..at].ends_with('\n');
        let text = if at_line_start { format!("{marker}\n") } else { format!("\n{marker}\n") };
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

    /// Adds every OpenLyrics song in a folder, skipping songs the library already has.
    pub fn import_openlyrics(&mut self, folder: &std::path::Path) {
        let (mut songs, failed) = artosia_core::openlyrics::read_folder(folder);
        if songs.is_empty() && failed == 0 {
            self.say(t::IMPORT_FOLDER_EMPTY(), None);
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

    pub fn dismiss_welcome(&mut self) {
        self.settings.welcomed = true;
        self.touch_settings();
    }

    // ---- Announcements ----

    fn notices_for(item: Option<&Announcement>) -> Notices {
        let mut form = match item {
            Some(item) => Notices {
                id: item.id,
                title: TextEdit::new(item.title.clone()),
                body: TextEdit::new(item.body.clone()),
                seconds: item.seconds,
                image: item.image.clone(),
                fill: item.fill,
            },
            None => Notices { seconds: 10, ..Notices::default() },
        };
        form.title.focused = true;
        form
    }

    /// Opens the announcement editor on the first announcement, or on a blank one.
    pub fn edit_announcements(&mut self) {
        self.notices = Some(App::notices_for(self.announcements.first()));
        self.editor = None;
        self.sheet = Sheet::None;
        self.close_palette();
    }

    /// The announcement in the form, as it would be saved; None while it has no words.
    pub fn notice_draft(&self) -> Option<Announcement> {
        let form = self.notices.as_ref()?;
        let (title, body) = (form.title.text.trim().to_string(), form.body.text.trim().to_string());
        let has_words = !title.is_empty() || !body.is_empty() || !form.image.is_empty() || form.fill != Fill::Nothing;
        has_words.then_some(Announcement { id: form.id, title, body, seconds: form.seconds.clamp(2, 600), image: form.image.clone(), fill: form.fill })
    }

    fn reload_announcements(&mut self) {
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
        }
    }

    /// Puts another announcement in the form; the one being written is saved first.
    pub fn pick_notice(&mut self, id: i64) {
        self.save_notice();
        let found = self.announcements.iter().find(|item| item.id == id);
        self.notices = Some(App::notices_for(found));
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

    pub fn set_notice_fill(&mut self, fill: Fill) {
        if let Some(form) = self.notices.as_mut() {
            form.fill = fill;
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
    fn media_folder(&self) -> std::path::PathBuf {
        let base = self.folder.clone().unwrap_or_else(|| std::env::temp_dir().join("artosia-practice"));
        base.join("media")
    }

    /// Brings a PowerPoint in as announcements: one per slide, tables as columns, pictures kept.
    pub fn import_pptx(&mut self, path: &std::path::Path) {
        let deck = match artosia_office::Pptx::open(path) {
            Ok(deck) => deck,
            Err(error) => {
                self.say(t::could_not_read(&error.to_string()), None);
                return;
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
            let mut item = Announcement { id: 0, title: draft.title, body: draft.body, seconds: 10, image, fill: Fill::Nothing };
            if self.store.save_announcement(&mut item).is_ok() {
                added += 1;
            }
        }
        self.reload_announcements();
        if self.notices.is_some() {
            self.notices = Some(App::notices_for(self.announcements.last()));
        }
        self.say(t::pptx_imported(added, lost), None);
    }

    /// Replaces the birthday list with the one in a Word or PowerPoint file.
    pub fn import_birthdays(&mut self, path: &std::path::Path) {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let found = match extension.as_str() {
            "pptx" => artosia_office::Pptx::open(path).map(|deck| artosia_office::from_pptx(&deck)),
            _ => artosia_office::Docx::open(path).map(|file| artosia_office::from_docx(&file)),
        };
        let found = match found {
            Ok(found) => found,
            Err(error) => {
                self.say(t::could_not_read(&error.to_string()), None);
                return;
            }
        };
        // Church lists repeat people across tables; each person is kept once.
        let people: Vec<Person> = artosia_office::dedup(found.people).into_iter().map(|person| Person { id: 0, name: person.name, day: person.day, month: person.month }).collect();
        if people.is_empty() {
            self.say(t::BIRTHDAYS_NONE_FOUND(), None);
            return;
        }
        if self.store.replace_people(&people).is_ok() {
            self.people = self.store.people().unwrap_or_default();
            self.refresh_decks();
            self.say(t::birthdays_imported(self.people.len(), found.unread), None);
        }
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
        self.notices = None;
    }

    pub fn open_announcements(&mut self) {
        self.save_notice();
        let deck = self.announcements_deck();
        self.open_deck(deck, None, 0);
        self.sheet = Sheet::None;
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
            let folder = rune::dirs::AppDirs::new("artosia").map(|dirs| dirs.data().join("recordings")).unwrap_or_else(std::env::temp_dir);
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

    /// Opens the listening dialog with the microphones this computer has now.
    pub fn open_listen(&mut self) {
        self.microphones = artosia_listen::devices();
        self.sheet = Sheet::Listen;
    }

    /// Starts or stops listening to the room. Words that are heard become suggestions; nothing
    /// is ever shown on the projector because of them.
    pub fn set_listening(&mut self, on: bool) {
        if let Some(listener) = self.listener.take() {
            // Stopping waits for a recognition in progress, so it happens off the window's thread.
            std::thread::spawn(move || drop(listener));
        }
        self.listen_problem = None;
        self.listen_level.store(0, std::sync::atomic::Ordering::Relaxed);
        if !on {
            return;
        }
        let folder = self.folder.clone().unwrap_or_else(std::env::temp_dir);
        let model = folder.join("models").join("ggml-base-q5_1.bin");
        let device = Some(self.settings.listen_device.clone()).filter(|name| !name.is_empty());
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        let (level, shown) = (self.listen_level.clone(), self.meter_shown.clone());
        let started = artosia_listen::Listener::start(device.as_deref(), &model, &self.settings.listen_language, move |heard| {
            let wake = match heard {
                artosia_listen::Heard::Level(now) => {
                    level.store(now.clamp(0.0, 1.0).to_bits(), std::sync::atomic::Ordering::Relaxed);
                    // The meter only needs drawing while someone can see it.
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
            Err(error) => self.listen_problem = Some(error.to_string()),
        }
    }

    /// Turns a sentence that was heard into what it may have named: a passage, or songs that
    /// hold its words.
    pub(crate) fn heard_final(&mut self, text: String) {
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

    /// F4: puts what the listener last recognised in the Preview pane, where Enter shows it.
    /// It never goes to the projector by itself: a misheard word must not reach the room.
    pub fn open_last_heard(&mut self) {
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
        let lines = self.settings.look.max_lines;
        let changed_lines = look.max_lines != lines;
        self.settings.look = look;
        self.touch_settings();
        if changed_lines {
            self.refresh_decks();
        }
    }

    pub fn set_background_file(&mut self, path: &std::path::Path) {
        let text = path.to_string_lossy().to_string();
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let background = match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => Background::Image(text),
            "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "wmv" => Background::Video(text),
            _ => {
                self.say(t::BACKGROUND_UNKNOWN(), None);
                return;
            }
        };
        let mut look = self.settings.look.clone();
        look.background = background;
        look.shadow = true;
        look.name = t::LOOK_CUSTOM().to_string();
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

    pub fn set_remote(&mut self, on: bool) {
        self.settings.remote_on = on;
        self.touch_settings();
        if !on {
            self.remote = None;
            return;
        }
        if self.remote.is_some() {
            return;
        }
        self.remote_pin = artosia_remote::new_pin();
        let (tx, waker) = (self.remote_tx.clone(), self.waker.clone());
        let started = RemoteServer::start(RemoteConfig { port: 4780, pin: self.remote_pin.clone() }, move |command| {
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
                }
                .to_string(),
                live: live_item == Some(item.id),
            })
            .collect();
        let mut state = RemoteState {
            text_hidden: self.text_hidden,
            black: self.black,
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
            state.slides = live.deck.slides.iter().map(|slide| RemoteSlide { label: slide.label.clone(), text: slide.text.clone() }).collect();
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
        self.settings.projector_off = options.off;
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
        if !t::set_language(code) {
            return;
        }
        self.settings.language = code.to_string();
        self.touch_settings();
        self.refresh_decks();
        if self.palette.open {
            self.search();
        }
    }

    /// Puts a camera behind the words, as a video background is.
    pub fn set_camera(&mut self, name: &str) {
        let mut look = self.settings.look.clone();
        look.background = Background::Camera(name.to_string());
        look.shadow = true;
        look.name = t::LOOK_CUSTOM().to_string();
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
        let mut wake = if self.countdown.is_some() { 0.25 } else { 1.0 };
        let was = self.live.as_ref().map(|live| live.index);
        if let Some(until) = self.loop_step(clock) {
            wake = until.min(wake);
            changed |= was != self.live.as_ref().map(|live| live.index);
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
                Arrived::Heard(artosia_listen::Heard::Problem(problem)) => self.listen_problem = Some(problem),
                Arrived::Heard(_) => {}
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
        self.meter_shown.store(self.sheet == Sheet::Listen && self.listener.is_some(), std::sync::atomic::Ordering::Relaxed);
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
        }
        if self.want_paste {
            self.want_paste = false;
            tx.request_paste();
        }
        if self.want_pptx {
            self.want_pptx = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_PPTX().to_string(), ..Default::default() }.filter(t::CMD_IMPORT_PPTX(), &["pptx"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.import_pptx(&path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
        if self.want_birthdays {
            self.want_birthdays = false;
            let options = rune::FileDialogOptions { title: t::CMD_IMPORT_BIRTHDAYS().to_string(), ..Default::default() }.filter(t::CMD_IMPORT_BIRTHDAYS(), &["docx", "pptx"]);
            tx.open_file(options, |s: &mut App, result, _| match result {
                Ok(Some(path)) => s.import_birthdays(&path),
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
        changed |= self.poll_desk(tx);

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
            RemoteCommand::ToggleLogo => self.toggle_logo(),
            RemoteCommand::ShowPlanItem(id) => {
                self.open_plan_item(id);
                if self.open.as_ref().is_some_and(|open| open.item == Some(id)) {
                    self.show(0);
                }
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
        let wanted = match &self.settings.look.background {
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
