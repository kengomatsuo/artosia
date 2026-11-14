//! The Offerings window's state and what its buttons do (OVGIVING1). The funds, the gifts and the
//! transfer account live in `App` and the library; the money cards read them. This holds the
//! filters, the sort, the gift dialog, the amount being edited in its cell, the funds dialog and
//! the account dialog, and writes the library. The sums are worked out in `artosia_core::giving`
//! every time they are drawn, never kept.

use std::cmp::Ordering;

use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, Fill};
use artosia_core::giving::{Account, Filter, Fund, Gift, Giving, Method, latest, parse_amount};
use rune::TextEdit;
use rune_kit::{SortDir, TableState};

use crate::state::{App, Tab, Undo};
use crate::strings as t;

/// The column the table opens sorted by, newest first.
pub const FIRST_SORT: &str = "date";

/// The list or month that is open under the gift dialog or the filters. One at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    None,
    Date,
    Fund,
    Method,
}

/// The gift in the dialog: a new one (`id` 0) or a saved one being changed.
pub struct GiftForm {
    pub id: i64,
    pub giver: TextEdit,
    /// The gift came in the collection plate, so it has no giver.
    pub plate: bool,
    pub date: Date,
    pub fund: i64,
    pub method: Method,
    pub amount: TextEdit,
    pub open: Part,
    /// The month the open month picker shows.
    pub shown: Date,
}

impl GiftForm {
    pub fn gift(&self) -> Gift {
        Gift { id: self.id, date: self.date, giver: if self.plate { String::new() } else { self.giver.text.trim().to_string() }, fund: self.fund, method: self.method, amount: parse_amount(&self.amount.text) }
    }

    /// A fund, an amount and, for a gift that is not the plate's, a giver.
    pub fn ready(&self) -> bool {
        self.fund != 0 && parse_amount(&self.amount.text) > 0 && (self.plate || !self.giver.text.trim().is_empty())
    }
}

/// The transfer account in its dialog, four typed fields.
pub struct AccountForm {
    pub bank: TextEdit,
    pub number: TextEdit,
    pub holder: TextEdit,
    pub note: TextEdit,
}

/// The Offerings tab and what its dialogs hold.
pub struct Window {
    pub table: TableState,
    /// The gifts shown, as places in `App::gifts`, filtered and in the sort's order.
    pub view: Vec<usize>,
    pub filter: Filter,
    /// The filter list or month that is open.
    pub open: Part,
    /// The month the date filter's picker shows.
    pub shown: Date,
    /// The first end of a range is picked and the second is waited for.
    pub pending: Option<Date>,
    pub form: Option<GiftForm>,
    /// The amount being edited in its cell: the gift and what is typed.
    pub amount: Option<(i64, TextEdit)>,
    /// The funds dialog is open, on this fund.
    pub manage: Option<i64>,
    pub rename: TextEdit,
    /// A fund's colour moved and is not written to the library yet.
    pub dirty: bool,
    pub account: Option<AccountForm>,
}

impl Default for Window {
    fn default() -> Window {
        let mut table = TableState::new().key("giving-table");
        table.set_sort(FIRST_SORT, SortDir::Descending);
        Window {
            table,
            view: Vec::new(),
            filter: Filter::default(),
            open: Part::None,
            shown: Date { year: 1970, month: 1, day: 1 },
            pending: None,
            form: None,
            amount: None,
            manage: None,
            rename: TextEdit::new(""),
            dirty: false,
            account: None,
        }
    }
}

/// How two gifts stand in a column's order; ties go to the newer date, then the order they were kept.
fn order(column: &str, dir: SortDir, a: &Gift, b: &Gift, funds: &[Fund]) -> Ordering {
    let name = |gift: &Gift| funds.iter().find(|fund| fund.id == gift.fund).map(|fund| fund.name.to_lowercase()).unwrap_or_default();
    let by = match column {
        "giver" => dir.apply(a.giver.to_lowercase().cmp(&b.giver.to_lowercase())),
        "fund" => dir.apply(name(a).cmp(&name(b))),
        "method" => dir.apply(a.method.code().cmp(b.method.code())),
        "amount" => dir.apply(a.amount.cmp(&b.amount)),
        _ => dir.apply(a.date.cmp(&b.date)),
    };
    by.then_with(|| b.date.cmp(&a.date)).then(a.id.cmp(&b.id))
}

