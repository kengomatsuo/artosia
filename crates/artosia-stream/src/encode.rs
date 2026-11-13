//! The encoder pipeline: one video branch and one audio branch, joined by the hub.
//!
//! ```text
//! appsrc (raw frames) -> encoder -> h264parse -> appsink  \
//!                                                            -> Hub -> one pipeline per destination
//! wasapi2src | silence -> audioconvert -> audioresample -> AAC -> aacparse -> appsink /
//! ```

use crate::args::Args;
use crate::capture::{self, Latest};
use crate::encoder::{self, Kind};
use crate::hub::{Hub, Packet};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Where the sound comes from once a device name has been looked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSource {
    /// One input, by the name its system opens it with (a WASAPI endpoint id on Windows).
    Device {
        id: String,
        loopback: bool,
    },
    /// The input the system's own sound settings chose. Linux and a Mac only: their sources
    /// open it when no device is named.
    SystemDefault,
    Silence,
}

/// A started encoder pipeline.
pub struct Encode {
    pipeline: gst::Pipeline,
    pub vsrc: gst_app::AppSrc,
    pub kind: Kind,
    pub audio_source: AudioSource,
}

fn make(factory: &str) -> Result<gst::Element, String> {
    gst::ElementFactory::make(factory)
        .build()
        .map_err(|_| format!("GStreamer's {factory} is not installed."))
}

/// The element that reads one sound input: WASAPI on Windows, PulseAudio (or PipeWire's
/// PulseAudio service) on Linux, Core Audio on a Mac.
fn device_source(id: Option<&str>, loopback: bool) -> Result<gst::Element, String> {
    if cfg!(windows) {
        let s = make("wasapi2src")?;
        if let Some(id) = id {
            s.set_property("device", id);
        }
        s.set_property("low-latency", true);
        if loopback {
            s.set_property("loopback", true);
        }
        // 1.28: a device that goes away gives silence and a warning, not an error.
        if s.find_property("continue-on-error").is_some() {
            s.set_property("continue-on-error", true);
        }
        return Ok(s);
    }
    // A Linux input that plays back what the computer plays is a source of its own (a
    // "monitor"), so nothing is set for it; a Mac has no such input without extra software.
    let s = make(if cfg!(target_os = "macos") { "osxaudiosrc" } else { "pulsesrc" })?;
    if let Some(id) = id {
        s.set_property_from_str("device", id);
    }
    Ok(s)
}

/// The AAC encoder to use: the first of these that is installed.
fn aac_encoder() -> Result<gst::Element, String> {
    for name in ["voaacenc", "mfaacenc", "avenc_aac"] {
        if let Ok(e) = gst::ElementFactory::make(name).build() {
            return Ok(e);
        }
    }
    Err("No AAC encoder is installed.".to_string())
}

impl Encode {
    pub fn start(
        kind: Kind,
        args: &Args,
        audio: &AudioSource,
        hub: &Arc<Hub>,
    ) -> Result<Self, String> {
        let pipeline = gst::Pipeline::with_name("encode");

        // Video.
        let vsrc = gst_app::AppSrc::builder()
            .caps(
                &capture::frame_caps(kind, args)
                    .parse::<gst::Caps>()
                    .map_err(|_| "Internal error: the frame caps do not parse.".to_string())?,
            )
            .format(gst::Format::Time)
            .is_live(true)
            .do_timestamp(true)
            .block(false)
            .max_buffers(0)
            .max_bytes(0)
            .build();
        let enc = encoder::make(kind, args)?;
        let vparse = make("h264parse")?;
        vparse.set_property("config-interval", -1i32);
        let vfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-h264")
                    .field("stream-format", "byte-stream")
                    .field("alignment", "au")
                    .build(),
            )
            .build()
            .map_err(|e| e.to_string())?;
        let vsink = gst_app::AppSink::builder().sync(false).build();
        {
            let hub = Arc::clone(hub);
            let hub_eos = Arc::clone(&hub);
            vsink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |s| {
                        if let Ok(sample) = s.pull_sample() {
                            hub.publish(Packet::Video(unshift(sample)));
                        }
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .eos(move |_| hub_eos.sink_eos())
                    .build(),
            );
        }
        let vchain: Vec<gst::Element> = vec![
            vsrc.clone().upcast(),
            enc.clone(),
            vparse,
            vfilter,
            vsink.clone().upcast(),
        ];
        pipeline
            .add_many(vchain.iter())
            .map_err(|e| e.to_string())?;
        gst::Element::link_many(vchain.iter()).map_err(|e| e.to_string())?;

