//! A song in several languages on one screen: one block per language, the lines of a language
//! together, every language at one size, each block in its own colour (board card MULTI1).
//!
//! The projector and the band's screen both draw their blocks here, so they size, space and
//! order them alike. A block sits in an equal share of the height, its lines centred in it. When
//! the blocks cannot be set at a size readable from the back of the room the slide is split
//! (`artosia_core::deck::split_in_step`) rather than shrunk, by the same `fits` this file gives
//! the deck builder, so the drawing and the splitting never disagree.

use artosia_core::deck::{Languages, Slide};
use artosia_core::song::Song;
use rune::{Color, El, Family, TextStyle, col};

use crate::fonts;
use crate::look::Look;

/// The smallest text size at which a split is not needed, at 1080 lines of height. Below it the
/// back of a room cannot read a block, so the deck makes more slides.
pub const FLOOR: f32 = 48.0;

/// The size a block may still be drawn at when no split can help (a single line too wide), at
/// 1080 lines of height: the one-language slide's own floor.
pub const HARD_FLOOR: f32 = 26.0;

/// The gap between two blocks, at 1080 lines of height.
const GAP: f32 = 28.0;

/// A line's face: a family, a weight and a line height as a multiple of the size.
#[derive(Clone, Copy)]
pub struct Face {
    pub family: Family,
    pub weight: f32,
    pub height: f32,
}

impl Face {
    pub fn at(&self, size: f32) -> TextStyle {
        TextStyle::new(size, self.weight).family(self.family).lh((size * self.height).round())
    }
}

/// The face the projector draws a look's lyrics in.
pub fn face(look: &Look) -> Face {
    Face { family: fonts::family(&look.font), weight: look.weight, height: 1.24 }
}

/// A slide's languages as (place in the list, words), in the order they are drawn.
pub fn of(slide: &Slide) -> Vec<(usize, &str)> {
    std::iter::once((usize::from(slide.slot), slide.text.as_str())).chain(slide.more.iter().map(|voice| (usize::from(voice.slot), voice.text.as_str()))).collect()
}

/// The gap between blocks on a screen `height` high.
pub fn gap(height: f32) -> f32 {
    (GAP * height / 1080.0).round()
}

/// The largest size, from `base` down, at which every block fits one line to a row and its
/// share of the height; `hard_floor` when none does. Each block is measured once per size, as
/// one text: its width is its widest line and its height its lines together.
pub fn fit(texts: &[&str], face: Face, width: f32, height: f32, gap: f32, base: f32, hard_floor: f32) -> f32 {
    let share = (height - gap * (texts.len().saturating_sub(1)) as f32) / texts.len().max(1) as f32;
    let mut size = base;
    loop {
        let style = face.at(size);
        let fits = texts.iter().all(|text| {
            let (wide, tall) = rune::text::measure(text, &[], style, None);
            wide <= width && tall <= share
        });
        if fits || size <= hard_floor {
            return size.max(hard_floor);
        }
        size = (size * 0.94).floor();
    }
}

/// The blocks, top to bottom, each in an equal share of `height` with its lines centred in it.
/// `line` makes the text of one block in a colour; `ink_of` is the colour of a place in the list.
pub fn column(items: &[(usize, &str)], width: f32, height: f32, gap: f32, ink_of: &dyn Fn(usize) -> Color, line: &dyn Fn(&str, Color) -> El) -> El {
    let share = (height - gap * (items.len().saturating_sub(1)) as f32) / items.len().max(1) as f32;
    let mut blocks = col().key("voices").size(width, height).gap(gap);
    for (at, (place, words)) in items.iter().enumerate() {
        blocks = blocks.child(col().key(format!("voice-{at}")).w(width).h(share).justify_center().child(line(words, ink_of(*place))));
    }
    blocks
}

/// The room a slide's text has on a screen `size` (width, height in pixels), as the projector
/// leaves it: every size is a fraction of the height, so the room is measured on a screen 1080
/// high and as wide as the real one is, with the foot of the slide (credit or place) kept free.
fn room(size: (f32, f32)) -> (f32, f32) {
    let width = (1080.0 * size.0 / size.1.max(1.0)).round();
    let (mx, my) = ((width * 0.07).round(), (1080.0f32 * 0.085).round());
    (width - 2.0 * mx, 1080.0 - 2.0 * my - 64.0)
}

/// The shape of a screen nobody has measured: the projector most rooms have.
pub const WIDESCREEN: (f32, f32) = (1920.0, 1080.0);

/// Whether blocks of lines, one per language, can be drawn at a readable size in a look on a
/// screen `size`: the test `song_deck` gives the deck builder. A preview is the 1920x1080 view
/// scaled, so it breaks where a 16:9 projector does; a projector of another shape breaks where
/// `size` says, and the operator passes the real one once the projector has reported it.
pub fn fits(look: &Look, size: (f32, f32)) -> impl Fn(&[&[String]]) -> bool + use<> {
    let (width, height) = room(size);
    let face = face(look);
    let base = (96.0 * look.size).round();
    move |blocks: &[&[String]]| {
        let texts: Vec<String> = blocks.iter().map(|lines| lines.join("\n")).collect();
        let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
        fit(&texts, face, width, height, gap(1080.0), base, HARD_FLOOR) >= FLOOR
    }
}

