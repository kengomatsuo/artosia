//! The command line.

use std::path::PathBuf;

/// Where the picture comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A window, found by its title.
    Window(String),
    /// A monitor, by the number `--list` gives it.
    Monitor(u32),
}

/// Where the sound comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audio {
    /// The Windows default input.
    Default,
    /// An input found by name.
    Device(String),
    /// A silent track.
    None,
}

/// Where the stream goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Rtmp(String),
    Srt(String),
    Record(PathBuf),
}

/// Which H.264 encoder to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncoderChoice {
    /// Try the hardware encoders in order, then software.
    Auto,
    Qsv,
    MediaFoundation,
    Nvenc,
    X264,
}

impl EncoderChoice {
    fn parse(text: &str) -> Option<Self> {
        match text.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "qsv" => Some(Self::Qsv),
            "mf" => Some(Self::MediaFoundation),
            "nvenc" => Some(Self::Nvenc),
            "x264" => Some(Self::X264),
            _ => None,
        }
    }
}

/// Everything the program was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub source: Source,
    pub audio: Audio,
    pub destinations: Vec<Destination>,
    /// Only consider windows of this process (when two copies of the projector run).
    pub window_pid: Option<u32>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_kbps: u32,
    pub audio_kbps: u32,
    pub encoder: EncoderChoice,
    pub x264_preset: String,
    /// Stop when stdin closes. Off by default: a crashed operator must not end a live stream.
    pub stop_on_eof: bool,
    /// Stop when this file appears.
    pub stop_file: Option<PathBuf>,
    pub list: bool,
    pub help: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            source: Source::Window("Artosia projector".to_string()),
            audio: Audio::Default,
            destinations: Vec::new(),
            window_pid: None,
            width: 1280,
            height: 720,
            fps: 30,
            video_kbps: 2500,
            audio_kbps: 128,
            encoder: EncoderChoice::Auto,
            x264_preset: "superfast".to_string(),
            stop_on_eof: false,
            stop_file: None,
            list: false,
            help: false,
        }
    }
}

pub const USAGE: &str = "\
artosia-stream: send the projector picture and the room's sound out.

