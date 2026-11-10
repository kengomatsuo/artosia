//! Birthdays out of a table's rows: a name, and a date that is a day plus a month. The year is
//! dropped on purpose; nobody should have ages on a screen.

use crate::docx::{Block, Docx};
use crate::pptx::Pptx;
use crate::Table;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Birthday {
    pub name: String,
    pub day: u8,
    pub month: u8,
}

/// What a read found, and what it could not make sense of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub people: Vec<Birthday>,
    /// Rows with content that gave no name or no date.
    pub unread: usize,
    /// Header rows ("No | NAMA | TANGGAL LAHIR") skipped; empty rows and rows holding only a
    /// number are skipped silently.
    pub headers: usize,
}

const MONTHS: &[(&str, u8)] = &[
    ("januari", 1), ("jan", 1), ("january", 1),
    ("februari", 2), ("pebruari", 2), ("feb", 2), ("february", 2),
    ("maret", 3), ("mar", 3), ("march", 3),
    ("april", 4), ("apr", 4),
    ("mei", 5), ("may", 5),
    ("juni", 6), ("jun", 6), ("june", 6),
    ("juli", 7), ("jul", 7), ("july", 7),
    ("agustus", 8), ("agu", 8), ("agt", 8), ("agst", 8), ("aug", 8), ("august", 8),
    ("september", 9), ("sep", 9), ("sept", 9),
    ("oktober", 10), ("okt", 10), ("oct", 10), ("october", 10),
    ("november", 11), ("nopember", 11), ("nov", 11), ("nop", 11),
    ("desember", 12), ("des", 12), ("dec", 12), ("december", 12),
];

const DAYS_IN_MONTH: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// One edit apart: a letter added, dropped or changed ("Nopvember").
fn one_edit_apart(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let skip = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let (ra, rb) = (&a[skip..], &b[skip..]);
    match ra.len().cmp(&rb.len()) {
        std::cmp::Ordering::Equal => ra[1..] == rb[1..],
        std::cmp::Ordering::Greater => ra[1..] == *rb,
        std::cmp::Ordering::Less => *ra == rb[1..],
    }
}

/// A month by name. The church's own files carry typos, so a long name one letter off still
/// counts when it can only mean one month.
fn month_name(token: &str) -> Option<u8> {
    if let Some((_, m)) = MONTHS.iter().find(|(name, _)| *name == token) {
        return Some(*m);
    }
    if token.chars().count() < 5 || !token.chars().all(char::is_alphabetic) {
        return None;
    }
    let mut near = MONTHS.iter().filter(|(name, _)| name.len() >= 5 && one_edit_apart(name, token)).map(|(_, m)| *m);
    let first = near.next()?;
    near.all(|m| m == first).then_some(first)
}

fn number(token: &str, max_digits: usize) -> Option<u32> {
    (!token.is_empty() && token.len() <= max_digits && token.bytes().all(|b| b.is_ascii_digit())).then(|| token.parse().ok()).flatten()
}

fn valid(day: u32, month: u32) -> Option<(u8, u8)> {
    ((1..=12).contains(&month) && day >= 1 && day <= u32::from(DAYS_IN_MONTH[month as usize - 1])).then_some((day as u8, month as u8))
}

/// Two, four or (a typo) five digits.
fn year(token: &str) -> bool {
    number(token, 5).is_some() && matches!(token.len(), 2 | 4 | 5)
}

