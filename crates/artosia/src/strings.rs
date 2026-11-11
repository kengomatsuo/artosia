//! Every word the operator reads, in English and in Indonesian. No language is the source the
//! other is translated from: each was written from what the screen has to say
//! (`docs/i18n/brief.json`), the Indonesian by a writer who never saw the English.
//! Counted nouns go through a function: English needs "1 slide" and "2 slides".
#![allow(non_snake_case)]

use crate::languages::{self, Arg, ENGLISH, INDONESIAN, counted, text_arg};

#[cfg(not(test))]
static LANGUAGE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(ENGLISH);

// Tests run side by side, one thread each: a language per thread keeps one test's Indonesian
// out of another's English.
#[cfg(test)]
thread_local! {
    static LANGUAGE: std::cell::Cell<usize> = const { std::cell::Cell::new(ENGLISH) };
}

/// Chooses the language of every later call by its code. False, and nothing changes, for a code
/// the registry does not know.
pub fn set_language(code: &str) -> bool {
    let Some(at) = languages::index_of(code) else { return false };
    #[cfg(not(test))]
    LANGUAGE.store(at, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    LANGUAGE.with(|language| language.set(at));
    // Markers written into lyrics from here on use this language's names for the sections.
    artosia_core::song::set_marker_words(marker_words());
    true
}

/// The window's names for the kinds of section, in the order of `SectionKind`, for a language
/// that has a table; None for English, whose markers stay `[1]` and `[chorus]`.
fn marker_words() -> Option<[String; 7]> {
    if !languages::has_table(language()) {
        return None;
    }
    Some(["verse", "pre_chorus", "chorus", "bridge", "intro", "ending", "other"].map(|kind| languages::text(language(), &format!("section.{kind}")).unwrap_or_default().to_string()))
}

/// The chosen language's position in `languages::LANGUAGES`.
pub fn language() -> usize {
    #[cfg(not(test))]
    return LANGUAGE.load(std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    return LANGUAGE.with(|language| language.get());
}

pub fn language_code() -> &'static str {
    languages::LANGUAGES[language()].code
}

/// The two-language switch, kept for the code written before the language was a code.
#[allow(dead_code)]
pub fn set_indonesian(on: bool) {
    set_language(if on { "id" } else { "en" });
}

pub fn indonesian() -> bool {
    language() == INDONESIAN
}

/// One text in English and Indonesian, read through a function so the language can change while
/// the program runs. Any other language reads its table by this key, and English when it lacks it.
macro_rules! text {
    ($name:ident, $en:expr, $id:expr) => {
        pub fn $name() -> &'static str {
            match language() {
                ENGLISH => $en,
                // Indonesian reads its table once it has one, and what is written here until then.
                INDONESIAN => languages::text(INDONESIAN, stringify!($name)).unwrap_or($id),
                other => languages::text(other, stringify!($name)).unwrap_or($en),
            }
        }
    };
}

/// A sentence of a table language. None for English and Indonesian, which the function writes
/// itself, and for a key the table lacks, which the function then writes in English.
fn say(key: &str, case: Option<&str>, count: Option<usize>, args: &[(&str, Arg)]) -> Option<String> {
    let lang = language();
    if !languages::has_table(lang) {
        return None;
    }
    languages::sentence(lang, key, case, count, args)
}

/// What an import of a song folder said, in a table language.
fn import_summary_table(added: usize, updated: usize, kept: usize) -> Option<String> {
    if !languages::has_table(language()) {
        return None;
    }
    let both = [("added", counted("songs", added)), ("updated", counted("songs", updated)), ("kept", counted("songs", kept))];
    let case = match (added, updated) {
        (0, 0) => None,
        (_, 0) => Some("changed_added"),
        (0, _) => Some("changed_updated"),
        _ => Some("changed_both"),
    };
    let changed = case.and_then(|case| say("import_summary", Some(case), None, &both));
    match (case.is_none(), kept) {
        (true, 0) => say("import_summary", Some("nothing_changed"), None, &[]),
        (false, 0) => changed,
        (true, _) => say("import_summary", Some("only_kept"), None, &both),
        (false, _) => say("import_summary", Some("changed_and_kept"), None, &[("changed", text_arg(changed?)), ("kept", counted("songs", kept))]),
    }
}

/// A section's name in a table language: `section.verse` and the others, English where missing.
fn section_label_table(tag: artosia_core::song::Tag) -> String {
    use artosia_core::song::SectionKind;
    let (key, english) = match tag.kind {
        SectionKind::Verse => ("verse", "Verse"),
        SectionKind::PreChorus => ("pre_chorus", "Pre-chorus"),
        SectionKind::Chorus => ("chorus", "Chorus"),
        SectionKind::Bridge => ("bridge", "Bridge"),
        SectionKind::Intro => ("intro", "Intro"),
        SectionKind::Ending => ("ending", "Ending"),
        SectionKind::Other => ("other", "Part"),
    };
    let name = languages::text(language(), &format!("section.{key}")).unwrap_or(english);
    if !(tag.kind == SectionKind::Verse || tag.number > 1) {
        return name.to_string();
    }
    say("section_numbered", None, None, &[("name", text_arg(name)), ("number", text_arg(tag.number))]).unwrap_or_else(|| format!("{name} {}", tag.number))
}

text!(APP_NAME, "Artosia", "Artosia");

// Keys, as printed on a keyboard.
text!(KEY_SEARCH, "Ctrl K", "Ctrl K");
text!(KEY_ENTER, "Enter", "Enter");
text!(KEY_SHOW_NOW, "Ctrl Enter", "Ctrl Enter");
text!(KEY_ADD, "Shift Enter", "Shift Enter");
text!(KEY_SAVE, "Ctrl S", "Ctrl S");
text!(KEY_START_HERE, "Ctrl M", "Ctrl M");

pub fn ctrl_digit(digit: usize) -> String {
    if let Some(text) = say("ctrl_digit", None, None, &[("digit", text_arg(digit))]) {
        return text;
    }
    format!("Ctrl {digit}")
}

// Search.
text!(SEARCH_PLACEHOLDER, "Search songs and the Bible", "Cari lagu atau ayat");
text!(SEARCH_SONGS, "Title or any line of a song", "Cari judul atau satu baris lirik");
text!(SEARCH_RESULTS, "Results", "Hasil");
text!(SCOPE_LABEL, "Search in", "Cari di");
text!(SCOPE_ALL, "All", "Semua");
text!(SCOPE_SONGS, "Songs", "Lagu");
text!(SCOPE_BIBLE, "Bible", "Alkitab");
text!(HINT_OPEN, "Open", "Buka");
text!(HINT_SECTIONS, "Its sections", "Cari bagian");
text!(SEARCH_SECTIONS, "Chorus, verse 2, or any line", "Bait 2, Reff, atau satu baris lirik");
text!(IN_PLAN, "In today's plan", "Sudah ada di acara");
text!(PLAN_CHIP, "Plan", "Di acara");
text!(KIND_LOOK, "Look", "Tampilan");
text!(KIND_BIBLE, "Bible translation", "Versi Alkitab");
text!(COUNTDOWN_DETAIL, "Shows on the projector until it ends", "Tayang di layar sampai waktu habis");
text!(MESSAGE_DETAIL, "Show this message to the room", "Pesan tampil di layar jemaat");

pub fn countdown_for(minutes: u32) -> String {
    if let Some(text) = say("countdown_for", None, Some(minutes as usize), &[("minutes", text_arg(minutes))]) {
        return text;
    }
    if indonesian() {
        return format!("Hitung mundur {minutes} menit");
    }
    if minutes == 1 { "Countdown, 1 minute".to_string() } else { format!("Countdown, {minutes} minutes") }
}
text!(BIBLE_LOADING, "Opening the Bible", "Menyiapkan Alkitab");
text!(OLD_TESTAMENT, "Old Testament", "Perjanjian Lama");
text!(NEW_TESTAMENT, "New Testament", "Perjanjian Baru");

pub fn pick_chapter(book: &str) -> String {
    if let Some(text) = say("pick_chapter", None, None, &[("book", text_arg(book))]) {
        return text;
    }
    if indonesian() {
        return format!("{book}: pilih pasal");
    }
    format!("{book}: pick a chapter")
}
text!(SONGS_EMPTY, "No songs yet", "Belum ada lagu");

pub fn nothing_found(query: &str) -> String {
    if let Some(text) = say("nothing_found", None, None, &[("query", text_arg(query))]) {
        return text;
    }
    if indonesian() {
        return format!("Tidak ada hasil untuk \u{201c}{query}\u{201d}");
    }
    format!("Nothing matches \u{201c}{query}\u{201d}")
}

pub fn new_song_called(title: &str) -> String {
    if let Some(text) = say("new_song_called", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("Buat lagu \u{201c}{title}\u{201d}");
    }
    format!("New song \u{201c}{title}\u{201d}")
}

pub fn songs(count: usize) -> String {
    if let Some(text) = say("songs", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return format!("{count} lagu");
    }
    if count == 1 { "1 song".to_string() } else { format!("{count} songs") }
}

/// The counted noun the plural tests of every language are written against.
#[allow(dead_code)]
pub fn slides(count: usize) -> String {
    if let Some(text) = say("slides", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return format!("{count} slide");
    }
    if count == 1 { "1 slide".to_string() } else { format!("{count} slides") }
}

pub fn chapters(count: usize) -> String {
    if let Some(text) = say("chapters", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return format!("{count} pasal");
    }
    if count == 1 { "1 chapter".to_string() } else { format!("{count} chapters") }
}

pub fn verses(count: usize) -> String {
    if let Some(text) = say("verses", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return format!("{count} ayat");
    }
    if count == 1 { "1 verse".to_string() } else { format!("{count} verses") }
}

// Commands in the search panel.
text!(CMD_NEW_SONG, "New song", "Lagu baru");
text!(CMD_NEW_SONG_DETAIL, "Type or paste the words", "Ketik atau tempel liriknya");
text!(CMD_PASTE_SONG, "Paste lyrics as a song", "Tempel lirik sebagai lagu");
text!(CMD_PASTE_SONG_DETAIL, "Copy lyrics anywhere, then choose this", "Salin liriknya dulu, lalu pilih ini");
text!(CMD_ANNOUNCEMENTS, "Announcements", "Pengumuman");
text!(CMD_ANNOUNCEMENTS_DETAIL, "Notices that loop before the service", "Tayang bergantian sebelum ibadah mulai");
text!(CMD_HEADING, "Add a heading to the plan", "Pemisah acara");
text!(CMD_HEADING_DETAIL, "A divider such as Worship or Word", "Misalnya Pujian dan Firman");
text!(CMD_LOOKS, "Design", "Design");
text!(CMD_LOOKS_DETAIL, "Background, words and logo on the projector", "Background, words and logo on the projector");
text!(CMD_SETTINGS, "Settings", "Settings");
text!(CMD_SETTINGS_DETAIL, "Language, Bible, keys and help", "Language, Bible, keys and help");
text!(CMD_SCREENS, "Screens", "Layar");
text!(CMD_SCREENS_DETAIL, "Choose the projector and the stage screen", "Pilih layar dan monitor panggung");
text!(CMD_REMOTE, "Phone remote", "Remote HP");
text!(CMD_REMOTE_DETAIL, "Change slides from a phone on the same Wi-Fi", "HP di Wi-Fi yang sama bisa ganti slide");
text!(CMD_IMPORT_FOLDER, "Import songs from a folder", "Impor lagu dari folder");
text!(CMD_IMPORT_FOLDER_DETAIL, "A folder of OpenLyrics song files", "Berkas OpenLyrics");
text!(IMPORT_FOLDER_EMPTY, "That folder has no OpenLyrics songs", "Tidak ada berkas OpenLyrics di folder itu");
text!(CMD_IMPORT, "Bring over EasiSlides", "Impor dari EasiSlides");
text!(CMD_IMPORT_DETAIL, "Songs and Bibles from EasiSlides on this computer", "Lagu dan Alkitab di komputer ini");
text!(CMD_STREAM, "Stream and record", "Siaran dan rekaman");
text!(CMD_STREAM_DETAIL, "Send the projector picture to YouTube or a file", "Kirim gambar layar ke situs video atau berkas");
text!(BTN_STREAM, "Stream", "Siaran");
text!(BTN_LISTEN, "Listen", "Dengar");
text!(STREAM_HINT, "The stream shows what the projector shows, with the room's sound.", "Penonton melihat isi layar dan mendengar suara ruangan");
text!(STREAM_ADDRESS, "Stream address", "URL streaming");
text!(STREAM_ADDRESS_HINT, "rtmp://a.rtmp.youtube.com/live2", "rtmp://a.rtmp.youtube.com/live2");
text!(STREAM_KEY, "Stream key", "Kunci streaming");
text!(STREAM_RECORD, "Keep a recording", "Simpan rekaman");
text!(STREAM_RECORD_HINT, "Saved with Artosia's library on this computer", "Disimpan di komputer ini bersama data Artosia");
text!(BTN_STREAM_START, "Start", "Mulai");
text!(BTN_STREAM_STOP, "Stop", "Berhenti");
text!(STREAM_STARTING, "Starting", "Sedang bersiap");
text!(STREAM_RECONNECTING, "The connection dropped. Trying again.", "Koneksi terputus, mencoba lagi");
text!(STREAM_STOPPING, "Finishing", "Sedang menyelesaikan");
text!(STREAM_MISSING, "This copy of Artosia came without the streaming program.", "Bagian siaran belum terpasang di Artosia ini");
text!(STREAM_NOTHING, "Give an address, or turn on the recording", "Isi URL streaming, atau nyalakan rekaman");

