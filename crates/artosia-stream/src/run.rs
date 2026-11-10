//! Running a stream: choosing the encoder, starting the parts, printing a line a second, and
//! stopping cleanly.

use crate::args::{Args, Source};
use crate::capture::{Capture, Latest, Target, sleep_unless};
use crate::dest::{self, Context, DestHandle};
use crate::encode::{self, Encode};
use crate::encoder::{self, Kind};
use crate::hub::{Hub, Packet};
use crate::list;
use crate::redact::Redactor;
use crate::report::Reporter;
use crate::status::{self, DestStatus, Snapshot, State};
use crate::win;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::io::BufRead;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

const PICTURE_OK: u8 = 0;
const PICTURE_WAITING: u8 = 1;

/// Runs the program for parsed arguments and returns the exit code.
pub fn run(args: Args) -> i32 {
    win::install_ctrl_handler();
    if let Err(e) = gst::init() {
        let r = Reporter::new(Redactor::default());
        r.problem(
            "GStreamer would not start. Check that it is installed and on the PATH.",
            Some(&e.to_string()),
        );
        return 2;
    }
    // GStreamer's own log lines go to stderr and can carry a stream URL. They stay off unless
    // the operator asks for them with GST_DEBUG.
    if std::env::var_os("GST_DEBUG").is_none() {
        gst::log::set_default_threshold(gst::DebugLevel::None);
    }
    if args.list {
        let r = Reporter::new(Redactor::default());
        r.emit(&list::list_json());
        return 0;
    }
    let reporter = Arc::new(Reporter::new(Redactor::new(&args.destinations)));

    // Listing the audio inputs takes seconds, and so does testing the encoder: do both at once.
    let wants_devices = !matches!(args.audio, crate::args::Audio::None);
    let devices_thread = std::thread::spawn(move || {
        if wants_devices {
            list::audio_devices()
        } else {
            Vec::new()
        }
    });

    // The encoder: the first one that can encode a test picture.
    let Some((kind, skipped)) = choose_encoder(&args) else {
        reporter.problem(
            "No video encoder works on this PC. Try --encoder x264.",
            None,
        );
        return 3;
    };
    // The sound, with a fall back to silence that is said out loud.
    let devices = devices_thread.join().unwrap_or_default();
    let (audio, note) = list::resolve_audio(&args.audio, &devices);
    if let Some(note) = note {
        reporter.problem(&note, None);
    }
    for (k, why) in skipped {
        reporter.info(&format!("The encoder {} was skipped.", k.factory()));
        let _ = why;
    }
    reporter.info(&format!(
        "Encoder: {} ({}).",
        kind.factory(),
        if kind.is_hardware() {
            "hardware"
        } else {
            "software"
        }
    ));

    let hub = Hub::new();
    let encode = match Encode::start(kind, &args, &audio, &hub) {
        Ok(e) => e,
        Err(msg) => {
            reporter.problem("The encoder would not start.", Some(&msg));
            return 3;
        }
    };

    let stop_capture = Arc::new(AtomicBool::new(false));
    let abort = Arc::new(AtomicBool::new(false));
    let latest = Arc::new(Latest::default());
    let picture = Arc::new(AtomicU8::new(PICTURE_WAITING));

    let capture_thread = {
        let (args, latest, picture, stop, reporter) = (
            args.clone(),
            Arc::clone(&latest),
            Arc::clone(&picture),
            Arc::clone(&stop_capture),
            Arc::clone(&reporter),
        );
        std::thread::spawn(move || capture_loop(kind, args, latest, picture, stop, reporter))
    };
    let bridge_thread = {
        let (vsrc, latest, hub, stop) = (
            encode.vsrc.clone(),
            Arc::clone(&latest),
            Arc::clone(&hub),
            Arc::clone(&stop_capture),
        );
        let fps = args.fps;
        std::thread::spawn(move || encode::bridge_loop(vsrc, latest, hub, fps, stop))
    };

    let ctx = Context {
        hub: Arc::clone(&hub),
        reporter: Arc::clone(&reporter),
        abort: Arc::clone(&abort),
    };
    let mut dests: Vec<DestHandle> = args
        .destinations
        .iter()
        .cloned()
        .map(|d| dest::spawn(d, ctx.clone()))
        .collect();

    let quit = Arc::new(AtomicBool::new(false));
    spawn_stdin_reader(Arc::clone(&quit), args.stop_on_eof);

    // The status loop.
    let start = Instant::now();
    let mut live_seconds = 0u64;
    let mut was_live = false;
    let mut last_bytes = 0u64;
    let mut exit_code = 0;
    let mut tick = 0u64;
    loop {
        tick += 1;
        let wake = start + Duration::from_secs(tick);
        while Instant::now() < wake {
            std::thread::sleep(Duration::from_millis(50));
            if should_stop(&quit, &args) {
                break;
            }
        }
        if should_stop(&quit, &args) {
            break;
        }
        if let Some(msg) = encode.bus_error() {
            reporter.problem(
                "The video encoder failed, so the stream cannot go on. Start it again.",
                Some(&msg),
            );
            exit_code = 4;
            break;
        }
        let states: Vec<State> = dests.iter().map(DestHandle::state).collect();
        let overall = status::overall(&states, was_live);
        if overall == State::Live {
            was_live = true;
            live_seconds += 1;
        }
        let bytes = hub.bytes_out.load(Ordering::Relaxed);
        let kbps = (bytes.saturating_sub(last_bytes)) * 8 / 1000;
        last_bytes = bytes;
        reporter.emit(
            &snapshot(
                overall,
                live_seconds,
                &hub,
                kind,
                kbps,
                &picture,
                &dests,
                &states,
            )
            .to_json(),
        );
        if overall == State::Stopped {
            // Every destination has given up (for example the only recording failed).
            break;
        }
    }

    if win::interrupted() {
        reporter.info(&format!(
            "Console event {}.",
            win::LAST_EVENT.load(Ordering::SeqCst)
        ));
    }
    // A watchdog: whatever hangs while stopping (a driver, a network call), the program ends.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(45));
        std::process::exit(5);
    });
    // Stop: the picture first, then flush the encoder, then close each destination.
    stop_capture.store(true, Ordering::SeqCst);
    let _ = capture_thread.join();
    let _ = bridge_thread.join();
    // The end of stream goes in on a helper thread: a pipeline that never received a frame
    // has not finished starting, and sending it an event can block.
    if hub.frames_encoded.load(Ordering::Relaxed) > 0 {
        let finish = encode.finisher();
        std::thread::spawn(finish);
    }
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline && !hub.finished.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(50));
    }
    hub.publish(Packet::End);
    for d in &mut dests {
        if !d.join(Duration::from_secs(15)) {
            reporter.problem(
                &format!("{} did not close in time. It was cut off.", d.label),
                None,
            );
            abort.store(true, Ordering::SeqCst);
            d.join(Duration::from_secs(3));
        }
    }
    encode.stop();
    let states: Vec<State> = dests.iter().map(DestHandle::state).collect();
    reporter.emit(
        &snapshot(
            State::Stopped,
            live_seconds,
            &hub,
            kind,
            0,
            &picture,
            &dests,
            &states,
        )
        .to_json(),
    );
    exit_code
}