/// Reads a whole string as a date: `05 Januari`, `5 Jan 1990`, `Jan 5`, `05/01`, `5-1`,
/// `05-01-1990`, `1990-01-05`. Day comes before month in numbers, as in Indonesia. A dot between
/// numbers is not a date separator, so an amount such as `10.5` never reads as 10 May.
/// Returns `(day, month)`; a year is checked and dropped.
pub fn parse_date(text: &str) -> Option<(u8, u8)> {
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | '-'))
        .map(|t| t.trim_end_matches('.'))
        .filter(|t| !t.is_empty())
        .map(|t| t.trim_end_matches("st").trim_end_matches("nd").trim_end_matches("rd").trim_end_matches("th"))
        .collect();
    // `trim_end_matches("st")` would also turn "agustus" into "agu"; month names are checked raw.
    let raw: Vec<&str> = lower.split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | '-')).map(|t| t.trim_end_matches('.')).filter(|t| !t.is_empty()).collect();
    let day_of = |i: usize| number(tokens.get(i)?, 2);
    let month_of = |i: usize| -> Option<u32> {
        let t = raw.get(i)?;
        month_name(t).map(u32::from).or_else(|| number(t, 2))
    };
    match raw.len() {
        2 => {
            // Day then month, or an English "Jan 5".
            if let (Some(d), Some(m)) = (day_of(0), month_of(1)) {
                return valid(d, m);
            }
            let m = month_name(raw[0])?;
            valid(day_of(1)?, u32::from(m))
        }
        3 => {
            if year(raw[2])
                && let (Some(d), Some(m)) = (day_of(0), month_of(1))
            {
                return valid(d, m);
            }
            if raw[0].len() == 4 && year(raw[0]) {
                // ISO order: year, month, day.
                return valid(number(raw[2], 2)?, month_of(1)?);
            }
            // English "Jan 5 1990".
            let m = month_name(raw[0])?;
            year(raw[2]).then_some(())?;
            valid(day_of(1)?, u32::from(m))
        }
        _ => None,
    }
}

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A cell that only numbers the row: `12`, `12.`, `#12`.
fn is_number_cell(cell: &str) -> bool {
    let c = cell.trim().trim_start_matches('#').trim_end_matches(['.', ')']);
    !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit())
}

fn is_header(cells: &[String]) -> bool {
    cells.iter().any(|c| {
        let c = c.to_lowercase();
        ["nama", "tanggal", "lahir", "name", "date", "birth"].iter().any(|w| c.contains(w))
    })
}

/// "Budi 5 Jan" in one cell: the date is the last two or three words.
fn trailing_date(cell: &str) -> Option<(String, u8, u8)> {
    let words: Vec<&str> = cell.split_whitespace().collect();
    for take in [3usize, 2] {
        if words.len() > take {
            let (name, date) = words.split_at(words.len() - take);
            if let Some((d, m)) = parse_date(&date.join(" ")) {
                let name = name.join(" ").trim_end_matches(['-', ',', ':', '(']).trim().to_string();
                if !name.is_empty() {
                    return Some((name, d, m));
                }
            }
        }
    }
    None
}

enum Row {
    Person(Birthday),
    Header,
    Blank,
    Unread,
}

/// `heading_month` is the month a heading before the table named; a date cell that is only a day
/// number takes it.
fn read_row(cells: &[String], heading_month: Option<u8>) -> Row {
    let cells: Vec<String> = cells.iter().map(|c| clean(c)).collect();
    // A row that only holds its number is an unfilled slot.
    if cells.iter().all(|c| c.is_empty() || is_number_cell(c)) {
        return Row::Blank;
    }
    let dated = cells.iter().position(|c| parse_date(c).is_some());
    let (date_at, (day, month)) = match dated {
        Some(at) => (at, parse_date(&cells[at]).unwrap_or((0, 0))),
        None => {
            if is_header(&cells) {
                return Row::Header;
            }
            // A bare day under a month heading, then the one-cell "Name 5 Jan" form.
            if let Some(m) = heading_month
                && let Some(at) = cells.iter().rposition(|c| !c.is_empty()).filter(|at| *at > 0 && number(&cells[*at], 2).is_some())
                && let Some((d, m)) = valid(number(&cells[at], 2).unwrap_or(0), u32::from(m))
                && let Some(name) = cells.iter().enumerate().filter(|(i, c)| *i != at && !c.is_empty() && !is_number_cell(c)).map(|(_, c)| c.clone()).next()
            {
                return Row::Person(Birthday { name, day: d, month: m });
            }
            return match cells.iter().find_map(|c| trailing_date(c)) {
                Some((name, day, month)) => Row::Person(Birthday { name, day, month }),
                None => Row::Unread,
            };
        }
    };
    let name = cells.iter().enumerate().filter(|(i, c)| *i != date_at && !c.is_empty() && !is_number_cell(c)).map(|(_, c)| c.clone()).next();
    match name {
        Some(name) => Row::Person(Birthday { name, day, month }),
        None => Row::Unread,
    }
}

