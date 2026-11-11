//! Ready backdrops: category chips, a grid of equal previews, a search field, and for the chosen
//! one a large preview with its author and licence and one button that puts it behind the words,
//! fetching it first when it is not on this computer. `panel` is the whole browser; `view` wraps it
//! in a dialog for now (the Look dialog's Background panel is to hold `panel`).

use std::path::Path;

use artosia_core::backdrops::{self, Entry, Kind};
use rune::{Cursor, El, GrowthSpec, Icon, ScrollSpec, SurfaceFit, col, el, icon, row, spacer, text};
use rune_kit::{Dialog, Kit, Progress, ScrollBar, TextInput};

use crate::backdrops::COLUMNS;
use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::btn;

/// The width the panel is built for; a place that holds it gives it this much.
pub(crate) const WIDTH: f32 = 640.0;
const GAP: f32 = space::S2;
/// A tile is a 16 to 9 preview.
const TILE_W: f32 = (WIDTH - GAP * (COLUMNS as f32 - 1.0)) / COLUMNS as f32;
const TILE_H: f32 = (TILE_W * 9.0 / 16.0).floor();
const ROW: f32 = TILE_H + GAP;
/// Three rows of tiles show at once.
const GRID: f32 = ROW * 3.0 - GAP;
const LARGE_W: f32 = 304.0;
const LARGE_H: f32 = LARGE_W * 9.0 / 16.0;

macro_rules! mark {
    ($name:ident, $id:literal, $body:literal) => {
        static $name: Icon = Icon { name: $id, view: 16.0, body: $body };
    };
}

