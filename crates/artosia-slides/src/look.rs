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

/// How thick the line around each letter is. The width is a fraction of the screen's height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outline {
    #[default]
    Off,
    Thin,
    Medium,
    Thick,
}

impl Outline {
    pub const ALL: [Outline; 4] = [Outline::Off, Outline::Thin, Outline::Medium, Outline::Thick];

    /// How far the line reaches past the letter, as a fraction of the screen's height.
    pub fn share(self) -> f32 {
        match self {
            Outline::Off => 0.0,
            Outline::Thin => 0.0028,
            Outline::Medium => 0.0052,
            Outline::Thick => 0.0085,
        }
    }

    /// How far the line reaches on a screen `height` points tall.
    pub fn reach(self, height: f32) -> f32 {
        self.share() * height
    }

    /// How many copies of the text, spread round a circle, make the line. More copies for a
    /// wider line, so the corners of a letter stay round.
    pub fn copies(self) -> usize {
        match self {
            Outline::Off => 0,
            Outline::Thin => 8,
            Outline::Medium | Outline::Thick => 16,
        }
    }
}

fn black() -> [u8; 3] {
    [0, 0, 0]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub name: String,
    pub background: Background,
    /// The text colour.
    pub ink: [u8; 3],
    /// The text's font family by name; empty is Artosia's own font. A family this computer
    /// lacks draws in Artosia's font too.
    #[serde(default)]
    pub font: String,
    #[serde(default)]
    pub outline: Outline,
    #[serde(default = "black")]
    pub outline_ink: [u8; 3],
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
            font: String::new(),
            outline: Outline::Off,
            outline_ink: black(),
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

    /// This look with `preset`'s background and text colour. The size, lines, place, font and
    /// outline are the person's own choices and stay.
    pub fn with_preset(&self, preset: Look) -> Look {
        Look {
            max_lines: self.max_lines,
            size: self.size,
            place: self.place,
            font: self.font.clone(),
            outline: self.outline,
            outline_ink: self.outline_ink,
            ..preset
        }
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

/// A colour typed as hex: `ffcc00`, `#ffcc00` or the short `fc0`.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    if !digits.is_ascii() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    let one = |at: usize| u8::from_str_radix(&digits[at..=at], 16).ok().map(|v| v * 17);
    match digits.len() {
        6 => Some([pair(0)?, pair(2)?, pair(4)?]),
        3 => Some([one(0)?, one(1)?, one(2)?]),
        _ => None,
    }
}

/// A colour as the six hex digits `parse_hex` reads back.
pub fn hex_of(color: [u8; 3]) -> String {
    format!("{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_saved_before_the_text_style_still_loads() {
        let old = r#"{"name":"Black","background":{"Solid":[0,0,0]},"ink":[255,255,255],"size":1.0,"weight":700.0,"centered":true,"place":"Middle","shadow":false,"dim":0.35,"max_lines":4}"#;
        let look: Look = serde_json::from_str(old).expect("an old look loads");
        assert_eq!((look.font.as_str(), look.outline, look.outline_ink), ("", Outline::Off, [0, 0, 0]));
        assert_eq!(look.ink, [255, 255, 255]);
    }

    #[test]
    fn the_outline_reaches_a_fraction_of_the_screen_height() {
        for outline in [Outline::Thin, Outline::Medium, Outline::Thick] {
            assert!((outline.reach(2160.0) - 2.0 * outline.reach(1080.0)).abs() < 1e-4);
            assert!(outline.reach(1080.0) >= 2.0, "{outline:?} must show at 1080 high");
        }
        assert_eq!(Outline::Off.reach(1080.0), 0.0);
        assert!(Outline::Thin.reach(1080.0) < Outline::Medium.reach(1080.0) && Outline::Medium.reach(1080.0) < Outline::Thick.reach(1080.0));
    }

    #[test]
    fn a_preset_keeps_the_persons_text_style() {
        let mine = Look { font: "Georgia".into(), outline: Outline::Thick, outline_ink: [255, 255, 255], ink: [1, 2, 3], ..Look::default() };
        let chosen = mine.with_preset(presets().swap_remove(1));
        assert_eq!((chosen.font.as_str(), chosen.outline, chosen.outline_ink), ("Georgia", Outline::Thick, [255, 255, 255]));
        assert_eq!((chosen.name.as_str(), chosen.ink), ("Classic blue", [255, 255, 255]));
    }

    #[test]
    fn hex_colours_read_back() {
        assert_eq!(parse_hex("#FFCC00"), Some([255, 204, 0]));
        assert_eq!(parse_hex("fc0"), Some([255, 204, 0]));
        assert_eq!(parse_hex("ffcc0"), None);
        assert_eq!(parse_hex("gg0000"), None);
        assert_eq!(hex_of([255, 204, 0]), "ffcc00");
    }
}