fn add_table(found: &mut Found, table: &Table, heading_month: Option<u8>) {
    for row in &table.rows {
        match read_row(row, heading_month) {
            Row::Person(p) => found.people.push(p),
            Row::Header => found.headers += 1,
            Row::Blank => {}
            Row::Unread => found.unread += 1,
        }
    }
}

/// Reads every table given.
pub fn from_tables<'a>(tables: impl IntoIterator<Item = &'a Table>) -> Found {
    let mut found = Found::default();
    for t in tables {
        add_table(&mut found, t, None);
    }
    found
}

/// Reads a birthday list. A paragraph that is only a month name ("Januari") before a table
/// gives that table's month to rows whose date cell is just a day.
pub fn from_docx(doc: &Docx) -> Found {
    let mut found = Found::default();
    let mut heading = None;
    for block in &doc.blocks {
        match block {
            Block::Paragraph(p) => {
                let text = clean(&p.text).to_lowercase();
                if let Some(m) = month_name(text.trim_end_matches([':', '.'])) {
                    heading = Some(m);
                }
            }
            Block::Table(t) => add_table(&mut found, t, heading),
        }
    }
    found
}

/// Reads the tables on every slide of a bulletin.
pub fn from_pptx(deck: &Pptx) -> Found {
    from_tables(deck.slides.iter().flat_map(|s| s.tables.iter()))
}

/// Drops repeats of the same name (any letter case) on the same day and month, keeping the first.
/// Office lists often hold a person twice, once per list they were pasted into.
pub fn dedup(people: Vec<Birthday>) -> Vec<Birthday> {
    let mut seen = std::collections::HashSet::new();
    people.into_iter().filter(|p| seen.insert((p.name.to_lowercase(), p.day, p.month))).collect()
}

/// Birthdays from `from` to `to`, both `(month, day)` and inclusive. A range whose end is
/// before its start runs over New Year (Dec 28 to Jan 3). Sorted from `from` onwards by date,
/// then by name.
pub fn in_range(people: &[Birthday], from: (u8, u8), to: (u8, u8)) -> Vec<&Birthday> {
    let key = |m: u8, d: u8| u16::from(m) * 100 + u16::from(d);
    let (start, end) = (key(from.0, from.1), key(to.0, to.1));
    let mut hits: Vec<&Birthday> = people
        .iter()
        .filter(|p| {
            let k = key(p.month, p.day);
            if start <= end { (start..=end).contains(&k) } else { k >= start || k <= end }
        })
        .collect();
    // After New Year, in a wrapped range, counts as later than December.
    let order = |p: &Birthday| {
        let k = key(p.month, p.day);
        (u16::from(k < start), k)
    };
    hits.sort_by(|a, b| order(a).cmp(&order(b)).then_with(|| a.name.cmp(&b.name)));
    hits
}

