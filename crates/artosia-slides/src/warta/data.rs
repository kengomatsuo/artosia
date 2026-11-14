//! A slide's words as the typed data the poster layouts read, and the fixed Indonesian words.

use artosia_core::calendar::Date;
use artosia_core::deck::{NoticeKind, NoticeWords, Slide, notice_body};

use super::fit;

/// The fixed words of the templates, as the designer's copy has them.
pub const WARTA: &str = "Warta jemaat";
pub const WIB: &str = "Semua jam WIB";
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



/// The fixed words a style writes: the slide's own, in the window's language, else the design's Indonesian.
pub struct Words {
    pub warta: String,
    pub wib: String,
    pub special: String,
    pub offerings: String,
    pub birthdays: String,
    pub total: String,
    pub total_gifts: String,
    pub verse: String,
    pub notice: String,
    pub account: String,
    pub holder: String,
    pub scan: String,
    pub sticker: String,
    months: Vec<String>,
    weekdays: Vec<String>,
}

impl Words {
    pub fn of(words: &NoticeWords) -> Words {
        let or = |given: &str, design: &str| if given.trim().is_empty() { design.to_string() } else { given.to_string() };
        let kind = |at: usize, design: &str| or(words.kinds.get(at).map_or("", String::as_str), design);
        let list = |given: &[String], design: &[&str]| if given.len() == design.len() { given.to_vec() } else { design.iter().map(|word| word.to_string()).collect() };
        Words {
            warta: or(&words.church_news, WARTA),
            wib: or(&words.local_time, WIB),
            special: or(&words.special, SPECIAL),
            offerings: kind(2, KAS),
            birthdays: kind(1, BIRTHDAYS),
            total: or(&words.total, TOTAL),
            total_gifts: or(&words.total_gifts, &format!("{TOTAL} {TOTAL_SUFFIX}")),
            verse: kind(4, VERSE_KICKER),
            notice: kind(3, NOTICE_KICKER),
            account: or(&words.account, BANK_KICKER),
            holder: or(&words.holder, HOLDER_LABEL),
            scan: or(&words.scan, QRIS_CAPTION),
            sticker: or(&words.sticker, STICKER),
            months: list(&words.months, &MONTHS),
            weekdays: list(&words.weekdays, &DAYS),
        }
    }

    pub fn month(&self, month: u8) -> &str {
        &self.months[usize::from(month.clamp(1, 12)) - 1]
    }

    /// A month's short name, as a day beside it writes it: "Okt".
    pub fn month_short(&self, month: u8) -> String {
        self.month(month).chars().take(3).collect()
    }

    pub fn day_name(&self, date: Date) -> &str {
        &self.weekdays[usize::from(date.weekday() % 7)]
    }

    /// A weekday's first letters, as a week's strip heads its days.
    pub fn dow(&self, date: Date) -> String {
        self.day_name(date).chars().take(3).collect()
    }

