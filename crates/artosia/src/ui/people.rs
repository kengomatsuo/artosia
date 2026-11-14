//! The Members window (OVPEOPLE1, `mock_b.rs` page 4): the church's people in a table sorted by
//! next birthday, a strip of the twelve months that filters it, the birthday slide as the card reads
//! it, and the dialogs that write one person and bring a list in. A birth date is three lists, never
//! typed; only a name is.

use std::borrow::Cow;

use artosia_core::deck::{Announcement, Fill};
use artosia_core::people::{Group, month_counts};
use artosia_office::Column;
use artosia_slides::look::rgb;
use rune::{Align, Color, Cursor, Cx, El, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Cell, Dialog, Kit, ScrollBar, SegmentedControl, Select, SelectAppearance, Table, TableColumn, TextInput};

use super::calendar::{field, short, tick};
use super::widgets::{btn, divider, enter_key, tag_chip};
use crate::people::{PREVIEW, Part, YEARS};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

/// The side panel's width, as the mock's.
const SIDE: f32 = 300.0;
/// The person dialog's width and what its fields have inside it.
const DIALOG: f32 = 480.0;
const FIELD: f32 = DIALOG - 2.0 * space::S4;
/// The import dialog's width, and the most columns it shows.
const WIDE: f32 = 680.0;
const COLUMNS: usize = 6;

fn month_short(month: u8) -> String {
    short(t::MONTHS()[usize::from(month.clamp(1, 12)) - 1])
}

fn initials(name: &str) -> String {
    name.split_whitespace().filter_map(|word| word.chars().next()).take(2).flat_map(char::to_uppercase).collect()
}

fn avatar(name: &str) -> El {
    let c = rgb(crate::calendar::next_colour(name.bytes().map(usize::from).sum()));
    row().size(24.0, 24.0).center().radius(12.0).fill(theme::tint_of(c, true)).child(text(initials(name)).font(type_::TINY).color(theme::ink_of(c))).a11y_hidden()
}

fn group_chip(group: Group) -> El {
    let (words, at) = match group {
        Group::Child => (t::PEOPLE_CHILD(), 2),
        Group::Youth => (t::PEOPLE_YOUTH(), 1),
        Group::Adult => (t::PEOPLE_ADULT(), 0),
    };
    tag_chip(words, rgb(crate::calendar::next_colour(at)))
}

// ---------------------------------------------------------------- the table

fn row_id(s: &App, at: usize) -> Cow<'_, str> {
    Cow::Owned(s.person_at(at).map_or(0, |person| person.id).to_string())
}

