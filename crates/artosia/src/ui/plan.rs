//! The rows of today's plan, in the order it will be shown. The shell draws the column.

use std::rc::Rc;

use artosia_core::plan::{PlanEntry, PlanItem};
use rune::{Anim, Cursor, Drag, El, Phase, ScrollSpec, Tx, col, el, icon, row, text};
use rune_kit::{Kit, Menu, MenuItem, ScrollBar};

use crate::palette::Scope;
use crate::state::{App, PlanDrag};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, state_pill};

pub(crate) fn entry_icon(entry: &PlanEntry) -> &'static rune::Icon {
    match entry {
        PlanEntry::Song { .. } => &icons::SONG,
        PlanEntry::Passage { .. } => &icons::BOOK,
        PlanEntry::Announcements => &icons::ANNOUNCE,
        PlanEntry::Heading { .. } => &icons::HEADING,
    }
}

pub(crate) fn heading_row(s: &App, k: &Kit, item: &PlanItem, menu: El) -> El {
    row()
        .key(format!("plan-{}", item.id))
        .w_full()
        .h(HEADING)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .hover_group()
        .child(grip(k))
        .child(text(s.entry_title(&item.entry)).font(type_::SMALL_STRONG).color(k.c.text_muted).truncate().grow(1.0))
        .child(menu)
}

/// The dots at a row's start that say it can be carried (owner, 2026-11-11: "Where's the drag
/// handler?"). The whole row drags; the grip is what shows it.
fn grip(k: &Kit) -> El {
    icon(&icons::GRIP, 14.0).color(k.c.text_faint).help(t::PLAN_DRAG())
}

pub(crate) fn item_row(s: &App, k: &Kit, item: &PlanItem, menu: El) -> El {
    let id = item.id;
    let open = s.open.as_ref().is_some_and(|open| open.item == Some(id));
    let live = s.live.as_ref().is_some_and(|live| live.item == Some(id));
    let missing = s.entry_is_missing(&item.entry);
    let ink = if missing { k.c.text_muted } else { k.c.text_base };
    let body = row()
        .key(format!("plan-{id}"))
        .role("listitem")
        .label(s.entry_title(&item.entry))
        .w_full()
        .h(size::ROW)
        .pl(space::S2)
        .pr(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(if open { k.c.layer_03 } else { rune::Color::TRANSPARENT })
        .hover_group()
        .child(grip(k))
        .child(icon(entry_icon(&item.entry), 16.0).color(if live && s.on_air() { theme::live() } else { k.c.text_muted }).a11y_hidden())
        .child(text(s.entry_title(&item.entry)).font(if open { type_::BODY_STRONG } else { type_::BODY }).color(ink).truncate().grow(1.0));
    if missing {
        // An entry that cannot be shown says so in words across the row, not by a cursor.
        return body.child(state_pill(t::PLAN_MISSING(), theme::warn(), rune::Color::BLACK)).child(menu);
    }
    // A song marked to begin at one of its parts shows that part's chip, as the lists draw it.
    let mut body = body;
    if let Some(tag) = s.entry_start(&item.entry) {
        body = body.label(format!("{}, {}", s.entry_title(&item.entry), t::starts_chip(&t::section_label(tag)))).child(crate::ui::widgets::section_chip(tag));
    }
    // The item on the projector carries a red dot where its menu button is; the button takes
    // the dot's place under the pointer, and while its menu is open (owner, 2026-11-11: a dot,
    // not the word, "like Claude does").
    let slot = if live {
        let open_menu = s.plan_menu == Some(id);
        el().size(26.0, 26.0)
            .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).center().child(el().size(8.0, 8.0).radius(4.0).fill(if s.on_air() { theme::live() } else { k.c.text_muted }).layer_opacity(s.beat_glow(s.clock))).group_opacity(if open_menu { 0.0 } else { 1.0 }, 0.0).help(t::LIVE()))
            .child(menu)
    } else {
        menu
    };
    body.child(slot)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
            // The press that ends a drag put the row down; it opens nothing.
            if std::mem::take(&mut s.plan_dropped) {
                return;
            }
            // A click brings the item into Preview and never onto the projector.
            s.open_plan_item(id);
            tx.blur();
        })
}

const HEADING: f32 = 30.0;
/// Points between two rows of the plan.
const GAP: f32 = 2.0;
/// Points the pointer travels before a press on a row becomes a drag, as in the kit's lists.
const SLOP: f32 = 4.0;

