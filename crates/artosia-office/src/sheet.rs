//! Reading a spreadsheet (.xlsx) or a csv file as a table of cell text, for the birthday list.
//! A date a spreadsheet holds as a number comes out as `1990-01-05`, which `parse_date` reads.

use std::path::Path;

use crate::Table;
use crate::error::Error;
use crate::package::{Package, Rel};
use crate::xml::Element;

/// Columns past this are not read: a damaged file names cells far to the right.
const MAX_COLUMNS: usize = 256;

// ---------------------------------------------------------------- csv

/// The rows of a csv file. The text is UTF-8 (a byte order mark is dropped), else Latin-1, which is
/// how a spreadsheet saves an "ANSI" file. The separator is the comma, the semicolon or the tab,
/// whichever the first line uses most.
pub fn csv_from_bytes(bytes: &[u8]) -> Table {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.trim_start_matches('\u{feff}').to_string(),
        Err(_) => bytes.iter().map(|byte| char::from(*byte)).collect(),
    };
    let separator = [',', ';', '\t'].into_iter().max_by_key(|candidate| (separators(text.lines().next().unwrap_or(""), *candidate), *candidate == ',')).unwrap_or(',');
    Table { rows: csv_rows(&text, separator) }
}

/// How many times `separator` stands outside quotes in a line.
fn separators(line: &str, separator: char) -> usize {
    let mut quoted = false;
    line.chars()
        .filter(|c| {
            if *c == '"' {
                quoted = !quoted;
            }
            !quoted && *c == separator
        })
        .count()
}

fn csv_rows(text: &str, separator: char) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut field, mut quoted) = (Vec::new(), Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' if quoted => quoted = false,
            '"' if field.is_empty() => quoted = true,
            c if c == separator && !quoted => row.push(std::mem::take(&mut field)),
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                // A blank line is no row.
                if row.len() > 1 || !row[0].trim().is_empty() {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            '\r' if !quoted => {}
            c => field.push(c),
        }
    }
    row.push(field);
    if row.len() > 1 || !row[0].trim().is_empty() {
        rows.push(row);
    }
    rows
}

// ---------------------------------------------------------------- xlsx

/// The first sheet of the workbook that holds anything.
pub fn xlsx_table(path: impl AsRef<Path>) -> Result<Table, Error> {
    read_xlsx(Package::open(path.as_ref())?)
}

pub fn xlsx_from_bytes(bytes: &[u8]) -> Result<Table, Error> {
    read_xlsx(Package::from_bytes(bytes.to_vec())?)
}

fn read_xlsx(mut package: Package) -> Result<Table, Error> {
    let main = package.rels("").into_iter().find(|rel| rel.kind == "officeDocument").map_or("xl/workbook.xml".to_string(), |rel| rel.target);
    let workbook = package.read_xml(&main)?;
    let rels = package.rels(&main);
    let part_of = |kind: &str| rels.iter().find(|rel| rel.kind == kind).map(|rel: &Rel| rel.target.clone());
    let strings = part_of("sharedStrings").and_then(|part| package.read_xml(&part).ok()).map(|sst| sst.children_named("si").map(rich_text).collect::<Vec<_>>()).unwrap_or_default();
    let dates = part_of("styles").and_then(|part| package.read_xml(&part).ok()).map(|styles| date_styles(&styles)).unwrap_or_default();
    let sheets: Vec<String> = workbook.child("sheets").into_iter().flat_map(|sheets| sheets.children_named("sheet")).filter_map(|sheet| rels.iter().find(|rel| Some(rel.id.as_str()) == sheet.prefixed("id")).map(|rel| rel.target.clone())).collect();
    for part in sheets {
        let table = sheet_table(&package.read_xml(&part)?, &strings, &dates);
        if !table.rows.is_empty() {
            return Ok(table);
        }
    }
    Ok(Table::default())
}

/// The text of a shared string or an inline one, phonetic runs left out.
fn rich_text(element: &Element) -> String {
    let mut out = String::new();
    for part in element.elements() {
        match part.name.as_str() {
            "t" => out.push_str(&part.text()),
            "r" => out.push_str(&rich_text(part)),
            _ => {}
        }
    }
    out
}

