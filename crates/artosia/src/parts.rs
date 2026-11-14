//! LYRICSEDIT1: the song editor's part cards. The song stays marker text (`Song::lyrics`); a
//! card's words are edited on their own and written back through `write_lyrics`, so saving,
//! search, decks and the projector read the song as before. After FreeShow's text editor,
//! which turns its groups to text and back (`~/Shared/inspo/artosia-lyrics-editor/github`).

use artosia_core::song::{Section, SectionKind, Tag, parse_lyrics, write_lyrics};
use rune::TextEdit;

use crate::state::{App, Undo};
use crate::strings as t;

/// The card being written in.
pub struct PartEdit {
    /// The section's place among the written ones; for a fresh part, where it will go.
    pub index: usize,
    pub tag: Tag,
    pub text: TextEdit,
    /// Not in the words yet: a part just added, written on its first keystroke.
    pub fresh: bool,
    /// The song's words when this card last wrote them. Anything else that rewrites them (the
    /// proposed marks, Tidy, a language switch, Undo) leaves them different, and the card closes
    /// instead of writing back a stale part.
    pub source: String,
}

impl PartEdit {
    /// Whether the card still stands on the words as they are.
    pub fn current(&self, words: &str) -> bool {
        self.source == words
    }
}

/// The tag a new or retagged part of `kind` takes: the next free number, one for an intro or
/// an ending, which a song sings once.
fn next_tag(sections: &[Section], kind: SectionKind, skip: Option<usize>) -> Tag {
    let taken = sections.iter().enumerate().filter(|(at, section)| Some(*at) != skip && section.tag.kind == kind).map(|(_, section)| section.tag.number).max().unwrap_or(0);
    let once = matches!(kind, SectionKind::Intro | SectionKind::Ending);
    Tag::new(kind, if once { 1 } else { taken + 1 })
}

impl App {
    /// The editor's song as written, part by part, in the language in the field.
    pub fn written_parts(&self) -> Vec<Section> {
        self.editor.as_ref().map(|editor| parse_lyrics(&editor.lyrics.text)).unwrap_or_default()
    }

    fn write_parts(&mut self, sections: &[Section]) {
        if let Some(editor) = self.editor.as_mut() {
            editor.lyrics.replace_text(write_lyrics(sections));
        }
        // Once a new song has words of its own, nothing more is offered for it.
        self.dismiss_suggestions();
    }

    /// Opens a card for writing, with the caret after its last word.
    pub fn open_part(&mut self, index: usize) {
        let sections = self.written_parts();
        let Some(section) = sections.get(index) else { return };
        let mut text = TextEdit::new(section.body());
        text.focused = true;
        let end = text.text.len();
        text.set_selection(end, end);
        if let Some(editor) = self.editor.as_mut() {
            let source = editor.lyrics.text.clone();
            editor.part = Some(PartEdit { index, tag: section.tag, text, fresh: false, source });
            editor.part_menu = None;
        }
    }

