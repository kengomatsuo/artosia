//! The announcement editor: the list on the left, the one being written in the middle, and
//! how it will look on the projector on the right, changing as you type.

use artosia_core::calendar::Date;
use artosia_core::deck::{Fill, NoticeStyle, PassageState, Template, passage_state};
use artosia_core::window::{Range, Window};
use artosia_slides::look::rgb;
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar};

use crate::calendar::date_words;
use crate::state::{App, Edge, Notices};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::calendar;
use crate::ui::templates::{empty_note, fill_words, glyph, read_line, reads, style_name};
use crate::ui::slide_edit;
use crate::ui::widgets::{btn, enter_key, section_label};

const LIST: f32 = 330.0;
/// The width of the rail the list folds into while a card is open on its slide (D1).
const RAIL: f32 = 56.0;
/// The width of one date picker of a custom range.
const PICKER: f32 = 190.0;
/// How long an announcement may stay up, in seconds.
pub(super) const SECONDS: [u32; 6] = [5, 8, 10, 15, 20, 30];

pub(super) fn escape_closes(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
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
pub(super) fn edit_of(s: &mut App, pick: fn(&mut Notices) -> &mut rune::TextEdit) -> &mut rune::TextEdit {
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
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(btn(t::ANNOUNCEMENT_NEW()).icon(&icons::PLUS).hint("Ctrl N").focusable().grow().build(k, |s, _| s.new_notice("", "")).key("notice-add"))
                .child(btn(t::EDIT_RAIL_CLOSE()).icon(&icons::CHEV_LEFT).icon_only().focusable().build(k, |s, _| s.notice_list(false)).key("notice-list-fold")),
        )
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
pub(super) fn templates_block(s: &App, k: &Kit, form: &Notices) -> El {
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
        .child(choice_chips(k, "notice-style", t::TPL_STYLE(), &names, NoticeStyle::ALL.iter().position(|own| *own == style), 30.0, |s, at| s.set_notice_style(NoticeStyle::ALL[at.min(NoticeStyle::ALL.len() - 1)])))
        .child(btn(t::TPL_SEE_ALL()).icon(&icons::ARROW_RIGHT).small().focusable().build(k, |s, _| s.open_tab(crate::state::Tab::Templates)))
}

/// What the card reads, as the pill beside the slide: a warning colour while the window is empty.
pub(super) fn reads_pill(s: &App, draft: Option<&artosia_core::deck::Announcement>, empty: bool) -> Option<El> {
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
pub(super) fn calendar_chips(s: &App, k: &Kit, form: &Notices) -> El {
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
            .h(24.0)
            .px(space::S2)
            .gap(6.0)
            .center_y()
            .radius(12.0)
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
fn choice_chips(k: &Kit, key: &str, label: &str, names: &[&str], on: Option<usize>, height: f32, press: fn(&mut App, usize)) -> El {
    let mut chips = row().key(key.to_string()).role("radiogroup").label(label.to_string()).w_full().gap(space::S1).flex_wrap().focus_group();
    for (at, name) in names.iter().enumerate() {
        let lit = on == Some(at);
        let chip = row()
            .key(format!("{key}-{at}"))
            .role("radio")
            .label(name.to_string())
            .checked(lit)
            .h(height)
            .px(space::S3)
            .center()
            .radius(if height < 30.0 { 6.0 } else { k.radius.md })
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
pub(super) fn window_panel(s: &App, k: &Kit, form: &Notices, window: Window, birthdays: bool) -> El {
    let shown = s.today().shown(&window);
    let range = form.window.range;
    let names = [t::NOTICE_WEEKLY(), t::NOTICE_MONTHLY(), t::NOTICE_QUARTERLY(), t::NOTICE_WEEK(), t::NOTICE_NEXT_WEEK(), t::NOTICE_MONTH(), t::NOTICE_NEXT_MONTH(), t::NOTICE_CUSTOM()];
    let place = QUICK.iter().position(|quick| *quick == range).or(matches!(range, Range::Custom(..)).then_some(QUICK.len()));
    let covers = if shown.from.day == 1 && shown.from.plus_months(1).plus_days(-1) == shown.to {
        t::birthdays_cover_month(shown.from.month)
    } else {
        t::birthdays_cover_week((shown.from.month, shown.from.day), (shown.to.month, shown.to.day))
    };
    // The days the card covers are said on the label's own line, as the mock sets them.
    let label = row().key("notice-covers").w_full().gap(space::S2).center_y().child(section_label(k, t::NOTICE_WINDOW())).child(spacer()).child(text(covers).font(type_::SMALL).color(k.c.text_muted));
    let mut panel = col().key("notice-window").w_full().gap(space::S1).child(label).child(choice_chips(
        k,
        "notice-range",
        t::NOTICE_WINDOW(),
        &names,
        place,
        24.0,
        |s, at| s.set_notice_range(QUICK.get(at).copied().unwrap_or(CUSTOM)),
    ));
    if let Range::Custom(..) = range {
        panel = panel.child(date_pickers(s, k, form, (Edge::RangeFrom, Edge::RangeTo), (shown.from, shown.to), (shown.from, shown.to)));
    }
    if birthdays {
        let marks = [t::NOTICE_HL_NONE(), t::NOTICE_HL_PAST(), t::NOTICE_HL_COMING(), t::NOTICE_HL_TODAY(), t::NOTICE_CUSTOM()];
        let lit = HIGHLIGHTS.iter().position(|own| *own == form.window.highlight).or(matches!(form.window.highlight, Some(Range::Custom(..))).then_some(HIGHLIGHTS.len()));
        panel = panel.child(el().h(space::S1)).child(section_label(k, t::NOTICE_HIGHLIGHT())).child(choice_chips(k, "notice-highlight", t::NOTICE_HIGHLIGHT(), &marks, lit, 24.0, |s, at| {
            s.set_notice_highlight(HIGHLIGHTS.get(at).copied().unwrap_or(Some(CUSTOM)))
        }));
        if let Some(Range::Custom(a, b)) = form.window.highlight {
            panel = panel.child(date_pickers(s, k, form, (Edge::HighlightFrom, Edge::HighlightTo), (a.min(b), a.max(b)), shown.highlight.unwrap_or((a.min(b), a.max(b)))));
        }
    }
    if shown.clamped {
        panel = panel.child(crate::ui::widgets::wrapped(t::NOTICE_CUT()).font(type_::SMALL).color(theme::warn_ink()).wrap());
    }
    panel
}

/// The card's template and style as two rows of chips, for a sheet that has no room for the rows of the list.
pub(super) fn template_chips(s: &App, k: &Kit, form: &Notices) -> El {
    let current = Template::ALL.iter().position(|own| *own == form.fill.template());
    let names: Vec<&str> = Template::ALL.iter().map(|own| crate::templates::name(*own)).collect();
    let style = form.style.unwrap_or(s.look().notice_style);
    let styles: Vec<&str> = NoticeStyle::ALL.iter().map(|own| style_name(*own)).collect();
    col()
        .w_full()
        .gap(space::S1)
        .child(section_label(k, t::TPL_HEADING()))
        .child(choice_chips(k, "notice-template", t::TPL_HEADING(), &names, current, 24.0, |s, at| s.set_notice_template(Template::ALL[at.min(Template::ALL.len() - 1)])))
        .child(el().h(space::S1))
        .child(section_label(k, t::TPL_STYLE()))
        .child(choice_chips(k, "notice-style", t::TPL_STYLE(), &styles, NoticeStyle::ALL.iter().position(|own| *own == style), 24.0, |s, at| s.set_notice_style(NoticeStyle::ALL[at.min(NoticeStyle::ALL.len() - 1)])))
}

/// The passage of a card is picked from the books, chapters and verses, never typed.
pub(super) fn passage_field(s: &App, k: &Kit, form: &Notices) -> El {
    let draft = s.notice_draft();
    let bible = s.bible().map(|bible| &**bible);
    let state = draft.as_ref().map(|draft| passage_state(draft, bible)).unwrap_or(PassageState::NotAPassage);
    let chosen = form.passage.text.trim().to_string();
    let words = if chosen.is_empty() { t::START_BIBLE_DETAIL().to_string() } else { chosen };
    let mut field = col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_PASSAGE())).child(btn(words).icon(&icons::BOOK).focusable().build(k, |s, _| s.pick_notice_passage()).key("notice-passage"));
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
    field
}

/// The picture rows of a card: its picture, a second one once it has the first, and the QRIS of a transfer.
pub(super) fn picture_rows(k: &Kit, form: &Notices) -> Vec<El> {
    let file_of = |path: &str| std::path::Path::new(path).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
    let line = |label: &str, path: &str, choose: fn(&mut App), remove: fn(&mut App)| {
        let mut line = row()
            .w_full()
            .h(36.0)
            .gap(space::S2)
            .center_y()
            .child(text(label.to_string()).font(type_::BODY).color(k.c.text_base))
            .child(text(file_of(path)).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0))
            .child(btn(t::LOOK_CHOOSE_LOGO()).icon(&icons::IMAGE).small().focusable().build(k, move |s, _| choose(s)));
        if !path.is_empty() {
            line = line.child(btn(t::BTN_REMOVE_PICTURE()).small().ghost().focusable().build(k, move |s, _| remove(s)));
        }
        line
    };
    let mut rows = vec![line(t::ANNOUNCEMENT_PICTURE(), &form.image, |s| s.want_notice_image = true, |s| s.set_notice_image(None))];
    if !form.image.is_empty() {
        rows.push(line(t::ANNOUNCEMENT_PICTURE2(), &form.image2, |s| s.want_notice_image2 = true, |s| s.set_notice_image2(None)));
    }
    if form.fill == Fill::Transfer {
        rows.push(line(t::ANNOUNCEMENT_QRIS(), &form.qris, |s| s.want_notice_qris = true, |s| s.set_notice_qris(None)));
    }
    rows
}

/// What the card's window holds today, in a grey line: the last date of the offerings, the
/// account's bank, or the ask for a list of birthdays; the days a card reads are in its window panel.
pub(super) fn holds(s: &App, k: &Kit, draft: Option<&artosia_core::deck::Announcement>) -> Option<El> {
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


/// The cards as numbers in a rail, the open one lit; the chevron opens the list over it.
fn rail(s: &App, k: &Kit, form: &Notices) -> El {
    let sf = theme::surf();
    let mut numbers = col().key("notice-rail-rows").role("list").label(t::CMD_ANNOUNCEMENTS()).w_full().gap(space::S1).items_center().focus_group().focus_follows();
    for (at, item) in s.announcements.iter().enumerate() {
        let id = item.id;
        let picked = form.id == id;
        numbers = numbers.child(
            row()
                .key(format!("notice-{id}"))
                .role("listitem")
                .label(shown_title(item, None))
                .w(36.0)
                .h(34.0)
                .center()
                .radius(8.0)
                .fill(if picked { k.c.layer_03 } else { rune::Color::TRANSPARENT })
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .focus_selected(picked)
                .on_key(enter_key)
                .child(text((at + 1).to_string()).font(if picked { type_::BODY_STRONG } else { type_::BODY }).color(if picked { k.c.text_base } else { k.c.text_muted }).a11y_hidden())
                .on_press(move |s: &mut App| s.pick_notice(id)),
        );
    }
    // A card not saved yet stands after the last one, lit, until it is.
    if form.id == 0 && s.notice_draft().is_some() {
        numbers = numbers.child(row().key("notice-new").role("listitem").label(shown_title(&artosia_core::deck::Announcement::default(), Some(form))).w(36.0).h(34.0).center().radius(8.0).fill(k.c.layer_03).child(text((s.announcements.len() + 1).to_string()).font(type_::BODY_STRONG).color(k.c.text_base).a11y_hidden()));
    }
    let new = row()
        .key("notice-add")
        .role("button")
        .label(t::ANNOUNCEMENT_NEW())
        .w(36.0)
        .h(34.0)
        .center()
        .radius(8.0)
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key)
        .child(icon(&icons::PLUS, 14.0).color(k.c.text_muted).a11y_hidden())
        .on_press(|s: &mut App| s.new_notice("", ""));
    let open = row()
        .key("notice-list-open")
        .role("button")
        .label(t::EDIT_RAIL_OPEN())
        .w(36.0)
        .h(30.0)
        .center()
        .radius(8.0)
        .fill(sf.work)
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key)
        .child(icon(&icons::CHEV_RIGHT, 14.0).color(k.c.text_muted).a11y_hidden())
        .on_press(|s: &mut App| s.notice_list(true));
    let _ = form;
    col().key("notice-rail").w(RAIL).h_full().min_h(0.0).items_center().pt(space::S2).gap(space::S4).fill(sf.side).child(open).child(numbers.child(new))
}

/// What the card reads, or that it holds the words typed, as the chip beside its name.
fn reads_chip(s: &App, k: &Kit, draft: Option<&artosia_core::deck::Announcement>, template: Template, empty: bool) -> El {
    if let Some(pill) = reads_pill(s, draft, empty) {
        return pill;
    }
    row()
        .key("notice-reads")
        .h(26.0)
        .px(space::S3)
        .gap(6.0)
        .center_y()
        .radius(13.0)
        .fill(theme::surf().raised)
        .child(icon(glyph(template), 14.0).color(k.c.text_muted).a11y_hidden())
        .child(text(reads(template)).font(type_::SMALL_STRONG).color(k.c.text_muted))
}

pub fn view(s: &App, k: &Kit, size: [f32; 2], layer: &mut Vec<El>) -> El {
    let Some(notices) = s.notices.as_ref() else { return el() };
    // A key typed into a text that does not fit draws the slide as it is; the cut waits for the caret to leave.
    artosia_slides::warta::while_typing(notices.slide.editing, || built(s, k, size, layer, notices))
}

fn built(s: &App, k: &Kit, size: [f32; 2], layer: &mut Vec<El>, notices: &Notices) -> El {
    let [width, height] = size;
    let in_plan = s.announcements_in_plan();
    let draft = s.notice_draft();
    let empty = draft.as_ref().is_some_and(|card| empty_note(s, card, s.card_slides(card).len()).is_some());
    let template = notices.fill.template();
    let header = row()
        .key("notice-header")
        .w_full()
        .h(slide_edit::HEAD)
        .px(space::S4)
        .gap(space::S3)
        .center_y()
        .child(text(crate::templates::name(template)).font(type_::HEADING).color(k.c.text_base).truncate().min_w(0.0))
        .child(reads_chip(s, k, draft.as_ref(), template, empty))
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
    let side = if notices.slide.list { list(s, k, notices) } else { rail(s, k, notices) };
    let side_w = if notices.slide.list { LIST } else { RAIL };
    col().key("notices").w_full().h_full().min_h(0.0).child(header).child(
        row()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .child(side)
            .child(crate::ui::widgets::vertical_divider(k))
            .child(slide_edit::main(s, k, notices, [width - side_w - 1.0, height - slide_edit::HEAD], layer)),
    )
}
