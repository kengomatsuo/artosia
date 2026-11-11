//! The window's arrangement: the plan on the left, then Preview and Live side by side, each a
//! numbered list of what it holds over its own picture. Read left to right: what is planned,
//! what is next, what is now.
//!
//! The owner picked this on 2026-11-10 from four trial arrangements, after OpenLP and
//! EasiSlides (`real-products-2/01-openlp-main-window.png` and
//! `easislides/01-easislides-main-window.jpg` in the research folder): it is the shape the
//! volunteers already know, and the words are readable in a list where a thumbnail is not.
//!
//! Preview is the safe side. A click there moves the ring and nothing else; only Show, Enter
//! and the Live pane put something on the projector (owner, 2026-11-11).

use artosia_core::deck::{Deck, DeckSource, Slide, SlideKind, tag_group};
use artosia_slides::audience::clock_text;
use artosia_slides::stage::group_color;
use rune::{Color, Cursor, Cx, El, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar};

use crate::state::{App, section_starts};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::live;
use crate::ui::widgets::{btn, keycap, state_pill, tag_chip};

/// Height of a row in the two lists: room for four lines of words.
const ROW: f32 = 76.0;
const PLAN: f32 = 256.0;
/// The least width the controls beside a picture need: two buttons side by side with the
/// longer Indonesian words.
const CONTROLS: f32 = 290.0;
/// Width of the Live pane while an editor has the rest of the window.
const NARROW: f32 = 288.0;

/// The strip under each list: a picture as large as the pane allows, and the controls in what
/// is left beside it, as tall as the picture (owner, 2026-11-11: the buttons took the width and
/// left the picture small).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Foot {
    pub picture: f32,
    pub height: f32,
    pub controls: f32,
}

impl Foot {
    /// For a pane `pane` points wide in a work area `tall` points high. Both panes get the
    /// same strip, so they end on one line.
    pub(crate) fn fit(pane: f32, tall: f32) -> Self {
        let inner = pane - 2.0 * space::S3;
        // The lists keep most of the height; the picture takes up to about a third.
        let highest = (tall * 0.36).clamp(128.0, 380.0);
        let picture = (inner - space::S3 - CONTROLS).min(highest * 16.0 / 9.0).max(220.0).round();
        Foot { picture, height: (picture * 9.0 / 16.0).round(), controls: (inner - space::S3 - picture).max(200.0) }
    }

    /// Key caps fit on the smaller buttons.
    pub(crate) fn roomy(self) -> bool {
        self.controls >= 400.0
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Rest,
    /// The selector: the slide Enter shows.
    Cursor,
    Live,
    /// The live slide while black, the logo or Clear text keeps it off the projector.
    Hidden,
}

fn hline() -> El {
    el().h(1.0).w_full().fill(theme::surf().edge).a11y_hidden()
}

fn vline() -> El {
    el().w(1.0).h_full().fill(theme::surf().edge).a11y_hidden()
}

/// The strip over a panel: its name, the keys that move in it, what it holds, and whatever
/// acts on it at the right.
fn title_bar(k: &Kit, name: &str, keys: Option<&str>, detail: String, trailing: Vec<El>) -> El {
    let mut bar = row().w_full().h(40.0).px(space::S3).gap(space::S2).center_y().fill(theme::surf().header).child(text(name.to_string()).font(type_::BODY_STRONG).color(k.c.text_base));
    if let Some(keys) = keys {
        bar = bar.child(keycap(k, keys, false));
    }
    bar.child(text(detail).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0)).child(spacer()).children(trailing)
}

