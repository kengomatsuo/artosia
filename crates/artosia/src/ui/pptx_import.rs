//! The dialog an imported PowerPoint opens (OVPPTX1): the two ways side by side with small pictures
//! of what each would make, the proposals a redraw found, and Import. Nothing is written until Import.

use rune::{Cursor, El, ScrollSpec, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Progress, ScrollBar};

use super::calendar::tick;
use super::widgets::{btn, divider, enter_key, section_label, wrapped};
use crate::pptx_import::{Kind, Made, PptxImport, Way};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

const WIDTH: f32 = 760.0;
/// A panel's inside: the dialog less its edges and the gap, halved, less the panel's own padding.
const PANEL: f32 = (WIDTH - 2.0 * space::S4 - space::S3) / 2.0;
const INSIDE: f32 = PANEL - 2.0 * space::S3 - 2.0;
const COLUMNS: usize = 3;
const GAP: f32 = space::S2;
const TILE_W: f32 = ((INSIDE - GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32).floor();
const TILE_H: f32 = (TILE_W * 9.0 / 16.0).round();
/// Two rows of tiles; the last tile says how many more there are.
const TILES: usize = 2 * COLUMNS;

fn grid(tiles: Vec<El>) -> El {
    let mut rows = col().w_full().gap(GAP);
    let mut tiles = tiles.into_iter().peekable();
    while tiles.peek().is_some() {
        rows = rows.child(row().w_full().gap(GAP).children(tiles.by_ref().take(COLUMNS)));
    }
    rows
}

/// The label of the "and so many more" tile.
fn more(count: usize) -> El {
    el().w(TILE_W).h(TILE_H).center().radius(4.0).fill(theme::tint_of(theme::accent(), false)).child(text(format!("+{count}")).font(type_::SMALL_STRONG).color(theme::accent()))
}

fn made_tiles(s: &App, k: &Kit, files: &[std::path::PathBuf]) -> El {
    let shown = if files.len() > TILES { TILES - 1 } else { files.len() };
    let mut tiles: Vec<El> = files
        .iter()
        .take(shown)
        .enumerate()
        .map(|(at, file)| {
            let tile = el().key(format!("pptx-made-{at}")).w(TILE_W).h(TILE_H).radius(4.0).clip_rounded(4.0).fill(k.c.layer_04).border(1.0, k.c.border_base).a11y_hidden();
            match s.images.as_ref() {
                Some(images) => tile.pixels_fit(images.file(file, TILE_W, 1.0).pixels(), rune::SurfaceFit::Fill),
                None => tile,
            }
        })
        .collect();
    if files.len() > shown {
        tiles.push(more(files.len() - shown));
    }
    grid(tiles)
}

fn card_tiles(k: &Kit, import: &PptxImport) -> El {
    let shown = if import.cards.len() > TILES { TILES - 1 } else { import.cards.len() };
    let mut tiles: Vec<El> = import
        .cards
        .iter()
        .take(shown)
        .enumerate()
        .map(|(at, (title, first))| {
            let head = if title.trim().is_empty() { first.as_str() } else { title.as_str() };
            let body = if title.trim().is_empty() { "" } else { first.as_str() };
            el().key(format!("pptx-card-{at}"))
                .w(TILE_W)
                .h(TILE_H)
                .radius(4.0)
                .clip_rounded(4.0)
                .fill(k.c.layer_03)
                .padding(space::S1)
                .child(col().w_full().gap(1.0).child(text(head).font(type_::TINY).color(k.c.text_base).truncate().min_w(0.0)).child(text(body).font(type_::TINY).color(k.c.text_muted).truncate().min_w(0.0)))
                .a11y_hidden()
        })
        .collect();
    if import.cards.len() > shown {
        tiles.push(more(import.cards.len() - shown));
    }
    grid(tiles)
}

/// What the As made side holds: a bar while drawing, the pictures, or the reason there are none.
fn made_body(s: &App, k: &Kit, import: &PptxImport) -> El {
    match &import.made {
        Made::Drawing { done } => {
            let total = import.slides.max(1);
            col()
                .w_full()
                .gap(space::S2)
                .child(row().w_full().gap(space::S3).center_y().child(el().grow(1.0).min_w(0.0).child(Progress::new((*done > 0).then(|| (*done as f32 / total as f32).clamp(0.0, 1.0))).label(t::PPTX_DRAWING()).build(k))).child(btn(t::BTN_CANCEL()).icon(&icons::CLOSE).focusable().build(k, |s, _| s.stop_pptx_export()).key("pptx-stop")))
                .child(text(t::pptx_progress(*done, import.slides)).key("pptx-progress").font(type_::SMALL).color(k.c.text_muted))
        }
        Made::Ready(files) => made_tiles(s, k, files),
        Made::NoRenderer => wrapped(t::PPTX_NO_RENDERER()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap(),
        Made::Failed => wrapped(t::PPTX_FAILED()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap(),
        Made::Stopped => wrapped(t::PPTX_STOPPED()).key("pptx-unavailable").font(type_::SMALL).color(k.c.text_muted).wrap(),
    }
}

/// Enter on a way is the dialog's default button, as in a Windows dialog: it imports once it can; Space chooses the way.
fn way_key(s: &mut App, ev: &rune::KeyEvent, _: &mut rune::Tx) -> bool {
    if ev.key != rune::Key::Enter || ev.mods != rune::Mods::default() {
        return false;
    }
    if s.pptx.as_ref().is_some_and(|import| import.ready()) {
        s.apply_pptx_import();
    }
    true
}

/// A way's heading: its radio, its name and what it makes. A press on it chooses the way.
fn heading(k: &Kit, key: &'static str, way: Way, chosen: bool, usable: bool, name: &'static str, hint: &'static str) -> El {
    let dot = el().size(16.0, 16.0).radius(8.0).border(1.0, if chosen { theme::accent() } else { k.c.border_strong }).center().child(if chosen { el().size(8.0, 8.0).radius(4.0).fill(theme::accent()) } else { el() });
    let head = row()
        .key(key)
        .role("radio")
        .label(name)
        .checked(chosen)
        .w_full()
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(way_key)
        .child(dot)
        .child(col().grow(1.0).min_w(0.0).child(text(name).font(type_::TITLE).color(k.c.text_base)).child(wrapped(hint).font(type_::SMALL).color(k.c.text_muted).wrap()))
        .on_press(move |s: &mut App| s.pptx_way(way));
    if usable { head.hover(k.c.hover) } else { head.layer_opacity(0.55) }
}

fn panel(k: &Kit, chosen: bool, head: El, body: El) -> El {
    col().w(PANEL).gap(space::S3).padding(space::S3).radius(k.radius.lg).border(if chosen { 2.0 } else { 1.0 }, if chosen { theme::accent() } else { k.c.border_base }).fill(theme::surf().work).child(head).child(body)
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
    let made_usable = matches!(import.made, Made::Ready(_) | Made::Drawing { .. });
    // While nothing can be pressed yet the dialog opens on the way that is chosen.
    let made_head = heading(k, "pptx-way-made", Way::AsMade, import.way == Way::AsMade, made_usable, t::PPTX_AS_MADE(), t::PPTX_AS_MADE_HINT());
    let as_made = panel(k, import.way == Way::AsMade, if import.ready() { made_head } else { made_head.autofocus() }, made_body(s, k, import));
    let mut redraw_body = col().w_full().gap(space::S3).child(card_tiles(k, import));
    if !import.proposals.is_empty() {
        redraw_body = redraw_body.child(proposals(k, import));
    }
    let redraw = panel(
        k,
        import.way == Way::Redraw,
        heading(k, "pptx-way-redraw", Way::Redraw, import.way == Way::Redraw, true, t::PPTX_REDRAW(), t::PPTX_REDRAW_HINT()),
        if import.way == Way::Redraw { redraw_body } else { redraw_body.layer_opacity(0.6) },
    );
    let top = row().w_full().gap(space::S3).center_y().child(icon(&icons::ANNOUNCE, 16.0).color(k.c.text_muted).a11y_hidden()).child(text(import.file.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0).grow(1.0)).child(text(t::slides(import.slides)).font(type_::SMALL).color(k.c.text_muted));
    let body = col().key("pptx-form").w_full().px(space::S4).pb(space::S3).gap(space::S3).child(top).child(row().w_full().gap(space::S3).child(as_made).child(redraw));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let go = btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).primary().disabled(!import.ready()).focusable().build(k, |s, _| s.apply_pptx_import()).key("pptx-go");
    let foot = row().w_full().px(space::S4).py(space::S3).gap(space::S2).center_y().child(spacer()).child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_pptx_import()).key("pptx-cancel")).child(if import.ready() { go.autofocus() } else { go });
    Dialog::new("pptx-import-dialog", t::CMD_IMPORT_PPTX()).open(true).width(WIDTH).fit().on_close(|s: &mut App| s.close_pptx_import()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}