/// Where a row carried `dy` points from place `from` would land: the number of other rows
/// whose middle it has passed. Rows differ in height, a divider being shorter than an item.
pub(crate) fn drop_place(from: usize, dy: f32, heights: &[f32]) -> usize {
    let middle = |index: usize| heights[..index].iter().map(|height| height + GAP).sum::<f32>() + heights[index] / 2.0;
    let carried = middle(from) + dy;
    (0..heights.len()).filter(|&other| other != from && middle(other) < carried).count()
}

/// How far a row sits from its place during a drag: the carried row under the pointer, and
/// the rows it has passed one step toward the gap it left.
fn drag_shift(drag: &PlanDrag, at: usize, heights: &[f32]) -> f32 {
    let step = heights[drag.from] + GAP;
    if at == drag.from {
        drag.dy
    } else if drag.from < at && at <= drag.to {
        -step
    } else if drag.to <= at && at < drag.from {
        step
    } else {
        0.0
    }
}

/// Makes a plan row something the pointer can carry to another place (owner, 2026-11-10:
/// "the plan items why can't be dragged for reordering?"). After the kit's `Reorder`, whose
/// own methods are private to the kit and take every row to be one height.
fn draggable(s: &App, body: El, id: u64, at: usize, heights: &Rc<[f32]>) -> El {
    let mut body = body;
    if let Some(drag) = s.plan_drag.filter(|drag| drag.from < heights.len()) {
        body = body.offset(0.0, drag_shift(&drag, at, heights));
        if drag.id == id {
            // The carried row follows the pointer exactly, opaque, over the rows it passes.
            body = body.raise().offset_motion(Anim::INSTANT).fill(theme::surf().raised).border(1.0, theme::surf().edge);
        }
    }
    let heights = heights.clone();
    body.on_drag(move |s: &mut App, drag: &mut Drag, tx: &mut Tx| match drag.phase {
        Phase::Down => s.plan_dropped = false,
        Phase::Move => {
            if s.plan_drag.is_none() && drag.dy.abs() < SLOP {
                return;
            }
            // The rows it passes spring aside.
            tx.animated();
            s.plan_menu = None;
            s.plan_drag = Some(PlanDrag { id, from: at, to: drop_place(at, drag.dy, &heights), dy: drag.dy });
        }
        Phase::Up => s.drop_plan_row(drag.cancelled()),
    })
}

pub(crate) fn row_menu(s: &App, k: &Kit, item: &PlanItem, at: usize, layer: &mut Vec<El>) -> El {
    let id = item.id;
    let open = s.plan_menu == Some(id);
    // The live item's button waits under its dot.
    let rest = if s.live.as_ref().is_some_and(|live| live.item == Some(id)) { 0.0 } else { 0.55 };
    let trigger = btn(t::PLAN_ITEM_MENU())
        .icon(&icons::DOTS)
        .icon_only()
        .ghost()
        .small()
        .build(k, move |s, tx| {
            s.plan_menu = if s.plan_menu == Some(id) { None } else { Some(id) };
            tx.blur();
        })
        .key(format!("plan-menu-{id}"))
        .group_opacity(if open { 1.0 } else { rest }, 1.0);
    let mut menu = Menu::new(format!("plan-menu-list-{id}")).trigger(trigger).open(open).min_width(236.0).on_close(|s: &mut App| s.plan_menu = None);
    if at > 0 {
        menu = menu.item(MenuItem::new(t::PLAN_MOVE_UP()).icon(&icons::ARROW_UP).on_select(move |s: &mut App| {
            s.shift_plan_item(id, -1);
            s.plan_menu = None;
        }));
    }
    if at + 1 < s.plan.items.len() {
        menu = menu.item(MenuItem::new(t::PLAN_MOVE_DOWN()).icon(&icons::ARROW_DOWN).on_select(move |s: &mut App| {
            s.shift_plan_item(id, 1);
            s.plan_menu = None;
        }));
    }
    // Where a song begins: its parts, with the one in force checked. Built only while the menu
    // is open, since reading a song's parts costs a little.
    if open && let PlanEntry::Song { song_id, .. } = &item.entry && let Some(place) = s.song_position(*song_id) {
        let current = s.entry_start(&item.entry);
        // What the search learned is offered here, never applied: one press, and the volunteer's.
        let usual = s.start_suggestion(&item.entry);
        menu = menu.separator().group_label(t::PLAN_STARTS_AT());
        menu = menu.item(MenuItem::new(t::PLAN_START_BEGINNING()).checked(current.is_none()).on_select(move |s: &mut App| {
            s.set_start(id, None);
            s.plan_menu = None;
        }));
        let mut seen = Vec::new();
        for section in s.songs[place].sections() {
            if seen.contains(&section.tag) {
                continue;
            }
            seen.push(section.tag);
            let (tag, label) = (section.tag, t::section_label(section.tag));
            let mut choice = MenuItem::new(label.clone()).checked(current == Some(tag));
            if let Some((usual_tag, times, of)) = usual && usual_tag == tag {
                choice = choice.description(t::starts_line(&label, times, of));
            }
            menu = menu.item(choice.on_select(move |s: &mut App| {
                s.set_start(id, Some(tag));
                s.plan_menu = None;
            }));
        }
        menu = menu.separator();
    }
    menu.item(MenuItem::new(t::PLAN_REMOVE()).icon(&icons::TRASH).danger(true).on_select(move |s: &mut App| s.remove_plan_item(id)))
        .build(k, layer)
}

