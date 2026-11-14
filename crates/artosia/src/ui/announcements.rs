//! The announcement editor: the list on the left, the one being written in the middle, and
//! how it will look on the projector on the right, changing as you type.

use artosia_core::calendar::Date;
use artosia_core::deck::{Fill, NoticeStyle, PassageState, Template, passage_state};
use artosia_core::window::{Range, Window};
use artosia_slides::look::rgb;
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar, SegmentedControl, TextArea, TextInput};

use crate::calendar::date_words;
use crate::state::{App, Edge, Notices};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::calendar;
use crate::ui::colour::ColourFor;
use crate::ui::templates::{empty_note, fill_words, glyph, read_line, reads, slide_or_note, style_name};
use crate::ui::widgets::{btn, enter_key, section_label};

const LIST: f32 = 330.0;
/// The widest the preview is drawn.
const PREVIEW: f32 = 340.0;
/// The width of one date picker of a custom range.
const PICKER: f32 = 190.0;
/// How long an announcement may stay up, in seconds.
const SECONDS: [u32; 6] = [5, 8, 10, 15, 20, 30];

fn escape_closes(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    // From any field, Ctrl and an arrow goes to the card before or after, as Ctrl moves the
    // highlight in Preview. A keys-only tester could edit only the first card.
    if (ev.mods.ctrl || ev.mods.cmd) && matches!(ev.key, Key::Up | Key::Down) {
        s.step_notice(if ev.key == Key::Up { -1 } else { 1 });
        return true;
    }
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

/// A card's name in the list: its title, or its first line, or Picture for a card that is a
/// picture alone (a slide of only a picture imported from PowerPoint had no name at all). The
/// card in the form reads what is being typed, so the list never shows an old name beside the new one.
fn shown_title(item: &artosia_core::deck::Announcement, form: Option<&Notices>) -> String {
    let (title, body) = match form {
        Some(form) => (form.title.text.as_str(), form.body.text.as_str()),
        None => (item.title.as_str(), item.body.as_str()),
    };
    let first = if title.trim().is_empty() { body.lines().next().unwrap_or_default().trim() } else { title.trim() };
    if first.is_empty() && !item.image.is_empty() { t::ANNOUNCEMENT_PICTURE().to_string() } else { first.to_string() }
}

fn list(s: &App, k: &Kit, form: &Notices) -> El {
    // One Tab stop: the arrows move through the cards and open the one they land on, Alt and
    // an arrow moves the card, Delete removes it. The buttons on a row are for the pointer;
    // with a Tab stop each, the tester needed fourteen presses to get past four cards.
    let mut rows = col().key("notice-rows").role("list").label(t::CMD_ANNOUNCEMENTS()).w_full().gap(2.0).focus_group().focus_follows();
    let count = s.announcements.len();
    for (at, item) in s.announcements.iter().enumerate() {
        let id = item.id;
        let picked = form.id == id;
        let mut line = row()
            .key(format!("notice-{id}"))
            .role("listitem")
            .label(shown_title(item, None))
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
            .focusable(true)
            .focus_selected(picked)
            .on_key(move |s: &mut App, ev: &rune::KeyEvent, _: &mut Tx| match &ev.key {
                rune::Key::Up if ev.mods.alt => {
                    s.move_notice(id, -1);
                    true
                }
                rune::Key::Down if ev.mods.alt => {
                    s.move_notice(id, 1);
                    true
                }
                rune::Key::Delete => {
                    s.delete_announcement(id);
                    true
                }
                _ => false,
            })
            .child(text(shown_title(item, picked.then_some(form))).font(if picked { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0));
        if picked {
            // The picked row carries its own tools, so nothing hides behind a hover.
            line = line
                .child(btn(t::PLAN_MOVE_UP()).icon(&icons::ARROW_UP).icon_only().ghost().small().disabled(at == 0).build(k, move |s, _| s.move_notice(id, -1)))
                .child(btn(t::PLAN_MOVE_DOWN()).icon(&icons::ARROW_DOWN).icon_only().ghost().small().disabled(at + 1 >= count).build(k, move |s, _| s.move_notice(id, 1)))
                .child(btn(t::BTN_DUPLICATE()).icon(&icons::COPY).icon_only().ghost().small().build(k, move |s, _| s.duplicate_notice(id)))
                .child(btn(t::BTN_DELETE()).icon(&icons::TRASH).icon_only().ghost().small().build(k, move |s, _| s.delete_announcement(id)));
        } else {
            line = line.child(text(t::seconds_short(item.seconds)).font(type_::SMALL).color(k.c.text_faint).pr(space::S1));
        }
        // On every row, so a card can go on the projector without being opened first.
        line = line.child(btn(t::BTN_SHOW_NOW()).icon(&icons::PLAY).icon_only().ghost().small().build(k, move |s, _| s.show_notice_now(id)));
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
    col()
        .key("notice-list")
        .w(LIST)
        .h_full()
        .min_h(0.0)
        .px(space::S3)
        .pb(space::S4)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE)
        .child(btn(t::ANNOUNCEMENT_NEW()).icon(&icons::PLUS).hint("Ctrl N").focusable().build(k, |s, _| s.new_notice("", "")).key("notice-add"))
        .child(crate::ui::tour::help_mark(s, k, crate::guide::lessons::Feature::Notices))
        .child(rows)
        .child(row().w_full().gap(space::S2).center_y().child(text(t::announcements(s.announcements.len())).font(type_::SMALL).color(k.c.text_faint).px(space::S3).grow(1.0)).child(el().pr(space::S2).child(crate::ui::widgets::keycap(k, "Ctrl Up Down", false))))
        .child(templates_block(s, k, form))
        .child(
            col()
                .w_full()
                .gap(space::S2)
                .child(section_label(k, t::ANNOUNCEMENT_BRING_IN()))
                .child(btn(t::CMD_IMPORT_PPTX()).icon(&icons::IMPORT).small().focusable().anchor(crate::palette::Command::ImportPptx, crate::guide::Place::Panel).build(k, |s, _| s.want_pptx = true))
                .child(btn(t::CMD_IMPORT_BIRTHDAYS()).icon(&icons::IMPORT).small().focusable().build(k, |s, _| s.want_birthdays = true))
                .child(text(t::birthdays_on_file(s.people.len())).font(type_::SMALL).color(k.c.text_faint)),
        )
        .scroll_bar(k)
}

/// The templates of a card, as rows: the one the card is has its name in bold and its icon filled.
fn templates_block(s: &App, k: &Kit, form: &Notices) -> El {
    let sf = theme::surf();
    let current = form.fill.template();
    // One Tab stop: the arrows move among the templates and pick as they go.
    let mut rows = col().key("notice-templates").role("toolbar").label(t::ANNOUNCEMENT_START_FROM()).w_full().gap(2.0).focus_group().focus_follows();
    for template in Template::ALL {
        let on = current == template;
        rows = rows.child(
            row()
                .key(format!("notice-template-{template:?}"))
                .help(fill_words(if on { form.fill } else { template.fill() }, &if on { form.window } else { Window::default() }).unwrap_or_default())
                .role("button")
                .label(crate::templates::name(template))
                .checked(on)
                .w_full()
                .h(52.0)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(if on { theme::accent().alpha(0.18) } else { rune::Color::TRANSPARENT })
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .focus_selected(on)
                .on_key(enter_key)
                .child(row().size(32.0, 32.0).center().radius(8.0).fill(if on { theme::accent() } else { sf.header }).child(icon(glyph(template), 16.0).color(if on { rune::Color::WHITE } else { k.c.text_muted }).a11y_hidden()))
                .child(
                    col()
                        .grow(1.0)
                        .min_w(0.0)
                        .child(text(crate::templates::name(template)).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().a11y_hidden())
                        .child(text(reads(template)).font(type_::SMALL).color(k.c.text_muted).truncate().a11y_hidden()),
                )
                .on_press(move |s: &mut App| s.set_notice_template(template)),
        );
    }
    let style = form.style.unwrap_or(s.look().notice_style);
    let names: Vec<&str> = NoticeStyle::ALL.iter().map(|style| style_name(*style)).collect();
    col()
        .w_full()
        .gap(space::S2)
        .child(section_label(k, t::TPL_HEADING()))
        .child(rows)
        .child(section_label(k, t::TPL_STYLE()))
        // Seven names do not fit one row of the column, so they wrap as chips.
        .child(choice_chips(k, "notice-style", t::TPL_STYLE(), &names, NoticeStyle::ALL.iter().position(|own| *own == style), |s, at| s.set_notice_style(NoticeStyle::ALL[at.min(NoticeStyle::ALL.len() - 1)])))
        .child(btn(t::TPL_SEE_ALL()).icon(&icons::ARROW_RIGHT).small().focusable().build(k, |s, _| s.open_tab(crate::state::Tab::Templates)))
}

/// What the card reads, as the pill beside the slide: a warning colour while the window is empty.
fn reads_pill(s: &App, draft: Option<&artosia_core::deck::Announcement>, empty: bool) -> Option<El> {
    let draft = draft.filter(|draft| draft.fill != Fill::Nothing)?;
    let ink = if empty { theme::warn_ink() } else { theme::accent() };
    Some(
        row()
            .key("notice-reads")
            .h(24.0)
            .px(space::S2)
            .gap(6.0)
            .center_y()
            .radius(12.0)
            .fill(if empty { theme::warn().alpha(0.2) } else { theme::accent().alpha(0.16) })
            .child(icon(glyph(draft.fill.template()), 14.0).color(ink).a11y_hidden())
            .child(text(read_line(s, draft)).font(type_::SMALL_STRONG).color(ink)),
    )
}

/// The calendars a schedule card reads, as chips in the calendars' own colours.
fn calendar_chips(s: &App, k: &Kit, form: &Notices) -> El {
    let mut chips = row().key("notice-calendars").role("group").label(t::NOTICE_CALENDARS()).w_full().gap(space::S1).flex_wrap();
    for calendar in &s.calendars {
        let id = calendar.id;
        let on = form.calendars.is_empty() || form.calendars.contains(&id);
        let colour = rgb(calendar.colour);
        let chip = row()
            .key(format!("notice-calendar-{id}"))
            .role("checkbox")
            .label(calendar.name.clone())
            .checked(on)
            .h(26.0)
            .px(space::S2)
            .gap(6.0)
            .center_y()
            .radius(13.0)
            .cursor(Cursor::Pointer)
            .focusable(true)
            .on_key(enter_key);
        let chip = if on {
            chip.fill(theme::tint_of(colour, false)).child(icon(&icons::CHECK, 12.0).color(theme::ink_of(colour)).a11y_hidden()).child(text(calendar.name.clone()).font(type_::SMALL_STRONG).color(theme::ink_of(colour)).a11y_hidden())
        } else {
            chip.border(1.0, k.c.border_base).child(crate::ui::calendar::dot(calendar.colour, 10.0)).child(text(calendar.name.clone()).font(type_::SMALL).color(k.c.text_base).a11y_hidden())
        };
        chips = chips.child(chip.on_press(move |s: &mut App| s.toggle_notice_calendar(id)));
    }
    col().w_full().gap(space::S1).child(section_label(k, t::NOTICE_CALENDARS())).child(chips)
}

/// The quick ranges of a window, in the order of their chips; Custom is the chip after them.
const QUICK: [Range; 7] = [Range::Coming(7), Range::Coming(30), Range::Coming(90), Range::Week(0), Range::Week(1), Range::Month(0), Range::Month(1)];
/// What can stand out, in the order of its chips; Custom is the chip after them.
const HIGHLIGHTS: [Option<Range>; 4] = [None, Some(Range::Past(7)), Some(Range::Coming(7)), Some(Range::Coming(1))];

/// The Custom chip's range: its dates are the days the card reads now, which the action puts in.
const CUSTOM: Range = Range::Custom(Date { year: 1970, month: 1, day: 1 }, Date { year: 1970, month: 1, day: 1 });

/// A row of choices that wraps, one lit: the chip's key is `{key}-{place}`.
fn choice_chips(k: &Kit, key: &str, label: &str, names: &[&str], on: Option<usize>, press: fn(&mut App, usize)) -> El {
    let mut chips = row().key(key.to_string()).role("radiogroup").label(label.to_string()).w_full().gap(space::S1).flex_wrap().focus_group();
    for (at, name) in names.iter().enumerate() {
        let lit = on == Some(at);
        let chip = row()
            .key(format!("{key}-{at}"))
            .role("radio")
            .label(name.to_string())
            .checked(lit)
            .h(30.0)
            .px(space::S3)
            .center()
            .radius(k.radius.md)
            .cursor(Cursor::Pointer)
            .focusable(true)
            .on_key(enter_key);
        let chip = if lit {
            chip.fill(theme::accent()).child(text(name.to_string()).font(type_::BODY_STRONG).color(rune::Color::WHITE).a11y_hidden())
        } else {
            chip.fill(k.c.layer_02).border(1.0, k.c.border_base).hover(k.c.hover).child(text(name.to_string()).font(type_::BODY).color(k.c.text_base).a11y_hidden())
        };
        chips = chips.child(chip.on_press(move |s: &mut App| press(s, at)));
    }
    chips
}

/// The two dates of a custom range or highlight, as pickers, with the month under the open one.
fn date_pickers(s: &App, k: &Kit, form: &Notices, edges: (Edge, Edge), dates: (Date, Date), within: (Date, Date)) -> El {
    let key = |edge: Edge| match edge {
        Edge::RangeFrom => "notice-range-from",
        Edge::RangeTo => "notice-range-to",
        Edge::HighlightFrom => "notice-highlight-from",
        Edge::HighlightTo => "notice-highlight-to",
    };
    let one = |edge: Edge, name: &str, date: Date| {
        let open = form.pick.is_some_and(|(at, _)| at == edge);
        calendar::field(k, name, calendar::picker(k, key(edge), name, None, date_words(date), open).w(PICKER).on_press(move |s: &mut App| s.notice_open_pick(edge)))
    };
    let mut both = col().w_full().gap(space::S2).child(row().w_full().gap(space::S2).child(one(edges.0, t::NOTICE_FROM(), dates.0)).child(one(edges.1, t::NOTICE_TO(), dates.1)));
    if let Some((edge, shown)) = form.pick.filter(|(edge, _)| *edge == edges.0 || *edge == edges.1) {
        let sf = theme::surf();
        let chosen = if edge == edges.0 { dates.0 } else { dates.1 };
        let grid = calendar::month_grid(s, k, "notice", shown, move |date| date == chosen, move |date| date >= within.0 && date <= within.1, |_| true, |s, by| s.notice_pick_month(by), |s, date| s.notice_pick_date(date), el());
        both = both.child(col().key("notice-picker").w(2.0 * PICKER + space::S2).padding(space::S3).radius(k.radius.md).fill(sf.raised).border(1.0, sf.edge).child(grid));
    }
    both
}

/// The days a schedule or birthday card reads: the quick ranges, two dates for a custom one, and for
/// birthdays what stands out, with the dates they come to and a word when a choice was cut to fit.
fn window_panel(s: &App, k: &Kit, form: &Notices, window: Window, birthdays: bool) -> El {
    let shown = s.today().shown(&window);
    let range = form.window.range;
    let names = [t::NOTICE_WEEKLY(), t::NOTICE_MONTHLY(), t::NOTICE_QUARTERLY(), t::NOTICE_WEEK(), t::NOTICE_NEXT_WEEK(), t::NOTICE_MONTH(), t::NOTICE_NEXT_MONTH(), t::NOTICE_CUSTOM()];
    let place = QUICK.iter().position(|quick| *quick == range).or(matches!(range, Range::Custom(..)).then_some(QUICK.len()));
    let mut panel = col().key("notice-window").w_full().gap(space::S2).child(section_label(k, t::NOTICE_WINDOW())).child(choice_chips(
        k,
        "notice-range",
        t::NOTICE_WINDOW(),
        &names,
        place,
        |s, at| s.set_notice_range(QUICK.get(at).copied().unwrap_or(CUSTOM)),
    ));
    if let Range::Custom(..) = range {
        panel = panel.child(date_pickers(s, k, form, (Edge::RangeFrom, Edge::RangeTo), (shown.from, shown.to), (shown.from, shown.to)));
    }
    if birthdays {
        let marks = [t::NOTICE_HL_NONE(), t::NOTICE_HL_PAST(), t::NOTICE_HL_COMING(), t::NOTICE_HL_TODAY(), t::NOTICE_CUSTOM()];
        let lit = HIGHLIGHTS.iter().position(|own| *own == form.window.highlight).or(matches!(form.window.highlight, Some(Range::Custom(..))).then_some(HIGHLIGHTS.len()));
        panel = panel.child(el().h(space::S1)).child(section_label(k, t::NOTICE_HIGHLIGHT())).child(choice_chips(k, "notice-highlight", t::NOTICE_HIGHLIGHT(), &marks, lit, |s, at| {
            s.set_notice_highlight(HIGHLIGHTS.get(at).copied().unwrap_or(Some(CUSTOM)))
        }));
        if let Some(Range::Custom(a, b)) = form.window.highlight {
            panel = panel.child(date_pickers(s, k, form, (Edge::HighlightFrom, Edge::HighlightTo), (a.min(b), a.max(b)), shown.highlight.unwrap_or((a.min(b), a.max(b)))));
        }
    }
    let covers = if shown.from.day == 1 && shown.from.plus_months(1).plus_days(-1) == shown.to {
        t::birthdays_cover_month(shown.from.month)
    } else {
        t::birthdays_cover_week((shown.from.month, shown.from.day), (shown.to.month, shown.to.day))
    };
    let mut notes = col().key("notice-covers").w_full().child(text(covers).font(type_::SMALL).color(k.c.text_muted));
    if shown.clamped {
        notes = notes.child(crate::ui::widgets::wrapped(t::NOTICE_CUT()).font(type_::SMALL).color(theme::warn_ink()).wrap());
    }
    panel.child(notes)
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
    let image2_file = std::path::Path::new(&form.image2).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let mut picture2 = row()
        .w_full()
        .h(36.0)
        .gap(space::S2)
        .center_y()
        .child(text(t::ANNOUNCEMENT_PICTURE2()).font(type_::BODY).color(k.c.text_base))
        .child(text(image2_file).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0))
        .child(btn(t::LOOK_CHOOSE_LOGO()).icon(&icons::IMAGE).small().focusable().build(k, |s, _| s.want_notice_image2 = true));
    if !form.image2.is_empty() {
        picture2 = picture2.child(btn(t::BTN_REMOVE_PICTURE()).small().ghost().focusable().build(k, |s, _| s.set_notice_image2(None)));
    }
    let qris_file = std::path::Path::new(&form.qris).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let mut qris = row()
        .w_full()
        .h(36.0)
        .gap(space::S2)
        .center_y()
        .child(text(t::ANNOUNCEMENT_QRIS()).font(type_::BODY).color(k.c.text_base))
        .child(text(qris_file).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0))
        .child(btn(t::LOOK_CHOOSE_LOGO()).icon(&icons::IMAGE).small().focusable().build(k, |s, _| s.want_notice_qris = true));
    if !form.qris.is_empty() {
        qris = qris.child(btn(t::BTN_REMOVE_PICTURE()).small().ghost().focusable().build(k, |s, _| s.set_notice_qris(None)));
    }
    // How it will look, drawn from what is in the form now; an empty window says so in its place.
    let shown = (width - 2.0 * space::S5).clamp(200.0, PREVIEW);
    let card = draft.clone().unwrap_or_default();
    let empty = draft.is_some() && empty_note(s, &card, slides.len()).is_some();
    let template = form.fill.template();
    let preview = col()
        .key("notice-preview")
        .w_full()
        .gap(space::S1)
        // A passage or a long list of birthdays takes several slides; the sample is the first, and says of how many.
        .child(row().w_full().gap(space::S2).center_y().child(section_label(k, t::ANNOUNCEMENT_PREVIEW())).child(spacer()).children((slides.len() > 1).then(|| text(t::slides(slides.len())).font(type_::SMALL).color(k.c.text_muted))))
        .child(if draft.is_some() { slide_or_note(s, k, &card, shown) } else { el().size(shown, (shown * 9.0 / 16.0).round()).radius(k.radius.lg).border(1.0, k.c.border_base) });
    let head = row()
        .w_full()
        .gap(space::S3)
        .center_y()
        .child(text(crate::templates::name(template)).font(type_::HEADING).color(k.c.text_base).truncate().min_w(0.0))
        .child(spacer())
        .children(reads_pill(s, draft.as_ref(), empty));
    let words = match template {
        Template::Passage => {
            // The passage is picked from the books, chapters and verses, never typed.
            let chosen = form.passage.text.trim().to_string();
            let words = if chosen.is_empty() { t::START_BIBLE_DETAIL().to_string() } else { chosen };
            let mut field = col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_PASSAGE())).child(
                btn(words).icon(&icons::BOOK).focusable().build(k, |s, _| s.pick_notice_passage()).key("notice-passage"),
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
                        .child(crate::ui::widgets::wrapped(problem).font(type_::SMALL).color(k.c.text_muted).wrap().grow(1.0))
                        .child(btn(t::BIBLES_MORE()).small().focusable().build(k, |s, _| s.open_bibles())),
                );
            }
            Some(field)
        }
        // Typed words are the only free text there is.
        Template::Plain => Some(
            col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_BODY())).child(
                TextArea::new(&form.body)
                    .width(None)
                    .label(t::ANNOUNCEMENT_BODY())
                    .placeholder(t::ANNOUNCEMENT_BODY_HINT())
                    .auto_grow(4, 14)
                    .on_edit(|s: &mut App| edit_of(s, |form| &mut form.body))
                    .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| crate::ui::widgets::enter_breaks_line(ev, tx) || escape_closes(s, ev, tx))
                    .build(k),
            ),
        ),
        _ => None,
    };
    let window = match template {
        Template::Schedule | Template::Birthdays => Some(window_panel(s, k, form, draft.as_ref().map_or(form.window, |draft| draft.window), template == Template::Birthdays)),
        _ => None,
    };
    let fields = col()
        .key("notice-form")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .px(space::S5)
        .pb(space::S3)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE)
        .child(head)
        .child(preview)
        .child(section_label(k, t::ANNOUNCEMENT_FILL()))
        .children(holds(s, k, draft.as_ref()))
        .children((template == Template::Schedule).then(|| calendar_chips(s, k, form)))
        .children(window)
        .children(words)
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
        .children(crate::ui::colour::rows(s, k, ColourFor::NoticeTitle, t::NOTICE_TITLE_COLOUR(), &crate::ui::look::text_colours()))
        .child(picture)
        .children((!form.image.is_empty()).then_some(picture2))
        .children((form.fill == Fill::Transfer).then_some(qris))
        .scroll_bar(k);
    // The two buttons that finish the form sit under it and never scroll away: the tester's
    // Save was half off the bottom edge of a laptop screen (2026-11-11).
    col().grow(1.0).min_w(0.0).h_full().min_h(0.0).child(fields).child(crate::ui::widgets::divider(k)).child(tools.px(space::S5).py(space::S3))
}

