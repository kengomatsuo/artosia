//! A song in several languages, in the operator window (board card MULTI1): the languages a
//! church and a plan show together, the one that shows alone for a while, the editor's chips,
//! and the list a person picks a language from. What can be decided without a window is a plain
//! function here with a test; `ui/languages.rs` draws it.
//!
//! Nothing here writes a language's words: they exist only because they were typed, pasted or
//! imported.

use artosia_core::deck::{Deck, Languages};
use artosia_core::languages::{self, Texts};
use artosia_core::text::normalize;
use rune::TextEdit;

use crate::state::{App, Editor, Sheet, Undo};
use crate::strings as t;

/// Where the language picked from the list goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickFor {
    /// A new language of the song in the editor.
    AddToSong,
    /// The code of a language of the song that has none yet, at this place.
    NameInSong(usize),
    /// One more of the languages a slide shows.
    Slide,
}

/// The list of languages to pick one from, while it is open.
pub struct Pick {
    pub purpose: PickFor,
    /// What is typed to shorten the list.
    pub filter: TextEdit,
    /// Every language not already taken, by name in the window's language: code, name, and the
    /// name folded for matching.
    rows: Vec<(String, String, String)>,
}

impl Pick {
    /// `taken` are the codes the target already has; `in_songs` every code found in a song.
    pub fn new(purpose: PickFor, window: &str, country: &str, taken: &[String], in_songs: &[&str]) -> Pick {
        let rows = languages::ordered(window, country, taken, in_songs).into_iter().map(|(code, name)| (code, name.clone(), normalize(&name))).collect();
        Pick { purpose, filter: TextEdit::new(""), rows }
    }

    /// The languages the filter leaves: every word typed is somewhere in the name or the code.
    pub fn visible(&self) -> Vec<(&str, &str)> {
        let words: Vec<String> = normalize(&self.filter.text).split_whitespace().map(str::to_string).collect();
        self.rows
            .iter()
            .filter(|(code, _, hay)| words.iter().all(|word| hay.contains(word.as_str()) || code.to_lowercase().contains(word.as_str())))
            .map(|(code, name, _)| (code.as_str(), name.as_str()))
            .collect()
    }
}

/// Where the slide at `index` of `old` stands in `new`, a deck of the same song built again (a
/// language was switched, a look changed): the same section, and the same time round when it is
/// sung more than once. A section that is gone keeps the old place, held inside the new deck.
pub fn same_place(old: &Deck, index: usize, new: &Deck) -> usize {
    let last = new.slides.len().saturating_sub(1);
    let Some(slide) = old.slides.get(index) else { return index.min(last) };
    let Some(tag) = slide.tag else { return index.min(last) };
    let time = old.slides[..index].iter().filter(|before| before.tag == Some(tag)).count();
    let part = slide.part.map_or(0, |(at, _)| usize::from(at).saturating_sub(1));
    let same: Vec<usize> = new.slides.iter().enumerate().filter(|(_, other)| other.tag == Some(tag)).map(|(at, _)| at).collect();
    // A section split in two slides in one deck and one in the other is entered at its first piece.
    let pieces_of = |deck: &Deck, first: usize| deck.slides[first..].iter().take_while(|other| other.tag == Some(tag)).count().max(1);
    let (old_first, new_first) = (index - part, same.first().copied());
    let _ = (old_first, pieces_of(old, old_first));
    match same.get(time).copied().or(new_first) {
        Some(at) => at.min(last),
        None => index.min(last),
    }
}

impl Editor {
    /// The editor of a song's languages: the codes and the words, the first language in the field.
    pub fn languages_of(texts: Texts) -> (Vec<String>, Vec<String>) {
        texts.into_iter().unzip()
    }

