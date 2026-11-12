//! Every text the setup shows, in one table. English is written here; every other language is
//! `strings-<code>.json`, written from `strings-brief.json` by a writer who never sees the
//! English, and any key a table lacks falls back to English.

use std::collections::HashMap;

macro_rules! table {
    ($($name:ident $key:literal $english:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key { $($name),* }

        impl Key {
            pub const ALL: &[Key] = &[$(Key::$name),*];

            /// The name a brief and every table know the text by.
            pub fn name(self) -> &'static str {
                match self { $(Key::$name => $key),* }
            }

            pub fn english(self) -> &'static str {
                match self { $(Key::$name => $english),* }
            }
        }
    };
}

table! {
    Title "title" "Artosia setup";
    InstallHeading "install_heading" "Installing Artosia";
    StepChecking "step_checking" "Looking for the latest version";
    StepDownloading "step_downloading" "Downloading, {done} of {total} MB";
    StepSettingUp "step_setting_up" "Setting up";
    RemovingHeading "removing_heading" "Removing Artosia";
    StepRemoving "step_removing" "Removing the program and its shortcuts";
    Cancel "cancel" "Cancel";
    FailedHeading "failed_heading" "Artosia was not installed";
    FailedOffline "failed_offline" "Artosia could not reach the internet. Check the connection and try again.";
    FailedDamaged "failed_damaged" "A downloaded file did not pass the safety check. Try again. If it keeps happening, download the setup once more.";
    FailedDisk "failed_disk" "Artosia could not save its files. Free some space, or let your antivirus open the folder, then try again.";
    FailedShortcuts "failed_shortcuts" "Artosia could not add its shortcuts. Try again.";
    FailedOther "failed_other" "Something went wrong. Try again.";
    TryAgain "try_again" "Try again";
    Close "close" "Close";
    RemoveHeading "remove_heading" "Remove Artosia?";
    RemoveBody "remove_body" "The program and its shortcuts will be removed. Your songs, plans, settings and recordings stay on this computer.";
    RemoveYes "remove_yes" "Remove";
    CloseFirstHeading "close_first_heading" "Close Artosia first";
    CloseFirstBody "close_first_body" "Artosia is open. Close it, with its projector window, and removing goes on by itself.";
    RemoveFailedHeading "remove_failed_heading" "Artosia was not fully removed";
    RemoveFailedBody "remove_failed_body" "Some files could not be deleted. Close Artosia and any folder open on it, then try again.";
    StartTagline "start_tagline" "Song words on the projector";
    StartBody "start_body" "Artosia installs for you alone and needs no administrator.";
    Install "install" "Install";
    FeatureSongs "feature_songs" "Songs found by any line";
    FeatureBible "feature_bible" "Bible verses by reference or by words";
    FeaturePhone "feature_phone" "A remote on your phone";
    FeatureProjector "feature_projector" "A projector that keeps running";
    WaitBody "wait_body" "Keep this window open. Artosia starts when it is ready.";
    WaitRemoveBody "wait_remove_body" "Keep this window open until Artosia is removed.";
    DoneHeading "done_heading" "Artosia is installed";
    DoneBody "done_body" "Artosia is opening now.";
}

/// Every language beside English: its code and its table.
const TABLES: &[(&str, &str)] = &[
    ("id", include_str!("../strings-id.json")),
    ("zh-Hans", include_str!("../strings-zh-Hans.json")),
    ("zh-Hant", include_str!("../strings-zh-Hant.json")),
    ("ko", include_str!("../strings-ko.json")),
    ("ja", include_str!("../strings-ja.json")),
    ("es", include_str!("../strings-es.json")),
    ("pt-BR", include_str!("../strings-pt-BR.json")),
    ("fr", include_str!("../strings-fr.json")),
    ("vi", include_str!("../strings-vi.json")),
];

/// A language the setup speaks, by its code (`en`, `id`, `zh-Hant`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Language(&'static str);

impl Language {
    pub const ENGLISH: Language = Language("en");

    pub fn code(self) -> &'static str {
        self.0
    }

    /// Every language, English first.
    pub fn all() -> Vec<Language> {
        std::iter::once(Language::ENGLISH).chain(TABLES.iter().map(|(code, _)| Language(code))).collect()
    }

    /// The language a code or a tag of the computer (`id-ID`, `zh-TW`) names, if the setup speaks it.
    pub fn named(tag: &str) -> Option<Language> {
        let codes: Vec<&'static str> = Language::all().iter().map(|language| language.0).collect();
        artosia_locale::pick(tag, &codes).map(Language)
    }

    /// The first of this person's display languages the setup speaks; English when it speaks none.
    pub fn of_this_computer() -> Language {
        artosia_locale::languages().iter().find_map(|tag| Language::named(tag)).unwrap_or(Language::ENGLISH)
    }

    /// The table as written, key name to text. Empty for English.
    pub fn table(self) -> HashMap<String, String> {
        TABLES.iter().find(|(code, _)| *code == self.0).and_then(|(_, json)| serde_json::from_str(json).ok()).unwrap_or_default()
    }
}

pub struct Strings {
    language: Language,
    table: HashMap<String, String>,
}

impl Strings {
    pub fn new(language: Language) -> Strings {
        Strings { language, table: language.table() }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn get(&self, key: Key) -> &str {
        self.table.get(key.name()).map_or(key.english(), String::as_str)
    }

    /// A text with `{name}` holes filled.
    pub fn fill(&self, key: Key, holes: &[(&str, &str)]) -> String {
        holes.iter().fold(self.get(key).to_string(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
    }
}