pub fn stream_failed(problem: &str) -> String {
    if let Some(text) = say("stream_failed", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Siaran gagal dimulai: {problem}");
    }
    format!("The stream could not start: {problem}")
}

/// How long the stream has been live, and how many pictures were lost on the way.
pub fn stream_live(seconds: u64, dropped: u64) -> String {
    if let Some(text) = say("stream_live", Some(if dropped == 0 { "none_lost" } else { "some_lost" }), Some(dropped as usize), &[("time", text_arg(languages::clock(seconds))), ("dropped", text_arg(dropped))]) {
        return text;
    }
    if indonesian() {
        return {
            let time = format!("{}:{:02}", seconds / 60, seconds % 60);
            if dropped == 0 { format!("Siaran langsung {time}") } else { format!("Siaran langsung {time}, {dropped} gambar hilang") }
        };
    }
    let time = format!("On air for {}:{:02}", seconds / 60, seconds % 60);
    match dropped {
        0 => time,
        1 => format!("{time}. 1 frame lost"),
        _ => format!("{time}. {dropped} frames lost"),
    }
}
text!(CMD_LISTEN, "Listen to the room", "Dengarkan lewat mikrofon");
text!(CMD_LISTEN_DETAIL, "Suggests the verse or song it hears", "Mengusulkan lagu atau ayat dari suara");
text!(LISTEN_SWITCH, "Listen", "Dengarkan suara");
text!(LISTEN_HINT, "It only suggests. Nothing shows by itself.", "Hanya mengusulkan. Layar tidak berubah sendiri.");
text!(LISTEN_MICROPHONE, "Microphone", "Mikrofon");
text!(LISTEN_DEFAULT, "The computer's own", "Bawaan komputer");
text!(LISTEN_SYSTEM_AUDIO, "System audio", "System audio");
text!(LISTEN_LANGUAGE, "Language", "Bahasa lisan");
text!(LISTEN_INDONESIAN, "Indonesian", "Indonesia");
text!(LISTEN_ENGLISH, "English", "Inggris");
text!(LISTEN_LEVEL, "Sound coming in", "Tingkat suara");
text!(LISTEN_HEARD, "Heard", "Yang terdengar");
text!(LISTEN_NOT_CONNECTED, "That microphone is not connected. Choose another.", "That microphone is not connected. Choose another.");
text!(LISTEN_WOULD_NOT_OPEN, "That microphone would not open. Choose another.", "That microphone would not open. Choose another.");
text!(LISTEN_INPUT_STOPPED, "The microphone stopped. Check its cable.", "The microphone stopped. Check its cable.");
text!(LISTEN_BEAT_ONLY, "This version hears the beat, not words.", "This version hears the beat, not words.");
text!(LISTEN_MODEL_DAMAGED, "The speech model is damaged. Download it again.", "The speech model is damaged. Download it again.");
text!(LISTEN_MODEL_MISSING, "To hear words, Artosia needs its speech model, 57 MB", "To hear words, Artosia needs its speech model, 57 MB");
pub fn listen_model_failed(why: &str) -> String {
    if let Some(text) = say("listen_model_failed", None, None, &[("why", text_arg(why))]) {
        return text;
    }
    format!("The speech model did not arrive: {why}")
}
text!(LISTEN_PART_LISTENING, "Listening", "Listening");
text!(LISTEN_PART_RECORDINGS, "Recordings", "Recordings");
text!(REC_SWITCH, "Keep recordings", "Keep recordings");
text!(REC_HINT, "Heard phrases and guesses, in a folder on this computer.", "Heard phrases and guesses, in a folder on this computer.");
text!(REC_NONE, "Nothing kept yet", "Nothing kept yet");
text!(REC_OPEN, "Open folder", "Open folder");
text!(REC_MOVE, "Move folder", "Move folder");
text!(REC_SAVE, "Save as one file", "Save as one file");
text!(REC_SAVING, "Saving", "Saving");
text!(REC_DELETE, "Delete all", "Delete all");
text!(REC_DELETED, "Recordings deleted", "Recordings deleted");
text!(REC_FAILED, "Recordings cannot be written there. Move the folder.", "Recordings cannot be written there. Move the folder.");
text!(REC_ON, "Recording", "Recording");
pub fn rec_kept(size: &str) -> String {
    if let Some(text) = say("rec_kept", None, None, &[("size", text_arg(size))]) {
        return text;
    }
    format!("{size} kept")
}
pub fn rec_saved(name: &str) -> String {
    if let Some(text) = say("rec_saved", None, None, &[("name", text_arg(name))]) {
        return text;
    }
    format!("Saved as {name}")
}
text!(LISTEN_TEMPO, "Tempo", "Tempo");
text!(LISTEN_BPM, "BPM", "BPM");
text!(LISTEN_TEMPO_HALF, "Half", "Half");
text!(LISTEN_TEMPO_AS_HEARD, "As heard", "As heard");
text!(LISTEN_TEMPO_DOUBLE, "Double", "Double");
text!(LISTEN_BEAT_BLINK, "Live marker follows the beat", "Live marker follows the beat");
text!(LISTEN_BEAT_BLINK_HINT, "A gentle pulse, just for fun.", "A gentle pulse, just for fun.");
text!(CMD_UPDATE, "Check for a new version", "Periksa pembaruan");
text!(CMD_UPDATE_DETAIL, "Artosia also checks by itself at each start", "Artosia juga memeriksa sendiri saat dibuka");

pub fn update_ready(version: &str) -> String {
    if let Some(text) = say("update_ready", None, None, &[("version", text_arg(version))]) {
        return text;
    }
    if indonesian() {
        return format!("Versi {version} sudah diunduh. Terpasang saat Artosia dibuka lagi.");
    }
    format!("Version {version} is ready. It installs when Artosia next starts.")
}

pub fn update_none(version: &str) -> String {
    if let Some(text) = say("update_none", None, None, &[("version", text_arg(version))]) {
        return text;
    }
    if indonesian() {
        return format!("Versi {version} adalah yang terbaru");
    }
    format!("Version {version} is the newest")
}
text!(CMD_KEYS, "Keyboard shortcuts", "Tombol pintas");
text!(CMD_KEYS_DETAIL, "Every key Artosia answers to", "Semua tombol yang dikenali Artosia");
text!(CMD_START, "Start the service", "Mulai ibadah");
text!(CMD_START_DETAIL, "Show the first thing in the plan", "Yang pertama di acara tayang di layar");
text!(CMD_PREVIOUS, "Previous slide", "Kembali");
text!(CMD_PREVIOUS_DETAIL, "Go back one slide", "Mundur satu slide");
text!(CMD_NEXT_DETAIL, "Go on one slide", "Maju satu slide");
text!(CMD_BLACK_DETAIL, "Turn the projector black, or bring it back", "Layar jadi hitam. Pilih lagi untuk batal");
text!(CMD_CLEAR_DETAIL, "Hide the words and keep the background", "Teks hilang, latar tetap tayang");
text!(CMD_LOGO_DETAIL, "Show the logo in place of the slide", "Logo gereja menggantikan slide di layar");
text!(CMD_STOP_COUNTDOWN, "Stop the countdown", "Hentikan hitung mundur");
text!(CMD_STOP_COUNTDOWN_DETAIL, "Take the countdown off the projector", "Hitungan hilang dari layar");
text!(CMD_HIDE_MESSAGE, "Hide the message", "Sembunyikan pesan");
text!(CMD_HIDE_MESSAGE_DETAIL, "Take the message off the screens", "Pesan hilang dari setiap layar");
text!(CMD_EDIT_SONG, "Edit this song", "Edit lagu");
text!(CMD_EDIT_SONG_DETAIL, "Change the words of the open song", "Perbaiki lirik lagu yang terbuka");
text!(CMD_UNDO_DETAIL, "Take back the last removal", "Mengembalikan yang terakhir dihapus");
text!(CMD_LOOP, "Loop the announcements", "Putar pengumuman");
text!(CMD_LOOP_DETAIL, "Show each one in turn until you stop", "Tayang bergantian sampai dihentikan");

// Top bar and status bar.
text!(BTN_LOOK, "Design", "Design");
text!(BTN_SETTINGS, "Settings", "Settings");
text!(DESIGN_BACKGROUND, "Background", "Background");
text!(DESIGN_TEXT, "Words", "Words");
text!(SETTINGS_SONGS, "Songs", "Songs");
text!(SETTINGS_ABOUT, "About", "About");
pub fn about_version(version: &str) -> String {
    if let Some(text) = say("about_version", None, None, &[("version", text_arg(version))]) {
        return text;
    }
    if indonesian() {
        return format!("Artosia {version}");
    }
    format!("Artosia {version}")
}
text!(BTN_SCREENS, "Screens", "Layar");
text!(BTN_PHONE, "Phone", "Remote HP");
text!(HINT_KEYS, "Keys", "Tombol");
text!(OUTPUT_OFF, "The projector is off", "Layar dimatikan");
text!(OUTPUT_PRACTICE, "Practice: nothing reaches a real screen", "Practice: nothing reaches a real screen");
text!(OUTPUT_STARTING, "Starting the projector", "Layar sedang dinyalakan");
text!(OUTPUT_MISSING, "The projector program is missing from Artosia's folder", "Program layar tidak ada di folder Artosia");
text!(OUTPUT_KEEPS_CLOSING, "The projector keeps closing as it starts", "Program layar terus menutup sendiri setelah mulai");

pub fn output_failed(problem: &str) -> String {
    if let Some(text) = say("output_failed", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Layar gagal dinyalakan: {problem}");
    }
    format!("The projector could not start: {problem}")
}
text!(OUTPUT_NO_SCREEN, "No second screen yet", "Belum ada layar kedua");
text!(OUTPUT_HERE, "Projector showing on this screen", "Layar ditampilkan di jendela ini");
text!(OUTPUT_STUCK, "The projector is not answering", "Layar tidak merespons");
text!(REMOTE_ON, "Phone remote is on", "Remote HP aktif");

pub fn output_on_screen(number: usize) -> String {
    if let Some(text) = say("output_on_screen", None, None, &[("number", text_arg(number))]) {
        return text;
    }
    if indonesian() {
        return format!("Tayang di layar {number}");
    }
    format!("Projector on screen {number}")
}

pub fn library_in_memory(problem: &str) -> String {
    if let Some(text) = say("library_in_memory", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Pustaka gagal dibuka. Pekerjaan sesi ini hilang saat ditutup: {problem}");
    }
    format!("The library could not be opened, so nothing will be saved: {problem}")
}

