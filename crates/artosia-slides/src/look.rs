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

pub use artosia_core::deck::NoticeStyle;

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
    /// How announcement slides are built.
    #[serde(default)]
    pub notice_style: NoticeStyle,
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
            notice_style: NoticeStyle::Hairline,
        }
    }

    pub fn ink_color(&self) -> Color {
        rgb(self.ink)
    }

    /// The colour of the language at `slot` of the list (see [`voice_ink`]).
    pub fn voice_ink(&self, slot: usize) -> Color {
        voice_ink(self.ink, slot)
    }

    /// This look with `preset`'s background. The size, lines, place, font and outline are the
    /// person's own choices and stay. So does a text colour the person chose, as long as it can
    /// be read on the new background (4.5 to 1, the house rule for text); a colour that only
    /// came with the present look goes with it. A tester made the words yellow and the six
    /// backgrounds still offered white ones.
    pub fn with_preset(&self, preset: Look) -> Look {
        let chosen = presets().iter().find(|known| known.name == self.name).is_none_or(|known| known.ink != self.ink);
        let ink = if chosen && reads_on(self.ink, &preset.background) { self.ink } else { preset.ink };
        Look {
            max_lines: self.max_lines,
            size: self.size,
            place: self.place,
            font: self.font.clone(),
            outline: self.outline,
            outline_ink: self.outline_ink,
            ink,
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

/// The hues the second, third and further languages of a slide are written in, in order: a
/// yellow, a blue, a green, a coral, a violet, an orange (oklch hue angles).
const VOICE_HUES: [f32; 6] = [95.0, 250.0, 150.0, 20.0, 310.0, 60.0];

/// The text colour of the language at `slot` in the list of languages, for text whose own
/// colour is `ink`. The first language is the ink itself. The others are derived from it: a
/// tint of one hue each, light where the ink is light (a dark ground, a photo) and deep where
/// the ink is dark (a light ground), so they read wherever the ink does.
pub fn voice_ink(ink: [u8; 3], slot: usize) -> Color {
    if slot == 0 {
        return rgb(ink);
    }
    let hue = VOICE_HUES[(slot - 1) % VOICE_HUES.len()];
    // 0.179 is the luminance at which white and black text read equally well.
    if luminance(ink) > 0.179 { rune::oklch(0.9, 0.11, hue) } else { rune::oklch(0.4, 0.12, hue) }
}

/// How bright a colour is to the eye, from 0 to 1 (WCAG's relative luminance).
fn luminance(color: [u8; 3]) -> f32 {
    let linear = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.03928 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

/// Whether text of this colour can be read on a plain background: 4.5 to 1 against every
/// colour it is made of. A picture, a video or a camera is not judged.
fn reads_on(ink: [u8; 3], background: &Background) -> bool {
    let against = |ground: [u8; 3]| {
        let (a, b) = (luminance(ink), luminance(ground));
        (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5
    };
    match background {
        Background::Solid(color) => against(*color),
        Background::Gradient(top, bottom) => against(*top) && against(*bottom),
        _ => true,
    }
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
    fn a_chosen_text_colour_stays_where_it_can_be_read() {
        let named = |name: &str| presets().into_iter().find(|look| look.name == name).expect("a preset of that name");
        let yellow = [255, 214, 0];
        let mine = Look { ink: yellow, ..named("Midnight") };
        assert_eq!(mine.with_preset(named("Forest")).ink, yellow, "yellow reads on the dark green");
        assert_eq!(mine.with_preset(named("Paper")).ink, named("Paper").ink, "yellow cannot be read on cream");
        // A colour that only came with the look goes with it.
        assert_eq!(named("Midnight").with_preset(named("Ember")).ink, named("Ember").ink);
    }

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

    /// Contrast of a colour (linear, as Rune holds it) against a plain ground.
    fn contrast(ink: Color, ground: [u8; 3]) -> f32 {
        let (a, b) = (ink.luminance(), luminance(ground));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn the_first_language_is_the_ink_and_the_others_read_on_every_look() {
        for look in presets() {
            assert_eq!(look.voice_ink(0), look.ink_color());
            let grounds = match &look.background {
                Background::Solid(color) => vec![*color],
                Background::Gradient(top, bottom) => vec![*top, *bottom],
                _ => vec![],
            };
            for slot in 1..=8 {
                for ground in &grounds {
                    let ratio = contrast(look.voice_ink(slot), *ground);
                    assert!(ratio >= 4.5, "{} slot {slot} reads {ratio:.1} to 1 on {ground:?}", look.name);
                }
            }
        }
        // Over a photo the dim darkens it and the ink is light: the tints are the light ones.
        let photo = Look { background: Background::Image("x.jpg".into()), dim: 0.35, ..Look::default() };
        assert!(photo.voice_ink(1).luminance() > 0.5);
        // The colours differ from one another and from the ink.
        let dark = Look::default();
        let all: Vec<Color> = (0..4).map(|slot| dark.voice_ink(slot)).collect();
        assert!(all.iter().enumerate().all(|(at, one)| all.iter().skip(at + 1).all(|other| one != other)));
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
