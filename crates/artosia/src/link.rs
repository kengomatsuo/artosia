//! The operator's side of the output process: start it, keep a socket to it, send it the
//! screen state. The output is its own process so that an operator crash leaves the
//! projector showing what it showed; a restarted operator finds it by its port. Nobody
//! should be able to tell: it has no taskbar button, and the operator turns it on and off.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use artosia_slides::frame::{DisplayInfo, FromOutput, PORT, ToOutput, decode, encode, trace};
use artosia_slides::look::Look;
use rune::{Pixels, Waker};

use crate::strings as t;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkStatus {
    pub connected: bool,
    pub displays: Vec<DisplayInfo>,
    pub audience: Option<usize>,
    pub stage: Option<usize>,
    pub video: bool,
    pub cameras: Vec<String>,
    /// Why the output cannot be reached, in words for the status bar.
    pub problem: Option<String>,
    /// A background the output could not show.
    pub media_error: Option<String>,
    /// The person at the projector window on this screen closed it; the operator has not
    /// yet turned its switch off.
    pub hidden_here: bool,
    /// When the worker began waiting for the output, for telling "starting" from "stuck".
    pub waiting_since: Option<Instant>,
}

/// Where the projector and the stage view go.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputOptions {
    /// The screen the projector uses; None takes the first screen that is not this one.
    pub audience: Option<usize>,
    pub stage: Option<usize>,
    /// Show the stage screen in a window when no screen is assigned to it.
    pub stage_window: bool,
    /// With one screen only: show the projector on it, until Escape there.
    pub here: bool,
    /// Show no projector at all.
    pub off: bool,
}

impl OutputOptions {
    fn place(&self) -> ToOutput {
        ToOutput::Place { audience: self.audience, stage: self.stage, stage_window: self.stage_window, here: self.here, off: self.off }
    }
}

/// The newest small picture of the projector's video or camera, which the two previews draw
/// under the words. The output sends one a few times a second while a moving background plays
/// (`artosia_slides::preview`). `Pixels::set` wakes only the windows showing it, so the window
/// redraws once per picture and not at all between them.
#[derive(Default)]
pub struct LivePicture {
    pixels: Pixels,
    /// A picture has arrived and the moving background has not ended since.
    on: AtomicBool,
}

impl LivePicture {
    /// Keeps `rgba` as the newest picture. Returns true when it is the first since the
    /// background started, so the caller wakes the window that is not yet drawing it.
    pub fn accept(&self, width: u32, height: u32, rgba: Vec<u8>) -> bool {
        self.pixels.set(width, height, rgba);
        !self.on.swap(true, Ordering::AcqRel)
    }

    /// Drops the picture; the previews go back to the still picture or black. Returns true
    /// when there was one.
    pub fn drop_picture(&self) -> bool {
        self.on.swap(false, Ordering::AcqRel)
    }

    /// The picture for a screen drawn with `look`, when `look` is the moving background the
    /// projector is playing now (`playing`) and a picture is there. A look that is a picture,
    /// a colour, or another video, gets none: what the projector plays is not what it shows.
    pub fn for_look(&self, look: &Look, playing: &Look) -> Option<&Pixels> {
        (self.on.load(Ordering::Acquire) && look.is_moving() && look.background == playing.background).then_some(&self.pixels)
    }
}

#[derive(Default)]
pub struct Link {
    tx: Option<Sender<ToOutput>>,
    status: Arc<Mutex<LinkStatus>>,
    options: Arc<Mutex<OutputOptions>>,
    live: Arc<LivePicture>,
}

/// The GStreamer the projector and the streaming program run on, and whether it is the one
/// that came in Artosia's own folder. That one is looked for first: a release carries the
/// files it was built and tried with, and an installed GStreamer may be another version.
fn gstreamer_root() -> Option<(PathBuf, bool)> {
    let mut roots: Vec<(PathBuf, bool)> = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(folder) = exe.parent() {
            roots.push((folder.join("gstreamer"), true));
        }
    if let Some(root) = std::env::var_os("GSTREAMER_1_0_ROOT_MSVC_X86_64") {
        roots.push((PathBuf::from(root), false));
    }
    roots.push((PathBuf::from(r"C:\Program Files\gstreamer\1.0\msvc_x86_64"), false));
    roots.push((PathBuf::from(r"C:\gstreamer\1.0\msvc_x86_64"), false));
    roots.into_iter().find(|(root, _)| root.join("bin").is_dir())
}

