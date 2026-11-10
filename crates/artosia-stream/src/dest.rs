//! One destination: its own small pipeline (mux and sink), fed from the hub.
//!
//! A destination that loses its connection tears its pipeline down and builds a new one after
//! a growing pause. The encoder and the other destinations are not touched. A new session
//! starts on a keyframe and its timestamps start at zero, as a new connection expects.

use crate::args::Destination;
use crate::backoff::{Backoff, HEALTHY_AFTER_SECS};
use crate::capture::sleep_unless;
use crate::hub::{Hub, Packet};
use crate::report::Reporter;
use crate::status::State;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// A destination waiting for its connection more than this long with data piling up is given up
/// on and rebuilt.
const STALL_LIMIT: Duration = Duration::from_secs(20);
/// The most data one destination may hold unsent before it skips ahead to a keyframe.
const BACKLOG_BYTES: u64 = 8 * 1024 * 1024;

pub struct DestHandle {
    pub label: String,
    state: Arc<AtomicU8>,
    thread: Option<JoinHandle<()>>,
}

fn encode_state(s: State) -> u8 {
    match s {
        State::Starting => 0,
        State::Live => 1,
        State::Reconnecting => 2,
        State::Stopped => 3,
    }
}

fn decode_state(v: u8) -> State {
    match v {
        1 => State::Live,
        2 => State::Reconnecting,
        3 => State::Stopped,
        _ => State::Starting,
    }
}

impl DestHandle {
    pub fn state(&self) -> State {
        decode_state(self.state.load(Ordering::SeqCst))
    }

