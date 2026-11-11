//! The start screen: what to do first, in words, with the ways in. It fills the Preview pane
//! while nothing is open.

use rune::{Cursor, El, Tx, col, icon, row, spacer, text};
use rune_kit::Kit;

use crate::palette::{Command, Scope};
use crate::state::App;
use crate::strings as t;
use crate::theme::{icons, space, type_};
use crate::ui::widgets::{btn, key_hint};

pub(crate) fn quick(k: &Kit, label: &'static str, detail: &'static str, glyph: &'static rune::Icon, press: impl Fn(&mut App, &mut Tx) + 'static) -> El {
    row()
        .role("button")
        .label(label)
        .w(232.0)
        .h(64.0)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.lg)
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .child(icon(glyph, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(
            col().min_w(0.0).grow(1.0).child(text(label).font(type_::BODY_STRONG).color(k.c.text_base).truncate()).child(text(detail).font(type_::SMALL).color(k.c.text_muted).truncate()),
        )
        .on_press_tx(press)
}

/// The start screen: what to do first, in words, with the three ways in.
pub(crate) fn start(s: &App, k: &Kit) -> El {
    let mut column = col()
        .key("start")
        .w_full()
        .h_full()
        .center()
        .gap(space::S5)
        .child(text(t::START_TITLE()).font(type_::HEADING).color(k.c.text_base))
        .child(key_hint(k, t::KEY_SEARCH(), t::START_HINT()));
    column = column.child(
        col()
            .gap(space::S3)
            .child(
                row()
                    .gap(space::S3)
                    .child(quick(k, t::START_SONGS(), t::START_SONGS_DETAIL(), &icons::SONG, |s, _| s.open_palette(Scope::Songs, "")))
                    .child(quick(k, t::START_BIBLE(), t::START_BIBLE_DETAIL(), &icons::BOOK, |s, _| s.open_palette(Scope::Bible, ""))),
            )
            .child(
                row()
                    .gap(space::S3)
                    .child(quick(k, t::CMD_NEW_SONG(), t::START_NEW_DETAIL(), &icons::PLUS, |s, _| s.run(Command::NewSong)))
                    .child(quick(k, t::CMD_ANNOUNCEMENTS(), t::START_ANNOUNCEMENTS_DETAIL(), &icons::ANNOUNCE, |s, _| {
                        // With announcements written, the card opens them to show; with none, to write.
                        if s.announcements.is_empty() { s.run(Command::Announcements) } else { s.open_announcements() }
                    })),
            ),
    );
    // A quiet way into the tour while there is nothing else to do.
    column = column.child(btn(t::CMD_TOUR()).icon(&icons::PLAY).ghost().build(k, |s, _| s.start_tour()));
    if s.easislides.is_some() && !s.settings.welcomed && s.folder.is_some() {
        column = column.child(
            // The words over the two buttons: side by side, the second sentence ran under them.
            col()
                .key("easislides-card")
                .w(232.0 * 2.0 + space::S3)
                .padding(space::S4)
                .gap(space::S3)
                .radius(k.radius.lg)
                .fill(k.c.layer_01)
                .border(1.0, k.c.border_base)
                .child(
                    row().w_full().gap(space::S3).items_start().child(icon(&icons::IMPORT, 16.0).color(k.c.text_muted).a11y_hidden()).child(
                        col().grow(1.0).min_w(0.0).gap(2.0).child(text(t::WELCOME_EASISLIDES()).font(type_::BODY_STRONG).color(k.c.text_base)).child(
                            text(t::WELCOME_EASISLIDES_DETAIL()).font(type_::SMALL).color(k.c.text_muted).wrap(),
                        ),
                    ),
                )
                .child(
                    row()
                        .w_full()
                        .gap(space::S2)
                        .child(spacer())
                        .child(btn(t::BTN_NOT_NOW()).ghost().build(k, |s, _| s.dismiss_welcome()))
                        .child(btn(t::BTN_BRING_OVER()).primary().build(k, |s, _| s.import_easislides())),
                ),
        );
    } else if s.songs.is_empty() {
        column = column.child(text(t::START_NO_SONGS()).font(type_::SMALL).color(k.c.text_muted));
    }
    column.child(spacer().max_h(60.0))
}
