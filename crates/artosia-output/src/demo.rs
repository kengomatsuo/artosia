//! Sample screens for headless snapshots: `--scene <name> --snap out.png`.

use artosia_core::deck::{Slide, SlideKind};
use artosia_core::deck::Languages;
use artosia_core::practice;
use artosia_core::song::{SectionKind, Tag};
use artosia_slides::frame::{Countdown, OutputState, StageChip, StageInfo};
use artosia_slides::look::{Background, Look, Outline, presets};
use artosia_slides::voices;

fn slide(kind: SlideKind, label: &str, tag: Option<Tag>, text: &str) -> Slide {
    Slide { kind, label: label.to_string(), short: tag.map(|tag| tag.short()).unwrap_or_default(), tag, text: text.to_string(), ..Slide::default() }
}

fn chorus() -> Slide {
    slide(
        SlideKind::Lyrics,
        "Chorus",
        Some(Tag::new(SectionKind::Chorus, 1)),
        "Sing it out across the water\nEvery heart together now\nFrom the hill to the open sea\nLet the morning carry our song",
    )
}

fn stage_info() -> StageInfo {
    let chip = |text: &str, group: usize, active: bool| StageChip { text: text.to_string(), group, active };
    StageInfo {
        title: "Morning on the Hill".to_string(),
        next: Some(slide(
            SlideKind::Lyrics,
            "Verse 2",
            Some(Tag::new(SectionKind::Verse, 2)),
            "The road is long, the well is deep\nYet you have been our keeper",
        )),
        next_title: String::new(),
        index: 1,
        count: 6,
        chips: vec![chip("V1", 0, false), chip("C", 1, true), chip("V2", 0, false), chip("C", 1, false), chip("B", 2, false), chip("C", 1, false)],
        message: String::new(),
        words: Default::default(),
    }
}

/// The made-up song in `codes`, on the chorus slide, over a preset look, for the projector and
/// the band's screen. `split` shows the worst case: both verses of the song as one section of
/// eight lines in each language, which the deck builder must cut in two.
pub fn languages(codes: &[&str], look_name: &str, split: bool) -> OutputState {
    let mut song = practice::three_languages();
    if split {
        // One section holding verse 1 and verse 2 in every language, so no slide break helps.
        let merged = |text: &str| {
            let verses: Vec<&str> = text.split("[chorus]
").collect();
            let second = verses[1].split("[2]
").nth(1).unwrap_or_default();
            format!("[1]
{}{second}", verses[0].trim_start_matches("[1]
"))
        };
        song.lyrics = merged(&song.lyrics);
        for translation in &mut song.translations {
            translation.text = merged(&translation.text);
        }
        song.order = String::new();
    }
    let look = look(look_name);
    let shown = Languages { shown: codes.iter().map(|code| code.to_string()).collect(), solo: None };
    let max_lines = if split { 0 } else { look.max_lines };
    let deck = voices::song_deck(&song, max_lines, "", &shown, &look, voices::WIDESCREEN);
    let at = deck.slides.iter().position(|slide| slide.label == "Chorus" || split).unwrap_or(0);
    let chip = |text: &str, group: usize, active: bool| StageChip { text: text.to_string(), group, active };
    let stage = StageInfo {
        title: song.title.clone(),
        next: deck.slides.get(at + 1).cloned(),
        index: at,
        count: deck.slides.len(),
        chips: vec![chip("V1", 0, false), chip("C", 1, true), chip("V2", 0, false), chip("C", 1, false)],
        ..StageInfo::default()
    };
    OutputState { slide: deck.slides.get(at).cloned(), look, stage, ..OutputState::default() }
}

fn look(name: &str) -> Look {
    presets().into_iter().find(|look| look.name == name).unwrap_or_default()
}

pub fn lyrics() -> OutputState {
    OutputState { slide: Some(chorus()), stage: stage_info(), ..OutputState::default() }
}

pub fn two_languages() -> OutputState {
    let mut slide = slide(
        SlideKind::Lyrics,
        "Verse 1",
        Some(Tag::new(SectionKind::Verse, 1)),
        "Your faithful love is new each morning\nGreat is your faithfulness, O Lord",
    );
    slide.second = "Kasih setia-Mu baru tiap pagi\nBesar kesetiaan-Mu ya Tuhan".to_string();
    slide.caption = "Artosia practice song".to_string();
    OutputState { slide: Some(slide), stage: stage_info(), ..OutputState::default() }
}

pub fn scripture() -> OutputState {
    let mut verse = slide(
        SlideKind::Scripture,
        "John 3:16",
        None,
        "For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.",
    );
    verse.caption = "John 3:16 KJV".to_string();
    let mut stage = stage_info();
    stage.title = "John 3:16-17".to_string();
    stage.chips.clear();
    stage.message = "Five minutes left".to_string();
    OutputState { slide: Some(verse), stage, ..OutputState::default() }
}

pub fn announcement() -> OutputState {
    let mut notice = slide(SlideKind::Announcement, "Youth night", None, "Friday at 7 pm in the hall.\nBring a friend.");
    notice.heading = "Youth night".to_string();
    OutputState { slide: Some(notice), look: look("Forest"), ..OutputState::default() }
}

pub fn countdown(now: f64) -> OutputState {
    OutputState {
        countdown: Some(Countdown { label: "We start in".to_string(), ends_at: now + 299.0 }),
        stage: StageInfo::default(),
        ..OutputState::default()
    }
}

pub fn message() -> OutputState {
    OutputState { message: "Parent of child 12, please come to the nursery".to_string(), ..lyrics() }
}

pub fn classic() -> OutputState {
    OutputState { look: look("Classic blue"), ..lyrics() }
}

pub fn paper() -> OutputState {
    OutputState { look: look("Paper"), ..lyrics() }
}

/// Yellow text in Georgia with a thick black outline, over a dark gradient.
pub fn styled() -> OutputState {
    let look = Look { ink: [255, 214, 0], font: "Georgia".to_string(), outline: Outline::Thick, outline_ink: [0, 0, 0], ..look("Midnight") };
    OutputState { look, ..lyrics() }
}

/// White text with a medium black outline over a bright background, where a picture would be.
pub fn outlined() -> OutputState {
    let look = Look { background: Background::Solid([232, 206, 140]), ink: [255, 255, 255], outline: Outline::Medium, ..look("Black") };
    OutputState { look, ..lyrics() }
}

/// A font this computer lacks: the words must still show, in the default font.
pub fn missing_font() -> OutputState {
    let look = Look { font: "No Such Family 123".to_string(), outline: Outline::Thin, ..look("Midnight") };
    OutputState { look, ..lyrics() }
}

pub fn black() -> OutputState {
    OutputState { black: true, ..lyrics() }
}
