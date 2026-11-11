//! Every text the setup shows, in one table. English is written here; Indonesian is
//! `strings-id.json`, written from `strings-brief.json` by a writer who never sees the English,
//! and any key it lacks falls back to English.

use std::collections::HashMap;

macro_rules! table {
    ($($name:ident $key:literal $english:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Key { $($name),* }

        impl Key {
            pub const ALL: &[Key] = &[$(Key::$name),*];

            /// The name a brief and the Indonesian table know the text by.
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Indonesian,
}

impl Language {
    /// From a Windows language id (`GetUserDefaultUILanguage`): the low ten bits are the language.
    pub fn from_lang_id(id: u16) -> Language {
        const INDONESIAN: u16 = 0x21;
        if id & 0x3ff == INDONESIAN { Language::Indonesian } else { Language::English }
    }
}

const INDONESIAN_TABLE: &str = include_str!("../strings-id.json");

/// The Indonesian table as written, key name to text. Empty until the writer's table lands.
pub fn indonesian_table() -> HashMap<String, String> {
    serde_json::from_str(INDONESIAN_TABLE).unwrap_or_default()
}

pub struct Strings {
    language: Language,
    indonesian: HashMap<String, String>,
}

impl Strings {
    pub fn new(language: Language) -> Strings {
        let indonesian = if language == Language::Indonesian { indonesian_table() } else { HashMap::new() };
        Strings { language, indonesian }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn get(&self, key: Key) -> &str {
        self.indonesian.get(key.name()).map_or(key.english(), String::as_str)
    }

    /// A text with `{name}` holes filled.
    pub fn fill(&self, key: Key, holes: &[(&str, &str)]) -> String {
        holes.iter().fold(self.get(key).to_string(), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
    }
}
