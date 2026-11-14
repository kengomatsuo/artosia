//! Rundown items: a video, a sound or a scene in the plan, and what Show hands the projector for
//! one. The plan holds the item's end action and level; the projector plays it (`artosia-output`
//! `items.rs`) and says where it is and when it ended.

use std::path::Path;

use artosia_core::deck::{Deck, DeckSource, Slide, SlideKind};
use artosia_core::plan::{Clears, Clip, PlanEntry, RunEnd};
use artosia_slides::frame::{MediaEnd, MediaItem, MediaKind, OutputState};
use artosia_slides::look::Background;
use rune::TextEdit;

use crate::state::App;
use crate::strings as t;

/// What the projector draws for one rundown item, apart from the words.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layers {
    /// The video or sound in front.
    pub front: Option<MediaItem>,
    /// A scene's music, under everything.
    pub music: Option<MediaItem>,
    /// A scene's own background; None keeps the look's.
    pub background: Option<Background>,
    pub logo: bool,
}

/// An item on the projector now: which plan item, the id the projector knows it by, and where it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playing {
    pub item: u64,
    pub id: u64,
    pub position_ms: u64,
    pub length_ms: u64,
    pub ended: bool,
    /// What the scene took away when its video ended.
    pub cleared: Clears,
}

impl Playing {
    /// How far through the file it is, 0 to 1; None while its length is not known.
    pub fn fraction(&self) -> Option<f32> {
        if self.ended {
            Some(1.0)
        } else if self.length_ms > 0 {
            Some((self.position_ms as f32 / self.length_ms as f32).clamp(0.0, 1.0))
        } else {
            None
        }
    }
}

/// The scene being made in its dialog.
pub struct SceneForm {
    pub name: TextEdit,
    pub background: String,
    pub video: Option<Clip>,
    pub music: Option<Clip>,
    pub logo: bool,
    pub clears: Clears,
}

#[derive(Default)]
pub struct Run {
    serial: u64,
    pub playing: Option<Playing>,
    pub form: Option<SceneForm>,
    /// A file dialog to open at the next poll, which has the window's `Tx`.
    pub pick: Option<Pick>,
    /// The pictures behind scenes of the plan, decoded for the operator's small pictures.
    pub pictures: std::collections::HashMap<String, rune::Image>,
}

/// What a file is being chosen for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Video,
    Audio,
    Layer(Layer),
}

const VIDEO_FILES: &[&str] = &["mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv"];
const AUDIO_FILES: &[&str] = &["mp3", "wav", "ogg", "flac", "m4a", "aac", "opus"];
const BACKGROUND_FILES: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif", "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv"];

impl Pick {
    /// The dialog's title and the kinds of file it lists.
    pub fn dialog(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Pick::Video | Pick::Layer(Layer::Video) => (t::RUN_PICK_VIDEO(), VIDEO_FILES),
            Pick::Audio | Pick::Layer(Layer::Music) => (t::RUN_PICK_AUDIO(), AUDIO_FILES),
            Pick::Layer(_) => (t::RUN_PICK_BACKGROUND(), BACKGROUND_FILES),
        }
    }
}

pub fn end_of(end: RunEnd) -> MediaEnd {
    match end {
        RunEnd::Stop => MediaEnd::Stop,
        RunEnd::Loop => MediaEnd::Loop,
        RunEnd::Next => MediaEnd::Next,
    }
}

/// A picture or a video as a background; None for any other file.
pub fn background_of(path: &str) -> Option<Background> {
    let extension = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" => Some(Background::Image(path.to_string())),
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" | "wmv" => Some(Background::Video(path.to_string())),
        _ => None,
    }
}

