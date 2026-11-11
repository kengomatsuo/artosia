//! The tour: five stops through the main window, shown the first time Artosia starts and
//! whenever someone asks for it again. Each stop dims the window around one part of it, rings
//! that part in the accent colour and puts a small card beside it. It teaches the model (plan,
//! Preview, Show, Live, search), never a list of keys, and Skip is on every stop.
//!
//! While it is open it owns the keyboard (`App::tour_key`) and the card is a modal, so nothing
//! behind it can be pressed and nothing in it can reach the projector.

use rune::{Align, Cx, El, Key, Side, col, el, row, spacer, text};
use rune_kit::Kit;

use crate::state::{App, Sheet};
use crate::strings as t;
use crate::theme::{self, space, type_};
use crate::ui::widgets::btn;

/// The card's width.
const CARD: f32 = 320.0;
/// Room between a ring and the part it surrounds.
const PAD: f32 = 4.0;

struct Stop {
    /// The key of the element the ring goes round.
    target: &'static str,
    /// Which side of it the card sits on, and how it lines up.
    side: Side,
    align: Align,
    main: fn() -> &'static str,
    more: fn() -> &'static str,
}

const STOPS: [Stop; 5] = [
    Stop { target: "plan", side: Side::Right, align: Align::Start, main: t::TOUR_PLAN, more: t::TOUR_PLAN_MORE },
    // The card goes under the list, over the strip with the picture, which this stop is not about.
    Stop { target: "slide-list", side: Side::Bottom, align: Align::Start, main: t::TOUR_PREVIEW, more: t::TOUR_PREVIEW_MORE },
    Stop { target: "show-button", side: Side::Top, align: Align::Start, main: t::TOUR_SHOW, more: t::TOUR_SHOW_MORE },
    Stop { target: "transport", side: Side::Top, align: Align::End, main: t::TOUR_LIVE, more: t::TOUR_LIVE_MORE },
    Stop { target: "search-button", side: Side::Bottom, align: Align::Start, main: t::TOUR_SEARCH, more: t::TOUR_SEARCH_MORE },
];

/// How many stops there are.
pub const STOP_COUNT: usize = STOPS.len();

impl App {
    /// Opens the tour at its first stop, over whatever the window shows.
    pub fn start_tour(&mut self) {
        self.sheet = Sheet::None;
        self.close_palette();
        self.tour = Some(0);
        self.tour_focus = true;
    }

    /// Next, or Done on the last stop.
    pub fn tour_next(&mut self) {
        match self.tour {
            Some(at) if at + 1 < STOP_COUNT => self.tour = Some(at + 1),
            Some(_) => self.end_tour(),
            None => {}
        }
    }

    pub fn tour_back(&mut self) {
        if let Some(at) = self.tour {
            self.tour = Some(at.saturating_sub(1));
        }
    }

    /// Closes the tour from any stop, and remembers it was seen so it does not start by itself again.
    pub fn end_tour(&mut self) {
        if self.tour.take().is_some() {
            self.tour_focus = false;
            if !self.settings.toured {
                self.settings.toured = true;
                self.touch_settings();
            }
        }
    }

    /// A key while the tour is open. Enter or Right goes on, Left goes back, Escape skips; every
    /// other key is taken and does nothing, so no key can change what is on the projector.
    /// `focused`: a button has focus, and Rune presses it on Enter itself.
    pub fn tour_key(&mut self, key: &Key, focused: bool) {
        match key {
            Key::Right => self.tour_next(),
            Key::Enter if !focused => self.tour_next(),
            Key::Left => self.tour_back(),
            Key::Escape => self.end_tour(),
            _ => {}
        }
    }
}

/// The dim around the part a stop points at, the ring, and the card.
pub(crate) fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) {
    let Some(at) = s.tour else { return };
    let stop = &STOPS[at.min(STOP_COUNT - 1)];
    let dim = rune::Color::BLACK.alpha(0.38);
    let card = card(k, at, stop);
    match cx.layout(stop.target) {
        Some([x, y, w, h]) => {
            let (x0, y0) = ((x - PAD).max(0.0), (y - PAD).max(0.0));
            let (x1, y1) = (x + w + PAD, y + h + PAD);
            let strip = || el().fill(dim).a11y_hidden();
            // Four strips leave the part itself undimmed.
            layer.push(strip().key("tour-dim-top").pin(Some(0.0), Some(0.0), Some(0.0), None).h(y0));
            layer.push(strip().key("tour-dim-bottom").pin(Some(0.0), Some(y1), Some(0.0), Some(0.0)));
            layer.push(strip().key("tour-dim-left").pin(Some(0.0), Some(y0), None, None).w(x0).h(y1 - y0));
            layer.push(strip().key("tour-dim-right").pin(Some(x1), Some(y0), Some(0.0), None).h(y1 - y0));
            layer.push(el().key("tour-ring").absolute(x0, y0).size(x1 - x0, y1 - y0).radius(k.radius.lg).border(3.0, theme::accent()).a11y_hidden());
            layer.push(card.anchored(stop.target, stop.side, stop.align, space::S3));
        }
        // The part is not on screen (an editor has the Preview pane): dim all of it and centre the card.
        None => {
            layer.push(el().key("tour-dim").absolute(0.0, 0.0).w_full().h_full().fill(dim).a11y_hidden());
            layer.push(col().key("tour-centre").absolute(0.0, 0.0).w_full().h_full().center().child(card));
        }
    }
}

