//! The tiles that act on what the room sees: Prev and Next, then the four states. The shell
//! places them beside the Live picture.
//!
//! Tiles of one row are one size, and the size comes from the strip, never from a word, so
//! every language draws the same strip. Next differs from Prev by its fill, not its size. A
//! state is a tile that is lit while it is on, named by what is then true (Black, Live), never
//! by a verb (owner, 2026-11-11; Apple's and Microsoft's button guidance in
//! `docs/ux-principles.md`).

use rune::{Color, El, col, row};
use rune_kit::Kit;

use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space};
use crate::ui::shell::Foot;
use crate::ui::widgets::tile;

/// `size` is the strip beside the picture; without one the tiles stack under the picture of
/// the thin pane beside an editor.
pub(crate) fn transport(s: &App, k: &Kit, size: Option<Foot>) -> El {
    let has_logo = !s.settings.logo.is_empty();
    let high = size.map(Foot::tile).unwrap_or(72.0);
    // Beside an editor the strip is a narrow column: four tiles share about 264 points.
    let narrow = size.is_none_or(|size| size.controls < 320.0);
    let frame = col().key("transport").gap(space::S2);
    let frame = match size {
        Some(size) => frame.w(size.controls),
        None => frame.w_full(),
    };
    let hides = |on: bool| (on, theme::warn(), Color::BLACK);
    let (black, text_off, logo) = (hides(s.black), hides(s.text_hidden), hides(s.logo_on));
    frame
        .child(
            row()
                .w_full()
                .gap(space::S2)
                // Each arrow is the key that does the same, drawn as that key.
                .child(tile(t::BTN_PREV()).arrow_key(&icons::ARROW_UP).height(high).help(t::TIP_PREV()).build(k, |s, tx| {
                    s.previous();
                    crate::follow(s, tx);
                    tx.blur();
                }))
                .child(tile(t::BTN_NEXT()).arrow_key(&icons::ARROW_DOWN).primary().keys("Space").height(high).help(t::TIP_NEXT()).build(k, |s, tx| {
                    s.next();
                    crate::follow(s, tx);
                    tx.blur();
                })),
        )
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(tile(t::BTN_NO_TEXT()).icon(&icons::NO_TEXT).on(text_off.0, text_off.1, text_off.2).keys("F1").height(high).narrow(narrow).help(t::BTN_CLEAR_TEXT()).build(k, |s, tx| {
                    s.toggle_text();
                    tx.blur();
                }))
                .child(tile(t::BTN_BLACK()).icon(&icons::BLACK_SCREEN).on(black.0, black.1, black.2).keys("F2").height(high).narrow(narrow).help(t::STATE_BLACK()).build(k, |s, tx| {
                    s.toggle_black();
                    tx.blur();
                }))
                .child(tile(t::BTN_LOGO()).icon(&icons::BADGE).on(logo.0, logo.1, logo.2).keys("F3").height(high).narrow(narrow).disabled(!has_logo).help(if has_logo { t::STATE_LOGO() } else { t::LOGO_MISSING() }).build(k, |s, tx| {
                    s.toggle_logo();
                    tx.blur();
                }))
                // Lit red while the room sees the Live list; off, the projector shows none of it.
                .child(tile(t::LIVE()).icon(if s.on_air() { &icons::ON_AIR } else { &icons::OFF_AIR }).on(s.on_air(), theme::live(), theme::on_live()).keys("F6").height(high).narrow(narrow).help(t::TIP_LIVE()).build(k, |s, tx| {
                    s.toggle_live();
                    tx.blur();
                })),
        )
}