fn cell_of(s: &App, at: usize, column: &str) -> Cell {
    let Some(person) = s.person_at(at) else { return Cell::text("") };
    let k = theme::kit();
    let today = s.civil_today();
    let next = person.next_birthday(today);
    let days = person.days_until(today);
    match column {
        "name" => Cell::el(
            row()
                .gap(space::S2)
                .center_y()
                .min_w(0.0)
                .child(avatar(&person.name))
                .child(text(person.name.clone()).font(if days == 0 { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().shrink().min_w(0.0).help_when_truncated(person.name.clone())),
        ),
        "born" => Cell::text(if person.year == 0 { t::day_month(person.day, &month_short(person.month)) } else { t::date_long(person.day, &month_short(person.month), i32::from(person.year)) }),
        "next" => {
            let words = if days == 0 { t::PEOPLE_TODAY().to_string() } else { t::people_in_days(days) };
            Cell::el(
                row()
                    .gap(space::S2)
                    .center_y()
                    .child(text(t::date_long(next.day, &month_short(next.month), next.year)).font(type_::SMALL).color(k.c.text_base))
                    .child(text(words).font(if days == 0 { type_::SMALL_STRONG } else { type_::SMALL }).color(if days == 0 { theme::accent() } else { k.c.text_muted })),
            )
        }
        "group" => person.group(today).map_or_else(|| Cell::text(""), |group| Cell::el(group_chip(group))),
        "age" => Cell::text(person.age(today).map(|age| age.to_string()).unwrap_or_default()).mono(),
        _ => {
            let id = person.id;
            Cell::el(
                row()
                    .gap(2.0)
                    .child(btn(t::PEOPLE_EDIT()).icon(&icons::PENCIL).icon_only().ghost().small().build(k, move |s, _| s.edit_person(id)).key(format!("people-edit-{id}")))
                    .child(btn(t::BTN_DELETE()).icon(&icons::TRASH).icon_only().ghost().small().build(k, move |s, _| s.delete_person(id)).key(format!("people-delete-{id}"))),
            )
        }
    }
}

fn table(s: &App, k: &Kit) -> El {
    Table::new(|s: &App| &s.members.table, |s: &mut App| &mut s.members.table, |s: &App| s.members.view.len(), row_id)
        .name(t::TAB_PEOPLE())
        .column(TableColumn::new("name", t::PEOPLE_COL_NAME()).flex(2.0).min(160.0).sortable())
        .column(TableColumn::new("born", t::PEOPLE_COL_BORN()).width(130.0).min(100.0).sortable())
        .column(TableColumn::new("next", t::PEOPLE_COL_NEXT()).width(210.0).min(150.0).sortable())
        .column(TableColumn::new("group", t::PEOPLE_COL_GROUP()).width(110.0).min(80.0).sortable())
        .column(TableColumn::new("age", t::PEOPLE_COL_AGE()).width(70.0).min(56.0).align(Align::End).sortable())
        .column(TableColumn::new("actions", "").width(76.0).resizable(false))
        .cell(cell_of)
        .zebra(true)
        .on_sort(|s: &mut App, _, _, _: &mut Tx| s.refresh_members())
        .on_open(|s: &mut App, id: &str, _: &mut Tx| {
            if let Ok(id) = id.parse() {
                s.edit_person(id);
            }
        })
        .build(s, k)
        .grow(1.0)
        .min_h(0.0)
}

/// What fills the table's place when there is nobody to list: no one at all, or no one under the filters.
fn nobody(s: &App, k: &Kit) -> El {
    let (key, title, hint) = if s.people.is_empty() { ("people-empty", t::PEOPLE_EMPTY(), t::PEOPLE_EMPTY_HINT()) } else { ("people-none", t::PEOPLE_NONE(), t::PEOPLE_NONE_HINT()) };
    let mut e = col()
        .key(key)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .center()
        .gap(space::S2)
        .child(icon(&icons::PERSON, 32.0).color(k.c.text_faint).a11y_hidden())
        .child(text(title).font(type_::HEADING).color(k.c.text_muted))
        .child(text(hint).font(type_::BODY).color(k.c.text_muted));
    if s.people.is_empty() {
        e = e.child(el().h(space::S2)).child(row().gap(space::S2).child(btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).build(k, |s, _| s.want_birthdays = true).key("people-empty-import")).child(btn(t::PEOPLE_ADD()).icon(&icons::PLUS).primary().build(k, |s, _| s.new_person()).key("people-empty-add")));
    }
    e
}

// ---------------------------------------------------------------- the window

/// The twelve months with their counts: the one pressed filters the table.
fn strip(s: &App, k: &Kit) -> El {
    let mut strip = row().key("people-months").role("group").label(t::PEOPLE_BIRTH_MONTH()).w_full().gap(space::S1);
    for (at, count) in month_counts(&s.people).into_iter().enumerate() {
        let month = at as u8 + 1;
        let on = s.members.month == Some(month);
        let (head, small) = if on { (Color::WHITE, Color::WHITE.alpha(0.85)) } else { (k.c.text_base, k.c.text_muted) };
        let tile = col()
            .key(format!("people-month-{month}"))
            .role("checkbox")
            .label(format!("{} {count}", t::MONTHS()[at]))
            .checked(on)
            .w(0.0)
            .grow(1.0)
            .h(46.0)
            .center()
            .radius(k.radius.md)
            .cursor(Cursor::Pointer);
        let tile = if on { tile.fill(theme::accent()) } else { tile.border(1.0, k.c.border_base).fill(k.c.layer_02).hover(k.c.hover) };
        strip = strip.child(
            tile.child(text(month_short(month)).font(type_::BODY_STRONG).color(head).a11y_hidden())
                .child(text(count.to_string()).font(type_::TINY.features("'tnum' 1")).color(small).a11y_hidden())
                .on_press(move |s: &mut App| s.members_month(month)),
        );
    }
    strip
}

fn filters(s: &App, k: &Kit) -> El {
    let search = TextInput::new(&s.members.search)
        .width(Some(220.0))
        .label(t::PEOPLE_SEARCH())
        .placeholder(t::PEOPLE_SEARCH())
        .leading_icon(&rune_kit::glyphs::SEARCH)
        .clear_button()
        .on_edit(|s: &mut App| &mut s.members.search)
        .on_change(|s: &mut App, _: &mut Tx| s.refresh_members())
        .build(k);
    let mut bar = row().w_full().gap(space::S2).center_y().child(search);
    if let Some(month) = s.members.month {
        let name = t::MONTHS()[usize::from(month - 1)];
        bar = bar.child(
            row()
                .key("people-month-chip")
                .role("button")
                .label(format!("{} {name}", t::PEOPLE_BIRTH_MONTH()))
                .h(32.0)
                .pl(space::S3)
                .pr(space::S2)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.md)
                .fill(theme::accent().alpha(0.18))
                .cursor(Cursor::Pointer)
                .child(text(t::PEOPLE_BIRTH_MONTH()).font(type_::BODY).color(k.c.text_muted).a11y_hidden())
                .child(text(name).font(type_::BODY_STRONG).color(k.c.text_base).a11y_hidden())
                .child(icon(&icons::CLOSE, 12.0).color(k.c.text_muted).a11y_hidden())
                .on_press(|s: &mut App| s.members_clear_month()),
        );
    }
    bar.child(spacer())
        .child(btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).build(k, |s, _| s.want_birthdays = true).key("people-import"))
        .child(btn(t::PEOPLE_ADD()).icon(&icons::PLUS).primary().hint("Ctrl N").build(k, |s, _| s.new_person()).key("people-add"))
}

