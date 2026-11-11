//! The plan's title, the list of every plan under the plan, and the box that sets a plan's day
//! and time from chips.

use rune::{Cursor, El, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, ScrollBar, TextInput};

use crate::plans::{Editing, PlanRow, clock_words, is_unnamed, slot_words};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::{btn, section_label};

fn edit_name(s: &mut App) -> &mut rune::TextEdit {
    match s.switcher.editing.as_mut() {
        Some(edit) => &mut edit.name,
        None => &mut s.spare_edit,
    }
}

/// The bar above the plan: what the open plan is, as a title and nothing else. The list of
/// plans is under the plan, so nothing drops down from here (owner, 2026-11-11: "make it show
/// title only id rather have that than a clipping dropdown").
pub(crate) fn title_bar(s: &App, k: &Kit, _layer: &mut Vec<El>, tools: El) -> El {
    // A plan with a name of its own says its day and time beside it.
    let slot = slot_words(s.plan.day, s.plan.time).filter(|_| !is_unnamed(&s.plan.name)).unwrap_or_default();
    row()
        .key("plan-title")
        .w_full()
        .h(40.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .fill(theme::surf().header)
        .child(text(s.plan_title()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0))
        .child(text(slot).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0))
        .child(spacer())
        .child(tools)
}

/// Points of one row of the list under the plan.
const STRIP_ROW: f32 = 30.0;
/// The list of plans never grows past this, about five rows; past it the list scrolls and
/// fades at the edge that hides more (owner, 2026-11-11: "The plans dont have the max height?").
const STRIP_MAX: f32 = 168.0;

/// One plan of the list: its time, its name when it has one, and its own menu. The open one is
/// filled; a press opens another.
fn strip_row(s: &App, k: &Kit, plan: &PlanRow, layer: &mut Vec<El>) -> El {
    let id = plan.id;
    let open = id == s.plan.id;
    let own_look = if open { s.has_own_look() } else { plan.own_look };
    let font = if open { type_::BODY_STRONG } else { type_::BODY };
    let mut line = row()
        .key(format!("plans-under-{id}"))
        .role("button")
        .label(plan.label())
        .w_full()
        .h(STRIP_ROW)
        .pl(space::S3)
        .pr(space::S1)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(if open { k.c.layer_03 } else { rune::Color::TRANSPARENT })
        .hover(k.c.hover)
        .press(k.c.pressed)
        .hover_group()
        .cursor(Cursor::Pointer);
    // The time leads the row, in digits of one width so the times line up; a plan with no day
    // reads by its name alone.
    match (plan.day, plan.time) {
        (Some(_), Some(time)) => {
            line = line.child(text(clock_words(time)).font(font.features("'tnum' 1")).color(k.c.text_base));
            line = line.child(text(plan.own_name().unwrap_or_default()).font(font).color(if open { k.c.text_base } else { k.c.text_muted }).truncate().grow(1.0).min_w(0.0));
        }
        _ => line = line.child(text(plan.label()).font(font).color(k.c.text_base).truncate().grow(1.0).min_w(0.0)),
    }
    if own_look {
        line = line.child(icon(&icons::LOOK, 14.0).color(k.c.icon_base).role("img").label(t::PLAN_OWN_LOOK()).help(t::PLAN_OWN_LOOK()));
    }
    // Each plan carries its own menu, so a copy is made from the plan it copies.
    let menu_open = s.switcher.menu == Some(id);
    let trigger = btn(t::PLAN_ITEM_MENU())
        .icon(&icons::DOTS)
        .icon_only()
        .ghost()
        .small()
        .build(k, move |s, tx| {
            s.switcher.menu = if s.switcher.menu == Some(id) { None } else { Some(id) };
            tx.blur();
        })
        .key(format!("plans-under-menu-{id}"))
        .group_opacity(if menu_open { 1.0 } else { 0.55 }, 1.0);
    let menu = Menu::new(format!("plans-under-list-{id}"))
        .trigger(trigger)
        .open(menu_open)
        .min_width(200.0)
        .on_close(|s: &mut App| s.switcher.menu = None)
        .item(MenuItem::new(t::PLAN_RENAME()).icon(&icons::TIMER).on_select(move |s: &mut App| s.begin_edit(id)))
        .item(MenuItem::new(t::PLAN_DUPLICATE()).icon(&icons::COPY).on_select(move |s: &mut App| s.begin_copy(id)))
        .item(MenuItem::new(t::PLAN_DELETE()).icon(&icons::TRASH).danger(true).on_select(move |s: &mut App| s.delete_plan_id(id)))
        .build(k, layer);
    line.child(menu).on_press_tx(move |s: &mut App, tx: &mut Tx| {
        s.switch_plan(id);
        tx.blur();
    })
}

