//! The Templates window (OVTEMPL1, `mock_b.rs` page 6): a rail of kinds and styles, live tiles
//! drawn with this church's own data, Use template, and "Apply style to". The names, icons and
//! "reads" lines here are shared with the template rows of the Announcements tab.

use artosia_core::deck::{Announcement, Fill, NoticeStyle, Template, source_count};
use rune::{Cursor, El, ScrollSpec, col, el, icon, row, spacer, text};
use rune_kit::{Kit, ScrollBar, SegmentedControl};

use super::widgets::{btn, enter_key, section_label};
use crate::state::App;
use crate::strings as t;
use crate::templates::{KINDS, Target, name, tiles};
use crate::theme::{self, icons, space, type_};

/// A tile's width, as the mock's.
const TILE: f32 = 330.0;
/// The rail's width.
const RAIL: f32 = 220.0;

pub fn glyph(template: Template) -> &'static rune::Icon {
    match template {
        Template::Schedule => &icons::CALENDAR,
        Template::Birthdays => &icons::BADGE,
        Template::Offerings => &icons::COIN,
        Template::Transfer => &icons::SEND,
        Template::Passage => &icons::BOOK,
        Template::Plain => &icons::ANNOUNCE,
    }
}

/// Which window a template reads, in a few words.
pub fn reads(template: Template) -> &'static str {
    match template {
        Template::Schedule => t::TPL_READS_CALENDAR(),
        Template::Birthdays => t::TPL_READS_MEMBERS(),
        Template::Offerings | Template::Transfer => t::TPL_READS_OFFERINGS(),
        Template::Passage => t::TPL_READS_BIBLE(),
        Template::Plain => t::TPL_READS_NOTHING(),
    }
}

/// What a data source gives, as the tip on a row or a tile; typed words give none.
pub fn fill_words(fill: Fill) -> Option<&'static str> {
    match fill {
        Fill::Nothing => None,
        Fill::BirthdaysWeek => Some(t::FILL_WEEK()),
        Fill::BirthdaysMonth => Some(t::FILL_MONTH()),
        Fill::Passage => Some(t::FILL_PASSAGE()),
        Fill::ScheduleWeek => Some(t::FILL_SCHEDULE()),
        Fill::Offerings => Some(t::FILL_OFFERINGS()),
        Fill::Transfer => Some(t::FILL_TRANSFER()),
    }
}

pub fn style_name(style: NoticeStyle) -> &'static str {
    match style {
        NoticeStyle::Hairline => t::STYLE_HAIRLINE(),
        NoticeStyle::Ledger => t::STYLE_LEDGER(),
        NoticeStyle::Band => t::STYLE_BAND(),
    }
}

/// What the card reads and how much it found, as one line; typed words read nothing.
pub fn read_line(s: &App, card: &Announcement) -> String {
    let count = source_count(card, &s.people, &s.events, &s.giving_data(), &s.today(), s.bible().map(|bible| &**bible)).unwrap_or(0);
    match card.fill.template() {
        Template::Schedule => t::read_calendar(count),
        Template::Birthdays => t::read_members(count),
        Template::Offerings => t::read_gifts(count),
        Template::Transfer => t::NOTICE_READ_ACCOUNT().to_string(),
        Template::Passage => t::NOTICE_READ_BIBLE().to_string(),
        Template::Plain => t::TPL_READS_NOTHING().to_string(),
    }
}

/// Why a card has no slide today, said in the slide's place; None while it has one.
pub fn empty_note(s: &App, card: &Announcement, slides: usize) -> Option<&'static str> {
    if slides > 0 {
        return None;
    }
    Some(match card.fill.template() {
        Template::Schedule => t::NOTICE_EMPTY_CALENDAR(),
        Template::Birthdays => t::NOTICE_EMPTY_MEMBERS(),
        Template::Offerings => t::GIVING_EMPTY(),
        Template::Transfer => t::GIVING_ACCOUNT_NONE(),
        Template::Passage if s.bible().is_none() => t::PASSAGE_NO_BIBLE(),
        Template::Passage => t::PASSAGE_UNREADABLE(),
        Template::Plain => t::ANNOUNCEMENT_BODY_HINT(),
    })
}

/// A slide in 16 by 9 for `card`: its first slide drawn small, or the empty note on a plain field.
pub fn slide_or_note(s: &App, k: &Kit, card: &Announcement, width: f32) -> El {
    let slides = s.card_slides(card);
    let height = (width * 9.0 / 16.0).round();
    match empty_note(s, card, slides.len()) {
        Some(note) => el()
            .size(width, height)
            .radius(k.radius.lg)
            .fill(k.c.layer_02)
            .border(1.0, k.c.border_base)
            .px(space::S4)
            .center()
            .child(text(note).font(type_::BODY).color(k.c.text_muted).wrap().w(width - 2.0 * space::S4)),
        None => el().size(width, height).radius(k.radius.lg).clip_rounded(k.radius.lg).border(1.0, k.c.border_base).child(super::thumb(s, &s.preview_state(slides.first().cloned()), width)),
    }
}

fn rail_row(k: &Kit, key: String, words: &str, glyph: Option<&'static rune::Icon>, on: bool, press: impl Fn(&mut App) + 'static) -> El {
    let mut line = row()
        .key(key)
        .role("button")
        .label(words.to_string())
        .checked(on)
        .w_full()
        .h(if glyph.is_some() { 36.0 } else { 30.0 })
        .px(space::S3)
        .gap(space::S3)
        .center_y()
        .radius(k.radius.md)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key)
        .focus_selected(on);
    line = if on { line.fill(theme::accent().alpha(0.18)) } else { line.hover(k.c.hover) };
    if let Some(glyph) = glyph {
        line = line.child(icon(glyph, 16.0).color(k.c.text_muted).a11y_hidden());
    }
    line.child(text(words.to_string()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(if glyph.is_some() || on { k.c.text_base } else { k.c.text_muted }).truncate().min_w(0.0).a11y_hidden()).on_press(press)
}

