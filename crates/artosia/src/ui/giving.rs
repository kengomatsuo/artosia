//! The Offerings window (OVGIVING1, `mock_b.rs` page 5): the gifts in a table with filters for a
//! range of days, a fund and a method, an amount edited where it stands, the total and the sum per
//! fund added from the rows, the money slide as the card reads it, and the transfer account. A
//! value has the control of its shape: a day from a month, a fund and a method from lists, an
//! amount from digits; only a giver's name and the account's words are typed.

use std::borrow::Cow;

use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, Fill};
use artosia_core::giving::{Filter, Fund, Method, fund_sum, rupiah, total};
use artosia_slides::look::rgb;
use rune::{Align, Cursor, Cx, El, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Cell, Dialog, Kit, Placement, Popover, ScrollBar, Select, SelectAppearance, Table, TableColumn, TableStyle, TextInput};

use super::calendar::{cap, dot, field, label, month_grid, picker, short, tick};
use super::colour::ColourFor;
use super::widgets::{btn, divider, enter_key, section_label};
use crate::calendar::date_words;
use crate::giving::{Part, Window};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, space, type_};

/// The side panel's width, as the mock's.
const SIDE: f32 = 340.0;
/// The gift dialog's width and what its fields have inside it.
const DIALOG: f32 = 480.0;
const FIELD: f32 = DIALOG - 2.0 * space::S4;
/// The account dialog and the funds dialog.
const ACCOUNT: f32 = 480.0;
const FUNDS: f32 = 660.0;
/// What the table's rows and header measure, as the mock's.
const ROW: f32 = 36.0;
const HEAD: f32 = 32.0;
/// The boxes of the filters, as the mock's.
const DATES_W: f32 = 210.0;
const FUND_W: f32 = 180.0;
const METHOD_W: f32 = 150.0;
/// The width of the amount's cell field and of the total beside the table's foot.
const AMOUNT_W: f32 = 170.0;
/// The field an amount becomes in its cell, as the mock's edited cell.
const FIELD_W: f32 = 140.0;

fn month_short(month: u8) -> String {
    short(t::MONTHS()[usize::from(month.clamp(1, 12)) - 1])
}

/// A day with its month shortened, as the table and the date filter write it.
fn date_short(date: Date) -> String {
    t::date_long(date.day, &month_short(date.month), date.year)
}

fn method_name(method: Method) -> &'static str {
    match method {
        Method::Cash => t::GIVING_CASH(),
        Method::Transfer => t::GIVING_TRANSFER(),
    }
}

/// A fund as a chip: its dot and name on its own tint.
fn fund_chip(fund: Option<&Fund>) -> El {
    let Some(fund) = fund else { return el() };
    let c = rgb(fund.colour);
    row().h(22.0).px(space::S2).gap(6.0).center_y().radius(11.0).fill(theme::tint_of(c, true)).child(dot(fund.colour, 8.0)).child(text(fund.name.clone()).font(type_::SMALL_STRONG).color(theme::ink_of(c)).truncate().min_w(0.0))
}

// ---------------------------------------------------------------- the table

fn row_id(s: &App, at: usize) -> Cow<'_, str> {
    Cow::Owned(s.gift_at(at).map_or(0, |gift| gift.id).to_string())
}

fn edit_amount(s: &mut App) -> &mut rune::TextEdit {
    match s.giving.amount.as_mut() {
        Some((_, edit)) => edit,
        None => &mut s.spare_edit,
    }
}

/// Esc leaves the field; the amount is already written.
fn amount_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == rune::Key::Escape {
        s.finish_amount();
        tx.blur();
        return true;
    }
    false
}

/// An amount as figures to read, which a press turns into a field holding its digits.
fn amount_cell(s: &App, k: &Kit, id: i64, amount: i64) -> Cell {
    let editing = s.giving.amount.as_ref().filter(|(at, edit)| *at == id && edit.focused);
    let words = rupiah(amount);
    let body = match editing {
        Some((_, edit)) => TextInput::new(edit)
            .width(Some(FIELD_W))
            .height(28.0)
            .label(t::GIVING_COL_AMOUNT())
            .on_edit(edit_amount)
            .on_change(|s: &mut App, _: &mut Tx| s.amount_changed())
            .on_submit(|s: &mut App, tx: &mut Tx| {
                s.finish_amount();
                tx.blur();
            })
            .on_key(amount_key)
            .build(k)
            .key(format!("gift-amount-field-{id}")),
        None => row()
            .key(format!("gift-amount-{id}"))
            .role("button")
            .label(format!("{} {words}", t::GIVING_COL_AMOUNT()))
            .h(28.0)
            .px(space::S2)
            .center_y()
            .justify_end()
            .radius(k.radius.sm)
            .hover(k.c.hover)
            .cursor(Cursor::Pointer)
            .focusable(true)
            .on_key(enter_key)
            .child(text(words).font(type_::BODY.features("'tnum' 1")).color(k.c.text_base).a11y_hidden())
            .on_press(move |s: &mut App| s.begin_amount(id)),
    };
    Cell::el(row().w_full().justify_end().child(body))
}

