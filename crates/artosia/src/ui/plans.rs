//! The plan column's title as a switcher between the church's plans, and the rename dialog.

use rune::{Cursor, El, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, TextInput};

use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, section_label};

/// Room for the name in the title bar, between the padding and the two tools beside it.
const NAME: f32 = size::PLAN - 2.0 * space::S2;

fn edit_name(s: &mut App) -> &mut rune::TextEdit {
    match s.switcher.renaming.as_mut() {
        Some(renaming) => &mut renaming.name,
        None => &mut s.spare_edit,
    }
}

fn edit_date(s: &mut App) -> &mut rune::TextEdit {
    match s.switcher.renaming.as_mut() {
        Some(renaming) => &mut renaming.date,
        None => &mut s.spare_edit,
    }
}

/// The bar above the plan: the open plan's name, which opens the list of plans, and `tools`.
pub(crate) fn title_bar(s: &App, k: &Kit, layer: &mut Vec<El>, tools: El) -> El {
    let open = s.switcher.open;
    let name = s.plan_title();
    let trigger = row()
        .key("plan-switcher-button")
        .role("button")
        .label(name.clone())
        .help(t::PLAN_SWITCH_LABEL())
        .w(NAME)
        .h(30.0)
        .px(space::S2)
        .gap(space::S1)
        .center_y()
        .radius(k.radius.md)
        // Drawn as the box of a list that drops down, with its edge and its arrow, as the
        // worship list's box is in EasiSlides: as bare words it read as a title, and the owner
        // could not find where the other plans were.
        .fill(if open { k.c.layer_03 } else { k.c.layer_02 })
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .child(text(name).font(type_::BODY_STRONG).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
        .child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden())
        .on_press_tx(|s: &mut App, tx: &mut Tx| {
            if s.switcher.open { s.close_switcher() } else { s.open_switcher() }
            tx.blur();
        });
    let mut menu = Menu::new("plan-switcher").trigger(trigger).open(open).min_width(size::PLAN - 2.0 * space::S3).on_close(|s: &mut App| s.close_switcher());
    for row in &s.switcher.rows {
        let id = row.id;
        let current = id == s.plan.id;
        // A plan with a look of its own carries a small mark before its name; the others keep the room, so the names line up.
        let mark = match row.own_look {
            true => icon(&icons::LOOK, 14.0).color(k.c.icon_base).role("img").label(t::PLAN_OWN_LOOK()),
            false => el().size(14.0, 14.0).a11y_hidden(),
        };
        menu = menu.item(MenuItem::new(row.label()).lead(mark).checked(current).badge(row.detail()).on_select(move |s: &mut App| s.switch_plan(id)));
    }
    menu = menu
        .separator()
        .item(MenuItem::new(t::PLAN_NEW()).icon(&icons::PLUS).on_select(|s: &mut App| s.create_plan()))
        .item(MenuItem::new(t::PLAN_DUPLICATE()).icon(&icons::COPY).on_select(|s: &mut App| s.duplicate_plan()))
        .item(MenuItem::new(t::PLAN_RENAME()).icon(&icons::PENCIL).on_select(|s: &mut App| s.begin_rename(false)))
        .item(MenuItem::new(t::PLAN_DELETE()).icon(&icons::TRASH).danger(true).on_select(|s: &mut App| s.delete_plan()));
    row()
        .key("plan-title")
        .w_full()
        .h(40.0)
        .px(space::S2)
        .gap(space::S2)
        .center_y()
        .fill(theme::surf().header)
        .child(menu.build(k, layer))
        .child(spacer())
        .child(tools)
}

