//! The H.264 encoders, in the order they are tried, and how each is set up.

use crate::args::{Args, EncoderChoice};
use gstreamer as gst;
use gstreamer::prelude::*;

/// One of the encoders GStreamer has on this PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Intel Quick Sync, `qsvh264enc`.
    Qsv,
    /// Media Foundation, `mfh264enc`: whatever hardware encoder the driver offers.
    MediaFoundation,
    /// NVIDIA NVENC, `nvh264enc`.
    Nvenc,
    /// Software, `x264enc`.
    X264,
    /// VA-API on Linux, `vah264enc`: the hardware encoders of Intel and AMD graphics.
    Va,
    /// Apple's VideoToolbox, `vtenc_h264`: a Mac's hardware encoder.
    VideoToolbox,
    /// Cisco's software encoder, `openh264enc`: what a system has where x264 is not installed.
    OpenH264,
}

impl Kind {
    pub fn factory(self) -> &'static str {
        match self {
            Kind::Qsv => "qsvh264enc",
            Kind::MediaFoundation => "mfh264enc",
            Kind::Nvenc => "nvh264enc",
            Kind::X264 => "x264enc",
            Kind::Va => "vah264enc",
            Kind::VideoToolbox => "vtenc_h264",
            Kind::OpenH264 => "openh264enc",
        }
    }

    /// Whether the encoder takes Direct3D 11 frames, so the picture never leaves the GPU.
    /// The others need the frame downloaded to system memory first.
    pub fn takes_gpu_frames(self) -> bool {
        matches!(self, Kind::Qsv | Kind::MediaFoundation)
    }

    /// The layout of the frames the encoder is given. OpenH264 takes only I420
    /// (`gst-inspect-1.0 openh264enc`); every other one here takes NV12.
    pub fn frame_format(self) -> &'static str {
        if self == Kind::OpenH264 { "I420" } else { "NV12" }
    }

    pub fn is_hardware(self) -> bool {
        !matches!(self, Kind::X264 | Kind::OpenH264)
    }
}

/// The encoders to try, first choice first.
pub fn order(choice: EncoderChoice) -> Vec<Kind> {
    match choice {
        // Each system's own hardware first, then NVIDIA's where it runs, then software.
        EncoderChoice::Auto if cfg!(target_os = "linux") => vec![Kind::Va, Kind::Nvenc, Kind::X264, Kind::OpenH264],
        EncoderChoice::Auto if cfg!(target_os = "macos") => vec![Kind::VideoToolbox, Kind::X264, Kind::OpenH264],
        EncoderChoice::Auto => vec![Kind::Qsv, Kind::MediaFoundation, Kind::Nvenc, Kind::X264],
        EncoderChoice::Qsv => vec![Kind::Qsv],
        EncoderChoice::MediaFoundation => vec![Kind::MediaFoundation],
        EncoderChoice::Nvenc => vec![Kind::Nvenc],
        EncoderChoice::X264 => vec![Kind::X264],
        EncoderChoice::Va => vec![Kind::Va],
        EncoderChoice::VideoToolbox => vec![Kind::VideoToolbox],
        EncoderChoice::OpenH264 => vec![Kind::OpenH264],
    }
}

/// A keyframe every two seconds: what YouTube and Facebook ask for.
pub fn gop_frames(fps: u32) -> u32 {
    fps * 2
}

/// Makes the encoder element with its settings for live use: constant bitrate, no B-frames,
/// no lookahead, so the delay stays under a second.
pub fn make(kind: Kind, args: &Args) -> Result<gst::Element, String> {
    let element = gst::ElementFactory::make(kind.factory())
        .name("encoder")
        .build()
        .map_err(|_| format!("The encoder {} is not installed.", kind.factory()))?;
    let gop = gop_frames(args.fps).to_string();
    let set = |name: &str, value: &str| set_if_present(&element, name, value);
    set("bitrate", &args.video_kbps.to_string());
    match kind {
        Kind::Qsv => {
            set("rate-control", "cbr");
            set("gop-size", &gop);
            set("b-frames", "0");
            set("low-latency", "true");
            // 1 is best quality, 7 is best speed. 4 is the driver's balance.
            set("target-usage", "4");
        }
        Kind::MediaFoundation => {
            set("rc-mode", "cbr");
            set("gop-size", &gop);
            set("bframes", "0");
            set("low-latency", "true");
        }
        Kind::Nvenc => {
            set("rc-mode", "cbr");
            set("gop-size", &gop);
            set("bframes", "0");
            set("zerolatency", "true");
            set("preset", "p2");
        }
        Kind::X264 => {
            set("speed-preset", &args.x264_preset);
            set("tune", "zerolatency");
            set("key-int-max", &gop);
            set("bframes", "0");
            set("byte-stream", "true");
            // A VBV buffer of one second keeps the rate steady enough for a service.
            set("vbv-buf-capacity", "1000");
        }
        Kind::Va => {
            set("rate-control", "cbr");
            set("key-int-max", &gop);
            set("b-frames", "0");
            set("target-usage", "4");
        }
        Kind::VideoToolbox => {
            set("max-keyframe-interval", &gop);
            set("realtime", "true");
            set("allow-frame-reordering", "false");
        }
        Kind::OpenH264 => {
            // This one counts its bitrate in bits a second (`gst-inspect-1.0 openh264enc`).
            let bits = (u64::from(args.video_kbps) * 1000).to_string();
            set("bitrate", &bits);
            set("max-bitrate", &bits);
            set("rate-control", "bitrate");
            set("gop-size", &gop);
            set("complexity", "low");
        }
    }
    Ok(element)
}

/// Sets a property from text when the element has it. A property one driver version lacks
/// is skipped, never a crash.
fn set_if_present(element: &gst::Element, name: &str, value: &str) {
    if element.find_property(name).is_some() {
        element.set_property_from_str(name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_tries_hardware_before_software() {
        let o = order(EncoderChoice::Auto);
        let first_software = o.iter().position(|k| !k.is_hardware()).unwrap();
        assert!(first_software > 0, "{o:?}");
        assert!(o[first_software..].iter().all(|k| !k.is_hardware()), "{o:?}");
        assert!(o.contains(&Kind::X264));
        if cfg!(windows) {
            assert_eq!(o, vec![Kind::Qsv, Kind::MediaFoundation, Kind::Nvenc, Kind::X264]);
        }
    }

    #[test]
    fn a_named_encoder_is_the_only_one_tried() {
        assert_eq!(order(EncoderChoice::Nvenc), vec![Kind::Nvenc]);
    }

    #[test]
    fn keyframes_come_every_two_seconds() {
        assert_eq!(gop_frames(30), 60);
        assert_eq!(gop_frames(25), 50);
    }
}