fn cell_of(s: &App, at: usize, column: &str) -> Cell {
    let Some(gift) = s.gift_at(at) else { return Cell::text("") };
    let k = theme::kit();
    match column {
        "date" => Cell::el(row().gap(6.0).center_y().child(text(date_short(gift.date)).font(type_::BODY).color(k.c.text_base)).child(icon(&icons::CALENDAR, 14.0).color(k.c.text_faint).a11y_hidden())),
        "giver" => {
            if gift.giver.is_empty() {
                Cell::el(text(t::GIVING_PLATE()).font(type_::BODY).color(k.c.text_muted).truncate().min_w(0.0))
            } else {
                Cell::el(text(gift.giver.clone()).font(type_::BODY).color(k.c.text_base).truncate().min_w(0.0).help_when_truncated(gift.giver.clone()))
            }
        }
        "fund" => Cell::el(fund_chip(s.fund_of(gift.fund))),
        "method" => Cell::text(method_name(gift.method)),
        "amount" => amount_cell(s, k, gift.id, gift.amount),
        _ => {
            let id = gift.id;
            Cell::el(
                row()
                    .gap(2.0)
                    .child(btn(t::GIVING_EDIT()).icon(&icons::PENCIL).icon_only().ghost().small().build(k, move |s, _| s.edit_gift(id)).key(format!("giving-edit-{id}")))
                    .child(btn(t::BTN_DELETE()).icon(&icons::TRASH).icon_only().ghost().small().build(k, move |s, _| s.delete_gift(id)).key(format!("giving-delete-{id}"))),
            )
        }
    }
}

fn table(s: &App, k: &Kit) -> El {
    let mut style = TableStyle::from_kit(k);
    style.row_height = ROW;
    style.header_height = HEAD;
    Table::new(|s: &App| &s.giving.table, |s: &mut App| &mut s.giving.table, |s: &App| s.giving.view.len(), row_id)
        .name(t::TAB_GIVING())
        .column(TableColumn::new("date", t::GIVING_COL_DATE()).width(140.0).min(120.0).sortable())
        .column(TableColumn::new("giver", t::GIVING_COL_GIVER()).flex(1.4).min(140.0).sortable())
        .column(TableColumn::new("fund", t::GIVING_COL_FUND()).width(190.0).min(130.0).sortable())
        .column(TableColumn::new("method", t::GIVING_COL_METHOD()).width(100.0).min(80.0).sortable())
        .column(TableColumn::new("amount", t::GIVING_COL_AMOUNT()).width(AMOUNT_W).min(140.0).align(Align::End).sortable())
        .column(TableColumn::new("actions", "").width(76.0).resizable(false))
        .cell(cell_of)
        .zebra(true)
        .style(style)
        .on_sort(|s: &mut App, _, _, _: &mut Tx| {
            s.finish_amount();
            s.refresh_giving();
        })
        .on_open(|s: &mut App, id: &str, _: &mut Tx| {
            if let Ok(id) = id.parse() {
                s.edit_gift(id);
            }
        })
        .build(s, k)
        .grow(1.0)
        .min_h(0.0)
}

/// What fills the table's place when there is nothing to list: no gift at all, or none under the filters.
fn nothing(s: &App, k: &Kit) -> El {
    let (key, title, hint) = if s.gifts.is_empty() { ("giving-empty", t::GIVING_EMPTY(), t::GIVING_EMPTY_HINT()) } else { ("giving-none", t::GIVING_NONE(), t::GIVING_NONE_HINT()) };
    let mut e = col()
        .key(key)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .center()
        .gap(space::S2)
        .child(icon(&icons::COIN, 32.0).color(k.c.text_faint).a11y_hidden())
        .child(text(title).font(type_::HEADING).color(k.c.text_muted))
        .child(text(hint).font(type_::BODY).color(k.c.text_muted));
    if s.gifts.is_empty() {
        e = e.child(el().h(space::S2)).child(btn(t::GIVING_ADD()).icon(&icons::PLUS).primary().build(k, |s, _| s.new_gift()).key("giving-empty-add"));
    }
    e
}

