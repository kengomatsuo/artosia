//! The Calendar window (OVCAL1, `mock_b.rs` page 3): a rail of the church's calendars and the
//! schedule slide that reads them, a month or a week of coloured chips, and the dialogs that
//! write an event and look after the calendars. Every value has the control of its shape: days
//! are toggles, times come from a list, dates from a month, and only a name is typed.

use std::collections::HashMap;

use artosia_core::calendar::{Date, Event, Position, occurrences};
use artosia_core::deck::{Announcement, Fill};
use artosia_slides::look::rgb;
use rune::{Align, Color, Cursor, Cx, El, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, ScrollBar, SegmentedControl, Select, SelectAppearance, TextInput};

use super::colour::ColourFor;
use super::widgets::{btn, divider, enter_key, section_label};
use crate::calendar::{EventForm, Kind, Part, STEP, date_words, position_name, rule_words, time_words, weekday_name};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

/// The rail's width, as the mock's.
const RAIL: f32 = 250.0;
/// The event dialog's width and what its fields have inside it.
const DIALOG: f32 = 580.0;
const FIELD: f32 = DIALOG - 2.0 * space::S4;
/// A chip of the month grid, and the room its day number takes above the chips.
const CHIP: f32 = 21.0;
const NUMBER: f32 = 24.0;
/// The grid's size before the first frame has measured it, for a window of 1366 by 768.
const GRID_FALLBACK: [f32; 2] = [1116.0, 520.0];

fn label(k: &Kit, words: &str) -> El {
    text(words).font(type_::SMALL_STRONG).color(k.c.text_muted)
}

fn cap(k: &Kit, words: &str) -> El {
    text(words.to_uppercase()).font(type_::TINY.tracking(0.8)).color(k.c.text_muted)
}

fn dot(colour: [u8; 3], d: f32) -> El {
    el().size(d, d).radius(d / 2.0).fill(rgb(colour)).a11y_hidden()
}

/// The first letters of a weekday's name, as a column head or a toggle prints it.
pub(super) fn short(name: &str) -> String {
    name.chars().take(3).collect()
}

fn hline(sf: &theme::Surf) -> El {
    el().h(1.0).w_full().fill(sf.edge).a11y_hidden()
}

/// A calendar's tick in the rail.
pub(super) fn tick(k: &Kit, on: bool, colour: Color) -> El {
    let e = row().size(16.0, 16.0).center().radius(4.0);
    if on { e.fill(colour).child(icon(&icons::CHECK, 12.0).color(Color::WHITE)) } else { e.border(1.0, k.c.border_strong) }
}

/// Where each event of every calendar that is drawn falls, by day, in time order.
fn by_day(s: &App, from: Date, to: Date) -> HashMap<Date, Vec<(&Event, [u8; 3])>> {
    let mut found: HashMap<Date, Vec<(&Event, [u8; 3])>> = HashMap::new();
    for event in &s.events {
        let Some(calendar) = s.calendar_of(event.calendar).filter(|calendar| !s.calendar.hidden.contains(&calendar.id)) else { continue };
        for date in occurrences(event, from, to) {
            found.entry(date).or_default().push((event, calendar.colour));
        }
    }
    for events in found.values_mut() {
        events.sort_by(|a, b| (a.0.start, &a.0.name).cmp(&(b.0.start, &b.0.name)));
    }
    found
}

/// One event in a day of the month: its colour at the left, its name, its time.
fn chip(k: &Kit, event: &Event, colour: [u8; 3], date: Date) -> El {
    let c = rgb(colour);
    let id = event.id;
    let time = time_words(event.start);
    row()
        .key(format!("cal-event-{id}-{}", date.code()))
        .role("button")
        .label(format!("{} {time}", event.name))
        .w_full()
        .h(CHIP)
        .pr(5.0)
        .gap(5.0)
        .center_y()
        .radius(4.0)
        .fill(theme::tint_of(c, true))
        .clip_soft(0.0)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(el().w(3.0).h_full().fill(c).a11y_hidden())
        .child(text(event.name.clone()).font(rune::TextStyle::new(11.0, 600.0).lh(14.0)).color(theme::ink_of(c)).truncate().grow(1.0).min_w(0.0).a11y_hidden())
        .child(text(time).font(rune::TextStyle::new(11.0, 500.0).lh(14.0).features("'tnum' 1")).color(theme::ink_of(c)).a11y_hidden())
        .on_press(move |s: &mut App| s.edit_event(id))
}

