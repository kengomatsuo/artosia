//! What this computer speaks and where it is, and which of a program's languages fits that.
//! The setup, the operator's first run and the tests all ask here, so "zh-TW means the
//! Traditional table" is written once.
//!
//! A language is named by a tag as Windows and browsers write it: `id`, `id-ID`, `pt-BR`,
//! `zh-Hant-TW`. A program offers codes of the same kind (`en`, `id`, `zh-Hans`, `pt-BR`).

/// The offered code that fits `wanted`, or None when the program does not speak it.
///
/// An exact match wins. Chinese is matched by script: a tag that says `Hant`, or names Taiwan,
/// Hong Kong or Macau, is Traditional, and every other Chinese tag is Simplified. Any other
/// language is matched by its first part, so `pt-PT` takes `pt-BR` and `es-MX` takes `es`.
pub fn pick<'a>(wanted: &str, offered: &[&'a str]) -> Option<&'a str> {
    let wanted = wanted.trim().replace('_', "-").to_ascii_lowercase();
    let wanted = wanted.split('.').next().unwrap_or_default();
    if wanted.is_empty() {
        return None;
    }
    if let Some(exact) = offered.iter().find(|code| code.eq_ignore_ascii_case(wanted)) {
        return Some(exact);
    }
    let parts: Vec<&str> = wanted.split('-').collect();
    // Java's and old Windows' names for Indonesian and Hebrew are not needed beyond `in`.
    let primary = if parts[0] == "in" { "id" } else { parts[0] };
    if primary == "zh" {
        // A script the tag names outright beats what its country usually writes.
        let names = |wanted: &[&str]| parts[1..].iter().any(|part| wanted.contains(part));
        let traditional = !names(&["hans"]) && names(&["hant", "tw", "hk", "mo"]);
        let script = if traditional { "zh-hant" } else { "zh-hans" };
        if let Some(code) = offered.iter().find(|code| code.eq_ignore_ascii_case(script)) {
            return Some(code);
        }
    }
    offered.iter().find(|code| code.split('-').next().is_some_and(|first| first.eq_ignore_ascii_case(primary))).copied()
}

/// The first of the person's languages that the program speaks.
pub fn pick_first<'a>(wanted: &[String], offered: &[&'a str]) -> Option<&'a str> {
    wanted.iter().find_map(|tag| pick(tag, offered))
}

/// The two capital letters of the country a tag names (`id-ID` gives `ID`), if it names one.
pub fn region_of(tag: &str) -> Option<String> {
    let tag = tag.trim().replace('_', "-");
    let tag = tag.split('.').next().unwrap_or_default();
    tag.split('-').skip(1).find(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic())).map(str::to_ascii_uppercase)
}

/// The languages this person set the computer to show, most wanted first. Empty when the
/// computer does not say.
pub fn languages() -> Vec<String> {
    system::languages()
}

/// The country this person set the computer to, as two capital letters (`ID`), or None when
/// the computer does not say.
pub fn region() -> Option<String> {
    system::region().or_else(|| languages().iter().find_map(|tag| region_of(tag)))
}

#[cfg(windows)]
mod system {
    use windows::Win32::Globalization::{GetUserDefaultLocaleName, GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::core::{PWSTR, s, w};

    /// The display language list: names divided by a null, ended by two (Microsoft's page for
    /// `GetUserPreferredUILanguages`, read 2026-11-12).
    pub fn languages() -> Vec<String> {
        let (mut count, mut size) = (0u32, 0u32);
        // SAFETY: a null buffer with a size of zero asks only for the size needed.
        if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut size) }.is_err() || size == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u16; size as usize];
        // SAFETY: the buffer holds `size` characters, the size the call above asked for.
        if unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, Some(PWSTR(buffer.as_mut_ptr())), &mut size) }.is_err() {
            return Vec::new();
        }
        buffer.split(|unit| *unit == 0).filter(|name| !name.is_empty()).map(String::from_utf16_lossy).collect()
    }

    /// The person's country: `GetUserDefaultGeoName` where Windows has it (10, version 1709 and
    /// later), found at run time so an older Windows still starts the program; before that,
    /// the country of the regional format.
    pub fn region() -> Option<String> {
        geo_name().or_else(|| {
            let mut buffer = [0u16; 85];
            // SAFETY: the buffer is 85 characters, the longest locale name Windows writes.
            let written = unsafe { GetUserDefaultLocaleName(&mut buffer) };
            let name = String::from_utf16_lossy(&buffer[..usize::try_from(written).ok()?.saturating_sub(1)]);
            super::region_of(&name)
        })
    }

    fn geo_name() -> Option<String> {
        type GeoName = unsafe extern "system" fn(*mut u16, i32) -> i32;
        // SAFETY: kernel32 is loaded in every process; the name is looked up, never assumed.
        let found = unsafe { GetProcAddress(GetModuleHandleW(w!("kernel32.dll")).ok()?, s!("GetUserDefaultGeoName")) }?;
        // SAFETY: the export has this signature (Microsoft's page, read 2026-11-12).
        let call: GeoName = unsafe { std::mem::transmute(found) };
        let mut buffer = [0u16; 16];
        // SAFETY: the buffer and its length are passed together.
        let written = unsafe { call(buffer.as_mut_ptr(), buffer.len() as i32) };
        let name = String::from_utf16_lossy(&buffer[..usize::try_from(written).ok()?.saturating_sub(1)]);
        // A place with no two-letter code comes back as a number; that is no country to offer.
        (name.len() == 2 && name.chars().all(|c| c.is_ascii_alphabetic())).then(|| name.to_ascii_uppercase())
    }
}

