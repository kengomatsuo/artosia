//! The song editor as part cards (LYRICSEDIT1, the owner's pick of three directions after
//! `~/Shared/inspo/artosia-lyrics-editor`): each part of the song is a card in its colour with
//! its words and its slides, its kind picked from its chip, a New part tile under the cards, and
//! the sung order as a strip of the parts at the foot, built by picking.
//!
//! The song stays marker text (`[1]`, `[chorus]`) because that is what pastes in from anywhere
//! and what EasiSlides wrote; `crate::parts` turns a card's words into its part and back. A new
//! song's title offers whole songs to fill in.

use artosia_core::deck::tag_group;
use artosia_core::marks::{Basis, Proposal, Stanza};
use artosia_core::song::{Section, SectionKind, Tag};
use artosia_slides::stage::group_color;
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, TextEdit, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar, TextArea, TextInput};

use crate::state::{App, Editor, Suggestion};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, keycap, section_label};

/// The width of a slide inside a part card.
const THUMB: f32 = 112.0;

/// The sections a button or Ctrl with a digit starts, in that order.
const SECTION_KEYS: [SectionKind; 5] = [SectionKind::Verse, SectionKind::Chorus, SectionKind::PreChorus, SectionKind::Bridge, SectionKind::Ending];

fn section_name(kind: SectionKind) -> &'static str {
    match kind {
        SectionKind::Chorus => t::MARK_CHORUS(),
        SectionKind::PreChorus => t::MARK_PRECHORUS(),
        SectionKind::Bridge => t::MARK_BRIDGE(),
        SectionKind::Ending => t::MARK_ENDING(),
        _ => t::MARK_VERSE(),
    }
}

/// A kind's own name for a chip that picks it: a verse without a number, the rest as the
/// slides name them, so an intro is never called a verse.
fn kind_name(kind: SectionKind) -> String {
    match kind {
        SectionKind::Verse => t::MARK_VERSE().to_string(),
        other => t::section_label(Tag::new(other, 1)),
    }
}

/// Escape in a field closes the editor at once. Left to Rune, the first Escape only takes the
/// focus off the field and the editor stays, which reads as a dead key.
fn escape_closes(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if !matches!(ev.key, Key::Escape) {
        return false;
    }
    s.close_editor();
    tx.blur();
    true
}

/// Keys in the title while whole songs are offered under it.
fn title_keys(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let offered = s.editor.as_ref().is_some_and(|editor| !editor.suggestions.is_empty());
    match ev.key {
        Key::Down if offered => s.move_suggestion(1),
        Key::Up if offered => s.move_suggestion(-1),
        // Enter takes a song only after an arrow chose it. Tab is the next field, as everywhere:
        // the offer goes away and the key goes on.
        Key::Enter if offered && s.editor.as_ref().is_some_and(|editor| editor.suggestion_armed) => {
            s.take_suggestion(None);
        }
        Key::Tab if offered => {
            s.dismiss_suggestions();
            return false;
        }
        Key::Escape if offered => {
            s.dismiss_suggestions();
        }
        _ => return escape_closes(s, ev, tx),
    }
    true
}

/// The editor field a text input writes to. A field can outlive the editor by a frame (it
/// hears its own blur after Save or Cancel), and then it must write to a spare edit: creating
/// an editor here would reopen an empty "New song" over the song just saved.
fn edit_of(s: &mut App, pick: fn(&mut Editor) -> &mut TextEdit) -> &mut TextEdit {
    match s.editor.as_mut() {
        Some(editor) => pick(editor),
        None => &mut s.spare_edit,
    }
}

fn field(k: &Kit, label: &'static str, edit: &TextEdit, access: fn(&mut App) -> &mut TextEdit) -> El {
    col().w_full().gap(space::S1).child(section_label(k, label)).child(TextInput::new(edit).width(None).label(label).on_edit(access).on_key(escape_closes).build(k))
}

