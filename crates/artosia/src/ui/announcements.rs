//! The announcement editor: the list on the left, the one being written in the middle, and
//! how it will look on the projector on the right, changing as you type.

use artosia_core::deck::{Fill, PassageState, announcement_slide, birthday_days, birthdays, passage_state};
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Tx, col, el, row, spacer, text};
use rune_kit::{Kit, ScrollBar, SegmentedControl, TextArea, TextInput};

use crate::state::{App, Notices};
use crate::strings as t;
use crate::theme::{icons, size, space, type_};
use crate::ui::widgets::{btn, section_label};

const LIST: f32 = 330.0;
/// The widest the preview is drawn.
const PREVIEW: f32 = 440.0;
/// The place of "Verse of the week" among `ANNOUNCEMENT_TEMPLATES`, in every language.
const PASSAGE_TEMPLATE: usize = 3;
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
            line = line.child(text(t::seconds_short(item.seconds)).font(type_::SMALL).color(k.c.text_faint).pr(space::S1));
        }
        // On every row, so a card can go on the projector without being opened first.
        line = line.child(btn(t::BTN_SHOW_NOW()).icon(&icons::PLAY).icon_only().ghost().small().focusable().build(k, move |s, _| s.show_notice_now(id)));
        rows = rows.child(line.on_press(move |s: &mut App| s.pick_notice(id)));
    }
    if form.id == 0 && s.notice_draft().is_some() {
        let title = form.title.text.trim();
        let shown = if title.is_empty() { form.body.text.lines().next().unwrap_or_default().trim().to_string() } else { title.to_string() };
        rows = rows.child(
            row()
                .key("notice-new")
                .role("listitem")
                .label(shown.clone())
                .w_full()
                .h(size::ROW)
                .pl(space::S3)
                .pr(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.md)
                .fill(k.c.layer_03)
                .child(text((count + 1).to_string()).font(type_::SMALL).color(k.c.text_faint).w(16.0))
                .child(text(shown).font(type_::BODY_STRONG).color(k.c.text_base).truncate().grow(1.0))
                .child(rune::icon(&icons::PENCIL, 14.0).color(k.c.text_muted).help(t::ANNOUNCEMENT_NEW())),
        );
    } else if s.announcements.is_empty() {
        rows = rows.child(text(t::ANNOUNCEMENTS_EMPTY()).font(type_::BODY).color(k.c.text_muted).px(space::S3).py(space::S2));
    }
    let mut templates = row().key("notice-templates").w_full().gap(6.0).flex_wrap();
    for (at, &(title, body)) in t::ANNOUNCEMENT_TEMPLATES().iter().enumerate() {
        // The verse of the week reads its words from the Bible; the others are words to replace.
        templates = if at == PASSAGE_TEMPLATE {
            templates.child(btn(title).small().focusable().build(k, move |s, _| s.new_passage_notice(title)))
        } else {
            templates.child(btn(title).small().focusable().build(k, move |s, _| s.new_notice(title, body)))
        };
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
    let draft = s.notice_draft();
    let bible = s.bible().map(|bible| &**bible);
    let state = draft.as_ref().map(|draft| passage_state(draft, bible)).unwrap_or(PassageState::NotAPassage);
    let slides = draft.as_ref().map(|draft| s.card_slides(draft)).unwrap_or_default();
    tools = tools
        .child(spacer())
        .child(btn(t::BTN_SHOW_NOW()).icon(&icons::PLAY).disabled(slides.is_empty()).focusable().build(k, |s, _| s.show_form_notice_now()))
        .child(btn(t::BTN_SAVE()).primary().hint(t::KEY_SAVE()).focusable().build(k, |s, _| s.save_notice()));
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
    // A passage card with no words yet draws nothing; the words under the fields say why.
    let slide = draft.as_ref().and_then(|draft| match draft.fill {
        Fill::Passage => slides.first().cloned(),
        _ => Some(slides.first().cloned().unwrap_or_else(|| announcement_slide(draft))),
    });
    let shown_state = s.preview_state(slide);
    let preview = col()
        .key("notice-preview")
        .w_full()
        .gap(space::S1)
        .child(section_label(k, t::ANNOUNCEMENT_PREVIEW()))
        .child(el().size(shown, (shown * 9.0 / 16.0).round()).radius(k.radius.lg).clip_rounded(k.radius.lg).border(1.0, k.c.border_base).child(crate::ui::thumb(s, &shown_state, shown)));
    let fields = col()
        .key("notice-form")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .px(space::S5)
        .pb(space::S3)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(preview)
        .children(holds(s, k, draft.as_ref()))
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
        .child(if form.fill == Fill::Passage {
            let mut field = col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_PASSAGE())).child(
                TextInput::new(&form.passage)
                    .width(None)
                    .label(t::ANNOUNCEMENT_PASSAGE())
                    .placeholder(t::ANNOUNCEMENT_PASSAGE_HINT())
                    .on_edit(|s: &mut App| edit_of(s, |form| &mut form.passage))
                    .on_key(escape_closes)
                    .build(k),
            );
            // Without words the card says why, and the Bibles dialog is one press away.
            let problem = match state {
                PassageState::NoBible if !s.bible_is_loading() => Some(t::PASSAGE_NO_BIBLE()),
                PassageState::Unreadable if !form.passage.text.trim().is_empty() => Some(t::PASSAGE_UNREADABLE()),
                _ => None,
            };
            if let Some(problem) = problem {
                field = field.child(
                    row()
                        .w_full()
                        .gap(space::S2)
                        .center_y()
                        .child(text(problem).font(type_::SMALL).color(k.c.text_muted).wrap().grow(1.0))
                        .child(btn(t::CMD_BIBLES()).small().focusable().build(k, |s, _| s.open_bibles())),
                );
            }
            field
        } else if form.fill != Fill::Nothing {
            // A birthday card writes its own lines; there is nothing to type.
            el()
        } else {
            col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_BODY())).child(
                TextArea::new(&form.body)
                    .width(None)
                    .label(t::ANNOUNCEMENT_BODY())
                    .placeholder(t::ANNOUNCEMENT_BODY_HINT())
                    .auto_grow(4, 14)
                    .on_edit(|s: &mut App| edit_of(s, |form| &mut form.body))
                    .on_key(escape_closes)
                    .build(k),
            )
        })
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
            // Two rows of two, because four labels in one row cut the birthday choices short; the
            // choice that is not picked in a row shows none selected.
            col()
                .w_full()
                .gap(space::S1)
                .child(section_label(k, t::ANNOUNCEMENT_FILL()))
                .child(
                    SegmentedControl::new(&[t::FILL_NOTHING(), t::FILL_PASSAGE()], [Fill::Nothing, Fill::Passage].iter().position(|fill| *fill == form.fill))
                        .width(None)
                        .label(t::ANNOUNCEMENT_FILL())
                        .on_pick(|s: &mut App, picked, _: &mut Tx| s.set_notice_fill([Fill::Nothing, Fill::Passage][picked.min(1)]))
                        .build(k),
                )
                .child(
                    SegmentedControl::new(&[t::FILL_WEEK(), t::FILL_MONTH()], [Fill::BirthdaysWeek, Fill::BirthdaysMonth].iter().position(|fill| *fill == form.fill))
                        .width(None)
                        .label(t::ANNOUNCEMENT_FILL())
                        .on_pick(|s: &mut App, picked, _: &mut Tx| s.set_notice_fill([Fill::BirthdaysWeek, Fill::BirthdaysMonth][picked.min(1)]))
                        .build(k),
                ),
        )

        .child(picture)
        .scroll_bar(k);
    // The two buttons that finish the form sit under it and never scroll away: the tester's
    // Save was half off the bottom edge of a laptop screen (2026-11-11).
    col().grow(1.0).min_w(0.0).h_full().min_h(0.0).child(fields).child(crate::ui::widgets::divider(k)).child(tools.px(space::S5).py(space::S3))
}

