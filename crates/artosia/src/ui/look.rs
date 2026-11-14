//! The Look dialog's text style: the colour of the words, their font and the line round them,
//! and the picture that shows the result while a control changes.

use artosia_slides::fonts;
use artosia_slides::look::{Look, Outline};
use rune::{El, Tx, el, row};
use rune_kit::{Combobox, ComboboxOption, ComboboxState, Kit, SegmentedControl};

use super::colour::ColourFor;

use super::sheets::setting;
use super::widgets::btn;
use crate::state::App;
use crate::strings as t;
use crate::theme::{icons, space};

/// What the font list holds between frames.
pub struct LookEdit {
    pub font: ComboboxState,
}

impl LookEdit {
    pub fn new(_look: &Look) -> LookEdit {
        LookEdit { font: ComboboxState::default() }
    }
}

/// The text colours offered as squares: the label, then the colour.
pub(super) fn text_colours() -> [(&'static str, [u8; 3]); 4] {
    [(t::LOOK_COLOUR_WHITE(), [255, 255, 255]), (t::LOOK_COLOUR_WARM(), [255, 244, 222]), (t::LOOK_COLOUR_YELLOW(), [255, 214, 0]), (t::LOOK_COLOUR_BLACK(), [0, 0, 0])]
}

fn outline_colours() -> [(&'static str, [u8; 3]); 2] {
    [(t::LOOK_COLOUR_BLACK(), [0, 0, 0]), (t::LOOK_COLOUR_WHITE(), [255, 255, 255])]
}

/// The font picker. The list of installed families is asked for only once it is opened, because
/// reading the system's fonts takes a moment and some memory.
fn font_row(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let current = s.look().font.clone();
    let default = ComboboxOption::new("", t::LOOK_FONT_DEFAULT());
    let mut options = vec![default];
    if s.look_edit.font.is_open() {
        options.extend(fonts::installed().iter().map(|name| ComboboxOption::new(name.clone(), name.clone())));
    } else if !current.is_empty() {
        options.push(ComboboxOption::new(current.clone(), current.clone()));
    }
    let picker = Combobox::new("look-font", &s.look_edit.font, &options, |s: &mut App| &mut s.look_edit.font)
        .label(t::LOOK_FONT())
        .current(Some(current.as_str()))
        .search_placeholder(t::LOOK_FONT_SEARCH())
        .empty_message(t::LOOK_FONT_NONE())
        .width(280.0)
        .max_height(240.0)
        .on_choose(|s: &mut App, name: &str, _: &mut Tx| {
            let mut look = s.look();
            look.font = name.to_string();
            s.set_look(look);
        })
        .build(k, layer);
    setting(k, t::LOOK_FONT(), el().grow(1.0).min_w(0.0).child(picker))
}

/// The rows that style the words: colour, font, outline and the outline's colour.
pub fn rows(s: &App, k: &Kit, layer: &mut Vec<El>) -> Vec<El> {
    let look = s.look();
    let at = Outline::ALL.iter().position(|outline| *outline == look.outline);
    let mut out = vec![chooser(s, k)];
    out.extend(super::colour::rows(s, k, ColourFor::Ink, t::LOOK_TEXT_COLOUR(), &text_colours()));
    out.push(font_row(s, k, layer));
    out.push(setting(
        k,
        t::LOOK_OUTLINE(),
        SegmentedControl::new(&[t::LOOK_OUTLINE_OFF(), t::LOOK_OUTLINE_THIN(), t::LOOK_OUTLINE_MEDIUM(), t::LOOK_OUTLINE_THICK()], at)
            .label(t::LOOK_OUTLINE())
            .fit()
            .on_pick(|s: &mut App, picked, _: &mut Tx| {
                let mut look = s.look();
                look.outline = Outline::ALL[picked.min(Outline::ALL.len() - 1)];
                s.set_look(look);
            })
            .build(k),
    ));
    out.extend(super::colour::rows(s, k, ColourFor::Outline, t::LOOK_OUTLINE_COLOUR(), &outline_colours()));
    out
}

/// The look's sample slide, `width` points wide, kept in view beside the settings while a control
/// changes.
pub fn stage(s: &App, k: &Kit, width: f32) -> El {
    let height = (width * 9.0 / 16.0).round();
    let state = s.preview_state(Some(super::sample_slide()));
    el().key("look-preview").size(width, height).radius(k.radius.md).clip_rounded(k.radius.md).child(super::thumb(s, &state, width))
}

/// What the dialog edits: the usual look, or the open plan's own. Two buttons of equal width
/// whatever the language, the chosen one filled; the plan's own carries the plan's name and the
/// same mark the plan list uses. Choosing it copies the usual look as a start; going back to the
/// usual look drops it, with Undo in a toast.
fn chooser(s: &App, k: &Kit) -> El {
    let own = s.has_own_look();
    // Design opens with the focus on the look being changed, never on its close button, which
    // Rune would pick as the first Tab stop (house rule: a dialog opens on what the person
    // came for). Back from the gallery the focus is the gallery's button instead.
    let lead = !s.backdrops.returned;
    let first = move |button: El, chosen: bool| if lead && chosen { button.autofocus() } else { button };
    // The chosen one carries a tick as well as its fill: a tester could not tell which was on.
    let usual = btn(t::LOOK_FOR_USUAL()).icon(if own { &icons::SCREEN } else { &icons::CHECK }).focusable().grow();
    let usual = if own { usual } else { usual.primary() };
    let plan = btn(t::LOOK_FOR_PLAN()).icon(if own { &icons::CHECK } else { &icons::LOOK }).help(s.plan_title()).focusable().grow();
    let plan = if own { plan.primary() } else { plan };
    row()
        .key("look-for")
        .w_full()
        .gap(space::S2)
        .child(el().w_full().max_w(280.0).grow(1.0).child(first(usual.build(k, |s: &mut App, _: &mut Tx| s.use_usual_look()).key("look-for-usual"), !own)))
        .child(el().w_full().max_w(280.0).grow(1.0).child(first(plan.build(k, |s: &mut App, _: &mut Tx| s.use_own_look()).key("look-for-plan"), own)))
}