/// One row of a list: the number key that jumps to it, its section, and its words.
fn list_row(k: &Kit, key: &str, index: usize, number: Option<usize>, slide: &Slide, mark: Mark, go_live: bool) -> El {
    let live = matches!(mark, Mark::Live | Mark::Hidden);
    let on_fill = if mark == Mark::Hidden { Color::BLACK } else { theme::on_live() };
    let ink = if live { on_fill } else { k.c.text_base };
    let faint = if live { on_fill } else { k.c.text_muted };
    let mut words = col().grow(1.0).min_w(0.0);
    match slide.kind {
        SlideKind::Lyrics => {
            for line in slide.text.lines().filter(|line| !line.trim().is_empty()).take(4) {
                words = words.child(text(line.to_string()).font(type_::SMALL).color(ink).truncate());
            }
        }
        // A verse is one long sentence: it wraps where a lyric line is cut.
        SlideKind::Scripture => words = words.child(text(slide.text.clone()).font(type_::SMALL).color(ink).wrap().max_lines(4)),
        SlideKind::Announcement => {
            words = words.child(text(slide.heading.clone()).font(type_::SMALL_STRONG).color(ink).truncate());
            for line in slide.text.lines().filter(|line| !line.trim().is_empty()).take(3) {
                // A table row reads with a gap where its tab was.
                words = words.child(text(line.replace('\t', "   ")).font(type_::SMALL).color(ink).truncate());
            }
        }
    }
    let chip = match (slide.tag, slide.kind) {
        (Some(_), _) if live => state_pill(&slide.short, Color::WHITE, Color::BLACK),
        (Some(tag), _) => tag_chip(&slide.short, group_color(tag_group(tag))),
        (None, SlideKind::Scripture) => text(slide.short.clone()).font(type_::SMALL_STRONG).color(faint),
        _ => el(),
    };
    // The number shown is the key that jumps here; a section's later slides carry none.
    let shown = number.filter(|n| *n < 10).map(|n| ((n + 1) % 10).to_string()).unwrap_or_default();
    let mut body = row()
        .w_full()
        .h(ROW)
        .px(space::S3)
        .py(space::S2)
        .gap(space::S2)
        .items_start()
        .radius(k.radius.md)
        .fill(match mark {
            Mark::Live => theme::live(),
            Mark::Hidden => theme::warn(),
            Mark::Cursor => k.c.layer_03,
            Mark::Rest => Color::TRANSPARENT,
        })
        .child(text(shown).font(type_::SMALL_STRONG).color(faint).w(20.0))
        .child(el().w(46.0).child(chip))
        .child(words);
    if mark == Mark::Cursor {
        body = body.border(2.0, k.c.text_base);
    }
    let line = body.hover(k.c.hover).cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx: &mut Tx| {
        // In the Live list a click moves the projector. In Preview it only moves the ring.
        if go_live { s.go_to(index) } else { s.select_at(index) }
        tx.blur();
    });
    col().key(format!("{key}-{index}")).role("button").label(format!("{} {}", slide.label, index + 1)).w_full().child(line).child(hline())
}

struct Pane<'a> {
    name: &'a str,
    key: &'a str,
    /// The keys that move in this pane now, as a key cap beside its name.
    keys: Option<&'a str>,
    detail: String,
    trailing: Vec<El>,
    deck: Option<&'a Deck>,
    live_at: Option<usize>,
    /// The live slide is kept off the projector.
    hidden: bool,
    /// A fixed width; without one the pane shares the row.
    width: Option<f32>,
    cursor: Option<usize>,
    go_live: bool,
    /// Shown in place of the list when there is no deck.
    empty: El,
    footer: El,
    ring: Option<Color>,
}

fn pane(k: &Kit, p: Pane<'_>) -> El {
    let mut list = col().key(format!("{}-list", p.key)).w_full().grow(1.0).min_h(0.0).px(space::S2).py(space::S1).scroll(ScrollSpec { stick_bottom: false });
    match p.deck {
        Some(deck) if !deck.slides.is_empty() => {
            let starts = section_starts(deck);
            for (index, slide) in deck.slides.iter().enumerate() {
                let mark = if p.live_at == Some(index) {
                    if p.hidden { Mark::Hidden } else { Mark::Live }
                } else if p.cursor == Some(index) {
                    Mark::Cursor
                } else {
                    Mark::Rest
                };
                let number = starts.iter().position(|start| *start == index);
                list = list.child(list_row(k, p.key, index, number, slide, mark, p.go_live));
            }
        }
        _ => list = list.child(p.empty),
    }
    let frame = col().key(p.key.to_string());
    let frame = match p.width {
        Some(width) => frame.w(width),
        None => frame.grow(1.0).min_w(0.0),
    };
    let mut frame = frame
        .h_full()
        .min_h(0.0)
        .fill(theme::surf().raised)
        .child(title_bar(k, p.name, p.keys, p.detail, p.trailing))
        .child(list.scroll_bar(k))
        .child(hline())
        .child(p.footer);
    if let Some(ring) = p.ring {
        // A ring on every side marks the pane the keys act on. It is its own element, laid
        // over the rest: the pane's own border is drawn under its children.
        frame = frame.child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).border(3.0, ring).a11y_hidden());
    }
    frame
}