/// What `entry` puts on the projector as the showing `id`, once the scene has taken away `cleared`.
/// A scene's music is the next id, so the projector tells a music's end from the video's.
pub fn layers(entry: &PlanEntry, id: u64, cleared: Clears) -> Layers {
    let item = |id, clip: &Clip, kind, end| MediaItem { id, path: clip.path.clone(), kind, end, volume: clip.volume };
    match entry {
        PlanEntry::Video { path, end, volume, .. } => Layers { front: Some(MediaItem { id, path: path.clone(), kind: MediaKind::Video, end: end_of(*end), volume: *volume }), ..Layers::default() },
        PlanEntry::Audio { path, end, volume, .. } => Layers { front: Some(MediaItem { id, path: path.clone(), kind: MediaKind::Audio, end: end_of(*end), volume: *volume }), ..Layers::default() },
        PlanEntry::Scene { background, video, music, logo, .. } => Layers {
            front: video.as_ref().filter(|_| !cleared.video).map(|clip| item(id, clip, MediaKind::Video, MediaEnd::Stop)),
            music: music.as_ref().filter(|_| !cleared.music).map(|clip| item(id + 1, clip, MediaKind::Audio, MediaEnd::Loop)),
            background: background_of(background),
            logo: *logo && !cleared.logo,
        },
        _ => Layers::default(),
    }
}

/// Whether the room is left looking at black when `entry` ends: a video that stops, and a scene
/// whose video stops and is not cleared (the layer stays and is black).
fn ends_black(entry: &PlanEntry) -> bool {
    match entry {
        PlanEntry::Video { end, .. } => *end == RunEnd::Stop,
        PlanEntry::Scene { video, clears, .. } => video.is_some() && !clears.video,
        _ => false,
    }
}

/// What a scene takes away, and what a video or sound does at its end, for the plan item `entry`.
fn ending(entry: &PlanEntry) -> (RunEnd, Clears) {
    match entry {
        PlanEntry::Video { end, .. } | PlanEntry::Audio { end, .. } => (*end, Clears::default()),
        PlanEntry::Scene { clears, .. } => (RunEnd::Stop, *clears),
        _ => (RunEnd::Stop, Clears::default()),
    }
}

impl App {
    /// The deck of a rundown item: one slide that names it and says nothing to the room. None
    /// when a file it needs is gone, so the row says so and Show does nothing.
    pub fn deck_of_media(&self, entry: &PlanEntry) -> Option<Deck> {
        let key = entry.media_key()?;
        if entry.files().iter().any(|file| !Path::new(file).is_file()) {
            return None;
        }
        let title = entry.title().to_string();
        let slide = Slide { kind: SlideKind::Lyrics, label: title.clone(), ..Slide::default() };
        Some(Deck { title, subtitle: String::new(), source: DeckSource::Media(key), slides: vec![slide] })
    }

    /// For the scenes that show rundown items: a plan of a song, a video that goes on to the next
    /// item, a sound that loops and a scene, on made-up empty files. Returns the three items' ids.
    pub fn demo_media(&mut self) -> (u64, u64, u64) {
        let keep: Vec<u64> = self.plan.items.iter().skip(1).take(2).map(|item| item.id).collect();
        self.plan.items.retain(|item| keep.contains(&item.id));
        let folder = std::env::temp_dir().join("artosia-practice").join("rundown");
        let file = |name: &str| {
            let _ = std::fs::create_dir_all(&folder);
            let path = folder.join(name);
            let _ = std::fs::File::create(&path);
            path.to_string_lossy().into_owned()
        };
        let video = self.plan.push(PlanEntry::Video { path: file("testimony.mp4"), title: "Testimony".into(), end: RunEnd::Next, volume: 80 });
        let audio = self.plan.push(PlanEntry::Audio { path: file("prayer-music.mp3"), title: "Prayer music".into(), end: RunEnd::Loop, volume: 60 });
        let scene = self.plan.push(PlanEntry::Scene {
            title: "Closing".into(),
            background: file("night.png"),
            video: Some(Clip { path: file("closing.mp4"), volume: 70 }),
            music: Some(Clip { path: file("closing-music.mp3"), volume: 50 }),
            logo: true,
            clears: Clears { video: true, music: true, logo: false },
        });
        (video, audio, scene)
    }

    /// A file a rundown item needs is not where it was.
    pub fn media_missing(entry: &PlanEntry) -> bool {
        entry.files().iter().any(|file| !Path::new(file).is_file())
    }