/// One event in a column of the week: the time over the name, so a long name has the width.
fn week_chip(k: &Kit, event: &Event, colour: [u8; 3], date: Date) -> El {
    let c = rgb(colour);
    let id = event.id;
    // A hyphen: the interface never draws an en dash.
    let time = format!("{}-{}", time_words(event.start), time_words(event.end));
    row()
        .key(format!("cal-event-{id}-{}", date.code()))
        .role("button")
        .label(format!("{} {time}", event.name))
        .w_full()
        .pr(6.0)
        .gap(6.0)
        .radius(6.0)
        .fill(theme::tint_of(c, true))
        .clip_soft(0.0)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(el().w(3.0).fill(c).a11y_hidden())
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .py(5.0)
                .gap(1.0)
                .child(text(event.name.clone()).font(rune::TextStyle::new(12.0, 600.0).lh(16.0)).color(theme::ink_of(c)).wrap().a11y_hidden())
                .child(text(time).font(rune::TextStyle::new(11.0, 500.0).lh(14.0).features("'tnum' 1")).color(theme::ink_of(c)).a11y_hidden()),
        )
        .on_press(move |s: &mut App| s.edit_event(id))
}

/// The day's number: a filled circle for today.
fn number(k: &Kit, date: Date, today: Date, faint: bool) -> El {
    let words = text(date.day.to_string()).font(type_::SMALL_STRONG).color(if date == today { Color::WHITE } else if faint { k.c.text_faint } else { k.c.text_base });
    if date == today { row().size(NUMBER, NUMBER).center().radius(NUMBER / 2.0).fill(theme::accent()).child(words) } else { row().h(NUMBER).px(4.0).center_y().child(words) }
}

