//! The controls that act on what the room sees: Back and Next, and the three toggles. The
//! shell places them under the Live list.

use rune::{El, col, row};
use rune_kit::Kit;

use crate::state::App;
use crate::strings as t;
use crate::theme::{icons, space};
use crate::ui::widgets::btn;

pub(crate) fn transport(s: &App, k: &Kit) -> El {
    let has_logo = !s.settings.logo.is_empty();
    col()
        .key("transport")
        .w_full()
        .gap(space::S2)
        .child(
            row()
                .w_full()
                .gap(space::S2)
                // The arrows point the way the list runs, and name the keys that do the same.
                .child(btn(t::BTN_BACK()).icon(&icons::ARROW_UP).tall().build(k, |s, tx| {
                    s.previous();
                    crate::follow(s, tx);
                    tx.blur();
                }))
                .child(btn(t::BTN_NEXT()).icon(&icons::ARROW_DOWN).primary().tall().grow().hint(t::KEY_NEXT()).build(k, |s, tx| {
                    s.next();
                    crate::follow(s, tx);
                    tx.blur();
                })),
        )
        .child(
            row()
                .w_full()
                .gap(space::S2)
                // Longer words take a second line instead of pushing Logo off the edge.
                .flex_wrap()
                .child(btn(t::BTN_CLEAR_TEXT()).toggle(s.text_hidden).grow().hint("F1").build(k, |s, tx| {
                    s.toggle_text();
                    tx.blur();
                }))
                .child(btn(t::BTN_BLACK()).toggle(s.black).grow().hint("F2").build(k, |s, tx| {
                    s.toggle_black();
                    tx.blur();
                }))
                .child(btn(t::BTN_LOGO()).toggle(s.logo_on).grow().hint("F3").disabled(!has_logo).help(if has_logo { t::BTN_LOGO() } else { t::LOGO_MISSING() }).build(
                    k,
                    |s, tx| {
                        s.toggle_logo();
                        tx.blur();
                    },
                )),
        )
}
