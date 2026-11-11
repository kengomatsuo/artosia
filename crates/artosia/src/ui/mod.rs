//! The operator window. Left to right is the order of work: the plan, the slides of the
//! open item, and what is live.

mod announcements;
mod bibles;
mod editor;
pub(crate) mod live;
mod palette;
pub(crate) mod plan;
mod plans;
mod sheets;
mod shell;
pub(crate) mod slides;
pub mod widgets;

use artosia_slides::audience::Media;
use rune::{Align, Cursor, Cx, El, Side, Transition, Tx, col, el, icon, row, spacer, text};
use rune_kit::Kit;

use crate::palette::Scope;
use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, size, space, type_};
use widgets::{btn, key_hint, keycap};

/// The slide the look and its previews are shown on.
pub fn sample_slide() -> artosia_core::deck::Slide {
    artosia_core::deck::Slide {
        kind: artosia_core::deck::SlideKind::Lyrics,
        label: String::new(),
        short: String::new(),
        tag: None,
        heading: String::new(),
        text: t::LOOK_SAMPLE().to_string(),
        second: String::new(),
        caption: String::new(),
        image: String::new(),
    }
}

/// The pictures the previews draw.
pub fn media(s: &App) -> Media<'_> {
    Media {
        picture: s.picture.as_ref().map(|(_, image)| image.pixels()),
        video: None,
        logo: s.logo.as_ref().map(|(_, image)| image.pixels()),
        slide: None,
    }
}

/// A screen state drawn small, with the slide's own picture when it has one.
pub fn thumb(s: &App, state: &artosia_slides::frame::OutputState, width: f32) -> El {
    let picture = state.slide.as_ref().filter(|slide| !slide.image.is_empty()).and_then(|slide| s.slide_pictures.get(&slide.image));
    let media = Media { slide: picture.map(|image| image.pixels()), ..media(s) };
    artosia_slides::audience::thumbnail(state, width, media, s.now)
}

