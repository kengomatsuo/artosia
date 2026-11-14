//! What the Offerings window keeps and works out: funds with a name and a colour, gifts in whole
//! rupiah, and the church's transfer account. A total is summed every time and never stored.

use serde::{Deserialize, Serialize};

use crate::calendar::Date;

/// What a gift is given for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fund {
    pub id: i64,
    pub name: String,
    pub colour: [u8; 3],
}

/// How a gift came in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    #[default]
    Cash,
    Transfer,
}

impl Method {
    pub const ALL: [Method; 2] = [Method::Cash, Method::Transfer];

    /// The word stored in the library.
    pub fn code(self) -> &'static str {
        match self {
            Method::Cash => "cash",
            Method::Transfer => "transfer",
        }
    }

    pub fn from_code(code: &str) -> Method {
        if code == "transfer" { Method::Transfer } else { Method::Cash }
    }
}

/// One gift: money in whole rupiah, the date it came in, and who gave it (empty for the plate).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gift {
    pub id: i64,
    pub date: Date,
    pub giver: String,
    pub fund: i64,
    pub method: Method,
    pub amount: i64,
}

/// The account a church asks for transfers on, kept once in the library: names and a number as
/// the bank prints them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub bank: String,
    pub number: String,
    pub holder: String,
    pub note: String,
}

impl Account {
    /// The three lines a slide cannot do without are written.
    pub fn ready(&self) -> bool {
        [&self.bank, &self.number, &self.holder].iter().all(|line| !line.trim().is_empty())
    }
}

/// Everything the money cards read.
#[derive(Clone, Copy, Debug)]
pub struct Giving<'a> {
    pub funds: &'a [Fund],
    pub gifts: &'a [Gift],
    pub account: &'a Account,
}

/// An amount as the church writes it: "Rp" and the digits in groups of three with dots.
pub fn rupiah(amount: i64) -> String {
    let digits = amount.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 4);
    if amount < 0 {
        out.push('-');
    }
    out.push_str("Rp ");
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push('.');
        }
        out.push(digit);
    }
    out
}

/// The amount typed in a field: only its digits count, and too many stop at the largest amount.
pub fn parse_amount(text: &str) -> i64 {
    text.chars().filter_map(|c| c.to_digit(10)).fold(0i64, |sum, digit| sum.saturating_mul(10).saturating_add(i64::from(digit)))
}

/// The sum of gifts; a sum too big for the figure stops at the largest.
pub fn total<'a>(gifts: impl IntoIterator<Item = &'a Gift>) -> i64 {
    gifts.into_iter().fold(0i64, |sum, gift| sum.saturating_add(gift.amount))
}

/// What one fund received.
pub fn fund_sum(gifts: &[Gift], fund: i64) -> i64 {
    total(gifts.iter().filter(|gift| gift.fund == fund))
}

/// The date of the newest gift.
pub fn latest(gifts: &[Gift]) -> Option<Date> {
    gifts.iter().map(|gift| gift.date).max()
}

/// The filters over the gifts: a range of dates, one fund, one method; None is all.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub from: Option<Date>,
    pub to: Option<Date>,
    pub fund: Option<i64>,
    pub method: Option<Method>,
}

impl Filter {
    pub fn keeps(&self, gift: &Gift) -> bool {
        self.from.is_none_or(|from| gift.date >= from) && self.to.is_none_or(|to| gift.date <= to) && self.fund.is_none_or(|fund| gift.fund == fund) && self.method.is_none_or(|method| gift.method == method)
    }
}

/// The lines of an offering card, as `notice_body` reads them: each fund that received something
/// on the newest date with its sum, and the total of those lines under them. None when no gift is
/// kept. `total_word` names the total in the window's language.
pub fn offering_text(giving: &Giving, total_word: &str) -> Option<String> {
    let day = latest(giving.gifts)?;
    let on_day: Vec<Gift> = giving.gifts.iter().filter(|gift| gift.date == day).cloned().collect();
    let sums: Vec<(&str, i64)> = giving.funds.iter().map(|fund| (fund.name.as_str(), fund_sum(&on_day, fund.id))).filter(|(_, sum)| *sum != 0).collect();
    if sums.is_empty() {
        return None;
    }
    let all = sums.iter().fold(0i64, |sum, (_, each)| sum.saturating_add(*each));
    let mut lines: Vec<String> = sums.iter().map(|(name, sum)| format!("{name}\t{}", rupiah(*sum))).collect();
    lines.push(format!("== {total_word}\t{}", rupiah(all)));
    Some(lines.join("\n"))
}