pub(crate) fn empty(k: &Kit) -> El {
    col()
        .key("plan-empty")
        .w_full()
        .grow(1.0)
        .center()
        .gap(space::S3)
        .padding(space::S4)
        .child(text(t::PLAN_EMPTY()).font(type_::BODY).color(k.c.text_muted))
}

/// The button under the plan that adds to it, and its menu of what can be added (owner,
/// 2026-11-11: "I don't even know how to add the announcement or verses to the plan").
pub(crate) fn add_bar(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let trigger = btn(t::PLAN_ADD()).icon(&icons::PLUS).grow().build(k, |s, tx| {
        s.add_menu = !s.add_menu;
        tx.blur();
    });
    let menu = Menu::new("plan-add-menu")
        .trigger(trigger.key("plan-add-button"))
        .open(s.add_menu)
        .on_close(|s: &mut App| s.add_menu = false)
        .item(MenuItem::new(t::PLAN_ADD_SONG()).icon(&icons::SONG).on_select(|s: &mut App| s.open_palette_adding(Scope::Songs)))
        .item(MenuItem::new(t::PLAN_ADD_PASSAGE()).icon(&icons::BOOK).on_select(|s: &mut App| s.open_palette_adding(Scope::Bible)))
        .item(MenuItem::new(t::CMD_ANNOUNCEMENTS()).icon(&icons::ANNOUNCE).on_select(|s: &mut App| {
            s.add_announcements_to_plan();
            s.add_menu = false;
        }))
        .item(MenuItem::new(t::PLAN_ADD_HEADING()).icon(&icons::HEADING).on_select(|s: &mut App| {
            s.add_heading();
            s.add_menu = false;
        }))
        .build(k, layer);
    row().key("plan-add").w_full().padding(space::S2).child(menu.grow(1.0))
}

/// The plan's rows, scrolling; the column and its title belong to the shell.
pub(crate) fn list(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    if s.plan.items.is_empty() {
        return empty(k);
    }
    let mut list = col().key("plan-list").role("list").label(s.plan_title()).w_full().grow(1.0).min_h(0.0).px(space::S2).py(space::S2).gap(GAP).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE);
    let heights: Rc<[f32]> = s.plan.items.iter().map(|item| if item.entry.is_heading() { HEADING } else { size::ROW }).collect();
    for (at, item) in s.plan.items.iter().enumerate() {
        let menu = row_menu(s, k, item, at, layer);
        let body = if item.entry.is_heading() { heading_row(s, k, item, menu) } else { item_row(s, k, item, menu) };
        list = list.child(draggable(s, body, item.id, at, &heights));
    }
    list.scroll_bar(k)
}

/// Preview's tool for a song of the plan: lit while the ring's slide is where the song starts,
/// pressed to make it so, pressed again to clear it. Its key is printed on it.
pub(crate) fn start_tool(s: &App, k: &Kit) -> Option<El> {
    use crate::starts::StartState;
    let (_, _, state) = s.start_control()?;
    Some(btn(t::TIP_START_HERE()).icon(&icons::PLAY).icon_only().small().hint(t::KEY_START_HERE()).active(state == StartState::Marked).disabled(state == StartState::Beginning).build(k, |s, tx| {
        s.toggle_start_here();
        tx.blur();
    }))
}

#[cfg(test)]
mod tests {
    use super::drop_place;

    #[test]
    fn a_carried_row_lands_past_the_rows_whose_middle_it_crossed() {
        // An item, a divider, two items, 2 points apart.
        let heights = [40.0, 30.0, 40.0, 40.0];
        assert_eq!(drop_place(0, 0.0, &heights), 0);
        assert_eq!(drop_place(0, 30.0, &heights), 0, "short of the divider's middle");
        assert_eq!(drop_place(0, 40.0, &heights), 1);
        assert_eq!(drop_place(0, 400.0, &heights), 3);
        assert_eq!(drop_place(3, -400.0, &heights), 0);
        assert_eq!(drop_place(2, -20.0, &heights), 2);
        assert_eq!(drop_place(2, -40.0, &heights), 1);
    }
}
