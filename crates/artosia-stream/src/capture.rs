//! Taking the picture: one window or one monitor, scaled and turned into frames of the
//! stream's size in the layout the encoder takes (NV12, or I420 for OpenH264), handed out
//! through an appsink. Each system has its own source:
//! `d3d11screencapturesrc` on Windows, where the frame stays on the GPU through `d3d11convert`;
//! `ximagesrc` on Linux under X11, `pipewiresrc` on the screen a Wayland desktop shares;
//! `avfvideosrc` on a Mac, which gives a whole screen.
//!
//! The newest frame is kept in [`Latest`]. A separate tick (see `bridge`) samples it at the
//! stream's frame rate, so a window that stops changing, or disappears, never stalls the
//! encoder.

use crate::args::Args;
use crate::encoder::Kind;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// What to capture, resolved to a handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Window(u64),
    Monitor(u64),
    /// A screen or window a Wayland desktop shares through PipeWire: the connection's file
    /// descriptor and the stream's number (`portal`).
    Shared { remote: i32, node: u32 },
}

/// The newest captured frame and a counter that says whether it is new.
#[derive(Default)]
pub struct Latest {
    sample: Mutex<Option<gst::Sample>>,
    pub counter: AtomicU64,
}

impl Latest {
    pub fn put(&self, sample: gst::Sample) {
        if let Ok(mut slot) = self.sample.lock() {
            *slot = Some(sample);
        }
        self.counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Replaces the stored frame by an independent copy. Call before the capture pipeline that
    /// made it is stopped, so the held picture does not depend on that pipeline's buffer pool.
    pub fn freeze(&self) {
        let Ok(mut slot) = self.sample.lock() else {
            return;
        };
        let copy = slot.as_ref().and_then(|sample| {
            let buffer = sample.buffer()?.copy_deep().ok()?;
            Some(
                gst::Sample::builder()
                    .buffer(&buffer)
                    .caps(&sample.caps_owned()?)
                    .build(),
            )
        });
        if copy.is_some() {
            *slot = copy;
        }
    }

    pub fn get(&self) -> Option<gst::Sample> {
        self.sample.lock().ok().and_then(|s| s.clone())
    }
}

/// The caps of the frames the encoder receives.
pub fn frame_caps(kind: Kind, args: &Args) -> String {
    let feature = if kind.takes_gpu_frames() {
        "(memory:D3D11Memory)"
    } else {
        ""
    };
    format!(
        "video/x-raw{feature},format={},width={},height={},framerate={}/1",
        kind.frame_format(),
        args.width,
        args.height,
        args.fps
    )
}

/// What turns downloaded NV12 frames into another layout, for an encoder that asks for one.
#[cfg(windows)]
pub fn to_frame_format(kind: Kind) -> Result<Vec<gst::Element>, String> {
    if kind.frame_format() == "NV12" {
        return Ok(Vec::new());
    }
    let convert = gst::ElementFactory::make("videoconvert")
        .build()
        .map_err(|_| "GStreamer's videoconvert is not installed.".to_string())?;
    let filter = gst::ElementFactory::make("capsfilter")
        .property("caps", gst::Caps::builder("video/x-raw").field("format", kind.frame_format()).build())
        .build()
        .map_err(|e| e.to_string())?;
    Ok(vec![convert, filter])
}

/// A running capture pipeline.
pub struct Capture {
    pipeline: gst::Pipeline,
}

impl Capture {
    /// Builds and starts the capture. Waits up to five seconds for the pipeline to play.
    pub fn start(
        kind: Kind,
        args: &Args,
        target: Target,
        latest: Arc<Latest>,
    ) -> Result<Self, String> {
        let mut chain = picture_chain(kind, args, target)?;

        let sink = gst_app::AppSink::builder()
            .max_buffers(1)
            .drop(true)
            .sync(false)
            .build();
        let latest_cb = Arc::clone(&latest);
        sink.set_callbacks(
            gst_app::AppSinkCallbacks::builder()
                .new_sample(move |s| {
                    if let Ok(sample) = s.pull_sample() {
                        latest_cb.put(sample);
                    }
                    Ok(gst::FlowSuccess::Ok)
                })
                .build(),
        );

        let pipeline = gst::Pipeline::with_name("capture");
        chain.push(sink.clone().upcast());
        pipeline.add_many(chain.iter()).map_err(|e| e.to_string())?;
        gst::Element::link_many(chain.iter()).map_err(|e| e.to_string())?;

        let capture = Capture { pipeline };
        capture
            .pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| {
                capture
                    .bus_error()
                    .unwrap_or_else(|| "The capture would not start.".into())
            })?;
        let (result, _, _) = capture.pipeline.state(gst::ClockTime::from_seconds(5));
        match result {
            Ok(_) => Ok(capture),
            Err(_) => Err(capture
                .bus_error()
                .unwrap_or_else(|| "The capture did not start in time.".to_string())),
        }
    }

