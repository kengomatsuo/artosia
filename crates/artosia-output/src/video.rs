//! The moving background: one worker owns the playing video file or camera and swaps it when the
//! look changes. Starting blocks until the first frame, so it never runs on the window's thread.
//!
//! When the picture goes wrong the room is never left on a frozen or empty window. A short
//! stall keeps the last frame. A longer one, a missing file, a decode failure or a camera that
//! was unplugged clears the surface, so the look's own black shows under the text, and the
//! operator is told in plain words. A camera is tried again every two seconds and comes back by
//! itself; a file every ten.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, SyncSender, channel, sync_channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use artosia_slides::frame::{FromOutput, trace};
use artosia_video::{CameraLoop, Cameras, VideoError, VideoLoop, VideoOptions};

use crate::shared;

/// What the background should be playing.
#[derive(Clone, Debug, PartialEq)]
pub enum Want {
    Nothing,
    File(String),
    /// A camera by device name; empty is the first one.
    Camera(String),
}

static WANT: OnceLock<Mutex<Sender<Want>>> = OnceLock::new();
static CAMERAS: OnceLock<Arc<Cameras>> = OnceLock::new();

/// A stalled picture older than this stops being shown.
const STALL: Duration = Duration::from_millis(2000);
const CAMERA_RETRY: Duration = Duration::from_secs(2);
const FILE_RETRY: Duration = Duration::from_secs(10);

/// Held while a preview picture is checked and written, and while the video is marked dark and
/// the operator told, so a picture still on its way can never land after "the video is gone".
static PREVIEW_GATE: Mutex<()> = Mutex::new(());

/// Operators connected now; the pictures are made only while there is one.
static OPERATORS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// An operator connected (true) or its socket ended (false).
pub fn operator_connected(connected: bool) {
    let now = if connected { OPERATORS.fetch_add(1, Ordering::AcqRel) + 1 } else { OPERATORS.fetch_sub(1, Ordering::AcqRel).saturating_sub(1) };
    // Measuring switch: ARTOSIA_NO_PREVIEW=1 makes no pictures, for costing them against none.
    artosia_video::want_taps(now > 0 && std::env::var_os("ARTOSIA_NO_PREVIEW").is_none());
}

/// Sends the operator a small picture of the background now and then, from the frames the
/// loop already decodes: each is shrunk from its NV12 planes on the decoder's thread, and a
/// writer thread sends the newest, so a slow socket can never hold the decoder back.
fn start_previews() {
    let (tx, rx): (SyncSender<artosia_slides::preview::Shrunk>, _) = sync_channel(1);
    artosia_video::set_tap(Duration::from_millis(1000 / u64::from(artosia_slides::preview::PER_SECOND)), move |view| {
        let frame = artosia_slides::preview::Nv12 {
            width: view.width,
            height: view.height,
            luma: view.luma,
            luma_stride: view.luma_stride,
            chroma: view.chroma,
            chroma_stride: view.chroma_stride,
            full_range: view.full_range,
            bt601: view.bt601,
        };
        if let Some(small) = artosia_slides::preview::shrink(&frame, artosia_slides::preview::WIDTH) {
            let _ = tx.try_send(small);
        }
    });
    let _ = std::thread::Builder::new().name("artosia-previews".into()).spawn(move || {
        let (mut count, mut began) = (0u32, Instant::now());
        while let Ok(small) = rx.recv() {
            let Ok(_gate) = PREVIEW_GATE.lock() else { continue };
            if !shared().video_live.load(Ordering::Acquire) {
                continue;
            }
            let rgb = artosia_slides::preview::pack(&small);
            shared().tell(&FromOutput::Preview { width: small.width, height: small.height, rgb });
            count += 1;
            if began.elapsed() >= Duration::from_secs(5) {
                trace(|| format!("previews: {count} sent in 5 s, {}x{}", small.width, small.height));
                (count, began) = (0, Instant::now());
            }
        }
    });
}

/// Asks for a file or camera behind the text, or for none.
pub fn want(wanted: Want) {
    if let Some(tx) = WANT.get().and_then(|tx| tx.lock().ok()) {
        let _ = tx.send(wanted);
    }
}