// Plan.
text!(PLAN_EMPTY, "Nothing planned yet", "Acara masih kosong");
text!(PLAN_ADD, "Add to the plan", "Tambah ke acara");
text!(PLAN_NEW, "Start a new plan", "Acara baru");
text!(PLAN_ITEM_MENU, "More", "Opsi");
text!(PLAN_MOVE_UP, "Move up", "Naikkan");
text!(PLAN_MOVE_DOWN, "Move down", "Turunkan");
text!(PLAN_REMOVE, "Remove", "Keluarkan");
text!(PLAN_MISSING, "Deleted", "Hilang");
text!(PLAN_HEADING_NEW, "Heading", "Pemisah baru");
text!(PLAN_HEADING_WORSHIP, "Worship", "Pujian");
text!(PLAN_HEADING_WORD, "Word", "Firman");
// Several plans, one for each service. The Indonesian slots hold the English until a person writes them from the briefs.
text!(PLANS_TITLE, "Plans", "Plans");
text!(PLANS_TODAY, "Today", "Today");
text!(PLANS_NO_DAY, "No day yet", "No day yet");
text!(PLAN_DUPLICATE, "Duplicate plan", "Duplicate plan");
text!(PLAN_RENAME, "Day and time", "Day and time");
text!(PLAN_DELETE, "Delete plan", "Delete plan");
text!(PLAN_NEW_DETAIL, "Start a plan for another service", "Start a plan for another service");
text!(PLAN_DAY, "Day", "Day");
text!(PLAN_HOUR, "Hour", "Hour");
text!(PLAN_MINUTE, "Minute", "Minute");
text!(PLAN_NAME_LABEL, "Name, if it has one", "Name, if it has one");
/// The days of the week, Monday first, as a plan's day is stored.
const WEEKDAYS_EN: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
pub fn WEEKDAYS() -> &'static [&'static str; 7] {
    if languages::has_table(language()) {
        return languages::weekdays(language(), &WEEKDAYS_EN);
    }
    &WEEKDAYS_EN
}
/// A time of day as the language writes it: 07:00, and 07.00 where a dot is used.
pub fn time_of_day(hour: u8, minute: u8) -> String {
    if let Some(text) = say("time_of_day", None, None, &[("hour", text_arg(format!("{hour:02}"))), ("minute", text_arg(format!("{minute:02}")))]) {
        return text;
    }
    format!("{hour:02}:{minute:02}")
}
/// A weekly service by its day and time: Sunday 07:00.
pub fn service_slot(day: &str, time: &str) -> String {
    if let Some(text) = say("service_slot", None, None, &[("day", text_arg(day)), ("time", text_arg(time))]) {
        return text;
    }
    format!("{day} {time}")
}
// A plan's own look. The Indonesian slots hold the English until a person writes them from the briefs.
text!(LOOK_FOR_USUAL, "The usual look", "The usual look");
text!(LOOK_FOR_PLAN, "This plan's own look", "This plan's own look");
text!(PLAN_OWN_LOOK, "Has its own look", "Has its own look");
text!(KIND_PLAN, "Plan", "Plan");

/// The toast after a plan goes back to the usual look.
pub fn look_own_dropped(name: &str) -> String {
    if let Some(text) = say("look_own_dropped", None, None, &[("name", text_arg(name))]) {
        return text;
    }
    format!("{name} uses the usual look again")
}

pub fn plan_deleted(name: &str) -> String {
    if let Some(text) = say("plan_deleted", None, None, &[("name", text_arg(name))]) {
        return text;
    }
    format!("Deleted plan {name}")
}

pub fn added_to_plan(title: &str) -> String {
    if let Some(text) = say("added_to_plan", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("\u{201c}{title}\u{201d} masuk ke acara");
    }
    format!("Added {title} to the plan")
}

pub fn removed_from_plan(title: &str) -> String {
    if let Some(text) = say("removed_from_plan", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("\u{201c}{title}\u{201d} keluar dari acara");
    }
    format!("Removed {title}")
}

// Slides.
text!(START_TITLE, "What do you want to show?", "Mau menayangkan apa?");
text!(START_HINT, "or just start typing", "atau langsung ketik");
text!(CMD_TOUR, "Take the tour", "Take the tour");
text!(CMD_TOUR_DETAIL, "Five short stops through this window", "Five short stops through this window");
text!(TOUR_SKIP, "Skip", "Skip");
text!(TOUR_DONE, "Done", "Done");
text!(TOUR_PLAN, "The plan holds everything you will show, in order.", "The plan holds everything you will show, in order.");
text!(TOUR_PLAN_MORE, "Drag an item to move it.", "Drag an item to move it.");
text!(TOUR_PREVIEW, "Preview is the slide that comes next.", "Preview is the slide that comes next.");
text!(TOUR_PREVIEW_MORE, "Nothing here reaches the room.", "Nothing here reaches the room.");
text!(TOUR_SHOW, "Show moves Preview into Live.", "Show moves Preview into Live.");
text!(TOUR_SHOW_MORE, "The room sees it only while the projector is on.", "The room sees it only while the projector is on.");
text!(TOUR_LIVE, "Live is what the room sees right now.", "Live is what the room sees right now.");
text!(TOUR_LIVE_MORE, "The Live button switches the projector on and off.", "The Live button switches the projector on and off.");
text!(TOUR_SEARCH, "Search finds any song or verse.", "Search finds any song or verse.");
text!(TOUR_SEARCH_MORE, "Open it, then press Add to plan.", "Open it, then press Add to plan.");
text!(START_SONGS, "Songs", "Lagu");
text!(START_SONGS_DETAIL, "Browse every song", "Telusuri semua lagu");
text!(START_BIBLE, "Bible", "Alkitab");
text!(START_BIBLE_DETAIL, "Pick a book and a verse", "Pilih kitab, lalu ayat");
text!(START_NEW_DETAIL, "Type or paste the words", "Ketik atau tempel liriknya");
text!(START_NO_SONGS, "The library has no songs yet", "Pustaka masih kosong, belum ada lagu");
text!(START_ANNOUNCEMENTS_DETAIL, "Notices before the service", "Tayang sebelum ibadah mulai");
text!(BTN_BRING_OVER, "Bring them over", "Impor sekarang");
text!(BTN_ADD_TO_PLAN, "Add to plan", "Tambah ke acara");
text!(BTN_EDIT, "Edit", "Edit");
text!(BTN_NEXT_VERSE, "Add the next verse", "Tambah ayat berikutnya");
text!(DECK_EMPTY, "This has no slides", "Belum ada slide");
text!(BTN_LOOP_START, "Start the loop", "Putar otomatis");
text!(BTN_LOOP_STOP, "Stop the loop", "Hentikan putaran");
text!(PANE_PREVIEW, "Preview", "Pratinjau");
text!(LIVE, "Live", "Tayang");

// Live column.
text!(BTN_NEXT, "Next", "Lanjut");
text!(BTN_CLEAR_TEXT, "Clear text", "Sembunyikan teks");
text!(BTN_BLACK, "Black", "Layar hitam");
text!(BTN_LOGO, "Logo", "Logo");
text!(LOGO_MISSING, "Choose a logo picture in Look first", "Pasang gambar logo di Tampilan dulu");
text!(STATE_BLACK, "Screen is black", "Layar sedang hitam");
text!(STATE_LOGO, "Logo is showing", "Logo sedang tayang");
text!(STATE_MESSAGE, "Message showing", "Pesan tampil");
text!(STATE_STAGE_MESSAGE, "Stage message showing", "Pesan pemusik tampil");
text!(STAGE_MESSAGE_DETAIL, "Show this message on the stage screen", "Tampil di monitor panggung saja");
text!(HEARD_NOTHING, "Nothing heard yet", "Belum ada yang dikenali");
text!(SUGGEST_HEARD, "Heard just now", "Heard just now");
text!(SUGGEST_HERE, "The singing is here", "The singing is here");
text!(SUGGEST_FOLLOWS, "Often sung after this", "Often sung after this");
text!(SUGGEST_LOST, "The words being sung are not on the screen", "The words being sung are not on the screen");
text!(SUGGEST_SEARCH, "Search", "Search");

pub fn state_countdown(clock: &str) -> String {
    if let Some(text) = say("state_countdown", None, None, &[("clock", text_arg(clock))]) {
        return text;
    }
    if indonesian() {
        return format!("Hitung mundur {clock}");
    }
    format!("Countdown {clock}")
}
text!(LIVE_NOTHING, "Nothing is on the projector", "Nothing is on the projector");
text!(BTN_SHOW, "Show", "Tayangkan");
text!(BTN_PREV, "Prev", "Prev");
text!(BTN_NO_TEXT, "No text", "No text");
text!(STATE_OFF, "Off", "Off");
text!(TIP_SHOW, "Send Preview to Live", "Send Preview to Live");
text!(TIP_PREV, "Previous slide", "Previous slide");
text!(TIP_NEXT, "Next slide", "Next slide");
text!(TIP_LIVE, "Projector on or off", "Projector on or off");
text!(BTN_PREVIOUS_ITEM, "Previous item", "Previous item");
text!(BTN_NEXT_ITEM, "Next item", "Next item");
text!(HELD_CTRL, "Ctrl held", "Ctrl held");
text!(HELD_SHIFT, "Shift held", "Shift held");
text!(HINT_RING, "Move in Preview", "Move in Preview");
text!(HINT_PICK, "Pick a live slide", "Pick a live slide");
text!(HINT_LET_GO, "Let go to show it", "Let go to show it");
text!(HINT_CANCEL, "Cancel", "Cancel");
text!(PLAN_ADD_SONG, "Song", "Song");
text!(PLAN_ADD_PASSAGE, "Bible passage", "Bible passage");
text!(PLAN_ADD_HEADING, "Divider", "Divider");
text!(PLAN_DRAG, "Drag to reorder", "Drag to reorder");
text!(ADDING_TO_PLAN, "Adding to the plan", "Adding to the plan");
text!(BTN_DONE, "Done", "Done");
text!(BTN_EARLIER, "Earlier", "Earlier");
text!(EARLIER_HINT, "Shown before. Opens in Preview.", "Shown before. Opens in Preview.");
text!(LETTER_KEYS, "Letters jump to sections", "Letters jump to sections");
text!(LETTER_KEYS_HINT, "C is the chorus, V a verse. Off, every letter starts a search.", "C is the chorus, V a verse. Off, every letter starts a search.");
text!(COUNTDOWN, "Countdown", "Hitung mundur");
text!(COUNTDOWN_LABEL, "We start in", "Ibadah dimulai dalam");

// Editor.
text!(EDITOR_NEW, "New song", "Lagu baru");
text!(EDITOR_EDIT, "Edit song", "Edit lagu");
text!(EDITOR_NO_SLIDES, "Slides appear here as you type", "Slide muncul di sini saat mengetik");
text!(FIELD_TITLE, "Title", "Judul");
text!(FIELD_AUTHOR, "Author", "Pencipta");
text!(FIELD_KEY, "Key", "Nada");
text!(FIELD_COPYRIGHT, "Copyright", "Hak cipta");
text!(FIELD_CCLI, "CCLI song number", "Nomor CCLI");
text!(FIELD_LYRICS, "Words", "Lirik");
text!(FIELD_ORDER, "Or type it: V1 C*2 V2 C B", "Ketik urutan, misalnya V1 C*2 B");
text!(ORDER, "Order", "Urutan");
text!(ORDER_AS_WRITTEN, "As written", "Sesuai ketikan");
text!(ORDER_EMPTY, "The order appears once the words have sections", "Urutan muncul setelah lirik dibagi per bagian");
text!(ORDER_CHORUS_EACH, "Chorus after each verse", "Reff setelah tiap bait");
text!(BTN_TIDY, "Tidy", "Rapikan");
text!(TIDY_HELP, "Make headings into markers and drop chord lines", "Judul bagian jadi penanda, baris akord dihapus");
text!(SUGGEST_LABEL, "Songs with this title", "Lagu dengan judul sama");
text!(SUGGEST_LIBRARY, "Already in your library", "Sudah di pustaka");
text!(SUGGEST_FETCHING, "Getting the words", "Mengambil dari internet");
text!(LICENCE_NOTE, "Projecting lyrics needs a licence such as CCLI.", "Lirik di layar perlu lisensi, misalnya CCLI");

