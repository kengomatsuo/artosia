//! The operator window's tokens: a light and a dark kit, the colours with a meaning, and the
//! icons. The window follows the computer's light or dark setting unless Look says otherwise.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::{Color, Icon, TextStyle, oklch};
use rune_kit::{Kit, PaletteSeed};

static DARK: AtomicBool = AtomicBool::new(true);

/// Chooses the kit every later `kit()` call returns. The view calls it once a frame.
pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Red means "on the projector now" and nothing else. Deep enough in both themes for white
/// words on it to read at 4.5 to 1.
pub fn live() -> Color {
    live_for(is_dark())
}

fn live_for(dark: bool) -> Color {
    if dark { oklch(0.52, 0.21, 25.0) } else { oklch(0.5, 0.21, 25.0) }
}

/// Text on a `live` fill.
pub fn on_live() -> Color {
    Color::WHITE
}

/// Yellow means a warning or something hidden from the room. A fill; text on it is black.
pub fn warn() -> Color {
    oklch(0.86, 0.16, 92.0)
}

/// A warning written as words on the window itself.
pub fn warn_ink() -> Color {
    if is_dark() { warn() } else { oklch(0.48, 0.11, 75.0) }
}

pub fn accent() -> Color {
    if is_dark() { oklch(0.68, 0.15, 256.0) } else { oklch(0.5, 0.18, 256.0) }
}

/// A section colour as text or an icon on the window. The colours are picked for a dark
/// slide: on a dark window they are lifted toward white, on a light one deepened, until they
/// read on their own tint.
pub fn ink_of(color: Color) -> Color {
    ink_for(color, is_dark())
}

fn ink_for(color: Color, dark: bool) -> Color {
    if dark {
        let lift = |channel: f32| channel + (1.0 - channel) * 0.45;
        Color { r: lift(color.r), g: lift(color.g), b: lift(color.b), a: color.a }
    } else {
        Color { r: color.r * 0.24, g: color.g * 0.24, b: color.b * 0.24, a: color.a }
    }
}

/// The tint behind a section's chip.
pub fn tint_of(color: Color, strong: bool) -> Color {
    tint_for(color, strong, is_dark())
}

fn tint_for(color: Color, strong: bool, dark: bool) -> Color {
    color.alpha(match (dark, strong) {
        (true, false) => 0.14,
        (true, true) => 0.2,
        (false, false) => 0.2,
        (false, true) => 0.36,
    })
}

fn dark_seed() -> PaletteSeed {
    PaletteSeed { bg: oklch(0.19, 0.006, 265.0), fg: oklch(0.955, 0.004, 265.0), accent: oklch(0.68, 0.15, 256.0) }
}

fn light_seed() -> PaletteSeed {
    PaletteSeed { bg: oklch(0.985, 0.003, 265.0), fg: oklch(0.21, 0.012, 265.0), accent: oklch(0.5, 0.18, 256.0) }
}

fn light_kit() -> Kit {
    let mut kit = Kit::from_palette(light_seed());
    // The stock grey is 3.7 to 1 on the grey panels; these read at 4.5 and over.
    kit.c.text_muted = oklch(0.4, 0.014, 265.0);
    kit.c.text_faint = oklch(0.46, 0.014, 265.0);
    kit
}

/// The kit for the window as it is now, light or dark.
pub fn kit() -> &'static Kit {
    static DARK_KIT: OnceLock<Kit> = OnceLock::new();
    static LIGHT_KIT: OnceLock<Kit> = OnceLock::new();
    if is_dark() { DARK_KIT.get_or_init(|| Kit::from_palette(dark_seed())) } else { LIGHT_KIT.get_or_init(light_kit) }
}

/// The surfaces of the window, from the back of it to the front. Panels differ by tone, so
/// which panel is which reads at a glance in both themes (owner, 2026-11-10: the dark window
/// "doesn't show at all the section hierarchy").
#[derive(Clone, Copy)]
pub struct Surf {
    /// The title bar and the status bar.
    pub chrome: Color,
    /// The plan, and the strips under the two lists.
    pub side: Color,
    /// Behind an editor.
    pub work: Color,
    /// A pane lifted off the window: the two lists.
    pub raised: Color,
    /// The title strip of a panel.
    pub header: Color,
    /// The hairline between panels.
    pub edge: Color,
}

pub fn surf() -> Surf {
    surf_for(is_dark())
}

