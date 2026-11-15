//! The Design tab's backgrounds that move (LIVEBG1): one moving tile per kind, and for the one
//! picked, its colours and its speed.

use artosia_slides::live::{COLOUR_SETS, Live, LiveKind, Pace};
use artosia_slides::look::{Background, Look, rgb};
use rune::{Cursor, El, col, el, row, text};
use rune_kit::{Kit, SegmentedControl};

use crate::state::App;
use crate::strings as t;
use crate::theme::{space, type_};

// Four to a row in the Design column, so the colours and the speed stay in view.
const TILE_W: f32 = 96.0;
const TILE_H: f32 = 54.0;
// Eight colour chips fit one row beside the label.
const SWATCH_W: f32 = 40.0;
const SWATCH_H: f32 = 26.0;

/// A kind's name, as the tiles and the background row say it.
pub fn name(kind: LiveKind) -> &'static str {
    match kind {
        LiveKind::Flow => t::LIVE_FLOW(),
        LiveKind::Aurora => t::LIVE_AURORA(),
        LiveKind::Ink => t::LIVE_INK(),
        LiveKind::Sea => t::LIVE_SEA(),
        LiveKind::Glass => t::LIVE_GLASS(),
        LiveKind::Lava => t::LIVE_LAVA(),
        LiveKind::Stars => t::LIVE_STARS(),
        LiveKind::Kaleidoscope => t::LIVE_KALEIDOSCOPE(),
    }
}

fn colours_name(id: &str) -> &'static str {
    match id {
        "dusk" => t::COLOURS_DUSK(),
        "ocean" => t::COLOURS_OCEAN(),
        "forest" => t::COLOURS_FOREST(),
        "ember" => t::COLOURS_EMBER(),
        "royal" => t::COLOURS_ROYAL(),
        "night" => t::COLOURS_NIGHT(),
        "glass" => t::COLOURS_GLASS(),
        _ => t::COLOURS_DAWN(),
    }
}

/// A label on the left and controls that wrap beside it, as tall as they need.
fn labelled(k: &Kit, label: &'static str, control: El) -> El {
    row().w_full().gap(space::S3).items_start().child(text(label).font(type_::BODY).color(k.c.text_base).w(150.0).pt(space::S1)).child(control.grow(1.0).min_w(0.0))
}

fn live_of(look: &Look) -> Option<&Live> {
    match &look.background {
        Background::Live(live) => Some(live),
        _ => None,
    }
}

/// The rows: the moving tiles, then the picked one's colours and speed.
pub fn rows(k: &Kit, look: &Look) -> Vec<El> {
    let current = live_of(look);
    let mut tiles = row().key("moving-tiles").gap(space::S2).flex_wrap();
    for kind in LiveKind::ALL {
        let on = current.is_some_and(|live| live.kind == kind);
        // Each tile moves in its kind's own colours, or the picked colours on the picked one.
        let live = match current {
            Some(live) if live.kind == kind => live.clone(),
            _ => Live::new(kind),
        };
        let (mark, data) = artosia_slides::live::slot(&live, true);
        let mut frame = el()
            .size(TILE_W, TILE_H)
            .radius(k.radius.md)
            .clip_rounded(k.radius.md)
            .gradient(rgb(live.colours[0]), rgb(live.colours[1]))
            .child(rune::effect(mark).effect_data(data).absolute(0.0, 0.0).size(TILE_W, TILE_H));
        if on {
            frame = frame.child(el().absolute(0.0, 0.0).size(TILE_W, TILE_H).radius(k.radius.md).border(2.0, k.c.text_base).a11y_hidden());
        }
        tiles = tiles.child(
            col()
                .key(format!("moving-{kind:?}").to_lowercase())
                .role("button")
                .label(name(kind))
                .w(TILE_W)
                .gap(space::S1)
                .focusable(true)
                .on_key(crate::ui::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .child(frame.hover(rune::Color::WHITE.alpha(0.06)))
                .child(text(name(kind)).font(if on { type_::SMALL_STRONG } else { type_::SMALL }).color(if on { k.c.text_base } else { k.c.text_muted }).truncate())
                .on_press(move |s: &mut App| {
                    let mut look = s.look();
                    look.background = Background::Live(Live::new(kind));
                    s.set_look(look);
                }),
        );
    }
    let mut rows = vec![labelled(k, t::LOOK_MOVING(), tiles)];
    let Some(live) = current else { return rows };
    let mut swatches = row().key("moving-colours").gap(space::S2).flex_wrap();
    for (id, colours) in COLOUR_SETS {
        let on = live.colours == colours;
        let mut chip = row().size(SWATCH_W, SWATCH_H).radius(SWATCH_H / 2.0).clip_rounded(SWATCH_H / 2.0);
        for c in colours {
            chip = chip.child(el().grow(1.0).h_full().fill(rgb(c)));
        }
        let ring = if on { el().absolute(0.0, 0.0).size(SWATCH_W, SWATCH_H).radius(SWATCH_H / 2.0).border(2.0, k.c.text_base).a11y_hidden() } else { el() };
        swatches = swatches.child(
            el().key(format!("moving-colours-{id}"))
                .role("button")
                .label(colours_name(id))
                .help(colours_name(id))
                .focusable(true)
                .on_key(crate::ui::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .child(chip)
                .child(ring)
                .on_press(move |s: &mut App| {
                    let mut look = s.look();
                    if let Background::Live(live) = &mut look.background {
                        live.colours = colours;
                    }
                    s.set_look(look);
                }),
        );
    }
    rows.push(labelled(k, t::LOOK_MOVING_COLOURS(), swatches));
    let at = Pace::ALL.iter().position(|pace| *pace == live.pace);
    let pace = SegmentedControl::new(&[t::PACE_SLOW(), t::PACE_NORMAL(), t::PACE_FAST()], at)
        .label(t::LOOK_MOVING_PACE())
        .fit()
        .on_pick(|s: &mut App, picked, _: &mut rune::Tx| {
            let mut look = s.look();
            if let Background::Live(live) = &mut look.background {
                live.pace = Pace::ALL[picked.min(Pace::ALL.len() - 1)];
            }
            s.set_look(look);
        })
        .build(k)
        .key("moving-pace");
    rows.push(labelled(k, t::LOOK_MOVING_PACE(), pace));
    rows
}