/// Every plan of the church, in view under the plan, where EasiSlides keeps its worship lists
/// (owner, 2026-11-11: "how to access plan picker? In easislide the plan list is just below the
/// plan"). The plans are grouped by their day of the week, today's first, each day in the order
/// of its times; one press opens another, and the plus starts a new one.
pub(crate) fn strip(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let head = row()
        .w_full()
        .h(28.0)
        .pl(space::S3)
        .pr(space::S2)
        .gap(space::S2)
        .center_y()
        .child(text(t::PLANS_TITLE()).font(type_::SMALL_STRONG).color(k.c.text_muted))
        .child(spacer())
        .child(btn(t::PLAN_NEW()).icon(&icons::PLUS).icon_only().ghost().small().build(k, |s, tx| {
            s.begin_new_plan();
            tx.blur();
        }));
    let today = s.weekday_now().0;
    let mut rows = col().key("plans-under").role("list").label(t::PLANS_TITLE()).w_full().px(space::S2).pb(space::S1).gap(2.0);
    for (day, plans) in s.plan_groups() {
        let heading = match day {
            Some(day) if day == today => t::PLANS_TODAY().to_string(),
            Some(day) => t::WEEKDAYS()[usize::from(day.min(6))].to_string(),
            None => t::PLANS_NO_DAY().to_string(),
        };
        rows = rows.child(row().w_full().h(22.0).px(space::S3).items_end().child(text(heading).font(type_::TINY).color(k.c.text_faint).truncate()));
        for plan in &plans {
            rows = rows.child(strip_row(s, k, plan, layer));
        }
    }
    let list = col().key("plans-scroll").w_full().max_h(STRIP_MAX).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(rows).scroll_bar(k);
    col().key("plans-strip").w_full().child(head).child(list)
}

/// One choice of a row of choices: filled when it is the chosen one.
fn chip(k: &Kit, key: String, label: String, on: bool, width: Option<f32>, press: impl Fn(&mut App) + 'static) -> El {
    let (fill, ink) = if on { (k.c.text_base, k.c.page) } else { (k.c.layer_02, k.c.text_base) };
    let mut chip = el()
        .key(key)
        .role("radio")
        .label(label.clone())
        .h(30.0)
        .px(space::S2)
        .center()
        .radius(k.radius.md)
        .fill(fill)
        .hover(if on { rune::Color::BLACK.alpha(0.08) } else { k.c.hover })
        .focusable(true)
        .focus_selected(on)
        .on_key(crate::ui::widgets::enter_key)
        .cursor(Cursor::Pointer)
        .child(text(label).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(ink).a11y_hidden());
    if !on {
        chip = chip.border(1.0, k.c.border_base);
    }
    if let Some(width) = width {
        chip = chip.w(width);
    }
    chip.on_press(press)
}

/// A row of choices under its label: one Tab stop, the arrows choose as they move.
fn choices(k: &Kit, label: &'static str, chips: Vec<El>) -> El {
    let mut group = row().role("radiogroup").label(label).w_full().gap(space::S1).flex_wrap().focus_group().focus_follows();
    for chip in chips {
        group = group.child(chip);
    }
    col().w_full().gap(space::S1).child(section_label(k, label)).child(group)
}

/// The box that sets a plan's day and time. Every part is a chip to press; only a name, which
/// a plan may do without, is typed (owner, 2026-11-11: "why cant like every sunday hh:mm you
/// love textfields too much").
pub(crate) fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(edit) = s.switcher.editing.as_ref() else { return };
    let title = match edit.what {
        Editing::New => t::PLAN_NEW(),
        Editing::Copy(_) => t::PLAN_DUPLICATE(),
        Editing::Existing(_) => t::PLAN_RENAME(),
    };
    let set = |change: fn(&mut crate::plans::PlanEdit, u8), value: u8| {
        move |s: &mut App| {
            if let Some(edit) = s.switcher.editing.as_mut() {
                change(edit, value);
            }
        }
    };
    let days: Vec<El> = (0..7u8)
        .map(|day| {
            let chip = chip(k, format!("plan-day-{day}"), t::WEEKDAYS()[usize::from(day)].to_string(), edit.day == day, None, set(|edit, day| edit.day = day, day));
            // The box opens on the day, the first thing to choose.
            if edit.day == day { chip.autofocus() } else { chip }
        })
        .collect();
    let hours: Vec<El> = (0..24u8).map(|hour| chip(k, format!("plan-hour-{hour}"), format!("{hour:02}"), edit.hour == hour, Some(40.0), set(|edit, hour| edit.hour = hour, hour))).collect();
    let minutes: Vec<El> = (0..12u8).map(|step| step * 5).map(|minute| chip(k, format!("plan-minute-{minute}"), format!("{minute:02}"), edit.minute == minute, Some(40.0), set(|edit, minute| edit.minute = minute, minute))).collect();
    // What the plan will read as, said back as the chips are pressed.
    let slot = slot_words(Some(edit.day), Some(edit.time())).unwrap_or_default();
    let name = TextInput::new(&edit.name)
        .width(None)
        .label(t::PLAN_NAME_LABEL())
        .placeholder(slot.clone())
        .on_edit(edit_name)
        .on_submit(|s: &mut App, tx: &mut Tx| {
            s.finish_edit(true);
            tx.blur();
        })
        .build(k);
    let foot = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(text(slot).font(type_::TITLE).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.finish_edit(false)))
        .child(btn(t::BTN_SAVE()).primary().focusable().build(k, |s, _| s.finish_edit(true)));
    let body = col()
        .w_full()
        .px(space::S4)
        .pb(space::S4)
        .gap(space::S3)
        .child(choices(k, t::PLAN_DAY(), days))
        .child(choices(k, t::PLAN_HOUR(), hours))
        .child(choices(k, t::PLAN_MINUTE(), minutes))
        .child(col().w_full().gap(space::S1).child(section_label(k, t::PLAN_NAME_LABEL())).child(name))
        .child(el().h(space::S1))
        .child(foot);
    Dialog::new("plan-edit", title).open(true).width(572.0).fit().on_close(|s: &mut App| s.finish_edit(false)).child(body).build(k, layer);
}