pub fn suggest_web(artist: &str, source: &str) -> String {
    if let Some(text) = say("suggest_web", Some(if artist.is_empty() { "without_artist" } else { "with_artist" }), None, &[("artist", text_arg(artist)), ("source", text_arg(source))]) {
        return text;
    }
    if indonesian() {
        return if artist.is_empty() { source.to_string() } else { format!("{artist}, {source}") };
    }
    if artist.is_empty() { format!("From {source}") } else { format!("{artist}, from {source}") }
}

pub fn words_from(source: &str) -> String {
    if let Some(text) = say("words_from", None, None, &[("source", text_arg(source))]) {
        return text;
    }
    if indonesian() {
        return format!("Lirik dari {source}. Cocokkan dulu dengan salinan gereja.");
    }
    format!("Words from {source}. Check them against your own copy.")
}
text!(FIELD_SECOND, "Second language", "Lirik bahasa kedua");
text!(LYRICS_PLACEHOLDER, "[1]\nFirst verse\n\n[chorus]\nChorus", "[1]\nBaris bait satu\n[chorus]\nBaris reff");
text!(SECOND_PLACEHOLDER, "The same sections in the other language", "Ketik bagian yang sama dalam bahasa kedua");
text!(MARK_VERSE, "Verse", "Bait");
text!(MARK_CHORUS, "Chorus", "Reff");
text!(MARK_BRIDGE, "Bridge", "Bridge");
text!(MARK_PRECHORUS, "Pre-chorus", "Pra-reff");
text!(MARK_ENDING, "Ending", "Penutup");
text!(KEY_MARK, "Ctrl M", "Ctrl M");
text!(MARKS_APPLY, "Mark", "Mark");
text!(MARKS_APPLY_HELP, "Write these section markers into the words", "Write these section markers into the words");
text!(MARKS_HIDE, "Hide suggestions", "Hide suggestions");
text!(MARKS_THIS, "This stanza", "This stanza");
text!(MARKS_WHY_NAMED, "Read from the name written in the words", "Read from the name written in the words");
text!(MARKS_WHY_REPEAT, "Sung again word for word", "Sung again word for word");
text!(MARKS_WHY_BRIDGE, "Short, and sung once near the end", "Short, and sung once near the end");
text!(MARKS_WHY_ORDER, "The next verse in order", "The next verse in order");
text!(BTN_ADD_SECOND, "Add a second language", "Tambah bahasa kedua");
text!(BTN_SAVE, "Save", "Simpan");
text!(BTN_CANCEL, "Cancel", "Batal");
text!(BTN_DELETE, "Delete", "Hapus");
text!(BTN_DUPLICATE, "Duplicate", "Buat salinan");
text!(BTN_CLOSE, "Close", "Tutup");
text!(BTN_UNDO, "Undo", "Urungkan");
text!(BTN_OPEN, "Open", "Buka");
text!(BTN_CHOOSE, "Choose", "Pilih");
text!(BTN_SHOW_NOW, "Show now", "Tayangkan");
text!(SONG_SAVED, "Song saved", "Lagu disimpan");
text!(EDITOR_CLOSED, "Closed without saving", "Ditutup, perubahan tidak disimpan");
text!(SONG_UNTITLED, "Untitled song", "Tanpa judul");
text!(PASTE_EMPTY, "The clipboard has no text", "Tidak ada teks yang disalin");

pub fn could_not_save(problem: &str) -> String {
    if let Some(text) = say("could_not_save", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Lagu gagal disimpan: {problem}");
    }
    format!("The song was not saved: {problem}")
}

pub fn song_deleted(title: &str) -> String {
    if let Some(text) = say("song_deleted", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("\u{201c}{title}\u{201d} dihapus");
    }
    format!("Deleted {title}")
}

