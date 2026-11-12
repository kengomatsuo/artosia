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
use crate::theme::{self, icons, space, type_};
use crate::ui::widgets::btn;

/// The card's width.
/// The card is wide enough for three buttons of one size in every language.
const CARD: f32 = 432.0;
/// Every button of the card has this width, so a longer word never changes the card. It holds
/// an icon and the nine letters a button's brief allows ("Siguiente" was cut at 100).
const BUTTON: f32 = 124.0;
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

/// The five stops and their words, in one table so they can be rewritten without touching the drawing.
const STOPS: [Stop; 5] = [
    Stop { target: "plan", side: Side::Right, align: Align::Start, main: t::TOUR_PLAN, more: t::TOUR_PLAN_MORE },
    // The card goes under the list, over the strip with the picture, which this stop is not about.
    Stop { target: "slide-list", side: Side::Bottom, align: Align::Start, main: t::TOUR_PREVIEW, more: t::TOUR_PREVIEW_MORE },
    // A ring goes round a container the shell keys, never one button: the buttons are rearranged often.
    Stop { target: "preview-controls", side: Side::Top, align: Align::End, main: t::TOUR_SHOW, more: t::TOUR_SHOW_MORE },
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

    /// The Next button's own press. Dropped when the Enter that caused it already moved on.
    pub fn tour_press_next(&mut self) {
        if self.clock != self.tour_entered_at {
            self.tour_next();
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
    /// Enter always goes on: a headless run showed Enter doing nothing while the Next button
    /// held focus, so the tour does not wait for Rune to press it.
    pub fn tour_key(&mut self, key: &Key) {
        match key {
            Key::Right => self.tour_next(),
            Key::Enter => {
                self.tour_entered_at = self.clock;
                self.tour_next();
            }
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
    // One size whatever the language, each with its icon; Back stays as an empty place on the first stop.
    let fixed = |button: El| row().w(BUTTON).child(button);
    let mut buttons = row().w_full().gap(space::S2).center_y().child(fixed(btn(t::TOUR_SKIP()).icon(&icons::CLOSE).ghost().grow().focusable().build(k, |s, _| s.end_tour()))).child(spacer());
    if at > 0 {
        buttons = buttons.child(fixed(btn(t::TOUR_BACK()).icon(&icons::ARROW_LEFT).grow().focusable().build(k, |s, _| s.tour_back())));
    }
    let (label, glyph) = if last { (t::TOUR_DONE(), &icons::CHECK) } else { (t::TOUR_NEXT(), &icons::ARROW_RIGHT) };
    buttons = buttons.child(fixed(btn(label).icon(glyph).primary().grow().focusable().build(k, |s, _| s.tour_press_next()).key("tour-next")));
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
                .child(crate::ui::widgets::wrapped((stop.main)()).font(type_::TITLE).color(k.c.text_base).wrap())
                .child(crate::ui::widgets::wrapped((stop.more)()).font(type_::BODY).color(k.c.text_muted).wrap()),
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
        app.start_tour();
        for key in &keys {
            app.tour_key(key);
            assert_eq!(app.live.as_ref().map(|live| (live.deck.title.clone(), live.index)), live);
            assert!(!app.black);
            assert!(app.tour.is_some(), "only Enter, Left, Right and Escape move the tour");
        }
    }

    #[test]
    fn keys_move_the_tour() {
        let mut app = App::practice();
        app.start_tour();
        app.tour_key(&Key::Right);
        app.tour_key(&Key::Enter);
        assert_eq!(app.tour, Some(2));
        // If the focused button also presses itself for that Enter, the tour steps once.
        app.tour_press_next();
        assert_eq!(app.tour, Some(2));
        // A press of its own, later, steps.
        app.clock += 1.0;
        app.tour_press_next();
        assert_eq!(app.tour, Some(3));
        app.tour_key(&Key::Left);
        app.tour_key(&Key::Left);
        assert_eq!(app.tour, Some(1));
        app.tour_key(&Key::Escape);
        assert_eq!(app.tour, None);
    }
}
