//! Redraw mapping: which table of a bulletin is a birthday list and which is a money table, with
//! typed rows for each. A money table needs a header row naming an amount column and a giver or
//! fund column; amounts are whole rupiah.

use crate::birthdays::{Found, from_tables, parse_date_year};
use crate::pptx::Pptx;
use crate::Table;

/// One line of a money table. Empty text when the table has no such column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gift {
    /// Day, month and year (0 when the cell gives none), when the date column reads.
    pub date: Option<(u8, u8, u16)>,
    pub giver: String,
    pub fund: String,
    pub method: String,
    /// Whole rupiah.
    pub amount: i64,
}

/// A money table read: its gifts and the total the table printed, to compare with their sum.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Money {
    pub gifts: Vec<Gift>,
    /// The amount on the table's total row, when it has one.
    pub printed_total: Option<i64>,
    /// Rows with content but no amount, left out.
    pub unread: usize,
}

impl Money {
    /// The sum of the gifts.
    pub fn sum(&self) -> i64 {
        self.gifts.iter().map(|g| g.amount).sum()
    }

    /// Whether the printed total equals the sum; `None` when the table printed none.
    pub fn total_matches(&self) -> Option<bool> {
        self.printed_total.map(|t| t == self.sum())
    }
}

/// What a table is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableKind {
    Birthdays(Found),
    Money(Money),
    Other,
}

/// A table of a bulletin and what it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mapped {
    /// 1-based slide number.
    pub slide: usize,
    pub kind: TableKind,
}

/// Every table of the bulletin, classified.
pub fn redraw(deck: &Pptx) -> Vec<Mapped> {
    deck.slides.iter().flat_map(|s| s.tables.iter().map(|t| Mapped { slide: s.number, kind: classify(t) })).collect()
}

/// A money table is checked first: its date and name columns would also read as birthdays.
pub fn classify(table: &Table) -> TableKind {
    if let Some(money) = read_money(table) {
        return TableKind::Money(money);
    }
    let found = from_tables([table]);
    if found.people.is_empty() { TableKind::Other } else { TableKind::Birthdays(found) }
}

/// Whole rupiah from `Rp 2.500.000`, `2.500.000,-`, `Rp2,500,000`, `2.500.000,00`, `2500000`.
/// Anything else (a date, a word, a negative) is `None`.
pub fn parse_rupiah(text: &str) -> Option<i64> {
    let mut s = text.trim().to_lowercase();
    for prefix in ["rp.", "rp", "idr"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.trim_start().to_string();
            break;
        }
    }
    let s = s.trim_end_matches(['-', '/', ',', '.', ' ']).trim_end();
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',') || !s.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    // A final separator followed by one or two digits is the decimal part: cents are dropped.
    let whole = match s.rfind(['.', ',']) {
        Some(i) if matches!(s.len() - i - 1, 1 | 2) => &s[..i],
        _ => s,
    };
    let groups: Vec<&str> = whole.split(['.', ',']).collect();
    if groups.len() > 1 {
        let (first, rest) = groups.split_first()?;
        if first.is_empty() || first.len() > 3 || rest.iter().any(|g| g.len() != 3) {
            return None;
        }
    }
    groups.concat().parse().ok()
}

#[derive(Default)]
struct Columns {
    date: Option<usize>,
    giver: Option<usize>,
    fund: Option<usize>,
    method: Option<usize>,
    amount: Option<usize>,
}

fn has_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

/// Reads one header cell's role into `cols`; the first column of a role wins.
fn assign(cols: &mut Columns, at: usize, header: &str) {
    let h = header.to_lowercase();
    let h = h.trim();
    let slot = if has_any(h, &["jumlah", "nominal", "amount", "nilai", "rupiah"]) || matches!(h, "rp" | "idr" | "rp.") {
        &mut cols.amount
    } else if has_any(h, &["tanggal", "tgl", "date"]) {
        &mut cols.date
    } else if has_any(h, &["cara", "metode", "method", "via"]) {
        &mut cols.method
    } else if has_any(h, &["persembahan", "jenis", "kategori", "fund", "keperluan", "peruntukan", "keterangan", "untuk"]) {
        &mut cols.fund
    } else if has_any(h, &["nama", "pemberi", "penyumbang", "donatur", "giver", "donor", "name"]) {
        &mut cols.giver
    } else {
        return;
    };
    slot.get_or_insert(at);
}

fn is_total_label(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    ["total", "grand total", "jumlah", "jumlah total", "subtotal", "sub total", "jumlah keseluruhan", "total keseluruhan"].iter().any(|w| t == *w || t.strip_prefix(w).is_some_and(|r| r.starts_with([' ', ':'])))
}