// Look.
text!(LOOK_SAMPLE, "The words appear\nlike this", "Contoh teks\nBaris kedua");
text!(LOOK_CUSTOM, "Your own", "Kustom");
/// The name a look is stored under once the person changed it; the screen says `LOOK_CUSTOM`.
pub const LOOK_CUSTOM_ID: &str = "Custom";
text!(LOOK_MIDNIGHT, "Midnight", "Midnight");
text!(LOOK_CLASSIC_BLUE, "Classic blue", "Classic blue");
text!(LOOK_BLACK, "Black", "Black");
text!(LOOK_FOREST, "Forest", "Forest");
text!(LOOK_EMBER, "Ember", "Ember");
text!(LOOK_PAPER, "Paper", "Paper");
/// A look's name on screen. Looks are stored under their English names, which never change,
/// so a library opened in another language still knows which look it has.
pub fn look_label(name: &str) -> String {
    match name {
        "Midnight" => LOOK_MIDNIGHT(),
        "Classic blue" => LOOK_CLASSIC_BLUE(),
        "Black" => LOOK_BLACK(),
        "Forest" => LOOK_FOREST(),
        "Ember" => LOOK_EMBER(),
        "Paper" => LOOK_PAPER(),
        // Libraries from before the fixed name stored the word of the day.
        "Custom" | "Your own" | "Kustom" => LOOK_CUSTOM(),
        other => other,
    }
    .to_string()
}
text!(SETTINGS_ADD_BIBLE, "More versions", "More versions");
text!(BTN_DELETE_SONG, "Delete song", "Delete song");
text!(CMD_PLAN_DUPLICATE_DETAIL, "The same items again, for another service", "The same items again, for another service");
text!(CMD_PLAN_RENAME_DETAIL, "This plan's day, its time and its name", "This plan's day, its time and its name");
text!(CMD_PLAN_DELETE_DETAIL, "Remove this plan. Undo brings it back", "Remove this plan. Undo brings it back");
text!(CMD_START_HERE, "Start the song here", "Start the song here");
text!(CMD_EARLIER, "Back to what was shown before", "Back to what was shown before");
text!(CMD_EARLIER_DETAIL, "Opens it in Preview at the slide it had reached", "Opens it in Preview at the slide it had reached");
text!(START_HERE_HOW, "Open the song in Preview and go to the slide it should start at, then choose this again", "Open the song in Preview and go to the slide it should start at, then choose this again");
text!(CMD_START_HERE_DETAIL, "The Preview song begins at the picked slide, for a medley", "The Preview song begins at the picked slide, for a medley");
text!(CMD_NEW_ANNOUNCEMENT_DETAIL, "A card for before the service, a verse or birthdays", "A card for before the service, a verse or birthdays");
text!(KEYS_MORE, "Everything else is in search: type what you want, such as copy plan or design", "Everything else is in search: type what you want, such as copy plan or design");
text!(SETTINGS_GENERAL, "General", "General");
/// The hint in the search box while it looks through the Bible. `passage` is an example in
/// the open Bible's own book names, so a KJV says Psalms and an Indonesian Bible says Mazmur.
pub fn search_bible(passage: &str) -> String {
    if let Some(text) = say("search_bible", None, None, &[("passage", text_arg(passage))]) {
        return text;
    }
    format!("{passage}, or words from the verse")
}
text!(NOTICE_SAVED, "Announcement saved", "Announcement saved");
text!(BTN_START_HERE, "Start here", "Start here");
text!(TOUR_BACK, "Back", "Back");
text!(TOUR_NEXT, "Next", "Next");
text!(KEYS_TO, "to", "to");
text!(KEYS_OR, "or", "or");
/// A language's name as the Bible catalogues write it in English, in the window's language
/// for the two languages this program has words for; any other name stays as the catalogue has it.
pub fn language_word(english: &str) -> String {
    match english {
        "Indonesian" => LISTEN_INDONESIAN(),
        "English" => LISTEN_ENGLISH(),
        other => other,
    }
    .to_string()
}
/// A reason written to follow a colon, shown as a line of its own: its first letter a capital.
pub fn as_sentence(clause: &str) -> String {
    let mut letters = clause.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// The key that jumps to the chorus: the first letter of the language's own word for it (R
/// where the word is Reff), unless another kind of section owns that letter; C otherwise. C
/// works in every language as well. Three native readers each said a volunteer who reads
/// Reff never learns that C means it.
pub fn chorus_key() -> char {
    let word = section_label(artosia_core::song::Tag::new(artosia_core::song::SectionKind::Chorus, 1));
    match word.chars().next().map(|letter| letter.to_ascii_uppercase()) {
        Some(letter) if letter.is_ascii_alphabetic() && !"VPBIEX".contains(letter) => letter,
        _ => 'C',
    }
}

/// Whether the language's word for a verse begins with V, so that the V key can stand beside
/// one. Where it does not, verses are reached by their digits and V still works unseen.
pub fn verse_key_shown() -> bool {
    section_label(artosia_core::song::Tag::new(artosia_core::song::SectionKind::Verse, 1)).to_uppercase().starts_with('V')
}

/// The letters of the keys sheet's row about sections, as this language shows them.
pub fn section_keys() -> String {
    if verse_key_shown() { format!("{} V B", chorus_key()) } else { format!("{} B", chorus_key()) }
}

text!(LOOK_BACKGROUND, "Picture or video", "Latar");
text!(LOOK_CHOOSE_FILE, "Choose a file", "Pilih berkas");
text!(LOOK_NO_FILE, "None chosen", "Belum ada berkas");
text!(LOOK_LINES, "Lines on a slide", "Baris per slide");
text!(LOOK_LINES_ALL, "All", "Semua");
text!(LOOK_SIZE, "Text size", "Ukuran teks");
text!(LOOK_SIZE_SMALL, "Small", "Kecil");
text!(LOOK_SIZE_NORMAL, "Normal", "Normal");
text!(LOOK_SIZE_LARGE, "Large", "Besar");
text!(LOOK_PLACE, "Text position", "Posisi teks");
text!(LOOK_PLACE_TOP, "Top", "Atas");
text!(LOOK_PLACE_MIDDLE, "Middle", "Tengah");
text!(LOOK_PLACE_BOTTOM, "Bottom", "Bawah");
text!(LOOK_BIBLE, "Bible", "Versi Alkitab");
text!(LOOK_SECOND_BIBLE, "Second Bible", "Versi Alkitab kedua");
text!(LOOK_NONE, "None", "Tanpa");
text!(LOOK_LOGO, "Logo", "Logo gereja");
text!(LOOK_CHOOSE_LOGO, "Choose a picture", "Pilih gambar");
text!(LOOK_LICENCE, "CCLI licence", "Nomor lisensi CCLI");
text!(LOOK_LICENCE_HINT, "Number", "Nomor");
text!(BACKGROUND_UNKNOWN, "That file is not a picture or a video", "Berkas ini bukan gambar atau video yang didukung");

pub fn bible_failed(code: &str, problem: &str) -> String {
    if let Some(text) = say("bible_failed", None, None, &[("code", text_arg(code)), ("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Alkitab {code} gagal dimuat: {problem}");
    }
    format!("{code} could not be opened: {problem}")
}

// Screens.
text!(SCREENS_OFF, "The projector is off in practice mode.", "Mode latihan: layar tidak dipakai");
text!(SCREENS_ONE, "Plug in the projector and it turns on by itself.", "Layar kedua yang tersambung akan menyala sendiri");
text!(SCREENS_SWITCH, "Projector", "Nyalakan layar");
text!(WALL, "Picture on the wall", "Gambar proyeksi");
text!(WALL_HINT, "Pick a corner, then move it until the picture is square.", "Pilih satu sudut, geser sampai gambar di dinding lurus");
text!(WALL_RESET, "Put it back", "Setel ulang");
text!(WALL_BRIGHTNESS, "Brightness", "Kecerahan");
text!(WALL_CONTRAST, "Contrast", "Kontras");
text!(WALL_COLOUR, "Colour", "Warna");
text!(CORNER_TOP_LEFT, "Top left corner", "Sudut kiri atas");
text!(CORNER_TOP_RIGHT, "Top right corner", "Sudut kanan atas");
text!(CORNER_BOTTOM_RIGHT, "Bottom right corner", "Sudut kanan bawah");
text!(CORNER_BOTTOM_LEFT, "Bottom left corner", "Sudut kiri bawah");
text!(NUDGE_UP, "Up", "Naik");
text!(NUDGE_DOWN, "Down", "Turun");
text!(NUDGE_LEFT, "Left", "Kiri");
text!(NUDGE_RIGHT, "Right", "Kanan");
text!(LESS, "Less", "Kurangi");
text!(MORE, "More", "Tambah");
text!(LOOK_CAMERA, "Camera", "Kamera");
text!(SCREENS_HERE, "Show it on this screen", "Layar di jendela ini");
text!(SCREENS_HERE_HINT, "Escape on the picture closes it.", "Tutup dengan Esc saat kursor di gambar");
text!(LOOK_WINDOW, "Artosia's window", "Warna jendela Artosia");
text!(LOOK_WINDOW_AUTO, "As the computer", "Ikut komputer");
text!(LOOK_WINDOW_LIGHT, "Light", "Terang");
text!(LOOK_WINDOW_DARK, "Dark", "Gelap");
text!(LOOK_TEXT_COLOUR, "Text colour", "Text colour");
text!(LOOK_COLOUR_WHITE, "White", "White");
text!(LOOK_COLOUR_WARM, "Warm white", "Warm white");
text!(LOOK_COLOUR_YELLOW, "Yellow", "Yellow");
text!(LOOK_COLOUR_BLACK, "Black", "Black");
text!(LOOK_COLOUR_HEX, "Custom colour", "Custom colour");
text!(LOOK_COLOUR_HEX_HINT, "Hex, like ffcc00", "Hex, like ffcc00");
text!(LOOK_FONT, "Font", "Font");
text!(LOOK_FONT_DEFAULT, "Artosia's font", "Artosia's font");
text!(LOOK_FONT_SEARCH, "Search fonts", "Search fonts");
text!(LOOK_FONT_NONE, "No font matches", "No font matches");
text!(LOOK_OUTLINE, "Outline", "Outline");
text!(LOOK_OUTLINE_OFF, "Off", "Off");
text!(LOOK_OUTLINE_THIN, "Thin", "Thin");
text!(LOOK_OUTLINE_MEDIUM, "Medium", "Medium");
text!(LOOK_OUTLINE_THICK, "Thick", "Thick");
text!(LOOK_OUTLINE_COLOUR, "Outline colour", "Outline colour");
text!(SCREEN_PROJECTOR, "Projector", "Layar jemaat");
text!(SCREEN_STAGE, "Stage", "Panggung");
text!(SCREEN_UNUSED, "Not used", "Tak dipakai");
text!(SCREENS_STAGE_WINDOW, "Show the stage screen in a window", "Monitor panggung di jendela terpisah");
text!(SCREENS_IDENTIFY, "Show numbers on the screens", "Tampilkan nomor layar");

pub fn screen_name(number: usize) -> String {
    if let Some(text) = say("screen_name", None, None, &[("number", text_arg(number))]) {
        return text;
    }
    if indonesian() {
        return format!("Layar {number}");
    }
    format!("Screen {number}")
}

/// The name in the list of screens, which also says which one Artosia's window is on.
pub fn screen_title(number: usize, this: bool) -> String {
    if let Some(text) = say("screen_title", Some(if this { "with_this" } else { "without_this" }), None, &[("number", text_arg(number))]) {
        return text;
    }
    if indonesian() {
        return if this { format!("Layar {number} (layar ini)") } else { format!("Layar {number}") };
    }
    if this { format!("Screen {number}, this one") } else { format!("Screen {number}") }
}

/// A screen's size, and its zoom when it has one.
pub fn screen_detail(width: u32, height: u32, scale: f32) -> String {
    if let Some(text) = say("screen_detail", Some(if scale > 0.0 && (scale - 1.0).abs() > 0.01 { "with_zoom" } else { "normal_zoom" }), None, &[("width", text_arg(width)), ("height", text_arg(height)), ("scale", text_arg(format!("{}%", (scale * 100.0).round() as u32)))]) {
        return text;
    }
    if indonesian() {
        return {
            if scale > 0.0 && (scale - 1.0).abs() > 0.01 {
                let scale = format!("{}%", (scale * 100.0).round() as u32);
                format!("{width} x {height}, zoom {scale}")
            } else {
                format!("{width} x {height}")
            }
        };
    }
    let mut out = format!("{width} by {height}");
    if scale > 0.0 && (scale - 1.0).abs() > 0.01 {
        out.push_str(&format!(" at {}%", (scale * 100.0).round() as u32));
    }
    out
}

// Phone remote.
text!(REMOTE_SWITCH, "Phone remote", "Remote HP");
text!(REMOTE_SWITCH_DETAIL, "Let phones on this Wi-Fi change slides", "HP di Wi-Fi yang sama bisa ganti slide");
text!(REMOTE_SCAN, "Scan the code, or open this address", "Pindai dengan kamera HP atau ketik alamatnya");
text!(REMOTE_PIN, "PIN", "PIN");

pub fn phones(count: usize) -> String {
    if let Some(text) = say("phones", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return if count == 0 { "Belum ada HP tersambung".to_string() } else { format!("{count} HP tersambung") };
    }
    match count {
        0 => "No phone connected".to_string(),
        1 => "1 phone connected".to_string(),
        _ => format!("{count} phones connected"),
    }
}

pub fn remote_failed(problem: &str) -> String {
    if let Some(text) = say("remote_failed", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Remote HP gagal dijalankan: {problem}");
    }
    format!("The phone remote could not start: {problem}")
}

// Import.
text!(IMPORT_FOUND, "Artosia will copy the songs and Bibles from this folder. EasiSlides keeps working.", "Lagu dan Alkitab EasiSlides akan disalin ke sini. EasiSlides tetap bisa dipakai.");
text!(IMPORT_NOT_FOUND, "EasiSlides was not found on this computer.", "EasiSlides tidak terpasang di komputer ini");

pub fn import_done(songs: usize, bibles: usize) -> String {
    if let Some(text) = say("import_done", Some(if bibles == 0 { "songs_only" } else { "songs_and_bibles" }), None, &[("songs", counted("songs", songs)), ("bibles", counted("bibles", bibles))]) {
        return text;
    }
    if indonesian() {
        return if bibles == 0 { format!("{songs} lagu ditambahkan") } else { format!("{songs} lagu dan {bibles} Alkitab ditambahkan") };
    }
    let songs = if songs == 1 { "1 song".to_string() } else { format!("{songs} songs") };
    match bibles {
        0 => format!("Added {songs}"),
        1 => format!("Added {songs} and 1 Bible"),
        _ => format!("Added {songs} and {bibles} Bibles"),
    }
}

/// What an import of a song folder did, as one or two whole sentences.
pub fn import_summary(added: usize, updated: usize, kept: usize) -> String {
    if let Some(text) = import_summary_table(added, updated, kept) {
        return text;
    }
    if indonesian() {
        return {
            let changed = match (added, updated) {
                (0, 0) => String::new(),
                (_, 0) => format!("{added} lagu ditambahkan"),
                (0, _) => format!("{updated} lagu diperbarui"),
                _ => format!("{added} lagu baru, {updated} diperbarui"),
            };
            match (changed.is_empty(), kept) {
                (true, 0) => "Semua lagu sudah ada".to_string(),
                (false, 0) => changed,
                (true, _) => format!("{kept} lagu dibiarkan karena diedit di sini"),
                (false, _) => format!("{changed}. {kept} dibiarkan karena diedit di sini."),
            }
        };
    }
    let count = |n: usize| if n == 1 { "1 song".to_string() } else { format!("{n} songs") };
    let changed = match (added, updated) {
        (0, 0) => String::new(),
        (added, 0) => format!("Added {}", count(added)),
        (0, updated) => format!("Updated {}", count(updated)),
        (added, updated) => format!("Added {added} and updated {}", count(updated)),
    };
    match (changed.is_empty(), kept) {
        (true, 0) => "Everything in that folder is already here".to_string(),
        (false, 0) => changed,
        (true, kept) => format!("Kept {} you edited here", count(kept)),
        (false, kept) => format!("{changed}. Kept {} you edited here", count(kept)),
    }
}

pub fn files_unreadable(failed: usize) -> String {
    if let Some(text) = say("files_unreadable", None, Some(failed), &[]) {
        return text;
    }
    if indonesian() {
        return format!("{failed} berkas gagal dimuat");
    }
    if failed == 1 { "1 file could not be read".to_string() } else { format!("{failed} files could not be read") }
}

pub fn import_partial(songs: usize, failed: usize) -> String {
    if let Some(text) = say("import_partial", None, None, &[("songs", counted("songs", songs)), ("files", counted("files", failed))]) {
        return text;
    }
    if indonesian() {
        return format!("{songs} lagu masuk, {failed} berkas gagal dimuat");
    }
    let files = if failed == 1 { "1 file".to_string() } else { format!("{failed} files") };
    format!("Added {songs} songs. {files} could not be read")
}

// Announcements.
text!(ANNOUNCEMENTS_EMPTY, "No announcements yet", "Belum ada pengumuman");
text!(ANNOUNCEMENT_NEW, "New announcement", "Pengumuman baru");
text!(ANNOUNCEMENT_BODY, "Details", "Keterangan");
text!(ANNOUNCEMENT_TITLE_HINT, "Such as Youth night", "Such as Youth night");
text!(ANNOUNCEMENT_BODY_HINT, "Such as Friday at 7 pm in the hall", "Such as Friday at 7 pm in the hall");
text!(ANNOUNCEMENT_SECONDS, "Seconds on screen", "Detik per pengumuman");
text!(ANNOUNCEMENT_START_FROM, "Start from", "Mulai dari contoh");
text!(ANNOUNCEMENT_BRING_IN, "Bring in", "Ambil dari berkas");
text!(ANNOUNCEMENT_FILL, "Fill in by itself", "Isi otomatis");
text!(FILL_NOTHING, "No", "Manual");
text!(FILL_WEEK, "Birthdays this week", "Ulang tahun minggu ini");
text!(FILL_MONTH, "Birthdays this month", "Ulang tahun bulan ini");
text!(ANNOUNCEMENT_PICTURE, "Picture", "Gambar");
text!(BTN_REMOVE_PICTURE, "Remove", "Lepas");
text!(CMD_IMPORT_PPTX, "Import a PowerPoint", "Impor dari PowerPoint");
text!(CMD_IMPORT_PPTX_DETAIL, "Each slide becomes an announcement", "Tiap slide jadi satu pengumuman");
text!(CMD_IMPORT_BIRTHDAYS, "Import a birthday list", "Impor ulang tahun");
text!(CMD_IMPORT_BIRTHDAYS_DETAIL, "A Word or PowerPoint table of names and dates", "Tabel nama dan tanggal lahir di Word");
text!(BIRTHDAYS_NONE_FOUND, "No names with dates were found in that file", "Tidak ada baris berisi nama dan tanggal lahir");
const MONTHS_EN: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_ID: [&str; 12] = ["Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember"];
pub fn MONTHS() -> &'static [&'static str; 12] {
    if languages::has_table(language()) {
        return languages::months(language(), &MONTHS_EN);
    }
    if indonesian() { &MONTHS_ID } else { &MONTHS_EN }
}

pub fn could_not_read(problem: &str) -> String {
    if let Some(text) = say("could_not_read", None, None, &[("problem", text_arg(problem))]) {
        return text;
    }
    if indonesian() {
        return format!("Berkas gagal dibuka: {problem}");
    }
    format!("That file could not be read: {problem}")
}

pub fn pptx_imported(added: usize, lost: usize) -> String {
    if let Some(text) = say("pptx_imported", Some(if lost == 0 { "without_lost" } else { "with_lost" }), None, &[("added", counted("announcements", added)), ("lost", counted("pictures", lost))]) {
        return text;
    }
    if indonesian() {
        return if lost == 0 { format!("{added} pengumuman dibuat") } else { format!("{added} pengumuman dibuat, {lost} gambar gagal dipindahkan") };
    }
    let added = if added == 1 { "Added 1 announcement".to_string() } else { format!("Added {added} announcements") };
    match lost {
        0 => added,
        1 => format!("{added}. 1 picture could not be brought in"),
        _ => format!("{added}. {lost} pictures could not be brought in"),
    }
}

pub fn birthdays_on_file(count: usize) -> String {
    if let Some(text) = say("birthdays_on_file", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return if count == 0 { "Belum ada data ulang tahun".to_string() } else { format!("{count} ulang tahun tersimpan") };
    }
    match count {
        0 => "No birthdays on file".to_string(),
        1 => "1 birthday on file".to_string(),
        _ => format!("{count} birthdays on file"),
    }
}

pub fn birthdays_imported(count: usize, unread: usize) -> String {
    if let Some(text) = say("birthdays_imported", Some(if unread == 0 { "none_skipped" } else { "some_skipped" }), if unread == 0 { Some(count) } else { None }, &[("birthdays", counted("birthdays", count)), ("rows", counted("rows", unread))]) {
        return text;
    }
    if indonesian() {
        return if unread == 0 { format!("{count} ulang tahun tersimpan") } else { format!("{count} ulang tahun tersimpan, {unread} baris dilewati") };
    }
    let kept = birthdays_on_file(count);
    match unread {
        0 => kept,
        1 => format!("{kept}. 1 row could not be read"),
        _ => format!("{kept}. {unread} rows could not be read"),
    }
}
text!(ANNOUNCEMENT_PREVIEW, "On the projector", "Pratinjau");

// Cards that fill themselves: a Bible passage and the birthdays. Indonesian is written later
// from the brief; English stands in both.
text!(FILL_PASSAGE, "Bible passage", "Bible passage");
text!(ANNOUNCEMENT_PASSAGE, "Bible reference", "Bible reference");
text!(PASSAGE_NO_BIBLE, "No Bible is installed, so this card has no words", "No Bible is installed, so this card has no words");
text!(PASSAGE_UNREADABLE, "The Bible has no verse at that reference", "The Bible has no verse at that reference");
text!(NOTICE_NOTHING_TO_SHOW, "This card has nothing to show today", "This card has nothing to show today");

/// Which days a birthday card that follows the week covers.
pub fn birthdays_cover_week(from: (u8, u8), to: (u8, u8)) -> String {
    let month = |month: u8| MONTHS()[usize::from(month.clamp(1, 12)) - 1];
    if let Some(text) = say("birthdays_cover_week", None, None, &[("from_day", text_arg(from.1)), ("from_month", text_arg(month(from.0))), ("to_day", text_arg(to.1)), ("to_month", text_arg(month(to.0)))]) {
        return text;
    }
    format!("Covers {} {} to {} {}", from.1, month(from.0), to.1, month(to.0))
}

/// Which days a birthday card that follows the month covers.
pub fn birthdays_cover_month(month: u8) -> String {
    if let Some(text) = say("birthdays_cover_month", None, None, &[("month", text_arg(MONTHS()[usize::from(month.clamp(1, 12)) - 1]))]) {
        return text;
    }
    format!("Covers all of {}", MONTHS()[usize::from(month.clamp(1, 12)) - 1])
}

/// How many names a birthday card holds today.
pub fn names_today(count: usize) -> String {
    if let Some(text) = say("names_today", None, Some(count), &[]) {
        return text;
    }
    match count {
        0 => "No names today".to_string(),
        1 => "1 name today".to_string(),
        _ => format!("{count} names today"),
    }
}
/// Ready-made announcements: a title and words to replace.
/// The four announcements of the practice library, in the window's language: a welcome, an
/// evening for the young people, a prayer meeting, and the week as a table (a tab between
/// words makes columns, as a table brought in from PowerPoint has).
const PRACTICE_NOTICES_EN: [(&str, &str); 4] = [
    ("Welcome", "We are glad you are here.\nThe service starts soon."),
    ("Youth night", "Friday at 7 pm in the hall.\nBring a friend."),
    ("Prayer meeting", "Wednesday at 6 am.\nEveryone is welcome."),
    ("This week", "Sunday\t9 am\tMain service\nSunday\t5 pm\tEvening service\nWednesday\t7 pm\tPrayer meeting\nSaturday\t4 pm\tYouth"),
];
pub fn PRACTICE_NOTICES() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "PRACTICE_NOTICES", &PRACTICE_NOTICES_EN);
    }
    &PRACTICE_NOTICES_EN
}
const ANNOUNCEMENT_TEMPLATES_EN: [(&str, &str); 6] = [
    ("Welcome", "We are glad you are here."),
    ("Service times", "Sunday at 9 am and 5 pm"),
    ("This week", "Wednesday: prayer meeting at 7 pm"),
    ("Verse of the week", "Write the verse here"),
    ("Offering", "Account name and number"),
    ("Birthdays", "Names this week"),
];
const ANNOUNCEMENT_TEMPLATES_ID: [(&str, &str); 6] = [
    ("Selamat datang", "Kami senang bisa beribadah bersama hari ini"),
    ("Jam ibadah", "Minggu, jam 07.00 dan 17.00"),
    ("Minggu ini", "Doa bersama Rabu malam, jam 19.00"),
    ("Ayat minggu ini", "Ganti baris ini dengan ayatnya"),
    ("Persembahan", "Tulis atas nama dan nomor rekening di sini"),
    ("Ulang tahun", "Tulis nama yang berulang tahun minggu ini"),
];
pub fn ANNOUNCEMENT_TEMPLATES() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "ANNOUNCEMENT_TEMPLATES", &ANNOUNCEMENT_TEMPLATES_EN);
    }
    if indonesian() { &ANNOUNCEMENT_TEMPLATES_ID } else { &ANNOUNCEMENT_TEMPLATES_EN }
}

