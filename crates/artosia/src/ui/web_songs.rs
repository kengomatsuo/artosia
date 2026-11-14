//! The Songs from the internet dialog (WEBDLG1): results on the left with their source, the
//! parsed words on the right as Artosia's own preview of sections, the credit and licence lines
//! under it, and Import on a button with its key. The page itself is never drawn.

use rune::{Cursor, El, Key, KeyEvent, Mods, ScrollSpec, Tx, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, ScrollBar, TextInput};

use super::widgets::{btn, divider, enter_key, section_chip, wrapped};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::web_songs::{Ready, Words};

const WIDTH: f32 = 920.0;
/// The results column; the preview takes the rest.
const LEFT: f32 = 340.0;
/// The height of the results and of the preview, which never change with what is in them.
const LIST: f32 = 340.0;
/// Two lines of small text under each column.
const NOTE: f32 = 36.0;
/// The chip column of the preview.
const CHIPS: f32 = 88.0;

/// Arrows walk the results, Enter asks at once, and Ctrl Enter imports, wherever the focus is in the dialog.
fn keys(s: &mut App, ev: &KeyEvent, _: &mut Tx) -> bool {
    match ev.key {
        Key::Enter if ev.mods.ctrl || ev.mods.cmd => s.web_import(),
        Key::Enter if ev.mods == Mods::default() => s.web_ask_now(),
        Key::Down => s.web_step(1),
        Key::Up => s.web_step(-1),
        _ => return false,
    }
    true
}

/// A result row takes the arrows too, and Enter presses it as it does anywhere.
fn row_keys(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    match ev.key {
        Key::Down => s.web_step(1),
        Key::Up => s.web_step(-1),
        Key::Enter if ev.mods.ctrl || ev.mods.cmd => s.web_import(),
        _ => return enter_key(s, ev, tx),
    }
    true
}