pub(crate) fn topbar(s: &App, k: &Kit, insets: rune::ChromeInsets) -> El {
    let search = row()
        .key("search-button")
        .role("button")
        .label(t::SEARCH_PLACEHOLDER())
        .w(340.0)
        .h(34.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(k.radius.lg)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .cursor(Cursor::Text)
        .child(icon(&rune_kit::glyphs::SEARCH, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(text(t::SEARCH_PLACEHOLDER()).font(type_::BODY).color(k.c.text_muted).a11y_hidden())
        .child(spacer())
        .child(keycap(k, t::KEY_SEARCH(), false))
        .on_press(|s: &mut App| s.open_palette(Scope::All, ""));
    let clock = text(artosia_slides::stage::wall_clock(s.now)).key("clock").font(type_::CLOCK).color(k.c.text_base);
    row()
        .key("topbar")
        .w_full()
        .h(size::TOPBAR)
        // The band is the window's title bar: it drags the window, and leaves room for the
        // system's own buttons at either end.
        .window_drag()
        .pl(space::S4 + insets.leading)
        .pr(space::S4 + insets.trailing)
        .gap(space::S3)
        .center_y()
        .fill(k.c.bg_base)
        .child(
            row()
                .w(size::PLAN - space::S4 - space::S3)
                .gap(space::S2)
                .center_y()
                .child(icon(&icons::LOGO, 18.0).color(k.c.text_base).a11y_hidden())
                .child(text(t::APP_NAME()).font(type_::TITLE).color(k.c.text_base)),
        )
        .child(search)
        .child(spacer())
        .child(btn(t::BTN_LOOK()).icon(&icons::LOOK).ghost().build(k, |s, _| s.sheet = Sheet::Looks))
        .child(btn(t::BTN_SCREENS()).icon(&icons::SCREEN).ghost().build(k, |s, _| s.sheet = Sheet::Screens))
        .child(btn(t::BTN_PHONE()).icon(&icons::PHONE).ghost().build(k, |s, _| s.sheet = Sheet::Remote))
        .child(btn(t::BTN_STREAM()).icon(&icons::STREAM).ghost().active(s.stream.is_some()).build(k, |s, _| s.sheet = Sheet::Stream))
        .child(btn(t::BTN_LISTEN()).icon(&icons::MIC).ghost().build(k, |s, _| s.open_listen()))
        .child(el().w(space::S2))
        .child(clock)
}

pub(crate) fn statusbar(s: &App, k: &Kit) -> El {
    let status = s.link.status();
    let (dot, words): (rune::Color, String) = if let Some(problem) = &s.store_problem {
        (theme::warn_ink(), t::library_in_memory(problem))
    } else if !s.link.is_started() {
        (k.c.text_faint, if s.folder.is_none() { t::OUTPUT_PRACTICE() } else { t::OUTPUT_OFF() }.to_string())
    } else if s.settings.projector_off {
        (k.c.text_faint, t::OUTPUT_OFF().to_string())
    } else if let Some(problem) = &status.problem {
        (theme::warn_ink(), problem.clone())
    } else if !status.connected {
        (theme::warn_ink(), t::OUTPUT_STARTING().to_string())
    } else {
        match status.audience {
            Some(screen) => (rune::oklch(0.62, 0.17, 150.0), t::output_on_screen(screen + 1)),
            None if s.settings.projector_here => (rune::oklch(0.62, 0.17, 150.0), t::OUTPUT_HERE().to_string()),
            None => (k.c.text_faint, t::OUTPUT_NO_SCREEN().to_string()),
        }
    };
    let mut bar = row()
        .key("statusbar")
        .w_full()
        .h(size::STATUS)
        .px(space::S4)
        .gap(space::S4)
        .center_y()
        .fill(k.c.bg_base)
        .child(
            row()
                .key("output-status")
                .role("button")
                .label(t::BTN_SCREENS())
                .h(24.0)
                .px(space::S1)
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .on_press(|s: &mut App| s.sheet = Sheet::Screens)
                .gap(space::S2)
                .center_y()
                .child(el().size(8.0, 8.0).radius(4.0).fill(dot).a11y_hidden())
                .child(text(words).font(type_::SMALL).color(k.c.text_muted).truncate()),
        );
    if let Some(error) = &status.media_error {
        bar = bar.child(text(error.clone()).font(type_::SMALL).color(theme::warn_ink()).truncate());
    }
    if s.remote.is_some() {
        bar = bar.child(text(t::REMOTE_ON()).font(type_::SMALL).color(k.c.text_muted));
    }
    bar.child(spacer())
        .child(key_hint(k, t::KEY_DOWN(), t::HINT_NEXT()))
        .child(key_hint(k, t::KEY_BACK(), t::HINT_BACK()))
        .child(key_hint(k, "Shift", t::HINT_SELECT()))
        .child(key_hint(k, t::KEY_ENTER(), t::BTN_SHOW()))
        .child(
            row()
                .key("keys-button")
                .role("button")
                .label(t::CMD_KEYS())
                .h(24.0)
                .px(space::S1)
                .center_y()
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .cursor(Cursor::Pointer)
                .child(key_hint(k, t::KEY_KEYS(), t::HINT_KEYS()))
                .on_press(|s: &mut App| s.sheet = Sheet::Keys),
        )
}

pub(crate) fn toast(s: &App, k: &Kit) -> Option<El> {
    let toast = s.toast.as_ref()?;
    let mut body = row()
        .key("toast")
        .h(44.0)
        .px(space::S4)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.lg)
        .fill(k.c.layer_04)
        .border(1.0, k.c.border_strong)
        .shadow(8.0, 24.0, rune::Color::BLACK.alpha(0.4))
        .live_region(rune::LiveRegion::Polite)
        .transition(Transition::opacity())
        .child(text(toast.text.clone()).font(type_::BODY).color(k.c.text_base).truncate());
    if toast.undo.is_some() {
        body = body.child(btn(t::BTN_UNDO()).small().build(k, |s, _| s.undo()));
    }
    Some(body.anchored("statusbar", Side::Top, Align::Center, space::S3))
}

pub fn view(s: &App, cx: &mut Cx<App>) -> El {
    theme::set_dark(match s.settings.appearance {
        crate::state::Appearance::Auto => cx.system_dark(),
        crate::state::Appearance::Light => false,
        crate::state::Appearance::Dark => true,
    });
    t::set_language(&s.settings.language);
    let k = theme::kit();
    cx.caption_color(k.c.text_base);
    let insets = cx.chrome_insets();
    let mut layer: Vec<El> = Vec::new();
    let sf = theme::surf();
    let hline = || el().h(1.0).w_full().fill(sf.edge).a11y_hidden();
    let body = col()
        .key("window")
        .w_full()
        .h_full()
        .fill(sf.chrome)
        .child(topbar(s, k, insets).fill(sf.chrome))
        .child(hline())
        .child(shell::view(s, cx, k, &mut layer))
        .child(hline())
        .child(statusbar(s, k).fill(sf.chrome))
        .on_paste(|s: &mut App, text: &str, _: &mut Tx| s.song_from_clipboard(text))
        .on_drop_files(|s: &mut App, paths: &[std::path::PathBuf], _: &mut Tx| {
            if let Some(path) = paths.first() {
                s.set_background_file(path);
            }
        });
    if s.palette.open {
        palette::view(s, k, &mut layer);
    }
    sheets::view(s, k, &mut layer);
    plans::view(s, k, &mut layer);
    layer.extend(toast(s, k));
    rune_kit::root(body, layer)
}