/// One plan of the list under the plan. The open one is filled; a press opens another.
fn strip_row(s: &App, k: &Kit, plan: &crate::plans::PlanRow) -> El {
    let id = plan.id;
    let open = id == s.plan.id;
    let own_look = if open { s.has_own_look() } else { plan.own_look };
    let mark = match own_look {
        true => icon(&icons::LOOK, 14.0).color(k.c.icon_base).role("img").label(t::PLAN_OWN_LOOK()).help(t::PLAN_OWN_LOOK()),
        false => el(),
    };
    row()
        .key(format!("plans-under-{id}"))
        .role("button")
        .label(plan.label())
        .w_full()
        .h(STRIP_ROW)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(if open { k.c.layer_03 } else { rune::Color::TRANSPARENT })
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .child(text(plan.label()).font(if open { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
        .child(mark)
        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
            s.switch_plan(id);
            tx.blur();
        })
}

/// Points of one row of the list under the plan.
const STRIP_ROW: f32 = 30.0;

/// The plans of the open plan's day, in view under the plan, where EasiSlides keeps its
/// worship lists (owner, 2026-11-11: "how to access plan picker? In easislide the plan list is
/// just below the plan"). One press opens another plan; the plus starts a new one; every
/// other plan of the church is behind the last row.
pub(crate) fn strip(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let (rows, others) = s.plan_strip();
    let day = crate::plans::day_text(&s.plan.date).unwrap_or_default();
    let head = row()
        .w_full()
        .h(28.0)
        .pl(space::S3)
        .pr(space::S2)
        .gap(space::S2)
        .center_y()
        .child(text(t::PLANS_TITLE()).font(type_::SMALL_STRONG).color(k.c.text_muted))
        .child(text(day).font(type_::SMALL).color(k.c.text_faint).truncate().grow(1.0).min_w(0.0))
        .child(btn(t::PLAN_NEW()).icon(&icons::PLUS).icon_only().ghost().small().build(k, |s, tx| {
            s.create_plan();
            tx.blur();
        }));
    let mut list = col().key("plans-under").role("list").label(t::PLANS_TITLE()).w_full().px(space::S2).gap(2.0);
    for plan in &rows {
        list = list.child(strip_row(s, k, plan));
    }
    if others > 0 {
        let open = s.switcher.all_open;
        let trigger = row()
            .key("plans-all-button")
            .role("button")
            .label(t::PLANS_ALL())
            .w_full()
            .h(STRIP_ROW)
            .px(space::S3)
            .gap(space::S2)
            .center_y()
            .radius(k.radius.md)
            .fill(if open { k.c.layer_03 } else { rune::Color::TRANSPARENT })
            .hover(k.c.hover)
            .press(k.c.pressed)
            .cursor(Cursor::Pointer)
            .child(text(t::PLANS_ALL()).font(type_::BODY).color(k.c.text_muted).truncate().grow(1.0).min_w(0.0))
            .child(text((rows.len() + others).to_string()).font(type_::SMALL).color(k.c.text_faint))
            .child(icon(k.glyph.chevron_down, 14.0).color(k.c.text_muted).a11y_hidden())
            .on_press_tx(|s: &mut App, tx: &mut Tx| {
                s.toggle_all_plans();
                tx.blur();
            });
        let mut menu = Menu::new("plans-all").trigger(trigger).open(open).min_width(size::PLAN - 2.0 * space::S3).on_close(|s: &mut App| s.switcher.all_open = false);
        for row in &s.switcher.rows {
            let id = row.id;
            menu = menu.item(MenuItem::new(row.label()).checked(id == s.plan.id).badge(row.detail()).on_select(move |s: &mut App| s.switch_plan(id)));
        }
        list = list.child(menu.build(k, layer));
    }
    col().key("plans-strip").w_full().pb(space::S2).child(head).child(list)
}

/// The dialog that names a plan and gives it a day.
pub(crate) fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(renaming) = s.switcher.renaming.as_ref() else { return };
    let ok = s.rename_date_ok();
    let field = |label: &'static str, input: El| col().w_full().gap(space::S1).child(section_label(k, label)).child(input);
    let name = TextInput::new(&renaming.name)
        .width(None)
        .label(t::PLAN_NAME_LABEL())
        .placeholder(t::PLAN_NAME_LABEL())
        .on_edit(edit_name)
        .on_submit(|s: &mut App, tx: &mut Tx| {
            s.finish_rename(true);
            tx.blur();
        })
        .build(k);
    let date = TextInput::new(&renaming.date)
        .width(None)
        .label(t::PLAN_DATE_LABEL())
        .placeholder(t::PLAN_DATE_HINT())
        .invalid(!ok)
        .on_edit(edit_date)
        .on_submit(|s: &mut App, tx: &mut Tx| {
            s.finish_rename(true);
            tx.blur();
        })
        .build(k);
    let foot = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.finish_rename(false)))
        .child(btn(t::BTN_SAVE()).primary().disabled(!ok).focusable().build(k, |s, _| s.finish_rename(true)));
    let body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(field(t::PLAN_NAME_LABEL(), name)).child(field(t::PLAN_DATE_LABEL(), date)).child(el().h(space::S1)).child(foot);
    Dialog::new("plan-rename", if renaming.fresh { t::PLAN_NEW() } else { t::PLAN_RENAME() }).open(true).width(420.0).fit().on_close(|s: &mut App| s.finish_rename(false)).child(body).build(k, layer);
}
