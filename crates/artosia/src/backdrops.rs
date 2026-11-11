//! Ready backdrops: browse the catalogue, fetch a picture or a video on a press, put it behind the
//! words. The catalogue holds addresses only (`artosia_core::backdrops`); a file comes from its
//! source's own server when a volunteer presses the button for it, and works without the
//! internet afterwards. Previews are small, loaded as tiles come into view and kept on disk.
//! Downloads and previews run on worker threads whose answers wait in `events` for the next frame.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use artosia_core::backdrops::{self, Catalogue, Entry, Kind};
use rune::TextEdit;

use crate::state::{App, Sheet};
use crate::strings as t;

/// Tiles in a row of the grid.
pub const COLUMNS: usize = 4;

/// Previews fetched at the same time.
const PREVIEWS_AT_ONCE: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    Offline,
    Cancelled,
    Server(u16),
    Disk(String),
    Empty,
}

pub enum Event {
    Downloaded(String, Result<PathBuf, Failure>),
    Preview(String, bool),
}

/// The download in progress: shared counters the worker moves and the window reads.
pub struct Fetch {
    pub id: String,
    pub got: Arc<AtomicU64>,
    pub total: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
}

impl Fetch {
    /// How far along, 0 to 1; None while the server has not said how big the file is.
    pub fn fraction(&self) -> Option<f32> {
        match self.total.load(Ordering::Relaxed) {
            0 => None,
            total => Some((self.got.load(Ordering::Relaxed) as f32 / total as f32).clamp(0.0, 1.0)),
        }
    }
}

pub struct Desk {
    pub catalogue: Catalogue,
    /// The chip chosen; None is "All".
    pub category: Option<String>,
    pub query: TextEdit,
    /// The backdrop whose large preview shows, by id.
    pub picked: Option<String>,
    /// The catalogue's indices to show, for `shown_for`.
    pub shown: Vec<usize>,
    shown_for: Option<(Option<String>, String)>,
    pub fetch: Option<Fetch>,
    /// Set the background when the download that is running arrives.
    apply: bool,
    pub events: Arc<Mutex<Vec<Event>>>,
    /// Ids whose file is on disk.
    pub have: HashSet<String>,
    /// Ids whose preview is on disk.
    pub previews: HashSet<String>,
    pub failed: HashSet<String>,
    loading: HashSet<String>,
    /// The rows of tiles on screen, as the grid last reported them.
    visible: std::ops::Range<usize>,
}

impl Default for Desk {
    fn default() -> Self {
        Desk::with(backdrops::built_in())
    }
}

impl Desk {
    pub fn with(catalogue: Catalogue) -> Desk {
        Desk {
            catalogue,
            category: None,
            query: TextEdit::new(""),
            picked: None,
            shown: Vec::new(),
            shown_for: None,
            fetch: None,
            apply: false,
            events: Arc::default(),
            have: HashSet::new(),
            previews: HashSet::new(),
            failed: HashSet::new(),
            loading: HashSet::new(),
            visible: 0..0,
        }
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.catalogue.entries.iter().find(|entry| entry.id == id)
    }

    pub fn picked_entry(&self) -> Option<&Entry> {
        self.entry(self.picked.as_deref()?)
    }

    /// Arranges the list for the chip and the words typed. Cheap to call every frame.
    pub fn arrange(&mut self) -> bool {
        let key = (self.category.clone(), self.query.text.trim().to_string());
        if self.shown_for.as_ref() == Some(&key) {
            return false;
        }
        self.shown = backdrops::matching(&self.catalogue.entries, key.0.as_deref(), &key.1);
        self.shown_for = Some(key);
        true
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(8)))
        // A line that stalls for this long has dropped; a slow one that keeps sending is let be.
        .timeout_per_read(Some(Duration::from_secs(20)))
        .user_agent(concat!("Artosia/", env!("CARGO_PKG_VERSION"), " (free worship presentation app)"))
        .build()
        .into()
}

fn failure_of(error: ureq::Error) -> Failure {
    match error {
        ureq::Error::StatusCode(code) => Failure::Server(code),
        ureq::Error::Timeout(_) | ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => Failure::Offline,
        other => Failure::Disk(other.to_string()),
    }
}

