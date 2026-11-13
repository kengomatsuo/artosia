//! The search panel: results on the left, the picked result in full on the right, so a
//! volunteer checks it is the right song before it goes anywhere.

use artosia_core::deck::{announcement_slide, tag_group};
use artosia_slides::look::presets;
use artosia_slides::stage::group_color;
use rune::{Cursor, El, GrowthSpec, Key, KeyEvent, ScrollSpec, Tx, col, el, icon, row, spacer, text, text_input};
use rune_kit::{Kit, ScrollBar, SegmentedControl};

use crate::palette::{Command, Group, Hit, Pick, Reason, Scope};
use crate::state::App;
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use crate::ui::widgets::{btn, highlighted, key_hint, keycaps, state_pill, tag_chip};

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 560.0;
const LIST: f32 = 400.0;

fn command_icon(command: Command) -> &'static rune::Icon {
    match command {
        Command::NewSong => &icons::PLUS,
        Command::PasteSong => &icons::PASTE,
        Command::Announcements | Command::LoopAnnouncements => &icons::ANNOUNCE,
        Command::Looks => &icons::PALETTE,
        Command::Settings => &icons::SLIDERS,
        Command::Backdrops => crate::ui::backdrops::mark(),
        Command::Screens | Command::Black | Command::Logo => &icons::SCREEN,
        Command::Import | Command::ImportFolder | Command::ImportZip | Command::ImportPptx | Command::ImportBirthdays | Command::BibleFile => &icons::IMPORT,
        Command::Bibles => &icons::BOOK,
        Command::Remote => &icons::PHONE,
        Command::Heading | Command::NewPlan => &icons::HEADING,
        Command::PlanRename => &icons::TIMER,
        Command::PlanDuplicate => &icons::COPY,
        Command::PlanDelete => &icons::TRASH,
        Command::StartHere => &icons::PLAY,
        Command::Earlier => &icons::HISTORY,
        Command::NewAnnouncement => &icons::ANNOUNCE,
        Command::Keys => &icons::KEYBOARD,
        Command::Tour => &icons::PLAY,
        Command::FirstRun | Command::Languages | Command::AllLanguages => &icons::GLOBE,
        Command::StartService => &icons::PLAY,
        Command::Next => &icons::ARROW_RIGHT,
        Command::Previous => &icons::ARROW_LEFT,
        Command::ClearText | Command::HideMessage => &icons::HIDE,
        Command::StopCountdown => &icons::TIMER,
        Command::EditSong => &icons::PENCIL,
        Command::Undo => &icons::UNDO,
        Command::CheckUpdate => &icons::IMPORT,
        Command::Listen => &icons::MIC,
        Command::Stream => &icons::STREAM,
    }
}

/// The icon, the two lines and the chips of a result that is not a song or a verse.
fn plain_row(k: &Kit, hit: &Hit) -> (&'static rune::Icon, String, String) {
    let _ = k;
    match hit {
        Hit::PlanItem { title, .. } => (&icons::BOOK, title.clone(), t::IN_PLAN().to_string()),
        Hit::Plan { name, .. } => (&icons::HEADING, name.clone(), t::KIND_PLAN().to_string()),
        Hit::Announcement { title, body, .. } => (&icons::ANNOUNCE, title.clone(), body.replace('\n', " ")),
        Hit::Look { name, .. } => (&icons::LOOK, name.clone(), t::KIND_LOOK().to_string()),
        Hit::Bible { name, .. } => (&icons::BOOK, name.clone(), t::KIND_BIBLE().to_string()),
        Hit::Command(command) => (command_icon(*command), command.label().to_string(), command.detail().to_string()),
        Hit::Countdown(minutes) => (&icons::TIMER, t::countdown_for(*minutes), t::COUNTDOWN_DETAIL().to_string()),
        Hit::Message(words) => (&icons::ANNOUNCE, words.clone(), t::MESSAGE_DETAIL().to_string()),
        Hit::StageMessage(words) => (&icons::ANNOUNCE, words.clone(), t::STAGE_MESSAGE_DETAIL().to_string()),
        Hit::OnlyLanguage { code, .. } => (&icons::GLOBE, t::cmd_only_language(&crate::ui::languages::name_in_sentence(code)), t::LANGUAGES_ON_SLIDE().to_string()),
        Hit::Language(code) => (&icons::GLOBE, crate::languages::index_of(code).map_or("", |at| crate::languages::LANGUAGES[at].name).to_string(), t::LOOK_LANGUAGE().to_string()),
        Hit::Song { .. } | Hit::Section { .. } | Hit::Passage { .. } | Hit::Group(_) => (&icons::SONG, String::new(), String::new()),
    }
}

