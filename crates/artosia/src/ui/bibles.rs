//! The Bibles dialog: bring in a file, or pick a free Bible from eBible's list.

use rune::{El, col, row, text};
use rune_kit::{Dialog, Kit};

use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{icons, space, type_};
use crate::ui::widgets::{btn, section_label};

fn own_file(k: &Kit) -> El {
    col().w_full().gap(space::S2).child(section_label(k, t::BIBLES_OWN_FILE())).child(
        row()
            .w_full()
            .gap(space::S3)
            .center_y()
            .child(btn(t::BIBLES_CHOOSE_FILE()).icon(&icons::IMPORT).focusable().build(k, |s, _| s.desk.want_file = true).autofocus().key("bible-file"))
            .child(text(t::BIBLES_FILE_HINT()).font(type_::SMALL).color(k.c.text_muted).grow(1.0).truncate()),
    )
}

pub fn view(_s: &App, k: &Kit, layer: &mut Vec<El>) {
    let body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(own_file(k));
    Dialog::new("bibles", t::BIBLES_TITLE()).open(true).width(640.0).fit().on_close(|s: &mut App| s.sheet = Sheet::None).child(body).build(k, layer);
}