/// Fetches `url` into `target`, through a `.part` file renamed when whole, so a half file is never
/// taken for a backdrop. `got` and `total` move as bytes arrive; `cancel` stops it.
pub fn download(url: &str, target: &Path, got: &AtomicU64, total: &AtomicU64, cancel: &AtomicBool, limit: u64) -> Result<(), Failure> {
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(|error| Failure::Disk(error.to_string()))?;
    }
    let mut response = match agent().get(url).call() {
        // The picture site asks a busy caller to wait: one more try after a pause, and a person
        // who presses again in a minute gets it. It answered 429 to the first real press here.
        Err(ureq::Error::StatusCode(429 | 503)) => {
            for _ in 0..30 {
                if cancel.load(Ordering::Relaxed) {
                    return Err(Failure::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            agent().get(url).call().map_err(failure_of)?
        }
        other => other.map_err(failure_of)?,
    };
    total.store(response.body().content_length().unwrap_or(0), Ordering::Relaxed);
    let part = target.with_extension("part");
    let result = (|| {
        let mut file = std::fs::File::create(&part).map_err(|error| Failure::Disk(error.to_string()))?;
        let mut reader = response.body_mut().as_reader();
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(Failure::Cancelled);
            }
            let read = reader.read(&mut buffer).map_err(|_| Failure::Offline)?;
            if read == 0 {
                break;
            }
            let so_far = got.fetch_add(read as u64, Ordering::Relaxed) + read as u64;
            if so_far > limit {
                return Err(Failure::Disk("the file is larger than expected".into()));
            }
            file.write_all(&buffer[..read]).map_err(|error| Failure::Disk(error.to_string()))?;
        }
        file.flush().map_err(|error| Failure::Disk(error.to_string()))?;
        if got.load(Ordering::Relaxed) == 0 {
            return Err(Failure::Empty);
        }
        Ok(())
    })();
    match result {
        Ok(()) => std::fs::rename(&part, target).map_err(|error| Failure::Disk(error.to_string())),
        Err(failure) => {
            let _ = std::fs::remove_file(&part);
            Err(failure)
        }
    }
}

/// The ids whose files are in `folder`, read once when the browser opens and after each download.
fn scan(folder: &Path, suffix: &str) -> HashSet<String> {
    let Ok(entries) = std::fs::read_dir(folder) else { return HashSet::new() };
    entries
        .flatten()
        .filter(|item| item.metadata().is_ok_and(|meta| meta.is_file() && meta.len() > 0))
        .filter_map(|item| {
            let name = item.file_name().to_string_lossy().to_string();
            let stem = name.rsplit_once('.').map_or(name.as_str(), |(stem, _)| stem).to_string();
            (suffix.is_empty() || name.ends_with(suffix)).then_some(stem)
        })
        .collect()
}

/// Largest file a backdrop may be: a source that sends more is not what the catalogue promised.
const LIMIT: u64 = 200 * 1024 * 1024;

impl App {
    /// Where the library keeps downloaded media; the practice library borrows a temporary folder.
    pub fn backdrop_media(&self) -> PathBuf {
        self.folder.clone().unwrap_or_else(|| std::env::temp_dir().join("artosia-practice")).join("media")
    }

    /// Opens the browser: reads which backdrops and previews are already on disk.
    pub fn open_backdrops(&mut self) {
        self.sheet = Sheet::Backdrops;
        // A preview that did not arrive last time is asked for again.
        self.backdrops.failed.clear();
        // The search box is what a person opens this for, so typing works at once.
        self.backdrops.query.focused = true;
        self.refresh_backdrops();
    }

    pub fn refresh_backdrops(&mut self) {
        let media = self.backdrop_media();
        let folder = backdrops::folder(&media);
        let ids: HashSet<String> = self.backdrops.catalogue.entries.iter().map(|entry| entry.id.clone()).collect();
        self.backdrops.have = scan(&folder, "").into_iter().filter(|stem| ids.contains(stem)).collect();
        self.backdrops.previews = scan(&folder.join("previews"), ".jpg").into_iter().filter(|stem| ids.contains(stem)).collect();
        self.backdrops.arrange();
    }

    pub fn pick_backdrop(&mut self, id: &str) {
        self.backdrops.picked = Some(id.to_string());
    }

    pub fn choose_backdrop_category(&mut self, category: Option<&str>) {
        self.backdrops.category = category.map(str::to_string);
        self.backdrops.arrange();
    }