enum Playing {
    File { video: VideoLoop, software: bool },
    Camera { camera: CameraLoop },
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// What the operator reads when `wanted` could not start.
fn words_for_start_failure(wanted: &Want, error: &VideoError) -> String {
    match (wanted, error) {
        (Want::Camera(name), VideoError::NoCamera(_)) if name.is_empty() => "No camera found. The background is black until one is plugged in.".to_string(),
        (Want::Camera(name), VideoError::NoCamera(_)) => format!("Camera {name} is not connected. The background is black until it returns."),
        (Want::Camera(_), _) => "The camera could not start. Another program may be using it. Trying again.".to_string(),
        (Want::File(path), VideoError::NotFound(_)) => format!("The video file is missing: {}. The background is black.", file_name(path)),
        (Want::File(path), _) => format!("The video could not play: {}. The background is black.", file_name(path)),
        (Want::Nothing, _) => String::new(),
    }
}

struct Worker {
    wanted: Want,
    playing: Option<Playing>,
    /// When to try the wanted background again after a failure.
    retry_at: Option<Instant>,
    /// The last notice sent, so the same one is not repeated.
    told: String,
    last_trace: Instant,
}

impl Worker {
    fn tell(&mut self, message: String) {
        if self.told != message {
            trace(|| format!("media notice: {}", if message.is_empty() { "cleared" } else { &message }));
            shared().tell(&FromOutput::MediaError(message.clone()));
            self.told = message;
        }
    }

    /// Shows black under the text until the background is back.
    fn go_dark(&mut self) {
        let shared = shared();
        if let Ok(_gate) = PREVIEW_GATE.lock() {
            shared.video_live.store(false, Ordering::Release);
            shared.tell(&FromOutput::PreviewEnd);
        }
        self.playing = None;
        shared.surface.clear();
        shared.touch();
    }

    fn switch_to(&mut self, wanted: Want) {
        self.wanted = wanted;
        self.retry_at = None;
        self.go_dark();
        self.tell(String::new());
    }

    fn try_start(&mut self, software: bool) {
        let shared = shared();
        let started = Instant::now();
        let result = match &self.wanted {
            Want::Nothing => return,
            Want::File(path) => {
                let mut options = VideoOptions { prefer_hardware: !software, ..VideoOptions::default() };
                // Measuring switches: ARTOSIA_VIDEO_DECODER=software|d3d11|d3d12, ARTOSIA_VIDEO_DISCARD=1.
                match std::env::var("ARTOSIA_VIDEO_DECODER").as_deref() {
                    Ok("software") => options.prefer_hardware = false,
                    Ok("d3d11") => options.only_decoder = Some("d3d11"),
                    Ok("d3d12") => options.only_decoder = Some("d3d12"),
                    _ => {}
                }
                options.discard_frames = std::env::var_os("ARTOSIA_VIDEO_DISCARD").is_some();
                VideoLoop::start(Path::new(path), shared.surface.clone(), options).map(|video| Playing::File { video, software })
            }
            Want::Camera(name) => match gstreamer_cameras() {
                Some(cameras) => cameras.open(name, &shared.surface).map(|camera| Playing::Camera { camera }),
                None => Err(VideoError::NoCamera(name.clone())),
            },
        };
        match result {
            Ok(playing) => {
                match &playing {
                    Playing::File { video, .. } => {
                        trace(|| format!("video started in {} ms, decoder {}", started.elapsed().as_millis(), video.decoder().unwrap_or_default()));
                        if let Some(why) = video.fallback() {
                            trace(|| format!("hardware decoding failed, using software: {why}"));
                            self.tell("Hardware video decoding failed, so the video plays with the processor instead.".to_string());
                        } else if software {
                            self.tell("Hardware video decoding failed, so the video plays with the processor instead.".to_string());
                        } else {
                            self.tell(String::new());
                        }
                    }
                    Playing::Camera { camera } => {
                        trace(|| format!("camera {} started in {} ms", camera.name(), started.elapsed().as_millis()));
                        self.tell(String::new());
                    }
                }
                shared.video_live.store(true, Ordering::Release);
                self.playing = Some(playing);
                self.retry_at = None;
            }
            Err(error) => {
                trace(|| format!("could not start {:?}: {error}", self.wanted));
                let words = words_for_start_failure(&self.wanted, &error);
                self.tell(words);
                let wait = if matches!(self.wanted, Want::Camera(_)) { CAMERA_RETRY } else { FILE_RETRY };
                // A file that is not there is looked for again soon, as a drive may be plugged in.
                let wait = if matches!(error, VideoError::NotFound(_)) { wait.min(Duration::from_secs(5)) } else { wait };
                self.retry_at = Some(Instant::now() + wait);
            }
        }
        shared.touch();
    }