fn rail(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let mut rail = col()
        .key("cal-rail")
        .w(RAIL)
        .h_full()
        .fill(sf.side)
        .padding(space::S4)
        .gap(space::S2)
        .child(
            row()
                .h(28.0)
                .center_y()
                .child(text(t::CAL_CALENDARS()).font(type_::TITLE).color(k.c.text_base))
                .child(spacer())
                .child(btn(t::CAL_MANAGE()).small().build(k, |s, _| s.open_manage()).key("cal-manage")),
        );
    for calendar in &s.calendars {
        let (id, shown) = (calendar.id, !s.calendar.hidden.contains(&calendar.id));
        rail = rail.child(
            row()
                .key(format!("cal-tick-{id}"))
                .role("checkbox")
                .label(calendar.name.clone())
                .checked(shown)
                .h(28.0)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.md)
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .child(tick(k, shown, rgb(calendar.colour)))
                .child(text(calendar.name.clone()).font(type_::BODY).color(if shown { k.c.text_base } else { k.c.text_muted }).truncate().grow(1.0).min_w(0.0).a11y_hidden())
                .on_press(move |s: &mut App| s.calendar_toggle(id)),
        );
    }
    // The schedule card as the announcements read it: the saved one, or a plain one made for this picture.
    // The saved card's own title, or the template's when it has none or there is no card.
    let mut card = s.announcements.iter().find(|card| card.fill == Fill::ScheduleWeek).cloned().unwrap_or(Announcement { fill: Fill::ScheduleWeek, seconds: 10, ..Announcement::default() });
    if card.title.trim().is_empty() {
        card.title = t::ANNOUNCEMENT_TEMPLATES()[crate::calendar::SCHEDULE_TEMPLATE].0.to_string();
    }
    let slide = s.card_slides(&card).into_iter().next();
    let width = RAIL - 2.0 * space::S4;
    let picture = match slide {
        Some(slide) => {
            let state = s.preview_state(Some(slide));
            el().key("cal-schedule").size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, k.c.border_base).child(crate::ui::thumb(s, &state, width))
        }
        None => el().key("cal-schedule").size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).border(1.0, k.c.border_base),
    };
    rail.child(el().h(space::S2)).child(label(k, t::CAL_READ_BY())).child(picture).child(text(t::CAL_READ_BY_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
}

/// The month on show, or the week: one month's name when the week stays in it, both ends when it does not.
fn title(s: &App) -> String {
    let month = |date: Date| t::MONTHS()[usize::from(date.month - 1)];
    if s.calendar.week {
        let (from, to) = (s.week_start(), s.week_start().plus_days(6));
        if (from.month, from.year) != (to.month, to.year) {
            return t::week_title((from.day, month(from), from.year), (to.day, month(to), to.year));
        }
        return t::month_year(month(from), from.year);
    }
    let anchor = s.calendar.anchor;
    t::month_year(month(anchor), anchor.year)
}

fn toolbar(s: &App, k: &Kit) -> El {
    let title = title(s);
    row()
        .w_full()
        .h(56.0)
        .px(space::S4)
        .gap(space::S2)
        .center_y()
        .child(btn(t::CAL_TODAY()).small().build(k, |s, _| s.calendar_today()).key("cal-today"))
        .child(btn(t::CAL_PREVIOUS()).icon(&icons::ARROW_LEFT).icon_only().small().build(k, |s, _| s.calendar_move(-1)).key("cal-previous"))
        .child(btn(t::CAL_NEXT()).icon(&icons::ARROW_RIGHT).icon_only().small().build(k, |s, _| s.calendar_move(1)).key("cal-next"))
        .child(text(title).font(type_::HEADING).color(k.c.text_base).mx(8.0))
        .child(spacer())
        .child(
            SegmentedControl::new(&[t::CAL_WEEK(), t::CAL_MONTH()], Some(usize::from(!s.calendar.week)))
                .label(t::TAB_CALENDAR())
                .fit()
                .on_pick(|s: &mut App, picked, _: &mut Tx| s.calendar_show_week(picked == 0))
                .build(k),
        )
        .child(btn(t::CAL_NEW_EVENT()).icon(&icons::PLUS).primary().hint("Ctrl N").build(k, |s, _| s.new_event_here()).key("cal-new"))
}

/// The grid of a month, Sunday first, today filled, each day with the chips that fit and "+N more".
fn month(s: &App, cx: &Cx<App>, k: &Kit) -> El {
    let sf = theme::surf();
    let today = s.civil_today();
    let anchor = s.calendar.anchor;
    let first = s.month_start();
    let weeks = (usize::from(anchor.first_of_month().weekday()) + usize::from(Date::month_days(anchor.year, anchor.month))).div_ceil(7);
    let found = by_day(s, first, first.plus_days(7 * weeks as i64 - 1));
    let size = cx.layout_size("cal-grid").unwrap_or(GRID_FALLBACK);
    // The room a day has under its number, from what the grid measured last frame.
    let row_height = (size[1] - 27.0) / weeks as f32 - 1.0;
    let room = row_height - 2.0 * 4.0 - NUMBER - 2.0;
    let mut heads = row().w_full().h(26.0);
    for day in 0..7u8 {
        heads = heads.child(row().w(0.0).grow(1.0).px(space::S2).justify_end().center_y().child(cap(k, &short(weekday_name(day)))));
    }
    let mut grid = col().key("cal-grid").w_full().grow(1.0).min_h(0.0).child(hline(&sf)).child(heads);
    for week in 0..weeks {
        let mut line = row().w_full().grow(1.0).min_h(0.0);
        for day in 0..7 {
            let date = first.plus_days((week * 7 + day) as i64);
            let inside = date.month == anchor.month;
            let events = found.get(&date).map(Vec::as_slice).unwrap_or_default();
            // Every chip when they fit, else one line less for the "+N more".
            let all = ((events.len() as f32) * (CHIP + 2.0) - 2.0) <= room;
            let shown = if all { events.len() } else { (((room - 16.0) / (CHIP + 2.0)).floor().max(0.0) as usize).min(events.len()) };
            let mut cell = col()
                .key(format!("cal-day-{}", date.code()))
                .w(0.0)
                .grow(1.0)
                .h_full()
                .padding(4.0)
                .gap(2.0)
                .border_sides([0.0, 0.0, 1.0, 1.0], sf.edge)
                .fill(if inside { Color::TRANSPARENT } else { sf.side.alpha(0.6) })
                .clip_soft(0.0)
                .child(row().w_full().justify_end().child(number(k, date, today, !inside)));
            for (event, colour) in &events[..shown] {
                cell = cell.child(chip(k, event, *colour, date));
            }
            if shown < events.len() {
                cell = cell.child(
                    row()
                        .key(format!("cal-more-{}", date.code()))
                        .role("button")
                        .label(t::more_events(events.len() - shown))
                        .w_full()
                        .h(14.0)
                        .px(4.0)
                        .center_y()
                        .cursor(Cursor::Pointer)
                        .child(text(t::more_events(events.len() - shown)).font(type_::TINY).color(k.c.text_muted).a11y_hidden())
                        .on_press(move |s: &mut App| {
                            s.calendar.anchor = date;
                            s.calendar.week = true;
                        }),
                );
            }
            line = line.child(cell.on_double_press(move |s: &mut App| s.new_event(date)));
        }
        grid = grid.child(line).child(hline(&sf));
    }
    grid
}

/// Seven columns, Sunday first, each a day's events in full.
fn week(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let today = s.civil_today();
    let first = s.week_start();
    let found = by_day(s, first, first.plus_days(6));
    let mut columns = row().key("cal-grid").w_full().grow(1.0).min_h(0.0).border_sides([1.0, 0.0, 0.0, 0.0], sf.edge);
    for day in 0..7 {
        let date = first.plus_days(day);
        let mut chips = col().w_full().px(6.0).pb(6.0).gap(6.0);
        for (event, colour) in found.get(&date).map(Vec::as_slice).unwrap_or_default() {
            chips = chips.child(week_chip(k, event, *colour, date));
        }
        let head = col().w_full().h(64.0).items_end().justify_center().px(space::S2).gap(2.0).child(cap(k, &short(weekday_name(date.weekday())))).child(number(k, date, today, false));
        columns = columns.child(
            col()
                .key(format!("cal-day-{}", date.code()))
                .w(0.0)
                .grow(1.0)
                .h_full()
                .min_h(0.0)
                .border_sides([0.0, 0.0, 0.0, 1.0], sf.edge)
                .child(head)
                .child(col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(chips).scroll_bar(k))
                .on_double_press(move |s: &mut App| s.new_event(date)),
        );
    }
    columns
}

pub fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) -> El {
    let sf = theme::surf();
    let main = col().grow(1.0).min_w(0.0).h_full().child(toolbar(s, k)).child(if s.calendar.week { week(s, k) } else { month(s, cx, k) });
    let tall = cx.layout_size("window").map(|size| size[1]).unwrap_or(768.0);
    event_dialog(s, k, layer, tall);
    manage_dialog(s, k, layer, tall);
    row().key("calendar").w_full().grow(1.0).min_h(0.0).fill(sf.work).child(rail(s, k)).child(el().w(1.0).h_full().fill(sf.edge).a11y_hidden()).child(main)
}