/// For each cell style, whether it formats a number as a date.
fn date_styles(styles: &Element) -> Vec<bool> {
    let custom: Vec<(u32, String)> = styles.child("numFmts").into_iter().flat_map(|formats| formats.children_named("numFmt")).filter_map(|format| Some((format.attr("numFmtId")?.parse().ok()?, format.attr("formatCode")?.to_string()))).collect();
    let formats = styles.child("cellXfs").into_iter().flat_map(|xfs| xfs.children_named("xf")).map(|xf| xf.attr("numFmtId").and_then(|id| id.parse::<u32>().ok()).unwrap_or(0));
    formats.map(|id| is_date_format(id, custom.iter().find(|(known, _)| *known == id).map(|(_, code)| code.as_str()))).collect()
}

fn is_date_format(id: u32, code: Option<&str>) -> bool {
    let Some(code) = code else { return matches!(id, 14..=22 | 27..=36 | 45..=47 | 50..=58) };
    // A day or a year letter outside quotes, brackets and escapes.
    let (mut quoted, mut bracketed, mut escaped) = (false, false, false);
    code.to_lowercase().chars().any(|c| match (c, quoted, bracketed, escaped) {
        (_, _, _, true) => {
            escaped = false;
            false
        }
        ('\\' | '_', ..) => {
            escaped = true;
            false
        }
        ('"', _, false, _) => {
            quoted = !quoted;
            false
        }
        ('[', false, ..) => {
            bracketed = true;
            false
        }
        (']', ..) => {
            bracketed = false;
            false
        }
        (c, false, false, _) => c == 'd' || c == 'y',
        _ => false,
    })
}

/// The column a cell reference such as `C12` sits in, from 0.
fn column_of(reference: &str) -> Option<usize> {
    let letters = reference.chars().take_while(char::is_ascii_alphabetic);
    let (count, number) = letters.fold((0, 0usize), |(count, number), letter| (count + 1, number * 26 + usize::from(letter.to_ascii_uppercase() as u8 - b'A' + 1)));
    (count > 0).then(|| number - 1)
}

fn sheet_table(sheet: &Element, strings: &[String], dates: &[bool]) -> Table {
    let mut rows = Vec::new();
    for row in sheet.child("sheetData").into_iter().flat_map(|data| data.children_named("row")) {
        let mut cells: Vec<String> = Vec::new();
        for cell in row.children_named("c") {
            let at = cell.attr("r").and_then(column_of).unwrap_or(cells.len());
            if at < MAX_COLUMNS {
                cells.resize(cells.len().max(at + 1), String::new());
                cells[at] = cell_text(cell, strings, dates);
            }
        }
        if cells.iter().any(|cell| !cell.trim().is_empty()) {
            rows.push(cells);
        }
    }
    Table { rows }
}

fn cell_text(cell: &Element, strings: &[String], dates: &[bool]) -> String {
    let value = cell.child("v").map(Element::text).unwrap_or_default();
    match cell.attr("t").unwrap_or("n") {
        "s" => value.trim().parse::<usize>().ok().and_then(|at| strings.get(at)).cloned().unwrap_or_default(),
        "inlineStr" => cell.child("is").map(rich_text).unwrap_or_default(),
        "n" => {
            let Ok(number) = value.trim().parse::<f64>() else { return value };
            let dated = cell.attr("s").and_then(|style| style.parse::<usize>().ok()).and_then(|style| dates.get(style)).copied().unwrap_or(false);
            if dated {
                let (year, month, day) = civil(number.floor() as i64 - 25_569);
                format!("{year:04}-{month:02}-{day:02}")
            } else if number.fract() == 0.0 && number.abs() < 1e15 {
                (number as i64).to_string()
            } else {
                value
            }
        }
        _ => value,
    }
}