Usage:
  artosia-stream [--window \"Artosia projector\" [--window-pid N] | --monitor N]
                 [--audio \"<device name>\" | --no-audio]
                 (--rtmp <url-with-key> | --srt <uri> | --record <file.mp4>)...
                 [--width 1280] [--height 720] [--fps 30]
                 [--video-kbps 2500] [--audio-kbps 128]
                 [--encoder auto|qsv|mf|nvenc|x264] [--x264-preset superfast]
                 [--stop-on-eof] [--stop-file <path>]
  artosia-stream --list

Stop and finish the files: type quit and press Enter, press Ctrl+C, or create the
--stop-file. Closing stdin does NOT stop the stream unless --stop-on-eof is given.
";

const X264_PRESETS: [&str; 9] = [
    "ultrafast",
    "superfast",
    "veryfast",
    "faster",
    "fast",
    "medium",
    "slow",
    "slower",
    "veryslow",
];

/// Parses the arguments after the program name. The error is a sentence for a volunteer.
pub fn parse<I, S>(items: I) -> Result<Args, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = Args::default();
    let mut it = items.into_iter().map(Into::into);
    let mut audio_flag: Option<&'static str> = None;
    let mut source_flag: Option<&'static str> = None;
    while let Some(flag) = it.next() {
        let mut value = |name: &str| -> Result<String, String> {
            it.next()
                .ok_or_else(|| format!("{name} needs a value after it."))
        };
        match flag.as_str() {
            "--help" | "-h" => args.help = true,
            "--list" => args.list = true,
            "--stop-on-eof" => args.stop_on_eof = true,
            "--stop-file" => args.stop_file = Some(PathBuf::from(value("--stop-file")?)),
            "--window" => {
                let v = value("--window")?;
                if v.trim().is_empty() {
                    return Err("--window needs a window title.".to_string());
                }
                if let Some(other) = source_flag.replace("--window").filter(|o| *o != "--window") {
                    return Err(format!("Use either {other} or --window, not both."));
                }
                args.source = Source::Window(v);
            }
            "--window-pid" => {
                let v = value("--window-pid")?;
                args.window_pid = Some(
                    v.parse::<u32>()
                        .map_err(|_| format!("--window-pid needs a number, not \"{v}\"."))?,
                );
            }
            "--monitor" => {
                let v = value("--monitor")?;
                let n = v
                    .parse::<u32>()
                    .map_err(|_| format!("--monitor needs a number such as 0, not \"{v}\"."))?;
                if let Some(other) = source_flag
                    .replace("--monitor")
                    .filter(|o| *o != "--monitor")
                {
                    return Err(format!("Use either {other} or --monitor, not both."));
                }
                args.source = Source::Monitor(n);
            }
            "--audio" => {
                let v = value("--audio")?;
                if audio_flag.replace("--audio") == Some("--no-audio") {
                    return Err("Use either --audio or --no-audio, not both.".to_string());
                }
                args.audio = if v.trim().is_empty() || v.eq_ignore_ascii_case("default") {
                    Audio::Default
                } else {
                    Audio::Device(v)
                };
            }
            "--no-audio" => {
                if audio_flag.replace("--no-audio") == Some("--audio") {
                    return Err("Use either --audio or --no-audio, not both.".to_string());
                }
                args.audio = Audio::None;
            }
            "--rtmp" => {
                let v = value("--rtmp")?;
                if !(v.starts_with("rtmp://") || v.starts_with("rtmps://")) {
                    return Err(
                        "--rtmp needs an address that starts with rtmp:// or rtmps://.".to_string(),
                    );
                }
                args.destinations.push(Destination::Rtmp(v));
            }
            "--srt" => {
                let v = value("--srt")?;
                if !v.starts_with("srt://") {
                    return Err("--srt needs an address that starts with srt://.".to_string());
                }
                args.destinations.push(Destination::Srt(v));
            }
            "--record" => {
                let v = value("--record")?;
                let path = PathBuf::from(&v);
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_ascii_lowercase);
                if ext.as_deref() != Some("mp4") {
                    return Err("--record needs a file name that ends in .mp4.".to_string());
                }
                args.destinations.push(Destination::Record(path));
            }
            "--width" => args.width = number(&value("--width")?, "--width", 160, 3840)?,
            "--height" => args.height = number(&value("--height")?, "--height", 90, 2160)?,
            "--fps" => args.fps = number(&value("--fps")?, "--fps", 5, 60)?,
            "--video-kbps" => {
                args.video_kbps = number(&value("--video-kbps")?, "--video-kbps", 100, 50_000)?;
            }
            "--audio-kbps" => {
                args.audio_kbps = number(&value("--audio-kbps")?, "--audio-kbps", 32, 320)?;
            }
            "--encoder" => {
                let v = value("--encoder")?;
                args.encoder = EncoderChoice::parse(&v).ok_or_else(|| {
                    format!("--encoder must be auto, qsv, mf, nvenc or x264, not \"{v}\".")
                })?;
            }
            "--x264-preset" => {
                let v = value("--x264-preset")?;
                if !X264_PRESETS.contains(&v.as_str()) {
                    return Err(format!(
                        "--x264-preset must be one of {}.",
                        X264_PRESETS.join(", ")
                    ));
                }
                args.x264_preset = v;
            }
            other => return Err(format!("I do not know the option \"{other}\". Try --help.")),
        }
    }
    if args.help || args.list {
        return Ok(args);
    }
    if args.width % 2 != 0 || args.height % 2 != 0 {
        return Err("--width and --height must be even numbers.".to_string());
    }
    if args.destinations.is_empty() {
        return Err("Give at least one place to send to: --rtmp, --srt or --record.".to_string());
    }
    Ok(args)
}

