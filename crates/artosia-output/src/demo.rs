//! Sample screens for headless snapshots: `--scene <name> --snap out.png`.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use artosia_core::deck::{NoticeKind, Slide, SlideKind};
use rune::{Color, El, TextStyle, el, text};
use artosia_slides::audience::{Detail, Media, audience};
use artosia_slides::notice;
use artosia_slides::look::{NoticeStyle, rgb};
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

/// The five kinds of announcement, made-up data in Indonesian: the title, the words and what
/// the sheet calls the kind in English.
const NOTICES: [(NoticeKind, &str, &str); 5] = [
    (NoticeKind::Schedule, "Schedule", "Jadwal kegiatan gereja"),
    (NoticeKind::Birthdays, "Birthdays", "Ulang tahun bulan November"),
    (NoticeKind::Money, "Money", "Persembahan syukur"),
    (NoticeKind::Account, "Account", "Persembahan lewat transfer"),
    (NoticeKind::Plain, "Plain", "Ibadah Padang"),
];

fn notice_text(kind: NoticeKind) -> String {
    let lines: Vec<String> = match kind {
        NoticeKind::Schedule => [
            "Minggu\t06.30\tIbadah Raya 1",
            "\t09.00\tIbadah Raya 2",
            "\t09.00\tSekolah Minggu dan remaja",
            "Rabu\t18.30\tIbadah doa",
            "Jumat\t18.30\tPersekutuan pemuda",
            "Sabtu\t17.00\tLatihan pujian",
        ]
        .map(String::from)
        .to_vec(),
        NoticeKind::Birthdays => {
            const WORDS: [&str; 20] = ["Alfa", "Beta", "Gama", "Delta", "Epsilon", "Zeta", "Eta", "Teta", "Iota", "Kapa", "Lamda", "Mu", "Nu", "Ksi", "Omikron", "Pi", "Ro", "Sigma", "Tau", "Upsilon"];
            // One word or two in turn, so the names differ in length; two only when short, or
            // three columns of them would not fit a slide at the smallest readable size.
            let wants = [2, 1, 2, 1, 2, 1];
            let mut next = 0;
            (0..30)
                .map(|at| {
                    let (first, second) = (WORDS[next % WORDS.len()], WORDS[(next + 1) % WORDS.len()]);
                    let words = if wants[at % wants.len()] == 2 && first.len() + second.len() <= 6 { vec![first, second] } else { vec![first] };
                    next += words.len();
                    // Some days twice, the last day the 30th.
                    format!("Contoh {}\t{} November", words.join(" "), at + 1 - usize::from(at % 6 == 3))
                })
                .collect()
        }
        NoticeKind::Money => [
            "Kel. Contoh Alfa\tRp 1.500.000",
            "Ibu Contoh Beta\tRp 500.000",
            "Bpk. Contoh Gama\tRp 100.000",
            "Kel. Contoh Delta\tRp 2.000.000",
            "Kel. Contoh Zeta\tRp 3.000.000",
            "Ibu Contoh Eta\tRp 400.000",
            "Ibu Contoh Teta dan Sdr. Contoh Iota\tRp 500.000",
            "Ibu Contoh Kapa\tRp 250.000",
            "Ibu Contoh Lamda\tRp 200.000",
            "== Total\tRp 8.450.000",
        ]
        .map(String::from)
        .to_vec(),
        NoticeKind::Account => ["Bank Contoh", "123 456 7890", "a.n. Gereja Contoh"].map(String::from).to_vec(),
        NoticeKind::Plain => ["Sabtu, 8 November 2026", "Jam 10.00"].map(String::from).to_vec(),
    };
    lines.join("\n")
}

fn notice_slide(kind: NoticeKind) -> Slide {
    let (_, _, title) = NOTICES.iter().copied().find(|(known, _, _)| *known == kind).unwrap_or(NOTICES[4]);
    Slide { kind: SlideKind::Announcement, notice: kind, label: title.to_string(), heading: title.to_string(), text: notice_text(kind), ..Slide::default() }
}