/// GStreamer's `bin` folder, which the output process needs on its PATH to play video.
pub fn gstreamer_bin() -> Option<PathBuf> {
    gstreamer_root().map(|(root, _)| root.join("bin"))
}

/// Lets a program Artosia starts find GStreamer. The files that came with Artosia are a
/// GStreamer of their own: it reads only its own plugins and keeps its own plugin cache, so
/// an installed one neither feeds it nor is disturbed by it. GStreamer's "Running GStreamer
/// Applications" page: without `GST_PLUGIN_SYSTEM_PATH` it "will fill in this list for you",
/// and `GST_REGISTRY` is for "a separate environment which should not affect the default cache".
pub fn with_gstreamer(command: &mut std::process::Command) {
    let Some((root, bundled)) = gstreamer_root() else { return };
    let mut paths = vec![root.join("bin")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        command.env("PATH", joined);
    }
    if bundled {
        command.env("GST_PLUGIN_SYSTEM_PATH_1_0", root.join("lib").join("gstreamer-1.0"));
        command.env("GST_PLUGIN_PATH_1_0", "");
        command.env("GST_REGISTRY_1_0", std::env::temp_dir().join("artosia-gstreamer-registry.bin"));
    }
}

fn output_program() -> Option<PathBuf> {
    let folder = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let name = |plain: bool| format!("artosia-output{}{}", if plain { "-plain" } else { "" }, if cfg!(windows) { ".exe" } else { "" });
    // The projector program that plays video does not start at all on a computer without
    // GStreamer: Windows cannot load it ("gio-2.0-0.dll: cannot open", tried 2026-11-10). A
    // release carries a twin built without video, so such a computer still shows the words.
    let plain = folder.join(name(true));
    if gstreamer_bin().is_none() && plain.is_file() {
        return Some(plain);
    }
    Some(folder.join(name(false))).filter(|path| path.is_file())
}

fn spawn_output() -> Result<(), String> {
    let program = output_program().ok_or_else(|| t::OUTPUT_MISSING().to_string())?;
    let mut command = std::process::Command::new(program);
    // Where its windows go arrives over the socket, so it opens nothing it was not asked for.
    command.arg("--port").arg(PORT.to_string());
    with_gstreamer(&mut command);
    command.stdin(std::process::Stdio::null());
    trace(|| format!("starting the output: {command:?}"));
    command.spawn().map(|_| ()).map_err(|error| t::output_failed(&error.to_string()))
}

/// Asks an output process that is already running to close, and gives it a moment to.
pub fn quit_stale_output() {
    if let Ok(mut stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, PORT)) {
        let _ = stream.write_all(encode(&ToOutput::Quit).as_bytes());
        std::thread::sleep(Duration::from_millis(600));
    }
}

fn set(status: &Arc<Mutex<LinkStatus>>, waker: &Waker, change: impl FnOnce(&mut LinkStatus)) {
    if let Ok(mut status) = status.lock() {
        change(&mut status);
    }
    waker();
}

/// What the worker keeps to resend to an output that connects later.
#[derive(Default)]
struct Last {
    state: Option<ToOutput>,
}