/// A heading over a group of suggested songs, as tall as a result so the list scrolls evenly.
fn group_row(k: &Kit, group: &Group) -> El {
    let words = match group {
        Group::Follows(title) => t::group_follows(title),
        Group::Weekday => t::GROUP_WEEKDAY().to_string(),
        Group::Recent => t::GROUP_RECENT().to_string(),
    };
    row().key("group").w_full().h(size::RESULT).px(space::S3).items_end().pb(space::S1).child(text(words).font(type_::SMALL_STRONG).color(k.c.text_muted).truncate())
}

fn hit_row(s: &App, k: &Kit, index: usize, hit: &Hit) -> El {
    if let Hit::Group(group) = hit {
        return group_row(k, group);
    }
    let picked = index == s.palette.picked;
    let mark = theme::accent();
    let mut trailing: Vec<El> = Vec::new();
    let (glyph, title, detail): (&'static rune::Icon, El, El) = match hit {
        Hit::Song { song, title_ranges, line, planned, reason } => {
            let found = &s.songs[*song];
            let title = highlighted(&found.title, title_ranges, type_::BODY_STRONG, k.c.text_base, mark);
            if *planned {
                trailing.push(state_pill(t::PLAN_CHIP(), k.c.layer_04, k.c.text_base));
            }
            // The habit chip is long, so it takes the room of the start and section chips: the
            // matched line is still in the second line, and the preview says the rest.
            let habit = matches!(reason, Some(Reason::Habit { .. }));
            if let Some((tag, _, _)) = s.learn.starts.get(song).filter(|_| !habit) {
                trailing.push(row().gap(3.0).center_y().role("img").label(t::starts_chip(&t::section_label(*tag))).help(t::starts_chip(&t::section_label(*tag))).child(icon(&icons::PLAY, 12.0).color(k.c.text_muted).a11y_hidden()).child(crate::ui::widgets::section_chip(*tag)));
            }
            if let Some(Reason::Habit { typed, picks, .. }) = reason {
                trailing.push(state_pill(&t::habit_chip(typed, *picks), k.c.layer_04, k.c.text_base));
            }
            match (line, reason) {
                // A suggested song says why in words, in the line a title match would use for lyrics.
                (_, Some(why @ (Reason::Follows { .. } | Reason::Weekday { .. } | Reason::Recent { .. }))) => {
                    (&icons::SONG, title, text(s.reason_line(why)).font(type_::SMALL).color(k.c.text_muted))
                }
                (Some(line), _) => {
                    // A song found by a line in another language says which language that line is in.
                    if line.lang != found.lang {
                        trailing.push(state_pill(&t::match_language(&crate::ui::languages::name_in_sentence(&line.lang)), k.c.layer_04, k.c.text_base).key("found-in-language"));
                    }
                    if !habit {
                        trailing.push(tag_chip(&t::section_label(line.tag), group_color(tag_group(line.tag))));
                    }
                    (&icons::SONG, title, highlighted(&line.text, &line.ranges, type_::SMALL, k.c.text_muted, mark))
                }
                (None, _) => {
                    let first = found.first_line().unwrap_or_default();
                    (&icons::SONG, title, text(if found.author.is_empty() { first } else { found.author.clone() }).font(type_::SMALL).color(k.c.text_muted))
                }
            }
        }
        Hit::Section { tag, label, text: words, .. } => {
            trailing.push(crate::ui::widgets::section_chip(*tag));
            let first = words.lines().next().unwrap_or_default().to_string();
            (&icons::SONG, text(label.clone()).font(type_::BODY_STRONG).color(k.c.text_base), text(first).font(type_::SMALL).color(k.c.text_muted))
        }
        Hit::Passage { title, text: body, ranges, .. } => (
            &icons::BOOK,
            text(title.clone()).font(type_::BODY_STRONG).color(k.c.text_base),
            highlighted(body, ranges, type_::SMALL, k.c.text_muted, mark),
        ),
        other => {
            let (glyph, title, detail) = plain_row(k, other);
            if matches!(other, Hit::PlanItem { .. }) {
                trailing.push(state_pill(t::PLAN_CHIP(), k.c.layer_04, k.c.text_base));
            }
            (glyph, text(title).font(type_::BODY_STRONG).color(k.c.text_base), text(detail).font(type_::SMALL).color(k.c.text_muted))
        }
    };
    // A row that can go into the plan has its own button for it, whether or not the pointer
    // is over it: the tester clicked a song, watched the search close, and had no Add left.
    let joins = matches!(hit, Hit::Song { planned: false, .. } | Hit::Section { .. } | Hit::Announcement { .. } | Hit::Passage { step: None, .. });
    if joins && !s.palette.adding {
        trailing.push(btn(t::BTN_ADD_TO_PLAN()).icon(&icons::PLUS).icon_only().small().build(k, move |s, _| {
            s.palette.picked = index;
            s.pick(Pick::AddToPlan);
        }));
    }
    row()
        .key("row")
        .role("option")
        .w_full()
        .h(size::RESULT)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .fill(if picked { k.c.layer_04 } else { rune::Color::TRANSPARENT })
        .hover(k.c.hover)
        .cursor(Cursor::Pointer)
        .child(icon(glyph, 16.0).color(if picked { k.c.text_base } else { k.c.text_muted }).a11y_hidden())
        .child(col().grow(1.0).min_w(0.0).gap(2.0).child(title.truncate()).child(detail.truncate()))
        .children(trailing)
        // A click does what Enter does: it opens the row in Preview, or plans it while adding.
        // Nothing here reaches the projector (owner, 2026-11-11: "clicking does nothing and
        // double-clicking shows to live?").
        .on_press_tx(move |s: &mut App, tx: &mut Tx| {
            s.palette.picked = index;
            s.pick(Pick::Open);
            if !s.palette.open && !s.wants_focus() {
                tx.blur();
            }
        })
}