    /// The one button: put the backdrop behind the words, fetching it first when it is not here yet.
    pub fn use_backdrop(&mut self, id: &str) {
        let Some(entry) = self.backdrops.entry(id).cloned() else { return };
        let path = backdrops::local_path(&self.backdrop_media(), &entry);
        if backdrops::is_downloaded(&self.backdrop_media(), &entry) {
            self.set_background_file(&path);
            self.back_to_design();
            return;
        }
        self.download_backdrop(id, true);
    }

    /// A backdrop was put behind the words: Design opens again, where its sample shows it and
    /// the sliders that dim a bright picture are.
    fn back_to_design(&mut self) {
        if self.sheet == Sheet::Backdrops {
            self.sheet = Sheet::Looks;
        }
    }

    /// Starts fetching a backdrop. Only a press calls this.
    pub fn download_backdrop(&mut self, id: &str, apply: bool) {
        let Some(entry) = self.backdrops.entry(id).cloned() else { return };
        if self.backdrops.fetch.is_some() {
            return;
        }
        if rune::is_headless() {
            self.say(t::BACKDROPS_OFFLINE(), None);
            return;
        }
        let target = backdrops::local_path(&self.backdrop_media(), &entry);
        let fetch = Fetch { id: id.to_string(), got: Arc::default(), total: Arc::default(), cancel: Arc::default() };
        let (got, total, cancel) = (fetch.got.clone(), fetch.total.clone(), fetch.cancel.clone());
        self.backdrops.fetch = Some(fetch);
        self.backdrops.apply = apply;
        let (events, waker, id) = (self.backdrops.events.clone(), self.waker.clone(), id.to_string());
        std::thread::spawn(move || {
            let result = download(&entry.url, &target, &got, &total, &cancel, LIMIT).map(|()| target);
            if let Ok(mut events) = events.lock() {
                events.push(Event::Downloaded(id, result));
            }
            if let Some(wake) = waker {
                wake();
            }
        });
    }