fn result(k: &Kit, at: usize, found: &artosia_lyrics::Found, picked: bool) -> El {
    let detail = if found.source == artosia_lyrics::PASTED { t::WEB_PASTED().to_string() } else { t::suggest_web(&found.artist, &found.source) };
    row()
        .key(format!("web-result-{at}"))
        .role("option")
        .label(found.title.clone())
        .w_full()
        .h(size::RESULT)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .fill(if picked { k.c.layer_04 } else { rune::Color::TRANSPARENT })
        .hover(k.c.hover)
        .focusable(true)
        .on_key(row_keys)
        .cursor(Cursor::Pointer)
        .child(icon(&icons::SONG, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(col().min_w(0.0).grow(1.0).child(text(found.title.clone()).font(if picked { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate()).child(text(detail).font(type_::SMALL).color(k.c.text_muted).truncate()))
        .on_press(move |s: &mut App| s.web_choose(at))
}

fn centred(k: &Kit, key: &'static str, words: String, warn: bool) -> El {
    col().key(key).w_full().h_full().center().padding(space::S4).child(wrapped(words).font(type_::BODY).color(if warn { theme::warn_ink() } else { k.c.text_muted }).wrap())
}

/// The results, or the one sentence that says why there are none.
fn results(s: &App, k: &Kit) -> El {
    let Some(web) = s.web.as_ref() else { return el() };
    let inside = if !web.found.is_empty() {
        let mut rows = col().key("web-results").role("listbox").label(t::SEARCH_RESULTS()).w_full().padding(space::S1).gap(2.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(size::FADE);
        for (at, found) in web.found.iter().enumerate() {
            rows = rows.child(result(k, at, found, web.picked == Some(at)));
        }
        col().w_full().h_full().min_h(0.0).child(rows.scroll_bar(k))
    } else if web.typed().is_empty() {
        centred(k, "web-hint", t::WEB_TYPE_HINT().to_string(), false)
    } else if web.searching {
        centred(k, "web-searching", t::WEB_SEARCHING().to_string(), false)
    } else {
        centred(k, "web-nothing", t::WEB_NOTHING().to_string(), false)
    };
    col().key("web-list").w_full().h(LIST).radius(k.radius.lg).fill(k.c.layer_02).clip_rounded(k.radius.lg).child(inside)
}

/// Under the results: LRCLIB's problem, else the state of the index. Quiet either way.
fn status(s: &App, k: &Kit) -> El {
    let Some(web) = s.web.as_ref() else { return el() };
    let line = if let Some(problem) = &web.problem {
        Some((problem.clone(), true))
    } else if s.gema.failed {
        Some((t::WEB_INDEX_FAILED().to_string(), false))
    } else if s.gema.reading && s.gema.index.is_none() {
        Some((t::WEB_INDEX_LOADING().to_string(), false))
    } else {
        None
    };
    let note = col().key("web-status").w_full().h(NOTE).min_h(0.0);
    match line {
        Some((words, warn)) => note.child(wrapped(words).font(type_::SMALL).color(if warn { theme::warn_ink() } else { k.c.text_muted }).wrap()),
        None => note,
    }
}

/// One section: its chip and its lines, as the slides will carry them.
fn section(k: &Kit, at: usize, section: &artosia_core::song::Section) -> El {
    let mut lines = col().min_w(0.0).grow(1.0);
    for line in &section.lines {
        lines = lines.child(wrapped(line.clone()).font(type_::BODY).color(k.c.text_base).wrap());
    }
    row().key(format!("web-section-{at}")).w_full().gap(space::S3).child(el().w(CHIPS).child(section_chip(section.tag))).child(lines)
}

fn preview_of(k: &Kit, ready: &Ready) -> El {
    let fetched = &ready.fetched;
    let mut head = col().w_full().px(space::S3).pt(space::S3).pb(space::S2).child(text(ready.song.title.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate());
    if !ready.song.author.is_empty() {
        head = head.child(text(ready.song.author.clone()).font(type_::SMALL).color(k.c.text_muted).truncate());
    }
    let mut sections = col().key("web-sections").role("list").w_full().px(space::S3).py(space::S2).gap(space::S3);
    for (at, one) in ready.sections.iter().enumerate() {
        sections = sections.child(section(k, at, one));
    }
    let scrolling = col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(size::FADE).child(sections).scroll_bar(k);
    let credit = col()
        .key("web-credit")
        .w_full()
        .px(space::S3)
        .py(space::S2)
        .gap(2.0)
        .child(wrapped(t::words_from(&fetched.source)).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(text(fetched.url.clone()).font(type_::SMALL).color(k.c.text_faint).truncate());
    col().w_full().h_full().min_h(0.0).child(head).child(scrolling).child(divider(k)).child(credit)
}

/// Artosia's own preview of the picked song, never the page.
fn preview(s: &App, k: &Kit) -> El {
    let Some(web) = s.web.as_ref() else { return el() };
    let inside = match &web.words {
        Words::Nothing => centred(k, "web-pick", t::WEB_PICK_HINT().to_string(), false),
        Words::Loading => centred(k, "web-getting", t::SUGGEST_FETCHING().to_string(), false),
        Words::Failed(why) => centred(k, "web-failed", why.clone(), true),
        Words::Ready(ready) => preview_of(k, ready),
    };
    col().key("web-preview").w_full().h(LIST).radius(k.radius.lg).fill(k.c.layer_02).clip_rounded(k.radius.lg).child(inside)
}

pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(web) = s.web.as_ref() else { return };
    let field = TextInput::new(&web.query)
        .width(None)
        .label(t::CMD_WEB_SONGS())
        .placeholder(t::WEB_FIELD())
        .leading_icon(&rune_kit::glyphs::SEARCH)
        .on_edit(|s: &mut App| match s.web.as_mut() {
            Some(web) => &mut web.query,
            None => &mut s.spare_edit,
        })
        .on_change(|s: &mut App, _: &mut Tx| s.web_typed())
        .on_key(keys)
        .build(k)
        .autofocus();
    let left = col().key("web-left").w(LEFT).gap(space::S2).child(field).child(results(s, k)).child(status(s, k));
    let right = col().key("web-right").grow(1.0).min_w(0.0).gap(space::S2).child(preview(s, k)).child(wrapped(t::WEB_LICENCE()).key("web-licence").font(type_::SMALL).color(k.c.text_faint).wrap().h(NOTE));
    let body = row().key("web-form").w_full().px(space::S4).pb(space::S3).gap(space::S4).child(left).child(right);
    let typed = web.typed();
    let can_search = !typed.is_empty() && !typed.to_ascii_lowercase().starts_with("http");
    let words = typed.to_string();
    let browser = btn(t::WEB_BROWSER()).icon(&icons::GLOBE).help(t::WEB_BROWSER_HELP()).disabled(!can_search).focusable().build(k, move |_, tx| {
        // The address is built here from a fixed https host, so it always passes the check.
        let _ = tx.open_url(&crate::web_songs::browser_search(&words));
    });
    let go = btn(t::PEOPLE_IMPORT()).icon(&icons::IMPORT).primary().hint(t::KEY_IMPORT()).disabled(!web.ready()).focusable().build(k, |s, _| s.web_import()).key("web-go");
    let foot = row().w_full().px(space::S4).py(space::S3).gap(space::S2).center_y().child(browser.key("web-browser")).child(spacer()).child(btn(t::BTN_CANCEL()).hint("Esc").focusable().build(k, |s, _| s.close_web_songs()).key("web-cancel")).child(go);
    Dialog::new("web-songs-dialog", t::CMD_WEB_SONGS()).open(true).width(WIDTH).fit().on_close(|s: &mut App| s.close_web_songs()).child(col().w_full().child(body).child(divider(k)).child(foot)).build(k, layer);
}
