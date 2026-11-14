//! Writes the projector's state for every birthday card setting in every announcement style, one
//! JSON file per slide, built by the same functions the operator uses (`announcement_pages`, then
//! `paginate_pages`). `tools/notice-matrix.sh` draws each file with `--scene state-file` and lays
//! the pictures out as contact sheets.
//!
//!     cargo run -p artosia-output --example notice_matrix -- <folder> [id|en]
//!
//! Names and dates are made up for Artosia; no church's list is read.

use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, Fill, NoticeStyle, Person, Today, announcement_pages};
use artosia_core::giving::{Account, Giving};
use artosia_core::window::{Range, Window};
use artosia_slides::frame::OutputState;
use artosia_slides::look::presets;

const FIRST: [&str; 12] = ["Yohana", "Daniel", "Maria", "Stefanus", "Ruth", "Yosua", "Lidia", "Samuel", "Debora", "Timotius", "Ester", "Natanael"];
const LAST: [&str; 10] = ["Siregar", "Wijaya", "Tanoto", "Lumban", "Hutapea", "Santoso", "Gunawan", "Pardede", "Halim", "Kusuma"];
const LONG: [&str; 4] = ["Theresia Margaretha Kusumawardhani Simanjuntak", "Bartholomeus Christian Hadinata Purba", "Fransiska Natalia Oktaviani Rumahorbo", "Immanuel Josephine Setiabudi Lumbantobing"];

/// `count` people, the nth born on the nth of `days` spread evenly from `from`.
fn people(count: usize, from: Date, days: usize, long: bool) -> Vec<Person> {
    (0..count)
        .map(|at| {
            let date = from.plus_days((at * days / count.max(1)) as i64);
            let name = if long && at % 3 == 0 { LONG[at / 3 % LONG.len()].to_string() } else { format!("{} {}", FIRST[at % FIRST.len()], LAST[(at * 7 + at / 12) % LAST.len()]) };
            Person { id: at as i64 + 1, name, day: date.day, month: date.month, year: 0 }
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(args.get(1).expect("a folder to write into"));
    let language = args.get(2).map(String::as_str).unwrap_or("id");
    std::fs::create_dir_all(&out).expect("the folder");
    let months_id = ["Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember"];
    let months_en = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let weekdays_id = ["Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu", "Minggu"];
    let weekdays_en = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let (months, weekdays, title) = if language == "en" { (&months_en, &weekdays_en, "Birthdays") } else { (&months_id, &weekdays_id, "Ulang Tahun") };
    // A Sunday, the day a card is shown.
    let today = Today { year: 2026, month: 11, day: 15, months, weekdays, clock: |h, m| format!("{h:02}.{m:02}"), date: |d, m| format!("{d} {m}"), total: "Total", first_weekday: 0, church: "GKI Contoh" };
    let nov = |day: u8| Date { year: 2026, month: 11, day };
    let week = Range::Week(0);
    let cases: Vec<(&str, Window, Vec<Person>)> = vec![
        ("a-week", Window { range: week, highlight: None }, people(7, nov(15), 7, false)),
        ("b-week-one", Window { range: week, highlight: None }, people(1, nov(18), 1, false)),
        ("c-month-past7", Window { range: Range::Month(0), highlight: Some(Range::Past(7)) }, people(30, nov(1), 30, false)),
        ("d-month-week", Window { range: Range::Month(0), highlight: Some(week) }, people(30, nov(1), 30, false)),
        ("e-month-45", Window { range: Range::Month(0), highlight: Some(week) }, people(45, nov(1), 30, false)),
        ("f-quarter", Window { range: Range::Quarter(0), highlight: Some(Range::Month(0)) }, people(90, Date { year: 2026, month: 10, day: 1 }, 92, false)),
        ("g-coming14", Window { range: Range::Coming(14), highlight: Some(Range::Coming(7)) }, people(12, nov(15), 14, false)),
        ("h-past7", Window { range: Range::Past(7), highlight: None }, people(5, nov(8), 7, false)),
        ("i-custom", Window { range: Range::Custom(nov(1), nov(30)), highlight: Some(Range::Custom(nov(10), nov(16))) }, people(20, nov(1), 30, false)),
        ("j-long-names", Window { range: Range::Month(0), highlight: Some(week) }, people(12, nov(1), 30, true)),
        ("k-next-month", Window { range: Range::Month(1), highlight: None }, people(8, Date { year: 2026, month: 12, day: 1 }, 31, false)),
        ("l-year-end", Window { range: Range::Custom(Date { year: 2026, month: 12, day: 20 }, Date { year: 2027, month: 1, day: 10 }), highlight: None }, people(9, Date { year: 2026, month: 12, day: 20 }, 22, false)),
        ("m-thirty-one", Window { range: Range::Month(0), highlight: Some(week) }, people(31, nov(1), 30, false)),
    ];
    let styles: [(&str, Option<NoticeStyle>, &str); 8] = [
        ("1-look-midnight", None, "Midnight"),
        ("2-look-paper", None, "Paper"),
        ("3-ledger", Some(NoticeStyle::Ledger), "Midnight"),
        ("4-band", Some(NoticeStyle::Band), "Midnight"),
        ("5-greige", Some(NoticeStyle::Greige), "Midnight"),
        ("6-pill", Some(NoticeStyle::Pill), "Midnight"),
        ("7-neon", Some(NoticeStyle::Neon), "Midnight"),
        ("8-split", Some(NoticeStyle::Split), "Midnight"),
    ];
    let account = Account::default();
    let giving = Giving { funds: &[], gifts: &[], account: &account };
    let mut written = 0;
    for (case, window, folk) in &cases {
        for (code, style, look_name) in styles {
            let card = Announcement { id: 1, title: title.to_string(), fill: Fill::Birthdays, style, window: *window, seconds: 12, ..Announcement::default() };
            let look = presets().into_iter().find(|look| look.name == look_name).unwrap_or_default();
            let pages = announcement_pages(std::slice::from_ref(&card), folk, &[], &giving, &today, None, 2);
            let pages = artosia_slides::warta::paginate_pages(pages, look.notice_style);
            if pages.is_empty() {
                println!("{case} {code}: no slide");
            }
            for (at, (_, slide)) in pages.into_iter().enumerate() {
                let state = OutputState { slide: Some(slide), look: look.clone(), ..OutputState::default() };
                let path = out.join(format!("{case}__{code}__{language}__{}.json", at + 1));
                std::fs::write(&path, serde_json::to_string(&state).expect("json")).expect("write");
                written += 1;
            }
        }
    }
    println!("{written} slides written to {}", out.display());
}
