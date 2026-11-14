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
mod calendar;
mod data;
mod firewall;
mod giving;
// GUIDE1: the guide engine; the window wires it in task G3.
#[allow(dead_code)]
mod guide;
mod languages;
mod learning;
mod link;
mod onboard;
mod palette;
mod parts;
mod people;
mod plans;
mod pptx_import;
mod recordings;
mod run;
mod starts;
mod templates;
mod song_languages;
mod shortcut_icon;
mod speech_model;
mod state;
mod store;
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
mod web_songs;
mod why;

use artosia_core::song::Song;
use artosia_slides::frame::ToOutput;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use rune::{AppSpec, ImageCache, Key, KeyEvent, Titlebar, Tx, Waker};

use palette::Scope;
use state::{App, Sheet, Tab};

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
    if s.settings.remote_on && s.folder.is_some() && !rune::is_headless() {
        s.resume_remote();
    }
    // A new church (a library with no settings file before this start) begins with the first
    // run, whose last page offers the tour. The first start on a library that has settings, and
    // every practice run, begins with the tour. A headless run never does unless a scene asks.
    if s.first_run {
        s.start_first_run();
    } else if !rune::is_headless() && (!s.settings.toured || s.folder.is_none()) {
        s.start_tour();
    } else if !rune::is_headless() && s.has_news() {
        // An update brought new features: they are shown once, after the tour has been seen.
        s.news = Some(0);
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
/// What Command and a letter switches on a Mac, where the F keys of the same switches need Fn
/// held: T the text, B black, L the logo, O the projector (`docs/macos.md` has the sources).
fn mac_switch(letter: char) -> Option<fn(&mut App)> {
    Some(match letter {
        't' => App::toggle_text,
        'b' => App::toggle_black,
        'l' => App::toggle_logo,
        'o' => App::toggle_live,
        _ => return None,
    })
}

fn on_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    // The tour takes every key, so none of them reaches the projector.
    if s.tour.is_some() {
        s.tour_key(&ev.key);
        return true;
    }
    // So does the first run, which is open before the tour and never together with it.
    if s.onboard.is_some() {
        return s.onboard_key(ev, tx);
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
    // The song editor lives in the plan's tab and keeps its words while another tab is open.
    let song_editor = s.editor.is_some() && s.tab == Tab::Plan;
    let editing = song_editor || s.notices.is_some();
    match &ev.key {
        Key::Command('k') => {
            s.open_palette(Scope::All, "");
            return true;
        }
        // The Songs from the internet dialog prints Ctrl Enter on Import, wherever its focus is.
        Key::Enter if (ev.mods.ctrl || ev.mods.cmd) && s.web.is_some() => {
            s.web_import();
            return true;
        }
        // The next and the previous window, as the next sheet of Excel and the next tab of
        // Chrome and Edge on Windows. Ctrl and a digit stay the Preview ring's.
        Key::PageDown | Key::PageUp if (ev.mods.ctrl || ev.mods.cmd) && !ev.mods.alt && !s.palette.open && s.sheet == Sheet::None => {
            s.open_tab(s.tab.step(if ev.key == Key::PageDown { 1 } else { -1 }));
            tx.blur();
            return true;
        }
        Key::Command('s') if song_editor => {
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
        // Ctrl Shift N is a new plan, beside Ctrl N for a new song; the plus of the list of
        // plans prints it.
        Key::Command('n') if ev.mods.shift && !editing && !s.palette.open && s.sheet == Sheet::None => {
            s.open_tab(Tab::Plan);
            s.begin_new_plan();
            return true;
        }
        Key::Command('n') if s.notices.is_some() && !s.palette.open => {
            s.new_notice("", "");
            return true;
        }
        // On the Calendar tab New means an event, as a card on the announcements' tab.
        Key::Command('n') if s.tab == Tab::Calendar && s.sheet == Sheet::None && !s.palette.open && s.calendar.form.is_none() && s.calendar.manage.is_none() => {
            s.new_event_here();
            return true;
        }
        // And a person on the Members tab.
        Key::Command('n') if s.tab == Tab::People && s.sheet == Sheet::None && !s.palette.open && s.members.form.is_none() && s.members.import.is_none() => {
            s.new_person();
            return true;
        }
        // And a gift on the Offerings tab.
        Key::Command('n') if s.tab == Tab::Giving && s.sheet == Sheet::None && !s.palette.open && s.giving.form.is_none() && s.giving.manage.is_none() && s.giving.account.is_none() => {
            s.new_gift();
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
        // A Mac's own keys for the Live pane's switches (`ui::widgets::switch_key` prints them).
        Key::Command(letter) if cfg!(target_os = "macos") && mac_switch(*letter).is_some() => {
            if let Some(switch) = mac_switch(*letter) {
                switch(s);
            }
            return true;
        }
        // Ctrl and a digit put the Preview ring on that section.
        Key::Command(digit) if digit.is_ascii_digit() && !editing && !s.palette.open && s.sheet == Sheet::None && s.tab == Tab::Plan => {
            let digit = digit.to_digit(10).unwrap_or(1) as usize;
            s.select_section(if digit == 0 { 9 } else { digit - 1 });
            follow(s, tx);
            return true;
        }
        // A toast that offers Undo prints Ctrl Z, so the key takes it back over a dialog too
        // (Delete all in Listen left the key dead while its toast was up). A text field uses
        // the key for its own typing first and only what it leaves arrives here.
        Key::Command('z') if s.sheet != Sheet::None && s.toast.as_ref().is_some_and(|toast| toast.undo.is_some()) => {
            s.undo();
            return true;
        }
        Key::Escape if s.palette.open => {
            s.palette_escape();
            return true;
        }
        // The colour picker prints Esc on its Cancel, wherever the focus is.
        Key::Escape if s.colour.open.is_some() && s.sheet == Sheet::None => {
            s.finish_colour(false);
            tx.blur();
            return true;
        }
        Key::Escape if song_editor && s.sheet == Sheet::None => {
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
    // The projector's keys act where Preview and Live are in view, and nowhere else.
    if focused || editing || s.web.is_some() || s.sheet != Sheet::None || s.tab != Tab::Plan {
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

/// The place of the practice library's song in three languages.
fn three_language_song(s: &App) -> usize {
    s.songs.iter().position(|song| song.translations.len() == 2).unwrap_or(0)
}

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
    // LYRICSEDIT1: a song's second part open for writing, its kinds offered.
    ("editor-part", |s| {
        s.open_song(0, None);
        s.edit_open_song();
        s.open_part(1);
        if let Some(editor) = s.editor.as_mut() {
            editor.part_menu = Some(1);
        }
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
    // A song in several languages (board card MULTI1): the editor's chips and picker, the dialog
    // from a plan and from Settings, the Live pane's chips and one language alone, a search hit
    // found by a line in another language.
    ("editor-languages", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.edit_open_song();
        if let Some(editor) = s.editor.as_mut() {
            editor.show(1);
            editor.lyrics.focused = false;
        }
    }),
    ("editor-language-picker", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.edit_open_song();
        s.open_language_pick(crate::song_languages::PickFor::AddToSong);
    }),
    ("editor-language-empty", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.edit_open_song();
        if let Some(editor) = s.editor.as_mut() {
            editor.add_language("ko");
            editor.lyrics.focused = false;
        }
    }),
    ("languages-plan", |s| {
        s.demo_plans();
        s.use_own_languages();
        s.set_languages(vec!["en".into(), "id".into()]);
        s.open_languages(true);
    }),
    ("languages-settings", |s| s.open_languages(false)),
    ("languages-picker", |s| {
        s.open_languages(false);
        s.open_language_pick(crate::song_languages::PickFor::Slide);
    }),
    ("settings-songs-languages", |s| s.sheet = Sheet::Settings),
    ("live-languages", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.show(0);
    }),
    ("live-language-solo", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.show(0);
        s.toggle_only_language("en");
    }),
    ("search-other-language", |s| {
        s.open_palette(Scope::All, "thank gladly");
    }),
    ("search-only-language", |s| {
        let at = three_language_song(s);
        s.open_song(at, None);
        s.show(0);
        s.open_palette(Scope::All, "alone");
    }),
    ("look", |s| s.open_tab(crate::state::Tab::Look)),
    ("settings", |s| s.sheet = Sheet::Settings),
    // Settings with several Bibles installed: each choice reads as people call that Bible,
    // never as its file is named. The names are made up.
    ("settings-bibles", |s| {
        let file = |code: &str, short: &str, name: &str| crate::state::BibleFile { code: code.into(), short: short.into(), name: name.into(), path: None };
        s.bible_files = vec![file("KJV", "KJV", "King James Version"), file("DEMOINDONESIANONE", "TC", "Alkitab Terjemahan Contoh"), file("DEMOA", "ACL", "Alkitab Contoh Lengkap")];
        s.settings.bible = "DEMOINDONESIANONE".into();
        s.settings.second_bible = "KJV".into();
        s.sheet = Sheet::Settings;
    }),
    // The language drop-down of Settings, open: its list must stay inside the dialog and the window.
    ("settings-language", |s| {
        s.sheet = Sheet::Settings;
        s.language_open = true;
    }),
    ("look-styled", |s| {
        s.open_tab(crate::state::Tab::Look);
        let styled = artosia_slides::look::Look { ink: [255, 214, 0], font: "Georgia".into(), outline: artosia_slides::look::Outline::Thick, ..s.look() };
        s.set_look(styled);
    }),
    // The owner's look: a camera behind words at the bottom. The frame is a made-up gradient
    // handed over as if the projector had sent it, so both previews show it under the words.
    // Design with a camera on the network as the background: the address in its field, Use lit.
    ("design-network-camera", |s| {
        let look = artosia_slides::look::Look { background: artosia_slides::look::Background::Camera("rtsp://192.168.1.20:8554/live".into()), shadow: true, ..s.look() };
        s.set_look(look);
        s.open_tab(crate::state::Tab::Look);
    }),
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
    // The two states of the firewall step, drawn without a server: nothing here listens.
    ("remote-firewall-asking", |s| {
        s.firewall = firewall::Standing::Asking;
        s.sheet = Sheet::Remote;
    }),
    ("remote-firewall-refused", |s| {
        s.firewall = firewall::Standing::Refused;
        s.sheet = Sheet::Remote;
    }),
    // UIOVER1: the overhaul's three directions, seven pages each.
    // UIOVER1: each window of the tab bar.
    // OVTEMPL1: the editor on a schedule card, in the three styles.
    ("tab-notices", |s| s.demo_notices("schedule")),
    ("notice-style-ledger", |s| {
        s.demo_notices("schedule");
        s.set_notice_style(artosia_core::deck::NoticeStyle::Ledger);
    }),
    ("notice-style-band", |s| {
        s.demo_notices("schedule");
        s.set_notice_style(artosia_core::deck::NoticeStyle::Band);
    }),
    ("tab-calendar", |s| s.open_tab(Tab::Calendar)),
    // OVCAL1: the week, the dialog on the weekly service, a new event, and the schedule card that fills itself.
    // The week that runs from November into December, so the title names both months.
    ("calendar-week", |s| {
        s.open_tab(Tab::Calendar);
        s.calendar.anchor = artosia_core::calendar::Date { year: 2026, month: 11, day: 30 };
        s.calendar_show_week(true);
    }),
    ("calendar-event", |s| {
        s.open_tab(Tab::Calendar);
        if let Some(id) = s.events.iter().find(|event| event.name == "Ibadah Raya 1").map(|event| event.id) {
            s.edit_event(id);
        }
    }),
    ("calendar-new", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        s.new_event(today);
    }),
    // CALREPEAT1: the dialog on a custom interval, every 3 weeks.
    ("calendar-repeat", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        s.new_event(today);
        if let Some(form) = s.calendar.form.as_mut() {
            form.name = rune::TextEdit::new("Cell group");
        }
        s.event_choice(5);
    }),
    // CALEXCEPT1: the question over the dialog, with a changed date the new rule would drop.
    ("calendar-this-or-all", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        let Some(event) = s.events.iter().find(|event| event.name == "Ibadah Raya 1").cloned() else { return };
        let dates = artosia_core::calendar::rule_dates(&event, today, today.plus_days(60));
        let Some(&third) = dates.get(2) else { return };
        s.cancel_date(event.id, dates[0]);
        s.toast = None;
        s.edit_date(event.id, third);
        s.event_choice(2);
        s.save_event();
    }),
    // CALEXCEPT1: a cancelled date stays in the month, struck, until its day has passed.
    ("calendar-cancelled", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        let Some(event) = s.events.iter().find(|event| event.name == "Ibadah Raya 2").cloned() else { return };
        if let Some(&next) = artosia_core::calendar::rule_dates(&event, today, today.plus_days(30)).get(1) {
            s.cancel_date(event.id, next);
            s.toast = None;
        }
    }),
    // CALDRAG1: a chip of the month carried to another day, and the same in the week with its new hour.
    ("calendar-drag", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        let Some(event) = s.events.iter().find(|event| event.name == "Ibadah Raya 1").cloned() else { return };
        if let Some(&from) = artosia_core::calendar::rule_dates(&event, today, today.plus_days(30)).first() {
            s.drag_chip(calendar::CalDrag { id: event.id, original: from, from, start0: event.start, to: from.plus_days(3), start: event.start });
        }
    }),
    ("calendar-drag-week", |s| {
        s.open_tab(Tab::Calendar);
        let today = s.civil_today();
        s.calendar_show_week(true);
        let Some(event) = s.events.iter().find(|event| event.name == "Ibadah Raya 1").cloned() else { return };
        if let Some(&from) = artosia_core::calendar::rule_dates(&event, today, today.plus_days(30)).first() {
            s.calendar.anchor = from;
            s.drag_chip(calendar::CalDrag { id: event.id, original: from, from, start0: event.start, to: from.plus_days(2), start: 19 * 60 + 15 });
        }
    }),
    ("notice-schedule", |s| s.demo_notices("schedule")),
    // SCHEDRANGE1 and BDAYRANGE1: next week's schedule, and a month of birthdays with the past week highlighted in each style.
    ("notice-schedule-next-week", |s| {
        s.demo_notices("schedule");
        s.set_notice_range(artosia_core::window::Range::Week(1));
    }),
    ("notice-birthdays-highlight", |s| s.demo_birthdays_highlight(None)),
    ("notice-birthdays-highlight-ledger", |s| s.demo_birthdays_highlight(Some(artosia_core::deck::NoticeStyle::Ledger))),
    ("notice-birthdays-highlight-band", |s| s.demo_birthdays_highlight(Some(artosia_core::deck::NoticeStyle::Band))),
    // The panel with its quick ranges and a custom highlight; the same with a month picker open.
    ("notice-window-panel", |s| {
        s.demo_birthdays_highlight(None);
        s.set_notice_highlight(Some(artosia_core::window::Range::Custom(artosia_core::calendar::Date { year: 1970, month: 1, day: 1 }, artosia_core::calendar::Date { year: 1970, month: 1, day: 1 })));
    }),
    ("notice-window-picker", |s| {
        s.demo_birthdays_highlight(None);
        s.set_notice_highlight(Some(artosia_core::window::Range::Custom(artosia_core::calendar::Date { year: 1970, month: 1, day: 1 }, artosia_core::calendar::Date { year: 1970, month: 1, day: 1 })));
        s.notice_open_pick(crate::state::Edge::HighlightFrom);
    }),
    ("tab-people", |s| s.open_tab(Tab::People)),
    // OVPEOPLE1: the dialog on one person, the import's column step with a made-up csv, an empty list.
    ("people-edit", |s| {
        s.open_tab(Tab::People);
        if let Some(id) = s.person_at(2).map(|person| person.id) {
            s.edit_person(id);
        }
    }),
    ("people-import", |s| {
        let path = std::env::temp_dir().join("artosia-people-import.csv");
        let rows = ["Nama;Tgl;Bulan;Tahun", "Alfa Beta;5;Januari;1991", "Gamma Delta;17;2;", "Epsilon Zeta;30;2;1980", "Eta Teta;9;11;2012", "Iota Kapa;21;6;1975", "Alfa Gamma;3;9;"];
        let _ = std::fs::write(&path, rows.join("
"));
        s.open_import(&path);
        let _ = std::fs::remove_file(path);
    }),
    ("people-empty", |s| {
        let _ = s.store.replace_people(&[]);
        s.people.clear();
        s.refresh_members();
        s.open_tab(Tab::People);
    }),
    ("tab-giving", |s| s.open_tab(Tab::Giving)),
    // OVGIVING1: an amount being edited in its cell, an empty list, and the two cards that read the data.
    ("giving-edit", |s| {
        s.open_tab(Tab::Giving);
        if let Some(id) = s.gift_at(3).map(|gift| gift.id) {
            s.begin_amount(id);
        }
    }),
    ("giving-empty", |s| {
        for gift in s.gifts.clone() {
            let _ = s.store.delete_gift(gift.id);
        }
        s.gifts.clear();
        s.refresh_giving();
        s.open_tab(Tab::Giving);
    }),
    ("giving-gift", |s| {
        s.open_tab(Tab::Giving);
        if let Some(id) = s.gift_at(1).map(|gift| gift.id) {
            s.edit_gift(id);
        }
    }),
    ("giving-gift-date", |s| {
        s.open_tab(Tab::Giving);
        s.new_gift();
        s.gift_open(crate::giving::Part::Date);
    }),
    ("giving-dates", |s| {
        s.open_tab(Tab::Giving);
        s.giving_open(crate::giving::Part::Date);
    }),
    ("giving-funds", |s| {
        s.open_tab(Tab::Giving);
        s.open_funds();
    }),
    ("giving-account", |s| {
        s.open_tab(Tab::Giving);
        s.edit_account();
    }),
    ("notice-offerings", |s| s.demo_notices("offerings")),
    ("notice-transfer", |s| s.demo_notices("transfer")),
    ("tab-templates", |s| s.open_tab(Tab::Templates)),
    // A church with no events: the schedule tiles say so where their slide would be.
    ("templates-empty-source", |s| {
        s.events.clear();
        s.open_tab(Tab::Templates);
    }),
    // OVRUN1: a plan with a video, a sound and a scene opened into its layers; the video live with
    // its progress line; the dialog a scene is made in.
    ("plan-media", |s| {
        let (_, _, scene) = s.demo_media();
        s.open_plan_item(scene);
    }),
    ("plan-media-live", |s| {
        let (video, _, _) = s.demo_media();
        s.open_plan_item(video);
        s.show(0);
        if let Some(playing) = s.run.playing.as_mut() {
            (playing.position_ms, playing.length_ms) = (82_000, 200_000);
        }
    }),
    ("plan-media-scene", |s| {
        s.demo_media();
        s.open_scene_form();
        s.scene_file(crate::run::Layer::Background, std::path::Path::new("night.png"));
        s.scene_file(crate::run::Layer::Video, std::path::Path::new("closing.mp4"));
        s.scene_file(crate::run::Layer::Music, std::path::Path::new("closing-music.mp3"));
        s.scene_clears(crate::run::Layer::Video, true);
    }),
    // OVPPTX1: the import dialog with both ways, a money table whose total differs, and no renderer.
    // WEBDLG1: the Songs from the internet dialog with made-up results and words, a pasted page, and LRCLIB failing.
    ("web-songs", |s| s.demo_web_songs("results")),
    ("web-songs-pasted", |s| s.demo_web_songs("pasted")),
    ("web-songs-offline", |s| s.demo_web_songs("offline")),
    ("pptx-import", |s| {
        s.open_tab(Tab::Notices);
        s.demo_pptx("made");
    }),
    ("pptx-import-money", |s| {
        s.open_tab(Tab::Notices);
        s.demo_pptx("money");
    }),
    ("pptx-import-none", |s| {
        s.open_tab(Tab::Notices);
        s.demo_pptx("none");
    }),
    ("tab-look", |s| s.open_tab(Tab::Look)),
    // OVCOLOR1: the colour picker open under the words' colour, a colour used before it.
    ("colour-picker", |s| {
        s.open_tab(Tab::Look);
        s.design_part = 1;
        s.settings.recent_colours = vec![[255, 214, 0], [120, 200, 255], [255, 120, 90]];
        s.toggle_colour(crate::ui::colour::ColourFor::Ink);
        s.colour.more = true;
    }),
    ("lyrics-a-p1", |s| s.mock = Some(('x', 1))),
    ("lyrics-a-p2", |s| s.mock = Some(('x', 2))),
    ("lyrics-a-p3", |s| s.mock = Some(('x', 3))),
    ("lyrics-b-p1", |s| s.mock = Some(('y', 1))),
    ("lyrics-b-p2", |s| s.mock = Some(('y', 2))),
    ("lyrics-b-p3", |s| s.mock = Some(('y', 3))),
    ("lyrics-c-p1", |s| s.mock = Some(('z', 1))),
    ("lyrics-c-p2", |s| s.mock = Some(('z', 2))),
    ("lyrics-c-p3", |s| s.mock = Some(('z', 3))),
    ("mock-a-p1", |s| s.mock = Some(('a', 1))),
    ("mock-a-p2", |s| s.mock = Some(('a', 2))),
    ("mock-a-p3", |s| s.mock = Some(('a', 3))),
    ("mock-a-p4", |s| s.mock = Some(('a', 4))),
    ("mock-a-p5", |s| s.mock = Some(('a', 5))),
    ("mock-a-p6", |s| s.mock = Some(('a', 6))),
    ("mock-a-p7", |s| s.mock = Some(('a', 7))),
    ("mock-b-p1", |s| s.mock = Some(('b', 1))),
    ("mock-b-p2", |s| s.mock = Some(('b', 2))),
    ("mock-b-p3", |s| s.mock = Some(('b', 3))),
    ("mock-b-p4", |s| s.mock = Some(('b', 4))),
    ("mock-b-p5", |s| s.mock = Some(('b', 5))),
    ("mock-b-p6", |s| s.mock = Some(('b', 6))),
    ("mock-b-p7", |s| s.mock = Some(('b', 7))),
    ("mock-c-p1", |s| s.mock = Some(('c', 1))),
    ("mock-c-p2", |s| s.mock = Some(('c', 2))),
    ("mock-c-p3", |s| s.mock = Some(('c', 3))),
    ("mock-c-p4", |s| s.mock = Some(('c', 4))),
    ("mock-c-p5", |s| s.mock = Some(('c', 5))),
    ("mock-c-p6", |s| s.mock = Some(('c', 6))),
    ("mock-c-p7", |s| s.mock = Some(('c', 7))),
    ("announcements", |s| s.edit_announcements()),
    ("announcement-passage", |s| s.demo_notices("passage")),
    ("announcement-passage-no-bible", |s| s.demo_notices("passage-no-bible")),
    ("announcement-birthdays", |s| s.demo_notices("birthdays")),
    ("keys", |s| s.sheet = Sheet::Keys),
    ("tour-plan", |s| s.start_tour()),
    // GUIDE1: the Screens lesson, with its control on screen and with the control hidden.
    ("lesson-screens", |s| s.start_lesson(crate::guide::lessons::SCREENS)),
    ("help-calendar", |s| {
        s.open_tab(Tab::Calendar);
        s.help = Some(crate::guide::lessons::Feature::Calendar);
    }),
    ("lesson-calendar", |s| s.start_lesson(crate::guide::lessons::CALENDAR)),
    ("whats-new", |s| s.news = Some(0)),
    ("lesson-plan", |s| s.start_lesson(crate::guide::lessons::PLAN)),
    ("help-plan", |s| s.help = Some(crate::guide::lessons::Feature::Plan)),
    ("lesson-phone", |s| {
        s.sheet = Sheet::Remote;
        s.start_lesson(crate::guide::lessons::PHONE);
    }),
    ("lesson-listen", |s| {
        s.open_listen();
        s.start_lesson(crate::guide::lessons::LISTEN);
    }),
    ("lesson-screens-two", |s| {
        s.demo_screens = true;
        s.sheet = Sheet::Screens;
        s.start_lesson(crate::guide::lessons::SCREENS);
    }),
    // tools/check-lessons.py walks each lesson from here: the lesson ARTOSIA_LESSON names.
    ("lesson-from-env", |s| {
        let wanted = std::env::var("ARTOSIA_LESSON").unwrap_or_default();
        let every = crate::guide::lessons::ALL.iter().chain(std::iter::once(&crate::ui::tour::BASICS));
        if let Some(lesson) = every.into_iter().find(|lesson| lesson.id == wanted) {
            s.start_lesson(*lesson);
        }
    }),
    // A song in Preview hides the start tiles: the web lesson's first step goes by the search.
    ("lesson-web-search", |s| {
        s.open_song(0, None);
        s.start_lesson(crate::guide::lessons::WEB);
    }),
    ("lesson-stream", |s| {
        s.open_stream();
        s.start_lesson(crate::guide::lessons::STREAM);
    }),
    ("lesson-web", |s| {
        s.demo_web_songs("results");
        s.start_lesson(crate::guide::lessons::WEB);
    }),
    ("lesson-pptx", |s| {
        s.open_tab(Tab::Notices);
        s.demo_pptx("made");
        s.start_lesson(crate::guide::lessons::PPTX);
    }),
    ("lesson-screens-jobs", |s| {
        s.demo_screens = true;
        s.sheet = Sheet::Screens;
        s.lesson = Some((crate::guide::lessons::SCREENS, 2));
    }),
    ("whats-new-people", |s| s.news = Some(1)),
    ("lesson-calendar-new", |s| {
        s.open_tab(Tab::Calendar);
        s.start_lesson(crate::guide::lessons::CALENDAR);
    }),
    ("tour-preview", |s| s.tour = Some(1)),
    ("tour-show", |s| s.tour = Some(2)),
    ("tour-live", |s| s.tour = Some(3)),
    ("tour-search", |s| s.tour = Some(4)),
    // The first run, with made-up answers (`ui/onboard.rs` `demo_onboard`).
    ("onboard-language", |s| s.demo_onboard("language")),
    ("onboard-country", |s| s.demo_onboard("country")),
    ("onboard-bible", |s| s.demo_onboard("bible")),
    ("onboard-bible-offline", |s| s.demo_onboard("bible-offline")),
    ("onboard-songs", |s| s.demo_onboard("songs")),
    ("onboard-sing", |s| s.demo_onboard("sing")),
    ("onboard-screens", |s| s.demo_onboard("screens")),
    ("onboard-one-screen", |s| s.demo_onboard("one-screen")),
    ("onboard-ready", |s| s.demo_onboard("ready")),
    ("onboard-ready-failed", |s| s.demo_onboard("ready-failed")),
    ("onboard-done", |s| s.demo_onboard("done")),
    ("listen", |s| s.sheet = Sheet::Listen),
    ("listen-tempo", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.listen_level.store(0.55f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.tempo = Some((artosia_listen::Tempo { bpm: 92.0, confidence: 0.7, period: 60.0 / 92.0, since_beat: 0.0 }, 0.0));
    }),
    ("listen-tempo-half", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.listen_level.store(0.55f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.tempo = Some((artosia_listen::Tempo { bpm: 136.0, confidence: 0.7, period: 60.0 / 136.0, since_beat: 0.0 }, 0.0));
        s.set_tempo_count(crate::state::TempoCount::Half);
    }),
    ("listen-quiet", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.listen_level.store(0.1f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.settings.beat_blink = true;
    }),
    // Everything the first part can hold at once: on a laptop its lower rows scroll under the
    // parts and the switch.
    ("listen-heard", |s| {
        s.demo_listening = true;
        s.sheet = Sheet::Listen;
        s.settings.recordings_folder = r"D:\Artosia\recordings".to_string();
        s.listen_level.store(0.4f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        s.tempo = Some((artosia_listen::Tempo { bpm: 92.0, confidence: 0.7, period: 60.0 / 92.0, since_beat: 0.0 }, 0.0));
        s.microphones = crate::actions::demo_microphones();
        s.heard.push("morning light upon the hill and every voice".to_string());
        s.heard_hits = (0..3).map(crate::state::HeardHit::Song).collect();
    }),
    // The second part of the Listen dialog: recordings kept in a picked folder, none yet, and
    // a folder that cannot be written; and the lit mark in the status bar while it keeps.
    ("listen-recordings", |s| {
        s.sheet = Sheet::Listen;
        s.listen_part = 1;
        s.settings.keep_recordings = true;
        s.settings.recordings_folder = r"D:\Church\OneDrive\Artosia recordings".to_string();
        s.recordings.used = 212 << 20;
        s.recordings.demo = true;
    }),
    ("listen-recordings-none", |s| {
        s.sheet = Sheet::Listen;
        s.listen_part = 1;
        s.settings.recordings_folder = r"D:\Artosia\recordings".to_string();
    }),
    ("listen-recordings-failed", |s| {
        s.sheet = Sheet::Listen;
        s.listen_part = 1;
        s.settings.keep_recordings = true;
        s.settings.recordings_folder = r"E:\Artosia recordings".to_string();
        s.recordings.failed = true;
    }),
    // A PC that read the first speech model quickly is offered the better one.
    ("listen-better", |s| {
        s.sheet = Sheet::Listen;
        s.better = crate::speech_model::Better::Offered;
    }),
    ("listen-problem", |s| {
        s.sheet = Sheet::Listen;
        s.listen_problem = Some(crate::why::listen_error(&artosia_listen::ListenError::NoDevice(String::new())));
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
    // The list that drops from the plan's title, and the same with a plan whose name is long.
    ("plans-drop", |s| {
        s.demo_plans();
        s.open_switcher();
    }),
    ("plans-drop-many", |s| {
        s.demo_plans();
        for (day, hour) in [(0u8, 18u8), (1, 6), (1, 19), (2, 18), (3, 5), (3, 19), (4, 18), (4, 20), (5, 9), (5, 16), (6, 6), (6, 8), (6, 12), (6, 17), (6, 19)] {
            s.begin_new_plan();
            if let Some(edit) = s.switcher.editing.as_mut() {
                (edit.day, edit.hour) = (day, Some(hour));
            }
            s.finish_edit(true);
        }
        s.open_switcher();
    }),
    ("plans-drop-long", |s| {
        s.demo_plans();
        s.begin_new_plan();
        if let Some(edit) = s.switcher.editing.as_mut() {
            edit.hour = Some(16);
            edit.name.replace_text("Persekutuan Doa Kaum Muda dan Remaja Gabungan");
        }
        s.finish_edit(true);
        s.open_switcher();
    }),
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
        s.open_tab(crate::state::Tab::Look);
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
    // The only notice of a new version: a toast after it was fetched in the background.
    ("toast-update", |s| s.say(crate::strings::update_ready("0.1.19"), None)),
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
    // Before the claim on the library and any window: a check that runs inside a package.
    if store::report_if_asked() {
        return;
    }
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