fn main_column(s: &App, k: &Kit) -> El {
    let body = if s.members.view.is_empty() { nobody(s, k) } else { table(s, k) };
    col()
        .key("people-main")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .padding(space::S5)
        .gap(space::S3)
        .child(row().w_full().gap(space::S3).center_y().child(text(t::TAB_PEOPLE()).font(type_::HEADING).color(k.c.text_base)).child(text(t::people_listed(s.people.len())).font(type_::BODY).color(k.c.text_muted)))
        .child(strip(s, k))
        .child(filters(s, k))
        .child(body)
        .child(row().w_full().h(24.0).center_y().children((!s.people.is_empty()).then(|| text(t::people_showing(s.members.view.len(), s.people.len())).font(type_::SMALL).color(k.c.text_muted))))
}

/// The birthday slide as the announcement reads it, the card it belongs to, and the way to bring a list in.
fn side(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let card = s.birthday_card().cloned().unwrap_or(Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[5].0.to_string(), fill: Fill::BirthdaysMonth, seconds: 10, ..Announcement::default() });
    let width = SIDE - 2.0 * space::S4;
    let frame = el().key("people-slide").size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, k.c.border_base);
    let picture = match s.card_slides(&card).into_iter().next() {
        Some(slide) => frame.child(crate::ui::thumb(s, &s.preview_state(Some(slide)), width)),
        None => frame.center().child(text(t::NOTICE_NOTHING_TO_SHOW()).font(type_::SMALL).color(k.c.text_muted)),
    };
    let drop = row()
        .key("people-drop")
        .role("button")
        .label(t::PEOPLE_DROP())
        .w_full()
        .h(64.0)
        .center()
        .radius(k.radius.md)
        .border(1.5, k.c.border_strong)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(text(t::PEOPLE_DROP()).font(type_::BODY).color(k.c.text_muted).a11y_hidden())
        .on_press(|s: &mut App| s.want_birthdays = true);
    col()
        .key("people-side")
        .w(SIDE)
        .h_full()
        .fill(sf.side)
        .padding(space::S4)
        .gap(space::S3)
        .child(text(t::PEOPLE_USED_BY()).font(type_::TITLE).color(k.c.text_base))
        .child(text(t::PEOPLE_USED_BY_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(picture)
        .child(row().w_full().gap(space::S2).center_y().child(icon(&icons::ANNOUNCE, 16.0).color(k.c.text_muted).a11y_hidden()).child(text(card.title.clone()).font(type_::SMALL_STRONG).color(k.c.text_base).truncate().min_w(0.0)))
        .child(btn(t::PEOPLE_OPEN_CARD()).icon(&icons::ARROW_RIGHT).small().build(k, |s, _| s.open_birthday_card()).key("people-open-card"))
        .child(el().h(space::S2))
        .child(text(t::PEOPLE_IMPORT_LIST()).font(type_::TITLE).color(k.c.text_base))
        .child(text(t::PEOPLE_IMPORT_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(drop)
}

pub fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) -> El {
    let sf = theme::surf();
    let tall = cx.layout_size("window").map(|size| size[1]).unwrap_or(768.0);
    person_dialog(s, k, layer, tall);
    import_dialog(s, k, layer, tall);
    row().key("people").w_full().grow(1.0).min_h(0.0).fill(sf.work).child(main_column(s, k)).child(el().w(1.0).h_full().fill(sf.edge).a11y_hidden()).child(side(s, k))
}

// ---------------------------------------------------------------- the person dialog

fn edit_name(s: &mut App) -> &mut rune::TextEdit {
    match s.members.form.as_mut() {
        Some(form) => &mut form.name,
        None => &mut s.spare_edit,
    }
}

/// One of the three lists of a birth date, closed on its value; the caller says what a choice does.
fn list(key: &str, name: &str, width: f32, part: Part, open: bool, options: &[&str], current: &str) -> Select {
    Select::new(key, options).label(name).appearance(SelectAppearance::Field).width(width).current(Some(current)).open(open).on_toggle(move |s: &mut App| s.person_open(part)).on_close(|s: &mut App| s.person_close_part())
}

fn person_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(form) = s.members.form.as_ref() else { return };
    let mut popups: Vec<El> = Vec::new();
    let name = TextInput::new(&form.name).width(None).label(t::PEOPLE_COL_NAME()).on_edit(edit_name).on_submit(|s: &mut App, _: &mut Tx| s.save_person()).build(k);

    let (day_w, month_w) = (90.0, 190.0);
    let year_w = FIELD - day_w - month_w - 2.0 * space::S3;
    // Only the open list builds its entries.
    let days: Vec<String> = if form.open == Part::Day { (1..=form.days()).map(|day| day.to_string()).collect() } else { Vec::new() };
    let (day_now, month_now) = (form.day.to_string(), t::MONTHS()[usize::from(form.month - 1)]);
    let day_options: Vec<&str> = if form.open == Part::Day { days.iter().map(String::as_str).collect() } else { vec![day_now.as_str()] };
    let day = list("person-day", t::PEOPLE_DAY(), day_w, Part::Day, form.open == Part::Day, &day_options, &day_now)
        .on_choose(|s: &mut App, picked: &str, _: &mut Tx| {
            if let Ok(day) = picked.parse() {
                s.person_day(day);
            }
        })
        .build(k, &mut popups);
    let month = list("person-month", t::PEOPLE_MONTH(), month_w, Part::Month, form.open == Part::Month, t::MONTHS(), month_now)
        .on_choose(|s: &mut App, picked: &str, _: &mut Tx| {
            if let Some(at) = t::MONTHS().iter().position(|month| *month == picked) {
                s.person_month(at as u8 + 1);
            }
        })
        .build(k, &mut popups);
    // The year is optional: "Not known" first, then this year back.
    let latest = s.civil_today().year;
    let unknown = t::PEOPLE_YEAR_UNKNOWN();
    let year_now = if form.year == 0 { unknown.to_string() } else { form.year.to_string() };
    let years: Vec<String> = if form.open == Part::Year { std::iter::once(unknown.to_string()).chain((latest - YEARS..=latest).rev().map(|year| year.to_string())).collect() } else { Vec::new() };
    let year_options: Vec<&str> = if form.open == Part::Year { years.iter().map(String::as_str).collect() } else { vec![year_now.as_str()] };
    let year = list("person-year", t::PEOPLE_YEAR(), year_w, Part::Year, form.open == Part::Year, &year_options, &year_now)
        .on_choose(|s: &mut App, picked: &str, _: &mut Tx| s.person_year(picked.parse().unwrap_or(0)))
        .build(k, &mut popups);

    let body = col()
        .key("person-form")
        .w_full()
        .px(space::S4)
        .pb(space::S3)
        .gap(space::S3)
        .child(field(k, t::PEOPLE_COL_NAME(), name))
        .child(row().w_full().gap(space::S3).child(field(k, t::PEOPLE_DAY(), day)).child(field(k, t::PEOPLE_MONTH(), month)).child(field(k, t::PEOPLE_YEAR(), year)));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let id = form.id;
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .children((id != 0).then(|| btn(t::BTN_DELETE()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, _| s.delete_person(id)).key("person-delete")))
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_person()))
        .child(btn(t::BTN_SAVE()).primary().disabled(!form.ready()).focusable().build(k, |s, _| s.save_person()).key("person-save"));
    let title = if id == 0 { t::PEOPLE_ADD() } else { t::PEOPLE_EDIT() };
    Dialog::new("person", title).open(true).width(DIALOG).fit().on_close(|s: &mut App| s.close_person()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
    layer.extend(popups);
}

