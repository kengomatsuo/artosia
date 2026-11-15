//! The dialog an imported PowerPoint opens (OVPPTX1, PPTXGALLERY1): one gallery of tiles, the file as
//! made first and then each template family that holds the redrawn cards, the proposals a redraw
//! found under it, and Import. Nothing is written until Import.

use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Progress, ScrollBar};

use super::calendar::tick;
use super::widgets::{btn, divider, enter_key, section_label, wrapped};
use crate::pptx_import::{Kind, Made, Pick, PptxImport};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

const WIDTH: f32 = 880.0;
const COLUMNS: usize = 4;
const GAP: f32 = space::S3;
const TILE_W: f32 = ((WIDTH - 2.0 * space::S4 - GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32).floor();
const TILE_H: f32 = (TILE_W * 9.0 / 16.0).round();

/// Ctrl Enter imports wherever the focus is in the dialog; the other keys are left to what has the focus.
fn import_key(s: &mut App, ev: &KeyEvent, _: &mut Tx) -> bool {
    if ev.key != Key::Enter || !(ev.mods.ctrl || ev.mods.cmd) {
        return false;
    }
    if s.pptx.as_ref().is_some_and(PptxImport::ready) {
        s.apply_pptx_import();
    }
    true
}

/// One tile of the gallery: its picture, ringed while picked, and its name under it. A press picks it,
/// and the arrows of the group move the pick as they move the focus.
fn tile(k: &Kit, key: String, pick: Pick, on: bool, usable: bool, picture: El, name: &str, sub: String) -> El {
    let mut frame = el().w(TILE_W).h(TILE_H).radius(k.radius.lg).child(picture);
    if on {
        frame = frame.child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(k.radius.lg).border(2.5, theme::accent()).a11y_hidden());
    }
    let tile = col()
        .key(key)
        .role("radio")
        .label(format!("{name}, {sub}"))
        .checked(on)
        .w(TILE_W)
        .gap(space::S1)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .focus_selected(on)
        .on_key(enter_key)
        .child(frame)
        .child(
            col()
                .w_full()
                .child(text(name.to_string()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().min_w(0.0).a11y_hidden())
                .child(text(sub).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0).a11y_hidden()),
        )
        .on_press(move |s: &mut App| s.pptx_pick(pick));
    // The dialog opens on the picked tile, the one tab stop of the gallery.
    let tile = if on { tile.autofocus() } else { tile };
    if usable { tile } else { tile.layer_opacity(0.55) }
}

/// The quiet frame of a picture that is not there yet: a plain field, with the count while it is drawn.
fn placeholder(k: &Kit, import: &PptxImport) -> El {
    let frame = el().w(TILE_W).h(TILE_H).radius(k.radius.lg).fill(k.c.layer_03).border(1.0, k.c.border_base).center().a11y_hidden();
    match import.made {
        Made::Drawing { done } => frame.child(text(t::pptx_progress(done, import.slides)).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0)),
        _ => frame,
    }
}

/// The file as made: its first picture once the worker has it.
fn made_tile(s: &App, k: &Kit, import: &PptxImport) -> El {
    let picture = match (&import.made, s.images.as_ref()) {
        (Made::Ready(files), Some(images)) if !files.is_empty() => el().w(TILE_W).h(TILE_H).radius(k.radius.lg).clip_rounded(k.radius.lg).border(1.0, k.c.border_base).pixels_fit(images.file(&files[0], TILE_W, 1.0).pixels(), rune::SurfaceFit::Fill),
        _ => placeholder(k, import),
    };
    let usable = matches!(import.made, Made::Ready(_) | Made::Drawing { .. });
    tile(k, "pptx-tile-made".into(), Pick::AsMade, import.pick == Pick::AsMade, usable, picture, t::PPTX_AS_MADE(), t::slides(import.slides))
}

/// A template family drawn with the first redrawn card in its first colourway.
fn family_tile(s: &App, k: &Kit, import: &PptxImport, style: artosia_core::deck::NoticeStyle) -> El {
    let first = import.cards.iter().find(|card| !card.title.trim().is_empty() || !card.body.trim().is_empty()).or(import.cards.first());
    let card = first.map(|card| artosia_core::deck::Announcement { style: Some(style), colourway: String::new(), image: String::new(), ..card.clone() }).unwrap_or_default();
    let picture = super::templates::slide_or_note(s, k, &card, TILE_W);
    tile(k, format!("pptx-tile-{}", style.code()), Pick::Redraw(style), import.pick == Pick::Redraw(style), true, picture, super::templates::style_name(style), t::PPTX_REDRAW().to_string())
}

