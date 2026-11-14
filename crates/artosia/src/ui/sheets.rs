//! The dialogs: the look, the screens, the phone remote, announcements and importing.

use artosia_slides::frame::ToOutput;
use artosia_slides::look::{Background, Place, presets};
use rune::{Cursor, El, FileDialogOptions, Tx, col, el, row, spacer, text};
use rune_kit::{Dialog, Kit, ScrollBar, SegmentedControl, Select, Switch, TextInput};

use crate::state::{App, Appearance, HeardHit, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::{btn, keycap, section_label};

/// The key of the first control of Settings. Its dialog would open on the close button, the
/// first Tab stop, and a kit switch of segments cannot be named as `autofocus`, so each place
/// that opens Settings asks for the focus by this key (`Tx::focus`).
pub const SETTINGS_FIRST: &str = "settings-first";

fn close(s: &mut App) {
    s.sheet = Sheet::None;
    s.backdrops.returned = false;
}

pub(super) fn setting(k: &Kit, label: &'static str, control: El) -> El {
    row().w_full().h(36.0).gap(space::S3).center_y().child(text(label).font(type_::BODY).color(k.c.text_base).w(150.0)).child(control)
}

fn file_name(path: &str) -> String {
    std::path::Path::new(path).file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default()
}

/// What the projector draws, as the Design tab: the settings in a column on the left and the
/// sample slide as large as the rest of the window allows (UIOVER1, direction B's Tampilan).
pub(super) fn design(s: &App, cx: &mut rune::Cx<App>, k: &Kit, layer: &mut Vec<El>) -> El {
    let look = s.look();
    // The cameras the projector program found on this computer; none in a build without video.
    let cameras = s.link.status().cameras;
    // The look's camera is one on the network.
    let far = matches!(&look.background, Background::Camera(name) if name.contains("://"));
    let tile = 172.0;
    let mut grid = row().key("look-presets").w_full().gap(space::S3).flex_wrap();
    for preset in presets() {
        let on = preset.name == look.name;
        let mut state = s.preview_state(Some(crate::ui::sample_slide()));
        state.look = look.with_preset(preset.clone());
        let chosen = state.look.clone();
        let mut frame = el().size(tile, (tile * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).child(crate::ui::thumb(s, &state, tile));
        if on {
            frame = frame.child(el().absolute(0.0, 0.0).size(tile, (tile * 9.0 / 16.0).round()).radius(k.radius.md).border(2.0, k.c.text_base).a11y_hidden());
        }
        grid = grid.child(
            col()
                .key(format!("look-{}", preset.name))
                .role("button")
                .label(t::look_label(&preset.name))
                .w(tile)
                .gap(space::S1)
                .focusable(true).on_key(crate::ui::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .child(frame.hover(rune::Color::WHITE.alpha(0.06)))
                .child(text(t::look_label(&preset.name)).font(if on { type_::SMALL_STRONG } else { type_::SMALL }).color(if on { k.c.text_base } else { k.c.text_muted }))
                .on_press(move |s: &mut App| s.set_look(chosen.clone())),
        );
    }
    let background = match &look.background {
        // A backdrop from the gallery is named by its title, not by the name its file was saved under.
        Background::Image(path) | Background::Video(path) => {
            let stem = std::path::Path::new(path).file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
            s.backdrops.entry(&stem).map(|entry| entry.title.clone()).unwrap_or_else(|| file_name(path))
        }
        Background::Camera(name) => name.clone(),
        _ => t::LOOK_NO_FILE().to_string(),
    };
    let lines = [2usize, 3, 4, 6, 0];
    let line_at = lines.iter().position(|n| *n == look.max_lines);
    let sizes = [0.8f32, 1.0, 1.2];
    let size_at = sizes.iter().position(|v| (*v - look.size).abs() < 0.05);
    let place_at = match look.place {
        Place::Top => 0,
        Place::Middle => 1,
        Place::Bottom => 2,
    };
    let pick_file = |title: &'static str, kinds: &'static [&'static str], apply: fn(&mut App, &std::path::Path)| {
        move |_: &mut App, tx: &mut Tx| {
            let options = FileDialogOptions { title: title.to_string(), ..Default::default() }.filter(title, kinds);
            tx.open_file(options, move |s: &mut App, result, _| match result {
                Ok(Some(path)) => apply(s, &path),
                Ok(None) => {}
                Err(error) => s.say(error.to_string(), None),
            });
        }
    };
    // What the projector draws, and nothing else: the background, the words, the logo. What
    // belongs to the program itself is in Settings (Microsoft's guidance on settings: "Commands
    // that are part of the typical app workflow ... shouldn't be in a settings page"; the owner,
    // 2026-11-11: nobody could guess that "Look" held the language and the Bibles).
    let mut rows = super::look::rows(s, k, layer);
    let chooser = rows.remove(0);
    // One part at a time, picked by a row of three: the whole dialog then fits a laptop screen,
    // and the colour of the words is one press away, not a scroll past six sample slides (Apple's
    // guidelines on settings group them into panes; a keys-only tester ran out of steps before
    // reaching the text colour, 2026-11-11).
    let part = s.design_part.min(2);
    let parts = SegmentedControl::new(&[t::DESIGN_BACKGROUND(), t::DESIGN_TEXT(), t::LOOK_LOGO()], Some(part))
        .label(t::BTN_LOOK())
        .fit()
        .on_pick(|s: &mut App, picked, _: &mut Tx| s.design_part = picked.min(2))
        .build(k)
        .key("design-parts");
    let pane = col().key(format!("design-part-{part}")).w_full().gap(space::S2);
    let pane = match part {
        0 => pane
        .child(grid)
        .child(setting(
            k,
            t::LOOK_BACKGROUND(),
            row()
                .grow(1.0)
                .min_w(0.0)
                .gap(space::S2)
                .center_y()
                .child({
                    let gallery = btn(t::BACKDROPS_TITLE()).icon(crate::ui::backdrops::mark()).focusable().help(t::CMD_BACKDROPS_DETAIL()).build(k, |s, _| s.open_backdrops()).key("design-backdrops");
                    // Back from the gallery, the focus returns to the button that opened it.
                    if s.backdrops.returned { gallery.autofocus() } else { gallery }
                })
                .child(btn(t::LOOK_CHOOSE_FILE()).icon(&icons::FOLDER).focusable().build(
                    k,
                    pick_file(t::LOOK_CHOOSE_FILE(), &["png", "jpg", "jpeg", "webp", "bmp", "mp4", "mov", "mkv", "webm", "m4v"], |s, path| s.set_background_file(path)),
                ))
                .child(text(background).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0)),
        ))
        // What is wrong with the background is said here too, over the camera rows, where it was just chosen:
        // the status bar is behind this dialog, and the rows' own foot is under the fold.
        .children(s.link.status().media_error.as_ref().map(|error| crate::ui::widgets::wrapped(crate::why::media_problem(error)).key("design-media-problem").font(type_::SMALL).color(theme::warn_ink()).wrap()))
        .children((!cameras.is_empty()).then(|| {
            let current = match &look.background {
                Background::Camera(name) => Some(name.clone()),
                _ => None,
            };
            let mut choices = row().grow(1.0).min_w(0.0).gap(space::S2).flex_wrap();
            for name in &cameras {
                let (on, wanted) = (current.as_deref() == Some(name.as_str()), name.clone());
                // Lit in the accent while it is the background: yellow means hidden or a warning.
                choices = choices.child(btn(name.clone()).icon(&icons::CAMERA).active(on).focusable().build(k, move |s, _| s.set_camera(&wanted)));
            }
            setting(k, t::LOOK_CAMERA(), choices)
        }))
        // A camera on the network, a phone among them: its address is typed or pasted once.
        .children((s.link.status().video || far).then(|| {
            setting(
                k,
                t::LOOK_CAMERA_FAR(),
                row()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(space::S2)
                    .center_y()
                    .child(el().grow(1.0).min_w(0.0).child(TextInput::new(&s.camera_address).width(None).label(t::LOOK_CAMERA_FAR()).placeholder(t::LOOK_CAMERA_FAR_HINT()).on_edit(|s: &mut App| &mut s.camera_address).build(k)))
                    .child(btn(t::LOOK_CAMERA_FAR_USE()).icon(&icons::CAMERA).active(far).focusable().build(k, |s, _| s.use_camera_address()).key("camera-far-use")),
            )
        }))
        .children((s.link.status().video || far).then(|| crate::ui::widgets::wrapped(t::LOOK_CAMERA_FAR_NOTE()).font(type_::SMALL).color(k.c.text_muted).wrap())),
        1 => pane
        .children(rows)
        .child(setting(
            k,
            t::LOOK_LINES(),
            SegmentedControl::new(&["2", "3", "4", "6", t::LOOK_LINES_ALL()], line_at)
                .label(t::LOOK_LINES())
                .fit()
                .on_pick(move |s: &mut App, i, _: &mut Tx| {
                    let mut look = s.look();
                    look.max_lines = lines[i.min(lines.len() - 1)];
                    s.set_look(look);
                })
                .build(k),
        ))
        .child(setting(
            k,
            t::LOOK_SIZE(),
            SegmentedControl::new(&[t::LOOK_SIZE_SMALL(), t::LOOK_SIZE_NORMAL(), t::LOOK_SIZE_LARGE()], size_at)
                .label(t::LOOK_SIZE())
                .fit()
                .on_pick(move |s: &mut App, i, _: &mut Tx| {
                    let mut look = s.look();
                    look.size = sizes[i.min(sizes.len() - 1)];
                    s.set_look(look);
                })
                .build(k),
        ))
        .child(setting(
            k,
            t::LOOK_PLACE(),
            SegmentedControl::new(&[t::LOOK_PLACE_TOP(), t::LOOK_PLACE_MIDDLE(), t::LOOK_PLACE_BOTTOM()], Some(place_at))
                .label(t::LOOK_PLACE())
                .fit()
                .on_pick(|s: &mut App, i, _: &mut Tx| {
                    let mut look = s.look();
                    look.place = [Place::Top, Place::Middle, Place::Bottom][i.min(2)];
                    s.set_look(look);
                })
                .build(k),
        )),
        _ => pane
        .child(setting(
            k,
            t::LOOK_LOGO(),
            row()
                .grow(1.0)
                .min_w(0.0)
                .gap(space::S2)
                .center_y()
                .child(btn(t::LOOK_CHOOSE_LOGO()).icon(&icons::IMAGE).focusable().build(k, pick_file(t::LOOK_CHOOSE_LOGO(), &["png", "jpg", "jpeg", "webp", "bmp"], |s, path| {
                    s.settings.logo = path.to_string_lossy().to_string();
                    s.touch_settings();
                })))
                .child(text(if s.settings.logo.is_empty() { t::LOOK_NO_FILE().to_string() } else { file_name(&s.settings.logo) }).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0)),
        )),
    };
    let sf = theme::surf();
    let body = col().w_full().px(space::S5).py(space::S5).gap(space::S3).child(chooser).child(row().w_full().child(parts)).child(pane);
    let settings = col()
        .key("design-settings")
        .w(620.0)
        .h_full()
        .min_h(0.0)
        .fill(sf.side)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .scroll_edge_fade(crate::theme::size::FADE)
        .child(body)
        .scroll_bar(k);
    // The slide takes the width that is left, up to a projector's own proportions in the height.
    let room = cx.layout_size("design-stage").unwrap_or([640.0, 560.0]);
    let width = (room[0] - 2.0 * space::S6).min((room[1] - 2.0 * space::S6) * 16.0 / 9.0).clamp(320.0, 1280.0).round();
    // While the dropper is lit, a press on the slide takes the colour under it.
    let slide = super::look::stage(s, k, width);
    let slide = if s.colour.dropper {
        slide.cursor(Cursor::Crosshair).border(2.0, theme::accent()).on_drag(|s: &mut App, d: &mut rune::Drag, _: &mut Tx| {
            if d.phase == rune::Phase::Down && d.w > 0.0 && d.h > 0.0 {
                s.drop_colour(d.x / d.w, d.y / d.h);
            }
        })
    } else {
        slide
    };
    let stage = col().key("design-stage").grow(1.0).min_w(0.0).h_full().min_h(0.0).px(space::S6).py(space::S6).center().child(slide);
    row().key("design").w_full().grow(1.0).min_h(0.0).fill(sf.work).child(settings).child(el().w(1.0).h_full().fill(sf.edge).a11y_hidden()).child(stage)
}

/// The program's own settings: what is chosen once and left. Grouped under one-word headings,
/// each change takes effect at once, and help and the version close the list (Microsoft's
/// guidelines for app settings; Apple's on settings: "general settings that affect your overall
/// experience").
fn settings(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let languages: Vec<&crate::languages::Language> = crate::languages::listed().map(|(_, language)| language).collect();
    // The drop-down's list is laid out after the dialog: an overlay finds its anchor only among the elements before it.
    let mut popups: Vec<El> = Vec::new();
    let body = col()
        .w_full()
        .px(space::S4)
        .pb(space::S4)
        .gap(space::S2)
        .child(section_label(k, t::SETTINGS_GENERAL()))
        .child(el().key(SETTINGS_FIRST).w_full().child(setting(
            k,
            t::LOOK_LANGUAGE(),
            // One drop-down whatever the number of languages, each in its own name: a row of segments
            // ran off the dialog's edge with ten.
            Select::new("settings-language", &languages.iter().map(|language| language.name).collect::<Vec<_>>())
                .label(t::LOOK_LANGUAGE())
                .width(220.0)
                .current(languages.iter().find(|language| language.code == t::language_code()).map(|language| language.name))
                .open(s.language_open)
                .on_toggle(|s: &mut App| s.language_open = !s.language_open)
                .on_close(|s: &mut App| s.language_open = false)
                .on_choose(|s: &mut App, name: &str, _: &mut Tx| {
                    s.language_open = false;
                    if let Some((_, language)) = crate::languages::listed().find(|(_, language)| language.name == name) {
                        s.set_language(language.code);
                    }
                })
                .build(k, &mut popups),
        )))
        .child(setting(
            k,
            t::LOOK_WINDOW(),
            SegmentedControl::new(&[t::LOOK_WINDOW_AUTO(), t::LOOK_WINDOW_LIGHT(), t::LOOK_WINDOW_DARK()], Some(s.settings.appearance as usize))
                .label(t::LOOK_WINDOW())
                .fit()
                .on_pick(|s: &mut App, picked, _: &mut Tx| {
                    s.settings.appearance = [Appearance::Auto, Appearance::Light, Appearance::Dark][picked.min(2)];
                    s.touch_settings();
                })
                .build(k),
        ))
        .child(setting(
            k,
            t::SETTINGS_WEEK_START(),
            SegmentedControl::new(&[crate::calendar::weekday_name(0), crate::calendar::weekday_name(1)], Some(usize::from(s.first_weekday().min(1))))
                .label(t::SETTINGS_WEEK_START())
                .fit()
                .on_pick(|s: &mut App, picked, _: &mut Tx| {
                    s.settings.first_weekday = u8::from(picked == 1);
                    s.touch_settings();
                    s.refresh_decks();
                })
                .build(k),
        ))
        .child(el().h(2.0))
        .child(section_label(k, t::KIND_BIBLE()))
        .child(setting(k, t::LOOK_BIBLE(), bible_picker(s, k, false)))
        .child(setting(k, t::LOOK_SECOND_BIBLE(), bible_picker(s, k, true)))
        .child(setting(
            k,
            t::SETTINGS_ADD_BIBLE(),
            row()
                .grow(1.0)
                .min_w(0.0)
                .gap(space::S2)
                .center_y()
                .child(btn(t::BIBLES_CHOOSE_FILE()).icon(&icons::FOLDER).focusable().build(k, |s, _| s.desk.want_file = true).key("look-bible-file"))
                .child(btn(t::BIBLES_MORE()).icon(&icons::GLOBE).focusable().build(k, |s, _| s.open_bibles()).key("look-bible-more")),
        ))
        .child(el().h(2.0))
        .child(section_label(k, t::SETTINGS_SONGS()))
        .child(setting(
            k,
            t::LOOK_LICENCE(),
            el().w(160.0).child(
                TextInput::new(&s.licence_edit).width(None)
                    .label(t::LOOK_LICENCE())
                    .placeholder(t::LOOK_LICENCE_HINT())
                    .on_edit(|s: &mut App| &mut s.licence_edit)
                    .on_change(|s: &mut App, _: &mut Tx| {
                        s.settings.licence = s.licence_edit.text.trim().to_string();
                        s.touch_settings();
                        s.refresh_decks();
                    })
                    .build(k),
            ),
        ))
        .child(row().w_full().min_h(36.0).gap(space::S3).center_y().flex_wrap().child(btn(t::LANGUAGES_ON_SLIDE()).icon(&icons::GLOBE).focusable().build(k, |s, _| s.open_languages(false)).key("settings-languages")).child(text(s.settings.song_languages.iter().map(|code| crate::ui::languages::name_of(code)).collect::<Vec<_>>().join(", ")).font(type_::SMALL).color(k.c.text_muted).truncate().grow(1.0).min_w(0.0)))
        .child(el().h(2.0))
        .child(section_label(k, t::LEARN_SECTION()))
        .child(switch_row(k, "learn-switch", t::LEARN_ON(), t::LEARN_ON_HINT(), s.settings.learn, |s| {
            s.settings.learn = !s.settings.learn;
            s.touch_settings();
            s.search();
        }))
        .child(
            row()
                .w_full()
                .h(36.0)
                .center_y()
                .child(btn(t::LEARN_FORGET()).focusable().disabled(s.store.history_rows() == 0).build(k, |s, _| s.forget_learning())),
        )
        .child(el().h(2.0))
        .child(section_label(k, t::KEYS_TITLE()))
        .child(switch_row(k, "letter-keys-switch", t::LETTER_KEYS(), t::LETTER_KEYS_HINT(), s.settings.letter_keys, |s| {
            s.settings.letter_keys = !s.settings.letter_keys;
            s.touch_settings();
        }))
        .child(row().w_full().h(36.0).center_y().child(btn(t::CMD_KEYS()).icon(&icons::KEYBOARD).focusable().build(k, |s, _| s.sheet = Sheet::Keys)))
        .child(el().h(2.0))
        .child(section_label(k, t::SETTINGS_ABOUT()))
        .child(
            // Three buttons and the version no longer fit one line in every language: the row wraps.
            row()
                .w_full()
                .min_h(36.0)
                .gap(space::S3)
                .center_y()
                .flex_wrap()
                .child(btn(t::CMD_TOUR()).icon(&icons::CHECK).focusable().build(k, |s, _| {
                    s.sheet = Sheet::None;
                    s.start_tour();
                }))
                .child(btn(t::CMD_FIRST_RUN()).icon(&icons::GLOBE).focusable().build(k, |s, _| {
                    s.sheet = Sheet::None;
                    s.start_onboarding();
                }))
                // The Store updates the program itself, so its edition offers no check of its own.
                .children((!crate::store::packaged()).then(|| btn(t::CMD_UPDATE()).icon(&icons::IMPORT).focusable().build(k, |s, _| s.run(crate::palette::Command::CheckUpdate))))
                .child(text(t::about_version(crate::update::VERSION)).font(type_::SMALL).color(k.c.text_muted)),
        );
    let body = col().w_full().max_h((tall - 84.0).max(240.0)).scroll(rune::ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(body).scroll_bar(k);
    Dialog::new("settings", t::BTN_SETTINGS()).open(true).width(620.0).fit().on_close(close).child(body).build(k, layer);
    layer.extend(popups);
}

/// The translations as a row of choices; the second picker starts with "None".
fn bible_picker(s: &App, k: &Kit, second: bool) -> El {
    // A choice reads as people call the Bible ("TB"); what is kept is the module's code.
    let (mut labels, mut codes): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    if second {
        labels.push(t::LOOK_NONE().to_string());
        codes.push(String::new());
    }
    for file in &s.bible_files {
        labels.push(file.short.clone());
        codes.push(file.code.clone());
    }
    let current = if second { &s.settings.second_bible } else { &s.settings.bible };
    let at = codes.iter().position(|code| code == current).or(if second { Some(0) } else { None });
    SegmentedControl::new(&labels, at)
        .label(if second { t::LOOK_SECOND_BIBLE() } else { t::LOOK_BIBLE() })
        .fit()
        .on_pick(move |s: &mut App, i, _: &mut Tx| {
            let code = codes.get(i).cloned().unwrap_or_default();
            if second {
                s.set_second_bible(if i == 0 { "" } else { &code });
            } else {
                s.set_bible(&code);
            }
        })
        .build(k)
}

/// A line with a switch at its end: what it turns on, and a word of help under it.
fn switch_row(k: &Kit, key: &'static str, label: &'static str, hint: &'static str, on: bool, flip: fn(&mut App)) -> El {
    switch_row_at(k, key, label, hint, on, flip, false)
}

/// The same; `lead` makes it the control its dialog opens on.
fn switch_row_at(k: &Kit, key: &'static str, label: &'static str, hint: &'static str, on: bool, flip: fn(&mut App), lead: bool) -> El {
    let mut words = col().grow(1.0).min_w(0.0).child(text(label).font(type_::BODY_STRONG).color(k.c.text_base));
    if !hint.is_empty() {
        words = words.child(text(hint).font(type_::SMALL).color(k.c.text_muted));
    }
    row().key(key).w_full().min_h(40.0).gap(space::S3).center_y().child(words).child({
        let switch = Switch::new(on).label(label).hide_label().on_toggle(flip).build(k);
        if lead { switch.autofocus() } else { switch }
    })
}

/// A value with a button either side of it, for a setting that moves in steps.
fn stepper(k: &Kit, label: &'static str, shown: String, which: usize, step: f32) -> El {
    row()
        .w_full()
        .h(36.0)
        .gap(space::S2)
        .center_y()
        .child(text(label).font(type_::BODY).color(k.c.text_base).grow(1.0))
        .child(btn(t::LESS()).icon(&icons::MINUS).icon_only().small().focusable().build(k, move |s, _| s.nudge_colour(which, -step)))
        .child(el().w(56.0).center().child(text(shown).font(type_::BODY_STRONG).color(k.c.text_base)))
        .child(btn(t::MORE()).icon(&icons::PLUS).icon_only().small().focusable().build(k, move |s, _| s.nudge_colour(which, step)))
}

/// Squaring up and tinting the projector picture: pick a corner on the small screen, nudge it
/// with the arrows, and watch the wall.
fn wall(s: &App, k: &Kit) -> El {
    let warp = s.settings.warp;
    let (width, height, dot) = (208.0f32, 117.0f32, 18.0f32);
    let names = [t::CORNER_TOP_LEFT(), t::CORNER_TOP_RIGHT(), t::CORNER_BOTTOM_RIGHT(), t::CORNER_BOTTOM_LEFT()];
    let mut screen = el().key("wall-screen").size(width + dot, height + dot).child(el().absolute(dot / 2.0, dot / 2.0).size(width, height).radius(k.radius.sm).fill(k.c.layer_03).border(1.0, k.c.border_base));
    for (at, corner) in warp.corners.iter().enumerate() {
        let picked = at == s.warp_corner;
        // The dots sit where the corners land, drawn a little inside the frame so all four stay in reach.
        let (x, y) = (corner[0].clamp(-0.04, 1.04) * width, corner[1].clamp(-0.04, 1.04) * height);
        screen = screen.child(
            el().key(format!("corner-{at}"))
                .role("button")
                .label(names[at])
                .absolute(x, y)
                .size(dot, dot)
                .radius(dot / 2.0)
                .fill(if picked { theme::accent() } else { k.c.layer_04 })
                .border(2.0, if picked { theme::accent() } else { k.c.text_muted })
                .hover(k.c.hover)
                .focusable(true).on_key(crate::ui::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .help(names[at])
                .on_press(move |s: &mut App| s.warp_corner = at),
        );
    }
    let step = 0.005;
    let arrow = |label: &'static str, glyph: &'static rune::Icon, dx: f32, dy: f32| btn(label).icon(glyph).icon_only().focusable().build(k, move |s, _| s.nudge_corner(dx, dy));
    let pad = col()
        .gap(space::S1)
        .child(text(names[s.warp_corner.min(3)]).font(type_::SMALL_STRONG).color(k.c.text_muted))
        .child(row().gap(space::S1).child(el().size(32.0, 32.0)).child(arrow(t::NUDGE_UP(), &icons::ARROW_UP, 0.0, -step)))
        .child(row().gap(space::S1).child(arrow(t::NUDGE_LEFT(), &icons::ARROW_LEFT, -step, 0.0)).child(arrow(t::NUDGE_DOWN(), &icons::ARROW_DOWN, 0.0, step)).child(arrow(t::NUDGE_RIGHT(), &icons::ARROW_RIGHT, step, 0.0)));
    let percent = |value: f32| format!("{}%", (value * 100.0).round() as i32);
    col()
        .key("wall")
        .w_full()
        .gap(space::S2)
        .child(row().w_full().center_y().child(text(t::WALL()).font(type_::BODY_STRONG).color(k.c.text_base).grow(1.0)).children((!warp.is_identity()).then(|| btn(t::WALL_RESET()).small().ghost().focusable().build(k, |s, _| s.reset_warp()))))
        .child(crate::ui::widgets::wrapped(t::WALL_HINT()).font(type_::SMALL).color(k.c.text_muted).wrap())
        .child(row().w_full().gap(space::S5).center_y().child(screen).child(pad))
        .child(stepper(k, t::WALL_BRIGHTNESS(), format!("{:+}", (warp.brightness * 100.0).round() as i32), 0, 0.02))
        .child(stepper(k, t::WALL_CONTRAST(), percent(warp.contrast), 1, 0.05))
        .child(stepper(k, t::WALL_COLOUR(), percent(warp.saturation), 2, 0.05))
}

fn screens(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let status = s.screens_status();
    let started = s.link.is_started() || s.demo_screens;
    let options = s.output_options();
    let mut body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(crate::ui::tour::help_row(k, crate::guide::lessons::Feature::Screens));
    if !started {
        body = body.child(crate::ui::widgets::wrapped(t::SCREENS_OFF()).font(type_::BODY).color(k.c.text_muted).wrap());
    } else {
        // The switch works whatever the output is doing, so the projector can always be turned off.
        body = body.child(switch_row(k, "projector-switch", t::SCREENS_SWITCH(), "", !options.off, |s| {
            let on = s.settings.projector_off;
            s.set_projector(on);
        }));
        let stuck = status.waiting_since.is_some_and(|since| since.elapsed() > std::time::Duration::from_secs(8));
        if let Some(problem) = &status.problem {
            body = body.child(crate::ui::widgets::wrapped(problem.clone()).font(type_::BODY).color(theme::warn_ink()).wrap());
        } else if !status.connected {
            body = body.child(crate::ui::widgets::wrapped(if stuck { t::OUTPUT_STUCK() } else { t::OUTPUT_STARTING() }).font(type_::BODY).color(k.c.text_muted).wrap());
        }
        if status.connected && status.displays.len() < 2 {
            body = body.child(crate::ui::widgets::wrapped(t::SCREENS_ONE()).font(type_::BODY).color(k.c.text_muted).wrap()).child(switch_row(k, "here-switch", t::SCREENS_HERE(), t::SCREENS_HERE_HINT(), options.here && !options.off, |s| {
                let mut next = s.output_options();
                next.here = !(next.here && !next.off);
                next.off = false;
                s.settings_screens(next);
            }));
        }
        if status.connected && status.displays.len() >= 2 {
            // The guide's Screens lesson rings the first monitor's choices.
            let first_choice = status.displays.iter().position(|display| !display.primary);
            for (index, display) in status.displays.iter().enumerate() {
                let role = if status.audience == Some(index) {
                    0
                } else if status.stage == Some(index) {
                    1
                } else {
                    2
                };
                body = body.child(
                    row()
                        .key(format!("screen-{index}"))
                        .w_full()
                        .h(64.0)
                        .px(space::S3)
                        .gap(space::S3)
                        .center_y()
                        .radius(k.radius.lg)
                        .fill(k.c.layer_02)
                        .child(el().size(64.0, 40.0).center().radius(k.radius.md).fill(k.c.layer_04).child(text((index + 1).to_string()).font(type_::HEADING).color(k.c.text_base)))
                        .child(
                            col().grow(1.0).min_w(0.0).child(text(t::screen_title(index + 1, display.primary)).font(type_::BODY_STRONG).color(k.c.text_base).truncate()).child(
                                text(t::screen_detail(display.width, display.height, display.scale)).font(type_::SMALL).color(k.c.text_muted).truncate(),
                            ),
                        )
                        // The screen Artosia's own window is on takes no role: a full-screen
                        // projector over it would lock the volunteer out.
                        .child(if display.primary {
                            text(t::SCREEN_OWN()).font(type_::SMALL).color(k.c.text_muted)
                        } else {
                            SegmentedControl::new(&[t::SCREEN_PROJECTOR(), t::SCREEN_STAGE(), t::SCREEN_UNUSED()], Some(role))
                                .label(t::screen_name(index + 1))
                                .fit()
                                .on_pick(move |s: &mut App, picked, _: &mut Tx| {
                                    let mut next = s.output_options();
                                    if next.audience == Some(index) {
                                        next.audience = None;
                                    }
                                    if next.stage == Some(index) {
                                        next.stage = None;
                                    }
                                    match picked {
                                        0 => {
                                            next.audience = Some(index);
                                            next.off = false;
                                        }
                                        1 => next.stage = Some(index),
                                        _ => {}
                                    }
                                    s.settings_screens(next);
                                })
                                .build(k)
                                .key(if first_choice == Some(index) { "screen-jobs".to_string() } else { format!("screen-jobs-{index}") })
                        }),
                );
            }
        }
        if status.connected {
            body = body.child(
                row()
                    .w_full()
                    .gap(space::S2)
                    .center_y()
                    .child(text(t::SCREENS_STAGE_WINDOW()).font(type_::BODY).color(k.c.text_base))
                    .child(
                        Switch::new(options.stage_window)
                            .label(t::SCREENS_STAGE_WINDOW())
                            .hide_label()
                            .on_toggle(|s: &mut App| {
                                let mut next = s.output_options();
                                next.stage_window = !next.stage_window;
                                s.settings_screens(next);
                            })
                            .build(k)
                            .key("stage-window-switch"),
                    )
                    .child(spacer())
                    .child(btn(t::SCREENS_IDENTIFY()).icon(&icons::SCREEN).focusable().build(k, |s, _| {
                        s.identify = !s.identify;
                        s.link.send(ToOutput::Identify(s.identify));
                    })),
            );
        }
    }
    if started {
        body = body.child(crate::ui::widgets::divider(k)).child(wall(s, k));
    }
    Dialog::new("screens", t::BTN_SCREENS()).open(true).width(660.0).fit().on_close(|s: &mut App| {
        if s.identify {
            s.identify = false;
            s.link.send(ToOutput::Identify(false));
        }
        s.sheet = Sheet::None;
    }).child(body).build(k, layer);
}

fn remote(s: &App, k: &Kit, layer: &mut Vec<El>) {
    // On while the firewall step is still answering: the remote starts after it.
    let on = s.remote.is_some() || s.firewall != crate::firewall::Standing::Idle;
    let mut body = col().child(crate::ui::tour::help_row(k, crate::guide::lessons::Feature::Phone)).w_full().px(space::S4).pb(space::S4).gap(space::S3).child(
        row()
            .w_full()
            .gap(space::S3)
            .center_y()
            .child(text(t::REMOTE_SWITCH_DETAIL()).font(type_::BODY).color(k.c.text_base).grow(1.0))
            .child(
                Switch::new(on)
                    .label(t::REMOTE_SWITCH())
                    .hide_label()
                    .on_toggle(|s: &mut App| {
                        let on = !(s.remote.is_some() || s.firewall != crate::firewall::Standing::Idle);
                        s.set_remote(on);
                    })
                    .build(k)
                    .key("remote-switch")
                    .autofocus(),
            ),
    );
    // What Windows is about to ask comes before its box; the warning replaces it when the answer was no.
    match s.firewall {
        crate::firewall::Standing::Asking => body = body.child(crate::ui::widgets::wrapped(t::REMOTE_FIREWALL_ASKING()).font(type_::BODY).color(k.c.text_muted).wrap()),
        crate::firewall::Standing::Refused => body = body.child(crate::ui::widgets::wrapped(t::REMOTE_FIREWALL_REFUSED()).font(type_::BODY).color(theme::warn_ink()).wrap()),
        _ => {}
    }
    if let Some(problem) = &s.remote_problem {
        body = body.child(crate::ui::widgets::wrapped(t::remote_failed(problem)).font(type_::BODY).color(theme::warn_ink()).wrap());
    }
    if let (Some(url), Some(server)) = (s.remote_url(), s.remote.as_ref()) {
        let mut card = row().w_full().padding(space::S4).gap(space::S4).center_y().radius(k.radius.lg).fill(k.c.layer_02);
        if let Some(code) = &s.remote_qr {
            card = card.child(el().size(168.0, 168.0).padding(8.0).radius(k.radius.md).fill(rune::Color::WHITE).child(el().size(152.0, 152.0).pixels(code)));
        }
        let plain = url.split('?').next().unwrap_or(&url).to_string();
        card = card.child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(space::S2)
                .child(section_label(k, t::REMOTE_SCAN()))
                .child(text(plain).font(type_::BODY_STRONG).color(k.c.text_base).selectable(true).wrap_anywhere())
                .child(el().h(space::S1))
                .child(section_label(k, t::REMOTE_PIN()))
                .child(text(s.remote_pin.clone()).font(type_::TIMER).color(k.c.text_base))
                .child(text(t::phones(server.paired_count())).font(type_::SMALL).color(k.c.text_muted)),
        );
        body = body.child(card);
    }
    Dialog::new("remote", t::CMD_REMOTE()).open(true).width(520.0).fit().on_close(close).child(body).build(k, layer);
}

fn import(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let mut body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3);
    match &s.easislides {
        Some(folder) => {
            body = body
                .child(crate::ui::widgets::wrapped(t::IMPORT_FOUND()).font(type_::BODY).color(k.c.text_base).wrap())
                .child(text(folder.display().to_string()).font(type_::SMALL).color(k.c.text_muted).truncate_middle())
                .child(row().w_full().gap(space::S2).child(spacer()).child(btn(t::BTN_CANCEL()).focusable().build(k, |s, _| s.sheet = Sheet::None)).child(btn(t::BTN_BRING_OVER()).primary().focusable().build(k, |s, _| s.import_easislides()).autofocus()));
        }
        None => body = body.child(crate::ui::widgets::wrapped(t::IMPORT_NOT_FOUND()).font(type_::BODY).color(k.c.text_muted).wrap()),
    }
    Dialog::new("import", t::CMD_IMPORT()).open(true).width(480.0).fit().on_close(close).child(body).build(k, layer);
}

/// One group of the key sheet: a heading and its rows.
fn key_group(k: &Kit, heading: &'static str, keys: &[(&'static str, &'static str)], rows: &[(&'static str, &'static str)]) -> El {
    let mut group = col().w_full().gap(2.0).child(section_label(k, heading));
    // The keys are the same in every language and are drawn as the keyboard prints them;
    // only what a key does is written per language.
    for ((keys, _), (_, does)) in keys.iter().zip(rows) {
        // The row about section letters shows this language's own letters.
        let own = t::section_keys();
        let keys: &str = if *keys == "C V B" { own.as_str() } else { keys };
        // A Mac has a Command key for each of the Live pane's switches; the F key works too.
        let on_a_mac = crate::ui::widgets::switch_key(keys, cfg!(target_os = "macos"));
        let both = format!("{on_a_mac} or {keys}");
        let keys: &str = if on_a_mac == keys { keys } else { both.as_str() };
        group = group.child(row().w_full().h(24.0).gap(space::S3).center_y().child(el().w(132.0).child(row().child(keycap(k, keys, false)))).child(text(*does).font(type_::BODY).color(k.c.text_base).truncate().grow(1.0)));
    }
    group
}

fn keys(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    // Two columns of about the same height: the keys of a service, and everything else. On a
    // laptop screen the whole sheet shows; a lower window scrolls the list under a title and
    // a close button that stay (the sheet once ran off both edges of a 768 point window).
    let rows = row()
        .w_full()
        .gap(space::S6)
        .child(col().grow(1.0).min_w(0.0).child(key_group(k, t::KEYS_SHOWING(), t::KEYS_SHOWING_KEYS(), t::KEYS_SHOWING_ROWS())))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(space::S3)
                .child(key_group(k, t::KEYS_FINDING(), t::KEYS_FINDING_KEYS(), t::KEYS_FINDING_ROWS()))
                .child(key_group(k, t::KEYS_PLAN(), t::KEYS_PLAN_KEYS(), t::KEYS_PLAN_ROWS()))
                .child(key_group(k, t::KEYS_EDITING(), t::KEYS_EDITING_KEYS(), t::KEYS_EDITING_ROWS())),
        );
    let _ = s;
    let list = col().key("keys-list").w_full().max_h((tall - 200.0).max(220.0)).scroll(rune::ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(rows).scroll_bar(k);
    // What has no key of its own is in the search, and the sheet says so where the eye ends
    // up: a keys-only tester read this sheet and still could not find how to copy a plan.
    let tour = row()
        .w_full()
        .gap(space::S3)
        .center_y()
        .child(btn(t::CMD_TOUR()).icon(&icons::PLAY).focusable().build(k, |s, _| s.start_tour()))
        .child(row().grow(1.0).min_w(0.0).gap(space::S2).center_y().child(keycap(k, "Ctrl K", false)).child(text(t::KEYS_MORE()).font(type_::BODY).color(k.c.text_muted).truncate().grow(1.0)));
    let body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(list).child(tour);
    Dialog::new("keys", t::KEYS_TITLE()).open(true).width(1060.0).fit().on_close(close).child(body).build(k, layer);
}

fn stream(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let running = s.stream.as_ref().map(|stream| stream.status());
    let on = running.is_some();
    let mut body = col().w_full().px(space::S4).pb(space::S4).gap(space::S3).child(crate::ui::tour::help_row(k, crate::guide::lessons::Feature::Stream));
    if !crate::stream::available() {
        body = body.child(crate::ui::widgets::wrapped(t::STREAM_MISSING()).font(type_::BODY).color(k.c.text_muted).wrap());
        return Dialog::new("stream", t::CMD_STREAM()).open(true).width(560.0).fit().on_close(close).child(body).build(k, layer);
    }
    // The dialog opens on what the person came for: the address while there is none
    // (`App::open_stream`), the button that starts or stops once there is. Rune would pick
    // the close button.
    let ready = on || !s.stream_address.text.trim().is_empty();
    let first = |control: El, lead: bool| if lead { control.autofocus() } else { control };
    body = body
        .child(crate::ui::widgets::wrapped(t::STREAM_HINT()).font(type_::BODY).color(k.c.text_muted).wrap())
        .child(
            col()
                .key("stream-fields")
                .w_full()
                .gap(space::S3)
                .child(col().w_full().gap(space::S1).child(section_label(k, t::STREAM_ADDRESS())).child(
                    TextInput::new(&s.stream_address).width(None).label(t::STREAM_ADDRESS()).placeholder(t::STREAM_ADDRESS_HINT()).disabled(on).on_edit(|s: &mut App| &mut s.stream_address).build(k),
                ))
                .child(col().w_full().gap(space::S1).child(section_label(k, t::STREAM_KEY())).child(
                    TextInput::new(&s.stream_key).width(None).label(t::STREAM_KEY()).secure().disabled(on).on_edit(|s: &mut App| &mut s.stream_key).build(k),
                )),
        )
        .child(switch_row(k, "record-switch", t::STREAM_RECORD(), t::STREAM_RECORD_HINT(), s.settings.stream_record, |s| {
            if s.stream.is_none() {
                s.settings.stream_record = !s.settings.stream_record;
                s.touch_settings();
            }
        }));
    // What the helper says about itself, in words: starting, live for how long, or what went wrong.
    let words = running.as_ref().map(|status| match status.state.as_str() {
        "live" => t::stream_live(status.seconds_live, status.frames_dropped),
        "reconnecting" => t::STREAM_RECONNECTING().to_string(),
        "stopped" => t::STREAM_STOPPING().to_string(),
        _ => t::STREAM_STARTING().to_string(),
    });
    let problem = running.as_ref().and_then(|status| status.problem.clone()).or_else(|| s.stream_problem.clone());
    let live = running.as_ref().is_some_and(|status| status.state == "live");
    let mut foot = row().w_full().gap(space::S3).center_y();
    if let Some(words) = words {
        foot = foot.child(text(words).font(if live { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate().grow(1.0));
    } else {
        foot = foot.child(spacer());
    }
    foot = foot.child(first(btn(if on { t::BTN_STREAM_STOP() } else { t::BTN_STREAM_START() }).icon(&icons::STREAM).toggle(on).tall().focusable().build(k, |s, _| s.toggle_stream()).key("stream-start"), ready));
    if let Some(problem) = problem {
        body = body.child(crate::ui::widgets::wrapped(problem).font(type_::BODY).color(theme::warn_ink()).wrap());
    }
    body = body.child(foot);
    Dialog::new("stream", t::CMD_STREAM()).open(true).width(560.0).fit().on_close(close).child(body).build(k, layer);
}

/// The beats per minute the room is playing, and a dot that lights on each beat. With no steady
/// pulse the dot is grey and there is no number.
fn tempo_row(s: &App, k: &Kit) -> El {
    let phase = s.beat_phase(s.clock);
    let still = s.motion_reduced.load(std::sync::atomic::Ordering::Relaxed);
    let strength = match phase {
        Some(phase) if !still => 0.3 + 0.7 * crate::actions::beat_pulse(phase),
        _ => 1.0,
    };
    let dot = el().key("beat-dot").w(12.0).h(12.0).radius(6.0).fill(if phase.is_some() { theme::accent() } else { k.c.layer_03 }).layer_opacity(strength);
    let mut line = row().key("tempo").w_full().min_h(32.0).gap(space::S3).center_y().child(dot);
    if let (Some(bpm), Some(_)) = (s.tempo_shown(), phase) {
        use crate::state::TempoCount;
        let counts = [TempoCount::Half, TempoCount::AsHeard, TempoCount::Double];
        // The sound cannot say whether a pulse is the beat or half of it, so a person can: the
        // number and the pulse follow the choice until the tempo changes.
        let count = SegmentedControl::new(&[t::LISTEN_TEMPO_HALF(), t::LISTEN_TEMPO_AS_HEARD(), t::LISTEN_TEMPO_DOUBLE()], counts.iter().position(|count| *count == s.tempo_count))
            .label(t::LISTEN_TEMPO())
            .fit()
            .on_pick(move |s: &mut App, picked, _: &mut Tx| s.set_tempo_count(counts[picked.min(2)]))
            .build(k);
        line = line
            .child(text((bpm.round() as i32).to_string()).font(type_::TITLE.features("'tnum' 1")).color(k.c.text_base))
            .child(text(t::LISTEN_BPM()).font(type_::SMALL).color(k.c.text_muted))
            .child(spacer())
            .child(count);
    }
    col().w_full().gap(space::S1).child(section_label(k, t::LISTEN_TEMPO())).child(line)
}

/// A folder as its last two names, with what came before cut away: the end says where it is,
/// and a path cut at its end says nothing.
fn folder_words(folder: &std::path::Path) -> String {
    let names: Vec<String> = folder.iter().map(|part| part.to_string_lossy().to_string()).collect();
    if names.len() <= 3 {
        return folder.to_string_lossy().to_string();
    }
    let between = std::path::MAIN_SEPARATOR;
    format!("...{between}{}{between}{}", names[names.len() - 2], names[names.len() - 1])
}

/// The second part of the Listen dialog: what is kept of what is heard, where, and the four
/// things to do with it. Keeping is off until its switch is pressed: a service holds people's
/// voices (owner, 2026-11-11: "make the app collect telemetry audio and the attempts for us
/// to learn"; nothing is sent, the folder is the whole of it).
fn recordings_part(s: &App, k: &Kit) -> El {
    let on = s.settings.keep_recordings;
    let folder = s.recordings_folder().map(|folder| folder_words(&folder)).unwrap_or_default();
    let kept = match s.recordings.used {
        0 => t::REC_NONE().to_string(),
        bytes => t::rec_kept(&crate::recordings::size_words(bytes)),
    };
    let nothing = s.recordings.used == 0;
    let place = row()
        .key("rec-where")
        .w_full()
        .min_h(40.0)
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .fill(k.c.layer_02)
        .child(rune::icon(&icons::FOLDER, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(text(folder).font(type_::BODY).color(k.c.text_base).truncate().grow(1.0).min_w(0.0))
        .child(text(kept).font(type_::BODY_STRONG.features("'tnum' 1")).color(k.c.text_base));
    let pair = |left: El, right: El| row().w_full().gap(space::S2).child(left).child(right);
    let open = btn(t::REC_OPEN()).icon(&icons::FOLDER).grow().focusable().build(k, |s, _| s.open_recordings_folder());
    let moved = btn(t::REC_MOVE()).icon(&icons::ARROW_RIGHT).grow().focusable().build(k, |_, tx| {
        tx.open_folder(FileDialogOptions { title: t::REC_MOVE().to_string(), ..Default::default() }, |s: &mut App, picked, _| {
            if let Ok(Some(folder)) = picked {
                s.move_recordings(folder);
            }
        });
    });
    let saving = s.recordings.saving;
    let save = btn(if saving { t::REC_SAVING() } else { t::REC_SAVE() }).icon(&icons::EXPORT).grow().disabled(nothing || saving).focusable().build(k, |_, tx| {
        let options = FileDialogOptions { title: t::REC_SAVE().to_string(), file_name: Some("Artosia recordings.zip".to_string()), ..Default::default() }.filter("zip", &["zip"]);
        tx.save_file(options, |s: &mut App, picked, _| {
            if let Ok(Some(file)) = picked {
                s.save_recordings(file);
            }
        });
    });
    let delete = btn(t::REC_DELETE()).icon(&icons::TRASH).grow().disabled(nothing).focusable().build(k, |s, _| s.delete_recordings());
    let mut part = col()
        .key("recordings")
        .w_full()
        .gap(space::S3)
        .child(switch_row(k, "rec-switch", t::REC_SWITCH(), t::REC_HINT(), on, |s| {
            let on = !s.settings.keep_recordings;
            s.set_keep_recordings(on);
        }))
        .child(place)
        .child(col().w_full().gap(space::S2).child(pair(open.key("rec-open"), moved.key("rec-move"))).child(pair(save.key("rec-save"), delete.key("rec-delete"))));
    if s.recordings.failed {
        part = part.child(crate::ui::widgets::wrapped(t::REC_FAILED()).font(type_::BODY).color(theme::warn_ink()).wrap());
    }
    part
}

fn listen(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    let on = s.listener.is_some() || s.demo_listening;
    let level = f32::from_bits(s.listen_level.load(std::sync::atomic::Ordering::Relaxed));
    // Practice keeps nothing, so it shows no second part.
    let keeps = s.recordings_folder().is_some();
    let part = if keeps { s.listen_part.min(1) } else { 0 };
    // The parts and the switch stay; what is under them scrolls when the window is lower
    // than the dialog (six microphones, a tempo and three heard songs passed 768 points).
    let mut head = col().child(crate::ui::tour::help_row(k, crate::guide::lessons::Feature::Listen)).w_full().px(space::S4).gap(space::S3);
    if keeps {
        // Two parts, as Design has three: with the recordings under the rest the dialog was
        // taller than a laptop's screen.
        head = head.child(
            SegmentedControl::new(&[t::LISTEN_PART_LISTENING(), t::LISTEN_PART_RECORDINGS()], Some(part))
                .label(t::CMD_LISTEN())
                .fit()
                .on_pick(|s: &mut App, picked, _: &mut Tx| {
                    s.listen_part = picked.min(1);
                    s.refresh_recordings();
                })
                .build(k)
                .key("listen-parts"),
        );
    }
    if part == 1 {
        let body = head.pb(space::S4).child(recordings_part(s, k));
        Dialog::new("listen", t::CMD_LISTEN()).open(true).width(560.0).fit().on_close(close).child(body).build(k, layer);
        return;
    }
    head = head.child(switch_row_at(k, "listen-switch", t::LISTEN_SWITCH(), t::LISTEN_HINT(), on, |s| {
        let on = s.listener.is_none();
        s.set_listening(on);
    }, true));
    let mut body = col().w_full().px(space::S4).pt(space::S3).pb(space::S4).gap(space::S3);
    // The sources as a row of choices: the computer's own microphone first, then the others, then
    // what the computer itself plays. Each carries what the setting holds (empty is the default).
    let mut sources: Vec<(String, String)> = vec![(t::LISTEN_DEFAULT().to_string(), String::new())];
    sources.extend(s.microphones.iter().map(|device| (device.name.clone(), device.id.clone())));
    if artosia_listen::system_audio_available() {
        sources.push((t::LISTEN_SYSTEM_AUDIO().to_string(), artosia_listen::SYSTEM_AUDIO.to_string()));
    }
    // Older settings saved a microphone's name, newer ones its id.
    let saved = s.settings.listen_device.as_str();
    let picked = sources
        .iter()
        .position(|(label, value)| !saved.is_empty() && (value == saved || label == saved))
        .unwrap_or(0);
    let mut list = col().key("microphones").role("list").label(t::LISTEN_MICROPHONE()).w_full().gap(2.0);
    for (at, (name, wanted)) in sources.iter().enumerate() {
        let chosen = at == picked;
        let wanted = wanted.clone();
        list = list.child(
            row()
                .key(format!("microphone-{at}"))
                .role("button")
                .label(name.clone())
                .w_full()
                .h(36.0)
                .px(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(if chosen { k.c.layer_04 } else { rune::Color::TRANSPARENT })
                .hover(k.c.hover)
                .focusable(true).on_key(crate::ui::widgets::enter_key)
                .cursor(Cursor::Pointer)
                .child(text(name.clone()).font(if chosen { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base).truncate())
                .on_press(move |s: &mut App| {
                    s.settings.listen_device = wanted.clone();
                    s.touch_settings();
                    if s.listener.is_some() {
                        s.set_listening(true);
                    }
                }),
        );
    }
    body = body.child(col().w_full().gap(space::S1).child(section_label(k, t::LISTEN_MICROPHONE())).child(list));
    body = body.child(setting(
        k,
        t::LISTEN_LANGUAGE(),
        SegmentedControl::new(&[t::LISTEN_INDONESIAN(), t::LISTEN_ENGLISH()], Some(usize::from(s.settings.listen_language == "en")))
            .label(t::LISTEN_LANGUAGE())
            .fit()
            .on_pick(|s: &mut App, picked, _: &mut Tx| {
                s.settings.listen_language = if picked == 1 { "en" } else { "id" }.to_string();
                s.touch_settings();
                if s.listener.is_some() {
                    s.set_listening(true);
                }
            })
            .build(k),
    ));
    if on {
        // How loud the room is: a bar that fills, so a dead microphone is plain to see.
        let width = 496.0;
        body = body.child(
            col().w_full().gap(space::S1).child(section_label(k, t::LISTEN_LEVEL())).child(
                el().key("level").w(width).h(10.0).radius(5.0).fill(k.c.layer_03).child(el().w((width * level.clamp(0.0, 1.0)).max(2.0)).h(10.0).radius(5.0).fill(theme::accent())),
            ),
        );
    }
    if on {
        body = body.child(tempo_row(s, k));
    }
    body = body.child(switch_row(k, "beat-switch", t::LISTEN_BEAT_BLINK(), t::LISTEN_BEAT_BLINK_HINT(), s.settings.beat_blink, |s| {
        s.settings.beat_blink = !s.settings.beat_blink;
        s.touch_settings();
    }));
    // A build that can hear words needs its model; a PC without one is offered it here.
    if App::speech_built() {
        use crate::speech_model::Model;
        let row_of = |words: String, button: Option<El>| row().key("speech-model").w_full().min_h(40.0).gap(space::S3).center_y().child(rune::icon(&icons::IMPORT, 16.0).color(k.c.text_muted).a11y_hidden()).child(crate::ui::widgets::wrapped(words).font(type_::BODY).color(k.c.text_base).wrap().grow(1.0).min_w(0.0)).children(button);
        let get = || btn(t::BIBLES_DOWNLOAD()).icon(&icons::IMPORT).focusable().build(k, |s, _| s.download_model());
        match &s.model {
            Model::Missing => body = body.child(row_of(t::LISTEN_MODEL_MISSING().to_string(), Some(get()))),
            Model::Downloading => body = body.child(row_of(t::BIBLES_DOWNLOADING().to_string(), None)),
            Model::Failed(why) => body = body.child(row_of(t::listen_model_failed(why), Some(get()))),
            Model::Ready | Model::Unknown => {}
        }
        // A PC that read the first model quickly is offered the better one, here too.
        use crate::speech_model::Better;
        let better = || btn(t::BIBLES_DOWNLOAD()).icon(&icons::IMPORT).focusable().build(k, |s, _| s.download_better());
        match &s.better {
            Better::Offered => body = body.child(row_of(t::LISTEN_MODEL_BETTER().to_string(), Some(better())).key("speech-better")),
            Better::Downloading => body = body.child(row_of(t::BIBLES_DOWNLOADING().to_string(), None).key("speech-better")),
            Better::Failed(why) => body = body.child(row_of(t::listen_model_failed(why), Some(better())).key("speech-better")),
            Better::No => {}
        }
    }
    if let Some(problem) = &s.listen_problem {
        body = body.child(crate::ui::widgets::wrapped(problem.clone()).font(type_::BODY).color(theme::warn_ink()).wrap());
    }
    if let Some(last) = s.heard.last() {
        body = body.child(col().w_full().gap(space::S1).child(section_label(k, t::LISTEN_HEARD())).child(crate::ui::widgets::wrapped(last.clone()).font(type_::BODY).color(k.c.text_base).wrap()));
    }
    for (at, hit) in s.heard_hits.iter().enumerate() {
        let (title, glyph) = match hit {
            HeardHit::Song(song) => (s.songs.get(*song).map(|song| song.title.clone()).unwrap_or_default(), &icons::SONG),
            HeardHit::Passage(_, title) => (title.clone(), &icons::BOOK),
        };
        let chosen = hit.clone();
        body = body.child(
            row()
                .key(format!("heard-{at}"))
                .w_full()
                .h(40.0)
                .px(space::S3)
                .gap(space::S3)
                .center_y()
                .radius(k.radius.md)
                .fill(k.c.layer_02)
                .child(rune::icon(glyph, 16.0).color(k.c.text_muted).a11y_hidden())
                .child(text(title).font(type_::BODY_STRONG).color(k.c.text_base).truncate().grow(1.0))
                .child(btn(t::BTN_OPEN()).small().focusable().build(k, move |s, _| s.open_heard(&chosen))),
        );
    }
    // The dialog's title, its own edges, and the parts and switch above the scrolling rest.
    let above = if keeps { 196.0 } else { 148.0 };
    let rest = col().key("listen-rest").w_full().max_h((tall - above).max(200.0)).scroll(rune::ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(body).scroll_bar(k);
    Dialog::new("listen", t::CMD_LISTEN()).open(true).width(560.0).fit().on_close(close).child(col().w_full().child(head).child(rest)).build(k, layer);
}

/// `tall` is the window's height: a sheet longer than the window scrolls inside itself.
pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>, tall: f32) {
    match s.sheet {
        Sheet::None => {}
        Sheet::Settings => settings(s, k, layer, tall),
        Sheet::Screens => screens(s, k, layer),
        Sheet::Remote => remote(s, k, layer),
        Sheet::Import => import(s, k, layer),
        Sheet::Keys => keys(s, k, layer, tall),
        Sheet::Listen => listen(s, k, layer, tall),
        Sheet::Stream => stream(s, k, layer),
        Sheet::Bibles => crate::ui::bibles::view(s, k, layer),
        Sheet::Backdrops => crate::ui::backdrops::view(s, k, layer),
        Sheet::Languages => crate::ui::languages::dialog(s, k, layer, tall),
    }
}
