//! A live camera as a moving background: device source -> videoconvert -> appsink (NV12, latest
//! frame wins) -> `NativeSurface`. Cameras are found with GStreamer's device monitor, which also
//! reports one being plugged in or pulled out.

use crate::VideoError;
use crate::looper::{Shared, bus_error, make_sink};
use gst::prelude::*;
use gstreamer as gst;
use rune::NativeSurface;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;
use std::time::Duration;

/// How long a camera gets to deliver its first picture.
const FIRST_FRAME: Duration = Duration::from_secs(6);
/// A camera on the network has to be reached and its stream opened first.
const FIRST_FRAME_FAR: Duration = Duration::from_secs(12);

/// Whether a camera's name is an address on the network (`rtsp://`, `srt://`, `http://` and
/// the like) and so no device of this computer. A phone is such a camera, through an app that
/// sends RTSP or SRT.
pub fn is_address(name: &str) -> bool {
    name.contains("://")
}

/// Cameras named in the file `ARTOSIA_HIDE_CAMERAS_FILE` (one per line) are left out of the list,
/// so a test can unplug and replug one without touching it. Unset in use.
fn hidden_by_test() -> Vec<String> {
    let Some(path) = std::env::var_os("ARTOSIA_HIDE_CAMERAS_FILE") else { return Vec::new() };
    std::fs::read_to_string(path).map(|text| text.lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect()).unwrap_or_default()
}

/// The cameras this computer has, kept current.
pub struct Cameras {
    monitor: gst::DeviceMonitor,
}

impl Cameras {
    /// Starts watching. Slow (about two seconds on Windows): call it off the UI thread.
    pub fn start() -> Result<Cameras, VideoError> {
        crate::init()?;
        let monitor = gst::DeviceMonitor::new();
        // Windows' Media Foundation provider says `Source/Video`; the others say `Video/Source`.
        for class in ["Source/Video", "Video/Source"] {
            monitor.add_filter(Some(class), None);
        }
        monitor.start().map_err(|e| VideoError::Pipeline(e.to_string()))?;
        Ok(Cameras { monitor })
    }

    /// The camera names now, each once, in the order the system lists them.
    pub fn names(&self) -> Vec<String> {
        self.devices().into_iter().map(|(name, _)| name).collect()
    }

    fn devices(&self) -> Vec<(String, gst::Device)> {
        let mut found: Vec<(String, gst::Device)> = Vec::new();
        let hidden = hidden_by_test();
        for device in self.monitor.devices() {
            let name = device.display_name().to_string();
            if hidden.contains(&name) {
                continue;
            }
            // Windows lists a camera through two providers; the first is kept.
            if !found.iter().any(|(known, _)| *known == name) {
                found.push((name, device));
            }
        }
        found
    }

    /// Whether a camera of this name is plugged in now.
    pub fn has(&self, name: &str) -> bool {
        // A camera on the network is no device: only its pictures say whether it is there.
        is_address(name) || self.devices().iter().any(|(known, _)| known == name)
    }

    /// True when a camera came or went since the last call.
    pub fn changed(&self) -> bool {
        let bus = self.monitor.bus();
        let mut changed = false;
        while let Some(message) = bus.pop() {
            changed |= matches!(message.view(), gst::MessageView::DeviceAdded(_) | gst::MessageView::DeviceRemoved(_));
        }
        changed
    }

    /// Opens the camera called `name`, or the first one when `name` is empty. Blocks until a
    /// picture arrives (up to 6 s): call it off the UI thread.
    pub fn open(&self, name: &str, surface: &NativeSurface) -> Result<CameraLoop, VideoError> {
        if is_address(name) {
            return CameraLoop::open_address(name, surface);
        }
        let devices = self.devices();
        let (found, device) = if name.is_empty() {
            devices.into_iter().next()
        } else {
            devices.into_iter().find(|(known, _)| known == name)
        }
        .ok_or_else(|| VideoError::NoCamera(name.to_string()))?;
        CameraLoop::start(&found, &device, surface)
    }
}

impl Drop for Cameras {
    fn drop(&mut self) {
        self.monitor.stop();
    }
}

/// A camera feeding a surface. Dropping it stops the camera.
pub struct CameraLoop {
    pipeline: gst::Pipeline,
    shared: Arc<Shared>,
    bus_thread: Option<JoinHandle<()>>,
    name: String,
}