/// The lines of a transfer card: bank, number, holder and the note when there is one. None while
/// the account lacks one of the first three.
pub fn transfer_text(account: &Account) -> Option<String> {
    if !account.ready() {
        return None;
    }
    let mut lines = vec![account.bank.trim(), account.number.trim(), account.holder.trim()];
    if !account.note.trim().is_empty() {
        lines.push(account.note.trim());
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: u8) -> Date {
        Date { year: 2026, month: 11, day }
    }

    fn gift(day: u8, fund: i64, method: Method, amount: i64) -> Gift {
        Gift { id: 0, date: date(day), giver: String::new(), fund, method, amount }
    }

    fn funds() -> Vec<Fund> {
        vec![Fund { id: 1, name: "Tithe".into(), colour: [1, 2, 3] }, Fund { id: 2, name: "Missions".into(), colour: [4, 5, 6] }, Fund { id: 3, name: "Empty".into(), colour: [7, 8, 9] }]
    }

    #[test]
    fn an_amount_is_written_with_dots_and_rp() {
        for (amount, written) in [
            (0, "Rp 0"),
            (7, "Rp 7"),
            (999, "Rp 999"),
            (1_000, "Rp 1.000"),
            (12_345, "Rp 12.345"),
            (100_000, "Rp 100.000"),
            (2_500_000, "Rp 2.500.000"),
            (123_456_789_012, "Rp 123.456.789.012"),
            (i64::MAX, "Rp 9.223.372.036.854.775.807"),
            (-1_500, "-Rp 1.500"),
            (i64::MIN, "-Rp 9.223.372.036.854.775.808"),
        ] {
            assert_eq!(rupiah(amount), written);
        }
    }

    #[test]
    fn a_typed_amount_keeps_its_digits_and_stops_at_the_largest() {
        assert_eq!(parse_amount("Rp 2.500.000"), 2_500_000);
        assert_eq!(parse_amount(""), 0);
        assert_eq!(parse_amount("abc"), 0);
        assert_eq!(parse_amount("0007"), 7);
        assert_eq!(parse_amount("99999999999999999999999"), i64::MAX);
        assert_eq!(parse_amount(&rupiah(123_456_789_012)), 123_456_789_012);
    }

    #[test]
    fn sums_are_worked_out_from_the_rows_and_do_not_overflow() {
        let gifts = vec![gift(1, 1, Method::Cash, 100), gift(1, 2, Method::Transfer, 250), gift(8, 1, Method::Cash, 50)];
        assert_eq!(total(&gifts), 400);
        assert_eq!(fund_sum(&gifts, 1), 150);
        assert_eq!(fund_sum(&gifts, 3), 0);
        let big = vec![gift(1, 1, Method::Cash, 900_000_000_000), gift(1, 1, Method::Cash, 99_999_999_999)];
        assert_eq!(rupiah(total(&big)), "Rp 999.999.999.999");
        assert_eq!(total(&[gift(1, 1, Method::Cash, i64::MAX), gift(1, 1, Method::Cash, 1)]), i64::MAX);
        assert_eq!(total(&[]), 0);
    }

    #[test]
    fn the_filters_keep_a_range_a_fund_and_a_method() {
        let gifts = [gift(1, 1, Method::Cash, 1), gift(8, 1, Method::Transfer, 2), gift(15, 2, Method::Cash, 3)];
        let kept = |filter: Filter| gifts.iter().filter(|gift| filter.keeps(gift)).map(|gift| gift.amount).collect::<Vec<_>>();
        assert_eq!(kept(Filter::default()), [1, 2, 3]);
        assert_eq!(kept(Filter { from: Some(date(8)), ..Filter::default() }), [2, 3]);
        assert_eq!(kept(Filter { from: Some(date(8)), to: Some(date(8)), ..Filter::default() }), [2]);
        assert_eq!(kept(Filter { fund: Some(1), ..Filter::default() }), [1, 2]);
        assert_eq!(kept(Filter { method: Some(Method::Cash), fund: Some(2), ..Filter::default() }), [3]);
        assert_eq!(kept(Filter { fund: Some(9), ..Filter::default() }), Vec::<i64>::new());
    }

    #[test]
    fn the_offering_card_lists_the_funds_of_the_newest_day_and_their_total() {
        let account = Account::default();
        let funds = funds();
        let gifts = vec![gift(1, 1, Method::Cash, 999), gift(8, 1, Method::Cash, 1_000), gift(8, 2, Method::Transfer, 250_000), gift(8, 1, Method::Transfer, 500)];
        let giving = Giving { funds: &funds, gifts: &gifts, account: &account };
        assert_eq!(offering_text(&giving, "Total").as_deref(), Some("Tithe\tRp 1.500\nMissions\tRp 250.000\n== Total\tRp 251.500"));
        let none = Giving { gifts: &[], ..giving };
        assert_eq!(offering_text(&none, "Total"), None);
        let stray = vec![gift(8, 9, Method::Cash, 5)];
        assert_eq!(offering_text(&Giving { gifts: &stray, ..giving }, "Total"), None, "a gift of no known fund has no line");
    }

    #[test]
    fn the_transfer_card_needs_bank_number_and_holder() {
        let mut account = Account { bank: "Bank Contoh".into(), number: "0123 4567 89".into(), holder: "Gereja Contoh".into(), note: String::new() };
        assert_eq!(transfer_text(&account).as_deref(), Some("Bank Contoh\n0123 4567 89\nGereja Contoh"));
        account.note = "Write the fund".into();
        assert_eq!(transfer_text(&account).as_deref(), Some("Bank Contoh\n0123 4567 89\nGereja Contoh\nWrite the fund"));
        account.number = "  ".into();
        assert_eq!(transfer_text(&account), None);
    }

    #[test]
    fn five_thousand_gifts_are_filtered_and_summed_in_a_frame() {
        let gifts: Vec<Gift> = (0..5000).map(|n| gift((n % 28 + 1) as u8, i64::from(n % 6), if n % 2 == 0 { Method::Cash } else { Method::Transfer }, 10_000 + i64::from(n))).collect();
        let filter = Filter { from: Some(date(5)), to: Some(date(20)), fund: None, method: Some(Method::Cash) };
        let started = std::time::Instant::now();
        let kept: Vec<Gift> = gifts.iter().filter(|gift| filter.keeps(gift)).cloned().collect();
        let sum = total(&kept);
        let sums: i64 = (0..6).map(|fund| fund_sum(&kept, fund)).sum();
        let took = started.elapsed();
        eprintln!("giving: 5000 gifts filtered and summed in {took:?}");
        assert_eq!(sum, sums);
        assert!(took.as_millis() < 16, "took {took:?}");
    }
}
