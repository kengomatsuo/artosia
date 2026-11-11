//! Sample screens for headless snapshots: `--scene <name> --snap out.png`.

use artosia_core::deck::{Slide, SlideKind};
use artosia_core::song::{SectionKind, Tag};
use artosia_slides::frame::{Countdown, OutputState, StageChip, StageInfo};
use artosia_slides::look::{Background, Look, Outline, presets};

fn slide(kind: SlideKind, label: &str, tag: Option<Tag>, text: &str) -> Slide {
    Slide {
        kind,
        label: label.to_string(),
        short: tag.map(|tag| tag.short()).unwrap_or_default(),
        tag,
        heading: String::new(),
        text: text.to_string(),
        second: String::new(),
        caption: String::new(),
        image: String::new(),
    }
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