/// Everyone born in `month` (1 to 12), by day then name.
pub fn in_month(people: &[Birthday], month: u8) -> Vec<&Birthday> {
    let mut hits: Vec<&Birthday> = people.iter().filter(|p| p.month == month).collect();
    hits.sort_by(|a, b| a.day.cmp(&b.day).then_with(|| a.name.cmp(&b.name)));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(name: &str, month: u8, day: u8) -> Birthday {
        Birthday { name: name.into(), day, month }
    }

    fn table(rows: &[&[&str]]) -> Table {
        Table { rows: rows.iter().map(|r| r.iter().map(|c| c.to_string()).collect()).collect() }
    }

    #[test]
    fn dates_in_both_languages_and_forms() {
        for (text, want) in [
            ("05 Januari", (5, 1)),
            ("5 Jan 1990", (5, 1)),
            ("31 Agustus", (31, 8)),
            ("2 Agu", (2, 8)),
            ("2 Agt", (2, 8)),
            ("9 Aug", (9, 8)),
            ("1 Mei", (1, 5)),
            ("14 okt", (14, 10)),
            ("3 Nop", (3, 11)),
            ("25 Des.", (25, 12)),
            ("05/01", (5, 1)),
            ("5-1", (5, 1)),
            ("05-01-1990", (5, 1)),
            ("1990-01-05", (5, 1)),
            ("Jan 5", (5, 1)),
            ("March 3rd", (3, 3)),
            ("  12   November ", (12, 11)),
            ("29 Februari", (29, 2)),
            ("12 Nopvember", (12, 11)),
            ("9 Mei 19900", (9, 5)),
        ] {
            assert_eq!(parse_date(text), Some(want), "{text}");
        }
    }

    #[test]
    fn things_that_are_not_dates() {
        for text in ["", "1", "Maria", "No", "32 Januari", "30 Februari", "5 13", "0 Mei", "10.5", "1.500.000", "TANGGAL LAHIR", "Mei", "5 Jan 199", "12 Tuesday"] {
            assert_eq!(parse_date(text), None, "{text}");
        }
    }

    #[test]
    fn a_table_gives_people_and_counts_what_it_could_not_read() {
        let t = table(&[
            &["No", "NAMA", "TANGGAL LAHIR"],
            &["1", "Anna Test", "05 Januari"],
            &["2", "Budi Contoh", "7 Feb 1980"],
            &["3", "Cici Palsu", "sometime"],
            &["", "", ""],
            &["4", "", "9 Mei"],
        ]);
        let found = from_tables([&t]);
        assert_eq!(found.people, vec![b("Anna Test", 1, 5), b("Budi Contoh", 2, 7)]);
        assert_eq!((found.unread, found.headers), (2, 1));
    }

    #[test]
    fn name_and_date_in_one_cell_and_a_bare_day_under_a_month_heading() {
        let one = table(&[&["Dodi Uji 12 Nov"], &["Eka Fiktif - 3 Mar 1999"]]);
        assert_eq!(from_tables([&one]).people, vec![b("Dodi Uji", 11, 12), b("Eka Fiktif", 3, 3)]);

        let doc = Docx {
            blocks: vec![
                Block::Paragraph(crate::docx::DocxParagraph { text: "Maret".into(), style: None }),
                Block::Table(table(&[&["1", "Fani Karang", "08"]])),
            ],
        };
        assert_eq!(from_docx(&doc).people, vec![b("Fani Karang", 3, 8)]);
    }

    #[test]
    fn a_row_number_is_never_taken_for_a_birthday_day() {
        let doc = Docx {
            blocks: vec![
                Block::Paragraph(crate::docx::DocxParagraph { text: "Maret".into(), style: None }),
                Block::Table(table(&[&["2", "Gina Rekaan", "sometime"], &["3", "", ""]])),
            ],
        };
        let found = from_docx(&doc);
        assert!(found.people.is_empty());
        assert_eq!(found.unread, 1);
    }

    #[test]
    fn range_wraps_over_new_year_and_sorts_by_date_then_name() {
        let people = vec![b("Zed", 1, 2), b("Amy", 12, 30), b("Bob", 12, 30), b("Cat", 6, 1), b("Dan", 1, 4), b("Eve", 12, 28)];
        let names: Vec<&str> = in_range(&people, (12, 28), (1, 3)).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Eve", "Amy", "Bob", "Zed"]);
        let names: Vec<&str> = in_range(&people, (1, 1), (6, 1)).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Zed", "Dan", "Cat"]);
        assert!(in_range(&people, (2, 1), (2, 28)).is_empty());
    }

    #[test]
    fn dedup_drops_exact_repeats_only() {
        let people = vec![b("Amy Test", 3, 1), b("amy test", 3, 1), b("Amy Test", 3, 2), b("Bob", 3, 1)];
        assert_eq!(dedup(people), vec![b("Amy Test", 3, 1), b("Amy Test", 3, 2), b("Bob", 3, 1)]);
    }

    #[test]
    fn a_month_by_day_then_name() {
        let people = vec![b("Zed", 12, 2), b("Amy", 12, 30), b("Bob", 12, 2), b("Cat", 6, 1)];
        let names: Vec<&str> = in_month(&people, 12).iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Bob", "Zed", "Amy"]);
    }
}