/// The picture the projector would draw for a slide.
fn picture(s: &App, k: &Kit, state: &artosia_slides::frame::OutputState, width: f32) -> El {
    el().size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, theme::surf().edge).child(crate::ui::thumb(s, state, width))
}

/// The strip under a list: its picture, and what acts on it.
fn foot(size: Foot, picture: El, controls: El) -> El {
    row().w_full().h(size.height + 2.0 * space::S3).px(space::S3).py(space::S3).gap(space::S3).items_start().fill(theme::surf().side).child(picture).child(controls)
}

/// What acts on the open item: add it to the plan, edit it, read on.
fn open_tools(s: &App, k: &Kit) -> Vec<El> {
    let Some(open) = s.open.as_ref() else { return Vec::new() };
    let mut tools = vec![text(t::slides(open.deck.slides.len())).font(type_::SMALL).color(k.c.text_muted)];
    if s.live.is_some() && !s.open_is_live() {
        tools.push(btn(t::BTN_BACK_TO_LIVE()).small().hint("Esc").build(k, |s, tx| {
            s.open_live();
            crate::follow(s, tx);
            tx.blur();
        }));
    }
    if !s.open_in_plan() {
        tools.push(btn(t::BTN_ADD_TO_PLAN()).icon(&icons::PLUS).small().build(k, |s, tx| {
            s.add_open_to_plan();
            tx.blur();
        }));
    }
    match open.deck.source {
        DeckSource::Song(_) => tools.push(btn(t::BTN_EDIT()).icon(&icons::PENCIL).small().hint(t::KEY_EDIT()).build(k, |s, _| s.edit_open_song())),
        DeckSource::Announcements | DeckSource::Announcement(_) => tools.push(btn(t::BTN_EDIT()).icon(&icons::PENCIL).small().build(k, |s, _| s.edit_announcements())),
        DeckSource::Passage { .. } => tools.push(btn(t::BTN_NEXT_VERSE()).icon(&icons::PLUS).small().build(k, |s, tx| {
            s.extend_passage();
            tx.blur();
        })),
    }
    tools
}

/// Beside the Preview picture: the slide the ring is on, the one button that sends it to the
/// projector, the two that take the item before or after from the plan, and what comes here
/// once this is shown.
fn preview_controls(s: &App, k: &Kit, size: Foot, live_here: bool, hidden: bool) -> El {
    let slide = s.open.as_ref().and_then(|open| open.deck.slides.get(open.cursor));
    let label = slide.map(|slide| slide.label.clone()).unwrap_or_else(|| t::NEXT_NOTHING().to_string());
    let after = s.after_preview_title();
    let step = if size.height < 150.0 { 28.0 } else { 32.0 };
    let fixed = 20.0 + space::S2 + space::S2 + step + if after.is_some() { space::S2 + 16.0 } else { 0.0 };
    let mut name = row().w_full().h(20.0).gap(space::S2).center_y().child(text(label).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0));
    if live_here && !s.preview_waits() {
        name = name.child(state_pill(t::LIVE(), if hidden { theme::warn() } else { theme::live() }, if hidden { Color::BLACK } else { theme::on_live() }));
    }
    let show = btn(t::BTN_SHOW()).primary().height((size.height - fixed).max(36.0)).grow().hint(t::KEY_ENTER()).disabled(!s.preview_waits()).build(k, |s, tx| {
        s.enter();
        crate::follow(s, tx);
        tx.blur();
    });
    let walk = |forward: bool| {
        let (label, glyph, keys) = if forward { (t::BTN_NEXT_ITEM(), &icons::ITEM_AFTER, "Tab") } else { (t::BTN_PREVIOUS_ITEM(), &icons::ITEM_BEFORE, "Shift Tab") };
        let mut button = btn(label).icon(glyph).height(step).grow().disabled(s.walk_target(forward).is_none());
        if size.roomy() {
            button = button.hint(keys);
        }
        button.build(k, move |s, tx| {
            s.walk_plan(forward);
            crate::follow(s, tx);
            tx.blur();
        })
    };
    let mut side = col().key("preview-controls").w(size.controls).h(size.height).gap(space::S2).child(name).child(row().w_full().child(show)).child(row().w_full().gap(space::S2).child(walk(false)).child(walk(true)));
    if let Some(after) = after {
        side = side.child(el().w_full().h(16.0).child(text(t::after_this(&after)).font(type_::SMALL).color(k.c.text_muted).truncate()));
    }
    side
}

