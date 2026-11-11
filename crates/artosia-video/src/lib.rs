//! Looping video backgrounds: GStreamer decodes (hardware first), NV12 frames go into a Rune
//! `NativeSurface`. See the README for build and runtime setup.

mod camera;
mod error;
mod frames;
mod hardware;
mod looper;
mod poster;
mod tap;

pub use camera::{CameraLoop, Cameras};
pub use error::VideoError;
pub use looper::{VideoLoop, VideoOptions};
pub use poster::{Poster, poster};
pub use tap::{Nv12View, set_tap, want as want_taps};

use std::sync::OnceLock;

/// Initialises GStreamer once; later calls return the first result.
pub fn init() -> Result<(), VideoError> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    INIT.get_or_init(|| gstreamer::init().map_err(|e| e.to_string())).clone().map_err(VideoError::Init)
}