mark!(CHECK, "b-check", r#"<path d="M3.5 8.5L6.5 11.5L12.5 4.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>"#);
mark!(SKY, "b-sky", r#"<path d="M4.5 12.5C2.8 12.5 1.5 11.2 1.5 9.6C1.5 8.1 2.6 6.9 4.1 6.7C4.6 4.9 6.1 3.5 8.1 3.5C10.3 3.5 12 5 12.3 7C13.7 7.2 14.5 8.3 14.5 9.6C14.5 11.1 13.4 12.5 11.8 12.5H4.5Z" stroke="currentColor" stroke-linejoin="round"/>"#);
mark!(MOUNTAINS, "b-mountains", r#"<path d="M1.5 13L6 5L9 10L10.5 8L14.5 13H1.5Z" stroke="currentColor" stroke-linejoin="round"/>"#);
mark!(WATER, "b-water", r#"<path d="M1.5 6C3 4.5 4.5 4.5 6 6C7.5 7.5 9 7.5 10.5 6C12 4.5 13.5 4.5 14.5 5.5M1.5 11C3 9.5 4.5 9.5 6 11C7.5 12.5 9 12.5 10.5 11C12 9.5 13.5 9.5 14.5 10.5" stroke="currentColor" stroke-linecap="round"/>"#);
mark!(NATURE, "b-nature", r#"<path d="M8 14V8M8 8C5 8 3.5 6 3.5 3C6.5 3 8 5 8 8ZM8 10.5C10.5 10.5 12.5 9 12.5 6C10 6 8 7.5 8 10.5Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
mark!(LIGHT, "b-light", r#"<circle cx="8" cy="8" r="2.8" stroke="currentColor"/><path d="M8 1.5V3.2M8 12.8V14.5M1.5 8H3.2M12.8 8H14.5M3.4 3.4L4.6 4.6M11.4 11.4L12.6 12.6M3.4 12.6L4.6 11.4M11.4 4.6L12.6 3.4" stroke="currentColor" stroke-linecap="round"/>"#);
mark!(NIGHT, "b-night", r#"<path d="M12.5 10.2A5.2 5.2 0 0 1 5.8 3.5A5.2 5.2 0 1 0 12.5 10.2Z" stroke="currentColor" stroke-linejoin="round"/>"#);
mark!(CANDLES, "b-candles", r#"<path d="M8 1.8C9.4 3.4 9.9 4.5 9.9 5.5C9.9 6.6 9.1 7.3 8 7.3C6.9 7.3 6.1 6.6 6.1 5.5C6.1 4.5 6.6 3.4 8 1.8Z" stroke="currentColor" stroke-linejoin="round"/><rect x="5.5" y="8.5" width="5" height="5.5" rx="0.6" stroke="currentColor"/>"#);
mark!(CHURCH, "b-church", r#"<path d="M8 1.8V14.2M3.8 5.6H12.2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>"#);
mark!(PEOPLE, "b-people", r#"<circle cx="8" cy="4.2" r="1.7" stroke="currentColor"/><path d="M3 2.5L5.2 7.2M13 2.5L10.8 7.2M5.2 7.2H10.8L10 14H6L5.2 7.2Z" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round"/>"#);
mark!(TEXTURE, "b-texture", r#"<rect x="2" y="2" width="12" height="12" rx="1.5" stroke="currentColor"/><path d="M2 8H14M8 2V14" stroke="currentColor" stroke-dasharray="1.6 1.6"/>"#);
mark!(ALL, "b-all", r#"<rect x="2" y="2" width="5" height="5" rx="1" stroke="currentColor"/><rect x="9" y="2" width="5" height="5" rx="1" stroke="currentColor"/><rect x="2" y="9" width="5" height="5" rx="1" stroke="currentColor"/><rect x="9" y="9" width="5" height="5" rx="1" stroke="currentColor"/>"#);
mark!(SUN, "b-sun", r#"<circle cx="8" cy="8" r="3" fill="currentColor"/><path d="M8 1.8V3.4M8 12.6V14.2M1.8 8H3.4M12.6 8H14.2" stroke="currentColor" stroke-linecap="round"/>"#);

fn category_icon(category: &str) -> &'static Icon {
    match category {
        "sky" => &SKY,
        "mountains" => &MOUNTAINS,
        "water" => &WATER,
        "nature" => &NATURE,
        "light" => &LIGHT,
        "night" => &NIGHT,
        "candles" => &CANDLES,
        "church" => &CHURCH,
        "people" => &PEOPLE,
        _ => &TEXTURE,
    }
}

fn category_name(category: &str) -> &'static str {
    match category {
        "sky" => t::BACKDROPS_SKY(),
        "mountains" => t::BACKDROPS_MOUNTAINS(),
        "water" => t::BACKDROPS_WATER(),
        "nature" => t::BACKDROPS_NATURE(),
        "light" => t::BACKDROPS_LIGHT(),
        "night" => t::BACKDROPS_NIGHT(),
        "candles" => t::BACKDROPS_CANDLES(),
        "church" => t::BACKDROPS_CHURCH(),
        "people" => t::BACKDROPS_PEOPLE(),
        _ => t::BACKDROPS_TEXTURE(),
    }
}

fn chip(k: &Kit, key: &str, label: &str, mark: &'static Icon, on: bool, press: impl Fn(&mut App) + 'static) -> El {
    let ink = if on { k.c.text_base } else { k.c.text_muted };
    let mut chip = row()
        .key(format!("backdrop-chip-{key}"))
        .role("button")
        .label(label.to_string())
        .h(28.0)
        .px(space::S3)
        .gap(6.0)
        .center_y()
        .radius(k.radius.md)
        .fill(if on { k.c.layer_04 } else { k.c.layer_02 })
        .hover(k.c.hover)
        .focusable(true).on_key(crate::ui::widgets::enter_key)
        .cursor(Cursor::Pointer)
        .child(icon(mark, 14.0).color(ink).a11y_hidden())
        .child(text(label.to_string()).font(if on { type_::SMALL_STRONG } else { type_::SMALL }).color(ink).a11y_hidden());
    if !on {
        chip = chip.border(1.0, k.c.border_muted);
    }
    chip.on_press(move |s: &mut App| press(s))
}

fn chips(s: &App, k: &Kit) -> El {
    // One Tab stop; the arrows move among the subjects.
    let mut flow = row().key("backdrop-chips").role("toolbar").label(t::BACKDROPS_TITLE()).w_full().gap(space::S2).flex_wrap().focus_group();
    flow = flow.child(chip(k, "all", t::BACKDROPS_ALL(), &ALL, s.backdrops.category.is_none(), |s| s.choose_backdrop_category(None)));
    for name in backdrops::CATEGORIES {
        let on = s.backdrops.category.as_deref() == Some(name);
        flow = flow.child(chip(k, name, category_name(name), category_icon(name), on, move |s| s.choose_backdrop_category(Some(name))));
    }
    flow
}

/// The preview picture of an entry: its file when it is on this computer (pictures), else the small preview.
fn picture_of(s: &App, entry: &Entry, large: bool) -> Option<rune::Image> {
    let images = s.images.as_ref()?;
    let media = s.backdrop_media();
    if large && entry.kind == Kind::Picture && s.backdrops.have.contains(&entry.id) {
        return Some(images.file(backdrops::local_path(&media, entry), LARGE_W, 1.0));
    }
    if s.backdrops.previews.contains(&entry.id) {
        return Some(images.file(backdrops::preview_path(&media, entry), if large { LARGE_W } else { TILE_W }, 1.0));
    }
    None
}

fn badge(mark: &'static Icon, label: &str) -> El {
    el().size(20.0, 20.0).center().radius(10.0).fill(rune::Color::BLACK.alpha(0.55)).help(label.to_string()).child(icon(mark, 12.0).color(rune::Color::WHITE).a11y_hidden())
}

fn tile(s: &App, k: &Kit, at: usize) -> El {
    let Some(entry) = s.backdrops.shown.get(at).and_then(|index| s.backdrops.catalogue.entries.get(*index)) else {
        return el().w(TILE_W).h(TILE_H);
    };
    let id = entry.id.clone();
    let picked = s.backdrops.picked.as_deref() == Some(entry.id.as_str());
    let mut frame = el().key(format!("backdrop-{}", entry.id)).role("button").label(entry.title.clone()).w(TILE_W).h(TILE_H).radius(k.radius.md).clip_rounded(k.radius.md).fill(k.c.layer_04).focusable(true).on_key(crate::ui::widgets::enter_key).cursor(Cursor::Pointer);
    match picture_of(s, entry, false) {
        Some(image) => frame = frame.pixels_fit(image.pixels(), SurfaceFit::Fill),
        None if s.backdrops.failed.contains(&entry.id) => frame = frame.center().child(icon(&icons::IMAGE, 20.0).color(k.c.text_muted).a11y_hidden()),
        None => {}
    }
    if entry.kind == Kind::Video {
        frame = frame.child(el().absolute(6.0, 6.0).child(badge(&icons::PLAY, entry.title.as_str())));
    }
    if s.backdrops.have.contains(&entry.id) {
        frame = frame.child(el().absolute(TILE_W - 26.0, TILE_H - 26.0).child(badge(&CHECK, t::BACKDROPS_HERE())));
    }
    if picked {
        // Drawn over the picture, inside the tile, so the picture never covers it.
        frame = frame.child(el().absolute(0.0, 0.0).size(TILE_W, TILE_H).radius(k.radius.md).border(2.0, k.c.text_base).a11y_hidden());
    }
    frame.on_press(move |s: &mut App| s.pick_backdrop(&id))
}

fn grid(s: &App, k: &Kit) -> El {
    if s.backdrops.shown.is_empty() {
        return col().key("backdrops-empty").w_full().h(GRID).center().padding(space::S4).child(text(t::nothing_found(s.backdrops.query.text.trim())).font(type_::BODY).color(k.c.text_muted).wrap());
    }
    let rows = s.backdrops.shown.len().div_ceil(COLUMNS);
    let list = col()
        .key("backdrop-rows")
        .growth(GrowthSpec::default())
        .virtual_rows(
            rows,
            ROW,
            |_: &App, at| format!("backdrop-row-{at}").into(),
            |s: &App, at| {
                let mut line = row().w_full().h(TILE_H).gap(GAP);
                for column in 0..COLUMNS {
                    line = line.child(tile(s, theme::kit(), at * COLUMNS + column));
                }
                // The row is as tall as a tile and the gap under it, so rows never touch.
                col().w_full().h(ROW).child(line)
            },
        )
        .on_visible_rows(|s: &mut App, rows| {
            s.show_rows(rows);
            false
        });
    // The grid is one Tab stop too, with the arrows moving from picture to picture: three
    // hundred pictures cannot each be a stop on the way to the one button under them.
    col().key("backdrop-scroll").role("grid").focus_group().label(t::BACKDROPS_TITLE()).w_full().h(GRID).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(crate::theme::size::FADE).child(list).scroll_bar(k)
}

fn clock(seconds: f32) -> String {
    let whole = seconds.round() as u32;
    format!("{}:{:02}", whole / 60, whole % 60)
}

fn fact(k: &Kit, label: &str, value: &str) -> El {
    row().w_full().gap(space::S2).child(el().w(64.0).child(text(label.to_string()).font(type_::SMALL).color(k.c.text_muted))).child(text(value.to_string()).font(type_::SMALL).color(k.c.text_base).truncate_middle().grow(1.0).min_w(0.0))
}

fn detail(s: &App, k: &Kit) -> El {
    let card = row().key("backdrop-detail").w_full().h(LARGE_H + 2.0 * space::S3).padding(space::S3).gap(space::S3).radius(k.radius.lg).fill(k.c.layer_02);
    let Some(entry) = s.backdrops.picked_entry() else {
        return card.center().child(text(t::BACKDROPS_PICK_HINT()).font(type_::BODY).color(k.c.text_muted));
    };
    let mut large = el().key("backdrop-large").w(LARGE_W).h(LARGE_H).radius(k.radius.md).clip_rounded(k.radius.md).fill(k.c.layer_04);
    if let Some(image) = picture_of(s, entry, true) {
        large = large.pixels_fit(image.pixels(), SurfaceFit::Fill);
    }
    let id = entry.id.clone();
    let size = match (entry.kind, entry.duration) {
        (Kind::Video, Some(seconds)) => format!("{} x {}, {}", entry.width, entry.height, clock(seconds)),
        _ => format!("{} x {}", entry.width, entry.height),
    };
    let mut head = row().w_full().gap(space::S2).center_y().child(icon(if entry.kind == Kind::Video { &icons::PLAY } else { &icons::IMAGE }, 14.0).color(k.c.text_muted).a11y_hidden()).child(text(entry.title.clone()).font(type_::BODY_STRONG).color(k.c.text_base).max_lines(2).grow(1.0));
    if entry.wants_dimming() {
        head = head.child(el().help(t::BACKDROPS_DIM()).role("img").label(t::BACKDROPS_DIM()).child(icon(&SUN, 14.0).color(theme::warn_ink()).a11y_hidden()));
    }
    let action = if let Some(fetch) = s.backdrops.fetch.as_ref().filter(|fetch| fetch.id == entry.id) {
        // The bar is the state: it fills as the file arrives, and Cancel stands beside it.
        row()
            .w_full()
            .gap(space::S3)
            .center_y()
            .child(el().grow(1.0).min_w(0.0).child(Progress::new(fetch.fraction()).label(t::BACKDROPS_DOWNLOADING()).build(k)))
            .child(btn(t::BTN_CANCEL()).icon(&icons::CLOSE).focusable().build(k, |s, _| s.cancel_backdrop()))
    } else {
        let busy = s.backdrops.fetch.is_some();
        let (label, mark) = (t::BACKDROPS_USE(), if s.backdrops.have.contains(&entry.id) { &icons::PLAY } else { &icons::IMPORT });
        row().w_full().child(spacer()).child(btn(label).icon(mark).primary().disabled(busy).focusable().build(k, move |s, _| s.use_backdrop(&id)))
    };
    card.child(large).child(
        col()
            .w_full()
            .h_full()
            .min_w(0.0)
            .gap(space::S2)
            .child(head)
            .child(fact(k, t::BACKDROPS_SIZE(), &size))
            .child(fact(k, t::BACKDROPS_AUTHOR(), &entry.author))
            .child(fact(k, t::BACKDROPS_LICENCE(), &entry.licence))
            .child(fact(k, t::BACKDROPS_PAGE(), host_of(&entry.page)))
            .child(spacer())
            .child(action)
            .grow(1.0),
    )
}

/// The address's host and path without the scheme: where the licence is stated.
fn host_of(url: &str) -> &str {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")).unwrap_or(url);
    Path::new(rest).to_str().unwrap_or(rest)
}

/// The whole browser. `layer` is for anything that must float over the window; nothing here does yet.
pub(crate) fn panel(s: &App, k: &Kit, _layer: &mut Vec<El>) -> El {
    let search = TextInput::new(&s.backdrops.query).width(None).label(t::BACKDROPS_SEARCH()).placeholder(t::BACKDROPS_SEARCH()).on_edit(|s: &mut App| &mut s.backdrops.query).build(k);
    col()
        .key("backdrops-panel")
        .w(WIDTH)
        .gap(space::S3)
        .child(search)
        .child(chips(s, k))
        .child(grid(s, k))
        .child(detail(s, k))
}

/// The mark of the browser itself, for the button in Design and the command in search.
pub(crate) fn mark() -> &'static Icon {
    &ALL
}

/// The browser in a dialog of its own, one step inside Design: the grid needs the room, and
/// closing it goes back to Design, where it was opened and where the sample is.
pub fn view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let body = col().w_full().px(space::S4).pb(space::S4).child(panel(s, k, layer));
    Dialog::new("backdrops", t::BACKDROPS_TITLE()).open(true).width(WIDTH + 2.0 * space::S4).fit().on_close(|s: &mut App| s.sheet = Sheet::Looks).child(body).build(k, layer);
}