    /// Show on a rundown item: each showing is a new id, so the same file shown again starts over.
    pub(crate) fn begin_media(&mut self) {
        let item = match &self.live {
            Some(live) if matches!(live.deck.source, DeckSource::Media(_)) => live.item,
            _ => None,
        };
        self.run.playing = item.map(|item| {
            self.run.serial += 2;
            Playing { item, id: self.run.serial, position_ms: 0, length_ms: 0, ended: false, cleared: Clears::default() }
        });
    }

    /// What the projector draws for the live item: nothing once the live item is another one.
    pub fn media_layers(&self) -> Layers {
        let (Some(live), Some(playing)) = (&self.live, &self.run.playing) else { return Layers::default() };
        if live.item != Some(playing.item) {
            return Layers::default();
        }
        match self.plan.items.iter().find(|item| item.id == playing.item) {
            Some(item) => layers(&item.entry, playing.id, playing.cleared),
            None => Layers::default(),
        }
    }

    /// Decodes the pictures scenes of the plan show behind them, and lets go of those no scene wants.
    fn load_scene_pictures(&mut self) {
        let Some(images) = &self.images else { return };
        let wanted: Vec<&str> = self
            .plan
            .items
            .iter()
            .filter_map(|item| match &item.entry {
                PlanEntry::Scene { background, .. } if matches!(background_of(background), Some(Background::Image(_))) => Some(background.as_str()),
                _ => None,
            })
            .collect();
        self.run.pictures.retain(|path, _| wanted.contains(&path.as_str()));
        for path in wanted {
            if !self.run.pictures.contains_key(path) {
                self.run.pictures.insert(path.to_string(), images.file(path, 640.0, 1.0));
            }
        }
    }

    /// Preview's picture of a scene that is open: its own background and its logo, as the room
    /// would see them once it is shown.
    pub fn with_scene_look(&self, mut state: OutputState) -> OutputState {
        let item = self.open.as_ref().and_then(|open| open.item);
        if let Some(PlanEntry::Scene { background, logo, .. }) = item.and_then(|id| self.plan.items.iter().find(|entry| entry.id == id)).map(|entry| &entry.entry) {
            if let Some(background) = background_of(background) {
                state.look.background = background;
            }
            state.logo_on = *logo;
        }
        state
    }

    /// The live row's progress, 0 to 1.
    pub fn media_progress(&self, item: u64) -> Option<f32> {
        self.run.playing.filter(|playing| playing.item == item).and_then(|playing| playing.fraction())
    }

    /// Show on a video or sound with a projector that cannot play it says so and shows nothing.
    pub(crate) fn media_blocked(&mut self) -> bool {
        if !self.open.as_ref().is_some_and(|open| matches!(open.deck.source, DeckSource::Media(_))) {
            return false;
        }
        let status = self.link.status();
        if status.connected && !status.video {
            self.say(t::MEDIA_ITEM_NONE(), None);
            return true;
        }
        false
    }

    /// Takes what the projector said about the item in front. An item set to Next moves the plan
    /// on when it ends; a scene's music or any older showing is told apart by its id and ignored.
    pub fn poll_media(&mut self) -> bool {
        self.load_scene_pictures();
        let (ended, at) = self.link.take_media();
        let mut changed = false;
        if let (Some((id, position_ms, length_ms)), Some(playing)) = (at, self.run.playing.as_mut())
            && playing.id == id
        {
            (playing.position_ms, playing.length_ms) = (position_ms, length_ms);
            changed = true;
        }
        for id in ended {
            let Some(playing) = self.run.playing.as_mut().filter(|playing| playing.id == id) else { continue };
            playing.ended = true;
            changed = true;
            let item = playing.item;
            let Some(((end, clears), black)) = self.plan.items.iter().find(|entry| entry.id == item).map(|entry| (ending(&entry.entry), ends_black(&entry.entry))) else { continue };
            if black {
                // The projector keeps its last frame in its own surface and draws black over it; the small picture goes black too.
                self.link.inject_live_picture(1, 1, vec![0, 0, 0, 255]);
            }
            if let Some(playing) = self.run.playing.as_mut() {
                playing.cleared = clears;
            }
            if end == RunEnd::Next && self.live.as_ref().is_some_and(|live| live.item == Some(item)) {
                self.next();
            }
        }
        changed
    }