/// The whole of a month, as the quick choice of the date filter has it.
fn month_of(date: Date) -> (Date, Date) {
    (date.first_of_month(), Date { day: Date::month_days(date.year, date.month), ..date })
}

impl App {
    /// The funds, gifts and account as the library holds them, and a first fund for a library with none.
    pub fn load_giving(&mut self) {
        if self.store.funds().map(|funds| funds.is_empty()).unwrap_or(false) {
            let mut fund = Fund { id: 0, name: t::GIVING_GENERAL_FUND().to_string(), colour: crate::calendar::next_colour(0) };
            let _ = self.store.save_fund(&mut fund);
        }
        self.funds = self.store.funds().unwrap_or_default();
        self.gifts = self.store.gifts().unwrap_or_default();
        self.account = self.store.account();
        self.giving.shown = self.civil_today();
        self.refresh_giving();
    }

    /// What the money cards read.
    pub fn giving_data(&self) -> Giving<'_> {
        Giving { funds: &self.funds, gifts: &self.gifts, account: &self.account }
    }

    pub fn fund_of(&self, id: i64) -> Option<&Fund> {
        self.funds.iter().find(|fund| fund.id == id)
    }

    /// Filters and orders the gifts for the table.
    pub fn refresh_giving(&mut self) {
        let filter = self.giving.filter;
        let mut view: Vec<usize> = (0..self.gifts.len()).filter(|at| filter.keeps(&self.gifts[*at])).collect();
        let (column, dir) = self.giving.table.sort().map_or((FIRST_SORT.to_string(), SortDir::Descending), |(column, dir)| (column.to_string(), dir));
        view.sort_by(|a, b| order(&column, dir, &self.gifts[*a], &self.gifts[*b], &self.funds));
        self.giving.view = view;
    }

    /// Reads the gifts again after a write, and what the cards read with them.
    pub(crate) fn reload_gifts(&mut self) {
        self.gifts = self.store.gifts().unwrap_or_default();
        self.refresh_giving();
        self.refresh_decks();
    }

    /// The gift a row of the table is.
    pub fn gift_at(&self, row: usize) -> Option<&Gift> {
        self.giving.view.get(row).and_then(|at| self.gifts.get(*at))
    }

    // ---- Filters ----

    pub fn giving_open(&mut self, part: Part) {
        self.finish_amount();
        self.giving.open = if self.giving.open == part { Part::None } else { part };
        if part == Part::Date && self.giving.open == Part::Date {
            self.giving.shown = self.giving.filter.from.unwrap_or_else(|| latest(&self.gifts).unwrap_or_else(|| self.civil_today()));
            self.giving.pending = None;
        }
    }

    pub fn giving_close_part(&mut self) {
        self.giving.open = Part::None;
        self.giving.pending = None;
    }

    pub fn giving_month(&mut self, by: i32) {
        self.giving.shown = self.giving.shown.first_of_month().plus_months(by);
    }

    /// A day of the range picker: the first press starts a range, the second ends it.
    pub fn giving_pick_date(&mut self, date: Date) {
        match self.giving.pending.take() {
            Some(first) => {
                self.giving.filter.from = Some(first.min(date));
                self.giving.filter.to = Some(first.max(date));
                self.giving.open = Part::None;
            }
            None => {
                self.giving.filter.from = Some(date);
                self.giving.filter.to = Some(date);
                self.giving.pending = Some(date);
            }
        }
        self.refresh_giving();
    }

    pub fn giving_all_dates(&mut self) {
        self.giving.filter.from = None;
        self.giving.filter.to = None;
        self.giving_close_part();
        self.refresh_giving();
    }

    pub fn giving_this_month(&mut self) {
        let (from, to) = month_of(self.civil_today());
        self.giving.filter.from = Some(from);
        self.giving.filter.to = Some(to);
        self.giving_close_part();
        self.refresh_giving();
    }

    pub fn giving_fund_filter(&mut self, fund: Option<i64>) {
        self.giving.filter.fund = fund;
        self.giving_close_part();
        self.refresh_giving();
    }

    pub fn giving_method_filter(&mut self, method: Option<Method>) {
        self.giving.filter.method = method;
        self.giving_close_part();
        self.refresh_giving();
    }

    // ---- One gift ----

    /// A new gift on today's date, from the first fund, until the lists say otherwise.
    pub fn new_gift(&mut self) {
        self.finish_amount();
        let today = self.civil_today();
        let mut giver = TextEdit::new("");
        giver.focused = false;
        let mut amount = TextEdit::new("");
        amount.focused = true;
        let fund = self.giving.filter.fund.or_else(|| self.funds.first().map(|fund| fund.id)).unwrap_or(0);
        self.giving.form = Some(GiftForm { id: 0, giver, plate: true, date: today, fund, method: self.giving.filter.method.unwrap_or_default(), amount, open: Part::None, shown: today });
        self.close_palette();
    }

    pub fn edit_gift(&mut self, id: i64) {
        self.finish_amount();
        let Some(gift) = self.gifts.iter().find(|gift| gift.id == id).cloned() else { return };
        let mut amount = TextEdit::new(gift.amount.to_string());
        amount.focused = true;
        self.giving.form = Some(GiftForm { id, giver: TextEdit::new(gift.giver.clone()), plate: gift.giver.is_empty(), date: gift.date, fund: gift.fund, method: gift.method, amount, open: Part::None, shown: gift.date });
    }

    pub fn close_gift(&mut self) {
        self.giving.form = None;
    }

    pub fn gift_open(&mut self, part: Part) {
        let Some(form) = self.giving.form.as_mut() else { return };
        form.open = if form.open == part { Part::None } else { part };
        if form.open == Part::Date {
            form.shown = form.date;
        }
    }

    pub fn gift_close_part(&mut self) {
        if let Some(form) = self.giving.form.as_mut() {
            form.open = Part::None;
        }
    }

    pub fn gift_month(&mut self, by: i32) {
        if let Some(form) = self.giving.form.as_mut() {
            form.shown = form.shown.first_of_month().plus_months(by);
        }
    }

    pub fn gift_pick_date(&mut self, date: Date) {
        if let Some(form) = self.giving.form.as_mut() {
            form.date = date;
            form.open = Part::None;
        }
    }

    pub fn gift_fund(&mut self, fund: i64) {
        if let Some(form) = self.giving.form.as_mut() {
            form.fund = fund;
            form.open = Part::None;
        }
    }

    pub fn gift_method(&mut self, method: Method) {
        if let Some(form) = self.giving.form.as_mut() {
            form.method = method;
            form.open = Part::None;
        }
    }

    /// The plate has no giver; a person's name is typed.
    pub fn gift_plate(&mut self, plate: bool) {
        if let Some(form) = self.giving.form.as_mut() {
            form.plate = plate;
            form.giver.focused = !plate;
        }
    }

    /// The amount typed in the dialog keeps only its digits.
    pub fn gift_amount_changed(&mut self) {
        if let Some(form) = self.giving.form.as_mut() {
            keep_digits(&mut form.amount);
        }
    }

    pub fn save_gift(&mut self) {
        let Some(mut gift) = self.giving.form.as_ref().filter(|form| form.ready()).map(GiftForm::gift) else { return };
        if self.store.save_gift(&mut gift).is_ok() {
            self.giving.form = None;
            self.reload_gifts();
            self.giving.table.select(gift.id.to_string());
        }
    }

    pub fn delete_gift(&mut self, id: i64) {
        self.finish_amount();
        let Some(gift) = self.gifts.iter().find(|gift| gift.id == id).cloned() else { return };
        if self.store.delete_gift(id).is_ok() {
            self.giving.form = None;
            self.giving.table.clear_selection();
            self.reload_gifts();
            self.say(t::gift_deleted(&artosia_core::giving::rupiah(gift.amount)), Some(Undo::Gift(gift)));
        }
    }

    pub fn restore_gift(&mut self, mut gift: Gift) {
        gift.id = 0;
        if self.store.save_gift(&mut gift).is_ok() {
            self.reload_gifts();
        }
    }

    /// Ctrl N on the Offerings tab, and "Add gift" in the search.
    pub fn add_gift_here(&mut self) {
        self.open_tab(Tab::Giving);
        self.new_gift();
    }

    // ---- An amount in its cell ----

    /// A press on an amount: that cell becomes a field holding its digits, all selected.
    pub fn begin_amount(&mut self, id: i64) {
        let Some(gift) = self.gifts.iter().find(|gift| gift.id == id) else { return };
        let mut edit = TextEdit::new(if gift.amount == 0 { String::new() } else { gift.amount.to_string() });
        edit.focused = true;
        let end = edit.text.len();
        edit.set_selection(0, end);
        self.giving.amount = Some((id, edit));
    }

    /// A key in the cell's field: only digits stay, and the gift is written as it reads.
    pub fn amount_changed(&mut self) {
        let Some((id, edit)) = self.giving.amount.as_mut() else { return };
        keep_digits(edit);
        let (id, amount) = (*id, parse_amount(&edit.text));
        let Some(mut gift) = self.gifts.iter().find(|gift| gift.id == id && gift.amount != amount).cloned() else { return };
        gift.amount = amount;
        if self.store.save_gift(&mut gift).is_ok() {
            if let Some(kept) = self.gifts.iter_mut().find(|kept| kept.id == id) {
                kept.amount = amount;
            }
            self.refresh_decks();
        }
    }

    /// The field is left: the amount shows as rupiah, and the table is put in order again.
    pub fn finish_amount(&mut self) {
        if self.giving.amount.take().is_some() {
            self.refresh_giving();
        }
    }

    // ---- Funds ----

    pub fn open_funds(&mut self) {
        let id = self.funds.first().map_or(0, |fund| fund.id);
        self.pick_fund(id);
    }

    pub fn pick_fund(&mut self, id: i64) {
        self.finish_colour(true);
        self.giving.rename = TextEdit::new(self.fund_of(id).map(|fund| fund.name.clone()).unwrap_or_default());
        self.giving.rename.focused = true;
        self.giving.manage = Some(id);
    }

    pub fn close_funds(&mut self) {
        self.finish_colour(true);
        self.giving.manage = None;
        self.refresh_giving();
    }

    /// Esc or the dialog's own close: an open colour picker is cancelled first, then the dialog closes.
    pub fn escape_funds(&mut self) {
        if self.colour.open.is_some() {
            self.finish_colour(false);
        } else {
            self.close_funds();
        }
    }

    pub fn add_fund(&mut self) {
        let mut fund = Fund { id: 0, name: t::GIVING_NEW_FUND().to_string(), colour: crate::calendar::next_colour(self.funds.len()) };
        if self.store.save_fund(&mut fund).is_ok() {
            self.funds = self.store.funds().unwrap_or_default();
            self.pick_fund(fund.id);
            let end = self.giving.rename.text.len();
            self.giving.rename.set_selection(0, end);
        }
    }

    /// The name typed in the funds dialog, kept as it is typed; an empty name waits for a letter.
    pub fn rename_fund(&mut self) {
        let Some(id) = self.giving.manage else { return };
        let name = self.giving.rename.text.trim().to_string();
        if let Some(mut fund) = self.fund_of(id).cloned().filter(|fund| !name.is_empty() && fund.name != name) {
            fund.name = name;
            if self.store.save_fund(&mut fund).is_ok() {
                self.funds = self.store.funds().unwrap_or_default();
                self.refresh_decks();
            }
        }
    }

    /// Takes a fund out with its gifts; Undo puts both back.
    pub fn delete_fund(&mut self) {
        let Some(id) = self.giving.manage else { return };
        self.finish_colour(true);
        let Some(fund) = self.fund_of(id).cloned() else { return };
        let gifts: Vec<Gift> = self.gifts.iter().filter(|gift| gift.fund == id).cloned().collect();
        if self.store.delete_fund(id).is_ok() {
            self.funds = self.store.funds().unwrap_or_default();
            if self.giving.filter.fund == Some(id) {
                self.giving.filter.fund = None;
            }
            match self.funds.first().map(|first| first.id) {
                Some(next) => self.pick_fund(next),
                None => self.giving.manage = None,
            }
            self.reload_gifts();
            self.say(t::fund_deleted(&fund.name), Some(Undo::Fund(fund, gifts)));
        }
    }

    pub fn restore_fund(&mut self, mut fund: Fund, gifts: Vec<Gift>) {
        fund.id = 0;
        if self.store.save_fund(&mut fund).is_ok() {
            for mut gift in gifts {
                gift.id = 0;
                gift.fund = fund.id;
                let _ = self.store.save_gift(&mut gift);
            }
            self.funds = self.store.funds().unwrap_or_default();
            self.reload_gifts();
        }
    }

    /// Writes a fund whose colour moved in the picker, once a frame at most.
    pub fn save_fund_colours(&mut self) {
        self.giving.dirty = false;
        for mut fund in self.funds.clone() {
            let _ = self.store.save_fund(&mut fund);
        }
        self.refresh_decks();
    }

    // ---- The account ----

    pub fn edit_account(&mut self) {
        let account = self.account.clone();
        let mut bank = TextEdit::new(account.bank);
        bank.focused = true;
        self.giving.account = Some(AccountForm { bank, number: TextEdit::new(account.number), holder: TextEdit::new(account.holder), note: TextEdit::new(account.note) });
    }

    pub fn close_account(&mut self) {
        self.giving.account = None;
    }

    pub fn save_account(&mut self) {
        let Some(form) = self.giving.account.as_ref() else { return };
        let account = Account { bank: form.bank.text.trim().to_string(), number: form.number.text.trim().to_string(), holder: form.holder.text.trim().to_string(), note: form.note.text.trim().to_string() };
        if self.store.set_account(&account).is_ok() {
            self.account = account;
            self.giving.account = None;
            self.refresh_decks();
        }
    }

    // ---- The cards ----

    /// The announcement that reads the gifts, or the account: the saved card for it, else None.
    pub fn money_card(&self, fill: Fill) -> Option<&Announcement> {
        self.announcements.iter().find(|card| card.fill == fill)
    }

    /// "Open the card" beside a slide of the side panel: the announcement editor on that card.
    pub fn open_money_card(&mut self, fill: Fill) {
        let card = self.money_card(fill).cloned();
        self.open_tab(Tab::Notices);
        if card.is_some() {
            self.notices = Some(App::notices_for(card.as_ref()));
        }
    }
}

