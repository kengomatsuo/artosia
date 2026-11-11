//! Recordings: when a person switches it on, the phrases the listener heard and what Artosia
//! made of them are kept in a folder on this computer, so the people who build Artosia can
//! learn from real services. Nothing is sent anywhere: the folder is handed over as one zip,
//! or sits where a cloud drive picks it up. Full account: `docs/recordings.md`.
//!
//! One folder for each time the listener was switched on (a sitting):
//! `about.json`, `attempts.jsonl` (one JSON object a line, in the order things happened) and
//! `phrase-00001.wav` onwards, written by the listener itself.

use std::io::Write;
use std::path::{Path, PathBuf};

use artosia_core::deck::DeckSource;
use serde_json::{Value, json};

use crate::state::{App, Arrived, Undo};
use crate::strings as t;

/// The most the folder holds; past it the oldest sittings go.
pub const LIMIT: u64 = 1 << 30;
/// Where Delete all puts the sittings until its Undo has passed. The zip leaves it out.
const DELETED: &str = ".deleted";
/// The folder made inside the one a person picks, so their drive is not strewn with sittings.
const PICKED: &str = "Artosia recordings";

#[derive(Default)]
pub struct Recordings {
    sitting: Option<Sitting>,
    /// Bytes kept, as of the last look at the folder plus what this sitting added since.
    pub used: u64,
    /// The folder could not be written.
    pub failed: bool,
    /// Delete all moved the sittings aside and its Undo may still be pressed.
    trash: bool,
    /// The zip is being written.
    pub saving: bool,
    /// A headless scene shows the lit mark as it looks while a sitting is open.
    pub demo: bool,
}

impl Recordings {
    /// As the program starts: a Delete all whose Undo never ran out, because the program was
    /// closed first, is finished at the first poll.
    pub fn at_start() -> Recordings {
        Recordings { trash: true, ..Default::default() }
    }
}

struct Sitting {
    log: std::fs::File,
    began: std::time::Instant,
    /// What was last written of each kind, so only changes are written.
    live: String,
    offers: String,
    tempo: String,
}

/// Bytes of every file under a folder, the set-aside ones left out.
fn size_of(folder: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(folder) else { return 0 };
    entries
        .flatten()
        .filter(|entry| entry.file_name() != DELETED)
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => size_of(&entry.path()),
            _ => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
        })
        .sum()
}