/// What the card's window holds today, in a grey line: the last date of the offerings, the
/// account's bank, or the ask for a list of birthdays; the days a card reads are in its window panel.
fn holds(s: &App, k: &Kit, draft: Option<&artosia_core::deck::Announcement>) -> Option<El> {
    let draft = draft?;
    let note = |words: String| Some(col().key("notice-holds").w_full().child(text(words).font(type_::SMALL).color(k.c.text_muted)));
    match draft.fill.template() {
        Template::Schedule | Template::Plain | Template::Passage => None,
        // The money cards write their lines from the Offerings window: what they hold is said once.
        Template::Offerings => note(artosia_core::giving::latest(&s.gifts).map_or_else(|| t::GIVING_EMPTY().to_string(), |date| t::giving_latest(&crate::calendar::date_words(date)))),
        Template::Transfer => note(if s.account.ready() { s.account.bank.clone() } else { t::GIVING_ACCOUNT_NONE().to_string() }),
        // With no list on file the card can hold nobody, so the list is asked for here.
        Template::Birthdays if s.people.is_empty() => Some(
            row()
                .key("notice-holds")
                .w_full()
                .gap(space::S2)
                .center_y()
                .child(text(t::birthdays_on_file(0)).font(type_::BODY).color(k.c.text_base).grow(1.0))
                .child(btn(t::CMD_IMPORT_BIRTHDAYS()).icon(&icons::IMPORT).small().focusable().build(k, |s, _| s.want_birthdays = true)),
        ),
        Template::Birthdays => None,
    }
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
