use std::fmt;
use std::path::PathBuf;

/// Everything this crate can fail at.
#[derive(Debug)]
pub enum VideoError {
    /// GStreamer could not initialise (missing DLLs on a machine that loads them lazily, bad plugin path).
    Init(String),
    /// The file does not exist or is not a regular file.
    NotFound(PathBuf),
    /// A required element (for example `playbin3` or `appsink`) is not installed.
    MissingElement(&'static str),
    /// The pipeline posted an error or refused a state change.
    Pipeline(String),
    /// The pipeline did not reach PAUSED (first frame decoded) in time.
    Timeout,
    /// A frame or its caps could not be read or converted.
    Frame(String),
    /// No camera has this name (empty: no camera at all).
    NoCamera(String),
}

impl fmt::Display for VideoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Init(e) => write!(f, "GStreamer failed to initialise: {e}"),
            Self::NotFound(p) => write!(f, "video file not found: {}", p.display()),
            Self::MissingElement(n) => write!(f, "GStreamer element `{n}` is not installed"),
            Self::Pipeline(e) => write!(f, "video pipeline error: {e}"),
            Self::Timeout => write!(f, "video pipeline did not produce a first frame in time"),
            Self::Frame(e) => write!(f, "video frame error: {e}"),
            Self::NoCamera(n) if n.is_empty() => write!(f, "no camera found"),
            Self::NoCamera(n) => write!(f, "camera not found: {n}"),
        }
    }
}

impl std::error::Error for VideoError {}