// ---------------------------------------------------------------- the window

/// A figure over the word that says what it counts.
fn big(k: &Kit, figure: String, words: String, key: &str) -> El {
    col().key(key).gap(2.0).child(text(figure).font(type_::HEADING.features("'tnum' 1")).color(k.c.text_base)).child(text(words).font(type_::SMALL).color(k.c.text_muted))
}

/// The box of the date filter, with the month picker that drops under it.
fn dates_filter(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let w: &Window = &s.giving;
    let open = w.open == Part::Date;
    let value = match (w.filter.from, w.filter.to) {
        (Some(from), Some(to)) => t::giving_range(&date_short(from), &date_short(to)),
        _ => t::GIVING_ALL_DATES().to_string(),
    };
    let trigger = picker(k, "giving-dates", t::GIVING_DATES(), Some(icon(&icons::CALENDAR, 16.0).color(k.c.text_muted).a11y_hidden()), value, open).h(28.0).w(DATES_W).on_press(|s: &mut App| s.giving_open(Part::Date));
    let Filter { from, to, .. } = w.filter;
    let foot = row().w_full().gap(space::S2).justify_end().child(btn(t::GIVING_THIS_MONTH()).small().focusable().build(k, |s, _| s.giving_this_month())).child(btn(t::GIVING_ALL_DATES()).small().focusable().build(k, |s, _| s.giving_all_dates()));
    let grid = month_grid(
        s,
        k,
        "giving",
        w.shown,
        move |date| from == Some(date) || to == Some(date),
        move |date| matches!((from, to), (Some(from), Some(to)) if date > from && date < to),
        |_| true,
        |s, by| s.giving_month(by),
        |s, date| s.giving_pick_date(date),
        foot,
    );
    let popover = Popover::new("giving-dates-popover").trigger(trigger).open(open).placement(Placement::BottomStart).padding(space::S3).on_close(|s: &mut App| s.giving_close_part()).child(el().w(280.0).child(grid)).build(k, layer);
    col().gap(2.0).w(DATES_W).child(label(k, t::GIVING_DATES())).child(popover)
}

/// A filter list: "all" and then each choice, closed on the one chosen.
struct Choice<'a> {
    key: &'a str,
    name: &'a str,
    width: f32,
    part: Part,
    options: &'a [String],
    current: usize,
    on_pick: fn(&mut App, usize),
}

fn list(s: &App, k: &Kit, choice: Choice, layer: &mut Vec<El>) -> El {
    let Choice { key, name, width, part, options, current, on_pick } = choice;
    let words = options.to_vec();
    let picked = Select::new(key, options)
        .label(name)
        .appearance(SelectAppearance::Field)
        .width(width)
        .current(options.get(current).map(String::as_str))
        .open(s.giving.open == part)
        .on_toggle(move |s: &mut App| s.giving_open(part))
        .on_close(|s: &mut App| s.giving_close_part())
        .on_choose(move |s: &mut App, word: &str, _: &mut Tx| {
            if let Some(at) = words.iter().position(|option| option == word) {
                on_pick(s, at);
            }
        })
        .build(k, layer);
    col().gap(2.0).w(width).child(label(k, name)).child(picked)
}