fn preview_pane(s: &App, k: &Kit, size: Foot) -> El {
    let open = s.open.as_ref();
    // Preview holds the live deck when nothing follows it; its live slide is marked there too.
    let live_index = s.live.as_ref().map(|live| live.index).filter(|_| s.open_is_live());
    let hidden = s.black || s.logo_on || s.text_hidden;
    let cursor_slide = open.and_then(|o| o.deck.slides.get(o.cursor).cloned());
    let state = s.preview_state(cursor_slide);
    // Ctrl moves the ring here while something is live; with nothing live the arrows do.
    let ctrl = s.held_ctrl && !s.held_shift;
    let keys = if s.live.is_some() { t::KEY_CTRL_ARROWS() } else { t::KEY_ARROWS() };
    pane(
        k,
        Pane {
            name: t::PANE_PREVIEW(),
            key: "slide",
            keys: open.map(|_| keys),
            detail: open.map(|o| o.deck.title.clone()).unwrap_or_default(),
            trailing: open_tools(s, k),
            deck: open.map(|o| &o.deck),
            live_at: live_index,
            hidden,
            width: None,
            cursor: open.map(|o| o.cursor).filter(|cursor| Some(*cursor) != live_index),
            go_live: false,
            // With nothing open the pane says what to do first.
            empty: if open.is_some() { empty_words(k, t::DECK_EMPTY()) } else { crate::ui::slides::start(s, k) },
            footer: foot(size, picture(s, k, &state, size.picture), preview_controls(s, k, size, live_index.is_some(), hidden)),
            ring: (ctrl && s.live.is_some() && open.is_some()).then(theme::accent),
        },
    )
}

fn empty_words(k: &Kit, words: &str) -> El {
    col().w_full().h(120.0).center().child(text(words.to_string()).font(type_::BODY).color(k.c.text_muted))
}

/// What the projector is doing, as a pill.
fn live_pill(s: &App, k: &Kit) -> El {
    let (fill, ink, words) = if s.black {
        (theme::warn(), Color::BLACK, t::STATE_BLACK())
    } else if s.logo_on {
        (theme::warn(), Color::BLACK, t::STATE_LOGO())
    } else if s.text_hidden {
        (theme::warn(), Color::BLACK, t::STATE_TEXT_HIDDEN())
    } else if s.live.is_some() {
        (theme::live(), theme::on_live(), t::STATE_ON_SCREEN())
    } else {
        (k.c.layer_03, k.c.text_muted, t::STATE_NOTHING())
    };
    state_pill(words, fill, ink)
}

/// A yellow pill for something extra on a screen, with a cross: pressing it takes that off.
fn off_pill(k: &Kit, key: &'static str, words: String, does: &'static str, off: fn(&mut App)) -> El {
    row()
        .key(key)
        .role("button")
        .label(does)
        .h(20.0)
        .pl(7.0)
        .pr(5.0)
        .gap(4.0)
        .center_y()
        .radius(6.0)
        .fill(theme::warn())
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(text(words).font(type_::TINY).color(Color::BLACK))
        .child(icon(&icons::CLOSE, 10.0).color(Color::BLACK).a11y_hidden())
        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
            off(s);
            tx.blur();
        })
}