/// The marker lines of the words, each in its section's colour, so the text reads like the
/// slides beside it.
fn suggestion_row(s: &App, k: &Kit, index: usize, suggestion: &Suggestion, picked: bool) -> El {
    let (glyph, title, detail) = match suggestion {
        Suggestion::Library(song) => {
            let found = &s.songs[*song];
            (&icons::SONG, found.title.clone(), t::SUGGEST_LIBRARY().to_string())
        }
        Suggestion::Web(found) => (&icons::GLOBE, found.title.clone(), t::suggest_web(&found.artist, &found.source)),
    };
    let mut line = row()
        .key(format!("suggestion-{index}"))
        .role("option")
        .label(title.clone())
        .w_full()
        .h(size::ROW)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .fill(if picked { k.c.layer_04 } else { rune::Color::TRANSPARENT })
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(icon(glyph, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(text(title).font(type_::BODY_STRONG).color(k.c.text_base).truncate())
        .child(text(detail).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0));
    if picked {
        line = line.child(keycap(k, "Enter", false));
    }
    line.on_press(move |s: &mut App| {
        s.take_suggestion(Some(index));
    })
}

/// The title, with whole songs offered under it while a new song has no words yet.
fn title_block(s: &App, k: &Kit, editor: &Editor) -> El {
    let input = TextInput::new(&editor.title)
        .width(None)
        .label(t::FIELD_TITLE())
        .on_edit(|s: &mut App| edit_of(s, |editor| &mut editor.title))
        .on_change(|s: &mut App, _: &mut Tx| s.title_changed())
        .on_key(title_keys)
        .build(k);
    let mut block = col().key("editor-title").w_full().gap(space::S1).child(section_label(k, t::FIELD_TITLE())).child(input);
    if editor.fetching {
        block = block.child(text(t::SUGGEST_FETCHING()).font(type_::SMALL).color(k.c.text_muted));
    }
    // Offered only while the title is being typed: once the focus moves on, the list goes,
    // and it never lies over the fields under it.
    if !editor.suggestions.is_empty() && editor.title.focused {
        let mut list = col()
            .key("suggestions")
            .role("listbox")
            .label(t::SUGGEST_LABEL())
            .w_full()
            .padding(space::S1)
            .gap(2.0)
            .radius(k.radius.lg)
            .fill(k.c.layer_02)
            .border(1.0, k.c.border_base)
            // A click on a row must not first take the focus from the title, which would close the list under it.
            .preserve_focus_on_press(true)
            .child(row().w_full().h(24.0).px(space::S3).gap(space::S2).center_y().child(text(t::SUGGEST_LABEL()).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0)).child(keycap(k, "Down Enter", false)));
        for (index, suggestion) in editor.suggestions.iter().enumerate() {
            list = list.child(suggestion_row(s, k, index, suggestion, editor.suggestion_armed && index == editor.suggestion));
        }
        if editor.suggestions.iter().any(|suggestion| matches!(suggestion, Suggestion::Web(_))) {
            list = list.child(text(t::LICENCE_NOTE()).font(type_::SMALL).color(k.c.text_faint).px(space::S3).py(space::S1));
        }
        block = block.child(list);
    }
    if let Some((source, _)) = &editor.credit {
        block = block.child(crate::ui::widgets::wrapped(t::words_from(source)).font(type_::SMALL).color(theme::warn_ink()).wrap());
    }
    block
}

/// The kinds a person can pick for a stanza, in the order of the section buttons and Intro.
const PICKABLE: [SectionKind; 6] =
    [SectionKind::Verse, SectionKind::Chorus, SectionKind::PreChorus, SectionKind::Bridge, SectionKind::Intro, SectionKind::Ending];

/// Why a stanza has the tag it has, said when the pointer rests on its chip.
fn why(stanza: &Stanza) -> String {
    match stanza.basis {
        Basis::Marked => t::section_label(stanza.tag),
        Basis::Named => t::MARKS_WHY_NAMED().to_string(),
        Basis::Repeat => t::MARKS_WHY_REPEAT().to_string(),
        Basis::Bridge => t::MARKS_WHY_BRIDGE().to_string(),
        Basis::Order => t::MARKS_WHY_ORDER().to_string(),
    }
}