/// Talks to one connected output until the socket breaks. Returns true when asked to quit.
fn serve(stream: TcpStream, rx: &Receiver<ToOutput>, last: &mut Last, options: &Arc<Mutex<OutputOptions>>, status: &Arc<Mutex<LinkStatus>>, live: &Arc<LivePicture>, waker: &Waker) -> bool {
    let _ = stream.set_nodelay(true);
    let Ok(reader) = stream.try_clone() else { return false };
    let (reader_status, reader_waker, reader_live) = (status.clone(), waker.clone(), live.clone());
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else { break };
            match decode::<FromOutput>(&line) {
                Some(FromOutput::Hello { displays, audience, stage, video, cameras }) => set(&reader_status, &reader_waker, |s| {
                    trace(|| format!("hello: {} screens, projector {audience:?}, stage {stage:?}, video {video}", displays.len()));
                    s.cameras = cameras;
                    s.displays = displays;
                    s.audience = audience;
                    s.stage = stage;
                    s.video = video;
                }),
                Some(FromOutput::MediaError(reason)) => set(&reader_status, &reader_waker, |s| s.media_error = Some(reason).filter(|reason| !reason.is_empty())),
                Some(FromOutput::ProjectorHidden) => set(&reader_status, &reader_waker, |s| s.hidden_here = true),
                Some(FromOutput::Preview { width, height, rgb }) => {
                    if let Some(rgba) = artosia_slides::preview::unpack(width, height, &rgb)
                        && reader_live.accept(width, height, rgba) {
                            reader_waker();
                        }
                }
                Some(FromOutput::PreviewEnd) if reader_live.drop_picture() => reader_waker(),
                Some(FromOutput::PreviewEnd) | None => {}
            }
        }
        // The projector is gone, and with it the picture.
        if reader_live.drop_picture() {
            reader_waker();
        }
    });
    set(status, waker, |s| {
        s.connected = true;
        s.problem = None;
        s.waiting_since = None;
    });
    trace(|| "connected to the output".to_string());
    let mut stream = stream;
    // Where to show comes before what to show, so nothing flashes on the wrong screen.
    let place = options.lock().map(|options| options.place()).unwrap_or(ToOutput::Place { audience: None, stage: None, stage_window: false, here: false, off: false });
    if stream.write_all(encode(&place).as_bytes()).is_err() {
        return false;
    }
    if let Some(state) = last.state.as_ref()
        && stream.write_all(encode(state).as_bytes()).is_err() {
            return false;
        }
    loop {
        let first = match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(message) => message,
            Err(RecvTimeoutError::Timeout) => {
                // An empty line tells a dead socket from an idle one.
                if stream.write_all(b"\n").is_err() {
                    return false;
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return true,
        };
        // Only the newest state matters; a burst of key presses sends one line.
        let mut batch = vec![first];
        while let Ok(more) = rx.try_recv() {
            batch.push(more);
        }
        let newest_state = batch.iter().rposition(|message| matches!(message, ToOutput::State(_)));
        for (i, message) in batch.iter().enumerate() {
            if matches!(message, ToOutput::State(_)) && Some(i) != newest_state {
                continue;
            }
            if stream.write_all(encode(message).as_bytes()).is_err() {
                return false;
            }
            match message {
                ToOutput::State(_) => last.state = Some(message.clone()),
                ToOutput::Quit => return true,
                ToOutput::Identify(_) | ToOutput::Place { .. } => {}
            }
        }
    }
}

