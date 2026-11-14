//! The Templates tab (OVTEMPL1, `mock_b.rs` page 6): every kind of announcement in its three styles,
//! drawn with this church's own data, and the two things a tile does: add a card of its kind, or
//! apply its style to a slide, to every announcement or to the open plan.

use artosia_core::deck::{Announcement, NoticeStyle, Template};
use artosia_core::window::{self, Range};

use crate::state::{App, Tab};
use crate::strings as t;

/// The templates the gallery draws; a Bible passage is a scripture slide, not a notice.
pub const KINDS: [Template; 5] = [Template::Schedule, Template::Birthdays, Template::Offerings, Template::Transfer, Template::Plain];

/// The tiles shown while no choice is made on the rail, as the mock draws them.
const CURATED: [(Template, NoticeStyle); 6] = [
    (Template::Schedule, NoticeStyle::Hairline),
    (Template::Birthdays, NoticeStyle::Ledger),
    (Template::Offerings, NoticeStyle::Band),
    (Template::Transfer, NoticeStyle::Hairline),
    (Template::Plain, NoticeStyle::Ledger),
    (Template::Schedule, NoticeStyle::Band),
];

/// What a press on "Apply style to" changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Target {
    #[default]
    Slide,
    Every,
    Plan,
}

/// The rail's choices and the tile whose style Apply carries.
pub struct Window {
    pub kind: Option<Template>,
    pub style: Option<NoticeStyle>,
    pub picked: (Template, NoticeStyle),
    /// The card that was open in the Announcements tab last; 0 when none was.
    pub card: i64,
    /// The colourway each poster style's tile is drawn in; none is the style's first.
    pub colourways: std::collections::HashMap<NoticeStyle, String>,
}

impl Default for Window {
    fn default() -> Window {
        Window { kind: None, style: None, picked: CURATED[0], card: 0, colourways: Default::default() }
    }
}

impl Window {
    /// The colourway a style's tile is drawn in; empty is the style's first.
    pub fn colourway_of(&self, style: NoticeStyle) -> String {
        self.colourways.get(&style).cloned().unwrap_or_default()
    }
}

/// The tiles for the rail's choices: the curated six for none, a kind in every style, a style
/// for every kind, or the one tile both name.
pub fn tiles(kind: Option<Template>, style: Option<NoticeStyle>) -> Vec<(Template, NoticeStyle)> {
    match (kind, style) {
        (None, None) => CURATED.to_vec(),
        (Some(kind), None) => NoticeStyle::ALL.iter().map(|style| (kind, *style)).collect(),
        (None, Some(style)) => KINDS.iter().map(|kind| (*kind, style)).collect(),
        (Some(kind), Some(style)) => vec![(kind, style)],
    }
}

/// The name of a template, as its row and its tile print it.
pub fn name(template: Template) -> &'static str {
    match template {
        Template::Schedule => t::TPL_SCHEDULE(),
        Template::Birthdays => t::TPL_BIRTHDAYS(),
        Template::Offerings => t::TPL_OFFERINGS(),
        Template::Transfer => t::TPL_TRANSFER(),
        Template::Passage => t::TPL_PASSAGE(),
        Template::Plain => t::TPL_PLAIN(),
    }
}

impl App {
    pub fn templates_filter(&mut self, kind: Option<Template>, style: Option<NoticeStyle>) {
        self.templates.kind = kind;
        self.templates.style = style;
    }

    /// Adds a card of `template` in `style` after the others and opens it in the Announcements tab.
    pub fn use_template(&mut self, template: Template, style: NoticeStyle) {
        self.save_notice();
        let mut card = self.tile_card(template, style);
        card.seconds = if template == Template::Plain { 10 } else { 12 };
        if self.store.save_announcement(&mut card).is_err() {
            return;
        }
        self.reload_announcements();
        self.open_tab(Tab::Notices);
        self.pick_notice(card.id);
    }

    /// Applies the picked tile's style: to the card open last, to every card, or to the plan's look.
    pub fn apply_style(&mut self, target: Target) {
        let style = self.templates.picked.1;
        let colourway = self.templates.colourway_of(style);
        if target == Target::Plan {
            self.use_own_look();
            let mut look = self.look();
            look.notice_style = style;
            self.set_look(look);
            self.refresh_decks();
            self.say(t::TPL_APPLIED_PLAN(), None);
            return;
        }
        let wanted = self.templates.card;
        let ids: Vec<i64> = self.announcements.iter().filter(|card| target == Target::Every || card.id == wanted).map(|card| card.id).collect();
        let mut changed = 0;
        for id in ids {
            if let Some(mut card) = self.announcements.iter().find(|card| card.id == id).cloned() {
                card.style = Some(style);
                card.colourway = colourway.clone();
                if self.store.save_announcement(&mut card).is_ok() {
                    changed += 1;
                }
            }
        }
        self.reload_announcements();
        self.say(t::style_applied(changed), None);
    }

