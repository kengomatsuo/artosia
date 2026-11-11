//! Artosia's operator window. `cargo run -p artosia`; `-- --snap out.png` renders it headless
//! with the practice library, and `--scene <name>` picks a screen (see `SCENES`).
//!
//! Flags: `--data <folder>` uses that library folder, `--practice` uses the practice library
//! in memory with the projector off, `--no-output` keeps the projector off, `--query <text>`
//! opens the search with that text, `--background <file>` puts a picture or video behind the
//! text, `--import-easislides` brings EasiSlides over at start, and
//! `--import-openlyrics <folder>` adds or updates every OpenLyrics song in a folder.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod backdrops;
mod bibles;
mod data;
mod languages;
mod learning;
mod link;
mod palette;
mod plans;
mod starts;
mod speech_model;
mod state;
mod stream;
mod strings;
#[cfg(all(test, feature = "speech"))]
mod heard_test;
#[cfg(test)]
mod strings_snapshot;
mod suggest;
mod theme;
mod ui;
mod update;
mod why;

use artosia_core::song::Song;
use artosia_slides::frame::ToOutput;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use rune::{AppSpec, ImageCache, Key, KeyEvent, Titlebar, Tx, Waker};

use palette::Scope;
use state::{App, Sheet};

fn setup(s: &mut App, waker: Waker) {
    s.waker = Some(waker.clone());
    s.images = Some(ImageCache::new(waker.clone()));
    let (first, second) = (s.settings.bible.clone(), s.settings.second_bible.clone());
    s.want_bible(&first);
    s.want_bible(&second);
    // The projector stays off while practising and in headless runs.
    if s.folder.is_some() && !rune::is_headless() && !data::has_flag("--no-output") {
        if update::just_updated() {
            // A projector left running by the old version would not understand the new one.
            link::quit_stale_output();
        }
        let options = s.output_options();
        s.link.start(waker, options);
    }
    if s.folder.is_some() {
        update::check(s.inbox.clone(), s.waker.clone(), false);
    }
    if s.settings.remote_on && s.folder.is_some() && !rune::is_headless() {
        s.set_remote(true);
    }
    // The first start on a library, and every practice run, begins with the tour. A headless
    // run never does unless a scene asks for it.
    if !rune::is_headless() && (!s.settings.toured || s.folder.is_none()) {
        s.start_tour();
    }
}

fn poll(s: &mut App, clock: f64, tx: &mut Tx) -> bool {
    // The tour's Next button takes focus once the card is on screen.
    if s.tour_focus {
        s.tour_focus = false;
        tx.focus("tour-next", false);
    }
    s.poll(clock, tx)
}