/// One stanza of the proposal: tinted where the words already have its mark, outlined where
/// pressing Mark would write it.
fn stanza_chip(k: &Kit, at: usize, stanza: &Stanza) -> El {
    let color = group_color(tag_group(stanza.tag));
    let ink = theme::ink_of(color);
    let written = !stanza.is_pending();
    el().key(format!("mark-chip-{at}"))
        .px(space::S2)
        .min_w(32.0)
        .h(24.0)
        .center()
        .radius(k.radius.sm)
        .fill(if written { theme::tint_of(color, true) } else { rune::Color::TRANSPARENT })
        .border(1.0, ink.alpha(if written { 0.0 } else { 0.6 }))
        .help(why(stanza))
        .child(text(t::section_label(stanza.tag)).font(type_::SMALL_STRONG).color(ink))
}

/// One kind a person can pick for the stanza at the caret. All are one width, so a longer
/// language changes nothing; the one the proposal chose is filled.
fn kind_chip(k: &Kit, start: usize, kind: SectionKind, tag: Tag, chosen: bool) -> El {
    let color = group_color(tag_group(tag));
    let ink = theme::ink_of(color);
    let name = t::section_label(tag);
    row()
        .key(format!("mark-kind-{}", tag.short()))
        .role("button")
        .label(name.clone())
        .w(96.0)
        .h(26.0)
        .gap(4.0)
        .center()
        .radius(k.radius.sm)
        .fill(if chosen { theme::tint_of(color, true) } else { rune::Color::TRANSPARENT })
        .border(1.0, ink.alpha(if chosen { 0.0 } else { 0.5 }))
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(icon(&icons::HEADING, 12.0).color(ink).a11y_hidden())
        .child(text(name).font(type_::SMALL_STRONG).color(ink).truncate().a11y_hidden())
        .on_press(move |s: &mut App| s.mark_stanza(start, kind))
}

/// A quiet strip over the words while some stanza has no mark: the sections the song would
/// get, one button that writes them, one that dismisses them; and, with the caret in an
/// unmarked stanza, the kinds to pick for that stanza alone.
fn marks_strip(s: &App, k: &Kit, editor: &Editor) -> Option<El> {
    let proposal: Proposal = s.marks_offer()?;
    let mut chips = row().key("marks-chips").grow(1.0).min_w(0.0).gap(6.0).flex_wrap().center_y();
    for (at, stanza) in proposal.stanzas.iter().enumerate() {
        chips = chips.child(stanza_chip(k, at, stanza));
    }
    let apply = row().min_w(148.0).child(btn(t::MARKS_APPLY()).icon(&icons::CHECK).small().hint(t::KEY_MARK()).help(t::MARKS_APPLY_HELP()).build(k, |s, _| s.apply_marks()));
    let hide = btn(t::MARKS_HIDE()).icon(&icons::CLOSE).small().ghost().icon_only().build(k, |s, _| s.hide_marks());
    let mut strip = col()
        .key("marks-strip")
        .w_full()
        .padding(space::S2)
        .gap(space::S2)
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_muted)
        .child(row().w_full().gap(space::S2).center_y().child(chips).child(apply).child(hide));
    let here = editor.lyrics.focused.then(|| proposal.stanza_at(editor.lyrics.range().0)).flatten().filter(|at| proposal.stanzas[*at].is_unmarked());
    if let Some(at) = here {
        let stanza = &proposal.stanzas[at];
        let mut line = row().key("marks-here").w_full().gap(6.0).flex_wrap().center_y().child(section_label(k, t::MARKS_THIS()));
        for kind in PICKABLE {
            let chosen = kind == stanza.tag.kind;
            let tag = if chosen { stanza.tag } else { proposal.tag_as(at, kind) };
            line = line.child(kind_chip(k, stanza.start, kind, tag, chosen));
        }
        strip = strip.child(line);
    }
    Some(strip)
}