fn card(k: &Kit, at: usize, stop: &Stop) -> El {
    let last = at + 1 == STOP_COUNT;
    let mut buttons = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(btn(t::TOUR_SKIP()).ghost().focusable().build(k, |s, _| s.end_tour()))
        .child(spacer());
    if at > 0 {
        buttons = buttons.child(btn(t::BTN_BACK()).focusable().build(k, |s, _| s.tour_back()));
    }
    buttons = buttons.child(btn(if last { t::TOUR_DONE() } else { t::BTN_NEXT() }).primary().focusable().build(k, |s, _| s.tour_next()).key("tour-next"));
    col()
        .key("tour-card")
        .role("dialog")
        .label(t::CMD_TOUR())
        .w(CARD)
        .padding(space::S4)
        .gap(space::S3)
        .radius(k.radius.lg)
        .fill(k.c.layer_04)
        .border(1.0, k.c.border_strong)
        .shadow(8.0, 24.0, rune::Color::BLACK.alpha(0.4))
        .child(
            col()
                .w_full()
                .gap(space::S1)
                .child(text(t::tour_place(at + 1, STOP_COUNT)).font(type_::SMALL).color(k.c.text_muted))
                .child(text((stop.main)()).font(type_::TITLE).color(k.c.text_base).wrap())
                .child(text((stop.more)()).font(type_::BODY).color(k.c.text_muted).wrap()),
        )
        .child(buttons)
        .modal(false, |s: &mut App| s.end_tour())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_forward_and_back_and_ends() {
        let mut app = App::practice();
        app.start_tour();
        assert_eq!(app.tour, Some(0));
        app.tour_back();
        assert_eq!(app.tour, Some(0), "Back on the first stop stays");
        for want in 1..STOP_COUNT {
            app.tour_next();
            assert_eq!(app.tour, Some(want));
        }
        app.tour_back();
        assert_eq!(app.tour, Some(STOP_COUNT - 2));
        app.tour_next();
        app.tour_next();
        assert_eq!(app.tour, None, "Next on the last stop is Done");
        assert!(app.settings.toured);
    }

    #[test]
    fn skip_saves_that_it_was_seen() {
        let mut app = App::practice();
        assert!(!app.settings.toured);
        app.start_tour();
        app.tour_next();
        app.end_tour();
        assert_eq!(app.tour, None);
        assert!(app.settings.toured);
        assert!(app.settings_dirty);
    }

    #[test]
    fn a_key_during_the_tour_never_changes_what_is_live() {
        let mut app = App::practice();
        app.open_song(0, None);
        app.show(1);
        let live = app.live.as_ref().map(|live| (live.deck.title.clone(), live.index));
        let keys = [Key::Down, Key::Up, Key::PageDown, Key::PageUp, Key::Home, Key::End, Key::Tab, Key::Delete, Key::Function(2), Key::Text(" ".into()), Key::Text("1".into()), Key::Command('z')];
        for focused in [false, true] {
            app.start_tour();
            for key in &keys {
                app.tour_key(key, focused);
                assert_eq!(app.live.as_ref().map(|live| (live.deck.title.clone(), live.index)), live);
                assert!(!app.black);
                assert!(app.tour.is_some(), "only Enter, Left, Right and Escape move the tour");
            }
        }
    }

    #[test]
    fn keys_move_the_tour() {
        let mut app = App::practice();
        app.start_tour();
        app.tour_key(&Key::Right, false);
        app.tour_key(&Key::Enter, false);
        assert_eq!(app.tour, Some(2));
        // A focused button presses itself on Enter; the window must not step twice.
        app.tour_key(&Key::Enter, true);
        assert_eq!(app.tour, Some(2));
        app.tour_key(&Key::Left, false);
        assert_eq!(app.tour, Some(1));
        app.tour_key(&Key::Escape, false);
        assert_eq!(app.tour, None);
    }
}