fn filters(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let w = &s.giving;
    let mut funds: Vec<String> = vec![t::GIVING_ALL_FUNDS().to_string()];
    funds.extend(s.funds.iter().map(|fund| fund.name.clone()));
    let at_fund = w.filter.fund.and_then(|id| s.funds.iter().position(|fund| fund.id == id)).map_or(0, |at| at + 1);
    let on_fund: fn(&mut App, usize) = |s, at| s.giving_fund_filter(at.checked_sub(1).and_then(|at| s.funds.get(at)).map(|fund| fund.id));
    let fund = list(s, k, Choice { key: "giving-fund", name: t::GIVING_COL_FUND(), width: FUND_W, part: Part::Fund, options: &funds, current: at_fund, on_pick: on_fund }, layer);
    let methods: Vec<String> = std::iter::once(t::GIVING_ALL_METHODS().to_string()).chain(Method::ALL.iter().map(|method| method_name(*method).to_string())).collect();
    let at_method = w.filter.method.and_then(|method| Method::ALL.iter().position(|each| *each == method)).map_or(0, |at| at + 1);
    let on_method: fn(&mut App, usize) = |s, at| s.giving_method_filter(at.checked_sub(1).and_then(|at| Method::ALL.get(at)).copied());
    let method = list(s, k, Choice { key: "giving-method", name: t::GIVING_COL_METHOD(), width: METHOD_W, part: Part::Method, options: &methods, current: at_method, on_pick: on_method }, layer);
    let shown = w.view.iter().filter_map(|at| s.gifts.get(*at));
    let (count, sum) = (w.view.len(), total(shown));
    row()
        .key("giving-filters")
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(dates_filter(s, k, layer))
        .child(fund)
        .child(method)
        .child(spacer())
        .child(big(k, count.to_string(), t::giving_count(count), "giving-count"))
        .child(el().w(space::S5))
        .child(big(k, rupiah(sum), t::GIVING_TOTAL().to_lowercase(), "giving-sum"))
}

/// The table's foot: Add gift and the total, which is summed from the rows shown every time.
fn summary(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let sum = total(s.giving.view.iter().filter_map(|at| s.gifts.get(*at)));
    row()
        .key("giving-summary")
        .w_full()
        .h(52.0)
        .px(space::S4)
        .gap(space::S4)
        .center_y()
        .fill(sf.header)
        .child(btn(t::GIVING_ADD()).icon(&icons::PLUS).small().hint("Ctrl N").build(k, |s, _| s.new_gift()).key("giving-add"))
        .child(crate::ui::tour::help_mark(s, k, crate::guide::lessons::Feature::Giving))
        .child(spacer())
        .child(cap(k, t::GIVING_TOTAL()))
        .child(text(rupiah(sum)).key("giving-total").font(type_::HEADING.features("'tnum' 1")).color(k.c.text_base).w(AMOUNT_W).align(Align::End))
        .child(el().w(62.0))
}

fn latest_words(s: &App) -> Option<String> {
    artosia_core::giving::latest(&s.gifts).map(|date| t::giving_latest(&date_words(date)))
}

fn main_column(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let sf = theme::surf();
    let head = row()
        .w_full()
        .gap(space::S3)
        .center_y()
        .child(text(t::TAB_GIVING()).font(type_::HEADING).color(k.c.text_base))
        .children(latest_words(s).map(|words| text(words).key("giving-latest").font(type_::BODY).color(k.c.text_muted)))
        .child(spacer())
        .child(btn(t::GIVING_FUNDS()).icon(&icons::COIN).small().build(k, |s, _| s.open_funds()).key("giving-funds"));
    let body = if s.giving.view.is_empty() {
        nothing(s, k)
    } else {
        col().key("giving-table-box").w_full().grow(1.0).min_h(0.0).radius(10.0).border(1.0, sf.edge).fill(sf.raised).clip_rounded(10.0).child(table(s, k)).child(el().h(1.0).w_full().fill(sf.edge).a11y_hidden()).child(summary(s, k))
    };
    col()
        .key("giving-main")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .padding(space::S5)
        .gap(space::S3)
        .child(head)
        .child(filters(s, k, layer))
        .child(body)
        .child(text(t::GIVING_SUMS_HINT()).font(type_::SMALL).color(k.c.text_muted))
}