        // Audio.
        let asrc = match audio {
            AudioSource::Device { id, loopback } => device_source(Some(id), *loopback)?,
            AudioSource::SystemDefault => device_source(None, false)?,
            AudioSource::Silence => {
                let s = make("audiotestsrc")?;
                s.set_property_from_str("wave", "silence");
                s.set_property("is-live", true);
                s.set_property("samplesperbuffer", 960i32);
                s
            }
        };
        let aconv = make("audioconvert")?;
        let ares = make("audioresample")?;
        let afilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("audio/x-raw")
                    .field("rate", 48000i32)
                    .field("channels", 2i32)
                    .build(),
            )
            .build()
            .map_err(|e| e.to_string())?;
        let aqueue = make("queue")?;
        let aenc = aac_encoder()?;
        if aenc.find_property("bitrate").is_some() {
            aenc.set_property("bitrate", (args.audio_kbps * 1000) as i32);
        }
        let aparse = make("aacparse")?;
        let asink = gst_app::AppSink::builder().sync(false).build();
        {
            let hub_a = Arc::clone(hub);
            let hub_end = Arc::clone(hub);
            asink.set_callbacks(
                gst_app::AppSinkCallbacks::builder()
                    .new_sample(move |s| {
                        if let Ok(sample) = s.pull_sample() {
                            hub_a.publish(Packet::Audio(sample));
                        }
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .eos(move |_| hub_end.sink_eos())
                    .build(),
            );
        }
        let achain: Vec<gst::Element> = vec![
            asrc,
            aconv,
            ares,
            afilter,
            aqueue,
            aenc,
            aparse,
            asink.clone().upcast(),
        ];
        pipeline
            .add_many(achain.iter())
            .map_err(|e| e.to_string())?;
        gst::Element::link_many(achain.iter()).map_err(|e| e.to_string())?;

        hub.set_encoder(enc);
        let started = Encode {
            pipeline,
            vsrc,
            kind,
            audio_source: audio.clone(),
        };
        started
            .pipeline
            .set_state(gst::State::Playing)
            .map_err(|_| {
                started
                    .bus_error()
                    .unwrap_or_else(|| "The encoder would not start.".to_string())
            })?;
        Ok(started)
    }

    /// The first error on the bus, as text.
    pub fn bus_error(&self) -> Option<String> {
        let bus = self.pipeline.bus()?;
        let msg = bus.pop_filtered(&[gst::MessageType::Error])?;
        if let gst::MessageView::Error(e) = msg.view() {
            let debug = e.debug().map(|d| d.to_string()).unwrap_or_default();
            return Some(format!("{} {}", e.error(), debug).trim().to_string());
        }
        None
    }

    /// A closure that asks the pipeline to finish: every branch flushes, and the sinks' end of
    /// stream tells the hub to close the destinations. It owns clones, so it can run on its own
    /// thread; it may block if the pipeline never finished starting.
    pub fn finisher(&self) -> impl FnOnce() + Send + 'static {
        let vsrc = self.vsrc.clone();
        let pipeline = self.pipeline.clone();
        move || {
            let _ = vsrc.end_of_stream();
            let _ = pipeline.send_event(gst::event::Eos::new());
        }
    }

    pub fn stop(self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}

/// GstVideoEncoder adds 1000 hours to the timestamps it outputs, so a decoder never sees a
/// negative DTS. The audio branch has no such shift, and a muxer needs both on one clock, so
/// the shift is taken off again here.
const ENCODER_SHIFT: gst::ClockTime = gst::ClockTime::from_seconds(1000 * 3600);

fn unshift(sample: gst::Sample) -> gst::Sample {
    let (Some(mut buffer), Some(caps)) = (sample.buffer_owned(), sample.caps_owned()) else {
        return sample;
    };
    let needs = buffer.pts().is_some_and(|p| p >= ENCODER_SHIFT)
        || buffer.dts().is_some_and(|d| d >= ENCODER_SHIFT);
    if !needs {
        return sample;
    }
    {
        let b = buffer.make_mut();
        let pts = b.pts().and_then(|p| p.checked_sub(ENCODER_SHIFT));
        let dts = b.dts().and_then(|d| d.checked_sub(ENCODER_SHIFT));
        b.set_pts(pts);
        b.set_dts(dts);
    }
    gst::Sample::builder().buffer(&buffer).caps(&caps).build()
}

/// Sends the newest captured frame to the encoder `fps` times a second.
///
/// The same frame is sent again when no new one has arrived, so the stream keeps its frame
/// rate when the window is still, or gone: viewers see the last picture. Repeats share the
/// GPU texture, so they cost no copy.
pub fn bridge_loop(
    vsrc: gst_app::AppSrc,
    latest: Arc<Latest>,
    hub: Arc<Hub>,
    fps: u32,
    stop: Arc<AtomicBool>,
) {
    let period = Duration::from_nanos(1_000_000_000 / u64::from(fps.max(1)));
    let duration = gst::ClockTime::from_nseconds(period.as_nanos() as u64);
    let mut next = Instant::now();
    let mut last_counter = 0u64;
    while !stop.load(Ordering::SeqCst) {
        next += period;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > period * 2 {
            // Far behind (the PC stalled): skip ahead rather than send a burst.
            let missed = ((now - next).as_nanos() / period.as_nanos()) as u64;
            hub.frames_dropped.fetch_add(missed, Ordering::Relaxed);
            next = now;
        }
        let Some(sample) = latest.get() else {
            continue;
        };
        let counter = latest.counter.load(Ordering::SeqCst);
        if counter == last_counter {
            hub.frames_repeated.fetch_add(1, Ordering::Relaxed);
        }
        last_counter = counter;
        if vsrc.current_level_buffers() >= 3 {
            // The encoder cannot keep up.
            hub.frames_dropped.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let Some(mut buffer) = sample.buffer_owned() else {
            continue;
        };
        {
            let b = buffer.make_mut();
            b.set_pts(None);
            b.set_dts(None);
            b.set_duration(duration);
        }
        if vsrc.push_buffer(buffer).is_err() {
            hub.frames_dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}