// ---------------------------------------------------------------- the event dialog

fn edit_name(s: &mut App) -> &mut rune::TextEdit {
    match s.calendar.form.as_mut() {
        Some(form) => &mut form.name,
        None => &mut s.spare_edit,
    }
}

/// A field of the dialog under its label.
pub(super) fn field(k: &Kit, words: &str, control: El) -> El {
    col().gap(4.0).child(label(k, words)).child(control)
}

/// The closed look of a list: a box with its value and a chevron, as the mock's.
pub(super) fn picker(k: &Kit, key: &str, name: &str, lead: Option<El>, value: String, open: bool) -> El {
    let mut e = row()
        .key(key)
        .role("button")
        .label(format!("{name} {value}"))
        .h(32.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .border(1.0, if open { theme::accent() } else { k.c.border_base })
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key);
    if let Some(lead) = lead {
        e = e.child(lead);
    }
    e.child(text(value).font(type_::BODY).color(k.c.text_base).truncate().grow(1.0).min_w(0.0).a11y_hidden()).child(icon(if open { k.glyph.chevron_up } else { k.glyph.chevron_down }, 14.0).color(k.c.text_muted).a11y_hidden())
}

/// A time list in 15-minute steps, for the start or the end; only the open list builds its 96 entries.
fn times(k: &Kit, form: &EventForm, part: Part, width: f32, popups: &mut Vec<El>) -> El {
    let (key, name, value) = if part == Part::Start { ("event-start", t::CAL_START(), form.start) } else { ("event-end", t::CAL_END(), form.end) };
    let open = form.open == part;
    let all: Vec<String> = if open { (0..24 * 60 / STEP).map(|at| time_words(at * STEP)).collect() } else { Vec::new() };
    let current = time_words(value);
    let options: Vec<&str> = if open { all.iter().map(String::as_str).collect() } else { vec![current.as_str()] };
    Select::new(key, &options)
        .label(name)
        .appearance(SelectAppearance::Field)
        .width(width)
        .current(Some(&current))
        .open(open)
        .on_toggle(move |s: &mut App| s.event_open(part))
        .on_close(|s: &mut App| s.event_close_part())
        .on_choose(move |s: &mut App, picked: &str, _: &mut Tx| {
            if let Some(at) = (0..24 * 60 / STEP).find(|at| time_words(at * STEP) == picked) {
                if part == Part::Start { s.event_start(at * STEP) } else { s.event_end(at * STEP) }
            }
        })
        .build(k, popups)
}