fn surf_for(dark: bool) -> Surf {
    if dark {
        Surf {
            chrome: oklch(0.14, 0.008, 265.0),
            side: oklch(0.18, 0.01, 265.0),
            work: oklch(0.235, 0.01, 265.0),
            raised: oklch(0.29, 0.012, 265.0),
            header: oklch(0.26, 0.012, 265.0),
            edge: oklch(0.36, 0.012, 265.0),
        }
    } else {
        Surf {
            chrome: oklch(0.9, 0.008, 265.0),
            side: oklch(0.94, 0.007, 265.0),
            work: oklch(0.99, 0.002, 265.0),
            raised: oklch(1.0, 0.0, 0.0),
            header: oklch(0.915, 0.008, 265.0),
            edge: oklch(0.8, 0.01, 265.0),
        }
    }
}

pub mod space {
    pub const S1: f32 = 4.0;
    pub const S2: f32 = 8.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;
    pub const S5: f32 = 20.0;
    pub const S6: f32 = 24.0;
}

pub mod size {
    /// Height of every list row.
    pub const ROW: f32 = 40.0;
    /// Height of a search result, which has two lines.
    pub const RESULT: f32 = 54.0;
    pub const TOPBAR: f32 = 52.0;
    pub const STATUS: f32 = 32.0;
    pub const PLAN: f32 = 272.0;
}

pub mod type_ {
    use super::TextStyle;
    pub const BODY: TextStyle = TextStyle::new(13.0, 400.0).lh(18.0);
    pub const BODY_STRONG: TextStyle = TextStyle::new(13.0, 600.0).lh(18.0);
    pub const SMALL: TextStyle = TextStyle::new(12.0, 400.0).lh(16.0);
    pub const SMALL_STRONG: TextStyle = TextStyle::new(12.0, 600.0).lh(16.0);
    pub const TINY: TextStyle = TextStyle::new(11.0, 600.0).lh(14.0);
    pub const HEADING: TextStyle = TextStyle::new(18.0, 650.0).lh(24.0);
    pub const TITLE: TextStyle = TextStyle::new(15.0, 600.0).lh(20.0);
    pub const SEARCH: TextStyle = TextStyle::new(17.0, 400.0).lh(24.0);
    pub const CLOCK: TextStyle = TextStyle::new(15.0, 600.0).lh(20.0).features("'tnum' 1");
    pub const TIMER: TextStyle = TextStyle::new(22.0, 650.0).lh(28.0).features("'tnum' 1");
}

macro_rules! icon {
    ($name:ident, $id:literal, $body:literal) => {
        pub static $name: Icon = Icon { name: $id, view: 16.0, body: $body };
    };
}

