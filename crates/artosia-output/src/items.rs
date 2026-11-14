//! The rundown's video and sound: one worker owns the item in front and the scene's music, so a
//! slow file never holds the window. A new item id starts it over, the same id again changes
//! nothing but its level, and no item draws or sounds once the state has none.
//!
//! A video item draws on its own surface, between the background and the words. When it ends
//! with Stop that layer stays and shows black; with Next the last frame stays until the operator
//! shows the next item. Music is lowered to a third while a video with sound plays, and comes
//! back over half a second.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use artosia_slides::frame::{FromOutput, MediaItem, MediaKind, trace};
use artosia_video::{VideoError, VideoLoop, VideoOptions};

use crate::media::{AtEnd, at_end, ducks, gain, music_gain, ramp};
use crate::shared;

type Want = (Option<MediaItem>, Option<MediaItem>);

static WANT: OnceLock<Mutex<Sender<Want>>> = OnceLock::new();

const REPORT: Duration = Duration::from_millis(500);
const STEP: Duration = Duration::from_millis(100);
const IDLE: Duration = Duration::from_millis(250);

/// Asks for the item in front and the music, or for none.
pub fn want(front: Option<MediaItem>, music: Option<MediaItem>) {
    if let Some(tx) = WANT.get().and_then(|tx| tx.lock().ok()) {
        let _ = tx.send((front, music));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Playing,
    Ended,
    Failed,
}

struct Slot {
    item: MediaItem,
    playing: Option<VideoLoop>,
    phase: Phase,
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// Why `item` could not play, as a kind the operator words in its own language (`why.rs`
/// `media_problem`).
fn trouble(item: &MediaItem, error: &VideoError) -> String {
    let name = file_name(&item.path);
    match (item.kind, error) {
        (MediaKind::Video, VideoError::NotFound(_)) => format!("video-missing\t{name}"),
        (MediaKind::Video, _) => format!("video-failed\t{name}"),
        (MediaKind::Audio, VideoError::NotFound(_)) => format!("audio-missing	{name}"),
        (MediaKind::Audio, _) => format!("audio-failed	{name}"),
    }
}

struct Items {
    front: Option<Slot>,
    music: Option<Slot>,
    /// The music's level now, on its way to the level it should have.
    level: f64,
    told_trouble: bool,
    reported: Instant,
}

impl Items {
    fn tell(&mut self, message: String) {
        self.told_trouble = !message.is_empty();
        shared().tell(&FromOutput::MediaError(message));
    }

    /// Black layer until the first frame, then the frames.
    fn clear_layer(&self) {
        let shared = shared();
        shared.item_live.store(false, Ordering::Release);
        shared.item_surface.clear();
        shared.touch();
    }

    /// Tells the background whether a video item now hides it, so it can stop decoding.
    fn covers(&self, on: bool) {
        crate::video::cover(on);
    }

    fn open(&self, item: &MediaItem, front: bool, level: f64) -> Result<VideoLoop, VideoError> {
        let options = VideoOptions { volume: level, muted: level == 0.0, looping: at_end(item.end) == AtEnd::Again, ..VideoOptions::default() };
        let path = Path::new(&item.path);
        if front && item.kind == MediaKind::Video {
            VideoLoop::start(path, shared().item_surface.clone(), options)
        } else {
            VideoLoop::start_audio(path, options)
        }
    }

    fn start(&mut self, item: MediaItem, front: bool) -> Slot {
        let level = if front { gain(item.volume) } else { music_gain(&item, self.front_ducks()) };
        match self.open(&item, front, level) {
            Ok(video) => {
                trace(|| format!("item {} started: {}", item.id, item.path));
                if !front {
                    self.level = level;
                } else if item.kind == MediaKind::Video {
                    shared().item_live.store(true, Ordering::Release);
                    shared().touch();
                    self.covers(true);
                } else {
                    self.covers(false);
                }
                if self.told_trouble {
                    self.tell(String::new());
                }
                Slot { item, playing: Some(video), phase: Phase::Playing }
            }
            Err(error) => {
                trace(|| format!("item {} could not start: {error}", item.id));
                self.covers(false);
                self.tell(trouble(&item, &error));
                Slot { item, playing: None, phase: Phase::Failed }
            }
        }
    }

    fn front_ducks(&self) -> bool {
        let front = self.front.as_ref();
        ducks(front.map(|slot| &slot.item), front.is_some_and(|slot| slot.phase == Phase::Playing))
    }

    fn change(&mut self, old: Option<Slot>, new: Option<MediaItem>, front: bool) -> Option<Slot> {
        match (old, new) {
            (old, None) => {
                if front && old.is_some() {
                    self.clear_layer();
                    self.covers(false);
                }
                None
            }
            (Some(mut slot), Some(new)) if slot.item.id == new.id => {
                if front && new.volume != slot.item.volume && let Some(video) = &slot.playing {
                    video.set_volume(gain(new.volume));
                }
                slot.item = new;
                Some(slot)
            }
            (old, Some(new)) => {
                drop(old);
                if front {
                    self.clear_layer();
                }
                Some(self.start(new, front))
            }
        }
    }

    fn apply(&mut self, (front, music): Want) {
        let old = self.front.take();
        self.front = self.change(old, front, true);
        let old = self.music.take();
        self.music = self.change(old, music, false);
    }

    fn ended(&mut self, front: bool) {
        let Some(slot) = (if front { self.front.as_mut() } else { self.music.as_mut() }) else { return };
        slot.phase = Phase::Ended;
        slot.playing = None;
        let (id, black) = (slot.item.id, at_end(slot.item.end) == AtEnd::Black && slot.item.kind == MediaKind::Video && front);
        trace(|| format!("item {id} ended"));
        let shared = shared();
        shared.tell(&FromOutput::MediaEnded { id });
        if black {
            shared.item_surface.clear();
            shared.touch();
        }
    }

    fn failed(&mut self, front: bool, why: String) {
        let Some(slot) = (if front { self.front.as_mut() } else { self.music.as_mut() }) else { return };
        slot.phase = Phase::Failed;
        slot.playing = None;
        trace(|| format!("item {} failed: {why}", slot.item.id));
        let message = trouble(&slot.item, &VideoError::Pipeline(why));
        if front && slot.item.kind == MediaKind::Video {
            self.clear_layer();
            self.covers(false);
        }
        self.tell(message);
    }

    fn ramping(&self) -> bool {
        self.music.as_ref().is_some_and(|slot| slot.phase == Phase::Playing && (self.level - music_gain(&slot.item, self.front_ducks())).abs() > 1e-9)
    }

    fn tick(&mut self) {
        for front in [true, false] {
            let slot = if front { self.front.as_ref() } else { self.music.as_ref() };
            let Some(video) = slot.filter(|slot| slot.phase == Phase::Playing).and_then(|slot| slot.playing.as_ref()) else { continue };
            if let Some(error) = video.error() {
                self.failed(front, error);
            } else if video.ended() {
                self.ended(front);
            }
        }
        if let Some(slot) = self.front.as_ref().filter(|slot| slot.phase == Phase::Playing)
            && let Some(video) = &slot.playing
            && self.reported.elapsed() >= REPORT
        {
            let position_ms = video.position().map_or(0, |at| at.as_millis() as u64);
            trace(|| format!("item {} pos={position_ms} ms loops={}", slot.item.id, video.loops()));
            let length_ms = video.duration().map_or(0, |length| length.as_millis() as u64);
            shared().tell(&FromOutput::MediaPlaying { id: slot.item.id, position_ms, length_ms });
            self.reported = Instant::now();
        }
        if self.ramping() {
            let target = self.music.as_ref().map_or(0.0, |slot| music_gain(&slot.item, self.front_ducks()));
            self.level = ramp(self.level, target, STEP.as_millis() as u64);
            trace(|| format!("music level {:.2} toward {target:.2}", self.level));
            if let Some(video) = self.music.as_ref().and_then(|slot| slot.playing.as_ref()) {
                video.set_volume(self.level);
            }
        }
    }
}

/// Starts the worker. GStreamer itself starts when the first item is played.
pub fn start() {
    let (tx, rx) = channel::<Want>();
    if WANT.set(Mutex::new(tx)).is_err() {
        return;
    }
    let _ = std::thread::Builder::new().name("artosia-items".into()).spawn(move || {
        let mut items = Items { front: None, music: None, level: 0.0, told_trouble: false, reported: Instant::now() };
        loop {
            let wait = if items.ramping() { STEP } else { IDLE };
            match rx.recv_timeout(wait) {
                Ok(mut wanted) => {
                    while let Ok(newer) = rx.try_recv() {
                        wanted = newer;
                    }
                    items.apply(wanted);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            items.tick();
        }
    });
}