/// The field keeps only its digits.
fn keep_digits(edit: &mut TextEdit) {
    if edit.text.chars().any(|c| !c.is_ascii_digit()) {
        let digits: String = edit.text.chars().filter(char::is_ascii_digit).collect();
        edit.replace_text(digits);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use artosia_core::giving::{rupiah, total};

    fn first_fund(app: &App) -> i64 {
        app.funds.first().map(|fund| fund.id).expect("a fund")
    }

    fn add(app: &mut App, giver: &str, date: Date, amount: &str) -> i64 {
        app.new_gift();
        let fund = first_fund(app);
        if let Some(form) = app.giving.form.as_mut() {
            form.plate = giver.is_empty();
            form.giver = TextEdit::new(giver);
            form.date = date;
            form.fund = fund;
            form.amount = TextEdit::new(amount);
        }
        app.save_gift();
        app.gifts.iter().find(|gift| gift.date == date && gift.giver == giver && gift.amount == parse_amount(amount)).map(|gift| gift.id).expect("saved")
    }

    fn shown(app: &App) -> Vec<i64> {
        app.giving.view.iter().map(|at| app.gifts[*at].amount).collect()
    }

    #[test]
    fn the_practice_library_has_gifts_over_two_sundays_in_six_funds_and_an_account() {
        let app = App::practice();
        assert_eq!(app.funds.len(), 6);
        let days: std::collections::BTreeSet<Date> = app.gifts.iter().map(|gift| gift.date).collect();
        assert_eq!(days.len(), 2);
        assert!(days.iter().all(|day| day.weekday() == 0), "Sundays");
        assert!(app.gifts.iter().all(|gift| app.fund_of(gift.fund).is_some() && gift.amount > 0));
        assert!(app.gifts.iter().any(|gift| gift.giver.is_empty()) && app.gifts.iter().any(|gift| !gift.giver.is_empty()));
        assert!(app.account.ready() && app.account.bank == "Bank Contoh");
        assert_eq!(app.giving.view.len(), app.gifts.len());
        let dates: Vec<Date> = app.giving.view.iter().map(|at| app.gifts[*at].date).collect();
        assert!(dates.windows(2).all(|pair| pair[0] >= pair[1]), "newest first");
    }

    #[test]
    fn the_table_filters_by_range_fund_and_method_and_sorts_by_a_column() {
        let mut app = App::practice();
        let all = app.gifts.len();
        let newest = latest(&app.gifts).expect("gifts");
        let older = app.gifts.iter().map(|gift| gift.date).min().expect("gifts");
        // Two presses make a range, the second in either order; the first alone is one day.
        app.giving_pick_date(newest);
        assert_eq!((app.giving.filter.from, app.giving.filter.to), (Some(newest), Some(newest)));
        assert!(app.giving.view.iter().all(|at| app.gifts[*at].date == newest));
        app.giving_pick_date(older);
        assert_eq!((app.giving.filter.from, app.giving.filter.to), (Some(older), Some(newest)));
        assert_eq!(app.giving.view.len(), all);
        app.giving_all_dates();
        let fund = app.funds[2].id;
        app.giving_fund_filter(Some(fund));
        assert!(app.giving.view.iter().all(|at| app.gifts[*at].fund == fund) && !app.giving.view.is_empty());
        app.giving_fund_filter(None);
        app.giving_method_filter(Some(Method::Cash));
        assert!(app.giving.view.iter().all(|at| app.gifts[*at].method == Method::Cash) && app.giving.view.len() < all);
        app.giving_method_filter(None);
        for dir in [SortDir::Ascending, SortDir::Descending] {
            app.giving.table.set_sort("amount", dir);
            app.refresh_giving();
            let amounts = shown(&app);
            assert!(amounts.windows(2).all(|pair| if dir == SortDir::Ascending { pair[0] <= pair[1] } else { pair[0] >= pair[1] }));
        }
    }

    #[test]
    fn the_total_and_the_fund_sums_add_themselves_from_the_rows() {
        let mut app = App::practice();
        let fund = first_fund(&app);
        let before = total(&app.gifts);
        let before_fund = artosia_core::giving::fund_sum(&app.gifts, fund);
        add(&mut app, "Sum Giver", Date { year: 2026, month: 12, day: 6 }, "123456789012");
        assert_eq!(total(&app.gifts), before + 123_456_789_012);
        assert_eq!(artosia_core::giving::fund_sum(&app.gifts, fund), before_fund + 123_456_789_012);
        assert_eq!(rupiah(total(app.giving.view.iter().map(|at| &app.gifts[*at]))), rupiah(total(&app.gifts)));
    }

    #[test]
    fn the_dialog_adds_edits_and_deletes_with_undo() {
        let mut app = App::practice();
        let before = app.gifts.len();
        app.new_gift();
        app.save_gift();
        assert!(app.giving.form.is_some() && app.gifts.len() == before, "a gift with no amount is not saved");
        let day = Date { year: 2026, month: 12, day: 6 };
        let id = add(&mut app, "", day, "1500000");
        assert!(app.giving.form.is_none() && app.gifts.len() == before + 1);
        app.edit_gift(id);
        app.gift_plate(false);
        app.save_gift();
        assert!(app.giving.form.is_some(), "a named gift needs its giver");
        if let Some(form) = app.giving.form.as_mut() {
            form.giver = TextEdit::new("  Named Giver ");
            form.amount = TextEdit::new("2.500x");
        }
        app.gift_amount_changed();
        app.gift_method(Method::Transfer);
        app.save_gift();
        let saved = app.gifts.iter().find(|gift| gift.id == id).expect("kept");
        assert_eq!((saved.giver.as_str(), saved.amount, saved.method), ("Named Giver", 2500, Method::Transfer));
        app.delete_gift(id);
        assert_eq!(app.gifts.len(), before);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert!(app.gifts.iter().any(|gift| gift.giver == "Named Giver" && gift.amount == 2500 && gift.date == day));
        assert_eq!(app.giving.view.len(), app.gifts.len(), "the table follows");
    }

    #[test]
    fn an_amount_is_edited_in_its_cell_and_every_key_writes_it() {
        let mut app = App::practice();
        let id = app.gifts[0].id;
        app.begin_amount(id);
        if let Some((_, edit)) = app.giving.amount.as_mut() {
            edit.replace_text("1.2a3");
        }
        app.amount_changed();
        assert_eq!(app.giving.amount.as_ref().map(|(_, edit)| edit.text.as_str()), Some("123"), "only digits stay");
        assert_eq!(app.gifts.iter().find(|gift| gift.id == id).map(|gift| gift.amount), Some(123));
        assert_eq!(app.store.gifts().expect("read").iter().find(|gift| gift.id == id).map(|gift| gift.amount), Some(123), "written to the library");
        app.finish_amount();
        assert!(app.giving.amount.is_none());
    }

    #[test]
    fn a_fund_goes_with_its_gifts_and_undo_brings_both_back_with_its_colour() {
        let mut app = App::practice();
        let (funds, gifts) = (app.funds.len(), app.gifts.len());
        let fund = app.funds[1].clone();
        let its = app.gifts.iter().filter(|gift| gift.fund == fund.id).count();
        assert!(its > 0);
        app.open_funds();
        app.pick_fund(fund.id);
        app.delete_fund();
        assert_eq!((app.funds.len(), app.gifts.len()), (funds - 1, gifts - its));
        app.undo();
        let back = app.funds.iter().find(|kept| kept.name == fund.name).expect("restored");
        assert_eq!(back.colour, fund.colour);
        assert_eq!((app.funds.len(), app.gifts.len()), (funds, gifts));
        assert_eq!(app.gifts.iter().filter(|gift| gift.fund == back.id).count(), its);
    }

    #[test]
    fn a_fund_colour_is_picked_in_the_one_picker_and_written_once() {
        use crate::ui::colour::ColourFor;
        let mut app = App::practice();
        let id = app.funds[0].id;
        app.toggle_colour(ColourFor::Fund(id));
        app.pick_colour([10, 20, 30]);
        assert!(app.giving.dirty);
        app.save_fund_colours();
        assert_eq!(app.store.funds().expect("read")[0].colour, [10, 20, 30]);
        app.finish_colour(true);
    }

    #[test]
    fn the_money_cards_read_the_data_and_follow_an_edit() {
        let mut app = App::practice();
        let offering = Announcement { title: "Offering".into(), fill: Fill::Offerings, seconds: 10, ..Announcement::default() };
        let transfer = Announcement { title: "Transfer".into(), fill: Fill::Transfer, seconds: 10, ..Announcement::default() };
        let slide = |app: &App, card: &Announcement| app.card_slides(card).into_iter().next();
        let first = slide(&app, &offering).expect("the newest Sunday has gifts");
        let newest = latest(&app.gifts).expect("gifts");
        let expected = total(app.gifts.iter().filter(|gift| gift.date == newest));
        assert!(first.text.contains(&format!("== Total\t{}", rupiah(expected))), "{}", first.text);
        let id = app.gifts.iter().find(|gift| gift.date == newest).map(|gift| gift.id).expect("a gift");
        app.begin_amount(id);
        if let Some((_, edit)) = app.giving.amount.as_mut() {
            edit.replace_text("1");
        }
        app.amount_changed();
        assert_ne!(slide(&app, &offering).expect("still has gifts").text, first.text, "the card follows the edit");
        let account = slide(&app, &transfer).expect("the practice account");
        assert!(account.text.starts_with("Bank Contoh\n"));
        app.account = Account::default();
        assert!(slide(&app, &transfer).is_none(), "no account, no slide");
    }

    #[test]
    fn a_new_library_has_one_fund_and_no_gifts() {
        let mut app = App::practice();
        for gift in app.gifts.clone() {
            let _ = app.store.delete_gift(gift.id);
        }
        for fund in app.funds.clone() {
            let _ = app.store.delete_fund(fund.id);
        }
        app.load_giving();
        assert_eq!((app.funds.len(), app.gifts.len()), (1, 0));
        assert!(app.giving.view.is_empty());
    }
}
