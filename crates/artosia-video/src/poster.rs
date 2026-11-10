use crate::looper::{bus_error, file_uri, playbin3, preroll};
use crate::{VideoError, frames};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use std::path::Path;

/// One decoded frame as straight-alpha sRGB RGBA, for thumbnails.
pub struct Poster {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The first frame of `path`, scaled down to at most `max_width` pixels wide (aspect kept).
/// Blocks while the file prerolls; call it off the UI thread.
pub fn poster(path: &Path, max_width: u32) -> Result<Poster, VideoError> {
    crate::init()?;
    let uri = file_uri(path)?;
    let sink = gst_app::AppSink::builder()
        .caps(&gst::Caps::builder("video/x-raw").build())
        .max_buffers(1)
        .sync(false)
        .build();
    let playbin = playbin3(&uri)?;
    playbin.set_property_from_str("flags", "video");
    playbin.set_property("video-sink", &sink);

    preroll(&playbin)?;
    let result = (|| {
        let sample = sink
            .try_pull_preroll(gst::ClockTime::from_seconds(5))
            .ok_or_else(|| VideoError::Pipeline(bus_error(&playbin).unwrap_or_else(|| "no first frame".into())))?;
        let caps = sample.caps().ok_or_else(|| VideoError::Frame("sample has no caps".into()))?;
        let info = gst_video::VideoInfo::from_caps(caps).map_err(|e| VideoError::Frame(e.to_string()))?;
        let (w, h) = (info.width().max(1), info.height().max(1));
        let tw = if max_width == 0 { w } else { w.min(max_width) };
        let th = ((u64::from(h) * u64::from(tw) + u64::from(w) / 2) / u64::from(w)).max(1) as u32;
        let target = gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .field("width", tw as i32)
            .field("height", th as i32)
            .build();
        let converted = gst_video::convert_sample(&sample, &target, Some(gst::ClockTime::from_seconds(5)))
            .map_err(|e| VideoError::Frame(e.to_string()))?;
        let (width, height, rgba) = frames::rgba_pixels(&converted)?;
        Ok(Poster { width, height, rgba })
    })();
    let _ = playbin.set_state(gst::State::Null);
    result
}
