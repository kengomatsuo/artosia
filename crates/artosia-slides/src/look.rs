//! The look: the one place that styles everything the projector shows.

use rune::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Background {
    Solid([u8; 3]),
    /// Top colour, bottom colour.
    Gradient([u8; 3], [u8; 3]),
    /// A picture file.
    Image(String),
    /// A video file, looped.
    Video(String),
    /// A live camera by its device name; an empty name is the first camera found.
    Camera(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub name: String,
    pub background: Background,
    pub ink: [u8; 3],
    /// Text size as a multiple of the standard size.
    pub size: f32,
    pub weight: f32,
    pub centered: bool,
    pub place: Place,
    /// A soft shadow under the text, for pictures and video.
    pub shadow: bool,
    /// How much a picture or video is darkened behind the text, 0 to 1.
    pub dim: f32,
    /// The most lyric lines on one slide; 0 keeps each section whole.
    pub max_lines: usize,
}

pub fn rgb(c: [u8; 3]) -> Color {
    let channel = |v: u8| rune::color::srgb_decode(f32::from(v) / 255.0);
    Color { r: channel(c[0]), g: channel(c[1]), b: channel(c[2]), a: 1.0 }
}

impl Look {
    fn base(name: &str, background: Background, ink: [u8; 3]) -> Look {
        Look {
            name: name.to_string(),
            background,
            ink,
            size: 1.0,
            weight: 700.0,
            centered: true,
            place: Place::Middle,
            shadow: false,
            dim: 0.35,
            max_lines: 4,
        }
    }

    pub fn ink_color(&self) -> Color {
        rgb(self.ink)
    }

    /// Whether the background is a file that has to be loaded.
    pub fn media_path(&self) -> Option<&str> {
        match &self.background {
            Background::Image(path) | Background::Video(path) => Some(path),
            _ => None,
        }
    }

    /// Whether the background is a picture, a video or a camera, which text sits over.
    pub fn is_media(&self) -> bool {
        matches!(self.background, Background::Image(_) | Background::Video(_) | Background::Camera(_))
    }

    /// Whether the background arrives as moving frames.
    pub fn is_moving(&self) -> bool {
        matches!(self.background, Background::Video(_) | Background::Camera(_))
    }
}

impl Default for Look {
    fn default() -> Self {
        presets().swap_remove(0)
    }
}

/// The looks Artosia ships. "Classic blue" is the EasiSlides default, kept so a church that
/// moves over sees its own screen.
pub fn presets() -> Vec<Look> {
    vec![
        Look::base("Midnight", Background::Gradient([16, 24, 48], [6, 8, 18]), [255, 255, 255]),
        Look::base("Classic blue", Background::Solid([0, 0, 160]), [255, 255, 255]),
        Look::base("Black", Background::Solid([0, 0, 0]), [255, 255, 255]),
        Look::base("Forest", Background::Gradient([14, 52, 44], [4, 18, 16]), [244, 248, 240]),
        Look::base("Ember", Background::Gradient([74, 24, 32], [20, 6, 12]), [255, 244, 232]),
        Look { weight: 600.0, ..Look::base("Paper", Background::Solid([246, 242, 232]), [28, 26, 24]) },
    ]
}