    /// Looks at the playing background and handles what went wrong with it.
    fn check(&mut self) {
        let problem = match &self.playing {
            None => None,
            Some(Playing::File { video, software }) => {
                let stuck = video.since_frame() > Duration::from_secs(3);
                match (video.error(), stuck) {
                    (Some(error), _) => Some((format!("video error: {error}"), *software)),
                    (None, true) => Some(("video stopped sending frames".to_string(), *software)),
                    _ => None,
                }
            }
            Some(Playing::Camera { camera }) => {
                let removed = CAMERAS.get().is_some_and(|cameras| !cameras.has(camera.name()));
                match (camera.error(), camera.since_frame() > STALL, removed) {
                    (Some(error), _, _) => Some((format!("camera error: {error}"), true)),
                    (None, _, true) => Some(("camera was unplugged".to_string(), true)),
                    (None, true, _) => Some(("camera stopped sending frames".to_string(), true)),
                    _ => None,
                }
            }
        };
        let Some((what, software_already)) = problem else { return };
        trace(|| what.clone());
        let was_camera = matches!(self.playing, Some(Playing::Camera { .. }));
        self.go_dark();
        if was_camera {
            self.tell("The camera stopped sending. The background is black until it returns.".to_string());
            self.retry_at = Some(Instant::now() + CAMERA_RETRY);
        } else if !software_already {
            // The hardware decoder gave up in the middle of the file: the processor tries next.
            trace(|| "retrying in software".to_string());
            self.try_start(true);
        } else {
            let words = match &self.wanted {
                Want::File(path) => format!("The video stopped playing: {}. The background is black.", file_name(path)),
                _ => String::new(),
            };
            self.tell(words);
            self.retry_at = Some(Instant::now() + FILE_RETRY);
        }
    }

    fn report(&mut self) {
        if self.last_trace.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_trace = Instant::now();
        match &self.playing {
            Some(Playing::File { video, .. }) => trace(|| {
                let position = video.position().map(|p| p.as_secs_f64()).unwrap_or(-1.0);
                format!("video pos={position:.2} loops={} frames={} decoder={}", video.loops(), video.frames(), video.decoder().unwrap_or_default())
            }),
            Some(Playing::Camera { camera }) => trace(|| format!("camera {} frames={} since_frame={} ms", camera.name(), camera.frames(), camera.since_frame().as_millis())),
            None => {}
        }
    }
}

/// Starts the worker. GStreamer itself starts only when a file or camera is first wanted.
/// The camera names for the operator come from the system every two seconds (`cameras.rs`),
/// because GStreamer's device monitor costs about 50 MB for as long as it runs.
pub fn start() {
    let (tx, rx) = channel::<Want>();
    start_previews();
    if WANT.set(Mutex::new(tx)).is_err() {
        return;
    }
    if std::env::var_os("ARTOSIA_NO_CAMERAS").is_none() {
        let _ = std::thread::Builder::new().name("artosia-cameras".into()).spawn(|| {
            loop {
                let names = crate::cameras::list();
                let changed = shared().cameras.lock().map(|mut known| (*known != names).then(|| *known = names.clone()).is_some()).unwrap_or(false);
                if changed {
                    trace(|| format!("cameras now: {names:?}"));
                    crate::send_hello();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        });
    }
    std::thread::spawn(move || {
        let mut worker = Worker { wanted: Want::Nothing, playing: None, retry_at: None, told: String::new(), last_trace: Instant::now() };
        loop {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(mut wanted) => {
                    while let Ok(newer) = rx.try_recv() {
                        wanted = newer;
                    }
                    // The same background again (every slide sends the whole state) changes nothing.
                    if wanted != worker.wanted {
                        worker.switch_to(wanted);
                        worker.try_start(false);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            worker.check();
            if worker.playing.is_none() && worker.wanted != Want::Nothing && worker.retry_at.is_some_and(|at| Instant::now() >= at) {
                worker.try_start(false);
            }
            worker.report();
        }
    });
}

/// GStreamer's camera list, started the first time a camera is wanted (about 1.5 s).
fn gstreamer_cameras() -> Option<&'static Arc<Cameras>> {
    if let Some(cameras) = CAMERAS.get() {
        return Some(cameras);
    }
    let began = Instant::now();
    match Cameras::start() {
        Ok(cameras) => {
            trace(|| format!("GStreamer camera list ready in {} ms: {:?}", began.elapsed().as_millis(), cameras.names()));
            let _ = CAMERAS.set(Arc::new(cameras));
            CAMERAS.get()
        }
        Err(error) => {
            trace(|| format!("no GStreamer camera list: {error}"));
            None
        }
    }
}