/// The sittings of a folder, oldest first: their names are their dates.
fn sittings(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|entries| entries.flatten().filter(|entry| entry.file_name() != DELETED && entry.path().is_dir()).map(|entry| entry.path()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

/// Deletes the oldest sittings until the folder holds less than `limit`. The newest stays:
/// it is the one being written.
fn make_room(root: &Path, limit: u64) {
    let mut used = size_of(root);
    let all = sittings(root);
    for sitting in all.iter().take(all.len().saturating_sub(1)) {
        if used < limit {
            break;
        }
        let size = size_of(sitting);
        if std::fs::remove_dir_all(sitting).is_ok() {
            used = used.saturating_sub(size);
        }
    }
}

/// Moves a folder, by copying when the two places are on different drives.
fn move_folder(from: &Path, to: &Path) -> std::io::Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            move_folder(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    std::fs::remove_dir_all(from)
}

/// A size as a person reads it: "212 MB", "1.4 GB", and "under 1 MB" as "1 MB".
pub fn size_words(bytes: u64) -> String {
    let megabytes = bytes.div_ceil(1 << 20).max(1);
    if megabytes >= 1000 { format!("{:.1} GB", megabytes as f64 / 1024.0) } else { format!("{megabytes} MB") }
}

/// A figure of the log with as many places as it is good for. A 32-bit figure written as it
/// stands reads 71.4000015258789.
fn tidy(value: f32, per_one: f64) -> f64 {
    (f64::from(value) * per_one).round() / per_one
}

/// How the tempo is counted, as the log names it.
fn count_name(count: crate::state::TempoCount) -> &'static str {
    match count {
        crate::state::TempoCount::Half => "half",
        crate::state::TempoCount::AsHeard => "as heard",
        crate::state::TempoCount::Double => "double",
    }
}

impl App {
    /// Where recordings are kept: the folder a person picked, else beside the library. None in
    /// practice, where nothing is kept.
    pub fn recordings_folder(&self) -> Option<PathBuf> {
        if !self.settings.recordings_folder.is_empty() {
            return Some(PathBuf::from(&self.settings.recordings_folder));
        }
        self.folder.as_ref().map(|folder| folder.join("recordings"))
    }

    /// A sitting is open: phrases and attempts are being kept right now.
    pub fn recording(&self) -> bool {
        self.recordings.sitting.is_some() || self.recordings.demo
    }

    /// Looks at the folder again for how much it holds.
    pub fn refresh_recordings(&mut self) {
        self.recordings.used = self.recordings_folder().map(|folder| size_of(&folder)).unwrap_or(0);
    }

    /// Opens a sitting when recordings are switched on, and returns its folder for the
    /// listener to write the phrases in. A headless run keeps nothing.
    pub(crate) fn begin_sitting(&mut self) -> Option<PathBuf> {
        self.end_sitting();
        if !self.settings.keep_recordings || rune::is_headless() {
            return None;
        }
        let root = self.recordings_folder()?;
        let time = rune::LocalTime::at((crate::state::unix_now() * 1000.0) as i64);
        let name = format!("{:04}-{:02}-{:02} {:02}.{:02}.{:02}", time.year, time.month, time.day, time.hour, time.minute, time.second);
        let folder = root.join(name);
        let about = json!({
            "artosia": crate::update::VERSION,
            "began": crate::state::unix_now() as i64,
            // What kind of source, never its name: a device's name says whose it is.
            "source": match self.settings.listen_device.as_str() {
                "" => "default",
                artosia_listen::SYSTEM_AUDIO => "system",
                _ => "microphone",
            },
            "language": self.settings.listen_language,
            "words": App::speech_built() && self.model_path().is_some_and(|path| path.is_file()),
            "songs": self.songs.len(),
        });
        let opened = std::fs::create_dir_all(&folder).and_then(|()| {
            make_room(&root, LIMIT);
            std::fs::write(folder.join("about.json"), format!("{about:#}\n"))?;
            std::fs::File::create(folder.join("attempts.jsonl"))
        });
        match opened {
            Ok(log) => {
                self.recordings.failed = false;
                self.recordings.sitting = Some(Sitting { log, began: std::time::Instant::now(), live: String::new(), offers: String::new(), tempo: String::new() });
                self.refresh_recordings();
                Some(folder)
            }
            Err(_) => {
                self.recordings.failed = true;
                None
            }
        }
    }

    pub(crate) fn end_sitting(&mut self) {
        if self.recordings.sitting.take().is_some() {
            self.refresh_recordings();
        }
    }

    /// Writes one line of the sitting: what happened, and when in seconds since it began.
    fn record(&mut self, kind: &str, mut fields: Value) {
        let Some(sitting) = self.recordings.sitting.as_mut() else { return };
        if let Some(map) = fields.as_object_mut() {
            map.insert("t".into(), json!((sitting.began.elapsed().as_secs_f64() * 100.0).round() / 100.0));
            map.insert("kind".into(), json!(kind));
        }
        if writeln!(sitting.log, "{fields}").is_err() {
            self.recordings.failed = true;
        }
    }

    /// The listener wrote phrase `n` as a wave file.
    pub(crate) fn recording_kept(&mut self, n: u32, seconds: f32) {
        // 16-bit samples at the recogniser's rate, and the wave file's 44 bytes.
        self.recordings.used += (seconds * artosia_listen::RECOGNISER_RATE as f32 * 2.0) as u64 + 44;
        self.record("phrase", json!({ "n": n, "seconds": tidy(seconds, 100.0) }));
        // The folder's limit is held during a long sitting too, by the older sittings.
        if self.recordings.used > LIMIT
            && let Some(root) = self.recordings_folder()
        {
            make_room(&root, LIMIT);
            self.refresh_recordings();
            // One sitting as large as the limit (a listener left on for nine hours): a new one
            // begins, and this one goes as the oldest.
            if self.recordings.used > LIMIT && self.listener.is_some() {
                self.set_listening(true);
            }
        }
    }

    /// The recogniser read phrase `n`: its words and what Artosia made of them.
    pub(crate) fn recording_read(&mut self, n: u32, text: Option<String>) {
        if self.recordings.sitting.is_none() {
            return;
        }
        let (near, named, passage) = text.as_deref().map(|text| self.attempt_of(text)).unwrap_or_default();
        let near: Vec<Value> = near.into_iter().map(|(title, together)| json!({ "title": title, "together": together })).collect();
        self.record("read", json!({ "n": n, "heard": text, "near": near, "named": named, "passage": passage }));
    }

    /// A person took an offer into Preview.
    pub(crate) fn recording_taken(&mut self, why: &str, title: &str) {
        self.record("taken", json!({ "why": why, "title": title }));
    }

    /// A person said how the tempo is counted.
    pub(crate) fn recording_count(&mut self) {
        let wanted = self.tempo_wanted.map(|bpm| tidy(bpm, 10.0));
        self.record("count", json!({ "to": count_name(self.tempo_count), "wanted": wanted }));
    }

    /// Writes what changed since the last look: what is on the projector, what is offered and
    /// the tempo. What is on the projector is the answer the guesses are measured against.
    pub(crate) fn note_recordings(&mut self) {
        if self.recordings.sitting.is_none() {
            return;
        }
        let live = match &self.live {
            Some(live) => {
                let slide = live.deck.slides.get(live.index);
                let (what, title) = match &live.deck.source {
                    DeckSource::Song(_) => ("song", Some(live.deck.title.clone())),
                    DeckSource::Passage { .. } => ("passage", Some(live.deck.title.clone())),
                    // A notice may hold people's names, so only that one is shown is written.
                    DeckSource::Announcement(_) | DeckSource::Announcements => ("notice", None),
                };
                json!({ "what": what, "title": title, "slide": live.index, "section": slide.map(|slide| slide.short.clone()), "in_the_room": self.on_air() && !self.black && !self.text_hidden })
            }
            None => json!({ "what": Value::Null }),
        };
        let offers: Vec<Value> = crate::suggest::suggestions(self).into_iter().map(|offer| json!({ "why": format!("{:?}", offer.why), "title": offer.title })).collect();
        let tempo = match (self.tempo, self.beat_phase(self.clock)) {
            (Some((tempo, _)), Some(_)) => json!({ "bpm": tidy(tempo.bpm, 10.0), "sure": tidy(tempo.confidence, 100.0), "count": count_name(self.tempo_count), "shown": self.tempo_shown().map(|bpm| bpm.round()) }),
            _ => json!({ "bpm": Value::Null }),
        };
        // The confidence wobbles with every report; it is written, and left out of what counts as a change.
        let tempo_mark = match (self.tempo, self.beat_phase(self.clock)) {
            (Some((tempo, _)), Some(_)) => format!("{:.0} {:?}", tempo.bpm, self.tempo_count),
            _ => String::new(),
        };
        let (live_mark, offers_mark) = (live.to_string(), Value::from(offers.clone()).to_string());
        let Some(sitting) = self.recordings.sitting.as_mut() else { return };
        let changed = (sitting.live != live_mark, sitting.offers != offers_mark, sitting.tempo != tempo_mark);
        (sitting.live, sitting.offers, sitting.tempo) = (live_mark, offers_mark, tempo_mark);
        if changed.0 {
            self.record("live", live);
        }
        if changed.1 {
            self.record("offered", json!({ "offers": offers }));
        }
        if changed.2 {
            self.record("tempo", tempo);
        }
    }

    /// Switches recordings on or off. The listener is started again so a sitting begins or ends.
    pub fn set_keep_recordings(&mut self, on: bool) {
        self.settings.keep_recordings = on;
        self.touch_settings();
        if self.listener.is_some() {
            self.set_listening(true);
        }
        self.refresh_recordings();
    }

    /// Delete all: every sitting goes, and Undo in the toast brings them back until it leaves.
    pub fn delete_recordings(&mut self) {
        let Some(root) = self.recordings_folder() else { return };
        let listening = self.listener.is_some();
        self.set_listening(false);
        let aside = root.join(DELETED);
        let _ = std::fs::remove_dir_all(&aside);
        let moved = std::fs::create_dir_all(&aside).is_ok() && sittings(&root).iter().all(|sitting| sitting.file_name().is_some_and(|name| std::fs::rename(sitting, aside.join(name)).is_ok()));
        self.recordings.trash = moved;
        self.refresh_recordings();
        self.say(t::REC_DELETED(), moved.then_some(Undo::Recordings));
        if listening {
            self.set_listening(true);
        }
    }

    /// Undo of Delete all.
    pub(crate) fn restore_recordings(&mut self) {
        let Some(root) = self.recordings_folder() else { return };
        for sitting in sittings(&root.join(DELETED)) {
            if let Some(name) = sitting.file_name() {
                let _ = std::fs::rename(&sitting, root.join(name));
            }
        }
        let _ = std::fs::remove_dir_all(root.join(DELETED));
        self.recordings.trash = false;
        self.refresh_recordings();
    }

    /// Poll: once the toast with Undo has left, what was deleted is gone for good.
    pub(crate) fn purge_recordings(&mut self) {
        if self.recordings.trash
            && self.toast.is_none()
            && let Some(root) = self.recordings_folder()
        {
            let _ = std::fs::remove_dir_all(root.join(DELETED));
            self.recordings.trash = false;
        }
    }

    /// Move folder: recordings are kept inside the folder a person picked from now on, and
    /// what was kept so far goes with them.
    pub fn move_recordings(&mut self, picked: PathBuf) {
        let to = if picked.file_name().is_some_and(|name| name == PICKED) { picked } else { picked.join(PICKED) };
        let from = self.recordings_folder();
        if from.as_ref() == Some(&to) {
            return;
        }
        let listening = self.listener.is_some();
        self.set_listening(false);
        let mut moved = std::fs::create_dir_all(&to).is_ok();
        if let (true, Some(from)) = (moved, from) {
            for sitting in sittings(&from) {
                if let Some(name) = sitting.file_name() {
                    moved &= move_folder(&sitting, &to.join(name)).is_ok();
                }
            }
            // The old place goes when nothing else is in it.
            let _ = std::fs::remove_dir(&from);
        }
        if moved {
            self.settings.recordings_folder = to.to_string_lossy().to_string();
            self.touch_settings();
        }
        self.recordings.failed = !moved;
        self.refresh_recordings();
        if listening {
            self.set_listening(true);
        }
    }

    /// Save as one file: the folder is packed into a zip off the window's thread.
    pub fn save_recordings(&mut self, to: PathBuf) {
        let Some(root) = self.recordings_folder() else { return };
        self.recordings.saving = true;
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let saved = artosia_core::pack::zip_folder(&root, &to).map(|_| to.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()).map_err(|_| ());
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::RecordingsSaved(saved));
            }
            if let Some(wake) = &waker {
                wake();
            }
        });
    }

    pub(crate) fn recordings_saved(&mut self, saved: Result<String, ()>) {
        self.recordings.saving = false;
        match saved {
            Ok(name) => self.say(t::rec_saved(&name), None),
            Err(()) => self.say(t::REC_FAILED(), None),
        }
    }

    /// Open folder: the computer's own file window on where the recordings are.
    pub fn open_recordings_folder(&mut self) {
        let Some(root) = self.recordings_folder() else { return };
        if std::fs::create_dir_all(&root).is_err() {
            self.recordings.failed = true;
            return;
        }
        let program = if cfg!(target_os = "windows") {
            "explorer"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(program).arg(&root).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("artosia-rec-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
        folder
    }

    fn sitting_of(root: &Path, name: &str, bytes: usize) {
        std::fs::create_dir_all(root.join(name)).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(root.join(name).join("phrase-00001.wav"), vec![0u8; bytes]).unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    fn the_oldest_sittings_go_when_the_folder_is_over_its_limit() {
        let root = scratch("room");
        for (name, bytes) in [("2026-11-01 07.00.00", 400), ("2026-11-08 07.00.00", 400), ("2026-11-15 07.00.00", 400)] {
            sitting_of(&root, name, bytes);
        }
        make_room(&root, 900);
        let left: Vec<String> = sittings(&root).iter().filter_map(|path| path.file_name().map(|name| name.to_string_lossy().to_string())).collect();
        assert_eq!(left, ["2026-11-08 07.00.00", "2026-11-15 07.00.00"]);
        assert_eq!(size_of(&root), 800);
        // The sitting being written stays, however large it is.
        make_room(&root, 100);
        assert_eq!(sittings(&root).len(), 1);
        assert_eq!(size_of(&root), 400);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_all_can_be_undone_until_the_toast_leaves_and_is_then_for_good() {
        let root = scratch("delete");
        let mut app = App::practice();
        app.settings.recordings_folder = root.to_string_lossy().to_string();
        sitting_of(&root, "2026-11-08 07.00.00", 300);
        sitting_of(&root, "2026-11-15 07.00.00", 500);
        app.refresh_recordings();
        assert_eq!(app.recordings.used, 800);
        app.delete_recordings();
        assert_eq!((app.recordings.used, sittings(&root).len()), (0, 0));
        app.undo();
        assert_eq!((app.recordings.used, sittings(&root).len()), (800, 2));
        // Deleted again, and this time the toast leaves: nothing is left on the disk.
        app.delete_recordings();
        app.toast = None;
        app.purge_recordings();
        assert!(!root.join(DELETED).exists());
        assert_eq!(size_of(&root), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_delete_cut_short_by_closing_the_program_is_finished_at_the_next_start() {
        let root = scratch("left");
        sitting_of(&root.join(DELETED), "2026-11-08 07.00.00", 300);
        let mut app = App::practice();
        app.settings.recordings_folder = root.to_string_lossy().to_string();
        app.recordings = Recordings::at_start();
        app.toast = None;
        app.purge_recordings();
        assert!(!root.join(DELETED).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_picked_folder_takes_what_was_kept_with_it() {
        let (from, picked) = (scratch("from"), scratch("picked"));
        let mut app = App::practice();
        app.settings.recordings_folder = from.to_string_lossy().to_string();
        sitting_of(&from, "2026-11-15 07.00.00", 500);
        app.move_recordings(picked.clone());
        let to = picked.join(PICKED);
        assert_eq!(app.recordings_folder(), Some(to.clone()));
        assert_eq!((size_of(&to), from.exists()), (500, false));
        assert!(!app.recordings.failed);
        for folder in [from, picked] {
            let _ = std::fs::remove_dir_all(folder);
        }
    }

    #[test]
    fn save_as_one_file_packs_every_sitting_and_says_the_files_name() {
        let (root, out) = (scratch("save"), scratch("saved"));
        let mut app = App::practice();
        app.settings.recordings_folder = root.to_string_lossy().to_string();
        sitting_of(&root, "2026-11-15 07.00.00", 500);
        let file = out.join("Artosia recordings.zip");
        app.save_recordings(file.clone());
        assert!(app.recordings.saving);
        let began = std::time::Instant::now();
        let saved = loop {
            if let Some(Arrived::RecordingsSaved(saved)) = app.inbox.lock().ok().and_then(|mut inbox| inbox.pop()) {
                break saved;
            }
            assert!(began.elapsed().as_secs() < 10, "the zip never arrived");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        app.recordings_saved(saved);
        assert!(!app.recordings.saving);
        assert_eq!(app.toast.as_ref().map(|toast| toast.text.clone()), Some(t::rec_saved("Artosia recordings.zip")));
        assert!(std::fs::metadata(&file).is_ok_and(|meta| meta.len() > 0));
        for folder in [root, out] {
            let _ = std::fs::remove_dir_all(folder);
        }
    }

    #[test]
    fn a_size_reads_as_a_person_says_it() {
        assert_eq!(size_words(10), "1 MB");
        assert_eq!(size_words(212 << 20), "212 MB");
        assert_eq!(size_words(1433 << 20), "1.4 GB");
    }

    #[test]
    fn a_sitting_writes_what_was_shown_heard_guessed_and_taken() {
        let root = scratch("sitting");
        let mut app = App::practice();
        app.settings.recordings_folder = root.to_string_lossy().to_string();
        app.settings.keep_recordings = true;
        app.settings.listen_device = "A microphone with a person's name on it".to_string();
        let folder = app.begin_sitting().unwrap_or_else(|| panic!("no sitting was opened"));
        assert!(app.recording());
        app.plan.items.clear();
        app.open_song(app.songs.iter().position(|song| song.title == "Morning on the Hill").unwrap_or(0), None);
        app.show(0);
        app.note_recordings();
        // Words of another practice song are heard while that one is shown.
        let heard = "down by the river we gather mercy flowing like the rain all that was broken made whole again";
        app.recording_kept(1, 7.33);
        app.recording_read(1, Some(heard.to_string()));
        app.heard_final(heard.to_string());
        app.note_recordings();
        let offers = crate::suggest::suggestions(&app);
        let offer = offers.first().unwrap_or_else(|| panic!("the heard song was not offered"));
        app.take_offer(offer);
        app.end_sitting();
        assert!(!app.recording());

        let about = std::fs::read_to_string(folder.join("about.json")).unwrap_or_else(|e| panic!("{e}"));
        assert!(about.contains("\"source\": \"microphone\""), "{about}");
        assert!(!about.contains("person"), "the device's name is never written: {about}");
        let log = std::fs::read_to_string(folder.join("attempts.jsonl")).unwrap_or_else(|e| panic!("{e}"));
        let lines: Vec<Value> = log.lines().map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}"))).collect();
        let of = |kind: &str| lines.iter().filter(|line| line["kind"] == kind).collect::<Vec<_>>();
        assert_eq!(of("live")[0]["title"], "Morning on the Hill");
        assert_eq!(of("live")[0]["what"], "song");
        assert_eq!(of("phrase")[0]["seconds"].to_string(), "7.33");
        let read = of("read")[0];
        assert_eq!(read["heard"], heard);
        assert_eq!(read["near"][0]["title"], "River of Mercy");
        assert_eq!(read["named"][0], "River of Mercy");
        assert_eq!(of("offered").last().map(|line| line["offers"][0]["title"].clone()), Some(json!("River of Mercy")));
        assert_eq!(of("taken")[0]["title"], "River of Mercy");
        // Every line says when it happened, and they are in order.
        let times: Vec<f64> = lines.iter().filter_map(|line| line["t"].as_f64()).collect();
        assert_eq!(times.len(), lines.len());
        assert!(times.windows(2).all(|pair| pair[0] <= pair[1]));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_log_writes_figures_as_short_as_they_are_good_for() {
        assert_eq!(json!(tidy(71.4, 10.0)).to_string(), "71.4");
        assert_eq!(json!(tidy(0.3, 100.0)).to_string(), "0.3");
        assert_eq!(count_name(crate::state::TempoCount::AsHeard), "as heard");
    }

    #[test]
    fn practice_keeps_nothing() {
        let mut app = App::practice();
        app.settings.keep_recordings = true;
        assert_eq!(app.recordings_folder(), None);
        assert_eq!(app.begin_sitting(), None);
        assert!(!app.recording());
    }
}