impl Link {
    /// Starts the worker that keeps the output process running and connected.
    pub fn start(&mut self, waker: Waker, options: OutputOptions) {
        if self.tx.is_some() {
            return;
        }
        if let Ok(mut current) = self.options.lock() {
            *current = options;
        }
        let (tx, rx) = channel::<ToOutput>();
        self.tx = Some(tx);
        let (status, options, live) = (self.status.clone(), self.options.clone(), self.live.clone());
        set(&status, &waker, |s| s.waiting_since = Some(Instant::now()));
        std::thread::spawn(move || {
            let mut last = Last::default();
            let mut spawned: Option<Instant> = None;
            // Starts since the output last answered; it should answer the first.
            let mut starts = 0u32;
            loop {
                match TcpStream::connect((Ipv4Addr::LOCALHOST, PORT)) {
                    Ok(stream) => {
                        starts = 0;
                        if serve(stream, &rx, &mut last, &options, &status, &live, &waker) {
                            return;
                        }
                        set(&status, &waker, |s| {
                            s.connected = false;
                            s.waiting_since = Some(Instant::now());
                        });
                    }
                    Err(_) => {
                        // A message sent while nothing listens is still wanted once something does.
                        while let Ok(message) = rx.try_recv() {
                            match message {
                                ToOutput::State(_) => last.state = Some(message),
                                ToOutput::Quit => return,
                                ToOutput::Identify(_) | ToOutput::Place { .. } => {}
                            }
                        }
                        let due = spawned.is_none_or(|at| at.elapsed() > Duration::from_secs(5));
                        if due {
                            spawned = Some(Instant::now());
                            starts += 1;
                            if let Err(problem) = spawn_output() {
                                set(&status, &waker, |s| s.problem = Some(problem));
                            } else if starts > 2 {
                                // It starts and dies each time: with video built in, a missing
                                // GStreamer does exactly this.
                                set(&status, &waker, |s| s.problem = Some(t::OUTPUT_KEEPS_CLOSING().to_string()));
                            }
                        }
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        });
    }

    /// The picture of the projector's moving background for a screen drawn with `look`.
    pub fn live_picture(&self, look: &Look, playing: &Look) -> Option<&Pixels> {
        self.live.for_look(look, playing)
    }

    /// Hands the previews a picture as if the output had sent it, for headless renders.
    pub fn inject_live_picture(&self, width: u32, height: u32, rgba: Vec<u8>) {
        self.live.accept(width, height, rgba);
    }

    pub fn is_started(&self) -> bool {
        self.tx.is_some()
    }

    pub fn send(&self, message: ToOutput) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(message);
        }
    }

    /// The size in pixels of the screen the projector is on, once the output has reported its
    /// screens; None before that, with no projector, or for a screen that gave no size.
    pub fn projector_size(&self) -> Option<(u32, u32)> {
        let status = self.status.lock().ok()?;
        status.audience.and_then(|at| status.displays.get(at)).map(|display| (display.width, display.height)).filter(|(width, height)| *width > 0 && *height > 0)
    }

    pub fn status(&self) -> LinkStatus {
        self.status.lock().map(|status| status.clone()).unwrap_or_default()
    }

    /// Moves the projector and the stage view, or turns the projector off, without
    /// restarting anything: the picture on a screen that stays is not interrupted.
    pub fn place(&self, options: OutputOptions) {
        let message = options.place();
        if let Ok(mut current) = self.options.lock() {
            *current = options;
        }
        self.send(message);
    }

    /// Whether the person at the projector window closed it since this was last asked.
    pub fn take_hidden_here(&self) -> bool {
        self.status.lock().map(|mut status| std::mem::take(&mut status.hidden_here)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use artosia_slides::look::Background;

    fn look(background: Background) -> Look {
        Look { background, ..Look::default() }
    }

    #[test]
    fn the_operator_drops_the_picture_when_the_background_is_no_longer_moving() {
        let link = Link::default();
        let camera = look(Background::Camera(String::new()));
        assert!(link.live_picture(&camera, &camera).is_none(), "nothing arrived yet");
        assert!(link.live.accept(2, 1, vec![0; 8]), "the first picture is told");
        assert!(!link.live.accept(2, 1, vec![0; 8]), "later ones are not");
        assert!(link.live_picture(&camera, &camera).is_some());
        // The look changes to a picture, a colour or another video: the projector no longer plays it.
        let colour = look(Background::Solid([0, 0, 0]));
        assert!(link.live_picture(&colour, &colour).is_none());
        assert!(link.live_picture(&look(Background::Image("a.png".into())), &camera).is_none());
        assert!(link.live_picture(&look(Background::Video("a.mp4".into())), &camera).is_none(), "another look's video");
        assert!(link.live_picture(&camera, &colour).is_none(), "the projector plays a colour now");
        // The output says the video ended.
        assert!(link.live.drop_picture());
        assert!(link.live_picture(&camera, &camera).is_none());
        assert!(!link.live.drop_picture(), "nothing left to drop");
    }
}