/// The keys of the window. A focused text field has already had its turn by the time a key
/// gets here, except when nothing has focus, which is the usual state while showing.
fn on_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    // The tour takes every key, so none of them reaches the projector.
    if s.tour.is_some() {
        s.tour_key(&ev.key);
        return true;
    }
    // A modifier on its own. Shift let go shows the slide picked while it was held. This is a
    // key event because Rune's `App::on_modifiers` is never called in a single-window app:
    // `src/window.rs` at the pinned rev wires `modifier_keys` and leaves `on_mods` unset.
    // Which of Ctrl and Shift is down is kept for the hint bar; every key event carries it.
    // A headless script never sends a modifier's release, so there the badge would stay lit
    // after every chord; the scenes that show it set it themselves.
    let held = if rune::is_headless() { (s.held_ctrl, s.held_shift) } else { (ev.mods.ctrl || ev.mods.cmd, ev.mods.shift) };
    let held_changed = held != (s.held_ctrl, s.held_shift);
    (s.held_ctrl, s.held_shift) = held;
    if let Key::Modifier { modifier, pressed } = &ev.key {
        let shift = matches!(modifier, rune::Modifier::ShiftLeft | rune::Modifier::ShiftRight);
        if shift && !pressed && !ev.mods.shift && s.release_pick() {
            follow(s, tx);
            return true;
        }
        // A change is handled so the window draws the new hints.
        return held_changed;
    }
    let focused = tx.focused().is_some();
    let editing = s.editor.is_some() || s.notices.is_some();
    match &ev.key {
        Key::Command('k') => {
            s.open_palette(Scope::All, "");
            return true;
        }
        Key::Command('s') if s.editor.is_some() => {
            s.save_editor();
            tx.blur();
            return true;
        }
        Key::Command('s') if s.notices.is_some() => {
            s.save_notice();
            return true;
        }
        Key::Up | Key::Down if s.notices.is_some() && (ev.mods.ctrl || ev.mods.cmd) && !s.palette.open => {
            s.step_notice(if ev.key == Key::Up { -1 } else { 1 });
            return true;
        }
        Key::Command('e') if !editing && !s.palette.open => {
            s.edit_open();
            return true;
        }
        // New means a new one of what is being edited: a card among the announcements, a song anywhere else.
        Key::Command('n') if s.notices.is_some() && !s.palette.open => {
            s.new_notice("", "");
            return true;
        }
        Key::Command('n') if !editing && !s.palette.open => {
            s.edit_new_song(Song::default());
            return true;
        }
        Key::Command(',') => {
            s.sheet = if s.sheet == Sheet::Settings { Sheet::None } else { Sheet::Settings };
            tx.focus(crate::ui::SETTINGS_FIRST, true);
            return true;
        }
        Key::Command('/') => {
            s.sheet = if s.sheet == Sheet::Keys { Sheet::None } else { Sheet::Keys };
            return true;
        }
        Key::Function(1) => {
            s.toggle_text();
            return true;
        }
        Key::Function(2) => {
            s.toggle_black();
            return true;
        }
        Key::Function(3) => {
            s.toggle_logo();
            return true;
        }
        Key::Function(6) => {
            s.toggle_live();
            return true;
        }
        // Ctrl and a digit put the Preview ring on that section.
        Key::Command(digit) if digit.is_ascii_digit() && !editing && !s.palette.open && s.sheet == Sheet::None => {
            let digit = digit.to_digit(10).unwrap_or(1) as usize;
            s.select_section(if digit == 0 { 9 } else { digit - 1 });
            follow(s, tx);
            return true;
        }
        Key::Escape if s.palette.open => {
            s.palette_escape();
            return true;
        }
        Key::Escape if s.editor.is_some() && s.sheet == Sheet::None => {
            if !s.dismiss_suggestions() {
                s.close_editor();
            }
            tx.blur();
            return true;
        }
        Key::Escape if s.notices.is_some() && s.sheet == Sheet::None => {
            s.close_announcements();
            tx.blur();
            return true;
        }
        _ => {}
    }
    if s.palette.open {
        // A key typed before the search field took focus still belongs to the query.
        if let (Key::Text(typed), false) = (&ev.key, focused) {
            s.palette.query.insert(typed);
            s.palette.picked = 0;
            s.search();
            return true;
        }
        return false;
    }
    if focused || editing || s.sheet != Sheet::None {
        return false;
    }
    // The arrows are EasiSlides': Up and Down are the slide before and after, Left the first
    // and Right the last, all on the projector. With Shift held they pick a slide of the live
    // song and letting Shift go shows it. With Ctrl held they move the ring in the Preview
    // pane and leave it there, as they do while nothing is live; Enter shows its slide.
    let plain = !ev.mods.cmd && !ev.mods.alt && !ev.mods.ctrl;
    let ring = ev.mods.ctrl || ev.mods.cmd;
    let picking = ev.mods.shift && !ring && !ev.mods.alt;
    if ev.key == Key::Escape && s.cancel_pick() {
        return true;
    }
    // A headless script sends no lone modifier, so a key without Shift says the same there.
    if !ev.mods.shift && s.release_pick() {
        follow(s, tx);
    }
    let only_select = ring || s.live.is_none();
    match &ev.key {
        Key::Down if ev.mods.alt => {
            if let Some(id) = s.open.as_ref().and_then(|open| open.item) {
                s.shift_plan_item(id, 1);
            }
        }
        Key::Up if ev.mods.alt => {
            if let Some(id) = s.open.as_ref().and_then(|open| open.item) {
                s.shift_plan_item(id, -1);
            }
        }
        Key::Down if picking => s.pick_slide(1),
        Key::Up if picking => s.pick_slide(-1),
        Key::Left | Key::Home if picking => s.pick_slide_edge(false),
        Key::Right | Key::End if picking => s.pick_slide_edge(true),
        Key::Down if only_select => drop(s.select(1)),
        Key::Up if only_select => drop(s.select(-1)),
        Key::Left | Key::Home if only_select => drop(s.select_edge(false)),
        Key::Right | Key::End if only_select => drop(s.select_edge(true)),
        // A presenter's clicker sends Page Down and Page Up.
        Key::Down | Key::PageDown => s.next(),
        Key::Text(typed) if typed == " " => s.next(),
        Key::Up | Key::PageUp => s.previous(),
        Key::Left | Key::Home => s.go_to_edge(false),
        Key::Right | Key::End => s.go_to_edge(true),
        Key::Enter if ev.mods.alt => return false,
        // Shift Enter adds what Preview holds to the plan, as it does in the search, and shows
        // nothing: a tester who knew it from the search put a song in front of the room.
        Key::Enter if ev.mods.shift => {
            if s.open.is_some() && !s.open_in_plan() {
                s.add_open_to_plan();
            }
        }
        Key::Enter => s.enter(),
        // Ctrl Tab is the next plan, as it is the next tab of a Windows program; Tab alone is
        // the next item of the open plan.
        Key::Tab if ev.mods.ctrl || ev.mods.cmd => s.walk_plans(!ev.mods.shift),
        Key::Tab => s.walk_plan(!ev.mods.shift),
        // Ctrl M: the Preview song starts at the ring's section (docs/operator.md).
        Key::Command('m') => s.toggle_start_here(),
        Key::Function(4) => s.open_last_heard(),
        Key::Function(5) => s.open_announcements(),
        Key::Escape if s.live.is_some() && !s.open_is_live() => s.open_live(),
        Key::Delete => {
            if let Some(id) = s.open.as_ref().and_then(|open| open.item) {
                s.remove_plan_item(id);
            }
        }
        // Back to what was shown before, in Preview: the room sees nothing until Enter.
        Key::Backspace if plain && !ev.mods.shift => {
            if !s.open_earlier() {
                return false;
            }
        }
        Key::Command('z') => s.undo(),
        Key::Text(typed) if typed == "?" => s.sheet = Sheet::Keys,
        // A digit jumps inside the live song. With no sections to jump to it starts a search,
        // so a hymn number can still be typed straight in.
        Key::Text(typed) if plain && typed.len() == 1 && typed.chars().all(|c| c.is_ascii_digit()) && s.target_deck().is_some_and(|deck| state::section_starts(deck).len() > 1) => {
            let digit = typed.parse::<usize>().unwrap_or(1);
            s.jump_to_section(if digit == 0 { 9 } else { digit - 1 });
        }
        Key::Text(typed) if typed == "/" => s.open_palette(Scope::All, ""),
        // A letter that names a section of the song the keys act on goes there. Any other
        // letter starts a search, and so does every letter once this is switched off.
        Key::Text(typed) if plain && s.settings.letter_keys && typed.chars().count() == 1 && typed.chars().next().is_some_and(|c| c.is_ascii_alphabetic() && s.marker_letters().contains(&c.to_ascii_uppercase())) => {
            let letter = typed.chars().next().unwrap_or('C').to_ascii_uppercase();
            s.jump_to_marker(letter);
        }
        Key::Text(typed) if plain && typed.chars().all(char::is_alphanumeric) && !typed.is_empty() => {
            // Typing anywhere starts a search.
            s.open_palette(Scope::All, typed);
        }
        _ => return false,
    }
    follow(s, tx);
    true
}