fn number(text: &str, name: &str, min: u32, max: u32) -> Result<u32, String> {
    let n = text
        .parse::<u32>()
        .map_err(|_| format!("{name} needs a number, not \"{text}\"."))?;
    if n < min || n > max {
        return Err(format!("{name} must be between {min} and {max}."));
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(items: &[&str]) -> Result<Args, String> {
        parse(items.iter().map(|s| s.to_string()))
    }

    #[test]
    fn defaults_and_one_destination() {
        let a = p(&["--rtmp", "rtmp://a.example/live/KEY"]).unwrap();
        assert_eq!(a.source, Source::Window("Artosia projector".into()));
        assert_eq!((a.width, a.height, a.fps), (1280, 720, 30));
        assert_eq!(a.video_kbps, 2500);
        assert_eq!(a.audio, Audio::Default);
        assert_eq!(a.destinations.len(), 1);
    }

    #[test]
    fn several_destinations_in_order() {
        let a = p(&[
            "--record",
            "out.mp4",
            "--rtmp",
            "rtmps://x/live/K",
            "--srt",
            "srt://h:1",
        ])
        .unwrap();
        assert_eq!(
            a.destinations,
            vec![
                Destination::Record("out.mp4".into()),
                Destination::Rtmp("rtmps://x/live/K".into()),
                Destination::Srt("srt://h:1".into()),
            ]
        );
    }

    #[test]
    fn monitor_audio_and_sizes() {
        let a = p(&[
            "--monitor",
            "1",
            "--audio",
            "Mixer",
            "--record",
            "a.MP4",
            "--width",
            "1920",
            "--height",
            "1080",
            "--fps",
            "25",
            "--video-kbps",
            "4500",
            "--audio-kbps",
            "96",
        ])
        .unwrap();
        assert_eq!(a.source, Source::Monitor(1));
        assert_eq!(a.audio, Audio::Device("Mixer".into()));
        assert_eq!(
            (a.width, a.height, a.fps, a.video_kbps, a.audio_kbps),
            (1920, 1080, 25, 4500, 96)
        );
    }

    #[test]
    fn no_destination_is_an_error() {
        assert!(p(&[]).unwrap_err().contains("at least one"));
    }

    #[test]
    fn list_needs_no_destination() {
        assert!(p(&["--list"]).unwrap().list);
    }

    #[test]
    fn bad_values_say_what_is_wrong() {
        assert!(p(&["--rtmp", "http://x"]).unwrap_err().contains("rtmp://"));
        assert!(p(&["--srt", "udp://x"]).unwrap_err().contains("srt://"));
        assert!(p(&["--record", "a.mkv"]).unwrap_err().contains(".mp4"));
        assert!(
            p(&["--fps", "abc", "--record", "a.mp4"])
                .unwrap_err()
                .contains("--fps")
        );
        assert!(
            p(&["--fps", "500", "--record", "a.mp4"])
                .unwrap_err()
                .contains("between")
        );
        assert!(
            p(&["--width", "1281", "--record", "a.mp4"])
                .unwrap_err()
                .contains("even")
        );
        assert!(p(&["--bogus"]).unwrap_err().contains("--bogus"));
        assert!(p(&["--rtmp"]).unwrap_err().contains("needs a value"));
    }

    #[test]
    fn conflicting_choices_are_refused() {
        assert!(p(&["--window", "a", "--monitor", "0", "--record", "a.mp4"]).is_err());
        assert!(p(&["--audio", "a", "--no-audio", "--record", "a.mp4"]).is_err());
        assert!(p(&["--no-audio", "--audio", "a", "--record", "a.mp4"]).is_err());
    }

    #[test]
    fn encoder_choice_and_preset() {
        let a = p(&[
            "--encoder",
            "MF",
            "--x264-preset",
            "ultrafast",
            "--record",
            "a.mp4",
        ])
        .unwrap();
        assert_eq!(a.encoder, EncoderChoice::MediaFoundation);
        assert_eq!(a.x264_preset, "ultrafast");
        assert!(p(&["--encoder", "gpu", "--record", "a.mp4"]).is_err());
        assert!(p(&["--x264-preset", "fastest", "--record", "a.mp4"]).is_err());
    }
}