pub fn announcements(count: usize) -> String {
    if let Some(text) = say("announcements", None, Some(count), &[]) {
        return text;
    }
    if indonesian() {
        return if count == 0 { "Belum ada pengumuman".to_string() } else { format!("{count} pengumuman") };
    }
    match count {
        0 => "No announcements yet".to_string(),
        1 => "1 announcement".to_string(),
        _ => format!("{count} announcements"),
    }
}

pub fn seconds_short(seconds: u32) -> String {
    if let Some(text) = say("seconds_short", None, None, &[("seconds", text_arg(seconds))]) {
        return text;
    }
    if indonesian() {
        return format!("{seconds} dtk");
    }
    format!("{seconds} s")
}

// The key sheet: what a key is called and what it does.
text!(KEYS_TITLE, "Keyboard shortcuts", "Tombol pintas");
text!(KEYS_SHOWING, "Showing", "Saat tayang");
text!(KEYS_FINDING, "Finding", "Mencari");
text!(KEYS_PLAN, "The plan", "Acara");
text!(KEYS_EDITING, "Editing", "Editor lagu");
const KEYS_SHOWING_ROWS_EN: [(&str, &str); 18] = [
    ("Down Space", "Next slide"),
    ("Up", "Previous slide"),
    ("Left", "First slide of the live song"),
    ("Right", "Last slide of the live song"),
    ("Shift Up Down", "Pick a slide of the live song, let go to show it"),
    ("Ctrl Up Down", "Move the ring in Preview, Enter shows it"),
    ("Enter", "Show the slide with the ring"),
    ("1 to 9", "Jump to that section of the live song"),
    ("C V B", "Jump to the chorus, the next verse, the bridge"),
    ("Ctrl 1 to 9", "Put the ring on that section in Preview"),
    ("Esc", "Bring the live song into Preview"),
    ("Backspace", "Bring what was shown before into Preview"),
    ("F1", "Clear the text"),
    ("F2", "Black"),
    ("F3", "Logo"),
    ("F4", "Bring what the listener heard into Preview"),
    ("F5", "Bring the announcements into Preview"),
    ("F6", "Switch the projector on or off"),
];
const KEYS_SHOWING_ROWS_ID: [(&str, &str); 18] = [
    ("Panah bawah, Spasi", "Tayangkan slide berikutnya"),
    ("Panah atas", "Tayangkan slide sebelumnya"),
    ("Panah kiri", "Loncat ke slide pertama lagu yang tayang"),
    ("Panah kanan", "Loncat ke slide terakhir lagu yang tayang"),
    ("Shift panah", "Pilih slide lagu tayang, lepas Shift tayangkan"),
    ("Ctrl panah", "Geser bingkai di pratinjau, tayang saat Enter"),
    ("Enter", "Tayangkan slide yang berbingkai putih"),
    ("1 sampai 9", "Loncat ke bagian lagu sesuai nomor"),
    ("C, V, B", "Jump to the chorus, the next verse, the bridge"),
    ("Ctrl 1 to 9", "Put the ring on that section in Preview"),
    ("Esc", "Bawa lagu yang tayang ke pratinjau"),
    ("Backspace", "Bring what was shown before into Preview"),
    ("F1", "Sembunyikan teks, latar tetap tayang"),
    ("F2", "Hitamkan layar"),
    ("F3", "Tayangkan logo saja"),
    ("F4", "Bawa ayat atau lagu yang dikenali ke pratinjau"),
    ("F5", "Bawa pengumuman ke pratinjau"),
    ("F6", "Switch the projector on or off"),
];
/// The key column of each group of the keys sheet. Keys are drawn as the keyboard prints
/// them and are never translated; only the second column is.
pub fn KEYS_SHOWING_KEYS() -> &'static [(&'static str, &'static str)] {
    &KEYS_SHOWING_ROWS_EN
}
pub fn KEYS_FINDING_KEYS() -> &'static [(&'static str, &'static str)] {
    &KEYS_FINDING_ROWS_EN
}
pub fn KEYS_PLAN_KEYS() -> &'static [(&'static str, &'static str)] {
    &KEYS_PLAN_ROWS_EN
}
pub fn KEYS_EDITING_KEYS() -> &'static [(&'static str, &'static str)] {
    &KEYS_EDITING_ROWS_EN
}
pub fn KEYS_SHOWING_ROWS() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "KEYS_SHOWING_ROWS", &KEYS_SHOWING_ROWS_EN);
    }
    if indonesian() { &KEYS_SHOWING_ROWS_ID } else { &KEYS_SHOWING_ROWS_EN }
}
const KEYS_FINDING_ROWS_EN: [(&str, &str); 7] = [
    ("A to Z", "Search songs, the Bible and commands"),
    ("Ctrl K or /", "Search, or browse every song"),
    ("Enter", "Open the picked result"),
    ("Ctrl Enter", "Show it now"),
    ("Shift Enter", "Add it to the plan, next in line"),
    ("Tab", "Search inside the picked song"),
    ("Esc", "Close the search"),
];
const KEYS_FINDING_ROWS_ID: [(&str, &str); 7] = [
    ("Huruf apa saja", "Ketik huruf di mana saja untuk mencari"),
    ("Ctrl K", "Buka pencarian, kosong berarti semua lagu"),
    ("Enter", "Buka hasil yang disorot"),
    ("Ctrl Enter", "Buka dan langsung tayangkan"),
    ("Shift Enter", "Tambah setelah yang tayang, atau di akhir"),
    ("Tab", "Cari bagian lagu yang disorot"),
    ("Esc", "Close the search"),
];
pub fn KEYS_FINDING_ROWS() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "KEYS_FINDING_ROWS", &KEYS_FINDING_ROWS_EN);
    }
    if indonesian() { &KEYS_FINDING_ROWS_ID } else { &KEYS_FINDING_ROWS_EN }
}
const KEYS_PLAN_ROWS_EN: [(&str, &str); 8] = [
    ("Tab", "Bring the next plan item into Preview"),
    ("Shift Tab", "Bring the item before it into Preview"),
    ("Alt Up Down", "Move the Preview item earlier or later"),
    ("Ctrl M", "Start the Preview song at the ring"),
    ("Delete", "Take the Preview item out of the plan"),
    ("Shift Enter", "Add what is in Preview to the plan"),
    ("Ctrl Tab", "Open the next plan; with Shift, the one before"),
    ("Ctrl Shift N", "Start a new plan"),
];
const KEYS_PLAN_ROWS_ID: [(&str, &str); 8] = [
    ("Tab", "Bawa item acara berikutnya ke pratinjau"),
    ("Shift Tab", "Bawa item acara sebelumnya ke pratinjau"),
    ("Alt panah atas, bawah", "Geser item pratinjau naik atau turun"),
    ("Ctrl M", "Start the Preview song at the ring"),
    ("Delete", "Keluarkan item pratinjau, bisa diurungkan"),
    ("Shift Enter", "Add what is in Preview to the plan"),
    ("Ctrl Tab", "Open the next plan; with Shift, the one before"),
    ("Ctrl Shift N", "Start a new plan"),
];
pub fn KEYS_PLAN_ROWS() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "KEYS_PLAN_ROWS", &KEYS_PLAN_ROWS_EN);
    }
    if indonesian() { &KEYS_PLAN_ROWS_ID } else { &KEYS_PLAN_ROWS_EN }
}
const KEYS_EDITING_ROWS_EN: [(&str, &str); 6] = [
    ("Ctrl N", "New song"),
    ("Ctrl E", "Edit the open song"),
    ("Ctrl S", "Save"),
    ("Ctrl 1 to 5", "Start a verse, chorus or other section"),
    ("Down Enter", "Fill in the whole song offered under the title"),
    ("Ctrl Z", "Undo the last removal"),
];
const KEYS_EDITING_ROWS_ID: [(&str, &str); 6] = [
    ("Ctrl N", "Mulai lagu baru"),
    ("Ctrl E", "Edit lagu yang terbuka"),
    ("Ctrl S", "Simpan"),
    ("Ctrl 1 sampai 5", "Sisipkan penanda bait, reff, atau lainnya"),
    ("Down Enter", "Terima lagu yang diusulkan sekaligus"),
    ("Ctrl Z", "Kembalikan yang terakhir dihapus"),
];
pub fn KEYS_EDITING_ROWS() -> &'static [(&'static str, &'static str)] {
    if languages::has_table(language()) {
        return languages::rows(language(), "KEYS_EDITING_ROWS", &KEYS_EDITING_ROWS_EN);
    }
    if indonesian() { &KEYS_EDITING_ROWS_ID } else { &KEYS_EDITING_ROWS_EN }
}

