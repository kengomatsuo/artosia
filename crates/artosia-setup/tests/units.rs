//! The pure parts: the registry value set, the paths and the string table.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use artosia_setup::entry::{Value, uninstall_values};
use artosia_setup::places::{Places, SETUP_EXE};
use artosia_setup::strings::{Key, Language, Strings, indonesian_table};
use std::collections::HashSet;
use std::path::Path;

fn brief() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../strings-brief.json")).unwrap()
}

fn placeholders(text: &str) -> Vec<String> {
    let mut found: Vec<String> = text.split('{').skip(1).filter_map(|rest| rest.split('}').next()).map(str::to_string).collect();
    found.sort();
    found
}

/// The house rules for a string a person reads: no dash, middle dot, emoji or exclamation
/// mark, sentence case.
fn assert_house_style(key: &str, text: &str) {
    assert!(!text.is_empty(), "{key} is empty");
    for banned in ['\u{2014}', '\u{2013}', '\u{b7}', '!', '\u{ff01}'] {
        assert!(!text.contains(banned), "{key} has {banned:?}: {text}");
    }
    assert!(!text.chars().any(|c| matches!(c as u32, 0x1F000..=0x1FFFF | 0x2600..=0x27BF | 0xFE0F)), "{key} has an emoji: {text}");
    assert!(text.chars().next().is_some_and(char::is_uppercase), "{key} starts in lower case: {text}");
}

#[test]
fn every_key_has_english_text_in_house_style_and_a_unique_name() {
    let mut names = HashSet::new();
    for key in Key::ALL {
        assert!(names.insert(key.name()), "{} appears twice", key.name());
        assert_house_style(key.name(), key.english());
        // Sentence case: after the first word only the product name and sentence starts are capitals.
        let text = key.english();
        let mut start = true;
        for word in text.split_whitespace() {
            // A unit such as MB is all capitals.
            let capital = word.chars().next().is_some_and(char::is_uppercase) && word.chars().any(char::is_lowercase);
            assert!(!capital || start || word.starts_with("Artosia"), "{}: {word:?} in {text}", key.name());
            start = word.ends_with('.') || word.ends_with('?');
        }
    }
}

#[test]
fn a_failure_says_what_happened_and_what_to_do_in_one_or_two_short_sentences() {
    for key in Key::ALL.iter().filter(|k| k.name().starts_with("failed_") && *k != &Key::FailedHeading || k.name() == "remove_failed_body") {
        let text = key.english();
        let sentences = text.split(['.', '?']).filter(|s| !s.trim().is_empty()).count();
        assert!((1..=3).contains(&sentences), "{}: {sentences} sentences", key.name());
        assert!(text.ends_with('.'), "{}", key.name());
        assert!(text.contains("try") || text.contains("Try"), "{} says what to do", key.name());
    }
}

#[test]
fn every_key_has_a_brief_and_every_brief_a_key() {
    let briefs = brief();
    let keys: HashSet<&str> = Key::ALL.iter().map(|k| k.name()).collect();
    let briefed: Vec<&str> = briefs.iter().map(|b| b["key"].as_str().unwrap()).collect();
    assert_eq!(briefed.len(), keys.len(), "one brief per key");
    for name in &briefed {
        assert!(keys.contains(name), "a brief for {name}, which is no key");
    }
    for entry in &briefs {
        let key = Key::ALL.iter().find(|k| k.name() == entry["key"]).unwrap();
        for field in ["dialog", "moment", "role", "does", "notes"] {
            assert!(entry[field].as_str().is_some_and(|t| t.len() > 8), "{}: {field}", key.name());
        }
        let budget = entry["budget"].as_u64().unwrap() as usize;
        assert!(key.english().chars().count() <= budget, "{} is over its budget of {budget}", key.name());
        let variables: Vec<String> = {
            let mut v: Vec<String> = entry["variables"].as_array().unwrap().iter().map(|v| v["name"].as_str().unwrap().to_string()).collect();
            v.sort();
            v
        };
        assert_eq!(variables, placeholders(key.english()), "{} variables match its holes", key.name());
        // A writer who never sees the English must not be handed it: no four words in a row.
        let words = |t: &str| t.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect::<Vec<_>>();
        let (english, does) = (words(key.english()), words(entry["does"].as_str().unwrap()));
        for run in english.windows(4) {
            assert!(!does.windows(4).any(|w| w == run), "{}: the brief repeats {run:?}", key.name());
        }
    }
}

#[test]
fn indonesian_is_empty_or_complete_and_follows_the_same_rules() {
    let table = indonesian_table();
    let keys: HashSet<&str> = Key::ALL.iter().map(|k| k.name()).collect();
    for name in table.keys() {
        assert!(keys.contains(name.as_str()), "strings-id.json has {name}, which is no key");
    }
    // Empty means "falls back to English"; once a writer fills it, it must be whole.
    assert!(table.is_empty() || table.len() == keys.len(), "strings-id.json holds {} of {} keys", table.len(), keys.len());
    for key in Key::ALL {
        if let Some(text) = table.get(key.name()) {
            assert_house_style(key.name(), text);
            assert_eq!(placeholders(text), placeholders(key.english()), "{} keeps its holes", key.name());
        }
    }
}

