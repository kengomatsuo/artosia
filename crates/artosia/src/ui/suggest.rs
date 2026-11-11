//! The strip that offers what to show next: up to three rows at the top of the Preview pane.
//! Signs, not sentences (owner, 2026-11-11): an icon for why it is offered (an ear for what
//! was heard, a clock for what usually follows), the title, the section as the chip the lists
//! use, and one icon button of a fixed size that brings it into Preview with its key printed
//! on it (F4 on the first). The reason in words is the tooltip. Nothing here ever reaches the
//! projector: Preview is the safe side, and a misheard word must not reach the room.
//! The model is `suggest.rs`; the shell places this strip.

use artosia_core::song::Tag;
use rune::{Cursor, El, Icon, col, el, icon, row, text};
use rune_kit::Kit;

use crate::strings as t;
use crate::suggest::{Suggestion, Target, Why, suggestions};
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::keycap;

/// Height of one row.
pub const ROW: f32 = 48.0;
/// Width of the button on every row, the same whatever the language: an icon, and a key cap
/// on the first.
const BUTTON: f32 = 116.0;

/// The strip, or nothing when there is nothing to offer, so the pane keeps all its room.
pub(crate) fn strip(s: &crate::state::App, k: &Kit) -> Option<El> {
    let offers = suggestions(s);
    if offers.is_empty() {
        return None;
    }
    let mut rows = col().key("suggest").w_full().fill(theme::surf().side);
    for (at, offer) in offers.into_iter().enumerate() {
        rows = rows.child(line(k, at, offer)).child(el().h(1.0).w_full().fill(theme::surf().edge).a11y_hidden());
    }
    Some(rows)
}

fn reason(why: Why) -> &'static str {
    match why {
        Why::Heard => t::SUGGEST_HEARD(),
        Why::Here => t::SUGGEST_HERE(),
        Why::Follows => t::SUGGEST_FOLLOWS(),
        Why::Lost => t::SUGGEST_LOST(),
    }
}

fn sign(why: Why) -> &'static Icon {
    match why {
        Why::Heard | Why::Here => &icons::EAR,
        Why::Follows => &icons::HISTORY,
        Why::Lost => &icons::MIC,
    }
}

fn section_chip(tag: Tag) -> El {
    crate::ui::widgets::section_chip(tag)
}

/// The one button of a row: an icon, and its key on the first row. It never takes focus, so
/// Space still means Next.
fn button(k: &Kit, key: &str, glyph: &'static Icon, tip: &str, first: bool, press: impl Fn(&mut crate::state::App, &mut rune::Tx) + 'static) -> El {
    // An icon and its word, as every button here has; the key rides on the first row only.
    let mut body = row().key(key.to_string()).role("button").label(tip.to_string()).w(BUTTON).h(28.0).gap(6.0).center().radius(k.radius.md).fill(k.c.layer_02).border(1.0, k.c.border_base);
    body = body.child(icon(glyph, 16.0).color(k.c.text_base).a11y_hidden()).child(text(tip.to_string()).font(type_::SMALL_STRONG).color(k.c.text_base).truncate().a11y_hidden());
    if first {
        body = body.child(keycap(k, "F4", false));
    }
    body.hover(k.c.hover).press(k.c.pressed).cursor(Cursor::Pointer).on_press_tx(move |s: &mut crate::state::App, tx: &mut rune::Tx| press(s, tx))
}

fn line(k: &Kit, at: usize, offer: Suggestion) -> El {
    let base = row().key(format!("suggest-{at}")).w_full().h(ROW).px(space::S3).gap(space::S2).center_y();
    let why = offer.why;
    let words = reason(why);
    if why == Why::Lost {
        // A warning, not an offer: a sign cannot say it, so the words stay, in yellow.
        let press = button(k, "suggest-search", &rune_kit::glyphs::SEARCH, t::SUGGEST_SEARCH(), at == 0, |s, tx| {
            s.take_offer(&Suggestion { why: Why::Lost, target: Target::Search, title: String::new(), tag: None });
            tx.blur();
        });
        return base
            .fill(theme::warn().alpha(if theme::is_dark() { 0.14 } else { 0.3 }))
            .child(icon(sign(why), 16.0).color(theme::warn_ink()).a11y_hidden())
            .child(text(words).font(type_::BODY_STRONG).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
            .child(press);
    }
    let kind = match offer.target {
        Target::Passage(_) => &icons::BOOK,
        _ => &icons::SONG,
    };
    // The reason stands on the row in words, after the title: an ear or a clock alone told a
    // reader nothing, and a tooltip is no answer for someone who never hovers (2026-11-11).
    let why_sign = el().w(20.0).h(ROW).center().role("img").label(words).child(icon(sign(why), 16.0).color(k.c.text_muted).a11y_hidden());
    let mut out = base
        .child(why_sign)
        .child(icon(kind, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .child(text(offer.title.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate())
                .child(text(words).font(type_::SMALL).color(k.c.text_muted).truncate()),
        );
    if let Some(tag) = offer.tag {
        out = out.child(section_chip(tag));
    }
    out.child(button(k, &format!("suggest-open-{at}"), &icons::ARROW_RIGHT, t::BTN_OPEN(), at == 0, move |s, tx| {
        s.take_offer(&offer);
        crate::follow(s, tx);
        tx.blur();
    }))
}
