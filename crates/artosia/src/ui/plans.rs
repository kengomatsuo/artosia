//! The plan column's title as a switcher between the church's plans, and the rename dialog.

use rune::{Cursor, El, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, TextInput};

use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, section_label};

/// Room for the name in the title bar, between the padding and the two tools beside it.
const NAME: f32 = size::PLAN - 2.0 * space::S3 - 2.0 * space::S2 - 26.0 - 32.0 - 12.0;

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
        .fill(if open { k.c.layer_03 } else { rune::Color::TRANSPARENT })
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
    Dialog::new("plan-rename", t::PLAN_RENAME()).open(true).width(420.0).fit().on_close(|s: &mut App| s.finish_rename(false)).child(body).build(k, layer);
}
