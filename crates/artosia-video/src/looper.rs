//! A looping muted background: playbin3 -> appsink (NV12, latest frame wins) -> NativeSurface.
//!
//! Looping is a segment seek: the first seek has `FLUSH | SEGMENT`, the pipeline then posts
//! `SEGMENT_DONE` shortly before the end and every one is answered with a non-flushing
//! `SEGMENT` seek back to 0. GStreamer's seeking design doc recommends exactly this for
//! seamless looping. A file whose pipeline posts EOS instead gets a flushing seek on EOS.

use crate::{VideoError, frames, hardware};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use rune::NativeSurface;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const PREROLL_TIMEOUT: gst::ClockTime = gst::ClockTime::from_seconds(10);

/// How a loop is started.
#[derive(Clone, Copy, Debug)]
pub struct VideoOptions {
    /// No audio is decoded or played (default true).
    pub muted: bool,
    /// Hardware decoders keep their normal rank; false demotes them so a software decoder is
    /// used (default true). The change is process-wide.
    pub prefer_hardware: bool,
    /// Use only hardware decoders whose factory name starts with this, such as `d3d11` (None:
    /// whichever ranks first). For measuring decoders against each other.
    pub only_decoder: Option<&'static str>,
    /// Decode and download every frame but never push it to the surface. A diagnostic that
    /// separates the decoder's cost from the draw's.
    pub discard_frames: bool,
    /// Sound level when not muted, 0.0 to 1.0 (default 1.0).
    pub volume: f64,
    /// Play again from the start at the end (default true). Off, the end is reported by
    /// [`VideoLoop::ended`] and the last frame stays on the surface.
    pub looping: bool,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self { muted: true, prefer_hardware: true, only_decoder: None, discard_frames: false, volume: 1.0, looping: true }
    }
}

/// What a running pipeline reports to its owner.
pub(crate) struct Shared {
    pub(crate) loops: AtomicU64,
    /// A non-looping item reached its end.
    pub(crate) ended: AtomicBool,
    pub(crate) stop: AtomicBool,
    pub(crate) error: Mutex<Option<String>>,
    /// Frames pushed so far.
    pub(crate) frames: AtomicU64,
    started: Instant,
    /// Milliseconds after `started` of the newest frame, plus one; 0 before the first.
    last_frame: AtomicU64,
}

impl Default for Shared {
    fn default() -> Self {
        Self { loops: AtomicU64::new(0), ended: AtomicBool::new(false), stop: AtomicBool::new(false), error: Mutex::new(None), frames: AtomicU64::new(0), started: Instant::now(), last_frame: AtomicU64::new(0) }
    }
}

impl Shared {
    pub(crate) fn set_error(&self, message: String) {
        if let Ok(mut e) = self.error.lock() {
            *e = Some(message);
        }
    }

    /// Restarts the stall clock, for a pipeline that has just begun to play.
    pub(crate) fn began_playing(&self) {
        self.last_frame.store(self.started.elapsed().as_millis() as u64 + 1, Ordering::Relaxed);
    }

    pub(crate) fn frame_pushed(&self) {
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.last_frame.store(self.started.elapsed().as_millis() as u64 + 1, Ordering::Relaxed);
    }

    /// How long since a frame arrived, or since the pipeline started when none has.
    pub(crate) fn since_frame(&self) -> Duration {
        let last = self.last_frame.load(Ordering::Relaxed);
        let now = self.started.elapsed();
        if last == 0 { now } else { now.saturating_sub(Duration::from_millis(last - 1)) }
    }

    pub(crate) fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|e| e.clone())
    }
}

/// An appsink that turns each NV12 sample into a frame on `surface`. `sync` waits for each
/// frame's time (a file); a live camera takes frames as they come.
pub(crate) fn make_sink(surface: &NativeSurface, shared: &Arc<Shared>, sync: bool, discard: bool) -> gst_app::AppSink {
    let caps = gst::Caps::builder("video/x-raw").field("format", "NV12").build();
    let next_id = Arc::new(AtomicU64::new(1));
    let on_frame = {
        let (shared, surface) = (shared.clone(), surface.clone());
        move |sink: &gst_app::AppSink| {
            let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
            if discard {
                shared.frame_pushed();
                return Ok(gst::FlowSuccess::Ok);
            }
            crate::tap::offer(&sample);
            let pushed = frames::nv12_frame(&sample).and_then(|f| {
                surface.push_bytes(f, next_id.fetch_add(1, Ordering::Relaxed)).map_err(VideoError::Frame)
            });
            match pushed {
                Ok(()) => shared.frame_pushed(),
                Err(e) => shared.set_error(e.to_string()),
            }
            Ok(gst::FlowSuccess::Ok)
        }
    };
    gst_app::AppSink::builder()
        .caps(&caps)
        .max_buffers(1)
        .drop(true)
        .sync(sync)
        .callbacks(gst_app::AppSinkCallbacks::builder().new_sample(on_frame).build())
        .build()
}

