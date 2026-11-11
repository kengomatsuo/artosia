//! The plan's title, which drops down the list of plans; the same list under the plan; and the
//! box that sets a plan's day and time from chips.

use rune::{Cursor, El, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, ScrollBar, TextInput};

use crate::plans::{Editing, PlanRow, clock_words, is_unnamed, slot_words};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, section_label};

fn edit_name(s: &mut App) -> &mut rune::TextEdit {
    match s.switcher.editing.as_mut() {
        Some(edit) => &mut edit.name,
        None => &mut s.spare_edit,
    }
}

/// The box of the drop-down: the plan column less the same room on each side, so both its ends
/// are inside the column. The list under it is at least this wide, so their edges line up.
const BOX: f32 = size::PLAN - 2.0 * space::S2;
/// The widest the list grows for a long name; past it the name ends in an ellipsis.
const LIST_MOST: f32 = 440.0;
/// What a row of the kit's menu takes beside its words (`rune-kit` `menu.rs` at the pinned rev):
/// the list's padding, the row's own, the mark before the words, the gaps and the tick.
const ROW_BESIDE: f32 = 70.0;

/// What a plan reads as in the list that drops from the title: under its day's heading, the
/// time and then the name when it has one.
fn drop_words(plan: &PlanRow) -> String {
    match (plan.day, plan.time) {
        (Some(_), Some(time)) => match plan.own_name() {
            Some(name) => format!("{} {name}", clock_words(time)),
            None => clock_words(time),
        },
        _ => plan.label(),
    }
}