fn list(s: &App, k: &Kit) -> El {
    let hits = &s.palette.hits;
    if hits.is_empty() {
        let query = s.palette.query.text.trim();
        let words = if s.bible_is_loading() && s.palette.scope != Scope::Songs {
            t::BIBLE_LOADING().to_string()
        } else if query.is_empty() {
            match s.palette.scope {
                Scope::Bible => t::BIBLE_LOADING().to_string(),
                _ => t::SONGS_EMPTY().to_string(),
            }
        } else {
            t::nothing_found(query)
        };
        let mut empty = col().key("no-hits").w(LIST).h_full().center().gap(space::S3).padding(space::S5).child(text(words).font(type_::BODY).color(k.c.text_muted));
        if !query.is_empty() && s.palette.scope != Scope::Bible && s.palette.chip.is_none() {
            let title = query.to_string();
            empty = empty.child(btn(t::new_song_called(query)).icon(&icons::PLUS).build(k, move |s, _| {
                s.edit_new_song(artosia_core::song::Song { title: title.clone(), ..Default::default() });
            }));
        }
        return empty;
    }
    // Only the rows on screen are built, so a library of thousands of songs scrolls freely.
    let rows = col().key("hits").growth(GrowthSpec::default()).virtual_rows(
        hits.len(),
        size::RESULT + 2.0,
        |_: &App, index| format!("hit-{index}").into(),
        |s: &App, index| match s.palette.hits.get(index) {
            Some(hit) => el().w_full().px(space::S2).py(1.0).child(hit_row(s, theme::kit(), index, hit)),
            None => el(),
        },
    );
    let viewport = col().key("hit-scroll").role("listbox").label(t::SEARCH_RESULTS()).w_full().grow(1.0).min_h(0.0).py(space::S1).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(rows);
    col().key("hit-list").w(LIST).h_full().min_h(0.0).child(viewport.scroll_bar(k))
}

