//! Where the library lives, and the practice library a first run and every snapshot starts with.

use std::path::PathBuf;

use artosia_core::deck::Announcement;
use artosia_core::song::Song;
use artosia_core::store::Store;

/// The value after a command-line flag, such as `--data <folder>`.
pub fn flag_value(flag: &str) -> Option<String> {
    let args = rune::extra_args();
    args.iter().position(|arg| arg == flag).and_then(|at| args.get(at + 1)).cloned()
}

pub fn has_flag(flag: &str) -> bool {
    rune::extra_args().iter().any(|arg| arg == flag)
}

/// Where the library is, and a direct download's library to copy into it the first time.
pub struct Library {
    pub folder: PathBuf,
    pub copy_from: Option<PathBuf>,
}

/// The library folder: `--data`, then `ARTOSIA_DATA`, then the user's app-data folder, or
/// Documents\Artosia when packaged (`packaged_library`). None in a headless run without
/// `--data`, which uses the practice library in memory so snapshots are the same on every
/// machine and never touch a real library. A new packaged folder is filled from the direct
/// download's first.
pub fn folder() -> Option<PathBuf> {
    let library = library()?;
    if let Some(from) = &library.copy_from {
        // A copy that fails leaves no folder, so the next start tries again; the old folder stays.
        let _ = copy_library(from, &library.folder);
    }
    Some(library.folder)
}

/// `folder()` without touching the disk.
pub fn library() -> Option<Library> {
    if let Some(folder) = flag_value("--data") {
        return Some(Library { folder: PathBuf::from(folder), copy_from: None });
    }
    if rune::is_headless() || has_flag("--practice") {
        return None;
    }
    if let Some(folder) = std::env::var_os("ARTOSIA_DATA") {
        return Some(Library { folder: PathBuf::from(folder), copy_from: None });
    }
    let direct = rune::dirs::AppDirs::new("artosia").map(|dirs| dirs.data().to_path_buf());
    if crate::store::packaged()
        && let Some(library) = packaged_library(crate::store::documents_folder(), direct.clone())
    {
        return Some(library);
    }
    direct.map(|folder| Library { folder, copy_from: None })
}

/// The packaged library lives in Documents because Windows deletes a packaged program's private
/// AppData copy on uninstall, and a church's songs must outlive that. The direct download's folder
/// is offered for a one-time copy while the new folder does not exist. None when Documents is
/// unknown.
fn packaged_library(documents: Option<PathBuf>, direct: Option<PathBuf>) -> Option<Library> {
    let folder = documents?.join("Artosia");
    let copy_from = direct.filter(|from| !folder.exists() && from.is_dir());
    Some(Library { folder, copy_from })
}

/// Copies a library folder as one step: into a sibling first, renamed when whole, so a copy that
/// stops half way is never mistaken for the library.
fn copy_library(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let target = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_tree(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }
    let partial = to.with_extension("copying");
    let _ = std::fs::remove_dir_all(&partial);
    if let Err(error) = copy_tree(from, &partial) {
        let _ = std::fs::remove_dir_all(&partial);
        return Err(error);
    }
    std::fs::rename(&partial, to)
}

/// Opens the library, creating its folder. Falls back to memory when the disk refuses, so
/// the app still opens; the caller shows why.
pub fn open_store(folder: Option<&PathBuf>) -> (Store, Option<String>) {
    let in_memory = |problem: Option<String>| match Store::in_memory() {
        Ok(store) => (store, problem),
        Err(error) => panic!("SQLite could not open a database in memory: {error}"),
    };
    let Some(folder) = folder else { return in_memory(None) };
    if let Err(error) = std::fs::create_dir_all(folder.join("bibles")) {
        return in_memory(Some(format!("{}: {error}", folder.display())));
    }
    match Store::open(&folder.join("library.db")) {
        Ok(store) => (store, None),
        Err(error) => in_memory(Some(error.to_string())),
    }
}

fn practice_song(number: u32, title: &str, author: &str, order: &str, lyrics: &str) -> Song {
    Song { number, title: title.into(), author: author.into(), order: order.into(), lyrics: lyrics.into(), ..Song::default() }
}

