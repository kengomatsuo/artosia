//! A slide's words as the typed data the poster layouts read, and the fixed Indonesian words.

use artosia_core::calendar::Date;
use artosia_core::deck::{NoticeKind, Slide, notice_body};

use super::fit;

/// The fixed words of the templates, as the designer's copy has them.
pub const WARTA: &str = "Warta jemaat";
pub const WIB: &str = "Semua jam WIB";
pub const THIS_WEEK: &str = "Pekan ini";
pub const KAS: &str = "Kas jemaat";
pub const BIRTHDAYS: &str = "Ulang tahun";
pub const SPECIAL: &str = "Khusus";
pub const TOTAL: &str = "Jumlah";
pub const TOTAL_SUFFIX: &str = "persembahan";
pub const VERSE_KICKER: &str = "Ayat Pekan Ini";
pub const NOTICE_KICKER: &str = "Pengumuman";
pub const BANK_KICKER: &str = "Rekening gereja";
pub const HOLDER_LABEL: &str = "atas nama";
pub const QRIS_CAPTION: &str = "Pindai dengan QRIS";
pub const STICKER: &str = "Info";
pub const SEP: &str = " \u{b7} ";

const MONTHS: [&str; 12] = ["Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember"];
const DAYS: [&str; 7] = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
const DOWS: [&str; 7] = ["Min", "Sen", "Sel", "Rab", "Kam", "Jum", "Sab"];

pub fn month_name(month: u8) -> &'static str {
    MONTHS[usize::from(month.clamp(1, 12)) - 1]
}

fn day_name(date: Date) -> &'static str {
    DAYS[usize::from(date.weekday() % 7)]
}

pub fn dow_of(date: Date) -> &'static str {
    DOWS[usize::from(date.weekday() % 7)]
}

/// Which pictures the slide has to draw.
#[derive(Clone, Copy)]
pub struct Have {
    pub photo: bool,
    pub photo2: bool,
    pub qris: bool,
}

pub struct Row {
    pub day: String,
    pub dow: String,
    /// 0 when the slide has no date for the row.
    pub date: u8,
    pub month: String,
    pub time: String,
    pub event: String,
    pub special: bool,
}

pub struct Name {
    pub day: u8,
    pub name: String,
    pub hl: bool,
}

pub struct Fund {
    pub label: String,
    pub rupiah: i64,
}

#[derive(Default)]
pub struct Account {
    pub bank: String,
    pub number: String,
    pub holder: String,
    pub note: String,
}

