//! The Bibles dialog: pick a free Bible from eBible's list, or bring in a file.

use artosia_core::ebible::{Source, Translation};
use rune::{El, GrowthSpec, ScrollSpec, col, el, row, spacer, text};
use rune_kit::{Dialog, Kit, ScrollBar, TextInput};

use crate::bibles::{Catalogue, Notice, Row, installed_code};
use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::{btn, divider, section_label, state_pill};

/// Every line of the list has this height, language headings included.
const ROW: f32 = 36.0;
/// Seven lines show at once.
const LIST: f32 = ROW * 7.0;

fn scope_name(scope: u8) -> &'static str {
    match scope {
        0 => t::BIBLES_SCOPE_WHOLE(),
        1 => t::NEW_TESTAMENT(),
        2 => t::OLD_TESTAMENT(),
        _ => t::BIBLES_SCOPE_PART(),
    }
}

fn source_name(source: Source) -> &'static str {
    match source {
        Source::Ebible => t::BIBLES_SOURCE_EBIBLE(),
        Source::Beblia => t::BIBLES_SOURCE_BEBLIA(),
    }
}

/// How much of the Bible a row holds, or the file's size when the list cannot say.
pub(crate) fn extent(item: &Translation) -> String {
    if item.scope() < 4 {
        return scope_name(item.scope()).to_string();
    }
    match item.size {
        0 => String::new(),
        size if size >= 1_000_000 => format!("{} MB", (size + 500_000) / 1_000_000),
        size => format!("{} KB", (size + 500) / 1000),
    }
}

pub(crate) fn installed(s: &App, id: &str) -> bool {
    let code = installed_code(id);
    s.bible_files.iter().any(|file| file.code == code)
}

/// Grey bars where the list will be, so the dialog keeps its shape while eBible answers.
fn skeleton(k: &Kit) -> El {
    let mut lines = col().key("bibles-skeleton").w_full().child(row().w_full().h(ROW).px(space::S3).center_y().child(text(t::BIBLES_LOADING()).font(type_::SMALL).color(k.c.text_muted)));
    for (at, width) in [180.0f32, 120.0, 260.0, 210.0, 150.0, 240.0].into_iter().enumerate() {
        lines = lines.child(
            row().key(format!("bibles-bar-{at}")).w_full().h(ROW).px(space::S3).center_y().gap(space::S3).a11y_hidden().child(el().w(width).h(12.0).radius(4.0).fill(k.c.layer_04)),
        );
    }
    lines
}