/// Songs written for Artosia so a new library is never empty. They are placeholders to
/// practise with, not songs anyone sings.
pub fn practice_songs() -> Vec<Song> {
    vec![
        practice_song(
            1,
            "Morning on the Hill",
            "Artosia practice song",
            "V1 C V2 C B C",
            "[1]\nFirst light breaks over the valley\nWe wake and lift our voices\nThe night is gone, the day is here\nAnd every shadow turns to gold\n[chorus]\nSing it out across the water\nEvery heart together now\nFrom the hill to the open sea\nLet the morning carry our song\n[2]\nThe road is long, the well is deep\nYet you have been our keeper\nThrough every storm, through every calm\nYour kindness goes before us\n[bridge]\nWe will not be moved\nWe will not be moved\nYou are with us still\n",
        ),
        practice_song(
            2,
            "River of Mercy",
            "Artosia practice song",
            "",
            "[1]\nDown by the river we gather\nMercy flowing like the rain\nAll that was broken made whole again\nAll that was lost brought home\n[chorus]\nLet the river carry us home\nLet the river carry us home\n[2]\nOver the stones and the deep places\nYour hand has never let go\nWhat can we bring but our thanks to you\nWhat can we say but amen\n",
        ),
        practice_song(
            3,
            "Quiet Harbour",
            "Artosia practice song",
            "",
            "[1]\nStill waters hold the evening\nThe wind has gone to rest\nHere in the quiet harbour\nMy soul has found its rest\n[2]\nThe lamps are lit along the shore\nThe boats are safely in\nAnd I will fear no evil\nFor you have brought me in\n[bridge]\nAnchor deep, anchor sure\nHold me fast for evermore\n",
        ),
        practice_song(
            4,
            "Hill of Morning Stars",
            "Artosia practice song",
            "",
            "[1]\nNight is over, stars are fading\nGlory rising in the east\nAll creation stands and wonders\nAt the coming of the King\n[chorus]\nHoly, holy is your name\nHigher than the mountains\nHoly, holy is your name\nDeeper than the sea\n",
        ),
        practice_song(
            5,
            "Bread for the Road",
            "Artosia practice song",
            "",
            "[1]\nYou set a table in the wild\nBread for the road and water from stone\nWe came with nothing in our hands\nAnd we went out with more than our own\n[chorus]\nEnough for today, enough for tomorrow\nYour faithfulness is new every morning\n",
        ),
        practice_song(
            6,
            "Kasih Setia-Mu",
            "Artosia practice song",
            "",
            "[1]\nKasih setia-Mu baru tiap pagi\nBesar kesetiaan-Mu ya Tuhan\nDi lembah dan di puncak gunung\nEngkau tetap menyertai\n[chorus]\nKu bersyukur, ku bersyukur\nAtas kasih-Mu yang tak berkesudahan\n",
        ),
        // One song in three languages, Indonesian first, so the practice mode shows them together.
        artosia_core::practice::three_languages(),
    ]
}

/// The practice calendar: made-up groups and events, as a church of this kind might keep: two
/// Sunday services, Sunday school, a Wednesday prayer, a Friday youth night, a Saturday rehearsal
/// and a communion on the first Sunday of each month. Names are the made-up church's own, so they
/// stay as they are in every window language.
pub fn practice_calendar(store: &Store) {
    use artosia_core::calendar::{Calendar, Date, Event, Position, Repeat};
    let groups = ["Ibadah umum", "Sekolah Minggu", "Pemuda", "Doa", "Musik"];
    let mut ids = Vec::new();
    for (at, name) in groups.iter().enumerate() {
        let mut calendar = Calendar { id: 0, name: (*name).into(), colour: crate::calendar::next_colour(at) };
        let _ = store.save_calendar(&mut calendar);
        ids.push(calendar.id);
    }
    let sunday = Date { year: 2026, month: 1, day: 4 };
    let weekly = |day: u8| Repeat::Weekly(1 << day);
    // Group, name, weekday (0 is Sunday), start and end in minutes.
    let rows: [(usize, &str, u8, u16, u16); 6] = [
        (0, "Ibadah Raya 1", 0, 6 * 60 + 30, 8 * 60),
        (0, "Ibadah Raya 2", 0, 9 * 60, 10 * 60 + 30),
        (1, "Sekolah Minggu", 0, 9 * 60, 10 * 60),
        (3, "Ibadah doa", 3, 18 * 60 + 30, 19 * 60 + 30),
        (2, "Persekutuan pemuda", 5, 18 * 60 + 30, 20 * 60),
        (4, "Latihan pujian", 6, 17 * 60, 18 * 60 + 30),
    ];
    for (group, name, day, start, end) in rows {
        let mut event = Event { id: 0, calendar: ids[group], name: name.into(), date: sunday.plus_days(i64::from(day)), start, end, repeat: weekly(day), until: None, skipped: Vec::new() };
        let _ = store.save_event(&mut event);
    }
    let mut communion = Event { id: 0, calendar: ids[0], name: "Perjamuan kudus".into(), date: Date { year: 2026, month: 1, day: 1 }, start: 11 * 60, end: 12 * 60, repeat: Repeat::Monthly(Position::Nth(1), 0), until: None, skipped: Vec::new() };
    let _ = store.save_event(&mut communion);
}