#[allow(clippy::too_many_arguments)]
fn snapshot(
    state: State,
    seconds_live: u64,
    hub: &Hub,
    kind: Kind,
    kbps: u64,
    picture: &AtomicU8,
    dests: &[DestHandle],
    states: &[State],
) -> Snapshot {
    Snapshot {
        state,
        seconds_live,
        frames_encoded: hub.frames_encoded.load(Ordering::Relaxed),
        frames_dropped: hub.frames_dropped.load(Ordering::Relaxed),
        frames_repeated: hub.frames_repeated.load(Ordering::Relaxed),
        encoder: kind.factory().to_string(),
        bitrate_kbps: kbps,
        picture: if picture.load(Ordering::SeqCst) == PICTURE_OK {
            "ok"
        } else {
            "waiting for window"
        },
        destinations: dests
            .iter()
            .zip(states)
            .map(|(d, s)| DestStatus {
                name: d.label.clone(),
                state: *s,
            })
            .collect(),
    }
}

fn should_stop(quit: &AtomicBool, args: &Args) -> bool {
    if quit.load(Ordering::SeqCst) || win::interrupted() {
        return true;
    }
    args.stop_file.as_ref().is_some_and(|p| p.exists())
}

/// Reads stdin. `quit` stops the program. When stdin closes the stream keeps going, unless
/// `stop_on_eof` is set, so an operator crash never ends a live stream.
fn spawn_stdin_reader(quit: Arc<AtomicBool>, stop_on_eof: bool) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) if l.trim().eq_ignore_ascii_case("quit") => {
                    quit.store(true, Ordering::SeqCst);
                    return;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        if stop_on_eof {
            quit.store(true, Ordering::SeqCst);
        }
    });
}

/// Tries each encoder with a few test frames. Returns the first that works and the ones skipped.
fn choose_encoder(args: &Args) -> Option<(Kind, Vec<(Kind, String)>)> {
    let mut skipped = Vec::new();
    for kind in encoder::order(args.encoder) {
        match probe(kind, args) {
            Ok(()) => return Some((kind, skipped)),
            Err(why) => skipped.push((kind, why)),
        }
    }
    None
}