/// A running loop. Dropping it stops the pipeline and frees it.
pub struct VideoLoop {
    pipeline: gst::Element,
    shared: Arc<Shared>,
    decoder: Mutex<Option<String>>,
    bus_thread: Option<JoinHandle<()>>,
    /// Why hardware decoding was given up for software, when it was.
    fallback: Option<String>,
}

pub(crate) fn file_uri(path: &Path) -> Result<String, VideoError> {
    if !path.is_file() {
        return Err(VideoError::NotFound(path.to_path_buf()));
    }
    let abs = std::path::absolute(path).map_err(|e| VideoError::Pipeline(e.to_string()))?;
    gst::glib::filename_to_uri(&abs, None)
        .map(|u| u.to_string())
        .map_err(|e| VideoError::Pipeline(e.to_string()))
}

pub(crate) fn playbin3(uri: &str) -> Result<gst::Element, VideoError> {
    if gst::ElementFactory::find("playbin3").is_none() {
        return Err(VideoError::MissingElement("playbin3"));
    }
    gst::ElementFactory::make("playbin3").property("uri", uri).build().map_err(|e| VideoError::Pipeline(e.to_string()))
}

/// The first error on the bus, if the pipeline posted one.
pub(crate) fn bus_error(pipeline: &gst::Element) -> Option<String> {
    let bus = pipeline.bus()?;
    let msg = bus.pop_filtered(&[gst::MessageType::Error])?;
    match msg.view() {
        gst::MessageView::Error(e) => {
            let debug = e.debug().map(|d| format!(" ({d})")).unwrap_or_default();
            Some(format!("{}{debug}", e.error()))
        }
        _ => None,
    }
}

/// Brings `pipeline` to PAUSED with a decoded first frame, or says why not.
pub(crate) fn preroll(pipeline: &gst::Element) -> Result<(), VideoError> {
    let fail = |pipeline: &gst::Element, fallback: VideoError| {
        let err = bus_error(pipeline).map_or(fallback, VideoError::Pipeline);
        let _ = pipeline.set_state(gst::State::Null);
        err
    };
    if pipeline.set_state(gst::State::Paused).is_err() {
        return Err(fail(pipeline, VideoError::Pipeline("cannot pause the pipeline".into())));
    }
    match pipeline.state(PREROLL_TIMEOUT).0 {
        Ok(gst::StateChangeSuccess::Async) => Err(fail(pipeline, VideoError::Timeout)),
        Ok(_) => Ok(()),
        Err(_) => Err(fail(pipeline, VideoError::Pipeline("pipeline failed to preroll".into()))),
    }
}

/// A seek to the start. `SEGMENT` makes the pipeline post `SEGMENT_DONE` instead of ending, which
/// is what a seamless loop needs; without it the end is a plain end of stream.
fn segment_seek(pipeline: &gst::Element, flush: bool, segment: bool) -> Result<(), gst::glib::BoolError> {
    let mut flags = gst::SeekFlags::empty();
    if segment {
        flags |= gst::SeekFlags::SEGMENT;
    }
    if flush {
        flags |= gst::SeekFlags::FLUSH;
    }
    pipeline.seek(1.0, flags, gst::SeekType::Set, gst::ClockTime::ZERO, gst::SeekType::None, gst::ClockTime::NONE)
}

impl VideoLoop {
    /// Prepares the pipeline (blocks until the first frame is decoded, up to 10 s: call it off
    /// the UI thread) and starts playing. `surface` should already have its waker set. When the
    /// hardware decoder cannot start the file, it is tried again with a software decoder;
    /// [`VideoLoop::fallback`] says why.
    pub fn start(path: &Path, surface: NativeSurface, options: VideoOptions) -> Result<VideoLoop, VideoError> {
        crate::init()?;
        let uri = file_uri(path)?;
        if !options.prefer_hardware {
            hardware::prefer_hardware(false);
            return Self::build(&uri, Some(&surface), options);
        }
        // On Windows the Direct3D 11 decoder comes first: measured on this PC it uses about
        // 60 MB less than the Direct3D 12 one for the same CPU, and it runs on older drivers.
        let family = options.only_decoder.or(if cfg!(windows) { Some("d3d11") } else { None });
        hardware::prefer_hardware(true);
        if let Some(prefix) = family {
            hardware::keep_only(prefix);
        }
        match Self::build(&uri, Some(&surface), options) {
            Ok(video) => Ok(video),
            Err(first @ (VideoError::Pipeline(_) | VideoError::Timeout)) => {
                hardware::prefer_hardware(false);
                let second = Self::build(&uri, Some(&surface), options);
                // Only this file needed software; the next one tries the hardware again.
                hardware::prefer_hardware(true);
                match second {
                    Ok(mut video) => {
                        video.fallback = Some(first.to_string());
                        Ok(video)
                    }
                    Err(second) => Err(VideoError::Pipeline(format!("{first}; software decoding failed too: {second}"))),
                }
            }
            Err(other) => Err(other),
        }
    }