    /// Every language as (code, words), with the one in the field as it is typed now.
    pub fn texts(&self) -> Texts {
        self.codes
            .iter()
            .enumerate()
            .map(|(at, code)| (code.clone(), if at == self.at { self.lyrics.text.replace("\r\n", "\n") } else { self.parked.get(at).cloned().unwrap_or_default() }))
            .collect()
    }

    fn park(&mut self) {
        if let Some(slot) = self.parked.get_mut(self.at) {
            *slot = self.lyrics.text.clone();
        }
    }

    /// Puts the language at `to` in the field; the one that was there waits with its words.
    pub fn show(&mut self, to: usize) {
        if to >= self.codes.len() || to == self.at {
            return;
        }
        self.park();
        self.at = to;
        self.lyrics.replace_text(self.parked[to].clone());
        self.lyrics.focused = true;
    }

    /// Adds a language with no words and shows it. False for a code the song already has.
    pub fn add_language(&mut self, code: &str) -> bool {
        if code.is_empty() || self.codes.iter().any(|have| have == code) {
            return false;
        }
        self.park();
        self.codes.push(code.to_string());
        self.parked.push(String::new());
        self.at = self.codes.len() - 1;
        self.lyrics.replace_text(String::new());
        self.lyrics.focused = true;
        true
    }

    /// Gives the language at `at` the code a person picked, when no other language has it.
    pub fn name_language(&mut self, at: usize, code: &str) -> bool {
        if code.is_empty() || at >= self.codes.len() || self.codes.iter().any(|have| have == code) {
            return false;
        }
        self.codes[at] = code.to_string();
        true
    }

    /// Takes the language at `at` out, with its words; the only language cannot go. What shows in
    /// the field stays the same language unless it was the one removed.
    pub fn remove_language(&mut self, at: usize) -> Option<(usize, (String, String))> {
        self.park();
        let mut texts: Texts = self.codes.iter().cloned().zip(self.parked.iter().cloned()).collect();
        let taken = languages::remove_text(&mut texts, at)?;
        (self.codes, self.parked) = texts.into_iter().unzip();
        let shown = if at == self.at { at.min(self.codes.len() - 1) } else if at < self.at { self.at - 1 } else { self.at };
        self.at = shown;
        self.lyrics.replace_text(self.parked[shown].clone());
        Some(taken)
    }

    /// Puts back what `remove_language` took and shows it.
    pub fn restore_language(&mut self, at: usize, entry: (String, String)) {
        self.park();
        let at = at.min(self.codes.len());
        self.codes.insert(at, entry.0);
        self.parked.insert(at, entry.1);
        self.at = at;
        self.lyrics.replace_text(self.parked[at].clone());
        self.lyrics.focused = true;
    }
}

impl App {
    /// Which languages a slide shows now: the plan's own list, else the church's, and a language
    /// showing alone when one was chosen in the Live pane.
    pub fn song_languages(&self) -> Languages {
        Languages { shown: languages::shown(self.plan.languages.as_deref(), &self.settings.song_languages).to_vec(), solo: self.only_language.clone() }
    }

    /// The shape of the screen the slides are split for: the projector's, once it has said.
    pub fn slide_shape(&self) -> (f32, f32) {
        self.projector_shape.map_or(artosia_slides::voices::WIDESCREEN, |(width, height)| (width as f32, height as f32))
    }

    pub fn has_own_languages(&self) -> bool {
        self.plan.languages.is_some()
    }

    /// The list the languages dialog edits: the open plan's own, else the church's.
    pub fn languages_list(&self) -> &[String] {
        languages::shown(self.plan.languages.as_deref(), &self.settings.song_languages)
    }

    /// Takes in what the projector reported about its screen. True when the slides had to be
    /// split again for another shape.
    pub fn track_projector_shape(&mut self) -> bool {
        let seen = self.link.projector_size();
        if seen == self.projector_shape {
            return false;
        }
        self.projector_shape = seen;
        self.refresh_decks();
        true
    }