fn day_toggle(k: &Kit, weekday: u8, on: bool) -> El {
    let name = weekday_name(weekday);
    let e = row()
        .key(format!("event-day-{weekday}"))
        .role("checkbox")
        .label(name)
        .checked(on)
        .w(46.0)
        .h(34.0)
        .center()
        .radius(k.radius.md)
        .focusable(true)
        .on_key(enter_key)
        .cursor(Cursor::Pointer);
    let e = if on { e.fill(theme::accent()).child(text(short(name)).font(type_::BODY_STRONG).color(Color::WHITE).a11y_hidden()) } else { e.fill(k.c.layer_02).border(1.0, k.c.border_base).hover(k.c.hover).child(text(short(name)).font(type_::BODY).color(k.c.text_base).a11y_hidden()) };
    e.on_press(move |s: &mut App| s.event_toggle_day(weekday))
}

/// A month to pick a day from, drawn under the date it picks for.
fn month_picker(s: &App, k: &Kit, form: &EventForm) -> El {
    let sf = theme::surf();
    let today = s.civil_today();
    let shown = form.shown;
    let first = shown.first_of_month();
    let start = first.plus_days(-i64::from(first.weekday()));
    let chosen = if form.open == Part::Until { form.until } else { Some(form.date) };
    let head = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(btn(t::CAL_PREVIOUS()).icon(&icons::ARROW_LEFT).icon_only().small().focusable().build(k, |s, _| s.event_month(-1)))
        .child(text(t::month_year(t::MONTHS()[usize::from(shown.month - 1)], shown.year)).font(type_::BODY_STRONG).color(k.c.text_base).grow(1.0).align(Align::Center))
        .child(btn(t::CAL_NEXT()).icon(&icons::ARROW_RIGHT).icon_only().small().focusable().build(k, |s, _| s.event_month(1)));
    let mut heads = row().w_full();
    for day in 0..7u8 {
        heads = heads.child(row().w(0.0).grow(1.0).h(20.0).center().child(cap(k, &short(weekday_name(day)))));
    }
    let mut grid = col().key("event-month").role("grid").w_full().gap(2.0).focus_group().focus_follows();
    // As many weeks as the month has, so a short month does not end in a row of next month's days.
    let weeks = (usize::from(first.weekday()) + usize::from(Date::month_days(shown.year, shown.month))).div_ceil(7) as i64;
    for week in 0..weeks {
        let mut line = row().w_full().gap(2.0);
        for day in 0..7 {
            let date = start.plus_days(week * 7 + day);
            let on = chosen == Some(date);
            let inside = date.month == shown.month;
            // The last day cannot come before the first.
            let allowed = form.open != Part::Until || date >= form.date;
            let words = text(date.day.to_string()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(if on { Color::WHITE } else if !allowed { k.c.text_faint } else if inside { k.c.text_base } else { k.c.text_muted });
            let mut cell = row().key(format!("event-date-{}", date.code())).role("button").label(date_words(date)).w(0.0).grow(1.0).h(28.0).center().radius(k.radius.md);
            cell = if on {
                cell.fill(theme::accent())
            } else if date == today {
                cell.border(1.5, theme::accent())
            } else {
                cell
            };
            cell = cell.child(words.a11y_hidden());
            line = line.child(if allowed { cell.hover(k.c.hover).cursor(Cursor::Pointer).focusable(true).focus_selected(on).on_key(enter_key).on_press(move |s: &mut App| s.event_pick_date(date)) } else { cell.disabled(true) });
        }
        grid = grid.child(line);
    }
    let mut foot = row().w_full().gap(space::S2).justify_end();
    if form.open == Part::Until {
        foot = foot.child(btn(t::CAL_NO_END()).small().focusable().build(k, |s, _| s.event_no_end()));
    }
    col().key("event-picker").w_full().padding(space::S3).gap(space::S2).radius(k.radius.md).fill(sf.raised).border(1.0, sf.edge).child(head).child(heads).child(grid).child(foot)
}

fn event_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(form) = s.calendar.form.as_ref() else { return };
    let mut popups: Vec<El> = Vec::new();
    let half = (FIELD - space::S3) / 2.0;

    let name = TextInput::new(&form.name)
        .width(None)
        .label(t::CAL_EVENT_NAME())
        .on_edit(edit_name)
        .on_submit(|s: &mut App, _: &mut Tx| s.save_event())
        .build(k);

    // The calendar: a drop-down whose rows carry the calendar's colour.
    let current = s.calendar_of(form.calendar);
    let open_calendar = form.open == Part::Calendar;
    let trigger = picker(k, "event-calendar-box", t::CAL_CALENDAR(), current.map(|calendar| dot(calendar.colour, 12.0)), current.map(|calendar| calendar.name.clone()).unwrap_or_default(), open_calendar).w(FIELD).on_press(|s: &mut App| s.event_open(Part::Calendar));
    let mut menu = Menu::new("event-calendar").trigger(trigger).open(open_calendar).min_width(FIELD).on_close(|s: &mut App| s.event_close_part());
    for calendar in &s.calendars {
        let id = calendar.id;
        menu = menu.item(MenuItem::new(calendar.name.clone()).lead(dot(calendar.colour, 12.0)).checked(id == form.calendar).on_select(move |s: &mut App| s.event_calendar(id)));
    }
    let calendar = menu.build(k, &mut popups);

    let at = Kind::ALL.iter().position(|kind| *kind == form.kind);
    let repeat = SegmentedControl::new(&[t::CAL_ONCE(), t::CAL_EVERY_WEEK(), t::CAL_EVERY_MONTH()], at)
        .label(t::CAL_REPEAT())
        .width(None)
        .on_pick(|s: &mut App, picked, _: &mut Tx| s.event_kind(Kind::ALL[picked.min(2)]))
        .build(k);

    let mut which: Vec<El> = Vec::new();
    match form.kind {
        Kind::Once => {}
        Kind::Week => {
            let mut days = row().key("event-days").role("group").label(t::CAL_DAYS()).gap(space::S1).focus_group();
            for weekday in 0..7u8 {
                days = days.child(day_toggle(k, weekday, form.days & (1 << weekday) != 0));
            }
            which.push(field(k, t::CAL_DAYS(), days));
        }
        Kind::Month => {
            let names: Vec<&str> = Position::ALL.iter().map(|position| position_name(*position)).collect();
            let position = Select::new("event-which", &names)
                .label(t::CAL_WHICH())
                .appearance(SelectAppearance::Field)
                .width(half)
                .current(Some(position_name(form.position)))
                .open(form.open == Part::Which)
                .on_toggle(|s: &mut App| s.event_open(Part::Which))
                .on_close(|s: &mut App| s.event_close_part())
                .on_choose(|s: &mut App, picked: &str, _: &mut Tx| {
                    if let Some(position) = Position::ALL.iter().find(|position| position_name(**position) == picked) {
                        s.event_position(*position);
                    }
                })
                .build(k, &mut popups);
            // The list reads Sunday first, as the calendar does.
            let days: Vec<&str> = (0..7u8).map(weekday_name).collect();
            let weekday = Select::new("event-weekday", &days)
                .label(t::CAL_WEEKDAY())
                .appearance(SelectAppearance::Field)
                .width(half)
                .current(Some(weekday_name(form.weekday)))
                .open(form.open == Part::Weekday)
                .on_toggle(|s: &mut App| s.event_open(Part::Weekday))
                .on_close(|s: &mut App| s.event_close_part())
                .on_choose(|s: &mut App, picked: &str, _: &mut Tx| {
                    if let Some(day) = (0..7u8).find(|day| weekday_name(*day) == picked) {
                        s.event_weekday(day);
                    }
                })
                .build(k, &mut popups);
            which.push(row().w_full().gap(space::S3).child(field(k, t::CAL_WHICH(), position)).child(field(k, t::CAL_WEEKDAY(), weekday)));
        }
    }

    let start = times(k, form, Part::Start, half, &mut popups);
    let end = times(k, form, Part::End, half, &mut popups);

    let first = picker(k, "event-first", t::CAL_FIRST_DATE(), Some(icon(&icons::CALENDAR, 16.0).color(k.c.text_muted).a11y_hidden()), date_words(form.date), form.open == Part::First).on_press(|s: &mut App| s.event_open(Part::First));
    let mut dates = row().w_full().gap(space::S3).child(col().w(0.0).grow(1.0).child(field(k, if form.kind == Kind::Once { t::CAL_DATE() } else { t::CAL_FIRST_DATE() }, first)));
    if form.kind != Kind::Once {
        let until = picker(k, "event-until", t::CAL_UNTIL(), Some(icon(&icons::CALENDAR, 16.0).color(k.c.text_muted).a11y_hidden()), form.until.map_or_else(|| t::CAL_NO_END().to_string(), date_words), form.open == Part::Until).on_press(|s: &mut App| s.event_open(Part::Until));
        dates = dates.child(col().w(0.0).grow(1.0).child(field(k, t::CAL_UNTIL(), until)));
    }

    let rule = row().key("event-rule").w_full().padding(space::S3).radius(k.radius.md).fill(theme::accent().alpha(0.14)).child(text(rule_words(&form.event())).font(type_::BODY_STRONG).color(k.c.text_base).grow(1.0).min_w(0.0).wrap());

    let mut body = col()
        .key("event-form")
        .w_full()
        .px(space::S4)
        .pb(space::S3)
        .gap(space::S3)
        .child(field(k, t::CAL_EVENT_NAME(), name))
        .child(field(k, t::CAL_CALENDAR(), calendar))
        .child(field(k, t::CAL_REPEAT(), repeat))
        .children(which)
        .child(row().w_full().gap(space::S3).child(field(k, t::CAL_START(), start)).child(field(k, t::CAL_END(), end)))
        .child(dates);
    if matches!(form.open, Part::First | Part::Until) {
        body = body.child(month_picker(s, k, form));
    }
    body = body.child(rule);
    if form.kind != Kind::Once {
        let listed = s.event_dates();
        let mut chips = row().key("event-dates").w_full().gap(space::S1).flex_wrap();
        for date in &listed {
            let date = *date;
            let skipped = form.skipped.contains(&date);
            let words = t::day_month(date.day, t::MONTHS()[usize::from(date.month - 1)]);
            let chip = row()
                .key(format!("event-skip-{}", date.code()))
                .role("button")
                .label(words.clone())
                .h(26.0)
                .pl(space::S2)
                .pr(6.0)
                .gap(6.0)
                .center_y()
                .radius(13.0)
                .fill(k.c.layer_03)
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .on_key(enter_key)
                .child(text(words).font(type_::SMALL_STRONG).color(if skipped { k.c.text_faint } else { k.c.text_base }).a11y_hidden())
                .child(icon(if skipped { &icons::PLUS } else { &icons::CLOSE }, 10.0).color(k.c.text_muted).a11y_hidden())
                .on_press(move |s: &mut App| s.event_skip(date));
            chips = chips.child(if skipped { chip.help(t::CAL_SKIPPED()).layer_opacity(0.6) } else { chip });
        }
        if listed.is_empty() {
            chips = chips.child(text(t::CAL_NO_DATES()).font(type_::SMALL).color(k.c.text_muted));
        }
        body = body.child(col().w_full().gap(4.0).child(label(k, t::CAL_DATES())).child(chips));
    }
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let id = form.id;
    let ready = form.ready();
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .children((id != 0).then(|| btn(t::BTN_DELETE()).icon(&icons::TRASH).ghost().focusable().build(k, |s, _| s.delete_event()).key("event-delete")))
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_event()))
        .child(btn(t::BTN_SAVE()).primary().disabled(!ready).focusable().build(k, |s, _| s.save_event()).key("event-save"));
    let all = col().w_full().child(scrolling).child(divider(k)).child(foot);
    let title = if id == 0 { t::CAL_NEW_EVENT() } else { t::CAL_EVENT() };
    Dialog::new("calendar-event", title).open(true).width(DIALOG).fit().on_close(|s: &mut App| s.close_event()).child(all).build(k, layer);
    layer.extend(popups);
}