    pub fn cancel_backdrop(&mut self) {
        if let Some(fetch) = &self.backdrops.fetch {
            fetch.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// The grid reports which rows of tiles are on screen: their previews are fetched, a few at a time.
    pub fn show_rows(&mut self, rows: std::ops::Range<usize>) {
        self.backdrops.visible = rows;
        self.fetch_previews();
    }

    fn fetch_previews(&mut self) {
        // A headless run draws and tests the window and asks nothing of the internet: a test
        // that replays its steps would ask the picture site for the same files again and again.
        if rune::is_headless() {
            return;
        }
        let media = self.backdrop_media();
        let rows = self.backdrops.visible.clone();
        let ids: Vec<String> = (rows.start * COLUMNS..(rows.end * COLUMNS).min(self.backdrops.shown.len()))
            .filter_map(|at| self.backdrops.catalogue.entries.get(*self.backdrops.shown.get(at)?).map(|entry| entry.id.clone()))
            .collect();
        for id in ids {
            if self.backdrops.loading.len() >= PREVIEWS_AT_ONCE {
                break;
            }
            if self.backdrops.previews.contains(&id) || self.backdrops.failed.contains(&id) {
                continue;
            }
            let Some(entry) = self.backdrops.entry(&id).cloned() else { continue };
            if !self.backdrops.loading.insert(id.clone()) {
                continue;
            }
            let target = backdrops::preview_path(&media, &entry);
            let (events, waker) = (self.backdrops.events.clone(), self.waker.clone());
            std::thread::spawn(move || {
                let (got, total, cancel) = (AtomicU64::new(0), AtomicU64::new(0), AtomicBool::new(false));
                let ok = download(&entry.preview, &target, &got, &total, &cancel, 8 * 1024 * 1024).is_ok();
                if let Ok(mut events) = events.lock() {
                    events.push(Event::Preview(entry.id.clone(), ok));
                }
                if let Some(wake) = waker {
                    wake();
                }
            });
        }
    }

    /// Runs before every frame: takes what the workers did and asks for the previews now in view.
    pub fn poll_backdrops(&mut self) -> bool {
        let mut changed = false;
        let arrived: Vec<Event> = self.backdrops.events.lock().map(|mut events| std::mem::take(&mut *events)).unwrap_or_default();
        for event in arrived {
            changed = true;
            match event {
                Event::Preview(id, ok) => {
                    self.backdrops.loading.remove(&id);
                    if ok {
                        self.backdrops.previews.insert(id);
                    } else {
                        self.backdrops.failed.insert(id);
                    }
                }
                Event::Downloaded(id, result) => {
                    self.backdrops.fetch = None;
                    let apply = std::mem::take(&mut self.backdrops.apply);
                    match result {
                        Ok(path) => {
                            self.backdrops.have.insert(id.clone());
                            if apply {
                                self.set_background_file(&path);
                                self.back_to_design();
                            }
                        }
                        Err(Failure::Cancelled) => {}
                        Err(Failure::Offline) => self.say(t::BACKDROPS_OFFLINE(), None),
                        Err(other) => self.say(t::backdrop_failed(&crate::why::backdrop(&other)), None),
                    }
                }
            }
        }
        if self.sheet == Sheet::Backdrops {
            changed |= self.backdrops.arrange();
            if changed {
                // A preview arrived or the list changed: the slots may be free for the next tiles.
                self.fetch_previews();
            }
        }
        changed
    }

    /// Puts the browser in a state for a headless render: "ready", "picked", "picked-video",
    /// "downloading", "downloaded", "offline", "empty" or "category".
    pub fn demo_backdrops(&mut self, state: &str) {
        self.backdrops = Desk::with(demo_catalogue());
        let media = self.backdrop_media();
        let _ = std::fs::remove_dir_all(backdrops::folder(&media));
        for entry in &self.backdrops.catalogue.entries.clone() {
            let (pixels, shade) = (demo_pixels(&entry.id), entry.brightness.unwrap_or(40));
            let path = backdrops::preview_path(&media, entry);
            if let Some(folder) = path.parent() {
                let _ = std::fs::create_dir_all(folder);
            }
            let _ = std::fs::write(&path, demo_jpeg(&pixels, shade));
        }
        self.open_backdrops();
        match state {
            "picked" => self.backdrops.picked = Some("demo-03".to_string()),
            "picked-video" => self.backdrops.picked = Some("demo-24".to_string()),
            "downloaded" => {
                self.backdrops.picked = Some("demo-03".to_string());
                for id in ["demo-03", "demo-07", "demo-24"] {
                    self.backdrops.have.insert(id.to_string());
                    if let Some(entry) = self.backdrops.entry(id).cloned() {
                        let path = backdrops::local_path(&media, &entry);
                        let _ = std::fs::write(&path, demo_jpeg(&demo_pixels(id), entry.brightness.unwrap_or(40)));
                    }
                }
            }
            "downloading" => {
                self.backdrops.picked = Some("demo-05".to_string());
                let fetch = Fetch { id: "demo-05".into(), got: Arc::new(AtomicU64::new(18)), total: Arc::new(AtomicU64::new(60)), cancel: Arc::default() };
                self.backdrops.fetch = Some(fetch);
            }
            "offline" => {
                self.backdrops.picked = Some("demo-05".to_string());
                self.say(t::BACKDROPS_OFFLINE(), None);
            }
            "empty" => self.backdrops.query.replace_text("zzzqq"),
            "category" => self.backdrops.category = Some("night".to_string()),
            _ => {}
        }
        self.backdrops.arrange();
    }
}

/// Made-up entries for a headless render: no real file's name stands in a screenshot.
fn demo_catalogue() -> Catalogue {
    let names = [
        ("Morning over the ridge", "sky"),
        ("Soft clouds", "sky"),
        ("Pink evening", "sky"),
        ("Blue peaks", "mountains"),
        ("Foggy range", "mountains"),
        ("Quiet shore", "water"),
        ("Slow waves", "water"),
        ("Green path", "nature"),
        ("Wheat in wind", "nature"),
        ("Light between trees", "light"),
        ("Warm rays", "light"),
        ("Stars over the field", "night"),
        ("Late moon", "night"),
        ("One candle", "candles"),
        ("Two lamps", "candles"),
        ("Plain cross", "church"),
        ("Window of colours", "church"),
        ("Open hands", "people"),
        ("Soft blur", "texture"),
        ("Paper grain", "texture"),
        ("Dusk sea", "water"),
        ("High meadow", "nature"),
        ("Winter peak", "mountains"),
        ("Rolling clouds", "sky"),
        ("Night stars loop", "night"),
        ("Sea loop", "water"),
        ("Lantern loop", "candles"),
        ("Cloud loop", "sky"),
    ];
    let entries = names
        .iter()
        .enumerate()
        .map(|(at, (title, category))| {
            let video = at >= 23;
            Entry {
                id: format!("demo-{:02}", at + 1),
                title: (*title).to_string(),
                kind: if video { Kind::Video } else { Kind::Picture },
                category: (*category).to_string(),
                url: format!("https://example.org/{at}.{}", if video { "webm" } else { "jpg" }),
                preview: format!("https://example.org/{at}-preview.jpg"),
                width: if video { 1280 } else { 1920 },
                height: if video { 720 } else { 1080 },
                duration: video.then_some(14.5),
                author: "Made-up Author".to_string(),
                licence: "CC0".to_string(),
                licence_url: "https://example.org/licence".to_string(),
                page: "https://example.org/page".to_string(),
                source: "wikimedia".to_string(),
                brightness: Some(if at % 5 == 0 { 70 } else { 20 + ((at * 7) % 40) as u8 }),
                busy: Some(((at * 11) % 70) as u8),
                megabytes: video.then_some(6.4),
            }
        })
        .collect();
    Catalogue { built: "2026-11-11".into(), source: "Made up".into(), entries }
}

/// A smooth made-up picture, drawn from the id so each tile differs.
fn demo_pixels(id: &str) -> Vec<(u8, u8, u8)> {
    let seed: u32 = id.bytes().fold(2166136261u32, |sum, byte| (sum ^ u32::from(byte)).wrapping_mul(16777619));
    let channel = |shift: u32| 30.0 + ((seed >> shift) % 200) as f32;
    let (r, g, b) = (channel(0), channel(9), channel(18));
    let mut out = Vec::with_capacity(32 * 18);
    for y in 0..18 {
        for x in 0..32 {
            let (u, v) = (x as f32 / 31.0, y as f32 / 17.0);
            let light = 0.35 + 0.65 * (1.0 - v);
            out.push((((r * (1.0 - u) + 255.0 * u * 0.4) * light) as u8, ((g * (1.0 - u * 0.5)) * light) as u8, ((b * (0.5 + u * 0.5)) * light) as u8));
        }
    }
    out
}

/// A small picture made of `pixels` (32 by 18), brightened or darkened toward `shade`. It is a PNG
/// in a file named like a preview: Rune reads the format from the bytes.
fn demo_jpeg(pixels: &[(u8, u8, u8)], shade: u8) -> Vec<u8> {
    let scale = 0.4 + f32::from(shade) / 100.0;
    let mut image = Vec::with_capacity(pixels.len() * 3);
    for (r, g, b) in pixels {
        for value in [r, g, b] {
            image.push((f32::from(*value) * scale).min(255.0) as u8);
        }
    }
    png_bytes(32, 18, &image)
}

/// A PNG with stored (uncompressed) deflate blocks, enough for a made-up preview.
fn png_bytes(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    fn crc(bytes: &[u8]) -> u32 {
        !bytes.iter().fold(!0u32, |c, &b| (0..8).fold(c ^ u32::from(b), |c, _| (c >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(c & 1))))
    }
    fn adler(bytes: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for byte in bytes {
            a = (a + u32::from(*byte)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        out.extend(kind);
        out.extend(data);
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(crc(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for row in rgb.chunks(width as usize * 3) {
        raw.push(0);
        raw.extend(row);
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(65535).collect();
    for (at, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(at + 1 == blocks.len()));
        zlib.extend((block.len() as u16).to_le_bytes());
        zlib.extend((!(block.len() as u16)).to_le_bytes());
        zlib.extend(*block);
    }
    zlib.extend(adler(&raw).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    header.extend([8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A server on this computer that answers one request with `body` and closes.
    fn serve(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("{error}"));
        let port = listener.local_addr().map(|address| address.port()).unwrap_or(0);
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request);
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(&body);
            }
        });
        format!("http://127.0.0.1:{port}/file.jpg")
    }

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("artosia-backdrops-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    fn entry_at(url: String) -> Entry {
        let mut entry = demo_catalogue().entries.remove(2);
        entry.url = url;
        entry
    }

    #[test]
    fn a_download_lands_in_the_media_folder_and_the_look_points_at_it() {
        let body: Vec<u8> = (0..100_000u32).map(|n| (n % 251) as u8).collect();
        let entry = entry_at(serve(body.clone()));
        let mut app = App::practice();
        let folder = scratch("lands");
        app.folder = Some(folder.clone());
        app.backdrops = Desk::with(Catalogue { entries: vec![entry.clone()], ..Catalogue::default() });
        app.use_backdrop(&entry.id);
        assert!(app.backdrops.fetch.is_some(), "a press on a backdrop that is not here starts the download");
        for _ in 0..200 {
            std::thread::sleep(Duration::from_millis(25));
            app.poll_backdrops();
            if app.backdrops.fetch.is_none() {
                break;
            }
        }
        let path = backdrops::local_path(&folder.join("media"), &entry);
        assert_eq!(std::fs::read(&path).unwrap_or_default(), body, "the file is in the library's media folder");
        assert!(!path.with_extension("part").exists(), "no half file is left");
        assert!(app.backdrops.have.contains(&entry.id));
        match &app.settings.look.background {
            artosia_slides::look::Background::Image(saved) => assert_eq!(std::path::Path::new(saved), path, "the look names the local file"),
            other => panic!("the look is {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_second_press_uses_the_file_and_asks_the_network_for_nothing() {
        let entry = entry_at("http://127.0.0.1:9/unreachable.jpg".into());
        let mut app = App::practice();
        let folder = scratch("again");
        app.folder = Some(folder.clone());
        let path = backdrops::local_path(&folder.join("media"), &entry);
        std::fs::create_dir_all(path.parent().unwrap_or(&folder)).unwrap_or_else(|error| panic!("{error}"));
        std::fs::write(&path, b"already here").unwrap_or_else(|error| panic!("{error}"));
        app.backdrops = Desk::with(Catalogue { entries: vec![entry.clone()], ..Catalogue::default() });
        app.use_backdrop(&entry.id);
        assert!(app.backdrops.fetch.is_none(), "nothing is fetched for a file that is here");
        assert!(matches!(&app.settings.look.background, artosia_slides::look::Background::Image(saved) if std::path::Path::new(saved) == path));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_cancelled_download_leaves_no_file_and_an_unreachable_one_says_offline() {
        let folder = scratch("cancel");
        let target = folder.join("a.jpg");
        let (got, total) = (AtomicU64::new(0), AtomicU64::new(0));
        let url = serve(vec![1u8; 10_000]);
        assert_eq!(download(&url, &target, &got, &total, &AtomicBool::new(true), LIMIT), Err(Failure::Cancelled));
        assert!(!target.exists() && !target.with_extension("part").exists());
        // Nothing listens on port 9 of this computer.
        assert_eq!(download("http://127.0.0.1:9/a.jpg", &target, &got, &total, &AtomicBool::new(false), LIMIT), Err(Failure::Offline));
        assert!(!target.exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_file_larger_than_promised_is_refused() {
        let folder = scratch("limit");
        let target = folder.join("big.jpg");
        let url = serve(vec![7u8; 50_000]);
        let (got, total) = (AtomicU64::new(0), AtomicU64::new(0));
        assert!(matches!(download(&url, &target, &got, &total, &AtomicBool::new(false), 10_000), Err(Failure::Disk(_))));
        assert!(!target.exists() && !target.with_extension("part").exists());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// One small real file from Wikimedia Commons, fetched once to prove the catalogue's addresses
    /// work: `cargo test -p artosia a_real_backdrop -- --ignored --nocapture`.
    #[test]
    #[ignore = "uses the network"]
    fn a_real_backdrop_downloads_from_its_own_server() {
        let catalogue = backdrops::built_in();
        let entry = catalogue.entries.iter().filter(|entry| entry.kind == Kind::Picture).min_by_key(|entry| entry.title.len()).unwrap_or_else(|| panic!("the catalogue has no picture"));
        let folder = scratch("real");
        let target = backdrops::local_path(&folder, entry);
        let (got, total) = (AtomicU64::new(0), AtomicU64::new(0));
        download(&entry.preview, &folder.join("preview.jpg"), &got, &total, &AtomicBool::new(false), LIMIT).unwrap_or_else(|failure| panic!("{failure:?}"));
        download(&entry.url, &target, &AtomicU64::new(0), &total, &AtomicBool::new(false), LIMIT).unwrap_or_else(|failure| panic!("{failure:?}"));
        let size = std::fs::metadata(&target).map(|meta| meta.len()).unwrap_or(0);
        println!("{} from {}: {size} bytes, preview {} bytes", entry.id, entry.url, std::fs::metadata(folder.join("preview.jpg")).map(|meta| meta.len()).unwrap_or(0));
        assert!(size > 50_000, "a 1920 wide picture is more than 50 KB");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
