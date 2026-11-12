//! A song in several languages, drawn (board card MULTI1): the dialog that chooses which
//! languages a slide shows, the list a language is picked from, the editor's row of chips and
//! the Live pane's. The rules they obey are in `song_languages.rs`.

use artosia_core::languages;
use rune::{Cursor, El, GrowthSpec, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, text};
use rune_kit::{Dialog, Kit, Menu, MenuItem, ScrollBar, Switch, TextInput};

use crate::song_languages::PickFor;
use crate::state::{App, Editor, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::{btn, wrapped};

/// A row of the dialog and of the picker.
const ROW: f32 = 44.0;
/// The picker's list, which scrolls inside the dialog.
const LIST: f32 = 300.0;

fn close(s: &mut App) {
    s.sheet = Sheet::None;
    s.language_pick = None;
}

/// The name of a code in the window's language; a code nobody named asks for one.
pub fn name_of(code: &str) -> String {
    if code.is_empty() { t::LANGUAGE_PICK_TITLE().to_string() } else { languages::name(code, t::language_code()) }
}

/// The same name as it reads inside a sentence.
pub fn name_in_sentence(code: &str) -> String {
    if code.is_empty() { t::LANGUAGE_PICK_TITLE().to_string() } else { languages::name_in_sentence(code, t::language_code()) }
}

// ---------------------------------------------------------------- the dialog

/// The two tiles of the plan's menu, as the look dialog's: the usual languages, or this plan's own.
/// Both are one size taken from the strip, whatever the words; the chosen one carries a tick.
fn chooser(s: &App, k: &Kit) -> El {
    let own = s.has_own_languages();
    let usual = btn(t::LANGUAGES_USUAL()).icon(if own { &icons::GLOBE } else { &icons::CHECK }).focusable().grow();
    let usual = if own { usual } else { usual.primary() };
    let plan = btn(t::LANGUAGES_THIS_PLAN()).icon(if own { &icons::CHECK } else { &icons::PLUS }).help(s.plan_title()).focusable().grow();
    let plan = if own { plan.primary() } else { plan };
    let first = move |button: El, chosen: bool| if chosen { button.autofocus() } else { button };
    row()
        .key("languages-for")
        .w_full()
        .gap(space::S2)
        .child(el().w_full().max_w(280.0).grow(1.0).child(first(usual.build(k, |s: &mut App, _: &mut Tx| s.use_usual_languages()).key("languages-for-usual"), !own)))
        .child(el().w_full().max_w(280.0).grow(1.0).child(first(plan.build(k, |s: &mut App, _: &mut Tx| s.use_own_languages()).key("languages-for-plan"), own)))
}

/// One language of the list: a switch, its name, and the two arrows that move it.
fn language_row(k: &Kit, code: &str, at: Option<usize>, last: usize) -> El {
    let name = name_of(code);
    let on = at.is_some();
    let pressed = code.to_string();
    let switch = Switch::new(on).label(name.clone()).hide_label().on_toggle(move |s: &mut App| s.language_switch(&pressed)).build(k);
    let mut line = row()
        .key(format!("language-{code}"))
        .w_full()
        .h(ROW)
        .px(space::S2)
        .gap(space::S3)
        .center_y()
        .child(switch)
        .child(text(name.clone()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(if on { k.c.text_base } else { k.c.text_muted }).truncate().grow(1.0).min_w(0.0));
    if let Some(at) = at {
        line = line
            .child(btn(t::PLAN_MOVE_UP()).icon(&icons::ARROW_UP).icon_only().small().ghost().disabled(at == 0).focusable().build(k, move |s, _| s.language_move(at, true)))
            .child(btn(t::PLAN_MOVE_DOWN()).icon(&icons::ARROW_DOWN).icon_only().small().ghost().disabled(at == last).focusable().build(k, move |s, _| s.language_move(at, false)));
    } else {
        line = line.child(el().w(26.0 * 2.0 + 4.0));
    }
    line
}

/// Which languages a slide shows together. From a plan's menu it opens on the two tiles; from
/// Settings it edits the church's own list.
pub fn dialog(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    if s.language_pick.is_some() {
        return;
    }
    let list = s.languages_list();
    let mut rows = col().key("languages-list").role("list").label(t::LANGUAGES_ON_SLIDE()).w_full().gap(2.0);
    for (at, code) in list.iter().enumerate() {
        rows = rows.child(language_row(k, code, Some(at), list.len().saturating_sub(1)));
    }
    for code in s.languages_off.iter().filter(|code| !list.contains(code)) {
        rows = rows.child(language_row(k, code, None, 0));
    }
    let add = btn(t::LANGUAGE_ADD()).icon(&icons::PLUS).focusable().build(k, |s, _| s.open_language_pick(PickFor::Slide)).key("language-add");
    let mut body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(wrapped(t::LANGUAGES_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap());
    if s.languages_from_plan {
        body = body.child(chooser(s, k));
    }
    body = body.child(rows).child(row().w_full().child(add));
    // A window lower than a laptop's scrolls the rows, never the buttons off the dialog.
    let body = col().w_full().max_h((tall - 120.0).max(240.0)).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(body).scroll_bar(k);
    Dialog::new("languages", t::LANGUAGES_ON_SLIDE()).open(true).width(520.0).fit().on_close(close).child(body).build(k, layer);
}

// ---------------------------------------------------------------- the picker

/// The list a language is picked from: pick, never type. The filter shortens it; Enter takes the first.
pub fn picker(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(pick) = &s.language_pick else { return };
    let field = TextInput::new(&pick.filter)
        .width(None)
        .height(38.0)
        .label(t::LANGUAGE_FILTER())
        .placeholder(t::LANGUAGE_FILTER())
        .leading_icon(&rune_kit::glyphs::SEARCH)
        .on_edit(|s: &mut App| match s.language_pick.as_mut() {
            Some(pick) => &mut pick.filter,
            None => &mut s.spare_edit,
        })
        .on_key(|s: &mut App, ev: &KeyEvent, _: &mut Tx| {
            if ev.key == Key::Enter {
                let first = s.language_pick.as_ref().and_then(|pick| pick.visible().first().map(|(code, _)| (*code).to_string()));
                if let Some(code) = first {
                    s.pick_language_code(&code);
                }
                return true;
            }
            if ev.key == Key::Escape {
                s.close_language_pick();
                return true;
            }
            false
        })
        .build(k)
        .key("language-filter");
    let visible = pick.visible();
    let inside = if visible.is_empty() {
        col().w_full().h(ROW).center_y().px(space::S3).child(wrapped(t::nothing_found(pick.filter.text.trim())).font(type_::BODY).color(k.c.text_muted).wrap())
    } else {
        col().w_full().growth(GrowthSpec::default()).virtual_rows(
            visible.len(),
            ROW,
            |s: &App, at| match s.language_pick.as_ref().and_then(|pick| pick.visible().get(at).map(|(code, _)| *code)) {
                Some(code) => format!("pick-language-{code}").into(),
                None => format!("pick-gap-{at}").into(),
            },
            |s: &App, at| {
                let Some((code, name)) = s.language_pick.as_ref().and_then(|pick| pick.visible().get(at).map(|(code, name)| ((*code).to_string(), (*name).to_string()))) else { return el() };
                let k = theme::kit();
                let chosen = code.clone();
                row()
                    .key(format!("pick-language-{code}"))
                    .role("option")
                    .label(name.clone())
                    .w_full()
                    .h(ROW)
                    .px(space::S3)
                    .gap(space::S3)
                    .center_y()
                    .radius(k.radius.md)
                    .hover(k.c.hover)
                    .cursor(Cursor::Pointer)
                    .focusable(true)
                    .on_key(crate::ui::widgets::enter_key)
                    .child(text(name).font(type_::BODY).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
                    .on_press(move |s: &mut App| s.pick_language_code(&chosen))
            },
        )
    };
    let list = col().key("language-list").role("listbox").label(t::LANGUAGE_PICK_TITLE()).w_full().h(LIST).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(inside).scroll_bar(k);
    let body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(field).child(list);
    Dialog::new("language-pick", t::LANGUAGE_PICK_TITLE()).open(true).width(440.0).fit().on_close(|s: &mut App| s.close_language_pick()).child(body).build(k, layer);
}

// ---------------------------------------------------------------- the editor's chips

/// One chip of the editor's row: the language's name, lit while its words are in the field, with a
/// menu that takes it out when the song has another. A language nobody named reads "Choose a
/// language" and a press asks for its code: the window's language is never guessed.
fn chip(k: &Kit, editor: &Editor, at: usize, layer: &mut Vec<El>) -> El {
    let code = editor.codes[at].clone();
    let (on, unnamed) = (at == editor.at, code.is_empty());
    let name = name_of(&code);
    let ink = if on { k.c.text_base } else { k.c.text_muted };
    let mut line = row()
        .key(format!("lang-chip-{at}"))
        .h(28.0)
        .radius(k.radius.sm)
        .gap(2.0)
        .center_y()
        .fill(if on { theme::accent().alpha(0.18) } else { rune::Color::TRANSPARENT })
        .border(1.0, if on { theme::accent() } else { k.c.border_base });
    let mut label = row().key(format!("lang-chip-name-{at}")).role("button").label(name.clone()).h(28.0).pl(space::S2).pr(if editor.codes.len() > 1 { 2.0 } else { space::S2 }).gap(6.0).center_y().hover(k.c.hover).cursor(Cursor::Pointer).focusable(true).on_key(crate::ui::widgets::enter_key);
    if unnamed {
        label = label.child(icon(&icons::GLOBE, 14.0).color(ink).a11y_hidden());
    }
    label = label.child(text(name).font(if on { type_::SMALL_STRONG } else { type_::SMALL }).color(ink).truncate().a11y_hidden()).on_press(move |s: &mut App| s.editor_chip(at));
    line = line.child(label);
    if editor.codes.len() > 1 {
        let trigger = btn(t::LANGUAGE_REMOVE()).icon(&icons::DOTS).icon_only().small().ghost().focusable().build(k, move |s, tx: &mut Tx| {
            if let Some(editor) = s.editor.as_mut() {
                editor.menu = if editor.menu == Some(at) { None } else { Some(at) };
            }
            tx.blur();
        });
        let menu = Menu::new(format!("lang-chip-menu-{at}"))
            .trigger(trigger.key(format!("lang-chip-menu-button-{at}")))
            .open(editor.menu == Some(at))
            .min_width(200.0)
            .on_close(|s: &mut App| {
                if let Some(editor) = s.editor.as_mut() {
                    editor.menu = None;
                }
            })
            .item(MenuItem::new(t::LANGUAGE_REMOVE()).icon(&icons::TRASH).on_select(move |s: &mut App| s.editor_remove_language(at)))
            .build(k, layer);
        line = line.child(menu);
    }
    line
}

/// The row of chips over the song's words: one for each language the song has, the first
/// language first, and last the one that adds a language.
pub fn chips(k: &Kit, editor: &Editor, layer: &mut Vec<El>) -> El {
    let mut chips = row().key("lang-chips").w_full().gap(6.0).flex_wrap().center_y();
    for at in 0..editor.codes.len() {
        chips = chips.child(chip(k, editor, at, layer));
    }
    chips.child(btn(t::LANGUAGE_ADD()).icon(&icons::PLUS).small().ghost().focusable().build(k, |s, _| s.open_language_pick(PickFor::AddToSong)).key("lang-add"))
}

// ---------------------------------------------------------------- the Live pane's chips

/// One chip of the Live pane: a dot in the language's own colour and its name. Pressing it shows
/// that language alone, lit; pressing it again shows all. It never takes the keyboard's focus, so
/// Space and the arrows still move the projector.
fn live_chip(s: &App, k: &Kit, code: String, name: String, slot: usize, widest: Option<f32>) -> El {
    let on = s.only_language.as_deref() == Some(code.as_str());
    let (fill, ink) = if on { (theme::warn(), rune::Color::BLACK) } else { (k.c.layer_03, k.c.text_base) };
    let dot = s.look().voice_ink(slot);
    let pressed = code.clone();
    let mut chip = row();
    if let Some(widest) = widest {
        chip = chip.max_w(widest);
    }
    chip.key(format!("live-language-{code}"))
        .role("button")
        .shrink()
        .label(name.clone())
        .h(20.0)
        .px(6.0)
        .gap(4.0)
        .center_y()
        .radius(6.0)
        .fill(fill)
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(el().size(8.0, 8.0).radius(4.0).fill(dot).border(1.0, k.c.border_strong).a11y_hidden())
        .child(text(name).font(type_::TINY).color(ink).truncate().a11y_hidden())
        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
            s.toggle_only_language(&pressed);
            tx.blur();
        })
}

/// What the thin Live pane (beside an editor) leaves for the chips: its width less its padding,
/// its heading and the live mark.
const THIN_PANE_ROOM: f32 = 190.0;

/// The chips beside the Live heading; none while the live slide shows one language.
pub fn live_chips(s: &App, k: &Kit, narrow: bool) -> Option<El> {
    let shown = s.live_languages();
    if shown.is_empty() {
        return None;
    }
    // The row and its chips give way in a thin pane (beside an editor): a name is cut with its dots, never clipped.
    let mut chips = row().key("live-languages").role("group").label(t::LIVE_LANGUAGES()).gap(4.0).center_y().shrink();
    // In the thin pane the chips share what the heading leaves, each the same width, so no name is
    // squeezed to a letter while another keeps nine.
    let widest = narrow.then(|| (THIN_PANE_ROOM / shown.len() as f32 - 4.0).max(28.0));
    for (code, name, slot) in shown {
        chips = chips.child(live_chip(s, k, code, name, slot, widest));
    }
    Some(chips)
}