/// What stands under the gallery while As made is not ready or cannot be: a bar and Cancel, or the reason.
fn made_note(k: &Kit, import: &PptxImport) -> Option<El> {
    match &import.made {
        Made::Drawing { done } => {
            let total = import.slides.max(1);
            Some(
                row()
                    .w_full()
                    .gap(space::S3)
                    .center_y()
                    .child(el().grow(1.0).min_w(0.0).child(Progress::new((*done > 0).then(|| (*done as f32 / total as f32).clamp(0.0, 1.0))).label(t::PPTX_DRAWING()).build(k)))
                    .child(btn(t::BTN_CANCEL()).icon(&icons::CLOSE).focusable().build(k, |s, _| s.stop_pptx_export()).key("pptx-stop")),
            )
        }
        Made::Ready(_) => None,
        Made::NoRenderer => Some(wrapped(t::PPTX_NO_RENDERER()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap()),
        Made::Failed => Some(wrapped(t::PPTX_FAILED()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap()),
        Made::Stopped => Some(wrapped(t::PPTX_STOPPED()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap()),
    }
}

fn proposals(k: &Kit, import: &PptxImport) -> El {
    let mut list = col().w_full().gap(space::S2).child(section_label(k, t::PPTX_FOUND()));
    for (at, proposal) in import.proposals.iter().enumerate() {
        let on = proposal.on;
        let words = match &proposal.kind {
            Kind::Members(_) => t::pptx_members(proposal.count()),
            Kind::Offerings { .. } => t::pptx_gifts(proposal.count()),
        };
        let mut block = col().w_full().gap(space::S1).child(
            row()
                .key(format!("pptx-tick-{at}"))
                .role("checkbox")
                .label(words.clone())
                .checked(on)
                .w_full()
                .h(28.0)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.md)
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .on_key(enter_key)
                .child(tick(k, on, theme::accent()))
                .child(icon(if matches!(proposal.kind, Kind::Members(_)) { &icons::PERSON } else { &icons::COIN }, 16.0).color(k.c.text_muted).a11y_hidden())
                .child(text(words).font(type_::BODY).color(k.c.text_base).truncate().min_w(0.0).a11y_hidden())
                .on_press(move |s: &mut App| s.pptx_tick(at)),
        );
        if let Kind::Offerings { money, .. } = &proposal.kind
            && money.total_matches() == Some(false)
            && let Some(printed) = money.printed_total
        {
            let said = t::pptx_total_differs(&artosia_core::giving::rupiah(printed), &artosia_core::giving::rupiah(money.sum()));
            block = block.child(wrapped(said).key(format!("pptx-differs-{at}")).font(type_::SMALL).color(theme::warn_ink()).wrap());
        }
        list = list.child(block);
    }
    list
}

pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(import) = s.pptx.as_ref() else { return };
    // The gallery is one tab stop: arrows rove among its tiles and the pick follows the focus.
    let mut gallery = row().key("pptx-ways").role("radiogroup").label(t::PPTX_PICK()).w_full().gap(GAP).flex_wrap().focus_group().focus_follows().child(made_tile(s, k, import));
    for &style in &import.families {
        gallery = gallery.child(family_tile(s, k, import, style));
    }
    let top = row().w_full().gap(space::S3).center_y().child(icon(&icons::ANNOUNCE, 16.0).color(k.c.text_muted).a11y_hidden()).child(text(import.file.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0).grow(1.0)).child(text(t::slides(import.slides)).font(type_::SMALL).color(k.c.text_muted));
    let mut body = col().key("pptx-form").w_full().px(space::S4).pb(space::S3).gap(space::S3).child(crate::ui::tour::help_row(k, crate::guide::lessons::Feature::Pptx)).child(top);
    // The bar and the reason come before the tiles, so Cancel is never below six rows of them.
    if let Some(note) = made_note(k, import) {
        body = body.child(note);
    }
    body = body.child(section_label(k, t::PPTX_PICK())).child(gallery);
    if import.families.is_empty() && !import.sorting {
        body = body.child(wrapped(t::PPTX_NO_FAMILY()).key("pptx-no-family").font(type_::SMALL).color(k.c.text_muted).wrap());
    }
    if matches!(import.pick, Pick::Redraw(_)) && !import.proposals.is_empty() {
        body = body.child(proposals(k, import));
    }
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let go = btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).primary().hint(t::KEY_IMPORT()).disabled(!import.ready()).focusable().build(k, |s, _| s.apply_pptx_import()).key("pptx-go");
    let foot = row().w_full().px(space::S4).py(space::S3).gap(space::S2).center_y().child(spacer()).child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_pptx_import()).key("pptx-cancel")).child(go);
    Dialog::new("pptx-import-dialog", t::CMD_IMPORT_PPTX()).open(true).width(WIDTH).fit().on_close(|s: &mut App| s.close_pptx_import()).child(col().w_full().on_key(import_key).child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}