    /// Changes what a video or sound does at its end.
    pub fn set_run_end(&mut self, id: u64, to: RunEnd) {
        if let Some(PlanEntry::Video { end, .. } | PlanEntry::Audio { end, .. }) = self.plan.items.iter_mut().find(|item| item.id == id).map(|item| &mut item.entry)
            && *end != to
        {
            *end = to;
            self.touch_plan();
        }
    }

    /// Changes a video's or sound's level, 0 to 100.
    pub fn set_run_volume(&mut self, id: u64, to: u8) {
        if let Some(PlanEntry::Video { volume, .. } | PlanEntry::Audio { volume, .. }) = self.plan.items.iter_mut().find(|item| item.id == id).map(|item| &mut item.entry)
            && *volume != to
        {
            *volume = to;
            self.touch_plan();
        }
    }

    /// A file was chosen in a file dialog.
    pub fn picked(&mut self, pick: Pick, path: &Path) {
        match pick {
            Pick::Video => self.add_media(true, path),
            Pick::Audio => self.add_media(false, path),
            Pick::Layer(layer) => self.scene_file(layer, path),
        }
    }

    /// Puts a chosen video or sound in the plan, named by its file.
    pub fn add_media(&mut self, video: bool, path: &Path) {
        let title = path.file_stem().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let path = path.to_string_lossy().into_owned();
        let entry = if video { PlanEntry::Video { path, title: title.clone(), end: RunEnd::Stop, volume: 100 } } else { PlanEntry::Audio { path, title: title.clone(), end: RunEnd::Stop, volume: 100 } };
        self.queue_entry(entry);
        self.requeue();
        self.add_menu = false;
        self.say(t::added_to_plan(&title), None);
    }

    pub fn open_scene_form(&mut self) {
        let mut name = TextEdit::new("");
        name.focused = true;
        self.run.form = Some(SceneForm { name, background: String::new(), video: None, music: None, logo: false, clears: Clears::default() });
        self.add_menu = false;
    }

    pub fn close_scene_form(&mut self) {
        self.run.form = None;
    }

    /// A scene needs something to show: a layer, or a name for a scene that only keeps the look.
    pub fn scene_ready(&self) -> bool {
        self.run.form.as_ref().is_some_and(|form| !form.background.is_empty() || form.video.is_some() || form.music.is_some() || form.logo)
    }

    pub fn save_scene(&mut self) {
        if !self.scene_ready() {
            return;
        }
        let Some(form) = self.run.form.take() else { return };
        let typed = form.name.text.trim();
        let title = if typed.is_empty() { t::PLAN_ADD_SCENE().to_string() } else { typed.to_string() };
        self.queue_entry(PlanEntry::Scene { title: title.clone(), background: form.background, video: form.video, music: form.music, logo: form.logo, clears: form.clears });
        self.requeue();
        self.say(t::added_to_plan(&title), None);
    }

    /// A layer's file was chosen in the scene's dialog. Music takes a sound and the video a video.
    pub fn scene_file(&mut self, layer: Layer, path: &Path) {
        let Some(form) = self.run.form.as_mut() else { return };
        let text = path.to_string_lossy().into_owned();
        match layer {
            Layer::Background if background_of(&text).is_some() => form.background = text,
            Layer::Background => self.say(t::BACKGROUND_UNKNOWN(), None),
            Layer::Logo => {}
            Layer::Video => form.video = Some(Clip { path: text, volume: form.video.as_ref().map_or(100, |clip| clip.volume) }),
            Layer::Music => form.music = Some(Clip { path: text, volume: form.music.as_ref().map_or(60, |clip| clip.volume) }),
        }
    }

    pub fn scene_clear(&mut self, layer: Layer) {
        let Some(form) = self.run.form.as_mut() else { return };
        match layer {
            Layer::Background => form.background.clear(),
            Layer::Video => form.video = None,
            Layer::Music => form.music = None,
            Layer::Logo => form.logo = false,
        }
    }