    /// The card a tile draws and a press on it adds: the template's name, its source and the style.
    pub fn tile_card(&self, template: Template, style: NoticeStyle) -> Announcement {
        let (title, body) = match template {
            // Typed words need a first line to replace.
            Template::Plain => t::ANNOUNCEMENT_TEMPLATES()[0],
            other => (name(other), ""),
        };
        // The gallery shows a month of birthdays: the list a church keeps is longer than a week's.
        let window = if template == Template::Birthdays { window::Window { range: Range::Month(0), highlight: None } } else { window::Window::default() };
        Announcement { title: title.into(), body: body.into(), fill: template.fill(), style: Some(style), colourway: self.templates.colourway_of(style), window, ..Announcement::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rail_narrows_the_tiles_by_kind_and_by_style() {
        assert_eq!(tiles(None, None).len(), 6);
        assert_eq!(tiles(Some(Template::Schedule), None).iter().map(|tile| tile.1).collect::<Vec<_>>(), NoticeStyle::ALL);
        assert_eq!(tiles(None, Some(NoticeStyle::Band)).len(), KINDS.len());
        assert_eq!(tiles(Some(Template::Plain), Some(NoticeStyle::Ledger)), [(Template::Plain, NoticeStyle::Ledger)]);
    }

    #[test]
    fn a_tile_adds_a_card_in_its_style_and_opens_it() {
        let mut app = App::practice();
        let before = app.announcements.len();
        app.use_template(Template::Schedule, NoticeStyle::Band);
        assert_eq!(app.announcements.len(), before + 1);
        let card = app.announcements.last().cloned().unwrap_or_default();
        assert_eq!((card.fill, card.style), (artosia_core::deck::Fill::Schedule, Some(NoticeStyle::Band)));
        assert_eq!((app.tab, app.notices.as_ref().map(|form| form.id)), (Tab::Notices, Some(card.id)));
        // It reads the calendar, so the practice week gives it a slide in its style.
        assert!(app.card_slides(&card).first().is_some_and(|slide| slide.style == Some(NoticeStyle::Band)));
    }

    #[test]
    fn apply_gives_the_picked_style_to_one_card_or_to_all() {
        let mut app = App::practice();
        app.templates.picked = (Template::Plain, NoticeStyle::Ledger);
        app.open_tab(Tab::Notices);
        let first = app.announcements.first().map(|card| card.id).unwrap_or_default();
        app.open_tab(Tab::Templates);
        app.apply_style(Target::Slide);
        assert_eq!(app.announcements.iter().filter(|card| card.style == Some(NoticeStyle::Ledger)).map(|card| card.id).collect::<Vec<_>>(), [first]);
        app.apply_style(Target::Every);
        assert!(app.announcements.iter().all(|card| card.style == Some(NoticeStyle::Ledger)));
        app.templates.picked.1 = NoticeStyle::Band;
        app.apply_style(Target::Plan);
        assert_eq!(app.look().notice_style, NoticeStyle::Band);
    }

    #[test]
    fn a_colourway_goes_with_use_template_and_apply_and_draws_in_the_tile() {
        let mut app = App::practice();
        let ids: Vec<&str> = artosia_slides::warta::colourways(NoticeStyle::Neon).iter().map(|swatch| swatch.id).collect();
        app.templates.colourways.insert(NoticeStyle::Neon, ids[2].to_string());
        assert_eq!(app.tile_card(Template::Schedule, NoticeStyle::Neon).colourway, ids[2]);
        assert_eq!(app.tile_card(Template::Schedule, NoticeStyle::Pill).colourway, "");
        assert!(app.card_slides(&app.tile_card(Template::Schedule, NoticeStyle::Neon)).iter().all(|slide| slide.colourway == ids[2]));
        app.use_template(Template::Schedule, NoticeStyle::Neon);
        assert_eq!(app.announcements.last().map(|card| card.colourway.as_str()), Some(ids[2]));
        app.templates.picked.1 = NoticeStyle::Neon;
        app.apply_style(Target::Every);
        assert!(app.announcements.iter().all(|card| card.colourway == ids[2]));
    }

    #[test]
    fn the_gallery_builds_its_six_slides_well_inside_a_frame() {
        let app = App::practice();
        let started = std::time::Instant::now();
        for _ in 0..20 {
            for (template, style) in tiles(None, None) {
                let card = app.tile_card(template, style);
                assert!(!app.card_slides(&card).is_empty(), "{template:?} has practice data");
            }
        }
        let each = started.elapsed().as_secs_f64() * 1000.0 / 20.0;
        println!("six tiles: {each:.3} ms per frame");
        assert!(each < 8.0, "{each} ms");
    }
}
