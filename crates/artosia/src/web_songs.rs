//! The "Songs from the internet" dialog (WEBDLG1): one field, the results of Gema SABDA's title
//! index and LRCLIB, and the picked song's words parsed into Artosia's own preview. A song comes
//! in only when the person presses Import; the page is never shown, only what was parsed from it.
//! The network work is two lanes: the Gema index in the background, and one call at a time
//! (a search or one pick's words) for the dialog, started after a pause in typing.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use artosia_core::song::{Section, Song, song_from_paste};
use artosia_lyrics::{Browse, Fetched, Found, GemaIndex, PASTED};
use rune::TextEdit;

use crate::state::{App, Arrived, Tab};
use crate::strings as t;

/// The pause after the last key before the web is asked: the song editor's.
const PAUSE: f64 = 0.4;
/// Where the Gema index is kept, in the library folder.
const INDEX_FILE: &str = "gema-index.json";
/// Longest word list sent to the person's browser, in characters.
const MOST_BROWSER_CHARS: usize = 200;

/// The number of the latest call, so an answer for a dialog or a pick that has gone is known.
static ROUNDS: AtomicU64 = AtomicU64::new(0);

/// The picked song's words.
pub enum Words {
    Nothing,
    Loading,
    Ready(Box<Ready>),
    Failed(String),
}

/// A page or record parsed, with the song it would make.
pub struct Ready {
    pub fetched: Fetched,
    pub song: Song,
    /// The song's sections, read once when the words arrive and never again per frame.
    pub sections: Vec<Section>,
}

impl Ready {
    fn of(fetched: Fetched) -> Box<Ready> {
        let song = song_of(&fetched);
        Box::new(Ready { sections: song.sections(), song, fetched })
    }
}

/// The dialog's state; nothing in it is saved.
pub struct WebSongs {
    pub query: TextEdit,
    pub found: Vec<Found>,
    /// Why LRCLIB gave nothing, in the window's language.
    pub problem: Option<String>,
    pub picked: Option<usize>,
    pub words: Words,
    /// Typed words are waiting for their answer.
    pub searching: bool,
    /// When the pause ends and the web is asked.
    pub ask_at: Option<f64>,
    /// The pick whose words wait for the lane to be free.
    want: Option<Found>,
    /// The number of the call out now; only one is.
    inflight: Option<u64>,
}

/// The Gema SABDA index of titles, shared by every opening of the dialog.
#[derive(Default)]
pub struct Gema {
    pub index: Option<Arc<GemaIndex>>,
    pub reading: bool,
    pub failed: bool,
    started: bool,
}

impl WebSongs {
    pub fn new() -> WebSongs {
        let mut query = TextEdit::new("");
        query.focused = true;
        WebSongs { query, found: Vec::new(), problem: None, picked: None, words: Words::Nothing, searching: false, ask_at: None, want: None, inflight: None }
    }

    /// The text typed, trimmed.
    pub fn typed(&self) -> &str {
        self.query.text.trim()
    }

    /// Whether Import can be pressed now.
    pub fn ready(&self) -> bool {
        matches!(self.words, Words::Ready(_))
    }

    /// The picked result, if any.
    pub fn pick(&self) -> Option<&Found> {
        self.picked.and_then(|at| self.found.get(at))
    }
}

/// The song a page or record makes: the first line is its title, as in a pasted song, and the
/// artist is its author. The preview and Import both read this, so what is shown is what is made.
pub fn song_of(fetched: &Fetched) -> Song {
    let title = fetched.title.trim();
    let text = if title.is_empty() { fetched.text.clone() } else { format!("{title}\n\n{}", fetched.text) };
    let mut song = song_from_paste(&text);
    song.author = fetched.artist.trim().to_string();
    song
}

/// A Google search for the typed words and "lirik", as an address for the person's browser.
pub fn browser_search(words: &str) -> String {
    let words: String = words.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MOST_BROWSER_CHARS).collect();
    let mut encoded = String::new();
    for byte in format!("{words} lirik").bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            b' ' => encoded.push('+'),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    format!("https://www.google.com/search?q={encoded}")
}