fn failed(k: &Kit, why: &str) -> El {
    col()
        .key("bibles-failed")
        .w_full()
        .h(LIST)
        .center()
        .gap(space::S3)
        .padding(space::S4)
        .child(text(t::BIBLES_LOAD_FAILED()).font(type_::BODY_STRONG).color(theme::warn_ink()))
        .child(crate::ui::widgets::wrapped(t::as_sentence(why)).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(btn(t::BIBLES_RETRY()).icon(&icons::LOOP).focusable().build(k, |s, _| s.start_catalogue()))
}

fn nothing(s: &App, k: &Kit) -> El {
    col().key("bibles-empty").w_full().h(LIST).center().padding(space::S4).child(crate::ui::widgets::wrapped(t::nothing_found(s.desk.query.text.trim())).font(type_::BODY).color(k.c.text_muted).wrap())
}

fn line(s: &App, k: &Kit, at: usize) -> El {
    match s.desk.rows.get(at) {
        Some(Row::Language { name, local }) => {
            let mut heading = row().key(format!("bibles-language-{name}")).w_full().h(ROW).px(space::S3).gap(space::S2).center_y().child(text(t::language_word(name)).font(type_::SMALL_STRONG).color(k.c.text_muted).truncate());
            if !local.is_empty() {
                heading = heading.child(text(local.clone()).font(type_::SMALL).color(k.c.text_muted).truncate());
            }
            heading
        }
        Some(Row::Translation(index)) => {
            let Some(item) = s.desk.translations().get(*index) else { return el() };
            let id = item.id.clone();
            let picked = s.desk.picked.as_deref() == Some(item.id.as_str());
            let title = s.desk.title_of(item).to_string();
            let mut line = row()
                .key(format!("bible-{}", item.id))
                .role("button")
                .label(title.clone())
                .w_full()
                .h(ROW)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(if picked { k.c.layer_04 } else { rune::Color::TRANSPARENT })
                .hover(k.c.hover)
                .focusable(true).on_key(crate::ui::widgets::enter_key)
                .cursor(rune::Cursor::Pointer)
                .child(el().w(space::S2))
                .child(text(title).font(if picked { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0))
                .child(text(extent(item)).font(type_::SMALL).color(k.c.text_muted))
                .child(el().w(64.0).child(text(source_name(item.source)).font(type_::SMALL).color(k.c.text_muted).truncate()));
            if installed(s, &item.id) {
                line = line.child(state_pill(t::BIBLES_INSTALLED(), k.c.layer_03, k.c.text_base));
            }
            line.on_press(move |s: &mut App| s.pick_bible(&id))
        }
        None => el(),
    }
}

fn list(s: &App, k: &Kit) -> El {
    let inside = match &s.desk.catalogue {
        Catalogue::Idle | Catalogue::Loading => skeleton(k),
        Catalogue::Failed(why) => failed(k, why),
        Catalogue::Ready(_) if s.desk.rows.is_empty() => nothing(s, k),
        Catalogue::Ready(_) => {
            // Only the lines on screen are built: the list holds over a thousand translations.
            let rows = col().key("bible-rows").growth(GrowthSpec::default()).virtual_rows(
                s.desk.rows.len(),
                ROW,
                |s: &App, at| match s.desk.rows.get(at) {
                    Some(Row::Language { name, .. }) => format!("language-{name}").into(),
                    Some(Row::Translation(index)) => s.desk.translations().get(*index).map(|item| format!("bible-{}", item.id)).unwrap_or_default().into(),
                    None => format!("row-{at}").into(),
                },
                |s: &App, at| line(s, theme::kit(), at),
            );
            col().key("bible-scroll").role("listbox").label(t::BIBLES_LIST()).w_full().h(LIST).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(rows).scroll_bar(k)
        }
    };
    col().key("bibles-list").w_full().h(LIST).radius(k.radius.lg).fill(k.c.layer_02).clip_rounded(k.radius.lg).child(inside)
}

/// A translation's own copyright line, shown before anything of it downloads. eBible's list
/// carries it; a Beblia file carries it inside, read when the row was picked.
pub(crate) fn copyright_line(s: &App, item: &Translation) -> String {
    match (item.source, s.desk.notices.get(&item.id)) {
        (Source::Ebible, _) => if item.copyright.is_empty() { item.description.clone() } else { item.copyright.clone() },
        (Source::Beblia, Some(Notice::Known { copyright, .. })) if copyright.is_empty() => t::BIBLES_NO_COPYRIGHT().to_string(),
        (Source::Beblia, Some(Notice::Known { copyright, .. })) => copyright.clone(),
        (Source::Beblia, Some(Notice::Failed)) => t::BIBLES_NOTICE_FAILED().to_string(),
        (Source::Beblia, _) => t::BIBLES_NOTICE_LOADING().to_string(),
    }
}

/// What the picked Bible is, its copyright line, and the button that fetches it.
fn detail(s: &App, k: &Kit) -> El {
    let card = col().key("bibles-detail").w_full().h(132.0).padding(space::S3).gap(space::S1).radius(k.radius.lg).fill(k.c.layer_02);
    let Some(item) = s.desk.picked_translation() else {
        return card.center().child(text(t::BIBLES_PICK_HINT()).font(type_::BODY).color(k.c.text_muted));
    };
    let code = installed_code(&item.id);
    let title = s.desk.title_of(item).to_string();
    let notice = copyright_line(s, item);
    let (have, using) = (installed(s, &item.id), s.settings.bible == code);
    let id = item.id.clone();
    let busy = s.desk.downloading.is_some();
    let action = if using {
        btn(t::BIBLES_IN_USE()).disabled(true).build(k, |_, _| {})
    } else if have {
        let wanted = code.clone();
        btn(t::BIBLES_USE()).primary().focusable().build(k, move |s, _| s.use_bible(&wanted))
    } else if s.desk.downloading.as_deref() == Some(item.id.as_str()) {
        btn(t::BIBLES_DOWNLOADING()).icon(&icons::IMPORT).disabled(true).build(k, |_, _| {})
    } else {
        btn(t::BIBLES_DOWNLOAD()).icon(&icons::IMPORT).primary().disabled(busy).focusable().build(k, move |s, _| s.download_bible(&id))
    };
    card.child(
        row()
            .w_full()
            .gap(space::S2)
            .center_y()
            .child(text(title).font(type_::BODY_STRONG).color(k.c.text_base).truncate().grow(1.0))
            .child(text(t::language_word(&item.language_english)).font(type_::SMALL).color(k.c.text_muted))
            .child(text(extent(item)).font(type_::SMALL).color(k.c.text_muted))
            .child(text(source_name(item.source)).font(type_::SMALL_STRONG).color(k.c.text_muted)),
    )
    .child(text(t::BIBLES_COPYRIGHT()).font(type_::SMALL_STRONG).color(k.c.text_muted))
    .child(text(notice).font(type_::SMALL).color(k.c.text_base).max_lines(2).grow(1.0))
    .child(row().w_full().child(spacer()).child(action))
}

fn own_file(k: &Kit) -> El {
    col().w_full().gap(space::S2).child(section_label(k, t::BIBLES_OWN_FILE())).child(
        row()
            .w_full()
            .gap(space::S3)
            .center_y()
            .child(btn(t::BIBLES_CHOOSE_FILE()).icon(&icons::FOLDER).focusable().build(k, |s, _| s.desk.want_file = true).key("bible-file"))
            .child(text(t::BIBLES_FILE_HINT()).font(type_::SMALL).color(k.c.text_muted).grow(1.0).truncate()),
    )
}

pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let search = TextInput::new(&s.desk.query).width(None).label(t::BIBLES_SEARCH()).placeholder(t::BIBLES_SEARCH()).on_edit(|s: &mut App| &mut s.desk.query).build(k).autofocus();
    let body = col()
        .w_full()
        .px(space::S4)
        .pb(space::S4)
        .gap(space::S3)
        .child(col().w_full().gap(space::S2).child(section_label(k, t::BIBLES_LIST())).child(search))
        .child(list(s, k))
        .child(detail(s, k))
        .child(divider(k))
        .child(own_file(k));
    Dialog::new("bibles", t::BIBLES_TITLE()).open(true).width(680.0).fit().on_close(|s: &mut App| s.sheet = Sheet::None).child(body).build(k, layer);
}