    /// Waits for the thread, at most `limit`. Returns false when it is still running.
    pub fn join(&mut self, limit: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < limit {
            if self.thread.as_ref().is_none_or(|t| t.is_finished()) {
                if let Some(t) = self.thread.take() {
                    let _ = t.join();
                }
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

/// What a destination thread shares with the rest of the program.
#[derive(Clone)]
pub struct Context {
    pub hub: Arc<Hub>,
    pub reporter: Arc<Reporter>,
    /// Set to stop at once without finishing files.
    pub abort: Arc<AtomicBool>,
}

pub fn spawn(dest: Destination, ctx: Context) -> DestHandle {
    let label = ctx.reporter.redactor().label(&dest);
    let state = Arc::new(AtomicU8::new(encode_state(State::Starting)));
    let thread = {
        let state = Arc::clone(&state);
        let label = label.clone();
        std::thread::Builder::new()
            .name("destination".into())
            .spawn(move || run(dest, label, state, ctx))
            .ok()
    };
    DestHandle {
        label,
        state,
        thread,
    }
}

struct Session {
    pipeline: gst::Pipeline,
    sink: gst::Element,
    networked: bool,
    vsrc: gst_app::AppSrc,
    asrc: gst_app::AppSrc,
}

enum Outcome {
    /// The encoder ended and the file or stream was closed properly.
    Finished,
    Aborted,
    Failed(String),
}

fn run(dest: Destination, label: String, state: Arc<AtomicU8>, ctx: Context) {
    let set = |s: State| state.store(encode_state(s), Ordering::SeqCst);
    let networked = !matches!(dest, Destination::Record(_));
    let mut backoff = Backoff::new();
    let mut first = true;
    loop {
        if ctx.abort.load(Ordering::SeqCst) {
            break;
        }
        set(if first {
            State::Starting
        } else {
            State::Reconnecting
        });
        first = false;
        let Some((vcaps, acaps)) = wait_for_caps(&ctx) else {
            break;
        };
        let sub = ctx.hub.subscribe();
        let session = match build(&dest, &vcaps, &acaps) {
            Ok(s) => s,
            Err(msg) => {
                ctx.reporter.problem(
                    &format!("Could not set up {label}. Check the address."),
                    Some(&msg),
                );
                set(State::Stopped);
                return;
            }
        };
        ctx.hub.request_keyframe();
        let started = Instant::now();
        let mut was_up = false;
        let outcome = pump(&session, &sub, &ctx, &state, &label, &mut was_up);
        let lived = started.elapsed();
        let _ = session.pipeline.set_state(gst::State::Null);
        match outcome {
            Outcome::Finished | Outcome::Aborted => break,
            Outcome::Failed(detail) => {
                if !networked {
                    ctx.reporter.problem(
                        &format!(
                            "The recording to {label} stopped. Check the disk space and the folder."
                        ),
                        Some(&detail),
                    );
                    set(State::Stopped);
                    return;
                }
                if lived >= Duration::from_secs(HEALTHY_AFTER_SECS) {
                    backoff.reset();
                }
                let pause = backoff.next_pause();
                set(State::Reconnecting);
                let what = if was_up {
                    "Lost the connection to"
                } else {
                    "Could not connect to"
                };
                ctx.reporter.problem(
                    &format!(
                        "{what} {label}. Trying again in {} seconds.",
                        pause.as_secs()
                    ),
                    Some(&detail),
                );
                sleep_unless(&ctx.abort, pause);
            }
        }
    }
    set(State::Stopped);
}

fn wait_for_caps(ctx: &Context) -> Option<(gst::Caps, gst::Caps)> {
    loop {
        if ctx.abort.load(Ordering::SeqCst) {
            return None;
        }
        if let Some(c) = ctx.hub.caps() {
            return Some(c);
        }
        if ctx.hub.finished.load(Ordering::SeqCst) {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn appsrc(caps: &gst::Caps) -> Result<gst_app::AppSrc, String> {
    Ok(gst_app::AppSrc::builder()
        .caps(caps)
        .format(gst::Format::Time)
        // Not live: the muxer then waits for both streams and orders them by timestamp.
        .is_live(false)
        .block(false)
        .build())
}

fn make(factory: &str) -> Result<gst::Element, String> {
    gst::ElementFactory::make(factory)
        .build()
        .map_err(|_| format!("GStreamer's {factory} is not installed."))
}

fn build(dest: &Destination, vcaps: &gst::Caps, acaps: &gst::Caps) -> Result<Session, String> {
    let pipeline = gst::Pipeline::new();
    let vsrc = appsrc(vcaps)?;
    let asrc = appsrc(acaps)?;
    let vparse = make("h264parse")?;
    let aparse = make("aacparse")?;
    let (mux, sink) = match dest {
        Destination::Rtmp(url) => {
            let mux = make("flvmux")?;
            mux.set_property("streamable", true);
            let sink = make("rtmp2sink")?;
            sink.set_property("location", url.as_str());
            sink.set_property("sync", false);
            (mux, sink)
        }
        Destination::Srt(uri) => {
            let mux = make("mpegtsmux")?;
            let sink = make("srtsink")?;
            sink.set_property("uri", uri.as_str());
            sink.set_property("sync", false);
            sink.set_property("wait-for-connection", false);
            (mux, sink)
        }
        Destination::Record(path) => {
            let mux = make("mp4mux")?;
            let sink = make("filesink")?;
            sink.set_property("location", path.to_string_lossy().as_ref());
            sink.set_property("sync", false);
            (mux, sink)
        }
    };
    let vsrc_e: gst::Element = vsrc.clone().upcast();
    let asrc_e: gst::Element = asrc.clone().upcast();
    pipeline
        .add_many([&vsrc_e, &asrc_e, &vparse, &aparse, &mux, &sink])
        .map_err(|e| e.to_string())?;
    gst::Element::link_many([&vsrc_e, &vparse, &mux]).map_err(|e| e.to_string())?;
    gst::Element::link_many([&asrc_e, &aparse, &mux]).map_err(|e| e.to_string())?;
    mux.link(&sink).map_err(|e| e.to_string())?;
    pipeline
        .set_state(gst::State::Playing)
        .map_err(|_| "The pipeline would not start.".to_string())?;
    Ok(Session {
        pipeline,
        sink,
        networked: !matches!(dest, Destination::Record(_)),
        vsrc,
        asrc,
    })
}

fn bus_problem(session: &Session) -> Option<String> {
    let bus = session.pipeline.bus()?;
    let msg = bus.pop_filtered(&[gst::MessageType::Error])?;
    if let gst::MessageView::Error(e) = msg.view() {
        let debug = e.debug().map(|d| d.to_string()).unwrap_or_default();
        return Some(format!("{} {}", e.error(), debug).trim().to_string());
    }
    None
}

fn pump(
    session: &Session,
    sub: &crate::hub::Subscription,
    ctx: &Context,
    state: &AtomicU8,
    label: &str,
    was_up: &mut bool,
) -> Outcome {
    let mut offset: Option<gst::ClockTime> = None;
    let mut need_key = true;
    let mut first_push: Option<Instant> = None;
    let mut backlog_since: Option<Instant> = None;
    let mut unanswered: Option<Instant> = None;
    let mut warned = false;
    loop {
        if ctx.abort.load(Ordering::SeqCst) {
            return Outcome::Aborted;
        }
        if let Some(problem) = bus_problem(session) {
            return Outcome::Failed(problem);
        }
        if sub.overflowed.swap(false, Ordering::SeqCst) {
            while sub.rx.try_recv().is_ok() {}
            need_key = true;
            ctx.hub.request_keyframe();
        }
        match sub.rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Packet::Video(sample)) => {
                let Some(buffer) = sample.buffer_owned() else {
                    continue;
                };
                let key = !buffer.flags().contains(gst::BufferFlags::DELTA_UNIT);
                if need_key {
                    if !key {
                        continue;
                    }
                    need_key = false;
                    offset = buffer.pts();
                }
                let Some(off) = offset else { continue };
                if session.vsrc.current_level_bytes() > BACKLOG_BYTES {
                    // The connection is not taking data. Skip to the next keyframe.
                    ctx.hub.frames_dropped.fetch_add(1, Ordering::Relaxed);
                    need_key = true;
                    backlog_since.get_or_insert_with(Instant::now);
                    continue;
                }
                if let Some(rebased) = rebase(buffer, off)
                    && session.vsrc.push_buffer(rebased).is_ok()
                {
                    first_push.get_or_insert_with(Instant::now);
                }
            }
            Ok(Packet::Audio(sample)) => {
                let (Some(off), Some(buffer)) = (offset, sample.buffer_owned()) else {
                    continue;
                };
                if session.asrc.current_level_bytes() > BACKLOG_BYTES {
                    continue;
                }
                if let Some(rebased) = rebase(buffer, off) {
                    let _ = session.asrc.push_buffer(rebased);
                }
            }
            Ok(Packet::End) => return finish(session, ctx),
            Err(RecvTimeoutError::Timeout) => {
                if ctx.hub.finished.load(Ordering::SeqCst) {
                    return finish(session, ctx);
                }
            }
            Err(RecvTimeoutError::Disconnected) => return finish(session, ctx),
        }
        let backlog = session.vsrc.current_level_bytes() > BACKLOG_BYTES / 2;
        if backlog {
            let since = backlog_since.get_or_insert_with(Instant::now);
            if since.elapsed() > STALL_LIMIT {
                return Outcome::Failed("The connection stopped taking data.".to_string());
            }
        } else {
            backlog_since = None;
        }
        // Live once the pipeline plays, has been fed for a second, and the sink says it is
        // connected. srtsink keeps retrying by itself, so for SRT the sink's statistics are
        // the truth: they only carry the round-trip time while a receiver is attached.
        let fed = first_push.is_some_and(|t| t.elapsed() >= Duration::from_secs(1));
        let up = fed
            && session.pipeline.current_state() == gst::State::Playing
            && connected(&session.sink);
        if up {
            state.store(encode_state(State::Live), Ordering::SeqCst);
            *was_up = true;
            unanswered = None;
        } else if fed {
            if *was_up {
                state.store(encode_state(State::Reconnecting), Ordering::SeqCst);
            }
            let since = *unanswered.get_or_insert_with(Instant::now);
            if !warned && since.elapsed() > Duration::from_secs(15) {
                warned = true;
                ctx.reporter
                    .problem(&format!("No answer from {label} yet. Still trying."), None);
            }
        }
    }
}

/// Whether the sink has a peer. rtmp2sink counts the bytes the server sent back, and
/// none have come before the server answers; srtsink keeps retrying by itself and carries the round-trip time only
/// while a receiver is attached. A file has no peer to wait for.
fn connected(sink: &gst::Element) -> bool {
    let name = sink.factory().map(|f| f.name().to_string());
    match name.as_deref() {
        Some("srtsink") => stats(sink).is_some_and(|s| s.has_field("rtt-ms")),
        // The server has answered at least one command (connect, createStream, publish).
        Some("rtmp2sink") => stats(sink)
            .and_then(|s| s.get::<u64>("in-bytes-total").ok())
            .is_some_and(|n| n > 0),
        _ => true,
    }
}

fn stats(sink: &gst::Element) -> Option<gst::Structure> {
    sink.property::<Option<gst::Structure>>("stats")
}

/// A copy of the buffer with its timestamps moved so the session starts at zero.
fn rebase(buffer: gst::Buffer, offset: gst::ClockTime) -> Option<gst::Buffer> {
    let pts = buffer.pts()?.checked_sub(offset)?;
    let dts = buffer.dts().and_then(|d| d.checked_sub(offset));
    let mut buffer = buffer;
    {
        let b = buffer.make_mut();
        b.set_pts(pts);
        b.set_dts(dts);
    }
    Some(buffer)
}

/// Ends the streams and waits for the muxer and sink to close the file or the connection.
fn finish(session: &Session, ctx: &Context) -> Outcome {
    let _ = session.vsrc.end_of_stream();
    let _ = session.asrc.end_of_stream();
    let Some(bus) = session.pipeline.bus() else {
        return Outcome::Finished;
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && !ctx.abort.load(Ordering::SeqCst) {
        if let Some(msg) = bus.timed_pop_filtered(
            gst::ClockTime::from_mseconds(200),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        ) {
            return match msg.view() {
                gst::MessageView::Error(e) => {
                    if !session.networked {
                        ctx.reporter.problem(
                            "The recording did not close cleanly. The file may not play.",
                            Some(&e.error().to_string()),
                        );
                    }
                    Outcome::Finished
                }
                _ => Outcome::Finished,
            };
        }
    }
    Outcome::Finished
}
