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

/// The library folder: `--data`, then `ARTOSIA_DATA`, then the user's app-data folder.
/// None in a headless run without `--data`, which uses the practice library in memory so
/// snapshots are the same on every machine and never touch a real library.
pub fn folder() -> Option<PathBuf> {
    if let Some(folder) = flag_value("--data") {
        return Some(PathBuf::from(folder));
    }
    if rune::is_headless() || has_flag("--practice") {
        return None;
    }
    if let Some(folder) = std::env::var_os("ARTOSIA_DATA") {
        return Some(PathBuf::from(folder));
    }
    rune::dirs::AppDirs::new("artosia").map(|dirs| dirs.data().to_path_buf())
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
    ]
}

/// The practice announcements, in the window's language (`strings::PRACTICE_NOTICES`): a tester
/// read "Wednesday" on a card of an Indonesian window.
pub fn practice_announcements() -> Vec<Announcement> {
    crate::strings::PRACTICE_NOTICES().iter().map(|(title, body)| Announcement { title: (*title).into(), body: (*body).into(), seconds: 10, ..Announcement::default() }).collect()
}
