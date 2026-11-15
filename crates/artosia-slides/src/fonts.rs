//! The font families a look can name.
//!
//! Rune draws a family by name (`Family::Named`): a registered or installed one loads the first
//! time text asks for it, and a name this computer lacks draws in Rune's sans font, so a look
//! saved on another PC still shows its words. Rune has no call that lists the installed
//! families, so the list comes from fontique, the library Rune itself reads the system with.

use std::sync::{Mutex, OnceLock};

use rune::Family;

/// Rune wants the name as a `&'static str`. Each distinct name a look has used is kept once.
fn interned(name: &str) -> &'static str {
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(kept) = names.iter().find(|kept| **kept == name) {
        return kept;
    }
    let kept: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.push(kept);
    kept
}

/// The family a look's font name stands for: Rune's own font for an empty name.
pub fn family(name: &str) -> Family {
    let name = name.trim();
    if name.is_empty() { Family::Sans } else { Family::Named(interned(name)) }
}

/// Every family installed on this computer, sorted, found once and kept. The first call asks
/// the system for its fonts, which takes a moment and some memory, so only the Look dialog's
/// font picker makes it.
pub fn installed() -> &'static [String] {
    static FAMILIES: OnceLock<Vec<String>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut collection = fontique::Collection::new(fontique::CollectionOptions { shared: false, system_fonts: true });
        // A name starting with @ is a vertical-writing duplicate Windows lists beside its family.
        let mut names: Vec<String> = collection.family_names().filter(|name| !name.starts_with('@') && !name.trim().is_empty()).map(str::to_string).collect();
        names.sort_by_key(|name| name.to_lowercase());
        names.dedup();
        names
    })
}