/// The colour of a book's group: law, history, wisdom, prophets, gospels, letters, vision.
fn book_color(book: u8) -> rune::Color {
    group_color([0, 5, 1, 2, 3, 0, 2][usize::from(artosia_core::bible::book_group(book)).min(6)])
}

fn grid_button(s: &App, k: &Kit, index: usize, label: String, width: f32, color: rune::Color) -> El {
    let picked = index == s.palette.picked;
    el().key(format!("hit-{index}"))
        .role("button")
        .label(label.clone())
        .w(width)
        .h(32.0)
        .px(space::S2)
        .center()
        .radius(k.radius.md)
        .fill(theme::tint_of(color, picked))
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Pointer)
        .child(text(label).font(type_::SMALL_STRONG).color(k.c.text_base).truncate())
        .on_press(move |s: &mut App| {
            s.palette.picked = index;
            s.pick(Pick::Open);
            s.palette.query.focused = true;
        })
}

/// Books, then chapters, as buttons: the way in for a volunteer who would rather click than type.
fn grid(s: &App, k: &Kit) -> El {
    let hits = &s.palette.hits;
    let chapters = hits.iter().any(|hit| matches!(hit, Hit::Passage { step: Some(step), .. } if step.ends_with(':')));
    let mut body = col().key("bible-grid").w_full().grow(1.0).min_h(0.0).px(space::S4).py(space::S3).gap(space::S2).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE);
    if chapters {
        let book = hits.first().and_then(|hit| match hit {
            Hit::Passage { reference, .. } => Some(reference.book),
            _ => None,
        });
        let name = book.and_then(|book| s.bible().and_then(|bible| bible.book(book)).map(|info| info.name.clone())).unwrap_or_default();
        body = body.child(text(t::pick_chapter(&name)).font(type_::SMALL_STRONG).color(k.c.text_muted));
        let mut wrap = row().w_full().gap(6.0).flex_wrap();
        for (index, hit) in hits.iter().enumerate() {
            let Hit::Passage { reference, .. } = hit else { continue };
            wrap = wrap.child(grid_button(s, k, index, reference.chapter.to_string(), 52.0, book_color(reference.book)));
        }
        return body.child(wrap).scroll_bar(k);
    }
    for (heading, range) in [(t::OLD_TESTAMENT(), 1u8..=39u8), (t::NEW_TESTAMENT(), 40u8..=66u8)] {
        let mut wrap = row().w_full().gap(6.0).flex_wrap();
        let mut any = false;
        for (index, hit) in hits.iter().enumerate() {
            let Hit::Passage { reference, title, .. } = hit else { continue };
            if range.contains(&reference.book) {
                wrap = wrap.child(grid_button(s, k, index, title.clone(), 116.0, book_color(reference.book)));
                any = true;
            }
        }
        if any {
            body = body.child(text(heading).font(type_::SMALL_STRONG).color(k.c.text_muted)).child(wrap);
        }
    }
    body.scroll_bar(k)
}

