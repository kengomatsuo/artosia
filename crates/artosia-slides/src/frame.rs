//! What the operator tells the output process, as JSON lines over a local socket.

use artosia_core::deck::Slide;
use serde::{Deserialize, Serialize};

use crate::look::Look;

/// The local port the output process listens on.
pub const PORT: u16 = 47823;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Countdown {
    pub label: String,
    /// Seconds since the Unix epoch.
    pub ends_at: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StageChip {
    pub text: String,
    /// The section colour group, as `artosia_core::deck::tag_group` numbers them.
    pub group: usize,
    pub active: bool,
}

/// What the band and the speaker see beside the live slide.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StageInfo {
    pub title: String,
    pub next: Option<Slide>,
    /// The item after this one, shown when the live slide is the last.
    pub next_title: String,
    pub index: usize,
    pub count: usize,
    pub chips: Vec<StageChip>,
    pub message: String,
    #[serde(default)]
    pub words: StageWords,
}

/// The few words the stage monitor writes itself, in the operator's language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StageWords {
    pub clock: String,
    pub timer: String,
    pub black: String,
    pub nothing: String,
    /// What comes next, with `{label}` where its name goes.
    pub next: String,
    /// Which slide of how many, with `{index}` and `{count}`.
    pub place: String,
}

impl Default for StageWords {
    fn default() -> Self {
        StageWords {
            clock: "Clock".into(),
            timer: "Timer".into(),
            black: "Screen is black".into(),
            nothing: "Nothing on screen".into(),
            next: "Next: {label}".into(),
            place: "Slide {index} of {count}".into(),
        }
    }
}

/// How the whole projector picture is bent and tinted to suit the wall it lands on.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Warp {
    /// Where the picture's corners land, as fractions of the screen: top left, top right,
    /// bottom right, bottom left. Untouched is (0,0), (1,0), (1,1), (0,1).
    pub corners: [[f32; 2]; 4],
    /// Added to every colour, -1 to 1; 0 leaves it alone.
    pub brightness: f32,
    /// 1 leaves it alone.
    pub contrast: f32,
    /// 1 leaves it alone, 0 is grey.
    pub saturation: f32,
}

impl Default for Warp {
    fn default() -> Self {
        Warp { corners: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], brightness: 0.0, contrast: 1.0, saturation: 1.0 }
    }
}

impl Warp {
    pub fn is_identity(&self) -> bool {
        *self == Warp::default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OutputState {
    pub slide: Option<Slide>,
    pub look: Look,
    pub black: bool,
    pub text_hidden: bool,
    /// A picture shown alone, such as the church logo; empty when none is set.
    pub logo: String,
    pub logo_on: bool,
    /// A message for the room, such as a call for a parent.
    pub message: String,
    pub countdown: Option<Countdown>,
    pub stage: StageInfo,
    #[serde(default)]
    pub warp: Warp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ToOutput {
    State(Box<OutputState>),
    /// Show each screen's number, so a volunteer can tell them apart.
    Identify(bool),
    /// Where the projector and the stage view go, changed without restarting the process. With
    /// no `audience` choice the projector takes the first screen that is not the primary one,
    /// and with only one screen it shows no window at all unless `here` is set. A pinned screen
    /// that is unplugged falls back to that rule until it returns.
    Place {
        /// Index into the `displays` of the last `Hello`; None lets the output choose.
        #[serde(default)]
        audience: Option<usize>,
        /// Index of the screen the stage view covers; None shows no stage view on a screen.
        #[serde(default)]
        stage: Option<usize>,
        /// Show the stage view in a window when it has no screen.
        #[serde(default)]
        stage_window: bool,
        /// Show the projector as a window on the primary screen when no screen is left for it.
        /// Escape or a click on that window hides it and the output says `ProjectorHidden`.
        #[serde(default)]
        here: bool,
        /// Show no projector at all, whatever screens there are.
        #[serde(default)]
        off: bool,
    },
    Quit,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    /// Physical pixels per point on this screen: 1.0 is 100%, 1.5 is 150%. 0 when unknown.
    #[serde(default)]
    pub scale: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FromOutput {
    Hello {
        displays: Vec<DisplayInfo>,
        /// Index into `displays` of the screen the audience window covers; None when it is an
        /// ordinary window.
        audience: Option<usize>,
        stage: Option<usize>,
        /// Whether this build can play video backgrounds.
        video: bool,
        /// The cameras this computer has, by device name, when the build can show one.
        #[serde(default)]
        cameras: Vec<String>,
    },
    /// A background that could not be shown, with the reason in words. An empty reason means
    /// the problem is over and the operator should clear its notice.
    MediaError(String),
    /// The person at the projector window hid it with Escape or a click. Sent only for the
    /// window shown by `Place { here: true }`; the operator should turn its switch off.
    ProjectorHidden,
    /// A small picture of the moving background (video or camera) as it is on the projector
    /// now, a few times a second: `rgb` is `width * height` RGB pixels as base64
    /// ([`crate::preview`]).
    Preview { width: u32, height: u32, rgb: String },
    /// The moving background stopped or changed; the operator drops the picture.
    PreviewEnd,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Outline;

    #[test]
    fn the_state_line_keeps_the_text_style() {
        let look = Look { ink: [255, 214, 0], font: "Georgia".into(), outline: Outline::Thick, outline_ink: [10, 20, 30], ..Look::default() };
        let state = OutputState { look: look.clone(), ..OutputState::default() };
        let line = encode(&ToOutput::State(Box::new(state)));
        let Some(ToOutput::State(back)) = decode::<ToOutput>(&line) else { panic!("the line decodes") };
        assert_eq!(back.look, look);
    }
}

/// Prints a line to stderr when `ARTOSIA_TRACE` is set, for following the two processes.
pub fn trace(line: impl FnOnce() -> String) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("ARTOSIA_TRACE").is_some()) {
        eprintln!("[artosia {}] {}", std::process::id(), line());
    }
}

/// One message as a line of JSON, newline included.
pub fn encode<T: Serialize>(message: &T) -> String {
    let mut line = serde_json::to_string(message).unwrap_or_default();
    line.push('\n');
    line
}

pub fn decode<'a, T: Deserialize<'a>>(line: &'a str) -> Option<T> {
    serde_json::from_str(line.trim()).ok()
}