fn rail(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let window = &s.templates;
    let mut kinds = col().key("templates-kinds").role("toolbar").label(t::TPL_HEADING()).w_full().gap(2.0).focus_group().focus_follows();
    kinds = kinds.child(rail_row(k, "tpl-kind-all".into(), t::TPL_ALL(), Some(&icons::FOLDER), window.kind.is_none(), |s| s.templates_filter(None, s.templates.style)));
    for template in KINDS {
        kinds = kinds.child(rail_row(k, format!("tpl-kind-{template:?}"), name(template), Some(glyph(template)), window.kind == Some(template), move |s| s.templates_filter(Some(template), s.templates.style)));
    }
    let mut styles = col().key("templates-styles").role("toolbar").label(t::TPL_STYLE()).w_full().gap(2.0).focus_group().focus_follows();
    styles = styles.child(rail_row(k, "tpl-style-all".into(), t::STYLE_ALL(), None, window.style.is_none(), |s| s.templates_filter(s.templates.kind, None)));
    for style in NoticeStyle::ALL {
        styles = styles.child(rail_row(k, format!("tpl-style-{}", style.code()), style_name(style), None, window.style == Some(style), move |s| s.templates_filter(s.templates.kind, Some(style))));
    }
    col().key("templates-rail").w(RAIL).h_full().fill(sf.side).padding(space::S3).gap(2.0).child(row().w_full().child(spacer()).child(crate::ui::tour::help_mark(s, k, crate::guide::lessons::Feature::Templates))).child(kinds).child(el().h(space::S4)).child(section_label(k, t::TPL_STYLE())).child(el().h(2.0)).child(styles)
}

fn tile(s: &App, k: &Kit, template: Template, style: NoticeStyle) -> El {
    let on = s.templates.picked == (template, style);
    let card = s.tile_card(template, style);
    let mut frame = el().w(TILE).h((TILE * 9.0 / 16.0).round()).radius(k.radius.lg).child(slide_or_note(s, k, &card, TILE));
    if on {
        frame = frame
            .child(el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(k.radius.lg).border(2.5, theme::accent()).a11y_hidden())
            .child(el().pin(None, Some(10.0), Some(10.0), None).child(btn(t::TPL_USE()).icon(&icons::PLUS).small().focusable().build(k, move |s, _| s.use_template(template, style)).key("tpl-use")));
    }
    // Style and source apart: the interface never joins words with a middle dot.
    let note = format!("{}, {}", style_name(style), reads(template));
    let tip = fill_words(card.fill).unwrap_or_default();
    col()
        .key(format!("tile-{template:?}-{}", style.code()))
        .help(tip)
        .role("button")
        .label(format!("{} {note}", name(template)))
        .checked(on)
        .w(TILE)
        .gap(space::S2)
        .cursor(Cursor::Pointer)
        .focusable(true)
        .on_key(enter_key)
        .child(frame)
        .child(
            row()
                .gap(space::S2)
                .center_y()
                .child(row().size(28.0, 28.0).center().radius(14.0).fill(theme::accent().alpha(0.2)).child(icon(glyph(template), 14.0).color(theme::accent()).a11y_hidden()))
                .child(
                    col()
                        .min_w(0.0)
                        .child(row().gap(space::S2).center_y().child(text(name(template)).font(type_::BODY_STRONG).color(k.c.text_base).truncate().min_w(0.0)).child(text(style_name(style)).font(type_::TINY).color(theme::accent())))
                        .child(text(reads(template)).font(type_::SMALL).color(k.c.text_muted).truncate()),
                ),
        )
        .on_press(move |s: &mut App| s.templates.picked = (template, style))
}

pub fn view(s: &App, k: &Kit) -> El {
    let sf = theme::surf();
    let mut grid = row().key("templates-grid").w_full().gap(space::S5).flex_wrap();
    for (template, style) in tiles(s.templates.kind, s.templates.style) {
        grid = grid.child(tile(s, k, template, style));
    }
    let apply = row()
        .w_full()
        .gap(space::S3)
        .center_y()
        .child(text(t::TPL_APPLY()).font(type_::BODY).color(k.c.text_muted))
        .child(
            SegmentedControl::new(&[t::TPL_APPLY_SLIDE(), t::TPL_APPLY_ALL(), t::TPL_APPLY_PLAN()], None)
                .label(t::TPL_APPLY())
                .fit()
                .on_pick(|s: &mut App, picked, _: &mut rune::Tx| s.apply_style([Target::Slide, Target::Every, Target::Plan][picked.min(2)]))
                .build(k),
        );
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .padding(space::S5)
        .gap(space::S4)
        .child(row().w_full().gap(space::S3).center_y().child(text(t::TPL_GALLERY()).font(type_::HEADING).color(k.c.text_base)).child(text(t::TPL_GALLERY_NOTE()).font(type_::BODY).color(k.c.text_muted)).child(spacer()))
        .child(col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(grid).scroll_bar(k))
        .child(apply);
    row().key("templates").w_full().grow(1.0).min_h(0.0).fill(sf.work).child(rail(s, k)).child(el().w(1.0).h_full().fill(sf.edge).a11y_hidden()).child(main)
}