fn actions(s: &App, k: &Kit, hit: &Hit) -> El {
    let bar = row().key("preview-actions").w_full().padding(space::S3).gap(space::S2).center_y();
    if s.palette.adding && matches!(hit, Hit::Song { .. } | Hit::Section { .. } | Hit::Announcement { .. } | Hit::Passage { step: None, .. }) {
        return bar
            .child(btn(t::BTN_DONE()).hint("Esc").build(k, |s, _| s.close_palette()))
            .child(spacer())
            .child(btn(t::BTN_ADD_TO_PLAN()).icon(&icons::PLUS).primary().hint(t::KEY_ENTER()).build(k, |s, _| s.pick(Pick::AddToPlan)));
    }
    let choose = || btn(t::BTN_CHOOSE()).primary().hint(t::KEY_ENTER()).build(k, |s, _| s.pick(Pick::Open));
    // Only the main button carries its key: the row under the list names all three, and with
    // three key caps the Indonesian buttons ran past the panel's edge.
    let add = || btn(t::BTN_ADD_TO_PLAN()).build(k, |s, _| s.pick(Pick::AddToPlan));
    let show = || btn(t::BTN_SHOW_NOW()).build(k, |s, _| s.pick(Pick::ShowNow));
    let open = || btn(t::BTN_OPEN()).primary().hint(t::KEY_ENTER()).build(k, |s, _| s.pick(Pick::Open));
    match hit {
        Hit::Group(_) => bar,
        Hit::Command(_) | Hit::OnlyLanguage { .. } | Hit::Look { .. } | Hit::Bible { .. } | Hit::Countdown(_) | Hit::Message(_) | Hit::StageMessage(_) | Hit::Language(_) | Hit::Plan { .. } => bar.child(spacer()).child(choose()),
        Hit::Passage { step: Some(_), .. } => bar.child(spacer()).child(btn(t::BTN_ADD_TO_PLAN()).build(k, |s, _| s.pick(Pick::AddToPlan))).child(choose()),
        Hit::PlanItem { .. } => bar.child(spacer()).child(show()).child(open()),
        _ => bar.child(add()).child(spacer()).child(show()).child(open()),
    }
}

/// A song's words, with the matched line or the picked section marked.
fn song_preview(s: &App, k: &Kit, song: usize, line: Option<&artosia_core::search::LineHit>, picked: Option<artosia_core::song::Tag>, notes: Vec<String>) -> Vec<El> {
    let found = &s.songs[song];
    let mut out = vec![crate::ui::widgets::wrapped(found.title.clone()).font(type_::HEADING).color(k.c.text_base).wrap()];
    if !found.author.is_empty() {
        out.push(text(found.author.clone()).font(type_::SMALL).color(k.c.text_muted));
    }
    // Why the song is where it is, in words with the real counts.
    for note in notes {
        out.push(crate::ui::widgets::wrapped(note).font(type_::SMALL_STRONG).color(k.c.text_muted).wrap());
    }
    for (at, section) in found.sections().iter().enumerate() {
        let color = group_color(tag_group(section.tag));
        let matched = line.filter(|line| line.section == at);
        let whole = picked == Some(section.tag);
        let mut lines = col().gap(1.0);
        for content in &section.lines {
            let hit_line = matched.filter(|line| line.text == *content);
            lines = lines.child(match hit_line {
                Some(line) => highlighted(content, &line.ranges, type_::BODY, k.c.text_base, theme::accent()).wrap(),
                None => crate::ui::widgets::wrapped(content.clone()).font(type_::BODY).color(if whole { k.c.text_base } else { k.c.text_muted }).wrap(),
            });
        }
        let mut block = col().key(format!("section-{at}")).gap(space::S1).child(row().child(tag_chip(&t::section_label(section.tag), color))).child(lines);
        if whole {
            block = block.padding(space::S2).radius(k.radius.md).fill(theme::tint_of(color, false));
        }
        out.push(block);
    }
    out
}