/// One announcement slide in a notice style over the default look.
pub fn notice(style: NoticeStyle, kind: NoticeKind) -> OutputState {
    OutputState { slide: Some(notice_slide(kind)), look: Look { notice_style: style, ..Look::default() }, ..OutputState::default() }
}

/// The style whose sheet the window draws, if a sheet scene asked for one.
static SHEET: Mutex<Option<NoticeStyle>> = Mutex::new(None);

pub fn show_sheet(style: NoticeStyle) -> OutputState {
    *SHEET.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(style);
    OutputState::default()
}

/// The five slides of the sheet's style on a neutral page, three on the first row and two on
/// the second, each with its kind's English name under it, drawn by the projector's own code.
pub fn sheet(w: f32, h: f32) -> Option<El> {
    let style = (*SHEET.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))?;
    let name = match style {
        NoticeStyle::Hairline => "Hairline",
        NoticeStyle::Ledger => "Ledger",
        NoticeStyle::Band => "Band",
    };
    let u = h / 1200.0;
    let (tile_w, gap) = ((w * 0.30).round(), (w * 0.025).round());
    let tile_h = (tile_w * 9.0 / 16.0).round();
    let margin = ((w - 3.0 * tile_w - 2.0 * gap) / 2.0).round();
    let mut page = el().key("sheet").absolute(0.0, 0.0).size(w, h).fill(rgb([26, 27, 31])).child(
        text(name).font(TextStyle::new((84.0 * u).round(), 800.0)).color(Color::WHITE).absolute(margin, (36.0 * u).round()),
    );
    for (at, (kind, english, _)) in NOTICES.iter().enumerate() {
        let (column, line) = ((at % 3) as f32, (at / 3) as f32);
        let (x, y) = (margin + column * (tile_w + gap), (190.0 * u).round() + line * (tile_h + (110.0 * u).round()));
        let state = notice(style, *kind);
        page = page
            .child(tile(&state, tile_w).absolute(x, y))
            .child(text(*english).font(TextStyle::new((30.0 * u).round(), 500.0)).color(Color::WHITE.alpha(0.7)).absolute(x, y + tile_h + (14.0 * u).round()));
    }
    Some(page)
}

/// A plain slide in the default style is drawn by the old text code unless something sends it
/// to `notice.rs`; the demo scene and the sheet's tile do, so the Plain kind is seen there too.
static DIRECT: AtomicBool = AtomicBool::new(false);

pub fn direct() -> bool {
    DIRECT.load(Ordering::Acquire)
}

/// The slide of `state` drawn by `notice.rs` over its look's background.
pub fn through_notice(state: &OutputState, w: f32, h: f32) -> El {
    let mut bare = state.clone();
    let slide = bare.slide.take();
    let mut page = el().key("direct").size(w, h).child(audience(&bare, w, h, Media::default(), Detail::Preview, 0.0).absolute(0.0, 0.0));
    if let Some(slide) = slide {
        page = page.child(notice::notice(&slide, &state.look, w, h));
    }
    page
}

fn tile(state: &OutputState, width: f32) -> El {
    let (w, h) = (1920.0, 1080.0);
    el().size(width, (width * 9.0 / 16.0).round()).child(through_notice(state, w, h).absolute(0.0, 0.0).origin(0.0, 0.0).scale(width / w).inert())
}

/// The state of a `notice-<style>-<kind>` scene.
pub fn notice_scene(style: NoticeStyle, kind: NoticeKind) -> OutputState {
    DIRECT.store(style == NoticeStyle::Hairline && kind == NoticeKind::Plain, Ordering::Release);
    notice(style, kind)
}

/// The state of a `notice-<style>-birthdays-marked` scene: the thirty names with seven rows, as a past week would, standing out.
pub fn notice_marked_scene(style: NoticeStyle) -> OutputState {
    let mut state = notice_scene(style, NoticeKind::Birthdays);
    if let Some(slide) = state.slide.as_mut() {
        slide.marked = (6..13).collect();
    }
    state
}