// ---------------------------------------------------------------- the calendars dialog

fn manage_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(picked) = s.calendar.manage else { return };
    let mut chips = row().key("manage-list").role("list").label(t::CAL_CALENDARS()).w_full().gap(space::S1).flex_wrap().focus_group().focus_follows();
    for calendar in &s.calendars {
        let (id, on) = (calendar.id, calendar.id == picked);
        chips = chips.child(
            row()
                .key(format!("manage-{id}"))
                .role("button")
                .label(calendar.name.clone())
                .h(30.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(15.0)
                .fill(if on { k.c.layer_03 } else { Color::TRANSPARENT })
                .border(1.0, if on { k.c.border_strong } else { k.c.border_base })
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .focus_selected(on)
                .on_key(enter_key)
                .child(dot(calendar.colour, 12.0))
                .child(text(calendar.name.clone()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).a11y_hidden())
                .on_press(move |s: &mut App| s.pick_calendar(id)),
        );
    }
    let name = TextInput::new(&s.calendar.rename)
        .width(None)
        .label(t::CAL_CALENDAR_NAME())
        .on_edit(|s: &mut App| &mut s.calendar.rename)
        .on_change(|s: &mut App, _: &mut Tx| s.rename_calendar())
        .build(k);
    let mut body = col().w_full().px(space::S4).pb(space::S3).gap(space::S3).child(chips);
    if s.calendar_of(picked).is_some() {
        body = body.child(col().w_full().gap(4.0).child(section_label(k, t::CAL_CALENDAR_NAME())).child(name));
        body = body.children(super::colour::rows(s, k, ColourFor::Calendar(picked), t::CAL_COLOUR(), &[]));
    }
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .child(btn(t::CAL_ADD_CALENDAR()).icon(&icons::PLUS).focusable().build(k, |s, _| s.add_calendar()).key("manage-add"))
        .child(btn(t::CAL_DELETE_CALENDAR()).icon(&icons::TRASH).ghost().disabled(s.calendar_of(picked).is_none()).focusable().build(k, |s, _| s.delete_calendar()).key("manage-delete"))
        .child(spacer())
        .child(btn(t::BTN_CLOSE()).primary().focusable().build(k, |s, _| s.close_manage()));
    Dialog::new("calendar-manage", t::CAL_CALENDARS()).open(true).width(660.0).fit().on_close(|s: &mut App| s.escape_manage()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}