/// The practice members: thirty made-up names (Greek letters, as the practice songs' titles are)
/// with birthdays spread over the year from today, so a render shows one today, some this month
/// and a few years left out. Names are the made-up church's own, the same in every language.
pub fn practice_people(store: &Store) {
    use artosia_core::calendar::Date;
    let today = crate::calendar::civil_at(crate::state::unix_now());
    let first = ["Alfa", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Teta", "Iota", "Kapa"];
    // Days from today to each birthday, 0 being today.
    let ahead: [i64; 30] = [0, 1, 2, 4, 5, 7, 9, 11, 13, 16, 20, 28, 35, 48, 62, 80, 101, 120, 140, 160, 185, 200, 225, 250, 270, 290, 310, 330, 345, 360];
    let mut people = Vec::new();
    for (at, days) in ahead.iter().enumerate() {
        let on: Date = today.plus_days(*days);
        let last = (at * 3 + at / 10 + 1) % 10;
        let last = if last == at % 10 { (last + 1) % 10 } else { last };
        // Every seventh person has no year; the rest are spread from 1944 to 2019.
        let year = if at % 7 == 3 { 0 } else { 1944 + (at as u16 * 37) % 76 };
        people.push(artosia_core::deck::Person { name: format!("{} {}", first[at % 10], first[last]), day: on.day, month: on.month, year, ..Default::default() });
    }
    let _ = store.replace_people(&people);
}

/// The practice offerings: six funds, nine made-up gifts across the last two Sundays and an
/// account at "Bank Contoh". Names are the made-up church's own, so they stay as they are in every
/// window language.
pub fn practice_giving(store: &Store) {
    use artosia_core::giving::{Account, Fund, Gift, Method};
    let today = crate::calendar::civil_at(crate::state::unix_now());
    let latest = today.plus_days(-i64::from(today.weekday()));
    let earlier = latest.plus_days(-7);
    let mut ids = Vec::new();
    for (at, name) in ["Persepuluhan", "Persembahan umum", "Pembangunan", "Misi", "Diakonia", "Syukur"].iter().enumerate() {
        let mut fund = Fund { id: 0, name: (*name).into(), colour: crate::calendar::next_colour(at) };
        let _ = store.save_fund(&mut fund);
        ids.push(fund.id);
    }
    // Which Sunday (0 the earlier), giver (empty is the plate), fund, method, rupiah.
    let rows: [(u8, &str, usize, Method, i64); 9] = [
        (0, "Contoh Alfa Beta", 0, Method::Transfer, 2_500_000),
        (0, "", 1, Method::Cash, 1_275_000),
        (0, "Contoh Gamma Delta", 2, Method::Transfer, 5_000_000),
        (1, "Contoh Epsilon Zeta", 0, Method::Transfer, 1_800_000),
        (1, "", 1, Method::Cash, 1_340_000),
        (1, "Contoh Eta Teta", 3, Method::Transfer, 750_000),
        (1, "", 4, Method::Cash, 400_000),
        (1, "Contoh Iota Kapa", 5, Method::Transfer, 350_000),
        (1, "", 2, Method::Cash, 600_000),
    ];
    for (week, giver, fund, method, amount) in rows {
        let mut gift = Gift { id: 0, date: if week == 0 { earlier } else { latest }, giver: giver.into(), fund: ids[fund], method, amount };
        let _ = store.save_gift(&mut gift);
    }
    let _ = store.set_account(&Account { bank: "Bank Contoh".into(), number: "0123 4567 89".into(), holder: "Gereja Contoh".into(), note: "Tulis jenis persembahan pada berita transfer".into() });
}

/// The practice announcements, in the window's language (`strings::PRACTICE_NOTICES`): a tester
/// read "Wednesday" on a card of an Indonesian window.
pub fn practice_announcements() -> Vec<Announcement> {
    crate::strings::PRACTICE_NOTICES().iter().map(|(title, body)| Announcement { title: (*title).into(), body: (*body).into(), seconds: 10, ..Announcement::default() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("artosia-data-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn packaged_library_is_documents_artosia() {
        let root = scratch("documents");
        let library = packaged_library(Some(root.clone()), None).unwrap();
        assert_eq!(library.folder, root.join("Artosia"));
        assert!(library.copy_from.is_none());
        assert!(packaged_library(None, Some(root.clone())).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_direct_library_is_offered_once() {
        let root = scratch("offer");
        let direct = root.join("direct");
        std::fs::create_dir_all(&direct).unwrap();
        let documents = root.join("docs");
        assert_eq!(packaged_library(Some(documents.clone()), Some(direct.clone())).unwrap().copy_from, Some(direct.clone()));
        std::fs::create_dir_all(documents.join("Artosia")).unwrap();
        assert!(packaged_library(Some(documents), Some(direct)).unwrap().copy_from.is_none());
        assert!(packaged_library(Some(root.join("other")), Some(root.join("missing"))).unwrap().copy_from.is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_copy_keeps_every_file_and_leaves_the_original() {
        let root = scratch("copy");
        let from = root.join("from");
        std::fs::create_dir_all(from.join("bibles")).unwrap();
        std::fs::write(from.join("library.db"), b"songs").unwrap();
        std::fs::write(from.join("bibles").join("a.bbl"), b"bible").unwrap();
        let to = root.join("Artosia");
        copy_library(&from, &to).unwrap();
        assert_eq!(std::fs::read(to.join("library.db")).unwrap(), b"songs");
        assert_eq!(std::fs::read(to.join("bibles").join("a.bbl")).unwrap(), b"bible");
        assert!(from.join("library.db").is_file());
        assert!(!root.join("Artosia.copying").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_failed_copy_leaves_no_folder() {
        let root = scratch("failed");
        let to = root.join("Artosia");
        assert!(copy_library(&root.join("missing"), &to).is_err());
        assert!(!to.exists() && !root.join("Artosia.copying").exists());
        let _ = std::fs::remove_dir_all(root);
    }
}