impl CameraLoop {
    fn start(name: &str, device: &gst::Device, surface: &NativeSurface) -> Result<CameraLoop, VideoError> {
        let shared = Arc::new(Shared::default());
        let sink = make_sink(surface, &shared, false, false);
        let source = device.create_element(None).map_err(|e| VideoError::Pipeline(e.to_string()))?;
        let convert = gst::ElementFactory::make("videoconvert").build().map_err(|_| VideoError::MissingElement("videoconvert"))?;
        let pipeline = gst::Pipeline::new();
        pipeline.add_many([&source, &convert, sink.upcast_ref()]).map_err(|e| VideoError::Pipeline(e.to_string()))?;
        gst::Element::link_many([&source, &convert, sink.upcast_ref()]).map_err(|e| VideoError::Pipeline(e.to_string()))?;
        Self::run(pipeline, shared, name, FIRST_FRAME)
    }

    /// A camera on the network: GStreamer's `playbin3` opens the address and picks the parts
    /// for it, as it does for a video file, and only its picture is taken. Blocks until a
    /// picture arrives (up to 12 s): call it off the UI thread.
    pub fn open_address(address: &str, surface: &NativeSurface) -> Result<CameraLoop, VideoError> {
        crate::init()?;
        let shared = Arc::new(Shared::default());
        let sink = make_sink(surface, &shared, false, false);
        let playbin = crate::looper::playbin3(address)?;
        playbin.set_property_from_str("flags", "video");
        playbin.set_property("video-sink", &sink);
        let pipeline = playbin.downcast::<gst::Pipeline>().map_err(|_| VideoError::Pipeline("playbin3 is not a pipeline".into()))?;
        Self::run(pipeline, shared, address, FIRST_FRAME_FAR)
    }

    /// Starts a built pipeline and waits `patience` for its first picture.
    fn run(pipeline: gst::Pipeline, shared: Arc<Shared>, name: &str, patience: Duration) -> Result<CameraLoop, VideoError> {
        let fail = |pipeline: &gst::Pipeline, fallback: String| {
            let reason = bus_error(pipeline.upcast_ref()).unwrap_or(fallback);
            let _ = pipeline.set_state(gst::State::Null);
            VideoError::Pipeline(reason)
        };
        if pipeline.set_state(gst::State::Playing).is_err() {
            return Err(fail(&pipeline, "the camera would not start".into()));
        }
        // A live source answers `NoPreroll`; the first frame is what proves it works.
        let waited = std::time::Instant::now();
        while shared.frames.load(Ordering::Relaxed) == 0 {
            if let Some(reason) = bus_error(pipeline.upcast_ref()) {
                let _ = pipeline.set_state(gst::State::Null);
                return Err(VideoError::Pipeline(reason));
            }
            if waited.elapsed() >= patience {
                return Err(fail(&pipeline, "no picture arrived".into()));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let bus = pipeline.bus().ok_or_else(|| VideoError::Pipeline("pipeline has no bus".into()))?;
        let bus_thread = {
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("artosia-camera-bus".into())
                .spawn(move || watch_bus(&bus, &shared))
                .map_err(|e| VideoError::Pipeline(e.to_string()))?
        };
        Ok(CameraLoop { pipeline, shared, bus_thread: Some(bus_thread), name: name.to_string() })
    }

    /// The camera's name as the system lists it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How long since a picture arrived.
    pub fn since_frame(&self) -> Duration {
        self.shared.since_frame()
    }

    /// Pictures received so far.
    pub fn frames(&self) -> u64 {
        self.shared.frames.load(Ordering::Relaxed)
    }

    /// The last pipeline error, such as the camera being pulled out.
    pub fn error(&self) -> Option<String> {
        self.shared.error()
    }
}

fn watch_bus(bus: &gst::Bus, shared: &Shared) {
    use gst::MessageView as M;
    while !shared.stop.load(Ordering::Relaxed) {
        let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else { continue };
        match msg.view() {
            M::Error(e) => {
                let debug = e.debug().map(|d| format!(" ({d})")).unwrap_or_default();
                shared.set_error(format!("{}{debug}", e.error()));
            }
            M::Eos(_) => shared.set_error("the camera stopped sending".to_string()),
            _ => {}
        }
    }
}

impl Drop for CameraLoop {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.bus_thread.take() {
            let _ = t.join();
        }
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
