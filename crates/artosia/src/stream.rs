//! Streaming and recording: the operator starts `artosia-stream` beside itself, which sends
//! the projector's picture and the room's sound to a streaming service or a file. It is its
//! own program for the reason the projector is: a crash here must not end the stream.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};

use rune::Waker;
use serde::Deserialize;

use crate::strings as t;

/// What the helper last said about itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StreamStatus {
    /// "starting", "live", "reconnecting" or "stopped".
    pub state: String,
    pub seconds_live: u64,
    pub frames_dropped: u64,
    /// The last thing that went wrong, in the helper's own words for a volunteer.
    pub problem: Option<String>,
    /// The helper has ended.
    pub ended: bool,
}

/// One line of the helper's output. Fields it does not send stay at their defaults.
#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    seconds_live: u64,
    #[serde(default)]
    frames_dropped: u64,
    #[serde(default)]
    message: String,
}

pub struct Stream {
    child: Child,
    stdin: Option<ChildStdin>,
    status: Arc<Mutex<StreamStatus>>,
    /// Creating this file stops a helper whose operator died and came back.
    stop_file: PathBuf,
}

pub struct StreamOptions {
    /// The service's address with the key at its end; empty when only recording.
    pub address: String,
    pub record: Option<PathBuf>,
}

fn program() -> Option<PathBuf> {
    let folder = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let name = if cfg!(windows) { "artosia-stream.exe" } else { "artosia-stream" };
    Some(folder.join(name)).filter(|path| path.is_file())
}

/// Whether this copy of Artosia came with the streaming program.
pub fn available() -> bool {
    // The streaming program is built on GStreamer and does not start without it.
    program().is_some() && crate::link::gstreamer_bin().is_some()
}

impl Stream {
    pub fn start(options: StreamOptions, waker: Option<Waker>) -> Result<Stream, String> {
        let program = program().ok_or_else(|| t::STREAM_MISSING().to_string())?;
        let stop_file = std::env::temp_dir().join(format!("artosia-stream-{}.stop", std::process::id()));
        let _ = std::fs::remove_file(&stop_file);
        let mut command = Command::new(program);
        command.arg("--window").arg("Artosia projector").arg("--stop-file").arg(&stop_file);
        if !options.address.is_empty() {
            let flag = if options.address.starts_with("srt://") { "--srt" } else { "--rtmp" };
            command.arg(flag).arg(&options.address);
        }
        if let Some(file) = &options.record {
            command.arg("--record").arg(file);
        }
        crate::link::with_gstreamer(&mut command);
        // The helper's error stream can carry an address at debug levels, so it is not kept.
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = command.spawn().map_err(|error| t::stream_failed(&error.to_string()))?;
        let status = Arc::new(Mutex::new(StreamStatus { state: "starting".to_string(), ..StreamStatus::default() }));
        if let Some(out) = child.stdout.take() {
            let status = status.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    let Ok(line) = serde_json::from_str::<Line>(&line) else { continue };
                    if let Ok(mut status) = status.lock() {
                        match line.kind.as_str() {
                            "status" => {
                                status.state = line.state;
                                status.seconds_live = line.seconds_live;
                                status.frames_dropped = line.frames_dropped;
                                if status.state == "live" {
                                    status.problem = None;
                                }
                            }
                            "problem" => status.problem = Some(line.message),
                            _ => {}
                        }
                    }
                    if let Some(wake) = &waker {
                        wake();
                    }
                }
                if let Ok(mut status) = status.lock() {
                    status.ended = true;
                    status.state = "stopped".to_string();
                }
                if let Some(wake) = &waker {
                    wake();
                }
            });
        }
        let stdin = child.stdin.take();
        Ok(Stream { child, stdin, status, stop_file })
    }

    pub fn status(&self) -> StreamStatus {
        self.status.lock().map(|status| status.clone()).unwrap_or_default()
    }

    /// Asks the helper to finish cleanly, so a recording is left playable.
    pub fn stop(&mut self) {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.write_all(b"quit\n");
            let _ = stdin.flush();
        }
        // The file is the second way in, for a helper that stopped reading.
        let _ = std::fs::write(&self.stop_file, b"stop");
    }

    /// Whether the helper process has gone.
    pub fn finished(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}
