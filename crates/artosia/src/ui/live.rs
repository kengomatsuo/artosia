//! The controls that act on what the room sees: Back and Next, the three toggles, and End.
//! The shell places them beside the Live picture, as tall as the picture.

use rune::{El, col, row};
use rune_kit::Kit;

use crate::state::App;
use crate::strings as t;
use crate::theme::{icons, space};
use crate::ui::shell::Foot;
use crate::ui::widgets::btn;

/// `size` is the strip beside the picture; without one the controls stack under the picture
/// of the thin pane beside an editor.
pub(crate) fn transport(s: &App, k: &Kit, size: Option<Foot>) -> El {
    let has_logo = !s.settings.logo.is_empty();
    // Three rows share the picture's height: Next gets half, the toggles a quarter each.
    let (tall, short, width, hints) = match size {
        Some(size) => {
            let rows = size.height - 2.0 * space::S2;
            let tall = (rows * 0.5).max(44.0).round();
            (tall, ((rows - tall) / 2.0).max(26.0).floor(), Some(size.controls), size.roomy())
        }
        None => (44.0, 32.0, None, false),
    };
    // Every button prints its key (principle: the key is on the control, not in a legend).
    let _ = hints;
    let hint = |button: crate::ui::widgets::Btn, keys: &str| button.hint(keys);
    let frame = col().key("transport").gap(space::S2);
    let frame = match (width, size) {
        (Some(width), Some(size)) => frame.w(width).h(size.height),
        _ => frame.w_full(),
    };
    frame
        .child(
            row()
                .w_full()
                .gap(space::S2)
                // The arrows point the way the list runs, and name the keys that do the same.
                .child(btn(t::BTN_BACK()).icon(&icons::ARROW_UP).height(tall).hint(t::KEY_BACK()).build(k, |s, tx| {
                    s.previous();
                    crate::follow(s, tx);
                    tx.blur();
                }))
                .child(btn(t::BTN_NEXT()).icon(&icons::ARROW_DOWN).primary().height(tall).grow().hint(t::KEY_NEXT()).build(k, |s, tx| {
                    s.next();
                    crate::follow(s, tx);
                    tx.blur();
                })),
        )
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(hint(btn(t::BTN_CLEAR_TEXT()).toggle(s.text_hidden).height(short).grow(), "F1").build(k, |s, tx| {
                    s.toggle_text();
                    tx.blur();
                }))
                .child(hint(btn(t::BTN_BLACK()).toggle(s.black).height(short).grow(), "F2").build(k, |s, tx| {
                    s.toggle_black();
                    tx.blur();
                })),
        )
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(hint(btn(t::BTN_LOGO()).toggle(s.logo_on).height(short).grow(), "F3").disabled(!has_logo).help(if has_logo { t::BTN_LOGO() } else { t::LOGO_MISSING() }).build(
                    k,
                    |s, tx| {
                        s.toggle_logo();
                        tx.blur();
                    },
                ))
                // End takes the live item off the projector; the toast puts it back.
                .child(hint(btn(t::BTN_END()).height(short).grow(), "F6").disabled(s.live.is_none()).build(k, |s, tx| {
                    s.end_live();
                    tx.blur();
                })),
        )
}