/// A song's slides in the languages `languages` picks, split where the blocks would not be
/// readable in `look` on a screen `size`. The look and the shape decide the split, so a deck is
/// built again when either changes.
pub fn song_deck(song: &Song, max_lines: usize, licence: &str, languages: &Languages, look: &Look, size: (f32, f32)) -> artosia_core::deck::Deck {
    artosia_core::deck::song_deck_in(song, max_lines, licence, languages, &fits(look, size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use artosia_core::practice::three_languages;

    fn all() -> Languages {
        Languages { shown: vec!["id".into(), "en".into(), "zh-Hans".into()], solo: None }
    }

    /// Both verses of the made-up song as one section of eight lines in each language.
    fn eight_lines() -> Song {
        let mut song = three_languages();
        let merged = |text: &str| {
            let verses: Vec<&str> = text.split("[chorus]\n").collect();
            format!("[1]\n{}{}", verses[0].trim_start_matches("[1]\n"), verses[1].split("[2]\n").nth(1).unwrap_or_default())
        };
        song.lyrics = merged(&song.lyrics);
        for translation in &mut song.translations {
            translation.text = merged(&translation.text);
        }
        song.order = String::new();
        song
    }

    #[test]
    fn a_verse_in_three_languages_fits_and_eight_lines_in_each_do_not() {
        let fits = fits(&Look::default(), WIDESCREEN);
        let song = three_languages();
        let block = |at: usize| song.texts().nth(at).map(|(_, text)| artosia_core::song::parse_lyrics(text)[1].lines.clone()).unwrap_or_default();
        let four: Vec<Vec<String>> = (0..3).map(block).collect();
        let refs: Vec<&[String]> = four.iter().map(Vec::as_slice).collect();
        assert!(fits(&refs));
        let eight: Vec<Vec<String>> = four.iter().map(|lines| lines.iter().chain(lines).cloned().collect()).collect();
        let refs: Vec<&[String]> = eight.iter().map(Vec::as_slice).collect();
        assert!(!fits(&refs));
        // One language alone has the whole height: eight lines of it fit.
        assert!(fits(&refs[..1]));
    }

    #[test]
    fn the_split_follows_the_shape_of_the_real_projector() {
        let look = Look::default();
        let song = three_languages();
        let verse: Vec<Vec<String>> = (0..3).map(|at| song.texts().nth(at).map(|(_, text)| artosia_core::song::parse_lyrics(text)[1].lines.clone()).unwrap_or_default()).collect();
        let refs: Vec<&[String]> = verse.iter().map(Vec::as_slice).collect();
        assert!(fits(&look, WIDESCREEN)(&refs));
        // The same verse on a screen as narrow as a phone stood on its end has no room.
        assert!(!fits(&look, (608.0, 1080.0))(&refs));
        assert!(room((1440.0, 1080.0)).0 < room(WIDESCREEN).0 && room((2560.0, 1080.0)).0 > room(WIDESCREEN).0);
    }

    #[test]
    fn the_deck_splits_exactly_where_the_screen_would_go_under_the_floor() {
        let look = Look::default();
        let deck = song_deck(&eight_lines(), 0, "", &all(), &look, WIDESCREEN);
        assert_eq!(deck.slides.iter().map(|slide| slide.part).collect::<Vec<_>>(), [Some((1, 2)), Some((2, 2))]);
        // Every slide of the split is drawn at or above the floor, in the size the screen will use.
        for slide in &deck.slides {
            let items = of(slide);
            let texts: Vec<&str> = items.iter().map(|(_, words)| *words).collect();
            let (w, h) = room(WIDESCREEN);
            let size = fit(&texts, face(&look), w, h, gap(1080.0), (96.0f32 * look.size).round(), HARD_FLOOR);
            assert!(size >= FLOOR, "{size}");
        }
        // The same song in one language is one slide, however long.
        let alone = song_deck(&eight_lines(), 0, "", &Languages::default(), &look, WIDESCREEN);
        assert_eq!((alone.slides.len(), alone.slides[0].part), (1, None));
    }

    /// What the split costs on this machine, in a debug build: printed with `--nocapture`.
    #[test]
    fn building_a_deck_of_a_three_language_song_is_quick() {
        let look = Look::default();
        let song = three_languages();
        let _ = song_deck(&song, 4, "", &all(), &look, WIDESCREEN);
        let start = std::time::Instant::now();
        let runs = 20;
        let mut slides = 0;
        for _ in 0..runs {
            slides = song_deck(&song, 4, "", &all(), &look, WIDESCREEN).slides.len();
        }
        let plain = start.elapsed() / runs;
        let start = std::time::Instant::now();
        let mut split = 0;
        for _ in 0..runs {
            split = song_deck(&eight_lines(), 0, "", &all(), &look, WIDESCREEN).slides.len();
        }
        let cut = start.elapsed() / runs;
        eprintln!("DECK three languages: {slides} slides in {plain:?}; eight-line section split into {split} slides in {cut:?}");
        assert!(plain.as_millis() < 500 && cut.as_millis() < 500);
    }
}
