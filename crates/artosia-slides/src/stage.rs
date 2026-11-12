//! The stage screen: what the band and the speaker read. The live lines large, the next
//! lines under them, the clock and the place in the song at the side, the arrangement along
//! the bottom. The layout follows Lerret's band view.

use rune::{Align, Color, El, LocalTime, TextStyle, col, el, oklch, row, spacer, text};

use crate::audience::clock_text;
use crate::frame::OutputState;
use crate::look::voice_ink;
use crate::voices::{self, Face};

/// The colour of a section group: verse, chorus, bridge, pre-chorus, intro or ending, other.
/// Red is left for "live" and yellow for messages.
pub fn group_color(group: usize) -> Color {
    match group {
        0 => oklch(0.72, 0.13, 250.0),
        1 => oklch(0.78, 0.15, 62.0),
        2 => oklch(0.72, 0.15, 305.0),
        3 => oklch(0.76, 0.11, 195.0),
        4 => oklch(0.74, 0.012, 265.0),
        _ => oklch(0.76, 0.14, 152.0),
    }
}

/// The wall clock as "10:42".
pub fn wall_clock(now: f64) -> String {
    let time = LocalTime::at((now * 1000.0) as i64);
    format!("{}:{:02}", time.hour, time.minute)
}

/// The whole stage screen at `w` by `h` points. `now` is seconds since the Unix epoch.
pub fn stage(state: &OutputState, w: f32, h: f32, now: f64) -> El {
    let u = h / 1080.0;
    let px = |v: f32| (v * u).round();
    let ink = Color::WHITE;
    let muted = oklch(0.72, 0.008, 265.0);
    let label = TextStyle::new(px(30.0).max(3.0), 700.0);
    let info = &state.stage;
    let side_w = px(460.0);
    let pad = px(56.0);

    let mut root = col().key("stage").size(w, h).fill(Color::BLACK);
    if !info.message.trim().is_empty() {
        root = root.child(
            row()
                .key("stage-message")
                .w_full()
                .px(pad)
                .py(px(22.0))
                .center_y()
                .fill(oklch(0.87, 0.17, 95.0))
                .child(text(info.message.trim()).font(TextStyle::new(px(52.0).max(4.0), 700.0)).color(Color::BLACK).truncate()),
        );
    }

    let live = state.slide.as_ref().filter(|_| !state.black);
    let hidden = state.text_hidden || state.logo_on;
    let main_w = w - side_w - pad * 2.0;
    let mut main = col().key("stage-main").grow(1.0).min_h(0.0).padding(pad).gap(px(20.0));
    match live {
        Some(slide) => {
            let accent = slide.tag.map(|tag| group_color(artosia_core::deck::tag_group(tag))).unwrap_or(muted);
            let status = if hidden { format!("{} (hidden from the room)", slide.label) } else { slide.label.clone() };
            main = main.child(text(status).key("stage-label").font(label).color(accent).truncate());
            // The largest size at which the live lines fit the room the next lines leave.
            let strips = if info.message.trim().is_empty() { 0.0 } else { px(100.0) } + if info.chips.is_empty() { 0.0 } else { px(104.0) };
            let next_room = if info.next.is_some() { px(250.0) } else if info.next_title.is_empty() { 0.0 } else { px(90.0) };
            let room = (h - strips - 2.0 * pad - px(70.0) - next_room).max(px(120.0));
            if !slide.more.is_empty() {
                // Several languages: the blocks of the projector, in its order and colours, on
                // the stage's white (muted while the room does not see the words).
                let (items, face) = (voices::of(slide), Face { family: rune::Family::Sans, weight: 700.0, height: 1.2 });
                let texts: Vec<&str> = items.iter().map(|(_, words)| *words).collect();
                let size = voices::fit(&texts, face, main_w, room, voices::gap(h), px(104.0), px(voices::HARD_FLOOR));
                let ink_of = |place: usize| if hidden { muted } else { voice_ink([255, 255, 255], place) };
                main = main.child(voices::column(&items, main_w, room, voices::gap(h), &ink_of, &|content, color| text(content.to_string()).font(face.at(size)).color(color).w(main_w).wrap()));
            } else {
                let mut size = px(104.0);
                for _ in 0..24 {
                    let style = TextStyle::new(size, 700.0).lh((size * 1.2).round());
                    if rune::text::measure(&slide.text, &[], style, Some(main_w)).1 <= room || size <= px(34.0) {
                        break;
                    }
                    size = (size * 0.94).floor();
                }
                main = main.child(
                    text(slide.text.clone())
                        .key("stage-text")
                        .font(TextStyle::new(size, 700.0).lh((size * 1.2).round()))
                        .color(if hidden { muted } else { ink })
                        .w(main_w)
                        .wrap(),
                );
            }
        }
        None => {
            let words = if state.black { info.words.black.clone() } else { info.words.nothing.clone() };
            main = main.child(text(words).key("stage-label").font(label).color(muted));
        }
    }
    main = main.child(spacer());
    let next_text = info.next.as_ref().map(|slide| slide.text.clone());
    let next_label = match (&info.next, info.next_title.is_empty()) {
        (Some(slide), _) => info.words.next.replace("{label}", &slide.label),
        (None, false) => info.words.next.replace("{label}", &info.next_title),
        (None, true) => String::new(),
    };
    if !next_label.is_empty() {
        main = main.child(el().h(px(2.0).max(1.0)).w_full().fill(Color::WHITE.alpha(0.16)).a11y_hidden());
        main = main.child(text(next_label).key("stage-next-label").font(label).color(muted).truncate());
    }
    if let Some(next) = next_text {
        let size = px(58.0).max(4.0);
        main = main.child(
            text(next).key("stage-next").font(TextStyle::new(size, 600.0).lh((size * 1.2).round())).color(muted).w(main_w).wrap().max_lines(2),
        );
    }

    let digits = TextStyle::new(px(118.0).max(8.0), 700.0).features("'tnum' 1");
    let mut side = col().key("stage-side").w(side_w).h_full().padding(pad).gap(px(10.0));
    side = side.child(text(info.words.clock.clone()).font(label).color(muted));
    side = side.child(text(wall_clock(now)).key("stage-clock").font(digits).color(ink));
    if let Some(timer) = &state.countdown {
        let heading = if timer.label.is_empty() { info.words.timer.clone() } else { timer.label.clone() };
        let left = timer.ends_at - now;
        side = side.child(el().h(px(18.0)));
        side = side.child(text(heading).font(label).color(muted).truncate());
        side = side.child(
            text(clock_text(left))
                .key("stage-timer")
                .font(TextStyle::new(px(88.0).max(6.0), 700.0).features("'tnum' 1"))
                .color(if left <= 60.0 { oklch(0.87, 0.17, 95.0) } else { ink }),
        );
    }
    if !info.title.is_empty() {
        side = side.child(el().h(px(18.0)));
        side = side.child(text(info.title.clone()).font(label).color(muted).wrap().max_lines(2));
        if info.count > 0 {
            let place = info.words.place.replace("{index}", &(info.index + 1).min(info.count).to_string()).replace("{count}", &info.count.to_string());
            side = side.child(text(place).font(TextStyle::new(px(36.0).max(3.0), 500.0)).color(ink));
        }
    }

    let body = row()
        .key("stage-body")
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(main)
        .child(el().w(px(2.0).max(1.0)).h_full().fill(Color::WHITE.alpha(0.16)).a11y_hidden())
        .child(side);
    root = root.child(body);

    if !info.chips.is_empty() {
        let mut strip = row().key("stage-chips").w_full().px(pad).py(px(22.0)).gap(px(12.0)).center_y().fill(Color::WHITE.alpha(0.06));
        for (i, chip) in info.chips.iter().enumerate().take(24) {
            let color = group_color(chip.group);
            strip = strip.child(
                el().key(format!("chip-{i}"))
                    .px(px(22.0))
                    .py(px(10.0))
                    .radius(px(12.0))
                    .fill(if chip.active { color } else { Color::WHITE.alpha(0.08) })
                    .child(
                        text(chip.text.clone())
                            .font(TextStyle::new(px(34.0).max(3.0), 700.0))
                            .color(if chip.active { Color::BLACK } else { muted })
                            .align(Align::Center),
                    ),
            );
        }
        root = root.child(strip);
    }
    root
}