/// Keeps the selector and the live slide in view in their lists.
pub(crate) fn follow(s: &App, tx: &mut Tx) {
    if let Some(open) = &s.open {
        tx.scroll_into_view(format!("slide-{}", open.cursor));
    }
    if let Some(live) = &s.live {
        tx.scroll_into_view(format!("live-slide-{}", s.live_pick.unwrap_or(live.index)));
    }
}

type Scene = (&'static str, fn(&mut App));

/// Words written for the scenes that show the section suggestions: no marks, a chorus sung three times.
const MARKS_DEMO: &str = "Lantern Song\n\nCarry the lantern through the night\nKeep the small flame burning bright\n\nShine, shine on the road ahead\nShine till the morning comes\n\nWalk beside me down the hill\nHold my hand and hold me still\n\nShine, shine on the road ahead\nShine till the morning comes\n\nCold wind over the sleeping sea\nStars come out for you and me\n\nShine, shine on the road ahead\nShine till the morning comes\n";

/// Screens a headless run can render: `--scene <name> --snap out.png`.
const SCENES: &[Scene] = &[
    ("start", |_| {}),
    ("song", |s| {
        s.open_song(0, None);
        s.show(2);
    }),
    ("preparing", |s| {
        s.open_song(0, None);
        s.show(1);
        s.open_song(1, None);
    }),
    ("passage", |s| {
        if let Some(id) = s.plan.items.last().map(|item| item.id) {
            s.open_plan_item(id);
            s.show(0);
        }
    }),
    ("black", |s| {
        s.open_song(0, None);
        s.show(2);
        s.toggle_black();
    }),
    ("search", |s| {
        s.open_palette(Scope::All, "rivr mercy");
    }),
    ("search-lyrics", |s| {
        s.open_palette(Scope::All, "anchor deep");
    }),
    ("search-bible", |s| {
        s.open_palette(Scope::All, "joh 3 16-18");
    }),
    ("search-words", |s| {
        s.open_palette(Scope::Bible, "shepherd shall not want");
    }),
    ("browse", |s| {
        s.open_palette(Scope::All, "");
    }),
    ("browse-bible", |s| {
        s.open_palette(Scope::Bible, "");
    }),
    ("nothing-found", |s| {
        s.open_palette(Scope::Songs, "zzzqqq");
    }),
    ("editor", |s| {
        s.open_song(0, None);
        s.edit_open_song();
    }),
    ("new-song", |s| {
        s.song_from_clipboard("Lantern Song\n\nVerse 1\nCarry the lantern through the night\nKeep the small flame burning bright\n\nChorus\nShine, shine on the road ahead\nShine till the morning comes\n");
    }),
    // A pasted song with no marks: the strip proposes its sections. The second scene has the
    // caret in a verse, which is offered the other kinds too.
    ("editor-marks", |s| {
        s.song_from_clipboard(MARKS_DEMO);
        if let Some(editor) = s.editor.as_mut() {
            editor.lyrics.focused = false;
        }
    }),
    ("editor-marks-caret", |s| {
        s.song_from_clipboard(MARKS_DEMO);
        if let Some(editor) = s.editor.as_mut() {
            let at = editor.lyrics.text.find("Walk beside").map_or(0, |at| at + 5);
            editor.lyrics.set_caret(at);
            editor.lyrics.focused = true;
        }
    }),
    ("look", |s| s.sheet = Sheet::Looks),
    ("settings", |s| s.sheet = Sheet::Settings),
    ("look-styled", |s| {
        s.sheet = Sheet::Looks;
        let styled = artosia_slides::look::Look { ink: [255, 214, 0], font: "Georgia".into(), outline: artosia_slides::look::Outline::Thick, ..s.look() };
        s.set_look(styled);
    }),
    // The owner's look: a camera behind words at the bottom. The frame is a made-up gradient
    // handed over as if the projector had sent it, so both previews show it under the words.
    ("camera-background", |s| {
        let look = artosia_slides::look::Look { background: artosia_slides::look::Background::Camera(String::new()), place: artosia_slides::look::Place::Bottom, shadow: true, ..s.look() };
        s.set_look(look);
        s.open_song(0, None);
        s.show(2);
        s.open_song(1, None);
        let (w, h) = (256u32, 144u32);
        let rgba = (0..w * h).flat_map(|i| [(i % w * 255 / w) as u8, (i / w * 255 / h) as u8, 200 - (i % w * 150 / w) as u8, 255]).collect();
        s.link.inject_live_picture(w, h, rgba);
    }),
    ("screens", |s| s.sheet = Sheet::Screens),
    ("screens-two", |s| {
        s.demo_screens = true;
        s.settings.warp.corners[1] = [0.94, 0.05];
        s.warp_corner = 1;
        s.sheet = Sheet::Screens;
    }),
    ("remote", |s| s.sheet = Sheet::Remote),
    ("announcements", |s| s.edit_announcements()),
    ("announcement-passage", |s| s.demo_notices("passage")),
    ("announcement-passage-no-bible", |s| s.demo_notices("passage-no-bible")),
    ("announcement-birthdays", |s| s.demo_notices("birthdays")),
    ("keys", |s| s.sheet = Sheet::Keys),
    ("tour-plan", |s| s.tour = Some(0)),
    ("tour-preview", |s| s.tour = Some(1)),
    ("tour-show", |s| s.tour = Some(2)),
    ("tour-live", |s| s.tour = Some(3)),
    ("tour-search", |s| s.tour = Some(4)),
    ("listen", |s| s.sheet = Sheet::Listen),
    ("listen-tempo", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.listen_level.store(0.55f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.tempo = Some((artosia_listen::Tempo { bpm: 92.0, confidence: 0.7, period: 60.0 / 92.0, since_beat: 0.0 }, 0.0));
    }),
    ("listen-quiet", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.listen_level.store(0.1f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.settings.beat_blink = true;
    }),
    ("stream", |s| s.sheet = Sheet::Stream),
    ("backdrops", |s| s.demo_backdrops("ready")),
    ("backdrops-picked", |s| s.demo_backdrops("picked")),
    ("backdrops-picked-video", |s| s.demo_backdrops("picked-video")),
    ("backdrops-downloaded", |s| s.demo_backdrops("downloaded")),
    ("backdrops-downloading", |s| s.demo_backdrops("downloading")),
    ("backdrops-offline", |s| s.demo_backdrops("offline")),
    ("backdrops-empty", |s| s.demo_backdrops("empty")),
    ("backdrops-category", |s| s.demo_backdrops("category")),
    ("bibles", |s| s.demo_bibles("ready")),
    ("bibles-picked", |s| s.demo_bibles("picked")),
    ("bibles-picked-file", |s| s.demo_bibles("picked-file")),
    ("bibles-picked-silent", |s| s.demo_bibles("picked-silent")),
    ("bibles-picked-reading", |s| s.demo_bibles("picked-reading")),
    ("bibles-loading", |s| s.demo_bibles("loading")),
    ("bibles-failed", |s| s.demo_bibles("failed")),
    ("bibles-empty", |s| s.demo_bibles("empty")),
    ("search-command", |s| {
        s.open_palette(Scope::All, "black");
    }),
    ("search-inside", |s| {
        s.open_palette(Scope::All, "morning");
        s.palette_tab();
    }),
    ("new-song-title", |s| {
        s.edit_new_song(Song { title: "River".into(), ..Song::default() });
    }),
    ("preparing-chorus", |s| {
        s.open_song(0, None);
        s.show(1);
        s.open_song(1, None);
        s.select(1);
    }),
    ("ctrl-held", |s| {
        s.open_song(0, None);
        s.show(1);
        s.open_song(1, None);
        s.select(1);
        s.held_ctrl = true;
    }),
    ("shift-held", |s| {
        s.open_song(0, None);
        s.show(1);
        s.pick_slide(2);
        s.held_shift = true;
    }),
    ("off-air", |s| {
        s.start_service();
        s.next();
        s.open_song(1, None);
        s.toggle_live();
    }),
    ("adding", |s| {
        s.open_palette_adding(Scope::Songs);
        s.set_query("gr");
    }),
    ("add-menu", |s| s.add_menu = true),
    ("announcements-live", |s| {
        s.open_announcements();
        s.show(0);
    }),
    ("countdown", |s| {
        s.start_countdown();
        s.message.replace_text("Parent of child 12, please come to the nursery");
        s.message_on = true;
    }),
    ("empty", |s| {
        s.plan.items.clear();
        s.songs.clear();
        s.index = artosia_core::search::SongIndex::build(&[]);
    }),
    // The search after a made-up history: groups under the two commands, and the reason chips.
    ("learned-browse", |s| {
        s.plan.items.clear();
        if let Some(id) = s.songs.first().map(|song| song.id) {
            s.plan.push(artosia_core::plan::PlanEntry::song(id, s.songs[0].title.clone()));
        }
        s.seed_demo_history();
        s.open_song(0, None);
        s.show(0);
        s.open_palette(Scope::All, "");
    }),
    ("learned-search", |s| {
        s.seed_demo_history();
        s.open_palette(Scope::All, "mor");
    }),
    // The suggestion strip, drawn over the top of the Preview pane: what the listener heard
    // (made-up words of two practice songs), what usually follows, and the lost-words line.
    ("suggest-heard", |s| {
        s.plan.items.clear();
        s.open_song(s.songs.iter().position(|song| song.title == "Morning on the Hill").unwrap_or(0), None);
        s.show(0);
        s.heard_final("down by the river we gather mercy flowing like the rain we will not be moved we will not be moved you are with us still".into());
    }),
    ("suggest-follows", |s| {
        s.plan.items.clear();
        s.hearing.demo = true;
        s.seed_demo_history();
        s.open_song(0, None);
        s.show(0);
    }),
    ("suggest-lost", |s| {
        s.plan.items.clear();
        s.hearing.demo = true;
        s.seed_demo_history();
        s.open_song(s.songs.iter().position(|song| song.title == "Morning on the Hill").unwrap_or(0), None);
        s.show(0);
        for words in ["first light breaks over the valley we wake and lift our voices", "lantern carpet violin tractor window pencil", "marble ladder compass orchard blanket whistle"] {
            s.now += 12.0;
            s.heard_final(words.into());
        }
    }),
    // Several plans: the list under the title with two more plans kept, and the rename dialog.
    ("plans", |s| s.demo_plans()),
    ("plan-new", |s| {
        s.demo_plans();
        s.begin_new_plan();
    }),
    ("plan-rename", |s| {
        s.demo_plans();
        s.begin_edit(s.plan.id);
    }),
    // A plan with a look of its own: the dialog on it, and the list with that plan marked.
    ("look-plan", |s| {
        s.demo_plans();
        s.use_own_look();
        let styled = artosia_slides::look::Look { ink: [255, 214, 0], outline: artosia_slides::look::Outline::Thick, ..s.look() };
        s.set_look(styled);
        s.sheet = Sheet::Looks;
    }),
    ("plans-own-look", |s| {
        s.demo_plans();
        s.use_own_look();
    }),
    ("search-plan", |s| {
        s.demo_plans();
        s.open_palette(Scope::All, "youth");
    }),
    // A medley: the second song of the plan is marked to begin at its chorus, and waits there.
    ("plan-start", |s| {
        if let Some((first, second, chorus)) = s.demo_medley() {
            s.set_start(second, Some(chorus));
            s.open_plan_item(first);
            s.show(0);
        }
    }),
    // The same song opened by hand, the ring on its chorus and nothing marked yet.
    ("plan-start-offer", |s| {
        if let Some((_, second, _)) = s.demo_medley() {
            s.open_plan_item(second);
            s.select(1);
        }
    }),
    // The row's menu, with the sections to start at.
    ("plan-start-menu", |s| {
        if let Some((first, second, _)) = s.demo_medley() {
            s.open_plan_item(first);
            s.plan_menu = Some(second);
        }
    }),
    ("toast", |s| {
        if let Some(id) = s.plan.items.get(2).map(|item| item.id) {
            s.remove_plan_item(id);
        }
    }),
];

/// The port one Artosia holds so a second start finds it. The output process has the next one down.
const INSTANCE_PORT: u16 = 47824;
const INSTANCE_WORD: &str = "artosia";

enum Instance {
    /// This is the only Artosia on the library; the listener answers later starts.
    First(TcpListener),
    /// Another Artosia runs and has been asked to come to the front.
    Second,
    /// Not checked: a headless run, a practice run, or a library named by hand.
    Free,
}

/// Two Artosias on one library would fight over the projector and overwrite each other's
/// plan, so the second start hands over to the first and leaves.
fn claim_instance() -> Instance {
    if rune::is_headless() || data::has_flag("--practice") || data::flag_value("--data").is_some() || std::env::var_os("ARTOSIA_DATA").is_some() {
        return Instance::Free;
    }
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, INSTANCE_PORT));
    if let Ok(listener) = TcpListener::bind(address) {
        return Instance::First(listener);
    }
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(500)) else { return Instance::Free };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.write_all(b"show\n");
    let mut answer = String::new();
    let _ = BufReader::new(stream).read_line(&mut answer);
    // Anything else on that port is not Artosia, and must not stop it from opening.
    if answer.trim() == INSTANCE_WORD { Instance::Second } else { Instance::Free }
}