#[cfg(not(windows))]
mod system {
    /// `LANGUAGE` is a list divided by colons; the others name one locale (`id_ID.UTF-8`).
    pub fn languages() -> Vec<String> {
        let mut found = Vec::new();
        for name in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
            for tag in std::env::var(name).unwrap_or_default().split(':') {
                let tag = tag.split('.').next().unwrap_or_default().replace('_', "-");
                if !tag.is_empty() && tag != "C" && tag != "POSIX" && !found.contains(&tag) {
                    found.push(tag);
                }
            }
        }
        found
    }

    pub fn region() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFERED: &[&str] = &["en", "id", "zh-Hans", "zh-Hant", "ko", "ja", "es", "pt-BR", "fr", "vi"];

    #[test]
    fn a_tag_takes_the_language_that_fits_it() {
        for (tag, code) in [
            ("id-ID", "id"),
            ("in-ID", "id"),
            ("id_ID.UTF-8", "id"),
            ("en-GB", "en"),
            ("zh-CN", "zh-Hans"),
            ("zh-SG", "zh-Hans"),
            ("zh", "zh-Hans"),
            ("zh-Hans-HK", "zh-Hans"),
            ("zh-TW", "zh-Hant"),
            ("zh-HK", "zh-Hant"),
            ("zh-Hant", "zh-Hant"),
            ("zh-hant-my", "zh-Hant"),
            ("pt-PT", "pt-BR"),
            ("PT-br", "pt-BR"),
            ("es-MX", "es"),
            ("fr-CA", "fr"),
            ("ja-JP", "ja"),
            ("ko-KR", "ko"),
            ("vi-VN", "vi"),
        ] {
            assert_eq!(pick(tag, OFFERED), Some(code), "{tag}");
        }
    }

    #[test]
    fn a_language_the_program_lacks_is_none_and_the_next_one_is_tried() {
        assert_eq!(pick("de-DE", OFFERED), None);
        assert_eq!(pick("", OFFERED), None);
        assert_eq!(pick("zh-TW", &["en", "id"]), None);
        assert_eq!(pick_first(&["jv-ID".to_string(), "id-ID".to_string(), "en-US".to_string()], OFFERED), Some("id"));
        assert_eq!(pick_first(&[], OFFERED), None);
    }

    #[test]
    fn a_tag_names_its_country_only_when_it_has_one() {
        assert_eq!(region_of("id-ID"), Some("ID".to_string()));
        assert_eq!(region_of("zh-Hant-TW"), Some("TW".to_string()));
        assert_eq!(region_of("pt_BR.UTF-8"), Some("BR".to_string()));
        assert_eq!(region_of("id"), None);
        assert_eq!(region_of("es-419"), None);
    }

    #[test]
    fn this_computer_answers_without_failing() {
        // What it answers depends on the computer; that it answers in the right shape does not.
        for tag in languages() {
            assert!(!tag.is_empty() && !tag.contains('\0'), "{tag:?}");
        }
        if let Some(region) = region() {
            assert!(region.len() == 2 && region.chars().all(|c| c.is_ascii_uppercase()), "{region}");
        }
    }
}