    /// Writes a new list to the plan that has its own, else to the church's usual one, and builds
    /// every deck again. What is on the projector keeps its place.
    pub fn set_languages(&mut self, list: Vec<String>) {
        if self.plan.languages.is_some() {
            self.plan.languages = Some(list);
            self.touch_plan();
        } else {
            self.settings.song_languages = list;
            self.touch_settings();
        }
        self.refresh_decks();
    }

    /// Gives the open plan its own choice of languages, starting from the church's.
    pub fn use_own_languages(&mut self) {
        if self.plan.languages.is_none() {
            self.plan.languages = Some(self.settings.song_languages.clone());
            self.touch_plan();
            self.refresh_decks();
        }
    }

    /// Drops the plan's own choice. The toast offers it back.
    pub fn use_usual_languages(&mut self) {
        let Some(own) = self.plan.languages.take() else { return };
        self.touch_plan();
        self.refresh_decks();
        self.say(t::languages_own_dropped(&self.plan_title()), Some(Undo::PlanLanguages { plan: self.plan.id, list: own }));
    }

    /// Undo of `use_usual_languages`, when the plan is still the open one.
    pub fn restore_plan_languages(&mut self, plan: i64, list: Vec<String>) {
        if self.plan.id == plan {
            self.plan.languages = Some(list);
            self.touch_plan();
            self.refresh_decks();
        }
    }

    /// The languages dialog, from a plan's menu (`from_plan`) or from Settings.
    pub fn open_languages(&mut self, from_plan: bool) {
        self.languages_from_plan = from_plan;
        self.languages_off.clear();
        self.language_pick = None;
        self.close_palette();
        self.switcher.open = false;
        self.sheet = Sheet::Languages;
    }

    /// A switch of the dialog: a language on shows, off is kept in its row until the dialog closes.
    pub fn language_switch(&mut self, code: &str) {
        let mut list = self.languages_list().to_vec();
        match list.iter().position(|have| have == code) {
            Some(at) => {
                list.remove(at);
                if !self.languages_off.iter().any(|have| have == code) {
                    self.languages_off.push(code.to_string());
                }
            }
            None => {
                list.push(code.to_string());
                self.languages_off.retain(|have| have != code);
            }
        }
        self.set_languages(list);
    }

    /// Moves a shown language one place, up or down the list.
    pub fn language_move(&mut self, at: usize, up: bool) {
        let mut list = self.languages_list().to_vec();
        let to = if up { at.saturating_sub(1) } else { (at + 1).min(list.len().saturating_sub(1)) };
        languages::move_to(&mut list, at, to);
        if list != self.languages_list() {
            self.set_languages(list);
        }
    }

    /// Opens the list of languages for `purpose`, leaving out what the target has already.
    pub fn open_language_pick(&mut self, purpose: PickFor) {
        let taken: Vec<String> = match purpose {
            PickFor::Slide => self.languages_list().iter().chain(self.languages_off.iter()).cloned().collect(),
            PickFor::AddToSong | PickFor::NameInSong(_) => self.editor.as_ref().map(|editor| editor.codes.clone()).unwrap_or_default(),
        };
        let mut found: Vec<&str> = self.songs.iter().flat_map(|song| song.texts().map(|(code, _)| code)).filter(|code| !code.is_empty()).collect();
        found.sort_unstable();
        found.dedup();
        let mut pick = Pick::new(purpose, t::language_code(), &self.settings.country, &taken, &found);
        // The caret is in the filter as the list opens: a dialog opens on what the person came for.
        pick.filter.focused = true;
        self.language_pick = Some(pick);
    }

    pub fn close_language_pick(&mut self) {
        self.language_pick = None;
    }