fn preview(s: &App, k: &Kit) -> El {
    let Some(hit) = s.palette.hits.get(s.palette.picked) else {
        return col().key("preview").grow(1.0).h_full();
    };
    let mut body = col().key("preview-body").w_full().grow(1.0).min_h(0.0).padding(space::S5).gap(space::S3).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE);
    let wide = WIDTH - LIST - 2.0 * space::S5 - 1.0;
    let slide_preview = |state: &artosia_slides::frame::OutputState| {
        el().size(wide, (wide * 9.0 / 16.0).round()).radius(k.radius.lg).clip_rounded(k.radius.lg).child(crate::ui::thumb(s, state, wide))
    };
    match hit {
        Hit::Song { song, line, reason, .. } => body = body.children(song_preview(s, k, *song, line.as_ref(), None, s.song_notes(*song, reason.as_ref()))),
        Hit::Section { song, tag, .. } => body = body.children(song_preview(s, k, *song, None, Some(*tag), Vec::new())),
        Hit::Passage { reference, title, step, .. } => {
            body = body.child(crate::ui::widgets::wrapped(title.clone()).font(type_::HEADING).color(k.c.text_base).wrap());
            if let Some(bible) = s.bible() {
                body = body.child(text(bible.name.clone()).font(type_::SMALL).color(k.c.text_muted));
                let limit = if step.is_some() { 12 } else { 60 };
                for verse in bible.verses(reference).iter().take(limit) {
                    body = body.child(
                        row()
                            .key(format!("verse-{}-{}", verse.chapter, verse.verse))
                            .gap(space::S2)
                            .child(text(verse.verse.to_string()).font(type_::SMALL_STRONG).color(k.c.text_faint).w(22.0))
                            .child(crate::ui::widgets::wrapped(verse.text.to_string()).font(type_::BODY).color(k.c.text_base).wrap().grow(1.0)),
                    );
                }
            }
        }
        Hit::Announcement { index, .. } => {
            if let Some(item) = s.announcements.get(*index) {
                // What the card gives today; a card with nothing to give shows its own words.
                let slide = s.card_slides(item).into_iter().next().unwrap_or_else(|| announcement_slide(item));
                body = body.child(slide_preview(&s.preview_state(Some(slide))));
            }
        }
        Hit::Look { index, name } => {
            if let Some(preset) = presets().into_iter().nth(*index) {
                let mut state = s.preview_state(Some(crate::ui::sample_slide()));
                state.look = s.look().with_preset(preset);
                body = body.child(slide_preview(&state)).child(text(name.clone()).font(type_::HEADING).color(k.c.text_base));
            }
        }
        other => {
            let (glyph, title, detail) = plain_row(k, other);
            body = body
                .child(row().gap(space::S3).center_y().child(icon(glyph, 16.0).color(k.c.text_muted)).child(crate::ui::widgets::wrapped(title).font(type_::HEADING).color(k.c.text_base).wrap().grow(1.0)))
                .child(crate::ui::widgets::wrapped(detail).font(type_::BODY).color(k.c.text_muted).wrap());
        }
    }
    col()
        .key("preview")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0)
        .fill(k.c.layer_01)
        .child(body.scroll_bar(k))
        .child(crate::ui::widgets::divider(k))
        .child(actions(s, k, hit))
}