/// The text a part card writes to; a spare edit once the card or the editor is gone, so a field
/// that hears its own blur late never makes state.
fn part_text(s: &mut App) -> &mut TextEdit {
    match s.editor.as_mut().and_then(|editor| editor.part.as_mut()) {
        Some(part) => &mut part.text,
        None => &mut s.spare_edit,
    }
}

/// Keys in a card: Ctrl with 1 to 5 picks the part's kind, Ctrl M writes the proposed marks,
/// Escape closes the card, then the editor.
fn part_keys(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if crate::ui::widgets::enter_breaks_line(ev, tx) {
        return true;
    }
    let open = s.editor.as_ref().and_then(|editor| editor.part.as_ref()).map(|part| part.index);
    match ev.key {
        Key::Command(digit @ '1'..='5') => {
            if let Some(at) = open {
                s.retag_part(at, SECTION_KEYS[digit as usize - '1' as usize]);
            }
            true
        }
        Key::Command('m') if s.marks_offer().is_some() => {
            s.apply_marks();
            true
        }
        Key::Escape if open.is_some() => {
            s.close_part();
            tx.blur();
            true
        }
        _ => escape_closes(s, ev, tx),
    }
}

/// A part's kind as a chip in its colour; a press opens the kinds to pick from.
fn part_kind_chip(k: &Kit, index: usize, tag: Tag) -> El {
    let color = group_color(tag_group(tag));
    row()
        .key(format!("part-kind-{index}"))
        .role("button")
        .label(t::section_label(tag))
        .h(28.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(14.0)
        .fill(theme::tint_of(color, true))
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(el().size(8.0, 8.0).radius(4.0).fill(color).a11y_hidden())
        .child(text(t::section_label(tag)).font(type_::BODY_STRONG).color(theme::ink_of(color)))
        .child(icon(k.glyph.chevron_down, 12.0).color(k.c.text_muted).a11y_hidden())
        .on_press(move |s: &mut App| {
            if let Some(editor) = s.editor.as_mut() {
                editor.part_menu = if editor.part_menu == Some(index) { None } else { Some(index) };
            }
        })
}

/// The kinds a part can be, as chips in their colours; a press gives the part that kind.
fn kind_choices(k: &Kit, index: usize, current: Tag) -> El {
    let mut line = row().key(format!("part-kinds-{index}")).w_full().gap(space::S2).flex_wrap().pl(22.0);
    for kind in PICKABLE {
        let tag = Tag::new(kind, 1);
        let color = group_color(tag_group(tag));
        let chosen = kind == current.kind;
        line = line.child(
            row()
                .role("button")
                .label(kind_name(kind))
                .help(SECTION_KEYS.iter().position(|key| *key == kind).map(|at| t::ctrl_digit(at + 1)).unwrap_or_default())
                .h(28.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(14.0)
                .fill(if chosen { theme::tint_of(color, true) } else { rune::Color::TRANSPARENT })
                .border(1.0, color.alpha(0.6))
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .child(el().size(8.0, 8.0).radius(4.0).fill(color).a11y_hidden())
                .child(text(kind_name(kind)).font(type_::SMALL_STRONG).color(k.c.text_base))
                .on_press(move |s: &mut App| s.retag_part(index, kind)),
        );
    }
    line
}

/// One part as a card: its kind, how many slides it makes, its words with a dashed rule where
/// a slide starts, and the slides themselves. A press on the words opens the card for writing.
#[allow(clippy::too_many_arguments)]
fn part_card(s: &App, k: &Kit, index: usize, section: &Section, open: Option<&crate::parts::PartEdit>, menu: bool, slides: &[artosia_core::deck::Slide], count: usize, placeholder: &str) -> El {
    let tag = open.map(|part| part.tag).unwrap_or(section.tag);
    let color = group_color(tag_group(tag));
    let grip = col().w(14.0).gap(3.0).pt(6.0).a11y_hidden().children((0..3).map(|_| row().gap(3.0).child(el().size(3.0, 3.0).radius(1.5).fill(k.c.text_faint)).child(el().size(3.0, 3.0).radius(1.5).fill(k.c.text_faint))));
    let header = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(grip)
        .child(part_kind_chip(k, index, tag))
        .child(spacer())
        .child(text(t::slides(slides.len().max(section.groups().len()))).font(type_::SMALL).color(k.c.text_muted))
        .child(btn(t::PLAN_MOVE_UP()).icon(&icons::ARROW_UP).icon_only().ghost().small().disabled(index == 0).build(k, move |s, _| s.move_part(index, -1)))
        .child(btn(t::PLAN_MOVE_DOWN()).icon(&icons::ARROW_DOWN).icon_only().ghost().small().disabled(index + 1 >= count).build(k, move |s, _| s.move_part(index, 1)))
        .child(btn(t::BTN_DELETE()).icon(&icons::TRASH).icon_only().ghost().small().build(k, move |s, _| s.delete_part(index)));
    let words = match open {
        Some(part) => TextArea::new(&part.text)
            .width(None)
            .label(t::section_label(tag))
            .placeholder(placeholder.to_string())
            .auto_grow(2, 400)
            .on_edit(part_text)
            .on_change(|s: &mut App, _: &mut Tx| s.part_changed())
            .on_key(part_keys)
            .build(k)
            .key("editor-words")
            .grow(1.0)
            .min_w(0.0),
        None => {
            let mut lines = col().grow(1.0).min_w(0.0).gap(space::S1);
            // A line the projector could not show at a readable size is marked where it stands.
            let too_long = artosia_slides::voices::too_long(&s.look(), s.slide_shape());
            let mut number = 0usize;
            for (at, group) in section.groups().iter().enumerate() {
                if at > 0 {
                    let dash = || el().h(1.0).grow(1.0).border(1.0, color.alpha(0.5)).border_dash(4.0, 4.0).a11y_hidden();
                    lines = lines.child(row().w_full().gap(space::S2).center_y().py(2.0).child(dash()).child(text((at + 1).to_string()).font(type_::TINY).color(k.c.text_faint)).child(dash()));
                }
                for line in group.iter() {
                    let place = number;
                    number += 1;
                    let mut row_of = row().w_full().gap(space::S2).center_y().child(text(line.clone()).font(type_::BODY).color(k.c.text_base).wrap().grow(1.0).min_w(0.0));
                    if too_long(line) {
                        row_of = row_of
                            .child(text(t::PART_TOO_LONG()).font(type_::SMALL_STRONG).color(theme::warn_ink()))
                            .child(btn(t::PART_SPLIT()).small().build(k, move |s, _| s.split_line(index, place)).key(format!("split-{index}-{place}")));
                    }
                    lines = lines.child(row_of);
                }
            }
            lines.key(format!("part-words-{index}")).role("button").label(t::section_label(tag)).cursor(Cursor::Text).on_press(move |s: &mut App| s.open_part(index))
        }
    };
    let mut thumbs = col().w(THUMB).gap(space::S1);
    if slides.is_empty() {
        thumbs = thumbs.child(crate::ui::widgets::wrapped(t::EDITOR_NO_SLIDES()).font(type_::SMALL).color(k.c.text_faint).wrap());
    }
    for slide in slides {
        let state = s.preview_state(Some(slide.clone()));
        thumbs = thumbs.child(el().size(THUMB, (THUMB * 9.0 / 16.0).round()).radius(4.0).clip_rounded(4.0).border(1.0, color.alpha(0.7)).child(crate::ui::thumb(s, &state, THUMB)));
    }
    let mut card = col()
        .key(format!("part-{index}"))
        .w_full()
        .gap(space::S2)
        .padding(space::S3)
        .radius(10.0)
        .fill(color.alpha(0.08))
        .border(if open.is_some() { 2.0 } else { 1.0 }, if open.is_some() { color } else { color.alpha(0.35) })
        .child(header);
    if menu {
        card = card.child(kind_choices(k, index, tag));
    }
    card.child(row().w_full().gap(space::S3).pl(22.0).child(words).child(thumbs))
}

/// The tile under the cards that starts a new part, one chip per kind.
fn new_part_tile(k: &Kit) -> El {
    let mut kinds = row().gap(space::S2).flex_wrap();
    for kind in SECTION_KEYS {
        let color = group_color(tag_group(Tag::new(kind, 1)));
        kinds = kinds.child(
            row()
                .role("button")
                .label(section_name(kind))
                .h(30.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(15.0)
                .border(1.0, color.alpha(0.6))
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .child(el().size(8.0, 8.0).radius(4.0).fill(color).a11y_hidden())
                .child(text(section_name(kind)).font(type_::SMALL_STRONG).color(k.c.text_base))
                .on_press(move |s: &mut App| s.add_part(kind)),
        );
    }
    row()
        .key("editor-parts")
        .w_full()
        .padding(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(10.0)
        .border(1.0, k.c.border_strong)
        .border_dash(6.0, 4.0)
        .child(icon(&icons::PLUS, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(text(t::PART_NEW()).font(type_::BODY_STRONG).color(k.c.text_muted))
        .child(spacer())
        .child(kinds)
}

/// The parts of the song as cards, in the order they are written, then the New part tile.
fn cards(s: &App, k: &Kit, editor: &Editor) -> El {
    let sections = s.written_parts();
    let open = editor.part.as_ref().filter(|part| part.current(&editor.lyrics.text));
    let placeholder = if editor.at == 0 { t::START_NEW_DETAIL().to_string() } else { t::editor_language_empty(&crate::ui::languages::name_in_sentence(&editor.codes[editor.at])) };
    // The slides of the song as written, so each card shows its own: a part's slides carry its tag.
    let mut written = s.editor_song().unwrap_or_default();
    written.order.clear();
    let deck = s.deck_of_song(&written);
    let mut cursor = 0;
    let mut stack = col().key("editor-cards").w_full().gap(space::S3);
    for (index, section) in sections.iter().enumerate() {
        let start = cursor;
        while cursor < deck.slides.len() && deck.slides[cursor].tag == Some(section.tag) {
            cursor += 1;
        }
        let here = open.filter(|part| !part.fresh && part.index == index);
        stack = stack.child(part_card(s, k, index, section, here, editor.part_menu == Some(index), &deck.slides[start..cursor], sections.len(), &placeholder));
    }
    // A part just added waits as a card of its own until its first word writes it.
    match open.filter(|part| part.fresh) {
        Some(part) => {
            let empty = artosia_core::song::Section { tag: part.tag, lines: Vec::new(), breaks: Vec::new() };
            stack = stack.child(part_card(s, k, part.index, &empty, Some(part), editor.part_menu == Some(part.index), &[], sections.len() + 1, &placeholder));
        }
        None if sections.is_empty() => {
            // An empty song waits with its first verse, written on the first press.
            let first = Tag::new(SectionKind::Verse, 1);
            let color = group_color(tag_group(first));
            stack = stack.child(
                col()
                    .key("part-first")
                    .w_full()
                    .gap(space::S2)
                    .padding(space::S3)
                    .radius(10.0)
                    .fill(color.alpha(0.08))
                    .border(1.0, color.alpha(0.35))
                    .role("button")
                    .label(t::START_NEW_DETAIL())
                    .cursor(Cursor::Text)
                    .on_press(|s: &mut App| s.add_part(SectionKind::Verse))
                    .child(row().gap(space::S2).center_y().child(part_kind_chip(k, 0, first)))
                    .child(text(t::START_NEW_DETAIL()).font(type_::BODY).color(k.c.text_faint).pl(22.0)),
            );
        }
        None => {}
    }
    stack.child(new_part_tile(k))
}

/// The sung order as a strip of the parts' colours, each as wide as the slides it shows, under
/// the cards. A press on a part takes it out; + offers the parts to sing again at the end.
fn order_strip(s: &App, k: &Kit, editor: &Editor) -> El {
    let song = s.editor_song().unwrap_or_default();
    let written = song.sections();
    let sung = song.arranged();
    let typed = !editor.order.text.trim().is_empty();
    // Consecutive repeats are one chip with a count, as Planning Center writes "Chorus x2".
    let mut runs: Vec<(usize, Tag, usize, usize)> = Vec::new();
    for (at, section) in sung.iter().enumerate() {
        let size = section.groups().len().max(1);
        match runs.last_mut() {
            Some(run) if run.1 == section.tag => run.2 += 1,
            _ => runs.push((at, section.tag, 1, size)),
        }
    }
    let mut strip = row().key("order-sung").w_full().h(40.0).gap(3.0);
    for (at, tag, times, size) in runs {
        let color = group_color(tag_group(tag));
        let mut chip = row()
            .key(format!("order-{at}"))
            .role("button")
            .label(t::section_label(tag))
            .h_full()
            .grow((size * times) as f32)
            .min_w(if times > 1 { 96.0 } else { 74.0 })
            .px(space::S2)
            .gap(4.0)
            .center_y()
            .radius(6.0)
            .fill(theme::tint_of(color, true))
            .border(1.0, color.alpha(0.6))
            .hover(k.c.hover)
            .cursor(Cursor::Pointer)
            .child(text(t::section_label(tag)).font(type_::SMALL_STRONG).color(theme::ink_of(color)).truncate().min_w(0.0))
            .on_press(move |s: &mut App| s.order_remove(at));
        if times > 1 {
            chip = chip.child(text(format!("\u{00d7}{times}")).font(type_::SMALL_STRONG).color(theme::ink_of(color)));
        }
        strip = strip.child(chip);
    }
    if sung.is_empty() {
        strip = strip.child(text(t::ORDER_EMPTY()).font(type_::SMALL).color(k.c.text_muted).grow(1.0));
    }
    let add = btn(t::ORDER()).icon(&icons::PLUS).icon_only().active(editor.order_menu).focusable().build(k, |s, _| {
        if let Some(editor) = s.editor.as_mut() {
            editor.order_menu = !editor.order_menu;
        }
    });
    strip = strip.child(add.key("order-plus"));
    let has_chorus = written.iter().any(|section| section.tag.kind == SectionKind::Chorus) && written.iter().any(|section| section.tag.kind == SectionKind::Verse);
    let mut tools = row().gap(space::S2).center_y();
    if has_chorus {
        tools = tools.child(btn(t::ORDER_CHORUS_EACH()).small().ghost().focusable().build(k, |s, _| s.order_chorus_each_verse()));
    }
    if typed {
        tools = tools.child(btn(t::ORDER_AS_WRITTEN()).small().ghost().focusable().build(k, |s, _| s.order_as_written()));
    }
    let mut block = col()
        .key("editor-order")
        .w_full()
        .px(space::S6)
        .py(space::S3)
        .gap(space::S2)
        .fill(theme::surf().side)
        .child(row().w_full().gap(space::S2).center_y().child(section_label(k, t::ORDER())).child(text(t::slides(sung.iter().map(|section| section.groups().len().max(1)).sum())).font(type_::SMALL).color(k.c.text_faint)).child(spacer()).child(tools));
    if editor.order_menu {
        // Each part once, however often it was written: a chorus typed twice is one chorus to add.
        let mut offered: Vec<Tag> = Vec::new();
        for section in &written {
            if !offered.contains(&section.tag) {
                offered.push(section.tag);
            }
        }
        let mut add = row().key("order-add").w_full().gap(6.0).flex_wrap().center_y();
        for (at, tag) in offered.into_iter().enumerate() {
            let color = group_color(tag_group(tag));
            add = add.child(
                row()
                    .key(format!("order-add-{at}"))
                    .role("button")
                    .label(t::section_label(tag))
                    .h(28.0)
                    .px(space::S3)
                    .gap(space::S2)
                    .center_y()
                    .radius(14.0)
                    .border(1.0, color.alpha(0.6))
                    .hover(k.c.hover)
                    .cursor(Cursor::Pointer)
                    .child(icon(&icons::PLUS, 12.0).color(k.c.text_muted).a11y_hidden())
                    .child(text(t::section_label(tag)).font(type_::SMALL_STRONG).color(k.c.text_base))
                    .on_press(move |s: &mut App| s.order_add(tag)),
            );
        }
        block = block.child(add);
    }
    block.child(strip)
}

fn form(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let Some(editor) = s.editor.as_ref() else { return el() };
    let mut column = col()
        .key("editor-form")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .px(space::S6)
        .pb(space::S6)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE)
        .child(title_block(s, k, editor));
    // Who wrote the song and its rights wait behind Song details: filled in last, when at all.
    if editor.details_open {
        column = column
            .child(
                row()
                    .w_full()
                    .gap(space::S3)
                    .child(el().grow(1.0).min_w(0.0).child(field(k, t::FIELD_AUTHOR(), &editor.author, |s| edit_of(s, |editor| &mut editor.author))))
                    .child(el().w(96.0).child(field(k, t::FIELD_KEY(), &editor.key, |s| edit_of(s, |editor| &mut editor.key)))),
            )
            .child(
                row()
                    .w_full()
                    .gap(space::S3)
                    .child(el().grow(1.0).min_w(0.0).child(field(k, t::FIELD_COPYRIGHT(), &editor.copyright, |s| edit_of(s, |editor| &mut editor.copyright))))
                    .child(el().w(140.0).child(field(k, t::FIELD_CCLI(), &editor.ccli, |s| edit_of(s, |editor| &mut editor.ccli)))),
            );
    }
    column = column
        .child(row().w_full().center_y().gap(space::S2).child(section_label(k, t::FIELD_LYRICS())).child(spacer()).child(btn(t::BTN_TIDY()).small().ghost().help(t::TIDY_HELP()).build(k, |s, _| s.tidy_editor())))
        .child(crate::ui::languages::chips(k, editor, layer))
        .children(marks_strip(s, k, editor))
        .child(cards(s, k, editor));
    col().grow(1.0).min_w(0.0).h_full().min_h(0.0).child(column.scroll_bar(k).grow(1.0).min_h(0.0)).child(order_strip(s, k, editor))
}

pub fn view(s: &App, k: &Kit, _width: f32, layer: &mut Vec<El>) -> El {
    let Some(editor) = s.editor.as_ref() else { return el() };
    let existing = editor.id != 0;
    let id = editor.id;
    let mut header = row()
        .key("editor-header")
        .w_full()
        .px(space::S6)
        .pt(space::S5)
        .pb(space::S4)
        .gap(space::S2)
        .center_y()
        .child(text(if existing { t::EDITOR_EDIT() } else { t::EDITOR_NEW() }).font(type_::HEADING).color(k.c.text_base))
        .child(crate::ui::tour::help_mark(s, k, crate::guide::lessons::Feature::Songs))
        .child(spacer());
    if !existing {
        header = header.child(btn(t::CMD_WEB_SONGS()).icon(&icons::GLOBE).ghost().focusable().anchor(crate::palette::Command::WebSongs, crate::guide::Place::Panel).build(k, |s, _| s.open_web_songs()));
    }
    if existing {
        header = header.child(btn(t::BTN_DELETE_SONG()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, tx: &mut Tx| {
            s.delete_song(id);
            tx.blur();
        }));
    }
    header = header
        .child(btn(t::EDITOR_DETAILS()).icon(&icons::SLIDERS).ghost().active(editor.details_open).focusable().build(k, |s, _| {
            if let Some(editor) = s.editor.as_mut() {
                editor.details_open = !editor.details_open;
            }
        }))
        .child(btn(t::BTN_CANCEL()).hint("Esc").focusable().build(k, |s, tx| {
            s.close_editor();
            tx.blur();
        }))
        .child(btn(t::BTN_SAVE()).primary().hint(t::KEY_SAVE()).focusable().build(k, |s, tx| {
            s.save_editor();
            tx.blur();
        }).key("editor-save"));
    col().key("editor").w_full().h_full().min_h(0.0).child(header).child(
        row().w_full().grow(1.0).min_h(0.0).child(form(s, k, layer)),
    )
}