pub fn announcement_deleted(title: &str) -> String {
    if let Some(text) = say("announcement_deleted", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("Pengumuman \u{201c}{title}\u{201d} dihapus");
    }
    format!("Deleted {title}")
}

// The screen that faces the musicians. Its words travel to the projector program with each state.
text!(STAGE_CLOCK, "Clock", "Jam");
text!(STAGE_TIMER, "Timer", "Hitung mundur");
text!(STAGE_BLACK, "Screen is black", "Layar jemaat hitam");
text!(STAGE_NOTHING, "Nothing on screen", "Belum ada yang tayang");

/// What comes after the live slide. Called with "{label}" it gives the pattern itself.
pub fn stage_next(label: &str) -> String {
    if let Some(text) = say("stage_next", None, None, &[("label", text_arg(label))]) {
        return text;
    }
    if indonesian() {
        return format!("Berikutnya: {label}");
    }
    format!("Next: {label}")
}

/// Which stop of the tour is showing, out of how many.
pub fn tour_place(at: usize, total: usize) -> String {
    if let Some(text) = say("tour_place", None, None, &[("at", text_arg(at.to_string())), ("total", text_arg(total.to_string()))]) {
        return text;
    }
    if indonesian() {
        return format!("{at} dari {total}");
    }
    format!("{at} of {total}")
}

/// Which slide of how many. Called with "{index}" and "{count}" it gives the pattern itself.
pub fn stage_place(index: &str, count: &str) -> String {
    if let Some(text) = say("stage_place", None, None, &[("index", text_arg(index)), ("count", text_arg(count))]) {
        return text;
    }
    if indonesian() {
        return format!("{index} dari {count}");
    }
    format!("Slide {index} of {count}")
}

/// A song section's name as a volunteer reads it: "Verse 2", "Chorus"; "Bait 2", "Reff".
pub fn section_label(tag: artosia_core::song::Tag) -> String {
    if languages::has_table(language()) {
        return section_label_table(tag);
    }
    use artosia_core::song::SectionKind;
    if !indonesian() {
        return tag.label();
    }
    let name = match tag.kind {
        SectionKind::Verse => "Bait",
        SectionKind::PreChorus => "Pra-reff",
        SectionKind::Chorus => "Reff",
        SectionKind::Bridge => "Bridge",
        SectionKind::Intro => "Intro",
        SectionKind::Ending => "Penutup",
        SectionKind::Other => "Bagian",
    };
    let numbered = matches!(tag.kind, SectionKind::Verse | SectionKind::Other) || tag.number > 1;
    if numbered { format!("{name} {}", tag.number) } else { name.to_string() }
}

// The language switch, named in each language's own word.
text!(LOOK_LANGUAGE, "Language", "Bahasa");
pub const LANGUAGE_ENGLISH: &str = "English";
pub const LANGUAGE_INDONESIAN: &str = "Bahasa Indonesia";

// What the search learned. Indonesian is written later from the brief; English stands in both.
text!(LEARN_SECTION, "Search", "Pencarian");
text!(LEARN_ON, "Learn from use", "Ingat pilihan di PC ini");
text!(LEARN_ON_HINT, "Songs you pick often come first among equal matches", "Hanya mengurutkan hasil yang sama cocok, acara tetap di atas");
text!(LEARN_FORGET, "Forget what the search learned", "Lupakan pilihan yang diingat");
text!(LEARN_FORGOTTEN, "Search history forgotten", "Semua pilihan sudah dilupakan");
text!(GROUP_WEEKDAY, "Sung on this day of the week", "Lagu di hari yang sama");
text!(GROUP_RECENT, "Picked from the list lately", "Lagu yang baru dipilih");

/// The heading over the songs that came after the live song.
pub fn group_follows(title: &str) -> String {
    if let Some(text) = say("group_follows", None, None, &[("title", text_arg(title))]) {
        return text;
    }
    if indonesian() {
        return format!("Lagu yang pernah menyusul {title}");
    }
    format!("Often sung after {title}")
}

fn times_of(count: u32) -> String {
    if count == 1 { "1 time".to_string() } else { format!("{count} times") }
}

fn services_of(count: u32) -> String {
    if count == 1 { "1 service".to_string() } else { format!("{count} services") }
}

/// On the row: how often this text led to this song, with the text.
pub fn habit_chip(typed: &str, picks: u32) -> String {
    if let Some(text) = say("habit_chip", None, Some(picks as usize), &[("typed", text_arg(typed)), ("picks", text_arg(picks))]) {
        return text;
    }
    if indonesian() {
        return format!("Dipilih {picks} kali untuk \u{201c}{typed}\u{201d}");
    }
    format!("Picked for \u{201c}{typed}\u{201d} {}", times_of(picks))
}

/// The same with the day of the last pick, in the preview.
pub fn habit_line(typed: &str, picks: u32, when: &str) -> String {
    if let Some(text) = say("habit_line", None, Some(picks as usize), &[("typed", text_arg(typed)), ("picks", text_arg(picks)), ("when", text_arg(when))]) {
        return text;
    }
    if indonesian() {
        return format!("Dipilih {picks} kali untuk \u{201c}{typed}\u{201d}, terakhir {when}");
    }
    format!("Picked for \u{201c}{typed}\u{201d} {}, last {when}", times_of(picks))
}

/// Under a song that came after the live one.
pub fn follows_line(count: u32, when: &str) -> String {
    if let Some(text) = say("follows_line", None, Some(count as usize), &[("when", text_arg(when))]) {
        return text;
    }
    if indonesian() {
        return format!("Menyusul {count} kali, terakhir {when}");
    }
    format!("Came next {}, last {when}", times_of(count))
}

/// Under a song sung on this day of the week: in `count` of the last `of` services that day.
pub fn weekday_line(count: u32, of: u32) -> String {
    if let Some(text) = say("weekday_line", None, Some(of as usize), &[("count", text_arg(count)), ("of", text_arg(of))]) {
        return text;
    }
    if indonesian() {
        return format!("Dinyanyikan {count} dari {of} ibadah");
    }
    format!("Sung on this day in {count} of {}", services_of(of))
}

/// Under a song picked from the list with nothing typed.
pub fn recent_line(picks: u32, when: &str) -> String {
    if let Some(text) = say("recent_line", None, Some(picks as usize), &[("picks", text_arg(picks)), ("when", text_arg(when))]) {
        return text;
    }
    if indonesian() {
        return format!("Dipilih {picks} kali, terakhir {when}");
    }
    format!("Picked {}, last {when}", times_of(picks))
}

/// A short chip: the song usually begins at this section.
pub fn starts_chip(section: &str) -> String {
    if let Some(text) = say("starts_chip", None, None, &[("section", text_arg(section))]) {
        return text;
    }
    if indonesian() {
        return format!("Mulai di {section}");
    }
    format!("Starts at {section}")
}

/// In the preview: the section it usually begins at, and how often.
pub fn starts_line(section: &str, times: u32, of: u32) -> String {
    if let Some(text) = say("starts_line", None, None, &[("section", text_arg(section)), ("times", text_arg(times)), ("of", text_arg(of))]) {
        return text;
    }
    if indonesian() {
        return format!("Biasanya mulai di {section}, {times} dari {of} kali");
    }
    format!("Usually started at {section}, {times} of {of} times")
}

// Where a song of the plan begins. Indonesian is written later from the brief; English stands in both.
text!(TIP_START_HERE, "Start the song at this slide", "Start the song at this slide");
text!(PLAN_STARTS_AT, "Starts at", "Starts at");
text!(PLAN_START_BEGINNING, "The beginning", "The beginning");

text!(SCREEN_OWN, "Artosia is on this screen", "Layar untuk Artosia sendiri");

// Bibles: a file the church holds, and eBible's free list.
text!(BIBLES_TITLE, "Bibles", "Tambah Alkitab");
text!(BIBLES_MORE, "More Bibles", "Tambah terjemahan");
text!(BIBLES_OWN_FILE, "Your own file", "Terjemahan dari berkas");
text!(BIBLES_CHOOSE_FILE, "Choose a Bible file", "Pilih berkas");
text!(BIBLES_FILE_HINT, "MyBible, USFX, or a zip of USFM files", "MyBible, USFX, atau zip berisi USFM");
text!(BIBLES_PRACTICE, "Practice mode keeps no Bibles", "Tidak bisa tambah Alkitab saat latihan");
text!(BIBLES_FILE_UNSUPPORTED, "That file is not a Bible Artosia can read", "Berkas ini bukan Alkitab yang bisa dibaca");
text!(BIBLES_FILE_EMPTY, "That file holds no verses", "Berkas ini tidak berisi ayat");
text!(CMD_BIBLE_FILE, "Add a Bible from a file", "Tambah Alkitab dari berkas");
text!(CMD_BIBLE_FILE_DETAIL, "MyBible, USFX, or a zip of USFM files", "MyBible, USFX, atau zip berisi USFM");
text!(CMD_BIBLES, "Get more Bibles", "Tambah terjemahan Alkitab");
text!(CMD_BIBLES_FIND_DETAIL, "Download translations in many languages", "Download translations in many languages");

text!(BIBLES_LIST, "Bibles to download", "Bibles to download");
text!(BIBLES_SEARCH, "Search by language or name", "Cari bahasa atau terjemahan");
text!(BIBLES_LOADING, "Getting the list of Bibles", "Mengambil daftar Alkitab");
text!(BIBLES_LOAD_FAILED, "Could not get the list of Bibles", "Daftar Alkitab tidak bisa diambil");
text!(BIBLES_RETRY, "Try again", "Coba lagi");
text!(BIBLES_INSTALLED, "Installed", "Sudah ada");
text!(BIBLES_DOWNLOAD, "Download", "Unduh");
text!(BIBLES_DOWNLOADING, "Downloading", "Mengunduh");
text!(BIBLES_USE, "Use this Bible", "Pakai terjemahan ini");
text!(BIBLES_IN_USE, "In use", "Sedang dipakai");
text!(BIBLES_COPYRIGHT, "Copyright", "Hak cipta");
text!(BIBLES_PICK_HINT, "Pick a Bible to read its copyright", "Pilih terjemahan untuk melihat hak ciptanya");
text!(BIBLES_SCOPE_WHOLE, "Whole Bible", "Alkitab lengkap");
text!(BIBLES_SCOPE_PART, "Some books", "Sebagian kitab");
text!(BIBLES_SOURCE_EBIBLE, "eBible", "eBible");
text!(BIBLES_SOURCE_BEBLIA, "Beblia", "Beblia");
text!(BIBLES_NO_COPYRIGHT, "This file states no copyright", "This file states no copyright");
text!(BIBLES_NOTICE_LOADING, "Reading the file's own notice", "Reading the file's own notice");
text!(BIBLES_NOTICE_FAILED, "Could not read the file's notice", "Could not read the file's notice");