fn read_money(table: &Table) -> Option<Money> {
    // The header is among the first rows and names an amount column and a giver or fund column.
    let (header_at, cols) = table.rows.iter().take(3).enumerate().find_map(|(i, row)| {
        let mut cols = Columns::default();
        for (at, cell) in row.iter().enumerate() {
            assign(&mut cols, at, cell);
        }
        (cols.amount.is_some() && (cols.giver.is_some() || cols.fund.is_some())).then_some((i, cols))
    })?;
    let amount_at = cols.amount?;
    let text = |row: &[String], col: Option<usize>| col.and_then(|c| row.get(c)).map(|c| c.trim().to_string()).unwrap_or_default();
    let mut money = Money::default();
    for row in &table.rows[header_at + 1..] {
        if row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let amount = row.get(amount_at).and_then(|c| parse_rupiah(c));
        if row.iter().enumerate().any(|(at, c)| at != amount_at && is_total_label(c)) {
            // Printed total: the amount column, else the last amount in the row.
            money.printed_total = amount.or_else(|| row.iter().rev().find_map(|c| parse_rupiah(c)));
            continue;
        }
        match amount {
            Some(amount) => money.gifts.push(Gift {
                date: cols.date.and_then(|c| row.get(c)).and_then(|c| parse_date_year(c)),
                giver: text(row, cols.giver),
                fund: text(row, cols.fund),
                method: text(row, cols.method),
                amount,
            }),
            None => money.unread += 1,
        }
    }
    (!money.gifts.is_empty() || money.printed_total.is_some()).then_some(money)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[&[&str]]) -> Table {
        Table { rows: rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect() }
    }

    #[test]
    fn amounts_in_the_forms_a_church_types() {
        for (text, want) in [
            ("Rp 2.500.000", Some(2_500_000)),
            ("2.500.000,-", Some(2_500_000)),
            ("Rp2,500,000", Some(2_500_000)),
            ("Rp. 150.000", Some(150_000)),
            ("2.500.000,00", Some(2_500_000)),
            ("2,500,000.50", Some(2_500_000)),
            ("500,00", Some(500)),
            ("2.500", Some(2_500)),
            ("2,500", Some(2_500)),
            ("2500000", Some(2_500_000)),
            ("IDR 1.000", Some(1_000)),
            ("0", Some(0)),
            ("12.11.2026", None),
            ("12.11.26", None),
            ("-500", None),
            ("Budi", None),
            ("", None),
            ("Rp", None),
        ] {
            assert_eq!(parse_rupiah(text), want, "{text}");
        }
    }

    fn offerings() -> Table {
        table(&[
            &["Tanggal", "Nama", "Jenis Persembahan", "Cara", "Jumlah"],
            &["12 Nov 2026", "Giver One", "Perpuluhan", "Tunai", "Rp 2.500.000"],
            &["12 Nov 2026", "Giver Two", "Syukur", "Transfer", "1.000.000,-"],
            &["", "", "", "", ""],
            &["", "Total", "", "", "Rp 3.500.000"],
        ])
    }

    #[test]
    fn a_money_table_gives_gifts_and_its_printed_total() {
        let TableKind::Money(m) = classify(&offerings()) else { panic!("not money") };
        assert_eq!(m.gifts.len(), 2);
        assert_eq!(m.gifts[0], Gift { date: Some((12, 11, 2026)), giver: "Giver One".into(), fund: "Perpuluhan".into(), method: "Tunai".into(), amount: 2_500_000 });
        assert_eq!(m.sum(), 3_500_000);
        assert_eq!(m.printed_total, Some(3_500_000));
        assert_eq!(m.total_matches(), Some(true));
    }

    #[test]
    fn a_printed_total_that_does_not_match_is_reported() {
        let mut t = offerings();
        t.rows[4][4] = "Rp 3.000.000".into();
        let TableKind::Money(m) = classify(&t) else { panic!("not money") };
        assert_eq!((m.sum(), m.printed_total, m.total_matches()), (3_500_000, Some(3_000_000), Some(false)));
    }

    #[test]
    fn a_fund_only_table_without_a_total_row() {
        let t = table(&[&["Persembahan", "Jumlah"], &["Perpuluhan", "Rp2,500,000"], &["Kolekte", "Rp 300.000"], &["Catatan", "-"]]);
        let TableKind::Money(m) = classify(&t) else { panic!("not money") };
        assert_eq!((m.gifts.len(), m.unread, m.printed_total, m.total_matches()), (2, 1, None, None));
        assert_eq!(m.gifts[0].giver, "");
    }

    #[test]
    fn a_birthday_table_is_birthdays_not_money() {
        let t = table(&[&["No", "NAMA", "TANGGAL LAHIR"], &["1", "Test Person", "05 Januari"], &["2", "Other Person", "6 Feb"]]);
        let TableKind::Birthdays(found) = classify(&t) else { panic!("not birthdays") };
        assert_eq!(found.people.len(), 2);
    }

    #[test]
    fn a_dated_name_and_amount_table_is_money_even_though_it_would_read_as_birthdays() {
        let t = table(&[&["Tanggal", "Nama", "Jumlah"], &["5 Jan", "Giver One", "Rp 100.000"]]);
        assert!(matches!(classify(&t), TableKind::Money(_)));
    }

    #[test]
    fn a_schedule_is_neither() {
        let t = table(&[&["Time", "Event"], &["08.00", "Service one"]]);
        assert_eq!(classify(&t), TableKind::Other);
    }

    #[test]
    fn redraw_names_the_slide_of_each_table() {
        let deck = Pptx::from_bytes(&crate::test_files::pptx_bytes()).unwrap();
        let mapped = redraw(&deck);
        assert_eq!(mapped.len(), 1);
        assert_eq!(mapped[0].kind, TableKind::Other);
    }
}