/// What the room sees. `narrow` is the pane beside an editor: the same list and controls in a
/// thin column, so Next is still one click away while a song is being corrected.
fn live_pane(s: &App, k: &Kit, size: Option<Foot>) -> El {
    let live = s.live.as_ref();
    let state = s.output_state();
    let narrow = size.is_none();
    let footer = match size {
        None => col().w_full().px(space::S3).py(space::S3).gap(space::S3).fill(theme::surf().side).child(picture(s, k, &state, NARROW - 2.0 * space::S3)).child(live::transport(s, k, None)),
        Some(size) => foot(size, picture(s, k, &state, size.picture), live::transport(s, k, Some(size))),
    };
    // Red while the words show, yellow while something hides them.
    let hidden = s.black || s.logo_on || s.text_hidden;
    let ring = if hidden { Some(theme::warn()) } else { live.map(|_| theme::live()) };
    // A countdown and the two messages are started from the search or the phone; while one
    // shows, its pill says so here and takes it off.
    let mut trailing = Vec::new();
    if let Some(timer) = &s.countdown {
        trailing.push(off_pill(k, "countdown-pill", t::state_countdown(&clock_text(timer.ends_at - s.now)), t::CMD_STOP_COUNTDOWN(), |s| s.countdown = None));
    }
    if s.message_on {
        trailing.push(off_pill(k, "message-pill", t::STATE_MESSAGE().to_string(), t::CMD_HIDE_MESSAGE(), |s| s.message_on = false));
    }
    if s.stage_message_on {
        trailing.push(off_pill(k, "stage-message-pill", t::STATE_STAGE_MESSAGE().to_string(), t::CMD_HIDE_MESSAGE(), |s| s.stage_message_on = false));
    }
    if live.is_some_and(|live| live.deck.source == DeckSource::Announcements) {
        let label = if s.looping { t::BTN_LOOP_STOP() } else { t::BTN_LOOP_START() };
        let toggle = btn(label).icon(&icons::LOOP).small().toggle(s.looping);
        trailing.push(if narrow { toggle.icon_only() } else { toggle }.build(k, |s, tx| {
            s.toggle_loop();
            tx.blur();
        }));
    }
    trailing.push(live_pill(s, k));
    pane(
        k,
        Pane {
            name: t::LIVE(),
            key: "live-slide",
            keys: live.filter(|_| !narrow).map(|_| if s.held_shift && !s.held_ctrl { t::KEY_SHIFT_ARROWS() } else { t::KEY_ARROWS() }),
            detail: live.filter(|_| !narrow).map(|l| l.deck.title.clone()).unwrap_or_default(),
            trailing,
            deck: live.map(|l| &l.deck),
            live_at: live.map(|l| l.index),
            hidden,
            width: narrow.then_some(NARROW),
            // The slide picked with Shift held, until Shift is let go.
            cursor: s.live_pick.filter(|pick| Some(*pick) != live.map(|l| l.index)),
            go_live: true,
            empty: empty_words(k, t::LIVE_NOTHING()),
            footer,
            ring,
        },
    )
}

fn plan_column(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    // The title is the plan's name and switches between the plans of the day's services.
    let bar = crate::ui::plans::title_bar(s, k, layer, el());
    col().key("plan").w(PLAN).h_full().min_h(0.0).fill(theme::surf().side).child(bar).child(crate::ui::plan::list(s, k, layer)).child(hline()).child(crate::ui::plan::add_bar(s, k, layer))
}

/// The whole window under the title bar and over the status bar.
pub fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) -> El {
    let area = cx.layout_size("centre");
    let width = area.map(|size| size[0]).unwrap_or(1366.0 - PLAN);
    let tall = area.map(|size| size[1]).unwrap_or(680.0);
    // An editor takes the Preview pane; what the room sees stays in view beside it.
    let editor = if s.editor.is_some() {
        Some(crate::ui::editor::view(s, k, width - NARROW - 1.0))
    } else if s.notices.is_some() {
        Some(crate::ui::announcements::view(s, k, width - NARROW - 1.0))
    } else {
        None
    };
    let centre = row().key("centre").grow(1.0).min_w(0.0).h_full().min_h(0.0);
    let centre = match editor {
        Some(editor) => centre.child(col().grow(1.0).min_w(0.0).h_full().min_h(0.0).fill(theme::surf().work).child(editor)).child(vline()).child(live_pane(s, k, None)),
        None => {
            let size = Foot::fit((width - 1.0) / 2.0, tall);
            centre.child(preview_pane(s, k, size)).child(vline()).child(live_pane(s, k, Some(size)))
        }
    };
    row().key("columns").w_full().grow(1.0).min_h(0.0).child(plan_column(s, k, layer)).child(vline()).child(centre)
}

#[cfg(test)]
mod tests {
    use super::Foot;

    #[test]
    fn the_picture_grows_with_the_window_and_the_controls_keep_their_room() {
        // A 1366 by 768 laptop: each pane is 554 points wide.
        let laptop = Foot::fit(554.0, 680.0);
        assert_eq!((laptop.picture, laptop.height), (228.0, 128.0));
        assert!(laptop.controls >= super::CONTROLS);
        // A wide screen: the picture stops at about a third of the height.
        let wide = Foot::fit(1151.0, 960.0);
        assert_eq!(wide.height, 345.0);
        assert!(wide.picture > 2.5 * laptop.picture);
        assert_eq!(wide.picture + wide.controls + 12.0, 1151.0 - 24.0);
        // A pane too thin for both still gets a picture.
        assert_eq!(Foot::fit(300.0, 400.0).picture, 220.0);
    }
}