/// For a birthday card: the days it covers and how many names it holds today.
fn holds(s: &App, k: &Kit, draft: Option<&artosia_core::deck::Announcement>) -> Option<El> {
    let draft = draft?;
    let today = s.today();
    let ((from_month, from_day), (to_month, to_day)) = birthday_days(draft.fill, &today)?;
    let covers = if draft.fill == Fill::BirthdaysMonth { t::birthdays_cover_month(from_month) } else { t::birthdays_cover_week((from_month, from_day), (to_month, to_day)) };
    let names = birthdays(&s.people, draft.fill, &today).len();
    Some(
        col()
            .key("notice-holds")
            .w_full()
            .child(text(covers).font(type_::SMALL).color(k.c.text_muted))
            .child(text(t::names_today(names)).font(type_::SMALL).color(k.c.text_muted)),
    )
}

pub fn view(s: &App, k: &Kit, width: f32) -> El {
    let Some(notices) = s.notices.as_ref() else { return el() };
    let in_plan = s.announcements_in_plan();
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
        .child(btn(if in_plan { t::IN_PLAN() } else { t::BTN_ADD_TO_PLAN() }).icon(&icons::PLUS).disabled(in_plan || s.announcements.is_empty()).focusable().build(k, |s, _| s.add_announcements_to_plan()))
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