/// Runs a short test pattern through the encoder, on the same path the real picture takes.
pub fn probe(kind: Kind, args: &Args) -> Result<(), String> {
    let pipeline = gst::Pipeline::new();
    let make = |f: &str| {
        gst::ElementFactory::make(f)
            .build()
            .map_err(|_| format!("{f} is missing"))
    };
    let src = make("videotestsrc")?;
    src.set_property("num-buffers", 20i32);
    let raw = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("width", args.width as i32)
                .field("height", args.height as i32)
                .field("framerate", gst::Fraction::new(args.fps as i32, 1))
                .build(),
        )
        .build()
        .map_err(|e| e.to_string())?;
    let upload = make("d3d11upload")?;
    let convert = make("d3d11convert")?;
    let nv12 = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .features(["memory:D3D11Memory"])
                .field("format", "NV12")
                .build(),
        )
        .build()
        .map_err(|e| e.to_string())?;
    let enc = encoder::make(kind, args)?;
    let sink = make("fakesink")?;
    sink.set_property("sync", false);
    let mut chain = vec![src, raw, upload, convert, nv12];
    if !kind.takes_gpu_frames() {
        chain.push(make("d3d11download")?);
    }
    chain.push(enc);
    chain.push(sink);
    pipeline.add_many(chain.iter()).map_err(|e| e.to_string())?;
    gst::Element::link_many(chain.iter()).map_err(|e| e.to_string())?;
    let bus = pipeline.bus().ok_or("no bus")?;
    if pipeline.set_state(gst::State::Playing).is_err() {
        let _ = pipeline.set_state(gst::State::Null);
        return Err("would not start".to_string());
    }
    let result = loop {
        match bus.timed_pop_filtered(
            gst::ClockTime::from_seconds(10),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        ) {
            Some(m) => match m.view() {
                gst::MessageView::Eos(_) => break Ok(()),
                gst::MessageView::Error(e) => break Err(e.error().to_string()),
                _ => {}
            },
            None => break Err("timed out".to_string()),
        }
    };
    let _ = pipeline.set_state(gst::State::Null);
    result
}

/// Keeps a capture running: finds the window, starts the capture, and starts again when the
/// window goes away and comes back.
fn capture_loop(
    kind: Kind,
    args: Args,
    latest: Arc<Latest>,
    picture: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    reporter: Arc<Reporter>,
) {
    let mut announced_waiting = false;
    while !stop.load(Ordering::SeqCst) {
        let target = match &args.source {
            Source::Window(title) => {
                let windows = win::windows();
                win::pick_window(&windows, title, args.window_pid).map(|w| Target::Window(w.handle))
            }
            Source::Monitor(n) => win::monitors()
                .into_iter()
                .find(|m| m.index == *n)
                .map(|m| Target::Monitor(m.handle)),
        };
        let Some(target) = target else {
            picture.store(PICTURE_WAITING, Ordering::SeqCst);
            if !announced_waiting {
                announced_waiting = true;
                let what = match &args.source {
                    Source::Window(t) => format!("The window \"{t}\" is not open."),
                    Source::Monitor(n) => format!("Monitor {n} is not connected."),
                };
                reporter.problem(
                    &format!("{what} Waiting for it. The last picture stays on the stream."),
                    None,
                );
            }
            sleep_unless(&stop, Duration::from_secs(1));
            continue;
        };
        let capture = match Capture::start(kind, &args, target, Arc::clone(&latest)) {
            Ok(c) => c,
            Err(msg) => {
                picture.store(PICTURE_WAITING, Ordering::SeqCst);
                if !announced_waiting {
                    announced_waiting = true;
                    reporter.problem(
                        "The picture could not be captured. Trying again.",
                        Some(&msg),
                    );
                }
                sleep_unless(&stop, Duration::from_secs(2));
                continue;
            }
        };
        if announced_waiting {
            reporter.info("The picture is back.");
        }
        announced_waiting = false;
        picture.store(PICTURE_OK, Ordering::SeqCst);
        // Watch the capture until it ends or its window goes.
        let reason = loop {
            if stop.load(Ordering::SeqCst) {
                break None;
            }
            if let Target::Window(h) = target
                && !win::window_exists(h)
            {
                break Some("The window closed.".to_string());
            }
            if let Some(e) = capture.bus_error() {
                break Some(e);
            }
            if capture.ended() {
                break Some("The capture ended.".to_string());
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        if reason.is_some() {
            latest.freeze();
        }
        capture.stop();
        if let Some(reason) = reason {
            picture.store(PICTURE_WAITING, Ordering::SeqCst);
            announced_waiting = true;
            reporter.problem(
                "The picture source went away. Waiting for it. The last picture stays on the stream.",
                Some(&reason),
            );
            sleep_unless(&stop, Duration::from_secs(1));
        }
    }
}