/// The year, month and day `days` after 1 January 1970 (Howard Hinnant's civil-from-days).
fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_files::Zip;

    fn text(rows: &Table) -> Vec<Vec<&str>> {
        rows.rows.iter().map(|row| row.iter().map(String::as_str).collect()).collect()
    }

    #[test]
    fn csv_reads_quotes_newlines_and_the_separator_the_first_line_uses() {
        let comma = csv_from_bytes(b"Nama,Tanggal\n\"Anna, Test\",\"5 \"\"Jan\"\"\"\n\nBudi,7 Feb\r\n");
        assert_eq!(text(&comma), [vec!["Nama", "Tanggal"], vec!["Anna, Test", "5 \"Jan\""], vec!["Budi", "7 Feb"]]);
        let semicolon = csv_from_bytes("\u{feff}Nama;Tanggal\nAnna;05/01/1990".as_bytes());
        assert_eq!(text(&semicolon), [vec!["Nama", "Tanggal"], vec!["Anna", "05/01/1990"]]);
        let multiline = csv_from_bytes(b"a\tb\n\"x\ny\"\tz");
        assert_eq!(text(&multiline), [vec!["a", "b"], vec!["x\ny", "z"]]);
    }

    #[test]
    fn csv_saved_as_ansi_reads_as_latin_1() {
        let table = csv_from_bytes(b"Nama;Tanggal\nJos\xe9;5 Jan");
        assert_eq!(table.rows[1][0], "Jos\u{e9}");
    }

    const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#;
    const BOOK_RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#;

    /// A heading row of shared strings, a person with a typed date, one with a date held as a number
    /// in a date format, one with an inline string and a number in a plain format, and a gap.
    fn workbook() -> Vec<u8> {
        let book = r#"<workbook xmlns="m" xmlns:r="r"><sheets><sheet name="Jemaat" sheetId="1" r:id="rId1"/></sheets></workbook>"#;
        let strings = r#"<sst xmlns="m"><si><t>Nama</t></si><si><t>Tanggal lahir</t></si><si><r><t>Anna </t></r><r><t>Test</t></r></si><si><t>5 Januari</t></si></sst>"#;
        let styles = r#"<styleSheet xmlns="m"><numFmts count="1"><numFmt numFmtId="164" formatCode="dd\ mmmm\ yyyy"/></numFmts><cellXfs count="3"><xf numFmtId="0"/><xf numFmtId="14"/><xf numFmtId="164"/></cellXfs></styleSheet>"#;
        let sheet = r#"<worksheet xmlns="m"><sheetData>
<row r="1"><c r="A1" t="s"><v>0</v></c><c r="C1" t="s"><v>1</v></c></row>
<row r="2"><c r="A2" t="s"><v>2</v></c><c r="C2" t="s"><v>3</v></c></row>
<row r="3"><c r="A3" t="inlineStr"><is><t>Budi Contoh</t></is></c><c r="C3" s="1"><v>32878</v></c><c r="D3"><v>1990.0</v></c></row>
<row r="4"><c r="A4" t="str"><f>x</f><v>Cici Palsu</v></c><c r="C4" s="2"><v>36526</v></c></row>
<row r="5"/>
</sheetData></worksheet>"#;
        Zip::new().add("_rels/.rels", RELS.as_bytes()).add("xl/workbook.xml", book.as_bytes()).add("xl/_rels/workbook.xml.rels", BOOK_RELS.as_bytes()).add("xl/sharedStrings.xml", strings.as_bytes()).add("xl/styles.xml", styles.as_bytes()).add("xl/worksheets/sheet1.xml", sheet.as_bytes()).finish()
    }

    #[test]
    fn a_workbook_reads_strings_gaps_and_dates_held_as_numbers() {
        let table = xlsx_from_bytes(&workbook()).unwrap();
        assert_eq!(
            text(&table),
            [vec!["Nama", "", "Tanggal lahir"], vec!["Anna Test", "", "5 Januari"], vec!["Budi Contoh", "", "1990-01-05", "1990"], vec!["Cici Palsu", "", "2000-01-01"]]
        );
        // What comes out is what the birthday reader understands.
        let found = crate::read(&table.rows, &crate::guess(&table.rows));
        assert_eq!(found.people.iter().map(|p| (p.name.as_str(), p.day, p.month, p.year)).collect::<Vec<_>>(), [("Anna Test", 5, 1, 0), ("Budi Contoh", 5, 1, 1990), ("Cici Palsu", 1, 1, 2000)]);
    }

    #[test]
    fn a_zip_that_is_no_workbook_is_an_error_and_an_empty_one_is_an_empty_table() {
        let zip = Zip::new().add("hello.txt", b"x").finish();
        assert!(matches!(xlsx_from_bytes(&zip), Err(Error::MissingPart(_))));
        assert!(csv_from_bytes(b"").rows.is_empty());
    }

    #[test]
    fn a_date_format_is_told_from_a_number_format() {
        assert!(is_date_format(14, None) && is_date_format(164, Some("dd\\ mmmm\\ yyyy")) && is_date_format(165, Some("[$-421]d mmm yy")));
        assert!(!is_date_format(0, None) && !is_date_format(2, None) && !is_date_format(166, Some("0.00\"day\"")) && !is_date_format(167, Some("#,##0 [Red]")));
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }
}
