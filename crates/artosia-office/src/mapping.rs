//! A brought-in table (a spreadsheet or a csv file) has no fixed columns. `guess` says which one
//! is the name, the date, the day, the month and the year, from the heading and from the values,
//! and `read` takes the people out under whatever mapping a person settles on.

use crate::birthdays::{Birthday, Found, birth_year, clean, is_header, is_number_cell, month_name, number, parse_date_year, valid};

/// What a column holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Ignore,
    Name,
    /// A whole date of birth in one cell.
    Date,
    Day,
    Month,
    Year,
}

impl Column {
    pub const ALL: [Column; 6] = [Column::Name, Column::Date, Column::Day, Column::Month, Column::Year, Column::Ignore];
}

/// A role for each column, and whether the first row is a heading.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Mapping {
    pub header: bool,
    pub columns: Vec<Column>,
}

/// The rows a guess looks at.
const SAMPLE: usize = 200;

fn has_letters(cell: &str) -> bool {
    cell.chars().any(char::is_alphabetic)
}

/// A row of words only: no number, no date.
fn all_words(row: &[String]) -> bool {
    let cells: Vec<&String> = row.iter().filter(|cell| !cell.trim().is_empty()).collect();
    !cells.is_empty() && cells.iter().all(|cell| has_letters(cell) && parse_date_year(cell).is_none())
}

fn has_header(rows: &[Vec<String>]) -> bool {
    match rows {
        [first, second, ..] => all_words(first) && (is_header(first) || !all_words(second)),
        _ => false,
    }
}

/// The role a heading names, if it does.
fn by_heading(heading: &str) -> Option<Column> {
    let heading = heading.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|word| heading.contains(word));
    if has(&["nama", "name"]) {
        Some(Column::Name)
    } else if has(&["tahun", "year", "thn"]) {
        Some(Column::Year)
    } else if has(&["bulan", "month"]) {
        Some(Column::Month)
    } else if heading.split_whitespace().any(|word| word == "day" || word == "hari") {
        Some(Column::Day)
    } else if has(&["tanggal", "tgl", "lahir", "birth", "date", "dob", "ulang tahun"]) {
        Some(Column::Date)
    } else {
        None
    }
}

/// The role the values of a column show; `day_taken` tells a month from a day when both fit.
fn by_values(values: &[String], day_taken: bool) -> Column {
    let mostly = |holds: &dyn Fn(&str) -> bool| values.iter().filter(|value| holds(value)).count() * 10 >= values.len() * 6;
    let numbers: Vec<u32> = values.iter().filter_map(|value| number(value, 4)).collect();
    let all_numbers = numbers.len() == values.len();
    if all_numbers && values.len() >= 3 && numbers.windows(2).all(|pair| pair[1] == pair[0] + 1) {
        // 1, 2, 3 ... numbers the rows.
        Column::Ignore
    } else if mostly(&|value| parse_date_year(value).is_some()) {
        Column::Date
    } else if all_numbers && values.iter().all(|value| birth_year(value) != 0) {
        Column::Year
    } else if all_numbers && numbers.iter().all(|n| (1..=31).contains(n)) {
        if numbers.iter().any(|n| *n > 12) || !day_taken { Column::Day } else { Column::Month }
    } else if mostly(&|value| month_name(&value.to_lowercase()).is_some()) {
        Column::Month
    } else if mostly(&|value| has_letters(value)) {
        Column::Name
    } else {
        Column::Ignore
    }
}

/// Guesses the roles of every column of `rows`, from the heading when there is one and from the
/// values otherwise. A role is given to one column only; the others are ignored.
pub fn guess(rows: &[Vec<String>]) -> Mapping {
    let rows = &rows[..rows.len().min(SAMPLE)];
    let header = has_header(rows);
    let body = &rows[usize::from(header)..];
    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut columns: Vec<Column> = Vec::with_capacity(width);
    for at in 0..width {
        let values: Vec<String> = body.iter().filter_map(|row| row.get(at)).map(|cell| clean(cell)).filter(|cell| !cell.is_empty()).collect();
        let role = if values.is_empty() {
            Column::Ignore
        } else {
            let heading = header.then(|| rows[0].get(at).map(String::as_str).unwrap_or("")).and_then(by_heading);
            match heading {
                // A column headed "Tgl" or "Tanggal" that only holds 1 to 31 is the day, not the date.
                Some(Column::Date) if values.iter().all(|value| number(value, 2).is_some_and(|n| (1..=31).contains(&n))) => Column::Day,
                Some(role) => role,
                None => by_values(&values, columns.contains(&Column::Day)),
            }
        };
        columns.push(if role != Column::Ignore && columns.contains(&role) { Column::Ignore } else { role });
    }
    Mapping { header, columns }
}