fn input_row(s: &App, k: &Kit) -> El {
    let inside = s.palette.chip.and_then(|song| s.songs.get(song));
    let field = text_input(&s.palette.query, type_::SEARCH)
        .key("palette-query")
        .label(t::SEARCH_PLACEHOLDER())
        .single_line()
        .grow(1.0)
        .color(k.c.text_base)
        .caret(k.c.text_base, k.selection())
        .placeholder(
            match (inside.is_some(), s.palette.scope) {
                (true, _) => t::SEARCH_SECTIONS().to_string(),
                (false, Scope::All) => t::SEARCH_PLACEHOLDER().to_string(),
                (false, Scope::Songs) => t::SEARCH_SONGS().to_string(),
                (false, Scope::Bible) => t::search_bible(&s.example_passage("23:1")),
            },
            k.c.text_faint,
        )
        .on_edit(|s: &mut App| &mut s.palette.query)
        .on_input_change(|s: &mut App, _: &mut Tx| {
            s.palette.picked = 0;
            s.search();
        })
        .on_input_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| {
            match ev.key {
                Key::Down => s.move_pick(1),
                Key::Up => s.move_pick(-1),
                Key::PageDown => s.move_pick(7),
                Key::PageUp => s.move_pick(-7),
                Key::Escape => {
                    s.palette_escape();
                    if !s.palette.open {
                        tx.blur();
                    }
                    return true;
                }
                Key::Backspace => return s.palette_detach(),
                Key::Tab => {
                    s.palette_tab();
                    return true;
                }
                Key::Enter => {
                    let how = if ev.mods.cmd {
                        Pick::ShowNow
                    } else if ev.mods.shift {
                        Pick::AddToPlan
                    } else {
                        Pick::Open
                    };
                    s.pick(how);
                    // A pick that opened a dialog or an editor leaves the focus to it. A blur
                    // here ran after the new field had asked for focus, in a real window only:
                    // the box that names a plan and the gallery's search took no typing until
                    // they were clicked (found pressing real keys, 2026-11-11).
                    if !s.palette.open && !s.wants_focus() {
                        tx.blur();
                    }
                    if s.sheet == crate::state::Sheet::Settings {
                        tx.focus(crate::ui::SETTINGS_FIRST, true);
                    }
                    return true;
                }
                _ => return false,
            }
            tx.scroll_into_view(format!("hit-{}", s.palette.picked));
            true
        });
    let mut line = row().key("palette-input").w_full().h(60.0).px(space::S4).gap(space::S3).center_y().child(icon(&rune_kit::glyphs::SEARCH, 16.0).color(k.c.text_muted).a11y_hidden());
    if s.palette.adding {
        // What Enter will do, said where the eye is while typing.
        line = line.child(
            row()
                .key("palette-adding")
                .h(28.0)
                .px(space::S2)
                .gap(6.0)
                .center_y()
                .radius(k.radius.md)
                .fill(theme::accent().alpha(0.22))
                .child(icon(&icons::PLUS, 16.0).color(k.c.text_base).a11y_hidden())
                .child(text(t::ADDING_TO_PLAN()).font(type_::BODY_STRONG).color(k.c.text_base)),
        );
    }
    if let Some(song) = inside {
        // The song the search is inside. A click on it, or Backspace in an empty field, lets go.
        line = line.child(
            row()
                .key("palette-chip")
                .role("button")
                .label(song.title.clone())
                .h(28.0)
                .max_w(260.0)
                .px(space::S2)
                .gap(6.0)
                .center_y()
                .radius(k.radius.md)
                .fill(theme::accent().alpha(0.22))
                .hover(theme::accent().alpha(0.32))
                .cursor(Cursor::Pointer)
                .child(icon(&icons::SONG, 16.0).color(k.c.text_base).a11y_hidden())
                .child(text(song.title.clone()).font(type_::BODY_STRONG).color(k.c.text_base).truncate())
                .on_press(|s: &mut App| {
                    s.palette.chip = None;
                    s.search();
                    s.palette.query.focused = true;
                }),
        );
        return line.child(field).child(close(k));
    }
    let scope = match s.palette.scope {
        Scope::All => 0,
        Scope::Songs => 1,
        Scope::Bible => 2,
    };
    let scopes = SegmentedControl::new(&[t::SCOPE_ALL(), t::SCOPE_SONGS(), t::SCOPE_BIBLE()], Some(scope))
        .label(t::SCOPE_LABEL())
        .fit()
        .on_pick(|s: &mut App, picked, _: &mut Tx| {
            s.set_scope(match picked {
                1 => Scope::Songs,
                2 => Scope::Bible,
                _ => Scope::All,
            });
            s.palette.query.focused = true;
        })
        .build(k);
    // Tab turns this switch unless the picked row gives Tab another job: its key is drawn
    // beside it, where the eye is when it wonders how to narrow the list.
    let turns = !matches!(s.palette.hits.get(s.palette.picked), Some(Hit::Song { .. } | Hit::Passage { step: Some(_), .. }));
    line.child(field).child(el().layer_opacity(if turns { 1.0 } else { 0.0 }).child(keycaps(k, "Tab", None))).child(scopes).child(close(k))
}