    /// A language was picked from the list.
    pub fn pick_language_code(&mut self, code: &str) {
        let Some(pick) = self.language_pick.take() else { return };
        match pick.purpose {
            PickFor::Slide => {
                let mut list = self.languages_list().to_vec();
                if !list.iter().any(|have| have == code) {
                    list.push(code.to_string());
                    self.set_languages(list);
                }
            }
            PickFor::AddToSong => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.add_language(code);
                }
            }
            PickFor::NameInSong(at) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.name_language(at, code);
                    editor.show(at);
                }
            }
        }
    }

    /// A chip of the editor was pressed: it shows that language, or asks for the code of one nobody named.
    pub fn editor_chip(&mut self, at: usize) {
        let Some(editor) = self.editor.as_mut() else { return };
        editor.menu = None;
        if editor.codes.get(at).is_some_and(String::is_empty) {
            editor.show(at);
            self.open_language_pick(PickFor::NameInSong(at));
        } else {
            editor.show(at);
        }
    }

    /// The editor's menu on a chip removes its language, and a toast offers it back.
    pub fn editor_remove_language(&mut self, at: usize) {
        let Some(editor) = self.editor.as_mut() else { return };
        editor.menu = None;
        let name = editor.codes.get(at).map(|code| if code.is_empty() { t::LANGUAGE_PICK_TITLE().to_string() } else { languages::name_in_sentence(code, t::language_code()) }).unwrap_or_default();
        if let Some((at, entry)) = editor.remove_language(at) {
            self.say(t::language_removed(&name), Some(Undo::EditorLanguage { at, entry }));
        }
    }

    pub fn restore_editor_language(&mut self, at: usize, entry: (String, String)) {
        if let Some(editor) = self.editor.as_mut() {
            editor.restore_language(at, entry);
        }
    }

    /// The languages the live song has to show together, in the order they are drawn, as (code,
    /// name, place in the list that picks its colour). Fewer than two for a song of one language
    /// and for anything that is not a song: there is then nothing to choose between.
    pub fn live_languages(&self) -> Vec<(String, String, usize)> {
        let Some(artosia_core::deck::DeckSource::Song(id)) = self.live.as_ref().map(|live| &live.deck.source) else { return Vec::new() };
        let Some(song) = self.song_position(*id).map(|at| &self.songs[at]) else { return Vec::new() };
        let all = Languages { shown: self.song_languages().shown, solo: None };
        let texts: Vec<(&str, &str)> = song.texts().collect();
        let window = t::language_code();
        let found: Vec<(String, String, usize)> = all.pick(song).into_iter().map(|(slot, at)| (texts[at].0.to_string(), languages::name(texts[at].0, window), usize::from(slot))).collect();
        if found.len() < 2 { Vec::new() } else { found }
    }

    /// Shows one language alone, or all again when it is the one already alone. Never saved.
    pub fn toggle_only_language(&mut self, code: &str) {
        self.only_language = languages::toggle_solo(self.only_language.as_deref(), code);
        self.refresh_decks();
    }

    /// All the languages together again.
    pub fn show_all_languages(&mut self) {
        if self.only_language.take().is_some() {
            self.refresh_decks();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use artosia_core::deck::{Slide, SlideKind};
    use artosia_core::song::{SectionKind, Song, Tag, Translation};

    fn editor(texts: &[(&str, &str)]) -> Editor {
        let texts: Texts = texts.iter().map(|(code, words)| (code.to_string(), words.to_string())).collect();
        let first = texts.first().map(|(_, words)| words.clone()).unwrap_or_default();
        let (codes, parked) = Editor::languages_of(texts);
        Editor { lyrics: TextEdit::new(first), codes, parked, ..Editor::default() }
    }

    #[test]
    fn a_chip_shows_its_language_in_the_same_field_and_the_words_wait_for_it() {
        let mut editor = editor(&[("id", "[1]\nsatu\n"), ("en", "[1]\none\n")]);
        editor.lyrics.replace_text("[1]\nsatu dua\n".to_string());
        editor.show(1);
        assert_eq!((editor.at, editor.lyrics.text.as_str()), (1, "[1]\none\n"));
        editor.lyrics.replace_text("[1]\none two\n".to_string());
        editor.show(0);
        assert_eq!(editor.lyrics.text, "[1]\nsatu dua\n");
        assert_eq!(editor.texts(), [("id".to_string(), "[1]\nsatu dua\n".to_string()), ("en".to_string(), "[1]\none two\n".to_string())]);
    }

    #[test]
    fn a_new_language_has_an_empty_field_and_the_same_code_is_not_added_twice() {
        let mut editor = editor(&[("id", "[1]\nsatu\n")]);
        assert!(editor.add_language("en"));
        assert_eq!((editor.at, editor.lyrics.text.as_str(), editor.codes.len()), (1, "", 2));
        assert!(!editor.add_language("en") && !editor.add_language(""));
        // The words of the first language were parked, not lost.
        assert_eq!(editor.texts()[0].1, "[1]\nsatu\n");
        // A language nobody named takes a code once, and not one the song has.
        let mut old = self::editor(&[("", "[1]\nsatu\n")]);
        assert!(!old.name_language(0, ""));
        assert!(old.name_language(0, "id") && old.codes[0] == "id");
        old.add_language("en");
        assert!(!old.name_language(1, "id"));
    }

    #[test]
    fn removing_a_language_and_undoing_it_gives_back_the_words_where_they_were() {
        let mut editor = editor(&[("id", "[1]\nsatu\n"), ("en", "[1]\none\n"), ("zh-Hans", "[1]\nyi\n")]);
        editor.show(2);
        // A language before the one in the field leaves the field on the same language.
        let taken = editor.remove_language(0).expect("removed");
        assert_eq!((editor.codes.clone(), editor.at, editor.lyrics.text.as_str()), (vec!["en".to_string(), "zh-Hans".to_string()], 1, "[1]\nyi\n"));
        editor.restore_language(taken.0, taken.1);
        assert_eq!((editor.codes.clone(), editor.at), (vec!["id".to_string(), "en".to_string(), "zh-Hans".to_string()], 0));
        assert_eq!(editor.lyrics.text, "[1]\nsatu\n");
        // The language in the field is removed: the one that takes its place shows.
        editor.show(1);
        let taken = editor.remove_language(1).expect("removed");
        assert_eq!((editor.at, editor.lyrics.text.as_str()), (1, "[1]\nyi\n"));
        editor.restore_language(taken.0, taken.1);
        assert_eq!(editor.texts().len(), 3);
        // The only language of a song stays.
        let mut one = self::editor(&[("id", "x")]);
        assert!(one.remove_language(0).is_none());
    }

    #[test]
    fn the_picker_leaves_out_what_is_taken_and_filters_by_name_in_the_window_s_language() {
        let pick = Pick::new(PickFor::AddToSong, "en", "", &["id".to_string()], &["xx-Test"]);
        let all = pick.visible();
        assert!(all.iter().all(|(code, _)| *code != "id") && all.iter().any(|(code, _)| *code == "xx-Test"));
        let mut typed = Pick::new(PickFor::AddToSong, "en", "", &[], &[]);
        typed.filter = TextEdit::new("mandar");
        assert!(typed.visible().iter().all(|(_, name)| name.to_lowercase().contains("mandar")));
        typed.filter = TextEdit::new("indones");
        assert_eq!(typed.visible().first().map(|(code, _)| *code), Some("id"));
        // Typing a code finds its language as well as the name.
        typed.filter = TextEdit::new("zh-hans");
        assert!(typed.visible().iter().any(|(code, _)| *code == "zh-Hans"));
        let id = Pick::new(PickFor::Slide, "id", "", &[], &[]);
        let mut id = id;
        id.filter = TextEdit::new("jepang");
        assert_eq!(id.visible().first().map(|(code, _)| *code), Some("ja"));
    }

    fn deck_of(tags: &[(SectionKind, u8)]) -> Deck {
        Deck { slides: tags.iter().map(|(kind, number)| Slide { kind: SlideKind::Lyrics, tag: Some(Tag::new(*kind, *number)), ..Slide::default() }).collect(), title: String::new(), subtitle: String::new(), source: artosia_core::deck::DeckSource::Song(0) }
    }

    #[test]
    fn a_deck_built_again_keeps_the_place_of_the_slide_on_the_projector() {
        use SectionKind::{Chorus, Verse};
        let old = deck_of(&[(Verse, 1), (Chorus, 1), (Verse, 2), (Chorus, 1)]);
        // Split in two slides a section that was one: the second chorus is still the second chorus.
        let new = deck_of(&[(Verse, 1), (Verse, 1), (Chorus, 1), (Verse, 2), (Chorus, 1)]);
        assert_eq!(same_place(&old, 3, &new), 4);
        assert_eq!(same_place(&old, 1, &new), 2);
        assert_eq!(same_place(&old, 2, &new), 3);
        // A section that is gone holds the old index inside the new deck.
        let gone = deck_of(&[(Verse, 1)]);
        assert_eq!(same_place(&old, 3, &gone), 0);
        assert_eq!(same_place(&old, 9, &new), 4);
        assert_eq!(same_place(&old, 0, &deck_of(&[])), 0);
    }

    #[test]
    fn the_plan_s_own_list_is_used_and_solo_changes_nothing_saved() {
        let mut app = App::practice();
        app.settings.song_languages = vec!["id".into(), "en".into()];
        assert_eq!(app.song_languages().shown, ["id", "en"]);
        app.use_own_languages();
        app.set_languages(vec!["en".into()]);
        assert_eq!((app.song_languages().shown, app.settings.song_languages.clone()), (vec!["en".to_string()], vec!["id".to_string(), "en".to_string()]));
        app.toggle_only_language("en");
        assert_eq!(app.song_languages().solo.as_deref(), Some("en"));
        app.toggle_only_language("en");
        assert_eq!(app.song_languages().solo, None);
        // Dropping the plan's own list offers it back.
        app.use_usual_languages();
        assert_eq!(app.song_languages().shown, ["id", "en"]);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert_eq!(app.song_languages().shown, ["en"]);
    }

    #[test]
    fn the_practice_song_in_three_languages_shows_all_three_blocks_and_solo_narrows_it() {
        let mut app = App::practice();
        app.settings.song_languages = vec!["id".into(), "en".into(), "zh-Hans".into()];
        let at = app.songs.iter().position(|song| song.translations.len() == 2).expect("the practice library has it");
        let deck = app.deck_of_song(&app.songs[at].clone());
        assert!(deck.slides.iter().all(|slide| slide.more.len() == 2));
        app.open_song(at, None);
        app.show(0);
        assert_eq!(app.live_languages().iter().map(|(code, _, _)| code.as_str()).collect::<Vec<_>>(), ["id", "en", "zh-Hans"]);
        app.toggle_only_language("en");
        assert_eq!(app.live.as_ref().map(|live| live.deck.slides[live.index].more.len()), Some(0));
        assert_eq!(app.live_languages().len(), 3, "the chips stay so a second press brings the others back");
        app.toggle_only_language("en");
        assert_eq!(app.live.as_ref().map(|live| live.deck.slides[live.index].more.len()), Some(2));
    }

    #[test]
    fn a_church_that_never_chose_sees_its_old_second_text_and_nothing_else() {
        let mut app = App::practice();
        app.settings.song_languages.clear();
        let song = Song { title: "Old".into(), lyrics: "[1]\nsatu\n".into(), translations: vec![Translation { lang: String::new(), text: "[1]\nuno\n".into() }], ..Song::default() };
        assert!(app.settings.song_languages.is_empty());
        let deck = app.deck_of_song(&song);
        assert_eq!((deck.slides[0].text.as_str(), deck.slides[0].more.len()), ("satu", 1));
    }
}