/// The people in `rows` under `mapping`. A row that holds no name or no date, or whose date is not
/// one, is counted in `unread`; empty rows and rows holding only a row number are left out silently.
pub fn read(rows: &[Vec<String>], mapping: &Mapping) -> Found {
    let at = |column: Column| mapping.columns.iter().position(|role| *role == column);
    let (name, date, day, month, year) = (at(Column::Name), at(Column::Date), at(Column::Day), at(Column::Month), at(Column::Year));
    let mut found = Found { headers: usize::from(mapping.header && !rows.is_empty()), ..Found::default() };
    for row in rows.iter().skip(usize::from(mapping.header)) {
        let cells: Vec<String> = row.iter().map(|cell| clean(cell)).collect();
        if cells.iter().all(|cell| cell.is_empty() || is_number_cell(cell)) {
            continue;
        }
        let get = |column: Option<usize>| column.and_then(|at| cells.get(at)).map_or("", String::as_str);
        let parts = || {
            let day = number(get(day), 2)?;
            let month = month_name(&get(month).to_lowercase()).map(u32::from).or_else(|| number(get(month), 2))?;
            valid(day, month).map(|(day, month)| (day, month, 0))
        };
        match (get(name), parse_date_year(get(date)).or_else(parts)) {
            ("", _) | (_, None) => found.unread += 1,
            (name, Some((day, month, from_date))) => {
                let typed = birth_year(get(year));
                found.people.push(Birthday { name: name.to_string(), day, month, year: if typed != 0 { typed } else { from_date } });
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use Column::*;

    fn rows(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter().map(|row| row.iter().map(|cell| cell.to_string()).collect()).collect()
    }

    #[test]
    fn a_heading_names_the_columns_and_a_row_number_is_ignored() {
        let table = rows(&[&["No", "NAMA", "TANGGAL LAHIR"], &["1", "Anna Test", "05 Januari 1990"], &["2", "Budi Contoh", "7 Feb"], &["3", "Cici Palsu", "9 Mei"]]);
        assert_eq!(guess(&table), Mapping { header: true, columns: vec![Ignore, Name, Date] });
    }

    #[test]
    fn with_no_heading_the_values_decide() {
        let table = rows(&[&["Anna Test", "5 Jan 1990"], &["Budi Contoh", "07/02"], &["Cici Palsu", "9 Mei"]]);
        assert_eq!(guess(&table), Mapping { header: false, columns: vec![Name, Date] });
        let apart = rows(&[&["Anna Test", "5", "1", "1990"], &["Budi Contoh", "17", "2", "1985"], &["Cici Palsu", "9", "5", "2001"]]);
        assert_eq!(guess(&apart).columns, [Name, Day, Month, Year]);
        let named = rows(&[&["Anna Test", "5", "Januari"], &["Budi Contoh", "17", "Februari"], &["Cici Palsu", "9", "Mei"]]);
        assert_eq!(guess(&named).columns, [Name, Day, Month]);
    }

    #[test]
    fn a_column_headed_date_that_holds_only_days_is_the_day() {
        let table = rows(&[&["Nama", "Tgl", "Bulan", "Tahun"], &["Anna Test", "5", "1", "1990"], &["Budi Contoh", "17", "2", "1985"]]);
        assert_eq!(guess(&table), Mapping { header: true, columns: vec![Name, Day, Month, Year] });
    }

    #[test]
    fn a_second_column_of_words_and_an_empty_column_are_ignored() {
        let table = rows(&[&["Anna Test", "Ani", "", "5 Jan"], &["Budi Contoh", "Bud", "", "7 Feb"], &["Cici Palsu", "Ci", "", "9 Mei"]]);
        assert_eq!(guess(&table).columns, [Name, Ignore, Ignore, Date]);
    }

    #[test]
    fn people_are_read_under_the_mapping_a_person_settled_on() {
        let table = rows(&[&["Nama", "Tgl", "Bulan", "Tahun"], &["Anna Test", "5", "Januari", "1990"], &["Budi Contoh", "17", "2", ""], &["Cici Palsu", "31", "2", "1980"], &["", "", "", ""], &["4", "", "", ""], &["", "9", "5", ""]]);
        let found = read(&table, &guess(&table));
        let people: Vec<(&str, u8, u8, u16)> = found.people.iter().map(|p| (p.name.as_str(), p.day, p.month, p.year)).collect();
        assert_eq!(people, [("Anna Test", 5, 1, 1990), ("Budi Contoh", 17, 2, 0)]);
        assert_eq!((found.unread, found.headers), (1, 1), "31 February is unread; a row of numbers only is an unfilled slot");
        // The same table read with the date taken as one column and the year typed apart.
        let one = rows(&[&["Anna", "5 Jan 1990", "2001"]]);
        let mapping = Mapping { header: false, columns: vec![Name, Date, Year] };
        assert_eq!(read(&one, &mapping).people[0].year, 2001, "a typed year wins over the date's");
    }
}