/// The money slide as the card reads the table, the sum per fund, and the transfer account.
fn side(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let card = s.money_card(Fill::Offerings).cloned().unwrap_or(Announcement { title: t::ANNOUNCEMENT_TEMPLATES()[4].0.to_string(), fill: Fill::Offerings, seconds: 10, ..Announcement::default() });
    let width = SIDE - 2.0 * space::S4;
    let frame = el().key("giving-slide").size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, k.c.border_base);
    let picture = match s.card_slides(&card).into_iter().next() {
        Some(slide) => frame.child(crate::ui::thumb(s, &s.preview_state(Some(slide)), width)),
        None => frame.center().child(text(t::NOTICE_NOTHING_TO_SHOW()).font(type_::SMALL).color(k.c.text_muted)),
    };
    let shown: Vec<_> = s.giving.view.iter().filter_map(|at| s.gifts.get(*at)).cloned().collect();
    let mut funds = col().key("giving-per-fund").w_full().gap(6.0);
    for fund in &s.funds {
        funds = funds.child(row().w_full().gap(space::S2).center_y().child(dot(fund.colour, 10.0)).child(text(fund.name.clone()).font(type_::SMALL).color(k.c.text_base).truncate().grow(1.0).min_w(0.0)).child(text(rupiah(fund_sum(&shown, fund.id))).font(type_::SMALL_STRONG.features("'tnum' 1")).color(k.c.text_base)));
    }
    let account = &s.account;
    let mut lines = col().key("giving-account").w_full().gap(2.0);
    if account.bank.is_empty() && account.number.is_empty() && account.holder.is_empty() {
        lines = lines.child(text(t::GIVING_ACCOUNT_NONE()).font(type_::SMALL).color(k.c.text_muted));
    } else {
        lines = lines
            .child(text(account.bank.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0))
            .child(text(account.number.clone()).font(type_::BODY_STRONG.features("'tnum' 1")).color(k.c.text_base).truncate().min_w(0.0))
            .child(text(account.holder.clone()).font(type_::SMALL).color(k.c.text_muted).truncate().min_w(0.0))
            .children((!account.note.is_empty()).then(|| text(account.note.clone()).font(type_::SMALL).color(k.c.text_muted).wrap()));
    }
    let content = col()
        .w_full()
        .padding(space::S4)
        .gap(space::S3)
        .child(text(t::GIVING_USED_BY()).font(type_::TITLE).color(k.c.text_base))
        .child(text(t::GIVING_USED_BY_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(picture)
        .child(cap(k, t::GIVING_BY_FUND()))
        .child(funds)
        .child(btn(t::GIVING_OPEN_CARD()).icon(&icons::ARROW_RIGHT).small().build(k, |s, _| s.open_money_card(Fill::Offerings)).key("giving-open-card"))
        .child(el().h(space::S2))
        .child(text(t::GIVING_ACCOUNT()).font(type_::TITLE).color(k.c.text_base))
        .child(text(t::GIVING_ACCOUNT_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(lines)
        .child(btn(t::GIVING_ACCOUNT_EDIT()).icon(&icons::PENCIL).small().build(k, |s, _| s.edit_account()).key("giving-edit-account"));
    col().key("giving-side").w(SIDE).h_full().fill(sf.side).child(col().w_full().h_full().scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(content).scroll_bar(k))
}

pub fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) -> El {
    let sf = theme::surf();
    let tall = cx.layout_size("window").map(|size| size[1]).unwrap_or(768.0);
    gift_dialog(s, k, layer, tall);
    funds_dialog(s, k, layer, tall);
    account_dialog(s, k, layer, tall);
    let main = main_column(s, k, layer);
    row().key("giving").w_full().grow(1.0).min_h(0.0).fill(sf.work).child(main).child(el().w(1.0).h_full().fill(sf.edge).a11y_hidden()).child(side(s, k))
}

// ---------------------------------------------------------------- the gift dialog

fn edit_in(s: &mut App, pick: fn(&mut crate::giving::GiftForm) -> &mut rune::TextEdit) -> &mut rune::TextEdit {
    match s.giving.form.as_mut() {
        Some(form) => pick(form),
        None => &mut s.spare_edit,
    }
}

fn plate_toggle(k: &Kit, on: bool) -> El {
    row()
        .key("gift-plate")
        .role("checkbox")
        .label(t::GIVING_PLATE())
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
        .child(text(t::GIVING_PLATE()).font(type_::BODY).color(k.c.text_base).a11y_hidden())
        .on_press(move |s: &mut App| s.gift_plate(!on))
}

fn gift_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(form) = s.giving.form.as_ref() else { return };
    let mut popups: Vec<El> = Vec::new();
    let half = (FIELD - space::S3) / 2.0;

    let date = picker(k, "gift-date", t::GIVING_COL_DATE(), Some(icon(&icons::CALENDAR, 16.0).color(k.c.text_muted).a11y_hidden()), date_words(form.date), form.open == Part::Date).on_press(|s: &mut App| s.gift_open(Part::Date));

    let giver = TextInput::new(&form.giver)
        .width(None)
        .label(t::GIVING_COL_GIVER())
        .disabled(form.plate)
        .on_edit(|s: &mut App| edit_in(s, |form| &mut form.giver))
        .on_submit(|s: &mut App, _: &mut Tx| s.save_gift())
        .build(k);

    let fund_names: Vec<String> = s.funds.iter().map(|fund| fund.name.clone()).collect();
    let current_fund = s.fund_of(form.fund).map(|fund| fund.name.clone()).unwrap_or_default();
    let by_name = fund_names.clone();
    let fund = Select::new("gift-fund", &fund_names)
        .label(t::GIVING_COL_FUND())
        .appearance(SelectAppearance::Field)
        .width(half)
        .current(Some(current_fund.as_str()))
        .open(form.open == Part::Fund)
        .on_toggle(|s: &mut App| s.gift_open(Part::Fund))
        .on_close(|s: &mut App| s.gift_close_part())
        .on_choose(move |s: &mut App, word: &str, _: &mut Tx| {
            if let Some(id) = by_name.iter().position(|name| name == word).and_then(|at| s.funds.get(at)).map(|fund| fund.id) {
                s.gift_fund(id);
            }
        })
        .build(k, &mut popups);

    let methods: Vec<&str> = Method::ALL.iter().map(|method| method_name(*method)).collect();
    let method = Select::new("gift-method", &methods)
        .label(t::GIVING_COL_METHOD())
        .appearance(SelectAppearance::Field)
        .width(half)
        .current(Some(method_name(form.method)))
        .open(form.open == Part::Method)
        .on_toggle(|s: &mut App| s.gift_open(Part::Method))
        .on_close(|s: &mut App| s.gift_close_part())
        .on_choose(|s: &mut App, word: &str, _: &mut Tx| {
            if let Some(method) = Method::ALL.iter().find(|method| method_name(**method) == word) {
                s.gift_method(*method);
            }
        })
        .build(k, &mut popups);

    let amount = TextInput::new(&form.amount)
        .width(None)
        .label(t::GIVING_COL_AMOUNT())
        .on_edit(|s: &mut App| edit_in(s, |form| &mut form.amount))
        .on_change(|s: &mut App, _: &mut Tx| s.gift_amount_changed())
        .on_submit(|s: &mut App, _: &mut Tx| s.save_gift())
        .build(k);
    let written = text(rupiah(artosia_core::giving::parse_amount(&form.amount.text))).key("gift-amount-words").font(type_::SMALL.features("'tnum' 1")).color(k.c.text_muted);

    let mut body = col()
        .key("gift-form")
        .w_full()
        .px(space::S4)
        .pb(space::S3)
        .gap(space::S3)
        .child(field(k, t::GIVING_COL_DATE(), date));
    if form.open == Part::Date {
        let chosen = form.date;
        let grid = month_grid(s, k, "gift", form.shown, move |day| day == chosen, |_| false, |_| true, |s, by| s.gift_month(by), |s, day| s.gift_pick_date(day), el());
        let sf = theme::surf();
        body = body.child(col().key("gift-picker").w_full().padding(space::S3).radius(k.radius.md).fill(sf.raised).border(1.0, sf.edge).child(grid));
    }
    body = body
        .child(field(k, t::GIVING_COL_GIVER(), col().w_full().gap(space::S1).child(giver).child(plate_toggle(k, form.plate))))
        .child(row().w_full().gap(space::S3).child(field(k, t::GIVING_COL_FUND(), fund)).child(field(k, t::GIVING_COL_METHOD(), method)))
        .child(field(k, t::GIVING_COL_AMOUNT(), col().w_full().gap(space::S1).child(amount).child(written)));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let id = form.id;
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .children((id != 0).then(|| btn(t::BTN_DELETE()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, _| s.delete_gift(id)).key("gift-delete")))
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_gift()))
        .child(btn(t::BTN_SAVE()).primary().disabled(!form.ready()).focusable().build(k, |s, _| s.save_gift()).key("gift-save"));
    let title = if id == 0 { t::GIVING_ADD() } else { t::GIVING_EDIT() };
    Dialog::new("gift", title).open(true).width(DIALOG).fit().on_close(|s: &mut App| s.close_gift()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
    layer.extend(popups);
}

