# The song editor as part cards (LYRICSEDIT1, owner picked direction A on 2026-11-14)

Research: `~/Shared/inspo/artosia-lyrics-editor/` (README, directions.md; 88 files). The picture
the owner chose is scene `lyrics-a-p1` (`crates/artosia/src/ui/mock_lyrics.rs`, `cards`): the mock
code is the spec.

```
SOURCES  mobbin/01-elevenlabs-music-section-blocks.png (part cards, strip), 02 (named empty part),
         github/code/FreeShow--formatTextEditor.ts + getTextEditor.ts (text <-> parts both ways),
         mobbin/07-descript-add-chapters-proposed.png (proposed parts), real-products/20 (part colour
         carried to slides), 61-worshiptools (order built by picking)
BASE     "part card": one section of a song shown and edited as a unit; the editor is its only caller
BOUNDARY the song stays marker text (`Song::lyrics`); cards read `parse_lyrics` and write back with
         `write_lyrics`; save, search, decks and the projector are untouched
SCREEN   song editor (new song, edit song), reached from Preview's New song tile, a plan row's menu,
         search's New song and Edit (Ctrl E)
TREE     head (title, subtitle "7 parts, 11 slides", Song details, Cancel, Save)
         · details panel (author, copyright, CCLI, key) only while Song details is on
         · language chips row (existing) · marks strip (existing proposal, unmarked paste)
         · cards: grip, kind chip (opens kind menu), slide count, ⋯ menu (up, down, delete),
           the part's lines with a dashed "slide N" rule between slides, the part's slide thumbnails
         · New part tile with five kind chips · order strip pinned at the bottom
KIT      cards are the app's own el()/row()/col(); colours `group_color(tag_group(tag))`, tint/ink
         through `theme`; thumbnails `ui::thumb`; buttons `widgets::btn`
DATA     editor.lyrics (marker text of language `at`) -> parse_lyrics -> cards, every frame
         editor.part: Option<(index, TextEdit)> holds the open card's body; each change rewrites
         editor.lyrics through write_lyrics with that section replaced
         anything else that rewrites lyrics (marks, tidy, language switch, undo) closes the open card
STATE    open card and kind menu are Editor fields; nothing else is stored that can be derived
ACTION   press a card's lines: open_part(i) · type: part_changed rewrites lyrics
         press kind chip: menu; pick: retag_part(i, kind) (a verse takes the next free number)
         ⋯ up/down/delete: move_part, delete_part (Undo in a toast, the editor before it)
         New part chip: add_part(kind), then open it · Esc in a card: close the card, again: editor
         Ctrl Down/Up in a card: next/previous card
         order strip chip: order_remove(at); + : menu of parts, order_add(tag)
EDGES    empty song: one "Verse 1" card waiting, written on the first keystroke · unmarked paste:
         stanzas show as verses and the marks strip proposes · a section with no lines · 30 parts
         · second language: cards show the language in the field; the other shows beside, read-only
GUIDE    Songs lesson keys: editor-words -> the open card's text, editor-parts -> New part tile,
         editor-save unchanged
```

Done when: the editor shows parts as cards with their slides, a part's kind is picked from its chip,
parts are added from the New part tile, the sung order is a strip built by picking, and no typed
order field or whole-song text box remains. Verify: `cargo test -p artosia editor`, the editor scenes
rendered in English and Indonesian, `tools/check-lessons.py songs`.