/// The static cuts of the poster styles' open-licence families (`assets/warta/licences`), embedded
/// so a face costs memory only when a style draws with it; Rune takes fonts at start.
const FACES: &[(&str, &[u8])] = &[
    ("Archivo-800.ttf", include_bytes!("../assets/warta/fonts/Archivo-800.ttf")),
    ("Archivo-900.ttf", include_bytes!("../assets/warta/fonts/Archivo-900.ttf")),
    ("ArchivoCondensed-800.ttf", include_bytes!("../assets/warta/fonts/ArchivoCondensed-800.ttf")),
    ("ArchivoCondensed-900.ttf", include_bytes!("../assets/warta/fonts/ArchivoCondensed-900.ttf")),
    ("ArchivoSemiCondensed-600.ttf", include_bytes!("../assets/warta/fonts/ArchivoSemiCondensed-600.ttf")),
    ("ArchivoSemiCondensed-700.ttf", include_bytes!("../assets/warta/fonts/ArchivoSemiCondensed-700.ttf")),
    ("ArchivoSemiCondensed-800.ttf", include_bytes!("../assets/warta/fonts/ArchivoSemiCondensed-800.ttf")),
    ("BarlowSemiCondensed-500.ttf", include_bytes!("../assets/warta/fonts/BarlowSemiCondensed-500.ttf")),
    ("BarlowSemiCondensed-600.ttf", include_bytes!("../assets/warta/fonts/BarlowSemiCondensed-600.ttf")),
    ("BarlowSemiCondensed-700.ttf", include_bytes!("../assets/warta/fonts/BarlowSemiCondensed-700.ttf")),
    ("Cinzel-600.ttf", include_bytes!("../assets/warta/fonts/Cinzel-600.ttf")),
    ("Cinzel-700.ttf", include_bytes!("../assets/warta/fonts/Cinzel-700.ttf")),
    ("BigShouldersDisplay-900.ttf", include_bytes!("../assets/warta/fonts/BigShouldersDisplay-900.ttf")),
    ("BodoniModaDisplay-600.ttf", include_bytes!("../assets/warta/fonts/BodoniModaDisplay-600.ttf")),
    ("BodoniModaDisplay-600i.ttf", include_bytes!("../assets/warta/fonts/BodoniModaDisplay-600i.ttf")),
    ("BodoniModaText-500.ttf", include_bytes!("../assets/warta/fonts/BodoniModaText-500.ttf")),
    ("BodoniModaText-700.ttf", include_bytes!("../assets/warta/fonts/BodoniModaText-700.ttf")),
    ("Allura-400.ttf", include_bytes!("../assets/warta/fonts/Allura-400.ttf")),
    ("Jost-300.ttf", include_bytes!("../assets/warta/fonts/Jost-300.ttf")),
    ("Jost-500.ttf", include_bytes!("../assets/warta/fonts/Jost-500.ttf")),
    ("Jost-600.ttf", include_bytes!("../assets/warta/fonts/Jost-600.ttf")),
    ("Figtree-500.ttf", include_bytes!("../assets/warta/fonts/Figtree-500.ttf")),
    ("Figtree-700.ttf", include_bytes!("../assets/warta/fonts/Figtree-700.ttf")),
    ("Fraunces-400.ttf", include_bytes!("../assets/warta/fonts/Fraunces-400.ttf")),
    ("Fraunces-400i.ttf", include_bytes!("../assets/warta/fonts/Fraunces-400i.ttf")),
    ("Fraunces-600.ttf", include_bytes!("../assets/warta/fonts/Fraunces-600.ttf")),
    ("HankenGrotesk-300.ttf",include_bytes!("../assets/warta/fonts/HankenGrotesk-300.ttf")),
    ("HankenGrotesk-500.ttf", include_bytes!("../assets/warta/fonts/HankenGrotesk-500.ttf")),
    ("HankenGrotesk-700.ttf", include_bytes!("../assets/warta/fonts/HankenGrotesk-700.ttf")),
    ("IBMPlexSansCondensed-500.ttf", include_bytes!("../assets/warta/fonts/IBMPlexSansCondensed-500.ttf")),
    ("IBMPlexSansCondensed-600.ttf", include_bytes!("../assets/warta/fonts/IBMPlexSansCondensed-600.ttf")),
    ("IBMPlexSansCondensed-700.ttf", include_bytes!("../assets/warta/fonts/IBMPlexSansCondensed-700.ttf")),
    ("Unbounded-700.ttf", include_bytes!("../assets/warta/fonts/Unbounded-700.ttf")),
    ("Unbounded-800.ttf", include_bytes!("../assets/warta/fonts/Unbounded-800.ttf")),
    ("Montserrat-700.ttf", include_bytes!("../assets/warta/fonts/Montserrat-700.ttf")),
    ("Montserrat-800.ttf", include_bytes!("../assets/warta/fonts/Montserrat-800.ttf")),
    ("IBMPlexMono-500.ttf", include_bytes!("../assets/warta/fonts/IBMPlexMono-500.ttf")),
    ("IBMPlexMono-700.ttf", include_bytes!("../assets/warta/fonts/IBMPlexMono-700.ttf")),
    ("InstrumentSans-500.ttf", include_bytes!("../assets/warta/fonts/InstrumentSans-500.ttf")),
    ("InstrumentSans-600.ttf", include_bytes!("../assets/warta/fonts/InstrumentSans-600.ttf")),
    ("InstrumentSans-700.ttf", include_bytes!("../assets/warta/fonts/InstrumentSans-700.ttf")),
    ("InstrumentSansCondensed-500.ttf", include_bytes!("../assets/warta/fonts/InstrumentSansCondensed-500.ttf")),
    ("InstrumentSansCondensed-600.ttf", include_bytes!("../assets/warta/fonts/InstrumentSansCondensed-600.ttf")),
    ("PlusJakartaSans-600.ttf", include_bytes!("../assets/warta/fonts/PlusJakartaSans-600.ttf")),
    ("PlusJakartaSans-700.ttf", include_bytes!("../assets/warta/fonts/PlusJakartaSans-700.ttf")),
    ("PlusJakartaSans-800.ttf", include_bytes!("../assets/warta/fonts/PlusJakartaSans-800.ttf")),
    ("Sora-300.ttf", include_bytes!("../assets/warta/fonts/Sora-300.ttf")),
    ("Sora-600.ttf", include_bytes!("../assets/warta/fonts/Sora-600.ttf")),
    ("SourceSerif4Display-400.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Display-400.ttf")),
    ("SourceSerif4Display-400i.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Display-400i.ttf")),
    ("SourceSerif4Display-600.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Display-600.ttf")),
    ("SourceSerif4Oldstyle-400i.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Oldstyle-400i.ttf")),
    ("SourceSerif4Text-400.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Text-400.ttf")),
    ("SourceSerif4Text-400i.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Text-400i.ttf")),
    ("SourceSerif4Text-600.ttf", include_bytes!("../assets/warta/fonts/SourceSerif4Text-600.ttf")),
    ("LibreCaslonDisplay-400.ttf", include_bytes!("../assets/warta/fonts/LibreCaslonDisplay-400.ttf")),
    ("LibreCaslonText-400.ttf", include_bytes!("../assets/warta/fonts/LibreCaslonText-400.ttf")),
    ("LibreCaslonText-400i.ttf", include_bytes!("../assets/warta/fonts/LibreCaslonText-400i.ttf")),
    ("LibreCaslonText-700.ttf", include_bytes!("../assets/warta/fonts/LibreCaslonText-700.ttf")),
    ("SpaceGrotesk-500.ttf", include_bytes!("../assets/warta/fonts/SpaceGrotesk-500.ttf")),
    ("SpaceGrotesk-700.ttf", include_bytes!("../assets/warta/fonts/SpaceGrotesk-700.ttf")),
];

/// Every bundled face, for the app's font list.
pub fn bundled() -> &'static [&'static [u8]] {
    static ALL: OnceLock<Vec<&'static [u8]>> = OnceLock::new();
    ALL.get_or_init(|| FACES.iter().map(|(_, bytes)| *bytes).collect())
}

/// A bundled face by file name.
pub fn face_bytes(file: &str) -> Option<&'static [u8]> {
    FACES.iter().find(|(name, _)| *name == file).map(|(_, bytes)| *bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_name_is_rune_s_own_font_and_a_name_is_kept_once() {
        assert_eq!(family(""), Family::Sans);
        assert_eq!(family("  "), Family::Sans);
        let (Family::Named(a), Family::Named(b)) = (family("Some Family"), family("Some Family")) else { panic!("a name is a named family") };
        assert!(std::ptr::eq(a, b));
    }
}
