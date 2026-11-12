//! The song editor: the words on the left, the slides they make on the right, changing as
//! you type.
//!
//! The words stay plain text with marker lines (`[1]`, `[chorus]`) because that is what pastes
//! in from anywhere and what EasiSlides wrote. The editor does the rest around it: marker lines
//! are coloured like their slides, a section button writes the next free marker, the order is
//! built from chips, and a new song's title offers whole songs to fill in.

use artosia_core::deck::tag_group;
use artosia_core::marks::{Basis, Proposal, Stanza};
use artosia_core::song::{SectionKind, Tag, heading};
use artosia_slides::stage::group_color;
use rune::{Cursor, El, Key, KeyEvent, ScrollSpec, Span, TextEdit, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar, TextArea, TextInput};

use crate::state::{App, Editor, Suggestion};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, keycap, section_label, tag_chip};

const PREVIEW: f32 = 248.0;

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

/// Keys in the words: Ctrl with 1 to 5 starts a section, Ctrl M writes the suggested marks,
/// Escape closes.
fn lyrics_keys(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if crate::ui::widgets::enter_breaks_line(ev, tx) {
        return true;
    }
    if let Key::Command(digit @ '1'..='5') = ev.key {
        s.insert_section(SECTION_KEYS[digit as usize - '1' as usize]);
        return true;
    }
    if matches!(ev.key, Key::Command('m')) && s.marks_offer().is_some() {
        s.apply_marks();
        return true;
    }
    escape_closes(s, ev, tx)
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
fn marker_spans(lyrics: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut at = 0;
    for line in lyrics.split_inclusive('\n') {
        let shown = line.trim_end_matches(['\n', '\r']);
        if let Some(tag) = heading(shown).filter(|_| !shown.trim().is_empty()) {
            let mut span = Span::new(at, at + shown.len()).weight(700.0);
            span.color = Some(theme::ink_of(group_color(tag_group(tag))));
            spans.push(span);
        }
        at += line.len();
    }
    spans
}

/// One whole song offered under the title.
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

/// The sung order as chips. A click on a chip in the order takes it out; a click on a section
/// under it sings that section again at the end.
fn order_block(s: &App, k: &Kit, editor: &Editor) -> El {
    let song = s.editor_song().unwrap_or_default();
    let written: Vec<Tag> = song.sections().iter().map(|section| section.tag).collect();
    let sung: Vec<Tag> = song.arranged().iter().map(|section| section.tag).collect();
    let typed = !editor.order.text.trim().is_empty();
    let chip = |key: String, tag: Tag, filled: bool, plus: bool| {
        let color = group_color(tag_group(tag));
        let label = if plus { format!("+ {}", t::section_label(tag)) } else { t::section_label(tag) };
        el().key(key)
            .role("button")
            .label(t::section_label(tag))
            .px(space::S2)
            .h(26.0)
            .center()
            .radius(k.radius.sm)
            .fill(if filled { theme::tint_of(color, true) } else { rune::Color::TRANSPARENT })
            .border(1.0, theme::ink_of(color).alpha(if filled { 0.0 } else { 0.5 }))
            .hover(k.c.hover)
            .cursor(Cursor::Pointer)
            .help(t::section_label(tag))
            .child(text(label).font(type_::SMALL_STRONG).color(theme::ink_of(color)))
    };
    let mut order = row().key("order-sung").w_full().gap(6.0).flex_wrap().center_y();
    for (at, tag) in sung.iter().enumerate() {
        order = order.child(chip(format!("order-{at}"), *tag, true, false).on_press(move |s: &mut App| s.order_remove(at)));
    }
    if sung.is_empty() {
        order = order.child(text(t::ORDER_EMPTY()).font(type_::SMALL).color(k.c.text_muted));
    }
    let mut add = row().key("order-add").w_full().gap(6.0).flex_wrap().center_y();
    // Each section once, however often it was written: a chorus typed twice is one chorus to add.
    let mut offered: Vec<Tag> = Vec::new();
    for tag in &written {
        if !offered.contains(tag) {
            offered.push(*tag);
        }
    }
    for (at, tag) in offered.into_iter().enumerate() {
        add = add.child(chip(format!("order-add-{at}"), tag, false, true).on_press(move |s: &mut App| s.order_add(tag)));
    }
    let has_chorus = written.iter().any(|tag| tag.kind == SectionKind::Chorus) && written.iter().any(|tag| tag.kind == SectionKind::Verse);
    let mut tools = row().gap(space::S2).center_y();
    if has_chorus {
        tools = tools.child(btn(t::ORDER_CHORUS_EACH()).small().focusable().build(k, |s, _| s.order_chorus_each_verse()));
    }
    if typed {
        tools = tools.child(btn(t::ORDER_AS_WRITTEN()).small().ghost().focusable().build(k, |s, _| s.order_as_written()));
    }
    col()
        .key("editor-order")
        .w_full()
        .gap(space::S2)
        .child(row().w_full().center_y().gap(space::S2).child(section_label(k, t::ORDER())).child(text(if typed { "" } else { t::ORDER_AS_WRITTEN() }).font(type_::SMALL).color(k.c.text_faint)).child(spacer()).child(tools))
        .child(order)
        .child(add)
        .child(TextInput::new(&editor.order).width(None).label(t::ORDER()).placeholder(t::FIELD_ORDER()).on_edit(|s: &mut App| edit_of(s, |editor| &mut editor.order)).on_key(escape_closes).build(k))
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

fn form(s: &App, k: &Kit, layer: &mut Vec<El>) -> El {
    let Some(editor) = s.editor.as_ref() else { return el() };
    let mut sections = row().gap(space::S1).center_y();
    for (at, kind) in SECTION_KEYS.into_iter().enumerate() {
        sections = sections.child(btn(section_name(kind)).small().help(t::ctrl_digit(at + 1)).build(k, move |s, _| s.insert_section(kind)));
    }
    let column = col()
        .key("editor-form")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .px(space::S6)
        .pb(space::S6)
        .gap(space::S3)
        .scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE)
        .child(title_block(s, k, editor))
        .child(
            col()
                .w_full()
                .gap(space::S1)
                .child(
                    row()
                        .w_full()
                        .center_y()
                        .gap(space::S2)
                        .child(section_label(k, t::FIELD_LYRICS()))
                        .child(spacer())
                        .child(sections)
                        .child(btn(t::BTN_TIDY()).small().ghost().help(t::TIDY_HELP()).build(k, |s, _| s.tidy_editor())),
                )
                .child(crate::ui::languages::chips(k, editor, layer))
                .children(marks_strip(s, k, editor))
                .child(
                    TextArea::new(&editor.lyrics)
                        .width(None)
                        .label(t::FIELD_LYRICS())
                        .placeholder(if editor.at == 0 { t::LYRICS_PLACEHOLDER().to_string() } else { t::editor_language_empty(&crate::ui::languages::name_in_sentence(&editor.codes[editor.at])) })
                        .auto_grow(10, 400)
                        .decorations(marker_spans(&editor.lyrics.text))
                        .on_edit(|s: &mut App| edit_of(s, |editor| &mut editor.lyrics))
                        .on_change(|s: &mut App, _: &mut Tx| {
                            // Once a new song has words of its own, nothing more is offered for it.
                            s.dismiss_suggestions();
                        })
                        .on_key(lyrics_keys)
                        .build(k),
                ),
        )
        .child(order_block(s, k, editor));
    // The words come right after the title, one Tab away; who wrote the song and its rights
    // are filled in last, when at all (two testers pressed Tab five times to reach the words).
    let column = column
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
    column.scroll_bar(k)
}

fn slides(s: &App, k: &Kit) -> El {
    let deck = s.editor_song().map(|song| s.deck_of_song(&song));
    let mut column = col().key("editor-slides").w_full().grow(1.0).min_h(0.0).padding(space::S4).gap(space::S3).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE);
    let slides = deck.map(|deck| deck.slides).unwrap_or_default();
    if slides.is_empty() {
        column = column.child(crate::ui::widgets::wrapped(t::EDITOR_NO_SLIDES()).font(type_::SMALL).color(k.c.text_muted).wrap());
    }
    let width = PREVIEW - 2.0 * space::S4;
    for (index, slide) in slides.iter().enumerate() {
        let state = s.preview_state(Some(slide.clone()));
        let mut caption = row().gap(space::S2).center_y().child(text((index + 1).to_string()).font(type_::SMALL).color(k.c.text_faint));
        if let Some(tag) = slide.tag {
            caption = caption.child(tag_chip(&slide.label, group_color(tag_group(tag))));
        }
        column = column.child(
            col().key(format!("editor-slide-{index}")).gap(space::S1).child(
                el().size(width, (width * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).child(crate::ui::thumb(s, &state, width)),
            ).child(caption),
        );
    }
    col().w(PREVIEW).h_full().min_h(0.0).fill(k.c.bg_base).child(column.scroll_bar(k))
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
        .child(spacer());
    if existing {
        header = header.child(btn(t::BTN_DELETE_SONG()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, tx: &mut Tx| {
            s.delete_song(id);
            tx.blur();
        }));
    }
    header = header
        .child(btn(t::BTN_CANCEL()).hint("Esc").focusable().build(k, |s, tx| {
            s.close_editor();
            tx.blur();
        }))
        .child(btn(t::BTN_SAVE()).primary().hint(t::KEY_SAVE()).focusable().build(k, |s, tx| {
            s.save_editor();
            tx.blur();
        }));
    col().key("editor").w_full().h_full().min_h(0.0).child(header).child(
        row().w_full().grow(1.0).min_h(0.0).child(form(s, k, layer)).child(crate::ui::widgets::vertical_divider(k)).child(slides(s, k)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_marker_lines_are_coloured() {
        let lyrics = "[1]\nA line of words\n\n[chorus]\nMore words\n";
        let spans = marker_spans(lyrics);
        let marked: Vec<&str> = spans.iter().map(|span| &lyrics[span.start as usize..span.end as usize]).collect();
        assert_eq!(marked, ["[1]", "[chorus]"]);
    }
}
