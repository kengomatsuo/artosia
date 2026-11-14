//! The tabs under the title bar, one per window of the program: the plan, the church's own data,
//! and what the projector looks like (UIOVER1 direction B, after Planning Center's product tabs
//! and ChurchSuite's module bar; `~/Shared/inspo/artosia-ui-overhaul/directions.md`).
//! Ctrl Page Down and Ctrl Page Up walk them, as Excel walks its sheets.

use rune::{Cursor, El, Icon, icon, row, spacer, text};
use rune_kit::Kit;

use crate::state::{App, Tab};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::key_hint;

/// The band's height.
pub const HEIGHT: f32 = 44.0;

pub fn name(tab: Tab) -> &'static str {
    match tab {
        Tab::Plan => t::TAB_PLAN(),
        Tab::Notices => t::CMD_ANNOUNCEMENTS(),
        Tab::Calendar => t::TAB_CALENDAR(),
        Tab::People => t::TAB_PEOPLE(),
        Tab::Giving => t::TAB_GIVING(),
        Tab::Templates => t::TAB_TEMPLATES(),
        Tab::Look => t::BTN_LOOK(),
    }
}

pub fn glyph(tab: Tab) -> &'static Icon {
    match tab {
        Tab::Plan => &icons::PLAN,
        Tab::Notices => &icons::ANNOUNCE,
        Tab::Calendar => &icons::CALENDAR,
        Tab::People => &icons::PERSON,
        Tab::Giving => &icons::COIN,
        Tab::Templates => &icons::TEMPLATES,
        Tab::Look => &icons::PALETTE,
    }
}

/// The element key of a tab's pill, for scripts and the tour.
pub fn key(tab: Tab) -> &'static str {
    match tab {
        Tab::Plan => "tab-plan",
        Tab::Notices => "tab-notices",
        Tab::Calendar => "tab-calendar",
        Tab::People => "tab-people",
        Tab::Giving => "tab-giving",
        Tab::Templates => "tab-templates",
        Tab::Look => "tab-look",
    }
}

pub fn bar(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let mut bar = row().key("tabs").role("tablist").w_full().h(HEIGHT).px(space::S4).gap(space::S1).center_y().fill(sf.side);
    for tab in Tab::ALL {
        let on = s.tab == tab;
        let pill = row()
            .key(key(tab))
            .role("tab")
            .label(name(tab))
            .checked(on)
            .h(30.0)
            .px(space::S3)
            .gap(space::S2)
            .center_y()
            .radius(15.0)
            .cursor(Cursor::Pointer)
            .on_press(move |s: &mut App| s.open_tab(tab));
        let pill = if on { pill.fill(theme::accent().alpha(0.2)) } else { pill.hover(k.c.hover).press(k.c.pressed) };
        bar = bar.child(
            pill.child(icon(glyph(tab), 16.0).color(if on { theme::accent() } else { k.c.text_muted }).a11y_hidden())
                .child(text(name(tab)).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(if on { k.c.text_base } else { k.c.text_muted }).a11y_hidden()),
        );
    }
    bar.child(spacer()).child(key_hint(k, "Ctrl PgUp or Ctrl PgDn", t::HINT_TABS()))
}