    /// Two days as a range in words: "8\u{2013}14 November" in one month, "29 Nov \u{2013} 5 Des" across two.
    pub fn range(&self, from: Date, to: Date) -> String {
        if from.year == to.year && from.month == to.month {
            if from.day == to.day { format!("{} {}", from.day, self.month(from.month)) } else { format!("{}\u{2013}{} {}", from.day, to.day, self.month(to.month)) }
        } else {
            format!("{} {} \u{2013} {} {}", from.day, self.month_short(from.month), to.day, self.month_short(to.month))
        }
    }
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
    /// The month's short name, written beside the day on a page whose names cross a month.
    pub month: String,
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
    /// The fixed words, in the window's language.
    pub w: Words,
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
    /// Seven days from the window's first, only when the window is one week: weekday abbreviation and day number.
    pub days: Vec<(String, u8)>,
    /// The month the names belong to, as a heading; "Oktober \u{2013} November" when they cross one.
    pub month: String,
    /// Whether the names of this page fall in more than one month, so each day carries its month.
    pub spans: bool,
    /// The highlighted days, else the window's, as words: "8\u{2013}14 November".
    pub week_label: String,
    /// Whether every name of this page falls inside those days.
    pub covers: bool,
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
        let words = Words::of(&slide.words);
        let first = slide.dates.first().copied();
        let month_of = |date: Option<Date>| date.map_or(String::new(), |date| words.month(date.month).to_string());
        let mut rows = Vec::new();
        let mut names = Vec::new();
        let mut funds = Vec::new();
        let mut before = String::new();
        let (mut month, mut week_short) = (String::new(), String::new());
        match slide.notice {
            NoticeKind::Schedule => {
                for (at, cells) in body.rows.iter().enumerate() {
                    let cell = |n: usize| cells.get(n).cloned().unwrap_or_default();
                    let date = slide.dates.get(at).copied();
                    let day = if !cell(0).is_empty() { cell(0) } else { date.map_or(before.clone(), |date| words.day_name(date).to_string()) };
                    before = day.clone();
                    let dow = date.map_or_else(|| day.chars().take(3).collect(), |date| words.dow(date));
                    rows.push(Row { day, dow, date: date.map_or(0, |date| date.day), month: month_of(date), time: cell(1), event: cell(2), special: slide.marked.contains(&at) });
                }
            }
            NoticeKind::Birthdays => {
                for (at, cells) in body.rows.iter().enumerate() {
                    if month.is_empty() && slide.dates.is_empty() {
                        month = cells.get(1).and_then(|cell| cell.split_whitespace().nth(1)).unwrap_or("").to_string();
                    }
                    let date = slide.dates.get(at).copied();
                    names.push(Name { day: date.map_or_else(|| cells.get(1).map_or(0, |cell| leading_number(cell)), |date| date.day), month: date.map_or_else(String::new, |date| words.month_short(date.month)), name: cells.first().cloned().unwrap_or_default(), hl: slide.marked.contains(&at) });
                }
                // The page's heading names its own months, never only its first row's.
                if let (Some(first), Some(last)) = (slide.dates.first(), slide.dates.last()) {
                    month = if (first.year, first.month) == (last.year, last.month) { words.month(first.month).to_string() } else { format!("{} \u{2013} {}", words.month(first.month), words.month(last.month)) };
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
            week_short = if from.month == to.month { format!("{} \u{2013} {} {} {}", from.day, to.day, words.month(to.month), to.year) } else { format!("{} {} \u{2013} {} {} {}", from.day, words.month(from.month), to.day, words.month(to.month), to.year) };
            if month.is_empty() {
                month = words.month(from.month).to_string();
            }
        }
        // A strip of seven days means one week, so a longer or shorter range has none.
        let days = slide.week.filter(|(from, to)| from.plus_days(6) == *to).map_or_else(Vec::new, |(from, _)| (0..7).map(|k| from.plus_days(k)).map(|date| (words.dow(date), date.day)).collect());
        let spans = slide.notice == NoticeKind::Birthdays && slide.dates.windows(2).any(|pair| (pair[0].year, pair[0].month) != (pair[1].year, pair[1].month));
        let week_label = slide.week.map_or_else(String::new, |(from, to)| words.range(from, to));
        let covers = slide.week.is_some_and(|(from, to)| slide.dates.iter().all(|date| *date >= from && *date <= to));
        let date_long = first.map(|date| format!("{}, {} {} {}", words.day_name(date), date.day, words.month(date.month), date.year));
        let date_card = first.filter(|_| slide.notice == NoticeKind::Plain).map(|date| (words.day_name(date).to_string(), date.day.to_string(), words.month(date.month).to_string()));
        let account = |at: usize| body.rows.get(at).map(|cells| cells.join(" ")).unwrap_or_default();
        let paragraphs = body.rows.iter().map(|cells| cells.join(" ")).collect();
        Data {
            w: words,
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
            days,
            month,
            spans,
            week_label,
            covers,
            week_short,
            date_long,
            date_card,
        }
    }

    /// "Warta jemaat . Church", or the first alone.
    pub fn kicker(&self, tail: &str) -> String {
        match (self.church.is_empty(), tail.is_empty()) {
            (true, true) => self.w.warta.clone(),
            (true, false) => format!("{}{SEP}{tail}", self.w.warta),
            (false, true) => format!("{}{SEP}{}", self.w.warta, self.church),
            (false, false) => format!("{}{SEP}{tail}", self.w.warta),
        }
    }

    /// The warta kicker with the church's name, or with `fallback` when it has none.
    pub fn church_kicker(&self, fallback: &str) -> String {
        if self.church.is_empty() { self.kicker(fallback) } else { self.kicker(&self.church) }
    }

    /// Whether this page has a highlighted name, so its range is worth a chip.
    pub fn highlighted(&self) -> bool {
        !self.week_label.is_empty() && self.names.iter().any(|name| name.hl)
    }

    /// A name's day as its row writes it: "08", or "08 Okt" on a page that crosses a month.
    pub fn day_of(&self, name: &Name) -> String {
        if self.spans && !name.month.is_empty() { format!("{:02} {}", name.day, name.month) } else { format!("{:02}", name.day) }
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
            (Some(a), Some(b)) if data.spans => format!("{} \u{2013} {}", data.day_of(a), data.day_of(b)),
            (Some(a), Some(b)) => format!("{:02}\u{2013}{:02}", a.day, b.day),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_names_its_month_once_and_both_months_across_one() {
        let day = |month: u8, day: u8| Date { year: 2026, month, day };
        let design = Words::of(&NoticeWords::default());
        assert_eq!(design.range(day(11, 8), day(11, 14)), "8\u{2013}14 November");
        assert_eq!(design.range(day(11, 29), day(12, 5)), "29 Nov \u{2013} 5 Des");
        assert_eq!(design.range(day(11, 15), day(11, 15)), "15 November");
        // A window in English writes English months and the church news line in English.
        let english = Words::of(&NoticeWords { months: ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"].map(String::from).to_vec(), church_news: "Church news".into(), ..NoticeWords::default() });
        assert_eq!(english.range(day(11, 29), day(12, 5)), "29 Nov \u{2013} 5 Dec");
        assert_eq!(english.warta, "Church news");
        assert_eq!(english.holder, HOLDER_LABEL, "an empty word keeps the design's");
    }

    #[test]
    fn a_birthday_page_across_two_months_writes_each_day_with_its_month() {
        let day = |month: u8, day: u8| Date { year: 2026, month, day };
        let slide = Slide { notice: NoticeKind::Birthdays, text: "Ana\t31 Oktober\nBudi\t1 November".into(), dates: vec![day(10, 31), day(11, 1)], marked: vec![1], week: Some((day(11, 1), day(11, 7))), ..Slide::default() };
        let data = Data::of(&slide);
        assert!(data.spans && data.highlighted() && !data.covers);
        assert_eq!(data.month, "Oktober \u{2013} November");
        assert_eq!(data.day_of(&data.names[0]), "31 Okt");
        assert_eq!(range_of(&slide), "31 Okt \u{2013} 01 Nov");
    }
}