    /// The first error on the bus, as text, without consuming other messages.
    pub fn bus_error(&self) -> Option<String> {
        let bus = self.pipeline.bus()?;
        let msg = bus.pop_filtered(&[gst::MessageType::Error])?;
        if let gst::MessageView::Error(e) = msg.view() {
            let debug = e.debug().map(|d| d.to_string()).unwrap_or_default();
            return Some(format!("{} {}", e.error(), debug).trim().to_string());
        }
        None
    }

    /// True when the pipeline reported end of stream, which the capture does when its window
    /// closes.
    pub fn ended(&self) -> bool {
        self.pipeline
            .bus()
            .and_then(|b| b.pop_filtered(&[gst::MessageType::Eos]))
            .is_some()
    }

    pub fn stop(self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// The elements from the picture's source to frames of the stream's size, in order.
/// On Windows the frame is scaled and converted on the GPU and comes down to system memory
/// only for an encoder that needs it there.
#[cfg(windows)]
fn picture_chain(kind: Kind, args: &Args, target: Target) -> Result<Vec<gst::Element>, String> {
    let source = gst::ElementFactory::make("d3d11screencapturesrc")
        .build()
        .map_err(|_| "GStreamer's screen capture is not installed.".to_string())?;
    match target {
        Target::Window(handle) => {
            // Capturing one window needs Windows Graphics Capture.
            source.set_property_from_str("capture-api", "wgc");
            source.set_property("window-handle", handle);
            source.set_property_from_str("window-capture-mode", "client");
        }
        Target::Monitor(handle) => {
            source.set_property("monitor-handle", handle);
        }
        Target::Shared { .. } => return Err("Only a Wayland desktop shares a screen this way.".to_string()),
    }
    source.set_property("show-cursor", false);

    let rate = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .features(["memory:D3D11Memory"])
                .field("framerate", gst::Fraction::new(args.fps as i32, 1))
                .build(),
        )
        .build()
        .map_err(|e| e.to_string())?;
    let convert = gst::ElementFactory::make("d3d11convert")
        .property("add-borders", true)
        .build()
        .map_err(|_| "GStreamer's d3d11convert is not installed.".to_string())?;
    // d3d11convert always produces D3D11 memory; `d3d11download` follows for encoders that
    // need system memory.
    let out_caps: gst::Caps = format!(
        "video/x-raw(memory:D3D11Memory),format=NV12,width={},height={}",
        args.width, args.height
    )
    .parse()
    .map_err(|_| "Internal error: the frame caps do not parse.".to_string())?;
    let out_filter = gst::ElementFactory::make("capsfilter")
        .property("caps", &out_caps)
        .build()
        .map_err(|e| e.to_string())?;

    let mut chain: Vec<gst::Element> = vec![source, rate, convert, out_filter];
    if !kind.takes_gpu_frames() {
        let download = gst::ElementFactory::make("d3d11download")
            .build()
            .map_err(|_| "GStreamer's d3d11download is not installed.".to_string())?;
        chain.push(download);
        chain.extend(to_frame_format(kind)?);
    }
    Ok(chain)
}

/// Off Windows the frame is converted and scaled in system memory, with borders where the
/// source's shape is not the stream's.
#[cfg(not(windows))]
fn picture_chain(kind: Kind, args: &Args, target: Target) -> Result<Vec<gst::Element>, String> {
    let make = |factory: &str| {
        gst::ElementFactory::make(factory)
            .build()
            .map_err(|_| format!("GStreamer's {factory} is not installed."))
    };
    let source = picture_source(target)?;
    let mut chain = vec![source];
    // A shared screen sends a frame when the picture changes and names no rate; the tick that
    // samples the newest frame gives the stream its own.
    if !matches!(target, Target::Shared { .. }) {
        chain.push(
            gst::ElementFactory::make("capsfilter")
                .property(
                    "caps",
                    gst::Caps::builder("video/x-raw")
                        .field("framerate", gst::Fraction::new(args.fps as i32, 1))
                        .build(),
                )
                .build()
                .map_err(|e| e.to_string())?,
        );
    }
    let convert = make("videoconvert")?;
    let scale = make("videoscale")?;
    scale.set_property("add-borders", true);
    let out_caps: gst::Caps = format!(
        "video/x-raw,format={},width={},height={},pixel-aspect-ratio=1/1",
        kind.frame_format(),
        args.width,
        args.height
    )
    .parse()
    .map_err(|_| "Internal error: the frame caps do not parse.".to_string())?;
    let out_filter = gst::ElementFactory::make("capsfilter")
        .property("caps", &out_caps)
        .build()
        .map_err(|e| e.to_string())?;
    chain.extend([convert, scale, out_filter]);
    Ok(chain)
}

/// X11's own source. It takes one window by its id, or a part of the whole screen by its
/// corners, which is how one monitor is taken (`gst-inspect-1.0 ximagesrc`: `xid`, `startx`,
/// `starty`, `endx`, `endy`, `use-damage`, `show-pointer`; read on GStreamer 1.28.2).
#[cfg(target_os = "linux")]
fn picture_source(target: Target) -> Result<gst::Element, String> {
    if let Target::Shared { remote, node } = target {
        let source = gst::ElementFactory::make("pipewiresrc")
            .build()
            .map_err(|_| "GStreamer's PipeWire source is not installed (the package gstreamer1.0-pipewire).".to_string())?;
        // `gst-inspect-1.0 pipewiresrc`, PipeWire 1.6: `fd` is the connection, `path` the stream.
        source.set_property("fd", remote);
        source.set_property("path", node.to_string());
        return Ok(source);
    }
    let source = gst::ElementFactory::make("ximagesrc")
        .build()
        .map_err(|_| "GStreamer's screen capture (ximagesrc) is not installed.".to_string())?;
    // Whole frames at the stream's rate, changed or not, and never the pointer.
    source.set_property("use-damage", false);
    source.set_property("show-pointer", false);
    match target {
        Target::Window(xid) => source.set_property_from_str("xid", &xid.to_string()),
        Target::Monitor(handle) => {
            let monitor = crate::win::monitors()
                .into_iter()
                .find(|m| m.handle == handle)
                .ok_or_else(|| "That monitor is not connected.".to_string())?;
            let corners = [
                ("startx", monitor.x),
                ("starty", monitor.y),
                ("endx", monitor.x + monitor.width - 1),
                ("endy", monitor.y + monitor.height - 1),
            ];
            for (name, value) in corners {
                source.set_property_from_str(name, &value.max(0).to_string());
            }
        }
        Target::Shared { .. } => {}
    }
    Ok(source)
}

/// A Mac's screen through AVFoundation. GStreamer has no source there for one window, so a
/// window's target is the screen it is on (`win::monitors` gives the same numbers). macOS asks
/// the person once whether Artosia may record the screen, and gives black frames until then.
#[cfg(target_os = "macos")]
fn picture_source(target: Target) -> Result<gst::Element, String> {
    let source = gst::ElementFactory::make("avfvideosrc")
        .build()
        .map_err(|_| "GStreamer's screen capture (avfvideosrc) is not installed.".to_string())?;
    source.set_property("capture-screen", true);
    if source.find_property("capture-screen-cursor").is_some() {
        source.set_property("capture-screen-cursor", false);
    }
    let screen = match target {
        Target::Window(handle) => crate::win::screen_of_window(handle),
        Target::Monitor(handle) => handle,
        Target::Shared { .. } => return Err("Only a Wayland desktop shares a screen this way.".to_string()),
    };
    source.set_property_from_str("device-index", &screen.to_string());
    Ok(source)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn picture_source(_target: Target) -> Result<gst::Element, String> {
    Err("This system has no screen capture Artosia knows.".to_string())
}

/// A pause that checks `stop` often.
pub fn sleep_unless(stop: &std::sync::atomic::AtomicBool, total: Duration) {
    let step = Duration::from_millis(100);
    let mut slept = Duration::ZERO;
    while slept < total && !stop.load(Ordering::SeqCst) {
        std::thread::sleep(step);
        slept += step;
    }
}