/// A day as the index keeps it, `YYYY-MM-DD`.
fn day_string(date: artosia_core::calendar::Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

fn is_address(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

impl App {
    fn wake_app(inbox: &Arc<std::sync::Mutex<Vec<Arrived>>>, waker: &Option<rune::Waker>, item: Arrived) {
        if let Ok(mut inbox) = inbox.lock() {
            inbox.push(item);
        }
        if let Some(wake) = waker {
            wake();
        }
    }

    /// Where the Gema index is kept, beside the library's media.
    fn index_path(&self) -> std::path::PathBuf {
        let base = self.folder.clone().unwrap_or_else(|| std::env::temp_dir().join("artosia-practice"));
        base.join(INDEX_FILE)
    }

    /// Opens the dialog from the Add menu, the Start screen, the search or the song editor.
    pub fn open_web_songs(&mut self) {
        self.close_palette();
        self.sheet = crate::state::Sheet::None;
        self.web = Some(WebSongs::new());
        if !self.gema.started || self.gema.failed {
            self.start_gema_index();
        }
    }

    pub fn close_web_songs(&mut self) {
        self.web = None;
    }

    /// Reads the kept index and, when it is missing or a week old, the lists again, on a worker.
    fn start_gema_index(&mut self) {
        self.gema.started = true;
        self.gema.reading = true;
        self.gema.failed = false;
        let (inbox, waker, path, today) = (self.inbox.clone(), self.waker.clone(), self.index_path(), day_string(self.civil_today()));
        std::thread::spawn(move || {
            let kept = GemaIndex::load(&path);
            let stale = kept.as_ref().is_none_or(|kept| kept.is_stale(&today));
            if let Some(kept) = kept {
                App::wake_app(&inbox, &waker, Arrived::WebIndex { index: Some(kept), done: !stale });
            }
            if stale {
                let fresh = GemaIndex::read(&today).ok();
                if let Some(fresh) = &fresh {
                    // A copy that cannot be kept is read again next time.
                    let _ = fresh.save(&path);
                }
                App::wake_app(&inbox, &waker, Arrived::WebIndex { index: fresh, done: true });
            }
        });
    }

    pub fn web_index_arrived(&mut self, index: Option<GemaIndex>, done: bool) {
        let fresh = index.is_some();
        if let Some(index) = index {
            self.gema.index = Some(Arc::new(index));
        }
        if done {
            self.gema.reading = false;
            self.gema.failed = !fresh && self.gema.index.is_none();
        }
        // Titles that were not there when the person typed may match now.
        let clock = self.clock;
        if let Some(web) = self.web.as_mut().filter(|web| fresh && !web.typed().is_empty() && !is_address(web.typed())) {
            web.searching = true;
            web.ask_at = Some(clock);
        }
    }

    /// The field changed: the web is asked after a pause in typing, and at once for an empty field.
    pub fn web_typed(&mut self) {
        let clock = self.clock;
        let Some(web) = self.web.as_mut() else { return };
        web.searching = !web.typed().is_empty();
        web.ask_at = Some(if web.typed().is_empty() { clock } else { clock + PAUSE });
    }

    /// Enter in the field: no more waiting for the pause.
    pub fn web_ask_now(&mut self) {
        let clock = self.clock;
        if let Some(web) = self.web.as_mut().filter(|web| !web.typed().is_empty()) {
            web.searching = true;
            web.ask_at = Some(clock);
        }
    }

    /// Starts the next call when none is out: the words of a pick first, then the search.
    /// Lowers `wake` to the time the pause ends.
    pub fn pump_web(&mut self, wake: &mut f64) {
        let clock = self.clock;
        let Some(web) = self.web.as_mut().filter(|web| web.inflight.is_none()) else { return };
        if let Some(found) = web.want.take() {
            let round = ROUNDS.fetch_add(1, Ordering::Relaxed) + 1;
            web.inflight = Some(round);
            let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
            std::thread::spawn(move || {
                let fetched = artosia_lyrics::words(&found).map_err(|error| crate::why::lyrics(&error));
                App::wake_app(&inbox, &waker, Arrived::WebWords(round, found.url, fetched));
            });
            return;
        }
        let Some(at) = web.ask_at else { return };
        if clock < at {
            *wake = wake.min((at - clock).max(0.05));
            return;
        }
        web.ask_at = None;
        let query = web.typed().to_string();
        if query.is_empty() {
            (web.found, web.problem, web.picked, web.words, web.searching, web.want) = (Vec::new(), None, None, Words::Nothing, false, None);
            return;
        }
        let round = ROUNDS.fetch_add(1, Ordering::Relaxed) + 1;
        web.inflight = Some(round);
        let (inbox, waker, index) = (self.inbox.clone(), self.waker.clone(), self.gema.index.clone());
        std::thread::spawn(move || {
            let browse = artosia_lyrics::browse(&query, index.as_deref());
            App::wake_app(&inbox, &waker, Arrived::WebFound(round, query, browse));
        });
    }

    /// The results for what was typed. An answer for words that have since changed is dropped.
    pub fn web_found_arrived(&mut self, round: u64, query: String, browse: Browse) {
        let Some(web) = self.web.as_mut().filter(|web| web.inflight == Some(round)) else { return };
        web.inflight = None;
        if web.typed() != query {
            return;
        }
        web.searching = web.ask_at.is_some();
        web.problem = browse.lrclib_problem.as_ref().map(crate::why::lyrics);
        let kept = web.pick().map(|found| found.url.clone());
        web.found = browse.found;
        web.picked = kept.and_then(|url| web.found.iter().position(|found| found.url == url));
        if web.picked.is_none() {
            web.words = Words::Nothing;
            // A pasted address, or the one song there is, is read at once: nothing else could be meant.
            if web.found.len() == 1 || web.found.first().is_some_and(|found| found.source == PASTED) {
                self.web_choose(0);
            }
        }
    }

    /// The words of the pick. A pick that is no longer the one in the list is dropped.
    pub fn web_words_arrived(&mut self, round: u64, url: String, fetched: Result<Fetched, String>) {
        let Some(web) = self.web.as_mut().filter(|web| web.inflight == Some(round)) else { return };
        web.inflight = None;
        if web.pick().is_none_or(|found| found.url != url) {
            return;
        }
        web.words = match fetched {
            Ok(fetched) => Words::Ready(Ready::of(fetched)),
            Err(why) => Words::Failed(why),
        };
    }

    /// Picks a result and asks for its words.
    pub fn web_choose(&mut self, at: usize) {
        let Some(web) = self.web.as_mut() else { return };
        let Some(found) = web.found.get(at).cloned() else { return };
        if web.picked == Some(at) && !matches!(web.words, Words::Failed(_)) {
            return;
        }
        web.picked = Some(at);
        web.words = Words::Loading;
        web.want = Some(found);
    }

    /// Down and Up in the field walk the results.
    pub fn web_step(&mut self, by: isize) {
        let Some(web) = self.web.as_ref().filter(|web| !web.found.is_empty()) else { return };
        let count = web.found.len() as isize;
        let at = match web.picked {
            Some(at) => (at as isize + by).rem_euclid(count),
            None if by < 0 => count - 1,
            None => 0,
        };
        self.web_choose(at as usize);
    }

    /// Import: the parsed song becomes a song of the library, opens in Preview, and the dialog closes.
    pub fn web_import(&mut self) {
        let Some(mut song) = self.web.as_ref().and_then(|web| match &web.words {
            Words::Ready(ready) => Some(ready.song.clone()),
            _ => None,
        }) else {
            return;
        };
        if let Err(error) = self.store.save_song(&mut song) {
            self.say(t::could_not_save(&error.to_string()), None);
            return;
        }
        self.web = None;
        // A new song nobody has typed into is not worth keeping over Preview.
        if self.editor.as_ref().is_some_and(|editor| editor.id == 0 && editor.title.text.trim().is_empty() && editor.lyrics.text.trim().is_empty()) {
            self.editor = None;
        }
        self.reload_songs();
        self.tab = Tab::Plan;
        if let Some(at) = self.song_position(song.id) {
            self.open_song(at, None);
        }
        self.say(t::SONG_SAVED(), None);
    }

    /// Fills the dialog for a headless render, with no network: "results" (Gema titles and one
    /// parsed song), "pasted" (an address that was parsed) or "offline" (LRCLIB failing, Gema's
    /// results still shown). The index, titles and words are all made up.
    pub fn demo_web_songs(&mut self, kind: &str) {
        let song = |id: &str, title: &str, artist: &str, source: &str, url: &str| Found { id: id.into(), title: title.into(), artist: artist.into(), source: source.into(), url: url.into() };
        let gema = "Gema SABDA";
        let sample = Fetched {
            title: "Lantern Song".into(),
            artist: "Sample Author".into(),
            text: "Carry the lantern through the night\nKeep the small flame burning bright\n\nShine, shine on the road ahead\nShine till the morning comes\n\nWalk beside me down the hill\nHold my hand and hold me still\n\nShine, shine on the road ahead\nShine till the morning comes\n".into(),
            source: gema.into(),
            url: "https://gema.example.test/lantern_song".into(),
        };
        let mut web = WebSongs::new();
        let ready = |fetched: Fetched| Words::Ready(Ready::of(fetched));
        match kind {
            "pasted" => {
                web.query.replace_text("https://lyrics.example.test/lantern-song");
                web.found = vec![song("p", "https://lyrics.example.test/lantern-song", "", PASTED, "https://lyrics.example.test/lantern-song")];
                web.picked = Some(0);
                web.words = ready(Fetched { source: "Lyrics Example".into(), url: "https://lyrics.example.test/lantern-song".into(), ..sample });
            }
            "offline" => {
                web.query.replace_text("lantern");
                web.found = vec![song("1", "Lantern Song", "", gema, "https://gema.example.test/lantern_song"), song("2", "Lantern in the Valley", "", gema, "https://gema.example.test/lantern_valley")];
                web.problem = Some(crate::why::lyrics(&artosia_lyrics::LyricsError::Network("down".into())));
                web.picked = Some(0);
                web.words = ready(sample);
            }
            _ => {
                web.query.replace_text("lantern");
                web.found = vec![
                    song("1", "Lantern Song", "", gema, "https://gema.example.test/lantern_song"),
                    song("2", "Lantern in the Valley", "", gema, "https://gema.example.test/lantern_valley"),
                    song("3", "Lantern Song", "Sample Band", "LRCLIB", "https://lrclib.example.test/api/get/3"),
                    song("4", "Hold the Lantern High", "Sample Choir", "LRCLIB", "https://lrclib.example.test/api/get/4"),
                ];
                web.picked = Some(0);
                web.words = ready(sample);
            }
        }
        self.gema.index = Some(Arc::new(GemaIndex::default()));
        self.gema.started = true;
        self.web = Some(web);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetched(title: &str, text: &str) -> Fetched {
        Fetched { title: title.into(), artist: " Sample Author ".into(), text: text.into(), source: "Gema SABDA".into(), url: "https://gema.example.test/a".into() }
    }

    #[test]
    fn a_page_makes_the_song_the_preview_shows() {
        let song = song_of(&fetched("Lantern Song", "One\nTwo\n\nThree\nFour\n"));
        assert_eq!(song.title, "Lantern Song");
        assert_eq!(song.author, "Sample Author");
        assert_eq!(song.sections().len(), 2);
    }

    #[test]
    fn a_one_line_first_stanza_is_not_taken_for_the_title() {
        let song = song_of(&fetched("Lantern Song", "Alone\n\nThree\nFour\n"));
        assert_eq!(song.title, "Lantern Song");
        assert_eq!(song.sections().len(), 2);
        assert_eq!(song_of(&fetched("", "One\nTwo\n\nThree")).title, "One");
    }

    #[test]
    fn the_browser_is_sent_the_words_and_lirik_as_one_encoded_query() {
        assert_eq!(browser_search("  kasih  setia "), "https://www.google.com/search?q=kasih+setia+lirik");
        assert_eq!(browser_search("Rindu & Doa"), "https://www.google.com/search?q=Rindu+%26+Doa+lirik");
        assert_eq!(browser_search("é"), "https://www.google.com/search?q=%C3%A9+lirik");
        assert!(browser_search(&"a".repeat(1000)).len() < 300);
    }

    #[test]
    fn a_day_is_written_as_the_index_reads_it() {
        assert_eq!(day_string(artosia_core::calendar::Date { year: 2026, month: 3, day: 9 }), "2026-03-09");
    }

    #[test]
    fn only_http_and_https_are_addresses() {
        assert!(is_address("HTTPS://example.test/a"));
        assert!(!is_address("ftp://example.test"));
        assert!(!is_address("kj 4"));
    }

    #[test]
    fn a_long_song_is_parsed_in_well_under_a_frame() {
        let stanza = "A line of made-up words for timing
Another line of made-up words
A third line of made-up words
A fourth line of made-up words

";
        let page = fetched("Long Song", &stanza.repeat(15));
        let start = std::time::Instant::now();
        let ready = Ready::of(page);
        let took = start.elapsed();
        eprintln!("web song of 60 lines parsed in {took:?}");
        assert_eq!(ready.sections.len(), 15);
        assert!(took < std::time::Duration::from_millis(5), "{took:?}");
    }
}