// ---------------------------------------------------------------- the funds dialog

fn funds_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(picked) = s.giving.manage else { return };
    let mut chips = row().key("funds-list").role("list").label(t::GIVING_FUNDS()).w_full().gap(space::S1).flex_wrap().focus_group().focus_follows();
    for fund in &s.funds {
        let (id, on) = (fund.id, fund.id == picked);
        chips = chips.child(
            row()
                .key(format!("fund-{id}"))
                .role("button")
                .label(fund.name.clone())
                .h(30.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(15.0)
                .fill(if on { k.c.layer_03 } else { rune::Color::TRANSPARENT })
                .border(1.0, if on { k.c.border_strong } else { k.c.border_base })
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .focusable(true)
                .focus_selected(on)
                .on_key(enter_key)
                .child(dot(fund.colour, 12.0))
                .child(text(fund.name.clone()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).a11y_hidden())
                .on_press(move |s: &mut App| s.pick_fund(id)),
        );
    }
    let name = TextInput::new(&s.giving.rename)
        .width(None)
        .label(t::GIVING_FUND_NAME())
        .on_edit(|s: &mut App| &mut s.giving.rename)
        .on_change(|s: &mut App, _: &mut Tx| s.rename_fund())
        .build(k);
    let mut body = col().w_full().px(space::S4).pb(space::S3).gap(space::S3).child(chips);
    if s.fund_of(picked).is_some() {
        body = body.child(col().w_full().gap(4.0).child(section_label(k, t::GIVING_FUND_NAME())).child(name));
        body = body.children(super::colour::rows(s, k, ColourFor::Fund(picked), t::GIVING_FUND_COLOUR(), &[]));
    }
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .child(btn(t::GIVING_ADD_FUND()).icon(&icons::PLUS).focusable().build(k, |s, _| s.add_fund()).key("funds-add"))
        .child(btn(t::GIVING_DELETE_FUND()).icon(&icons::TRASH).ghost().disabled(s.fund_of(picked).is_none()).focusable().build(k, |s, _| s.delete_fund()).key("funds-delete"))
        .child(spacer())
        .child(btn(t::BTN_CLOSE()).primary().focusable().build(k, |s, _| s.close_funds()));
    Dialog::new("giving-funds-dialog", t::GIVING_FUNDS()).open(true).width(FUNDS).fit().on_close(|s: &mut App| s.escape_funds()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}

// ---------------------------------------------------------------- the account dialog

fn edit_account_in(s: &mut App, pick: fn(&mut crate::giving::AccountForm) -> &mut rune::TextEdit) -> &mut rune::TextEdit {
    match s.giving.account.as_mut() {
        Some(form) => pick(form),
        None => &mut s.spare_edit,
    }
}

fn account_dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let Some(form) = s.giving.account.as_ref() else { return };
    let input = |edit: &rune::TextEdit, name: &str| TextInput::new(edit).width(None).label(name).on_submit(|s: &mut App, _: &mut Tx| s.save_account());
    let bank = input(&form.bank, t::GIVING_BANK()).on_edit(|s: &mut App| edit_account_in(s, |form| &mut form.bank)).build(k);
    let number = input(&form.number, t::GIVING_NUMBER()).on_edit(|s: &mut App| edit_account_in(s, |form| &mut form.number)).build(k);
    let holder = input(&form.holder, t::GIVING_HOLDER()).on_edit(|s: &mut App| edit_account_in(s, |form| &mut form.holder)).build(k);
    let note = input(&form.note, t::GIVING_NOTE()).on_edit(|s: &mut App| edit_account_in(s, |form| &mut form.note)).build(k);
    let body = col().key("account-form").w_full().px(space::S4).pb(space::S3).gap(space::S3).child(field(k, t::GIVING_BANK(), bank)).child(field(k, t::GIVING_NUMBER(), number)).child(field(k, t::GIVING_HOLDER(), holder)).child(field(k, t::GIVING_NOTE(), note));
    let scrolling = col().w_full().max_h((tall - 84.0 - 60.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    let foot = row()
        .w_full()
        .px(space::S4)
        .py(space::S3)
        .gap(space::S2)
        .center_y()
        .child(spacer())
        .child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.close_account()))
        .child(btn(t::BTN_SAVE()).primary().focusable().build(k, |s, _| s.save_account()).key("account-save"));
    Dialog::new("giving-account-dialog", t::GIVING_ACCOUNT()).open(true).width(ACCOUNT).fit().on_close(|s: &mut App| s.close_account()).child(col().w_full().child(scrolling).child(divider(k)).child(foot)).build(k, layer);
}
