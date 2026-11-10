//! Taking the picture: `d3d11screencapturesrc` of one window or one monitor, scaled and turned
//! into NV12 on the GPU by `d3d11convert`, handed out through an appsink.
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
        "video/x-raw{feature},format=NV12,width={},height={},framerate={}/1",
        args.width, args.height, args.fps
    )
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
        let out_caps: gst::Caps = frame_caps_no_rate(args)
            .parse()
            .map_err(|_| "Internal error: the frame caps do not parse.".to_string())?;
        let out_filter = gst::ElementFactory::make("capsfilter")
            .property("caps", &out_caps)
            .build()
            .map_err(|e| e.to_string())?;

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
        let mut chain: Vec<gst::Element> = vec![source, rate, convert, out_filter];
        if !kind.takes_gpu_frames() {
            let download = gst::ElementFactory::make("d3d11download")
                .build()
                .map_err(|_| "GStreamer's d3d11download is not installed.".to_string())?;
            chain.push(download);
        }
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

fn frame_caps_no_rate(args: &Args) -> String {
    // d3d11convert always produces D3D11 memory; `d3d11download` follows for encoders that
    // need system memory.
    format!(
        "video/x-raw(memory:D3D11Memory),format=NV12,width={},height={}",
        args.width, args.height
    )
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