// Backdrops: ready pictures and videos to put behind the words. English in both slots until the brief is written for each language.
text!(BACKDROPS_TITLE, "Backdrops", "Backdrops");
text!(BACKDROPS_SEARCH, "Search backdrops", "Search backdrops");
text!(BACKDROPS_ALL, "All", "All");
text!(BACKDROPS_SKY, "Sky", "Sky");
text!(BACKDROPS_MOUNTAINS, "Mountains", "Mountains");
text!(BACKDROPS_WATER, "Water", "Water");
text!(BACKDROPS_NATURE, "Nature", "Nature");
text!(BACKDROPS_LIGHT, "Light", "Light");
text!(BACKDROPS_NIGHT, "Night", "Night");
text!(BACKDROPS_CANDLES, "Candles", "Candles");
text!(BACKDROPS_CHURCH, "Church", "Church");
text!(BACKDROPS_PEOPLE, "Hands", "Hands");
text!(BACKDROPS_TEXTURE, "Textures", "Textures");
text!(BACKDROPS_USE, "Use", "Use");
text!(BACKDROPS_DOWNLOADING, "Downloading", "Downloading");
text!(BACKDROPS_OFFLINE, "No internet connection", "No internet connection");
text!(BACKDROPS_AUTHOR, "Author", "Author");
text!(BACKDROPS_LICENCE, "Licence", "Licence");
text!(BACKDROPS_PAGE, "Page", "Page");
text!(BACKDROPS_SIZE, "Size", "Size");
text!(BACKDROPS_PICK_HINT, "Pick a backdrop to see its author and licence", "Pick a backdrop to see its author and licence");
text!(BACKDROPS_DIM, "Bright or busy, so dim the look", "Bright or busy, so dim the look");
text!(BACKDROPS_HERE, "On this computer", "On this computer");
text!(BACKDROPS_IN_USE, "Behind the words now", "Behind the words now");

pub fn backdrop_failed(why: &str) -> String {
    if let Some(text) = say("backdrop_failed", None, None, &[("why", text_arg(why))]) {
        return text;
    }
    format!("Could not get the backdrop: {why}")
}
text!(CMD_BACKDROPS_DETAIL, "Ready pictures and videos to put behind the words", "Ready pictures and videos to put behind the words");

// Why a download or a file failed: clauses that finish "Could not ...: {why}".
text!(WHY_OFFLINE, "there is no internet connection", "there is no internet connection");
text!(WHY_SLOW, "it took too long", "it took too long");
text!(WHY_BUSY, "the site is busy, try again in a minute", "the site is busy, try again in a minute");
text!(WHY_BROKEN, "the connection broke on the way", "the connection broke on the way");
text!(WHY_UNREADABLE, "what arrived could not be read", "what arrived could not be read");
text!(WHY_NOT_BIBLE, "the file is not a Bible", "the file is not a Bible");
text!(WHY_DAMAGED, "the file arrived damaged", "the file arrived damaged");
text!(WHY_DISK, "the file could not be saved on this computer", "the file could not be saved on this computer");
pub fn why_status(code: u16) -> String {
    if let Some(text) = say("why_status", None, None, &[("code", text_arg(code))]) {
        return text;
    }
    format!("the server answered {code}")
}
text!(LYRICS_OFFLINE, "Could not reach the lyrics service. Check the internet connection", "Could not reach the lyrics service. Check the internet connection");
text!(LYRICS_NOT_FOUND, "The lyrics service does not have this song", "The lyrics service does not have this song");
text!(LYRICS_BUSY, "The lyrics service is busy. Try again in a minute", "The lyrics service is busy. Try again in a minute");
text!(LYRICS_UNREADABLE, "The lyrics service sent something Artosia cannot read", "The lyrics service sent something Artosia cannot read");
pub fn lyrics_busy_for(seconds: u64) -> String {
    if let Some(text) = say("lyrics_busy_for", None, Some(seconds as usize), &[("seconds", text_arg(seconds))]) {
        return text;
    }
    format!("The lyrics service is busy. Try again in {seconds} seconds")
}

pub fn bibles_download_failed(why: &str) -> String {
    if let Some(text) = say("bibles_download_failed", None, None, &[("why", text_arg(why))]) {
        return text;
    }
    if indonesian() {
        return format!("Alkitab tidak bisa diunduh: {why}");
    }
    format!("Could not download the Bible: {why}")
}

pub fn bible_added(name: &str, verses: usize) -> String {
    if let Some(text) = say("bible_added", None, Some(verses), &[("name", text_arg(name)), ("verses", text_arg(verses))]) {
        return text;
    }
    if indonesian() {
        return format!("{name} ditambah, {verses} ayat");
    }
    format!("{name} added, {verses} verses")
}

pub fn bibles_file_failed(why: &str) -> String {
    if let Some(text) = say("bibles_file_failed", None, None, &[("why", text_arg(why))]) {
        return text;
    }
    if indonesian() {
        return format!("Alkitab tidak bisa ditambah: {why}");
    }
    format!("Could not add the Bible: {why}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counted_nouns_have_a_singular() {
        assert_eq!(slides(1), "1 slide");
        assert_eq!(slides(8), "8 slides");
        assert_eq!(phones(0), "No phone connected");
        assert_eq!(import_done(1, 1), "Added 1 song and 1 Bible");
        assert_eq!(import_summary(0, 0, 0), "Everything in that folder is already here");
        assert_eq!(import_summary(3, 1, 0), "Added 3 and updated 1 song");
        assert_eq!(import_summary(0, 12, 2), "Updated 12 songs. Kept 2 songs you edited here");
        assert_eq!(import_summary(0, 0, 1), "Kept 1 song you edited here");
    }

    /// The house rules for words on screen: no dash or dot used as punctuation.
    #[test]
    fn no_dash_or_middot_in_any_string() {
        let source = include_str!("strings.rs");
        let (code, _) = source.split_once("#[cfg(test)]").unwrap_or((source, ""));
        for banned in ['\u{2014}', '\u{2013}', '\u{b7}'] {
            assert!(!code.contains(banned), "found {banned:?}");
        }
    }

    /// Spellings Indonesian readers catch at once. Each is a known-wrong form: the right one
    /// is beside it. A space after the prefix di- makes a verb read as a place.
    #[test]
    fn indonesian_keeps_its_own_spelling() {
        let source = include_str!("strings.rs");
        let (code, _) = source.split_once("#[cfg(test)]").unwrap_or((source, ""));
        let lower = code.to_lowercase();
        let wrong = [
            ("resiko", "risiko"), ("aktifitas", "aktivitas"), ("silahkan", "silakan"), ("merubah", "mengubah"), ("ijin", "izin"),
            ("di tampilkan", "ditampilkan"), ("di simpan", "disimpan"), ("di hapus", "dihapus"), ("di tambahkan", "ditambahkan"),
            ("dilayar", "di layar"), ("diatas", "di atas"), ("dibawah", "di bawah"), ("kedalam", "ke dalam"),
            ("proyektor", "layar"), ("al kitab", "Alkitab"), ("jema'at", "jemaat"), ("praktek", "praktik"), (" anda ", "no honorific"),
        ];
        for (bad, good) in wrong {
            assert!(!lower.contains(bad), "found {bad:?}; write {good:?}");
        }
    }

    /// The Indonesian in this file is the Indonesian that was reviewed (`docs/i18n/id.json`).
    /// A line changed here and not there, or there and not here, fails.
    /// The Indonesian table against the briefs it was written from: every brief has a text,
    /// no text is longer than its brief allows, and a variable the brief names is in the text.
    #[test]
    fn the_indonesian_table_answers_every_brief() {
        let table: serde_json::Value = serde_json::from_str(include_str!("../../../docs/i18n/id.json")).unwrap_or_default();
        let briefs: serde_json::Value = serde_json::from_str(include_str!("../../../docs/i18n/brief.json")).unwrap_or_default();
        let mut checked = 0;
        for brief in briefs.as_array().into_iter().flatten() {
            let key = brief["key"].as_str().unwrap_or_default();
            let entry = &table[key]["text"];
            assert!(!entry.is_null(), "{key} has a brief and no Indonesian text");
            // A text is one string, or one string per case or per plural form.
            let texts: Vec<&str> = match entry {
                serde_json::Value::String(text) => vec![text.as_str()],
                serde_json::Value::Object(forms) => forms.values().flat_map(|form| match form {
                    serde_json::Value::String(text) => vec![text.as_str()],
                    serde_json::Value::Object(inner) => inner.values().filter_map(|text| text.as_str()).collect(),
                    _ => Vec::new(),
                }).collect(),
                _ => Vec::new(),
            };
            assert!(!texts.is_empty(), "{key} has no Indonesian text to read");
            let budget = brief["budget"].as_u64().unwrap_or(u64::MAX) as usize;
            for text in &texts {
                // A text of several lines is measured by its longest line.
                // A text that carries a variable is as long as what fills it; the writer's own
                // check measured those with the brief's examples.
                let longest = text.lines().map(|line| line.chars().count()).max().unwrap_or(0);
                assert!(text.contains('{') || longest <= budget.max(1), "{key} is {longest} characters and its brief allows {budget}");
                assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}') && !text.contains('\u{b7}') && !text.contains('!'), "{key} carries a mark the interface never uses");
            }
            checked += 1;
        }
        assert!(checked > 500, "only {checked} briefs were checked");
    }

    /// Each row of the keys sheet is described by the brief written for those very keys. A
    /// brief names its keys in words, and a row that moved in the code without its brief put
    /// "take it out of the plan" beside Ctrl M (2026-11-11).
    #[test]
    fn every_key_row_has_the_brief_of_its_own_keys() {
        let briefs: serde_json::Value = serde_json::from_str(include_str!("../../../docs/i18n/brief.json")).unwrap_or_default();
        let screen_of = |key: &str| briefs.as_array().into_iter().flatten().find(|brief| brief["key"] == key).and_then(|brief| brief["screen"].as_str()).unwrap_or_default().to_string();
        let groups: [(&str, &[(&str, &str)]); 4] = [("KEYS_SHOWING_ROWS", &KEYS_SHOWING_ROWS_EN), ("KEYS_FINDING_ROWS", &KEYS_FINDING_ROWS_EN), ("KEYS_PLAN_ROWS", &KEYS_PLAN_ROWS_EN), ("KEYS_EDITING_ROWS", &KEYS_EDITING_ROWS_EN)];
        for (name, rows) in groups {
            for (at, (keys, _)) in rows.iter().enumerate() {
                let screen = screen_of(&format!("{name}[{at}].1"));
                let said = screen.rsplit("The keys at the left are:").next().unwrap_or_default();
                let named = keys.split(' ').filter(|token| token.len() > 1 && token.starts_with(char::is_uppercase) && !matches!(*token, "Up" | "Down" | "Left" | "Right" | "Space"));
                for token in named {
                    assert!(said.contains(token), "{name}[{at}] is drawn with {keys} and its brief describes: {said}");
                }
            }
        }
    }

    #[test]
    fn both_languages_answer() {
        set_indonesian(false);
        assert_eq!((BTN_SAVE(), slides(2), section_label(artosia_core::song::Tag::new(artosia_core::song::SectionKind::Chorus, 1))), ("Save", "2 slides".to_string(), "Chorus".to_string()));
        // Indonesian says what its table says, for a word, a counted noun and a section's name.
        let table: serde_json::Value = serde_json::from_str(include_str!("../../../docs/i18n/id.json")).unwrap_or_default();
        let of = |key: &str| table[key]["text"].as_str().unwrap_or_default().to_string();
        set_indonesian(true);
        assert_eq!(BTN_SAVE(), of("BTN_SAVE"));
        assert_eq!(slides(2), of("slides").replace("{count}", "2"));
        assert_eq!(section_label(artosia_core::song::Tag::new(artosia_core::song::SectionKind::Chorus, 1)), of("section.chorus"));
        assert_ne!(BTN_SAVE(), "Save");
        set_indonesian(false);
    }
}