/// Line icons on a 16 point grid, drawn for this app.
pub mod icons {
    use super::Icon;
    icon!(SONG, "a-song", r#"<path d="M6 12.5V3.6L13 2.2V11" stroke="currentColor" stroke-linejoin="round"/><circle cx="4.3" cy="12.5" r="1.7" stroke="currentColor"/><circle cx="11.3" cy="11" r="1.7" stroke="currentColor"/>"#);
    icon!(BOOK, "a-book", r#"<path d="M8 4.5V13.5M8 4.5C7 3.5 5.2 3 2.5 3V12C5.2 12 7 12.5 8 13.5M8 4.5C9 3.5 10.8 3 13.5 3V12C10.8 12 9 12.5 8 13.5" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(ANNOUNCE, "a-announce", r#"<path d="M2.5 6.5V9.5H4.5L9.5 12.5V3.5L4.5 6.5H2.5Z" stroke="currentColor" stroke-linejoin="round"/><path d="M11.8 6C12.6 6.5 13 7.2 13 8C13 8.8 12.6 9.5 11.8 10" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(PLUS, "a-plus", r#"<path d="M8 3V13M3 8H13" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(CLOSE, "a-close", r#"<path d="M4 4L12 12M12 4L4 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.6"/>"#);
    icon!(ARROW_LEFT, "a-arrow-left", r#"<path d="M13.5 8H2.5M7 3.5L2.5 8L7 12.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(ARROW_RIGHT, "a-arrow-right", r#"<path d="M2.5 8H13.5M9 3.5L13.5 8L9 12.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(ARROW_UP, "a-arrow-up", r#"<path d="M8 13V3M4 7L8 3L12 7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(ARROW_DOWN, "a-arrow-down", r#"<path d="M8 3V13M4 9L8 13L12 9" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(SCREEN, "a-screen", r#"<rect x="1.5" y="2.5" width="13" height="9" rx="1" stroke="currentColor"/><path d="M5.5 14H10.5M8 11.5V14" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(PENCIL, "a-pencil", r#"<path d="M2.5 13.5L3.2 10.5L10.5 3.2L12.8 5.5L5.5 12.8L2.5 13.5Z" stroke="currentColor" stroke-linejoin="round"/><path d="M9.2 4.5L11.5 6.8" stroke="currentColor"/>"#);
    icon!(TRASH, "a-trash", r#"<path d="M2.5 4.5H13.5M6 4.5V2.5H10V4.5M4 4.5L4.6 13.5H11.4L12 4.5M6.7 7V11M9.3 7V11" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(DOTS, "a-dots", r#"<circle cx="3.5" cy="8" r="1.1" fill="currentColor"/><circle cx="8" cy="8" r="1.1" fill="currentColor"/><circle cx="12.5" cy="8" r="1.1" fill="currentColor"/>"#);
    icon!(IMAGE, "a-image", r#"<rect x="1.5" y="2.5" width="13" height="11" rx="1.5" stroke="currentColor"/><circle cx="5.5" cy="6.5" r="1.2" stroke="currentColor"/><path d="M2 12L6 8L9 11L11 9L14 12" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(PHONE, "a-phone", r#"<rect x="4.5" y="1.5" width="7" height="13" rx="1.5" stroke="currentColor"/><path d="M7.4 12.3H8.6" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(IMPORT, "a-import", r#"<path d="M8 2.5V10.5M4.5 7.5L8 11L11.5 7.5M2.5 13.5H13.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(PASTE, "a-paste", r#"<path d="M5.5 3.5H3.5V14H12.5V3.5H10.5" stroke="currentColor" stroke-linejoin="round"/><rect x="5.5" y="2" width="5" height="3" rx="0.8" stroke="currentColor"/><path d="M6 8.5H10M6 11H9" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(LOOK, "a-look", r#"<circle cx="8" cy="8" r="6" stroke="currentColor"/><path d="M8 2V14C4.7 14 2 11.3 2 8C2 4.7 4.7 2 8 2Z" fill="currentColor"/>"#);
    icon!(KEYBOARD, "a-keyboard", r#"<rect x="1.5" y="4" width="13" height="8.5" rx="1.5" stroke="currentColor"/><path d="M4 6.8H4.6M7 6.8H7.6M10 6.8H10.6M4.5 10H11.5" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(PLAY, "a-play", r#"<path d="M4.5 3L12.5 8L4.5 13V3Z" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(HIDE, "a-hide", r#"<path d="M1.8 8C3.2 5.3 5.4 4 8 4C10.6 4 12.8 5.3 14.2 8C12.8 10.7 10.6 12 8 12C5.4 12 3.2 10.7 1.8 8Z" stroke="currentColor" stroke-linejoin="round"/><path d="M3 13L13 3" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(TIMER, "a-timer", r#"<circle cx="8" cy="9" r="5" stroke="currentColor"/><path d="M8 6.2V9L9.8 10.2M6.5 1.8H9.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(UNDO, "a-undo", r#"<path d="M5.5 3.5L2.5 6.5L5.5 9.5M2.5 6.5H9.5C11.7 6.5 13.5 8.1 13.5 10.2C13.5 12.2 11.7 13.5 9.5 13.5H7" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(GLOBE, "a-globe", r#"<circle cx="8" cy="8" r="6" stroke="currentColor"/><path d="M2 8H14M8 2C6 4 6 12 8 14M8 2C10 4 10 12 8 14" stroke="currentColor"/>"#);
    icon!(COPY, "a-copy", r#"<rect x="5.5" y="5.5" width="8" height="8" rx="1.2" stroke="currentColor"/><path d="M10.5 5.5V3.5C10.5 2.9 10.1 2.5 9.5 2.5H3.5C2.9 2.5 2.5 2.9 2.5 3.5V9.5C2.5 10.1 2.9 10.5 3.5 10.5H5.5" stroke="currentColor"/>"#);
    icon!(LOOP, "a-loop", r#"<path d="M3 7C3 5.3 4.3 4 6 4H12.5M10.5 2L12.5 4L10.5 6M13 9C13 10.7 11.7 12 10 12H3.5M5.5 10L3.5 12L5.5 14" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    // Artosia's mark, an A cut out of a round loaf (docs/brand.md, assets/brand/artosia-mark-mono.svg).
    icon!(LOGO, "a-logo", r#"<path d="M1.723 14.48L3.346 14.48L5.059 10.24L9.147 10.24L8.5 8.64L5.705 8.64L8 2.96L12.654 14.48L14.277 14.48A8 8 0 1 0 1.723 14.48Z" fill="currentColor"/>"#);
    icon!(MIC, "a-mic", r#"<rect x="6" y="1.8" width="4" height="7.4" rx="2" stroke="currentColor"/><path d="M3.5 7.5C3.5 10 5.5 11.8 8 11.8C10.5 11.8 12.5 10 12.5 7.5M8 11.8V14.2M5.8 14.2H10.2" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(GRIP, "a-grip", r#"<circle cx="6" cy="4" r="1.1" fill="currentColor"/><circle cx="10" cy="4" r="1.1" fill="currentColor"/><circle cx="6" cy="8" r="1.1" fill="currentColor"/><circle cx="10" cy="8" r="1.1" fill="currentColor"/><circle cx="6" cy="12" r="1.1" fill="currentColor"/><circle cx="10" cy="12" r="1.1" fill="currentColor"/>"#);
    icon!(SPACE_BAR, "a-space-bar", r#"<path d="M2.5 6.5V10H13.5V6.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(SEND, "a-send", r#"<path d="M2.5 8H10.5M7.5 4.5L11 8L7.5 11.5M13.5 3V13" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(ON_AIR, "a-on-air", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor"/><circle cx="8" cy="8" r="2.6" fill="currentColor"/>"#);
    icon!(OFF_AIR, "a-off-air", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor"/>"#);
    icon!(NO_TEXT, "a-no-text", r#"<path d="M4 4.5H12M8 4.5V12" stroke="currentColor" stroke-linecap="round"/><path d="M2.5 13.5L13.5 2.5" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(BLACK_SCREEN, "a-black-screen", r#"<rect x="2" y="3.5" width="12" height="8" rx="1.5" fill="currentColor"/><path d="M6 13.5H10" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(BADGE, "a-badge", r#"<circle cx="8" cy="7" r="4.5" stroke="currentColor"/><path d="M6 10.8L5 14L8 12.6L11 14L10 10.8" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(HISTORY, "a-history", r#"<circle cx="8" cy="8" r="5.5" stroke="currentColor"/><path d="M8 5V8L10 9.5" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
    icon!(ITEM_BEFORE, "a-item-before", r#"<path d="M4 3.5H12" stroke="currentColor" stroke-linecap="round"/><path d="M8 6.5L11.5 12H4.5Z" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(ITEM_AFTER, "a-item-after", r#"<path d="M4 12.5H12" stroke="currentColor" stroke-linecap="round"/><path d="M8 9.5L4.5 4H11.5Z" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(MINUS, "a-minus", r#"<path d="M3 8H13" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(CAMERA, "a-camera", r#"<rect x="1.5" y="4.5" width="9" height="7" rx="1.5" stroke="currentColor"/><path d="M10.5 7.2L14.5 5V11L10.5 8.8" stroke="currentColor" stroke-linejoin="round"/>"#);
    icon!(STREAM, "a-stream", r#"<circle cx="8" cy="8" r="1.6" fill="currentColor"/><path d="M4.6 4.6C2.7 6.5 2.7 9.5 4.6 11.4M11.4 4.6C13.3 6.5 13.3 9.5 11.4 11.4M2.4 2.6C-0.4 5.6 -0.4 10.4 2.4 13.4M13.6 2.6C16.4 5.6 16.4 10.4 13.6 13.4" stroke="currentColor" stroke-linecap="round"/>"#);
    icon!(HEADING, "a-heading", r#"<path d="M2.5 8H13.5" stroke="currentColor" stroke-linecap="round"/>"#);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrast(a: Color, b: Color) -> f32 {
        let (a, b) = (a.luminance(), b.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// A see-through colour laid over a solid one.
    fn over(top: Color, under: Color) -> Color {
        let mix = |t: f32, u: f32| t * top.a + u * (1.0 - top.a);
        Color { r: mix(top.r, under.r), g: mix(top.g, under.g), b: mix(top.b, under.b), a: 1.0 }
    }

    /// The house rule is 4.5 to 1 for text. The section chips and the Live pill missed it on
    /// the first dark window (3.0 to 3.7), found by measuring on 2026-11-10.
    #[test]
    fn section_chips_and_the_live_pill_read_in_both_themes() {
        for dark in [true, false] {
            let kit = if dark { Kit::from_palette(dark_seed()) } else { light_kit() };
            let sf = surf_for(dark);
            assert!(contrast(Color::WHITE, live_for(dark)) >= 4.5, "white on live, dark {dark}");
            // Words, and the grey words under them, on every surface of the window.
            for (name, surface) in [("chrome", sf.chrome), ("side", sf.side), ("work", sf.work), ("raised", sf.raised), ("header", sf.header)] {
                for (ink, what) in [(kit.c.text_base, "text"), (kit.c.text_muted, "muted text")] {
                    let found = contrast(ink, surface);
                    assert!(found >= 4.5, "{what} on {name}, dark {dark}: {found:.2}");
                }
            }
            for surface in [kit.c.bg_base, kit.c.page, kit.c.layer_02, sf.side, sf.raised] {
                for group in 0..6 {
                    let color = artosia_slides::stage::group_color(group);
                    for strong in [false, true] {
                        let chip = over(tint_for(color, strong, dark), surface);
                        let found = contrast(ink_for(color, dark), chip);
                        assert!(found >= 4.5, "group {group}, strong {strong}, dark {dark}: {found:.2}");
                    }
                }
            }
        }
    }
}