fn main() {
    let instance = claim_instance();
    if matches!(instance, Instance::Second) {
        return;
    }
    // An update downloaded last time is swapped in before any window opens. The new program
    // is then started, and this one lets go of its claim and leaves.
    if update::apply_before_window() {
        drop(instance);
        return;
    }
    // The mark on the window, the taskbar and Alt+Tab (docs/brand.md).
    let _ = rune::set_app_icon(include_bytes!("../../../assets/brand/artosia-icon-256.png").to_vec());
    let spec = AppSpec {
        min_size: Some((1120.0, 640.0)),
        scenes: SCENES,
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        ..AppSpec::new("Artosia", (1366.0, 768.0), App::load, ui::view)
    };
    // The window opens filling its screen. At its own size Windows placed it 128 points in from
    // the corner, and its foot hung under the taskbar of a 1080 point screen and off a 768
    // point one, where the status bar and the lowest tiles could not be pressed (owner,
    // 2026-11-11: "summoning the window partly out of bounds").
    let app = rune::App::new(spec).titlebar(Titlebar::new(theme::size::TOPBAR)).maximized(true).modifier_keys().on_close(|s: &mut App, _: &mut Tx| {
        // Closing the operator on purpose closes the projector too, and ends a stream cleanly;
        // a crash never gets here, and then both go on.
        if let Some(stream) = s.stream.as_mut() {
            stream.stop();
        }
        s.link.send(ToOutput::Quit);
    });
    if let Instance::First(listener) = instance {
        let handle = app.handle();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut line = String::new();
                let Ok(reader) = stream.try_clone() else { continue };
                let _ = reader.set_read_timeout(Some(Duration::from_millis(800)));
                let _ = BufReader::new(reader).read_line(&mut line);
                if line.trim() == "show" {
                    let mut stream = stream;
                    let _ = stream.write_all(format!("{INSTANCE_WORD}\n").as_bytes());
                    handle.open();
                }
            }
        });
    }
    app.run();
}