// ---------------------------------------------------------------- the import dialog

fn role_name(role: Column) -> &'static str {
    match role {
        Column::Name => t::PEOPLE_COL_NAME(),
        Column::Date => t::PEOPLE_COL_BORN(),
        Column::Day => t::PEOPLE_DAY(),
        Column::Month => t::PEOPLE_MONTH(),
        Column::Year => t::PEOPLE_YEAR(),
        Column::Ignore => t::PEOPLE_COL_IGNORE(),
    }
}

/// The first rows of the file, each column under the list that says what it holds.
fn columns(s: &App, k: &Kit, popups: &mut Vec<El>) -> El {
    let Some(import) = s.members.import.as_ref() else { return el() };
    let Some((rows, mapping)) = import.table.as_ref() else { return text(t::PEOPLE_READ_AS_IS()).font(type_::SMALL).color(k.c.text_muted) };
    let shown = mapping.columns.len().min(COLUMNS);
    let width = ((WIDE - 2.0 * space::S4 - (shown as f32 - 1.0) * space::S2) / shown as f32).floor();
    let names: Vec<&str> = Column::ALL.iter().map(|role| role_name(*role)).collect();
    let mut grid = row().key("import-columns").w_full().gap(space::S2).items_start();
    for (at, role) in mapping.columns.iter().copied().enumerate().take(shown) {
        let ignored = role == Column::Ignore;
        let choose = Select::new(format!("import-col-{at}"), &names)
            .label(role_name(role))
            .appearance(SelectAppearance::Field)
            .width(width)
            .current(Some(role_name(role)))
            .open(import.open == Some(at))
            .on_toggle(move |s: &mut App| s.import_open(at))
            .on_close(|s: &mut App| s.import_close_part())
            .on_choose(move |s: &mut App, picked: &str, _: &mut Tx| {
                if let Some(role) = Column::ALL.iter().find(|role| role_name(**role) == picked) {
                    s.import_role(at, *role);
                }
            })
            .build(k, popups);
        let mut cells = col().w(width).gap(0.0).child(choose).child(el().h(space::S2));
        for (n, cells_row) in rows.iter().take(PREVIEW + usize::from(mapping.header)).enumerate() {
            let heading = mapping.header && n == 0;
            let words = cells_row.get(at).map(|cell| cell.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default();
            let ink = if ignored { k.c.text_faint } else if heading { k.c.text_muted } else { k.c.text_base };
            cells = cells.child(
                row()
                    .w_full()
                    .h(26.0)
                    .center_y()
                    .border_sides([0.0, 0.0, 1.0, 0.0], theme::surf().edge)
                    .child(text(words).font(if heading { type_::SMALL_STRONG } else { type_::SMALL }).color(ink).truncate().min_w(0.0)),
            );
        }
        grid = grid.child(cells);
    }
    col().w_full().gap(space::S2).child(text(t::PEOPLE_COLUMNS_HINT()).font(type_::SMALL).color(k.c.text_muted)).child(grid).child(header_toggle(k, mapping.header))
}

fn header_toggle(k: &Kit, on: bool) -> El {
    row()
        .key("import-header")
        .role("checkbox")
        .label(t::PEOPLE_FIRST_ROW())
        .checked(on)
        .h(28.0)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.md)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key)
        .child(tick(k, on, theme::accent()))
        .child(text(t::PEOPLE_FIRST_ROW()).font(type_::BODY).color(k.c.text_base).a11y_hidden())
        .on_press(move |s: &mut App| s.import_header(!on))
}