/// The way out for the pointer: the panel stays open after a song is added to the plan with
/// its plus, and a tester who had finished found only a press outside it to leave by.
fn close(k: &Kit) -> El {
    btn(t::BTN_CLOSE()).icon(&icons::CLOSE).icon_only().ghost().small().hint("Esc").build(k, |s, tx| {
        s.close_palette();
        tx.blur();
    })
}

fn footer(s: &App, k: &Kit) -> El {
    let count = match s.palette.scope {
        _ if s.palette.chip.is_some() => String::new(),
        Scope::Bible => String::new(),
        _ if s.palette.query.text.trim().is_empty() => t::songs(s.songs.len()),
        _ => String::new(),
    };
    // Tab says what it will do to the picked row.
    let picked = s.palette.hits.get(s.palette.picked);
    // A row that is chosen, such as a command or a book of the Bible, has one key: Enter.
    let chosen = matches!(picked, Some(Hit::Command(_) | Hit::OnlyLanguage { .. } | Hit::Look { .. } | Hit::Bible { .. } | Hit::Countdown(_) | Hit::Message(_) | Hit::StageMessage(_) | Hit::Language(_) | Hit::Plan { .. } | Hit::Passage { step: Some(_), .. }));
    let tab = match picked {
        _ if s.palette.chip.is_some() => None,
        Some(Hit::Song { .. }) => Some(t::HINT_SECTIONS()),
        _ => None,
    };
    let mut bar = row()
        .key("palette-footer")
        .w_full()
        .h(36.0)
        .px(space::S4)
        .gap(space::S4)
        .center_y();
    bar = if s.palette.adding {
        bar.child(key_hint(k, t::KEY_ENTER(), t::BTN_ADD_TO_PLAN())).child(key_hint(k, "Esc", t::BTN_DONE()))
    } else if chosen {
        bar.child(key_hint(k, t::KEY_ENTER(), t::BTN_CHOOSE()))
    } else {
        bar.child(key_hint(k, t::KEY_ENTER(), t::HINT_OPEN())).child(key_hint(k, t::KEY_SHOW_NOW(), t::BTN_SHOW_NOW())).child(key_hint(k, t::KEY_ADD(), t::BTN_ADD_TO_PLAN()))
    };
    if let Some(does) = tab {
        bar = bar.child(key_hint(k, "Tab", does));
    }
    bar.child(spacer()).child(text(count).font(type_::SMALL).color(k.c.text_faint))
}

pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let panel = col()
        .key("palette")
        .role("dialog")
        .label(t::SEARCH_PLACEHOLDER())
        .w_full()
        .max_w(WIDTH)
        .h(HEIGHT)
        .radius(k.radius.xl)
        .clip_rounded(k.radius.xl)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_strong)
        .shadow(24.0, 64.0, rune::Color::BLACK.alpha(0.5))
        .instant_changes()
        .child(input_row(s, k))
        .child(crate::ui::widgets::divider(k))
        .child(if s.palette_is_grid() {
            grid(s, k)
        } else {
            row().w_full().grow(1.0).min_h(0.0).child(list(s, k)).child(crate::ui::widgets::vertical_divider(k)).child(preview(s, k))
        })
        .child(crate::ui::widgets::divider(k))
        .child(footer(s, k))
        .modal(true, |s: &mut App| s.close_palette());
    layer.push(el().key("palette-scrim").absolute(0.0, 0.0).w_full().h_full().fill(rune::Color::BLACK.alpha(0.55)).a11y_hidden());
    layer.push(col().key("palette-layer").absolute(0.0, 0.0).w_full().h_full().px(space::S4).pt(64.0).items_center().child(panel));
}