pub struct Data {
    pub kind: NoticeKind,
    pub title: String,
    pub church: String,
    pub rows: Vec<Row>,
    pub names: Vec<Name>,
    pub funds: Vec<Fund>,
    /// The card's whole total, which a part of a long card keeps; None sums the funds shown.
    pub whole: Option<i64>,
    pub account: Account,
    pub verses: Vec<(u32, String)>,
    pub reference: String,
    pub paragraphs: Vec<String>,
    /// The first and last day of the week the card stands out, as day numbers.
    pub week: Option<(u8, u8)>,
    /// Seven days from the window's first: weekday abbreviation and day number.
    pub days: Vec<(&'static str, u8)>,
    /// The month the names belong to, as a heading.
    pub month: String,
    /// "15 - 21 November 2026".
    pub week_short: String,
    /// "Minggu, 15 November 2026", when the slide has a date.
    pub date_long: Option<String>,
    /// Weekday, day and month of a notice's date.
    pub date_card: Option<(String, String, String)>,
}

fn leading_number(cell: &str) -> u8 {
    cell.split_whitespace().next().and_then(|word| word.parse().ok()).unwrap_or(0)
}

impl Data {
    pub fn of(slide: &Slide) -> Data {
        let body = notice_body(&slide.text);
        let first = slide.dates.first().copied();
        let month_of = |date: Option<Date>| date.map_or(String::new(), |date| month_name(date.month).to_string());
        let mut rows = Vec::new();
        let mut names = Vec::new();
        let mut funds = Vec::new();
        let mut before = String::new();
        let (mut month, mut week_short) = (String::new(), String::new());
        let week = slide.week.map(|(from, to)| (from.day, to.day));
        match slide.notice {
            NoticeKind::Schedule => {
                for (at, cells) in body.rows.iter().enumerate() {
                    let cell = |n: usize| cells.get(n).cloned().unwrap_or_default();
                    let date = slide.dates.get(at).copied();
                    let day = if !cell(0).is_empty() { cell(0) } else { date.map_or(before.clone(), |date| day_name(date).to_string()) };
                    before = day.clone();
                    let dow = date.map_or_else(|| day.chars().take(3).collect(), |date| dow_of(date).to_string());
                    rows.push(Row { day, dow, date: date.map_or(0, |date| date.day), month: month_of(date), time: cell(1), event: cell(2), special: slide.marked.contains(&at) });
                }
            }
            NoticeKind::Birthdays => {
                for (at, cells) in body.rows.iter().enumerate() {
                    if month.is_empty() {
                        month = cells.get(1).and_then(|cell| cell.split_whitespace().nth(1)).unwrap_or("").to_string();
                    }
                    names.push(Name { day: cells.get(1).map_or(0, |cell| leading_number(cell)), name: cells.first().cloned().unwrap_or_default(), hl: slide.marked.contains(&at) });
                }
            }
            NoticeKind::Money => {
                for cells in &body.rows {
                    funds.push(Fund { label: cells.first().cloned().unwrap_or_default(), rupiah: cells.get(1).map_or(0, |cell| fit::amount(cell)) });
                }
            }
            _ => {}
        }
        if let Some((from, to)) = slide.week {
            week_short = if from.month == to.month { format!("{} \u{2013} {} {} {}", from.day, to.day, month_name(to.month), to.year) } else { format!("{} {} \u{2013} {} {} {}", from.day, month_name(from.month), to.day, month_name(to.month), to.year) };
            if month.is_empty() {
                month = month_name(from.month).to_string();
            }
        }
        let days = slide.week.map_or_else(Vec::new, |(from, _)| (0..7).map(|k| from.plus_days(k)).map(|date| (dow_of(date), date.day)).collect());
        let date_long = first.map(|date| format!("{}, {} {} {}", day_name(date), date.day, month_name(date.month), date.year));
        let date_card = first.filter(|_| slide.notice == NoticeKind::Plain).map(|date| (day_name(date).to_string(), date.day.to_string(), month_name(date.month).to_string()));
        let account = |at: usize| body.rows.get(at).map(|cells| cells.join(" ")).unwrap_or_default();
        let paragraphs = body.rows.iter().map(|cells| cells.join(" ")).collect();
        Data {
            kind: slide.notice,
            title: slide.heading.clone(),
            church: slide.church.clone(),
            rows,
            names,
            funds,
            whole: body.total.as_ref().and_then(|cells| cells.get(1)).map(|cell| fit::amount(cell)),
            account: Account { bank: account(0), number: account(1), holder: account(2), note: body.rows.iter().skip(3).map(|cells| cells.join(" ")).collect::<Vec<_>>().join("\n") },
            verses: slide.verses.clone(),
            reference: slide.label.clone(),
            paragraphs,
            week,
            days,
            month,
            week_short,
            date_long,
            date_card,
        }
    }

    /// "Warta jemaat . Church", or the first alone.
    pub fn kicker(&self, tail: &str) -> String {
        match (self.church.is_empty(), tail.is_empty()) {
            (true, true) => WARTA.to_string(),
            (true, false) => format!("{WARTA}{SEP}{tail}"),
            (false, true) => format!("{WARTA}{SEP}{}", self.church),
            (false, false) => format!("{WARTA}{SEP}{tail}"),
        }
    }

    /// The warta kicker with the church's name, or with `fallback` when it has none.
    pub fn church_kicker(&self, fallback: &str) -> String {
        if self.church.is_empty() { self.kicker(fallback) } else { self.kicker(&self.church) }
    }

    pub fn total(&self) -> i64 {
        self.whole.unwrap_or_else(|| self.funds.iter().map(|fund| fund.rupiah).sum())
    }

    /// How many rows of the schedule fall on each of the week's days, by day number.
    pub fn count_on(&self, day: u8) -> usize {
        self.rows.iter().filter(|row| row.date == day).count() + self.names.iter().filter(|name| name.day == day).count()
    }
}

/// The range a part of a long card holds, written at the foot of its slide.
pub fn range_of(slide: &Slide) -> String {
    let data = Data::of(slide);
    match slide.notice {
        NoticeKind::Schedule => match (data.rows.first(), data.rows.last()) {
            (Some(a), Some(b)) => format!("{} {} \u{2013} {} {}", a.dow, a.date, b.dow, b.date),
            _ => String::new(),
        },
        NoticeKind::Birthdays => match (data.names.first(), data.names.last()) {
            (Some(a), Some(b)) => format!("{:02}\u{2013}{:02}", a.day, b.day),
            _ => String::new(),
        },
        _ => String::new(),
    }
}
