//! The announcement editor: the list on the left, the one being written in the middle, and
//! how it will look on the projector on the right, changing as you type.

use artosia_core::deck::{Fill, announcement_slide};
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Tx, col, el, row, spacer, text};
use rune_kit::{Kit, ScrollBar, SegmentedControl, TextArea, TextInput};

use crate::state::{App, Notices};
use crate::strings as t;
use crate::theme::{icons, size, space, type_};
use crate::ui::widgets::{btn, section_label};

const LIST: f32 = 280.0;
/// The widest the preview is drawn.
const PREVIEW: f32 = 440.0;
/// How long an announcement may stay up, in seconds.
const SECONDS: [u32; 6] = [5, 8, 10, 15, 20, 30];

fn escape_closes(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if !matches!(ev.key, Key::Escape) {
        return false;
    }
    s.close_announcements();
    tx.blur();
    true
}

/// The form field a text input writes to; the spare edit once the editor has closed under it.
fn edit_of(s: &mut App, pick: fn(&mut Notices) -> &mut rune::TextEdit) -> &mut rune::TextEdit {
    match s.notices.as_mut() {
        Some(form) => pick(form),
        None => &mut s.spare_edit,
    }
}

fn list(s: &App, k: &Kit, form: &Notices) -> El {
    let mut rows = col().key("notice-rows").role("list").label(t::CMD_ANNOUNCEMENTS()).w_full().gap(2.0);
    let count = s.announcements.len();
    for (at, item) in s.announcements.iter().enumerate() {
        let id = item.id;
        let picked = form.id == id;
        let mut line = row()
            .key(format!("notice-{id}"))
            .role("listitem")
            .label(item.title.clone())
            .w_full()
            .h(size::ROW)
            .pl(space::S3)
            .pr(space::S1)
            .gap(space::S2)
            .center_y()
            .radius(k.radius.md)
            .fill(if picked { k.c.layer_03 } else { rune::Color::TRANSPARENT })
            .hover(k.c.hover)
            .cursor(Cursor::Pointer)
            .child(text((at + 1).to_string()).font(type_::SMALL).color(k.c.text_faint).w(16.0))
            .child(text(if item.title.is_empty() { item.body.lines().next().unwrap_or_default().to_string() } else { item.title.clone() }).font(if picked { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0));
        if picked {
            // The picked row carries its own tools, so nothing hides behind a hover.
            line = line
                .child(btn(t::PLAN_MOVE_UP()).icon(&icons::ARROW_UP).icon_only().ghost().small().disabled(at == 0).focusable().build(k, move |s, _| s.move_notice(id, -1)))
                .child(btn(t::PLAN_MOVE_DOWN()).icon(&icons::ARROW_DOWN).icon_only().ghost().small().disabled(at + 1 >= count).focusable().build(k, move |s, _| s.move_notice(id, 1)))
                .child(btn(t::BTN_DUPLICATE()).icon(&icons::COPY).icon_only().ghost().small().focusable().build(k, move |s, _| s.duplicate_notice(id)))
                .child(btn(t::BTN_DELETE()).icon(&icons::TRASH).icon_only().ghost().small().focusable().build(k, move |s, _| s.delete_announcement(id)));
        } else {
            line = line.child(text(t::seconds_short(item.seconds)).font(type_::SMALL).color(k.c.text_faint).pr(space::S2));
        }
        rows = rows.child(line.on_press(move |s: &mut App| s.pick_notice(id)));
    }
    if s.announcements.is_empty() {
        rows = rows.child(text(t::ANNOUNCEMENTS_EMPTY()).font(type_::BODY).color(k.c.text_muted).px(space::S3).py(space::S2));
    }
    let mut templates = row().key("notice-templates").w_full().gap(6.0).flex_wrap();
    for &(title, body) in t::ANNOUNCEMENT_TEMPLATES() {
        templates = templates.child(btn(title).small().focusable().build(k, move |s, _| s.new_notice(title, body)));
    }
    col()
        .key("notice-list")
        .w(LIST)
        .h_full()
        .min_h(0.0)
        .px(space::S3)
        .pb(space::S4)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(btn(t::ANNOUNCEMENT_NEW()).icon(&icons::PLUS).focusable().build(k, |s, _| s.new_notice("", "")))
        .child(rows)
        .child(text(t::announcements(s.announcements.len())).font(type_::SMALL).color(k.c.text_faint).px(space::S3))
        .child(
            col()
                .w_full()
                .gap(space::S2)
                .child(section_label(k, t::ANNOUNCEMENT_BRING_IN()))
                .child(btn(t::CMD_IMPORT_PPTX()).icon(&icons::IMPORT).small().focusable().build(k, |s, _| s.want_pptx = true))
                .child(btn(t::CMD_IMPORT_BIRTHDAYS()).icon(&icons::IMPORT).small().focusable().build(k, |s, _| s.want_birthdays = true))
                .child(text(t::birthdays_on_file(s.people.len())).font(type_::SMALL).color(k.c.text_faint)),
        )
        .child(col().w_full().gap(space::S2).child(section_label(k, t::ANNOUNCEMENT_START_FROM())).child(templates))
        .scroll_bar(k)
}

fn form(s: &App, k: &Kit, form: &Notices, width: f32) -> El {
    let at = SECONDS.iter().position(|seconds| *seconds == form.seconds);
    let labels: Vec<String> = SECONDS.iter().map(|seconds| seconds.to_string()).collect();
    let id = form.id;
    let mut tools = row().w_full().gap(space::S2).center_y();
    if id != 0 {
        tools = tools.child(btn(t::BTN_DELETE()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, _| s.delete_announcement(id)));
    }
    tools = tools.child(spacer()).child(btn(t::BTN_SAVE()).primary().hint(t::KEY_SAVE()).focusable().build(k, |s, _| s.save_notice()));
    let file = std::path::Path::new(&form.image).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let mut picture = row()
        .w_full()
        .h(36.0)
        .gap(space::S2)
        .center_y()
        .child(text(t::ANNOUNCEMENT_PICTURE()).font(type_::BODY).color(k.c.text_base))
        .child(text(file).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0))
        .child(btn(t::LOOK_CHOOSE_LOGO()).icon(&icons::IMAGE).small().focusable().build(k, |s, _| s.want_notice_image = true));
    if !form.image.is_empty() {
        picture = picture.child(btn(t::BTN_REMOVE_PICTURE()).small().ghost().focusable().build(k, |s, _| s.set_notice_image(None)));
    }
    // How it will look, drawn from what is in the form now.
    let shown = (width - 2.0 * space::S5).clamp(200.0, PREVIEW);
    let draft = s.notice_draft();
    let slide = draft.as_ref().map(|draft| {
        let pages = artosia_core::deck::announcement_pages(std::slice::from_ref(draft), &s.people, &s.today());
        pages.into_iter().next().map(|(_, slide)| slide).unwrap_or_else(|| announcement_slide(draft))
    });
    let state = s.preview_state(slide);
    let preview = col()
        .key("notice-preview")
        .w_full()
        .gap(space::S1)
        .child(section_label(k, t::ANNOUNCEMENT_PREVIEW()))
        .child(el().size(shown, (shown * 9.0 / 16.0).round()).radius(k.radius.lg).clip_rounded(k.radius.lg).border(1.0, k.c.border_base).child(crate::ui::thumb(s, &state, shown)));
    col()
        .key("notice-form")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .px(space::S5)
        .pb(space::S5)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(preview)
        .child(
            col().w_full().gap(space::S1).child(section_label(k, t::FIELD_TITLE())).child(
                TextInput::new(&form.title)
                    .width(None)
                    .label(t::FIELD_TITLE())
                    .placeholder(t::ANNOUNCEMENT_TITLE_HINT())
                    .on_edit(|s: &mut App| edit_of(s, |form| &mut form.title))
                    .on_key(escape_closes)
                    .build(k),
            ),
        )
        .child(
            col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_BODY())).child(
                TextArea::new(&form.body)
                    .width(None)
                    .label(t::ANNOUNCEMENT_BODY())
                    .placeholder(t::ANNOUNCEMENT_BODY_HINT())
                    .auto_grow(4, 14)
                    .on_edit(|s: &mut App| edit_of(s, |form| &mut form.body))
                    .on_key(escape_closes)
                    .build(k),
            ),
        )
        .child(
            row().w_full().h(36.0).gap(space::S3).center_y().child(text(t::ANNOUNCEMENT_SECONDS()).font(type_::BODY).color(k.c.text_base)).child(spacer()).child(
                SegmentedControl::new(&labels, at)
                    .label(t::ANNOUNCEMENT_SECONDS())
                    .fit()
                    .on_pick(|s: &mut App, picked, _: &mut Tx| s.set_notice_seconds(SECONDS[picked.min(SECONDS.len() - 1)]))
                    .build(k),
            ),
        )
        .child(
            col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_FILL())).child(
                SegmentedControl::new(&[t::FILL_NOTHING(), t::FILL_WEEK(), t::FILL_MONTH()], Some(form.fill as usize))
                    // The whole row: the kit's stock 232 points cut both birthday choices short.
                    .width(None)
                    .label(t::ANNOUNCEMENT_FILL())
                    .on_pick(|s: &mut App, picked, _: &mut Tx| s.set_notice_fill([Fill::Nothing, Fill::BirthdaysWeek, Fill::BirthdaysMonth][picked.min(2)]))
                    .build(k),
            ),
        )
        .child(picture)
        .child(tools)
        .scroll_bar(k)
}

pub fn view(s: &App, k: &Kit, width: f32) -> El {
    let Some(notices) = s.notices.as_ref() else { return el() };
    let header = row()
        .key("notice-header")
        .w_full()
        .px(space::S6)
        .pt(space::S5)
        .pb(space::S4)
        .gap(space::S2)
        .center_y()
        .child(text(t::CMD_ANNOUNCEMENTS()).font(type_::HEADING).color(k.c.text_base))
        .child(spacer())
        .child(btn(if s.looping { t::BTN_LOOP_STOP() } else { t::BTN_LOOP_START() }).icon(&icons::LOOP).toggle(s.looping).disabled(s.announcements.is_empty() && !s.looping).focusable().build(k, |s, tx| {
            s.toggle_loop();
            tx.blur();
        }))
        .child(btn(t::BTN_CLOSE()).hint("Esc").focusable().build(k, |s, tx| {
            s.close_announcements();
            tx.blur();
        }));
    col().key("notices").w_full().h_full().min_h(0.0).child(header).child(
        row()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(list(s, k, notices))
            .child(crate::ui::widgets::vertical_divider(k))
            .child(form(s, k, notices, width - LIST - 1.0)),
    )
}