fn import_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(import) = s.members.import.as_ref() else { return };
    let mut popups: Vec<El> = Vec::new();
    let mode = SegmentedControl::new(&[t::PEOPLE_ADD_TO(), t::PEOPLE_REPLACE()], Some(usize::from(import.replace)))
        .label(t::PEOPLE_IMPORT())
        .width(None)
        .on_pick(|s: &mut App, picked, _: &mut Tx| s.import_replace(picked == 1))
        .build(k);
    let body = col()
        .key("import-form")
        .w_full()
        .px(space::S4)
        .pb(space::S3)
        .gap(space::S3)
        .child(text(import.file.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0))
        .child(columns(s, k, &mut popups))
        .child(text(t::people_found(import.found.people.len(), import.found.unread)).key("import-found").font(type_::BODY_STRONG).color(k.c.text_base))
        .child(field(k, t::PEOPLE_IMPORT(), mode));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let ready = !import.found.people.is_empty();
    let go = btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).primary().disabled(!ready).focusable().build(k, |s, _| s.apply_import()).key("import-go");
    let foot = row().w_full().px(space::S4).py(space::S3).gap(space::S2).center_y().child(spacer()).child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_import())).child(if ready { go.autofocus() } else { go });
    Dialog::new("people-import-dialog", t::PEOPLE_IMPORT_LIST()).open(true).width(WIDE).fit().on_close(|s: &mut App| s.close_import()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
    layer.extend(popups);
}