    /// A sound file played on its own (playbin3 with video off); `options.muted`, `prefer_hardware`
    /// and the frame options mean nothing here. Blocks until the pipeline is ready.
    pub fn start_audio(path: &Path, options: VideoOptions) -> Result<VideoLoop, VideoError> {
        crate::init()?;
        let uri = file_uri(path)?;
        Self::build(&uri, None, options)
    }

    /// `surface` None plays the sound only.
    fn build(uri: &str, surface: Option<&NativeSurface>, options: VideoOptions) -> Result<VideoLoop, VideoError> {
        let shared = Arc::new(Shared::default());

        let playbin = playbin3(uri)?;
        let flags = match (surface.is_some(), options.muted) {
            (false, _) => "audio",
            (true, true) => "video",
            (true, false) => "video+audio",
        };
        playbin.set_property_from_str("flags", flags);
        let muted = options.muted && surface.is_some();
        playbin.set_property("mute", muted);
        playbin.set_property("volume", options.volume.clamp(0.0, 1.0));
        if let Some(surface) = surface {
            playbin.set_property("video-sink", make_sink(surface, &shared, true, options.discard_frames));
        }

        preroll(&playbin)?;
        let started = segment_seek(&playbin, true, options.looping).and_then(|()| {
            playbin.set_state(gst::State::Playing).map(|_| ()).map_err(|e| gst::glib::bool_error!("{e}"))
        });
        if let Err(e) = started {
            let err = bus_error(&playbin).unwrap_or_else(|| e.to_string());
            let _ = playbin.set_state(gst::State::Null);
            return Err(VideoError::Pipeline(err));
        }

        shared.began_playing();
        let bus = playbin.bus().ok_or_else(|| VideoError::Pipeline("pipeline has no bus".into()))?;
        let bus_thread = {
            let (pipeline, shared) = (playbin.clone(), shared.clone());
            std::thread::Builder::new()
                .name("artosia-video-bus".into())
                .spawn(move || watch_bus(&bus, &pipeline, &shared, options.looping))
                .map_err(|e| VideoError::Pipeline(e.to_string()))?
        };
        Ok(VideoLoop { pipeline: playbin, shared, decoder: Mutex::new(None), bus_thread: Some(bus_thread), fallback: None })
    }

    /// Why this loop runs on a software decoder, when hardware decoding failed to start it.
    pub fn fallback(&self) -> Option<&str> {
        self.fallback.as_deref()
    }

    /// How long since a frame reached the surface. A loop that is playing stays under a frame
    /// time or two; a long gap means the pipeline is stuck.
    pub fn since_frame(&self) -> Duration {
        self.shared.since_frame()
    }

    /// Frames pushed so far.
    pub fn frames(&self) -> u64 {
        self.shared.frames.load(Ordering::Relaxed)
    }

    /// Current playback position within the file.
    pub fn position(&self) -> Option<Duration> {
        self.pipeline.query_position::<gst::ClockTime>().map(|t| Duration::from_nanos(t.nseconds()))
    }

    /// The file's length, once the player knows it.
    pub fn duration(&self) -> Option<Duration> {
        self.pipeline.query_duration::<gst::ClockTime>().map(|t| Duration::from_nanos(t.nseconds()))
    }

    /// Whether a non-looping file has played to its end.
    pub fn ended(&self) -> bool {
        self.shared.ended.load(Ordering::Relaxed)
    }

    /// Changes the sound level while it plays, 0.0 to 1.0.
    pub fn set_volume(&self, volume: f64) {
        self.pipeline.set_property("volume", volume.clamp(0.0, 1.0));
    }

    /// Times playback wrapped back to the start.
    pub fn loops(&self) -> u64 {
        self.shared.loops.load(Ordering::Relaxed)
    }

    /// Factory name of the video decoder in use, for example `d3d11h264dec` or `avdec_h264`.
    pub fn decoder(&self) -> Option<String> {
        let mut cached = self.decoder.lock().ok()?;
        if cached.is_none() {
            *cached = self.pipeline.downcast_ref::<gst::Bin>().and_then(hardware::decoder_in);
        }
        cached.clone()
    }

    /// The last pipeline error, if any.
    pub fn error(&self) -> Option<String> {
        self.shared.error()
    }
}

fn watch_bus(bus: &gst::Bus, pipeline: &gst::Element, shared: &Shared, looping: bool) {
    use gst::MessageView as M;
    while !shared.stop.load(Ordering::Relaxed) {
        let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) else { continue };
        match msg.view() {
            M::Eos(_) if !looping => shared.ended.store(true, Ordering::Relaxed),
            M::SegmentDone(_) | M::Eos(_) => {
                let eos = matches!(msg.view(), M::Eos(_));
                shared.loops.fetch_add(1, Ordering::Relaxed);
                if let Err(e) = segment_seek(pipeline, eos, true) {
                    shared.set_error(format!("loop seek failed: {e}"));
                }
            }
            M::Error(e) => {
                let debug = e.debug().map(|d| format!(" ({d})")).unwrap_or_default();
                shared.set_error(format!("{}{debug}", e.error()));
            }
            _ => {}
        }
    }
}

impl Drop for VideoLoop {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.bus_thread.take() {
            let _ = t.join();
        }
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