    pub fn close_part(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.part = None;
            editor.part_menu = None;
        }
    }

    /// The open card's words changed: the song is written again with that part replaced. A part
    /// emptied out leaves the words and waits as a fresh one; a part that grew a marker line of
    /// its own splits the song, and the card closes on the new shape.
    pub fn part_changed(&mut self) {
        let Some(editor) = self.editor.as_mut() else { return };
        let Some(part) = editor.part.as_mut() else { return };
        if !part.current(&editor.lyrics.text) {
            editor.part = None;
            return;
        }
        let mut sections = parse_lyrics(&editor.lyrics.text);
        let body = Section::from_body(part.tag, &part.text.text);
        if part.fresh {
            if body.lines.is_empty() {
                return;
            }
            let at = part.index.min(sections.len());
            sections.insert(at, body);
            part.index = at;
            part.fresh = false;
        } else if part.index < sections.len() {
            if body.lines.is_empty() {
                sections.remove(part.index);
                part.fresh = true;
            } else {
                sections[part.index] = body;
            }
        }
        let count = sections.len();
        self.write_parts(&sections);
        if self.written_parts().len() != count {
            self.close_part();
        }
        self.keep_part_source();
    }

    /// The open card stands on the words as this editor just wrote them.
    fn keep_part_source(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            let words = editor.lyrics.text.clone();
            if let Some(part) = editor.part.as_mut() {
                part.source = words;
            }
        }
    }

    /// A part's kind picked from its chip: a verse takes the next free number.
    pub fn retag_part(&mut self, index: usize, kind: SectionKind) {
        let fresh = self.editor.as_ref().and_then(|editor| editor.part.as_ref()).is_some_and(|part| part.fresh && part.index == index);
        let mut sections = self.written_parts();
        let tag = next_tag(&sections, kind, Some(index));
        if !fresh {
            let Some(section) = sections.get_mut(index) else { return };
            section.tag = tag;
            self.write_parts(&sections);
        }
        if let Some(editor) = self.editor.as_mut() {
            editor.part_menu = None;
            if let Some(part) = editor.part.as_mut().filter(|part| part.index == index) {
                part.tag = tag;
            }
        }
        self.keep_part_source();
    }

    /// A new part of `kind` after the last one, opened for writing; the words gain it on the
    /// first keystroke.
    pub fn add_part(&mut self, kind: SectionKind) {
        let sections = self.written_parts();
        let tag = next_tag(&sections, kind, None);
        let mut text = TextEdit::default();
        text.focused = true;
        if let Some(editor) = self.editor.as_mut() {
            let source = editor.lyrics.text.clone();
            editor.part = Some(PartEdit { index: sections.len(), tag, text, fresh: true, source });
            editor.part_menu = None;
        }
    }

    /// Moves a part up or down among the written ones.
    pub fn move_part(&mut self, index: usize, by: isize) {
        let mut sections = self.written_parts();
        let to = index as isize + by;
        if index >= sections.len() || to < 0 || to as usize >= sections.len() {
            return;
        }
        sections.swap(index, to as usize);
        self.close_part();
        self.write_parts(&sections);
    }

    /// Splits a line too long for the screen in two at the space nearest its middle.
    pub fn split_line(&mut self, index: usize, line: usize) {
        let mut sections = self.written_parts();
        let Some(section) = sections.get_mut(index) else { return };
        let Some(words) = section.lines.get(line).cloned() else { return };
        let middle = words.len() / 2;
        let cut = words.char_indices().filter(|(_, c)| *c == ' ').map(|(at, _)| at).min_by_key(|at| at.abs_diff(middle));
        let Some(cut) = cut else { return };
        section.lines[line] = words[..cut].trim_end().to_string();
        section.lines.insert(line + 1, words[cut..].trim_start().to_string());
        // Slide breaks after the line move down with it.
        for at in section.breaks.iter_mut() {
            if *at > line {
                *at += 1;
            }
        }
        self.close_part();
        self.write_parts(&sections);
    }

    /// Takes a part out of the song; Undo in the toast puts the words back as they were.
    pub fn delete_part(&mut self, index: usize) {
        let mut sections = self.written_parts();
        if index >= sections.len() {
            return;
        }
        let before = self.editor.as_ref().map(|editor| editor.lyrics.text.clone()).unwrap_or_default();
        sections.remove(index);
        self.close_part();
        self.write_parts(&sections);
        self.say(t::PART_REMOVED(), Some(Undo::EditorWords(before)));
    }

    /// The words of the song in the editor as they were before a part was taken out.
    pub fn restore_editor_words(&mut self, words: String) {
        if let Some(editor) = self.editor.as_mut() {
            editor.part = None;
            editor.lyrics.replace_text(words);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with(words: &str) -> App {
        let mut app = App::practice();
        app.edit_new_song(artosia_core::song::Song::default());
        if let Some(editor) = app.editor.as_mut() {
            editor.lyrics.replace_text(words.to_string());
        }
        app
    }

    fn words(app: &App) -> String {
        app.editor.as_ref().map(|editor| editor.lyrics.text.clone()).unwrap_or_default()
    }

    #[test]
    fn writing_in_a_card_changes_only_its_part() {
        let mut app = editor_with("[1]\nFirst line\n[chorus]\nShine on\n");
        app.open_part(0);
        if let Some(part) = app.editor.as_mut().and_then(|editor| editor.part.as_mut()) {
            part.text.replace_text("New first line\n\nSecond slide".to_string());
        }
        app.part_changed();
        let parts = app.written_parts();
        assert_eq!(parts[0].lines, ["New first line", "Second slide"]);
        assert_eq!(parts[0].breaks, [1], "the empty line starts a slide");
        assert_eq!(parts[1].lines, ["Shine on"], "the chorus is untouched");
    }

    #[test]
    fn a_new_part_is_written_on_its_first_word_and_takes_the_next_number() {
        let mut app = editor_with("[1]\nFirst line\n");
        app.add_part(SectionKind::Verse);
        assert_eq!(app.written_parts().len(), 1, "nothing is written before a word");
        if let Some(part) = app.editor.as_mut().and_then(|editor| editor.part.as_mut()) {
            assert_eq!(part.tag, Tag::new(SectionKind::Verse, 2));
            part.text.replace_text("A second verse".to_string());
        }
        app.part_changed();
        assert_eq!(app.written_parts().len(), 2);
        assert!(words(&app).contains("A second verse"));
    }

    #[test]
    fn a_long_line_splits_at_the_space_nearest_its_middle_and_keeps_its_slide_breaks() {
        let mut app = editor_with("[1]\nOne two three four five six\n\nNext slide\n");
        app.split_line(0, 0);
        let part = &app.written_parts()[0];
        assert_eq!(part.lines, ["One two three", "four five six", "Next slide"]);
        assert_eq!(part.breaks, [2], "the slide break moved down with the line");
    }

    #[test]
    fn a_kind_is_picked_and_a_removed_part_comes_back_with_undo() {
        let mut app = editor_with("[1]\nOne\n[2]\nTwo\n");
        app.retag_part(1, SectionKind::Chorus);
        assert_eq!(app.written_parts()[1].tag.kind, SectionKind::Chorus);
        app.move_part(1, -1);
        assert_eq!(app.written_parts()[0].tag.kind, SectionKind::Chorus);
        let before = words(&app);
        app.delete_part(0);
        assert_eq!(app.written_parts().len(), 1);
        app.undo();
        assert_eq!(words(&app), before);
    }
}