#[test]
fn a_missing_indonesian_text_falls_back_to_english_and_holes_are_filled() {
    let id = Strings::new(Language::Indonesian);
    let en = Strings::new(Language::English);
    for key in Key::ALL {
        assert!(!id.get(*key).is_empty());
        assert!(!en.get(*key).is_empty());
    }
    assert_eq!(en.fill(Key::StepDownloading, &[("done", "42"), ("total", "163")]), "Downloading, 42 of 163 MB");
}

#[test]
fn the_windows_display_language_picks_the_table() {
    assert_eq!(Language::from_lang_id(0x0421), Language::Indonesian);
    assert_eq!(Language::from_lang_id(0x0021), Language::Indonesian);
    assert_eq!(Language::from_lang_id(0x0409), Language::English);
    assert_eq!(Language::from_lang_id(0x0c09), Language::English);
    assert_eq!(Language::from_lang_id(0x0411), Language::English);
}

#[test]
fn the_uninstall_entry_has_exactly_the_asked_values_with_the_right_types() {
    let places = Places::under(Path::new(r"C:\t\root"));
    let values = uninstall_values(&places, "0.1.13", 163 * 1024 * 1024 + 500);
    let names: Vec<&str> = values.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        ["DisplayName", "DisplayVersion", "Publisher", "DisplayIcon", "InstallLocation", "UninstallString", "QuietUninstallString", "NoModify", "NoRepair", "EstimatedSize"]
    );
    let get = |name: &str| values.iter().find(|(n, _)| *n == name).unwrap().1.clone();
    let setup = places.setup_exe().display().to_string();
    assert_eq!(get("DisplayVersion"), Value::Text("0.1.13".into()));
    assert_eq!(get("DisplayIcon"), Value::Text(places.app_exe().display().to_string()));
    assert_eq!(get("InstallLocation"), Value::Text(places.install.display().to_string()));
    assert_eq!(get("UninstallString"), Value::Text(format!("\"{setup}\" --uninstall")));
    assert_eq!(get("QuietUninstallString"), Value::Text(format!("\"{setup}\" --uninstall --silent")));
    assert_eq!(get("NoModify"), Value::Dword(1));
    assert_eq!(get("NoRepair"), Value::Dword(1));
    assert_eq!(get("EstimatedSize"), Value::Dword(163 * 1024), "kilobytes, as a DWORD");
    assert_eq!(uninstall_values(&places, "1", u64::MAX).last().unwrap().1, Value::Dword(u32::MAX), "saturates, never wraps");
}

#[test]
fn a_test_root_moves_every_folder_and_the_key_and_the_real_names_stay_real() {
    let a = Places::under(Path::new(r"C:\t\one"));
    let b = Places::under(Path::new(r"C:\t\two"));
    for place in [&a.install, &a.start_menu, &a.desktop, &a.data[0]] {
        assert!(place.starts_with(r"C:\t\one"), "{place:?}");
    }
    assert_eq!(a.install, Path::new(r"C:\t\one\Programs\Artosia"));
    assert!(a.key_name.starts_with("Artosia-test-"));
    assert_ne!(a.key_name, b.key_name);
    assert_eq!(a.key_name, Places::under(Path::new(r"C:\T\ONE")).key_name, "the same root, the same key, in any case");
    assert_ne!(a.key_name, "Artosia");
    assert!(a.registry_path().ends_with(&format!(r"\Uninstall\{}", a.key_name)));
    assert_eq!(a.setup_exe().file_name().unwrap(), SETUP_EXE);
    assert_eq!(a.start_lnk().parent().unwrap(), a.start_menu);
    assert!(!a.data.iter().any(|d| d.starts_with(&a.install)), "the data folder is not inside the install folder");
}

#[cfg(windows)]
#[test]
fn the_real_places_are_the_users_own_folders_and_data_is_apart_from_the_install_folder() {
    // SAFETY-free: only reads. Skips when the test root variable is set by the caller.
    if std::env::var_os(artosia_setup::places::ROOT_ENV).is_some() {
        return;
    }
    let real = Places::resolve().unwrap();
    assert_eq!(real.key_name, "Artosia");
    assert!(real.install.ends_with(r"Programs\Artosia"), "{:?}", real.install);
    // Rune keeps its state and log folders in the LOCALAPPDATA folder named artosia, which on Windows is the same folder as one named Artosia
    // on Windows: the install folder, which an uninstall empties, must not be that folder or inside it.
    let local = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).unwrap();
    let (install, rune_state) = (real.install.to_string_lossy().to_lowercase(), local.join("artosia").to_string_lossy().to_lowercase());
    assert!(!install.starts_with(&rune_state) && !rune_state.starts_with(&install), "{install} and {rune_state}");
    assert!(real.install.to_string_lossy().to_lowercase().contains("appdata\\local") || std::env::var_os("LOCALAPPDATA").is_some());
    assert!(!real.data.is_empty());
    assert!(real.data.iter().all(|d| d != &real.install), "{:?}", real.data);
}

#[cfg(windows)]
#[test]
fn a_path_past_260_characters_is_written_and_read_back() {
    // What the install does for a long user profile: std puts the verbatim prefix on by itself.
    let base = std::env::temp_dir().join(format!("artosia-setup-long-\u{e9}-{}", std::process::id()));
    let deep = (0..8).fold(base.clone(), |p, _| p.join("d".repeat(40)));
    std::fs::create_dir_all(&deep).unwrap();
    let file = deep.join("artosia.exe");
    assert!(file.as_os_str().len() > 260);
    std::fs::write(&file, b"x").unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"x");
    std::fs::remove_dir_all(&base).unwrap();
}