    pub fn scene_volume(&mut self, layer: Layer, to: u8) {
        let Some(form) = self.run.form.as_mut() else { return };
        let clip = match layer {
            Layer::Video => form.video.as_mut(),
            Layer::Music => form.music.as_mut(),
            Layer::Background | Layer::Logo => None,
        };
        if let Some(clip) = clip {
            clip.volume = to;
        }
    }

    pub fn scene_clears(&mut self, layer: Layer, on: bool) {
        let Some(form) = self.run.form.as_mut() else { return };
        match layer {
            Layer::Video => form.clears.video = on,
            Layer::Music => form.clears.music = on,
            Layer::Logo => form.clears.logo = on,
            Layer::Background => {}
        }
    }
}

/// A layer of a scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Background,
    Video,
    Music,
    Logo,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene() -> PlanEntry {
        PlanEntry::Scene {
            title: "Closing".into(),
            background: "b.png".into(),
            video: Some(Clip { path: "c.mp4".into(), volume: 70 }),
            music: Some(Clip { path: "m.mp3".into(), volume: 50 }),
            logo: true,
            clears: Clears { video: true, music: true, logo: false },
        }
    }

    #[test]
    fn a_video_item_goes_to_the_projector_with_its_end_and_level() {
        let entry = PlanEntry::Video { path: "v.mp4".into(), title: "T".into(), end: RunEnd::Next, volume: 80 };
        let shown = layers(&entry, 6, Clears::default());
        assert_eq!(shown.front, Some(MediaItem { id: 6, path: "v.mp4".into(), kind: MediaKind::Video, end: MediaEnd::Next, volume: 80 }));
        assert_eq!((shown.music, shown.background, shown.logo), (None, None, false));
        let sound = PlanEntry::Audio { path: "a.mp3".into(), title: "T".into(), end: RunEnd::Loop, volume: 40 };
        assert_eq!(layers(&sound, 8, Clears::default()).front.map(|item| (item.kind, item.end)), Some((MediaKind::Audio, MediaEnd::Loop)));
        assert_eq!(layers(&PlanEntry::Announcements, 1, Clears::default()), Layers::default());
    }

    #[test]
    fn a_scene_draws_its_layers_and_loses_what_it_clears() {
        let whole = layers(&scene(), 4, Clears::default());
        assert_eq!(whole.front.as_ref().map(|item| (item.id, item.end)), Some((4, MediaEnd::Stop)));
        assert_eq!(whole.music.as_ref().map(|item| (item.id, item.end, item.volume)), Some((5, MediaEnd::Loop, 50)), "the music has an id of its own");
        assert_eq!(whole.background, Some(Background::Image("b.png".into())));
        assert!(whole.logo);
        let PlanEntry::Scene { clears, .. } = scene() else { unreachable!() };
        let after = layers(&scene(), 4, clears);
        assert_eq!((after.front, after.music, after.logo), (None, None, true));
        assert_eq!(after.background, Some(Background::Image("b.png".into())), "the background stays");
    }

    #[test]
    fn only_a_picture_or_a_video_is_a_background() {
        assert_eq!(background_of("w.MP4"), Some(Background::Video("w.MP4".into())));
        assert_eq!(background_of("p.jpg"), Some(Background::Image("p.jpg".into())));
        assert_eq!(background_of("song.mp3"), None);
        assert_eq!(background_of(""), None);
    }

    #[test]
    fn progress_is_the_position_over_the_length_and_full_once_ended() {
        let mut playing = Playing { item: 1, id: 2, position_ms: 0, length_ms: 0, ended: false, cleared: Clears::default() };
        assert_eq!(playing.fraction(), None, "the length is not known yet");
        (playing.position_ms, playing.length_ms) = (30_000, 120_000);
        assert_eq!(playing.fraction(), Some(0.25));
        playing.position_ms = 130_000;
        assert_eq!(playing.fraction(), Some(1.0));
        (playing.position_ms, playing.ended) = (5, true);
        assert_eq!(playing.fraction(), Some(1.0));
    }

    fn live_item(app: &App) -> Option<u64> {
        app.live.as_ref().and_then(|live| live.item)
    }

    #[test]
    fn show_sends_the_item_with_a_new_id_each_time_and_leaving_clears_it() {
        let mut app = App::practice();
        let (video, audio, _) = app.demo_media();
        app.open_plan_item(video);
        app.show(0);
        let first = app.output_state().media.expect("the video goes to the projector");
        assert_eq!((first.kind, first.end, first.volume, first.path.ends_with("testimony.mp4")), (MediaKind::Video, MediaEnd::Next, 80, true));
        // Show goes on from what Preview holds, so the video is opened again to be shown again.
        app.open_plan_item(video);
        app.show(0);
        let again = app.output_state().media.expect("shown again");
        assert_ne!(first.id, again.id, "the same file shown again starts over");
        // Changing the level while it plays keeps the id: the projector only changes the volume.
        app.set_run_volume(video, 30);
        let louder = app.output_state().media.expect("still there");
        assert_eq!((louder.id, louder.volume), (again.id, 30));
        app.open_plan_item(audio);
        app.show(0);
        let sound = app.output_state().media.expect("the sound");
        assert_eq!((sound.kind, sound.end), (MediaKind::Audio, MediaEnd::Loop));
        let song = app.plan.items.iter().find(|item| matches!(item.entry, PlanEntry::Song { .. })).map(|item| item.id).expect("a song");
        app.open_plan_item(song);
        app.show(0);
        assert_eq!(app.output_state().media, None, "leaving the item clears the projector");
    }

    #[test]
    fn a_video_set_to_next_moves_the_plan_on_when_it_ends_and_a_stop_does_not() {
        let mut app = App::practice();
        let (video, audio, _) = app.demo_media();
        app.open_plan_item(video);
        app.show(0);
        let id = app.run.playing.expect("playing").id;
        app.link.inject_media(&[id], Some((id, 5_000, 10_000)));
        assert!(app.poll_media());
        assert_eq!(live_item(&app), Some(audio), "Next moved on to the next plan item");
        // The sound is set to Loop and a Stop item beside it must not move the plan.
        app.set_run_end(audio, RunEnd::Stop);
        app.open_plan_item(audio);
        app.show(0);
        let id = app.run.playing.expect("playing").id;
        app.link.inject_media(&[id], None);
        app.poll_media();
        assert_eq!(live_item(&app), Some(audio), "a Stop item waits");
        assert_eq!(app.media_progress(audio), Some(1.0));
    }

    #[test]
    fn a_scene_s_music_ending_never_moves_the_plan_and_its_video_clears_what_it_says() {
        let mut app = App::practice();
        let (_, _, scene) = app.demo_media();
        app.open_plan_item(scene);
        app.show(0);
        let playing = app.run.playing.expect("playing");
        let shown = app.output_state();
        assert_eq!((shown.media.as_ref().map(|item| item.id), shown.music.as_ref().map(|item| item.id)), (Some(playing.id), Some(playing.id + 1)));
        assert!(matches!(&shown.look.background, Background::Image(path) if path.ends_with("night.png")), "the scene's own background");
        assert!(shown.logo_on);
        // The music's id is not the item's: nothing happens.
        app.link.inject_media(&[playing.id + 1], None);
        assert!(!app.poll_media());
        assert_eq!(live_item(&app), Some(scene));
        assert!(app.output_state().music.is_some());
        // The video's end takes away the video and the music, and keeps the logo.
        app.link.inject_media(&[playing.id], None);
        app.poll_media();
        let after = app.output_state();
        assert_eq!((after.media, after.music, after.logo_on), (None, None, true));
    }

    #[test]
    fn checking_a_scene_s_files_on_every_redraw_costs_microseconds() {
        let mut app = App::practice();
        let (_, _, scene) = app.demo_media();
        let entry = app.plan.items.iter().find(|item| item.id == scene).map(|item| item.entry.clone()).expect("the scene");
        let start = std::time::Instant::now();
        let rounds = 2_000;
        for _ in 0..rounds {
            assert!(!App::media_missing(&entry));
        }
        let each = start.elapsed() / rounds;
        eprintln!("media_missing on a four-file scene: {each:?} per call");
        assert!(each < std::time::Duration::from_millis(1), "{each:?}");
    }
}
