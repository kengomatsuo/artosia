//! The strip that offers what to show next: up to three rows at the top of the Preview pane,
//! each with why it is offered, its title and section, and one button that brings it into
//! Preview. Never onto the projector: Preview is the safe side, and a misheard word must not
//! reach the room. The model is `suggest.rs`; the shell places this strip.

use artosia_core::song::Tag;
use artosia_slides::stage::group_color;
use rune::{El, col, el, icon, row, text};
use rune_kit::Kit;

use crate::strings as t;
use crate::suggest::{Suggestion, Why, suggestions};
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::{btn, tag_chip};

/// Height of one row: a reason line over a title, in either language.
pub const ROW: f32 = 48.0;

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

fn section_chip(tag: Tag) -> El {
    tag_chip(&tag.short(), group_color(artosia_core::deck::tag_group(tag)))
}

fn line(k: &Kit, at: usize, offer: Suggestion) -> El {
    let base = row().key(format!("suggest-{at}")).w_full().h(ROW).px(space::S3).gap(space::S2).center_y();
    if offer.why == Why::Lost {
        // A warning, not an offer: yellow, with the one thing a person can do about it.
        let ink = theme::warn_ink();
        let press = btn(t::SUGGEST_SEARCH()).small().hint(t::KEY_SEARCH()).build(k, |s, tx| {
            s.take_suggestion(&Suggestion { why: Why::Lost, target: crate::suggest::Target::Search, title: String::new(), tag: None });
            tx.blur();
        });
        return base
            .fill(theme::warn().alpha(if theme::is_dark() { 0.14 } else { 0.3 }))
            .child(icon(&icons::MIC, 16.0).color(ink).a11y_hidden())
            .child(text(reason(offer.why)).font(type_::BODY_STRONG).color(k.c.text_base).wrap().max_lines(2).grow(1.0).min_w(0.0))
            .child(press);
    }
    let icon_of = match offer.target {
        crate::suggest::Target::Passage(_) => &icons::BOOK,
        _ => &icons::SONG,
    };
    let words = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(2.0)
        .child(text(reason(offer.why)).font(type_::TINY).color(k.c.text_muted).truncate())
        .child(text(offer.title.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate());
    let mut out = base.child(icon(icon_of, 16.0).color(k.c.text_muted).a11y_hidden()).child(words);
    if let Some(tag) = offer.tag {
        out = out.child(section_chip(tag));
    }
    let mut press = btn(t::BTN_OPEN()).small();
    if at == 0 {
        press = press.hint("F4");
    }
    out.child(press.build(k, move |s, tx| {
        s.take_suggestion(&offer);
        crate::follow(s, tx);
        tx.blur();
    }))
}
