//! The Look dialog's text style: the colour of the words, their font and the line round them,
//! and the picture that shows the result while a control changes.

use artosia_slides::fonts;
use artosia_slides::look::{Look, Outline, hex_of, parse_hex, rgb};
use rune::{Color, Cursor, El, TextEdit, Tx, el, row};
use rune_kit::{Combobox, ComboboxOption, ComboboxState, Kit, SegmentedControl, TextInput};

use super::sheets::setting;
use super::widgets::btn;
use crate::state::App;
use crate::strings as t;
use crate::theme::{icons, space};

/// What the dialog's typed colours and its font list hold between frames.
pub struct LookEdit {
    pub ink: TextEdit,
    pub outline: TextEdit,
    pub font: ComboboxState,
}

impl LookEdit {
    pub fn new(look: &Look) -> LookEdit {
        LookEdit { ink: TextEdit::new(hex_of(look.ink)), outline: TextEdit::new(hex_of(look.outline_ink)), font: ComboboxState::default() }
    }

    /// Follows a look chosen somewhere else, such as a preset. A colour being typed that already
    /// reads as the look's colour is left alone, so `fc0` does not turn into `ffcc00` mid-edit.
    pub fn sync(&mut self, look: &Look) {
        if parse_hex(&self.ink.text) != Some(look.ink) {
            self.ink = TextEdit::new(hex_of(look.ink));
        }
        if parse_hex(&self.outline.text) != Some(look.outline_ink) {
            self.outline = TextEdit::new(hex_of(look.outline_ink));
        }
    }
}

/// The text colours offered as squares: the label, then the colour.
fn text_colours() -> [(&'static str, [u8; 3]); 4] {
    [(t::LOOK_COLOUR_WHITE(), [255, 255, 255]), (t::LOOK_COLOUR_WARM(), [255, 244, 222]), (t::LOOK_COLOUR_YELLOW(), [255, 214, 0]), (t::LOOK_COLOUR_BLACK(), [0, 0, 0])]
}

fn outline_colours() -> [(&'static str, [u8; 3]); 2] {
    [(t::LOOK_COLOUR_BLACK(), [0, 0, 0]), (t::LOOK_COLOUR_WHITE(), [255, 255, 255])]
}

/// A square of colour. The chosen one has a ring round it.
fn swatch(k: &Kit, key: String, name: &'static str, color: [u8; 3], on: bool, press: impl Fn(&mut App) + 'static) -> El {
    let ring = if on { k.c.text_base } else { Color::BLACK.alpha(0.0) };
    el().key(key)
        .role("button")
        .label(name)
        .size(36.0, 36.0)
        .center()
        .radius(k.radius.md + 2.0)
        .border(2.0, ring)
        .focusable(true).on_key(crate::ui::widgets::enter_key)
        .focus_selected(on)
        .cursor(Cursor::Pointer)
        .child(el().size(26.0, 26.0).radius(k.radius.md).fill(rgb(color)).border(1.0, k.c.border_strong).hover(Color::WHITE.alpha(0.12)))
        .on_press(press)
}

/// The typed colour: six hex digits, applied as soon as they read as a colour.
fn hex_field(k: &Kit, edit: &TextEdit, outline: bool) -> El {
    let field = TextInput::new(edit).width(None).label(t::LOOK_COLOUR_HEX()).placeholder(t::LOOK_COLOUR_HEX_HINT());
    let field = if outline {
        field.on_edit(|s: &mut App| &mut s.look_edit.outline).on_change(|s: &mut App, _: &mut Tx| {
            if let Some(color) = parse_hex(&s.look_edit.outline.text) {
                let mut look = s.look();
                look.outline_ink = color;
                s.set_look(look);
            }
        })
    } else {
        field.on_edit(|s: &mut App| &mut s.look_edit.ink).on_change(|s: &mut App, _: &mut Tx| {
            if let Some(color) = parse_hex(&s.look_edit.ink.text) {
                let mut look = s.look();
                look.ink = color;
                s.set_look(look);
            }
        })
    };
    el().w(120.0).child(field.build(k))
}

fn colour_row(k: &Kit, label: &'static str, current: [u8; 3], choices: &[(&'static str, [u8; 3])], edit: &TextEdit, outline: bool) -> El {
    // One choice of several: the arrows choose as they move, as in the switches of this dialog.
    let mut squares = row().role("toolbar").label(label).gap(space::S1).center_y().focus_group().focus_follows();
    for (name, color) in choices {
        let color = *color;
        let key = format!("look-{}-{}", if outline { "outline" } else { "ink" }, hex_of(color));
        squares = squares.child(swatch(k, key, name, color, current == color, move |s: &mut App| {
            let mut look = s.look();
            if outline {
                look.outline_ink = color;
            } else {
                look.ink = color;
            }
            s.set_look(look);
        }));
    }
    // A tester stopped on the first square with Tab and never tried the arrows.
    let squares = row().gap(space::S2).center_y().child(squares).child(crate::ui::widgets::keycaps(k, "Left Right", None));
    setting(k, label, row().grow(1.0).min_w(0.0).gap(space::S2).center_y().child(squares).child(rune::text("#").font(crate::theme::type_::BODY).color(k.c.text_muted).a11y_hidden()).child(hex_field(k, edit, outline)))
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
    vec![
        chooser(s, k),
        colour_row(k, t::LOOK_TEXT_COLOUR(), look.ink, &text_colours(), &s.look_edit.ink, false),
        font_row(s, k, layer),
        setting(
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
        ),
        colour_row(k, t::LOOK_OUTLINE_COLOUR(), look.outline_ink, &outline_colours(), &s.look_edit.outline, true),
    ]
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