/// The bar above the plan: the open plan's name as the box of a drop-down list, with its edge
/// and its arrow, as the worship list's box is in EasiSlides. The list holds every plan under
/// its day and what can be done with the open one. The owner had it taken out for being cut on
/// the right, then asked for it back whole (2026-11-11: "keep the dropdown but don't let it
/// clip right side like it does today"); the list under the plan stays as well.
pub(crate) fn title_bar(s: &App, k: &Kit, layer: &mut Vec<El>, tools: El, tall: f32) -> El {
    let open = s.switcher.open;
    let name = s.plan_title();
    let wide = |words: &str, font: rune::TextStyle| rune::text::measure(words, &[], font, None).0;
    // A plan with a name of its own says its day and time beside it, when both fit the box:
    // half a day's name after a name cut short says nothing.
    let slot = slot_words(s.plan.day, s.plan.time).filter(|_| !is_unnamed(&s.plan.name)).unwrap_or_default();
    let room = BOX - 2.0 * space::S2 - 2.0 * space::S2 - 14.0;
    let slot = if wide(&name, type_::BODY_STRONG) + wide(&slot, type_::SMALL) <= room { slot } else { String::new() };
    let trigger = row()
        .key("plan-switcher-button")
        .role("button")
        .label(name.clone())
        .help(t::PLANS_TITLE())
        .w(BOX)
        .h(30.0)
        .px(space::S2)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(if open { k.c.layer_03 } else { k.c.layer_02 })
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .child(text(name).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0))
        // An empty text that may shrink still draws its ellipsis, so it is left out.
        .children((!slot.is_empty()).then(|| text(slot).font(type_::SMALL).color(k.c.text_muted)))
        .child(spacer())
        .child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden())
        .on_press_tx(|s: &mut App, tx: &mut Tx| {
            if s.switcher.open {
                s.close_switcher();
            } else {
                s.open_switcher();
                // The list may be longer than its room: it opens with the open plan in view.
                if let Some(entry) = s.switcher_entry() {
                    tx.scroll_into_view(format!("plan-switcher-item{entry}"));
                }
            }
            tx.blur();
        });
    // The list is as wide as its longest row needs, so no name is cut while there is room for
    // it; it may then reach over Preview, where it is drawn whole with its edge and shadow.
    let groups = s.plan_groups();
    let longest = groups.iter().flat_map(|(_, plans)| plans.iter()).map(|plan| wide(&drop_words(plan), k.ty.ui_compact)).fold(0.0f32, f32::max);
    let keys = wide(t::PLAN_NEW(), k.ty.ui_compact) + wide("Ctrl Shift N", k.ty.small_medium) + space::S2;
    let width = (longest.max(keys) + ROW_BESIDE).clamp(BOX, LIST_MOST);
    let mut menu = Menu::new("plan-switcher").trigger(trigger).open(open).min_width(width).on_close(|s: &mut App| s.close_switcher());
    let today = s.weekday_now().0;
    for (day, plans) in groups {
        menu = menu.group_label(match day {
            Some(day) if day == today => t::PLANS_TODAY().to_string(),
            Some(day) => t::WEEKDAYS()[usize::from(day.min(6))].to_string(),
            None => t::PLANS_NO_DAY().to_string(),
        });
        for plan in &plans {
            let id = plan.id;
            let current = id == s.plan.id;
            let own_look = if current { s.has_own_look() } else { plan.own_look };
            // A plan with a look of its own carries a small mark before its name; the others
            // keep the room, so the times line up.
            let mark = match own_look {
                true => icon(&icons::LOOK, 14.0).color(k.c.icon_base).role("img").label(t::PLAN_OWN_LOOK()),
                false => el().size(14.0, 14.0).a11y_hidden(),
            };
            menu = menu.item(MenuItem::new(drop_words(plan)).lead(mark).checked(current).on_select(move |s: &mut App| {
                s.close_switcher();
                s.switch_plan(id);
            }));
        }
    }
    let here = s.plan.id;
    menu = menu
        .separator()
        .item(MenuItem::new(t::PLAN_NEW()).icon(&icons::PLUS).shortcut("Ctrl Shift N").on_select(|s: &mut App| {
            s.close_switcher();
            s.begin_new_plan();
        }))
        .item(MenuItem::new(t::PLAN_RENAME()).icon(&icons::TIMER).on_select(move |s: &mut App| {
            s.close_switcher();
            s.begin_edit(here);
        }))
        .item(MenuItem::new(t::PLAN_DUPLICATE()).icon(&icons::COPY).on_select(move |s: &mut App| {
            s.close_switcher();
            s.begin_copy(here);
        }))
        .item(MenuItem::new(t::PLAN_DELETE()).icon(&icons::TRASH).danger(true).on_select(|s: &mut App| {
            s.close_switcher();
            s.delete_plan();
        }));
    let mounted = layer.len();
    let built = menu.build(k, layer);
    // The kit lets a menu run to the foot of the window. This one stops at about three fifths
    // of it and scrolls, fading where more waits (owner, 2026-11-11: "The plans dont have the
    // max height?"). The surface is the last thing the menu mounted.
    if layer.len() > mounted
        && let Some(surface) = layer.pop()
    {
        layer.push(surface.max_h(drop_max(tall)));
    }
    row().key("plan-title").w_full().h(40.0).px(space::S2).gap(space::S2).center_y().fill(theme::surf().header).child(built).child(tools)
}

/// The most the list that drops from the title may take of a window `tall` points high.
pub(crate) fn drop_max(tall: f32) -> f32 {
    (tall * 0.6).clamp(240.0, 520.0)
}

/// Points of one row of the list under the plan.
const STRIP_ROW: f32 = 30.0;

/// The most the list of plans may take of a column `tall` points high: about a third, so three
/// Sunday services and a Saturday one show without scrolling on a 768 point screen, and never
/// less than five rows or more than ten. Past it the list scrolls and fades at the edge that
/// hides more (owner, 2026-11-11: "The plans dont have the max height?"; a tester then found
/// four plans did not fit a fixed five rows).
pub(crate) fn strip_max(tall: f32) -> f32 {
    (tall * 0.34).clamp(170.0, 330.0)
}

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
    // The open plan carries a tick as well as its fill, which alone a tester took for a hover;
    // the others keep the tick's room, so the times line up.
    line = line.child(match open {
        true => icon(&icons::CHECK, 12.0).color(k.c.text_base).a11y_hidden(),
        false => el().size(12.0, 12.0).a11y_hidden(),
    });
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
    // Ctrl Tab is drawn on the row it opens. Beside the heading it ran past the column's edge
    // in Indonesian, whose word for the list is longer.
    if s.plan_beside(true) == Some(id) {
        line = line.child(crate::ui::widgets::keycaps(k, "Ctrl Tab", None));
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
        // The menu names the plan it acts on: it may open beside another row.
        .group_label(plan.label())
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
pub(crate) fn strip(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) -> El {
    let head = row()
        .w_full()
        .h(28.0)
        .pl(space::S3)
        .pr(space::S2)
        .gap(space::S2)
        .center_y()
        .child(text(t::PLANS_TITLE()).font(type_::SMALL_STRONG).color(k.c.text_muted))
        .child(spacer())
        .child(btn(t::PLAN_NEW()).icon(&icons::PLUS).icon_only().ghost().small().hint("Ctrl Shift N").build(k, |s, tx| {
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
    let list = col().key("plans-scroll").w_full().max_h(strip_max(tall)).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(rows).scroll_bar(k);
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
        .on_key(box_keys)
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

/// Keys while a chip of the box has the focus. Enter saves, the one thing the box is for; a
/// chip is already chosen by the arrow that reached it. A digit types the time. A keys-only
/// tester pressed Enter on an hour and nothing happened, and counted nine arrows from 16 to 07.
fn box_keys(s: &mut App, ev: &rune::KeyEvent, tx: &mut Tx) -> bool {
    if ev.mods != rune::Mods::default() {
        return false;
    }
    match &ev.key {
        rune::Key::Enter => {
            s.finish_edit(true);
            tx.scroll_into_view(format!("plans-under-{}", s.plan.id));
            true
        }
        rune::Key::Text(typed) if typed.len() == 1 && typed.chars().all(|c| c.is_ascii_digit()) => {
            if let Some(digit) = typed.chars().next() {
                s.plan_box_digit(digit);
            }
            true
        }
        // A day is typed by its name, "sun" or "min": a tester typed "sunday 07:00" and only
        // the hour was taken.
        rune::Key::Text(typed) if typed.chars().count() == 1 && typed.chars().all(char::is_alphabetic) => {
            if let Some(letter) = typed.chars().next() {
                s.plan_box_letter(letter);
            }
            true
        }
        // The marks between a day and a time, and inside a time, are part of typing one.
        rune::Key::Text(typed) if matches!(typed.as_str(), " " | ":" | "." | ",") => true,
        _ => false,
    }
}

/// A row of choices under its label: one Tab stop, the arrows choose as they move.
fn choices(k: &Kit, label: &'static str, chips: Vec<El>) -> El {
    let mut group = row().role("radiogroup").label(label).w_full().gap(space::S1).flex_wrap().focus_group().focus_follows();
    for chip in chips {
        group = group.child(chip);
    }
    // The arrows that move in the row are drawn beside its name: a tester found out by trying.
    col().w_full().gap(space::S1).child(row().gap(space::S2).center_y().child(section_label(k, label)).child(crate::ui::widgets::keycaps(k, "Left Right", None))).child(group)
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
                // A press starts the typed time afresh.
                edit.typed.clear();
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
    let hours: Vec<El> = (0..24u8).map(|hour| chip(k, format!("plan-hour-{hour}"), format!("{hour:02}"), edit.hour == Some(hour), Some(40.0), set(|edit, hour| edit.hour = Some(hour), hour))).collect();
    let minutes: Vec<El> = (0..12u8).map(|step| step * 5).map(|minute| chip(k, format!("plan-minute-{minute}"), format!("{minute:02}"), edit.minute == minute, Some(40.0), set(|edit, minute| edit.minute = minute, minute))).collect();
    // What the plan will read as, said back as the chips are pressed.
    let slot = slot_words(Some(edit.day), edit.time()).unwrap_or_default();
    let ready = edit.hour.is_some();
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
        .child(btn(t::BTN_SAVE()).primary().disabled(!ready).focusable().build(k, |s, tx| {
            s.finish_edit(true);
            // The plan just made or changed is brought into view in the list.
            tx.scroll_into_view(format!("plans-under-{}", s.plan.id));
        }));
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
