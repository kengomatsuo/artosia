//! The tour: five stops through the main window, shown the first time Artosia starts and
//! whenever someone asks for it again. Each stop dims the window around one part of it, rings
//! that part in the accent colour and puts a small card beside it. It teaches the model (plan,
//! Preview, Show, Live, search), never a list of keys, and Skip is on every stop.
//!
//! While it is open it owns the keyboard (`App::tour_key`) and the card is a modal, so nothing
//! behind it can be pressed and nothing in it can reach the projector.

use rune::{Align, Cx, El, Key, Side, Tx, col, el, icon, row, spacer, text};
use rune_kit::Kit;

use crate::guide::lessons::Feature;
use crate::guide::window::{Action, Window};
use crate::guide::{Goal, Lesson, Runner, Show, Step};
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

/// A stop's card: where it sits beside the part its lesson step points at, and its words.
struct Stop {
    /// Which side of it the card sits on, and how it lines up.
    side: Side,
    align: Align,
    main: fn() -> &'static str,
    more: fn() -> &'static str,
}

/// The five stops and their words, in one table so they can be rewritten without touching the drawing.
const STOPS: [Stop; 5] = [
    Stop { side: Side::Right, align: Align::Start, main: t::TOUR_PLAN, more: t::TOUR_PLAN_MORE },
    // The card goes under the list, over the strip with the picture, which this stop is not about.
    Stop { side: Side::Bottom, align: Align::Start, main: t::TOUR_PREVIEW, more: t::TOUR_PREVIEW_MORE },
    // A ring goes round a container the shell keys, never one button: the buttons are rearranged often.
    Stop { side: Side::Top, align: Align::End, main: t::TOUR_SHOW, more: t::TOUR_SHOW_MORE },
    Stop { side: Side::Top, align: Align::End, main: t::TOUR_LIVE, more: t::TOUR_LIVE_MORE },
    Stop { side: Side::Bottom, align: Align::Start, main: t::TOUR_SEARCH, more: t::TOUR_SEARCH_MORE },
];

/// The whole tour after its five stops: each feature's lesson, in the order a volunteer meets
/// them, from the plan and its songs to the church's windows and the dialogs.
pub(crate) const COURSE: [Feature; 15] = [
    Feature::Plan,
    Feature::Songs,
    Feature::Bible,
    Feature::WebSongs,
    Feature::Notices,
    Feature::Templates,
    Feature::Pptx,
    Feature::Calendar,
    Feature::People,
    Feature::Giving,
    Feature::Look,
    Feature::Screens,
    Feature::Phone,
    Feature::Listen,
    Feature::Stream,
];

/// How many stops there are.
pub const STOP_COUNT: usize = STOPS.len();

/// The tour as the guide's first lesson: each stop explains the part its key names. The guide's
/// runner decides where the ring goes, so a later lesson of goal steps draws through this view.
const BASICS_STEPS: [Step<Action>; STOP_COUNT] = [
    Step { id: "plan", goal: Goal::Point("plan"), needs: None, done: None, restore: None },
    Step { id: "preview", goal: Goal::Point("slide-list"), needs: None, done: None, restore: None },
    Step { id: "show", goal: Goal::Point("preview-controls"), needs: None, done: None, restore: None },
    Step { id: "live", goal: Goal::Point("transport"), needs: None, done: None, restore: None },
    Step { id: "search", goal: Goal::Point("search-button"), needs: None, done: None, restore: None },
];
pub(crate) const BASICS: Lesson<Action> = Lesson { id: "basics", steps: &BASICS_STEPS };

impl App {
    /// Opens the tour at its first stop. On a church's library the tour shows the practice
    /// library instead, so every stop has songs, a plan and the church's windows filled in: the
    /// church's app waits in `church`, untouched, and comes back when the tour ends. Nothing the
    /// tour shows is saved, and the practice app never starts the projector, so the room keeps
    /// its last picture (TOURROOM1, `docs/plans/artosia/tour-room.md`).
    pub fn start_tour(&mut self) {
        if self.folder.is_some() && self.church.is_none() {
            // Nothing of the church's waits unsaved while its app is set aside.
            self.save_pending();
            let mut practice = App::load_in(None);
            // What the runtime gave the church's app, and the window's own choices.
            practice.waker = self.waker.clone();
            practice.images = self.images.take();
            practice.settings.language = self.settings.language.clone();
            practice.settings.appearance = self.settings.appearance;
            t::set_language(&practice.settings.language);
            let church = std::mem::replace(self, practice);
            self.church = Some(Box::new(church));
        }
        // The five stops open the whole tour; every feature's lesson follows them.
        self.course = Some(0);
        self.lesson = None;
        self.sheet = Sheet::None;
        self.close_palette();
        self.tour = Some(0);
        self.tour_focus = true;
    }

    /// Next, or Done on the last stop.
    pub fn tour_next(&mut self) {
        match self.tour {
            Some(at) if at + 1 < STOP_COUNT => self.tour = Some(at + 1),
            Some(_) => self.finish_stops(),
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

    /// Done on the last stop: the whole tour goes on with the features' lessons.
    fn finish_stops(&mut self) {
        if self.course.is_some() {
            self.tour = None;
            self.tour_focus = false;
            // The stops seen count as the tour seen, on the church's library while it waits aside.
            let owner = match self.church.as_mut() {
                Some(church) => church.as_mut(),
                None => self,
            };
            if !owner.settings.toured {
                owner.settings.toured = true;
                owner.touch_settings();
            }
            self.next_course_lesson();
        } else {
            self.end_tour();
        }
    }

    /// The next feature's lesson of the whole tour, or the end of the tour after the last.
    pub fn next_course_lesson(&mut self) {
        let Some(started) = self.course else { return };
        match COURSE.get(started) {
            Some(feature) => {
                self.start_lesson(feature.lesson());
                self.course = Some(started + 1);
            }
            None => self.end_tour(),
        }
    }

    /// Closes the tour from any stop or lesson of it, gives the church's library back, and
    /// remembers it was seen so it does not start by itself again.
    pub fn end_tour(&mut self) {
        let stops = self.tour.take().is_some();
        let course = self.course.take().is_some();
        if stops || course {
            self.tour_focus = false;
            if course {
                self.lesson = None;
            }
            // The church's own library comes back as it was left.
            if let Some(church) = self.church.take() {
                let mut church = *church;
                church.images = self.images.take();
                t::set_language(&church.settings.language);
                *self = church;
            }
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

impl App {
    /// Starts a lesson at its first step, over whatever the window shows.
    pub fn start_lesson(&mut self, lesson: Lesson<Action>) {
        SETTLED.with(|settled| settled.set(None));
        self.close_palette();
        self.lesson = Some((lesson, 0));
    }

    /// Skip on a lesson: within the whole tour the next feature's lesson starts.
    pub fn end_lesson(&mut self) {
        if self.course.is_some() {
            self.next_course_lesson();
        } else {
            self.lesson = None;
        }
    }

    /// Next on a lesson's explanation step: the step counts as read.
    pub fn lesson_next(&mut self) {
        settle_lesson(self);
        if let Some((_, at)) = self.lesson.as_mut() {
            *at += 1;
        }
    }

    /// The centred bubble's button: the search, with the step's command already typed, or the
    /// tab itself when the step asks for a tab.
    fn lesson_search(&mut self, action: Action) {
        settle_lesson(self);
        match action {
            Action::Run(command) => self.open_palette(crate::palette::Scope::All, command.label()),
            Action::Tab(tab) => self.open_tab(tab),
            Action::Press(_) => {}
        }
    }

    /// Whether this version brought features the church has not been shown.
    pub fn has_news(&self) -> bool {
        !crate::guide::news::unseen(&self.settings.news_seen, env!("CARGO_PKG_VERSION")).is_empty()
    }

    /// What's new closes and its version counts as seen.
    pub fn close_news(&mut self) {
        self.news = None;
        if let Some(version) = crate::guide::news::newest(env!("CARGO_PKG_VERSION")).filter(|version| self.settings.news_seen != *version) {
            self.settings.news_seen = version.to_string();
            self.touch_settings();
        }
    }

    /// An entry's Try: What's new closes and the feature's lesson starts.
    pub fn try_news(&mut self, feature: Feature) {
        self.close_news();
        self.show_me(feature);
    }

    /// A help mark's press: its card opens, or closes when it was open.
    pub fn toggle_help(&mut self, feature: Feature) {
        self.help = if self.help == Some(feature) { None } else { Some(feature) };
    }

    /// The help card's Show me: the feature's lesson starts and the card closes.
    pub fn show_me(&mut self, feature: Feature) {
        self.help = None;
        self.start_lesson(feature.lesson());
    }
}

/// A running lesson: the ring round the control the guide picked, a dim that swallows presses
/// everywhere but there, and a bubble that waits for the person's own press. With no control on
/// screen the bubble sits in the middle and opens the search with the command typed.
fn lesson_view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) {
    let Some((lesson, at)) = s.lesson else { return };
    let shown = {
        // Where the window shows each part now: a control in a scrolled list moves with it.
        let layout = |key: &str| cx.window_layout(key);
        Runner::starting_at(lesson, at).advance(&Window { app: s, layout: &layout })
    };
    let reached = match &shown {
        Show::At { step, .. } | Show::Search { step } => Some(*step),
        Show::Finished => None,
    };
    SETTLED.with(|settled| settled.set(Some((lesson.id, reached))));
    let dim = rune::Color::BLACK.alpha(0.38);
    let total = lesson.steps.len();
    match shown {
        Show::Finished => {}
        Show::At { step, key } => {
            REVEAL.with(|reveal| reveal.borrow_mut().0 = Some(key.clone()));
            let Some([x, y, w, h]) = cx.window_layout(&key) else { return };
            let (x0, y0) = ((x - PAD).max(0.0), (y - PAD).max(0.0));
            let (x1, y1) = (x + w + PAD, y + h + PAD);
            // The dim takes presses so only the ringed control can be pressed (Driver's overlay).
            let strip = || el().fill(dim).a11y_hidden().on_press_tx(|_: &mut App, _: &mut Tx| {});
            layer.push(strip().key("lesson-dim-top").pin(Some(0.0), Some(0.0), Some(0.0), None).h(y0));
            layer.push(strip().key("lesson-dim-bottom").pin(Some(0.0), Some(y1), Some(0.0), Some(0.0)));
            layer.push(strip().key("lesson-dim-left").pin(Some(0.0), Some(y0), None, None).w(x0).h(y1 - y0));
            layer.push(strip().key("lesson-dim-right").pin(Some(x1), Some(y0), Some(0.0), None).h(y1 - y0));
            layer.push(el().key("lesson-ring").absolute(x0, y0).size(x1 - x0, y1 - y0).radius(k.radius.lg).border(3.0, theme::accent()).a11y_hidden());
            let window = cx.layout_size("window").unwrap_or([1366.0, 768.0]);
            let card = bubble(s, k, lesson.steps[step].id, step, total, None);
            // A kit dialog's box is keyed by its sheet's name; the part may sit inside it.
            let dialog = (s.sheet != Sheet::None).then(|| format!("{:?}-dialog", s.sheet).to_lowercase()).and_then(|key| cx.layout(&key));
            let tall = cx.layout("lesson-card").map_or(260.0, |rect| rect[3]);
            match dialog.filter(|[dx, dy, dw, dh]| x >= *dx && y >= *dy && x + w <= dx + dw && y + h <= dy + dh).and_then(|box_| beside(box_, y0, window, tall)) {
                Some((bx, by)) => layer.push(card.absolute(bx, by)),
                None => {
                    let side = if y1 + 220.0 < window[1] { Side::Bottom } else { Side::Top };
                    layer.push(card.anchored(&key, side, Align::Start, space::S3));
                }
            }
        }
        Show::Search { step } => {
            let command = match lesson.steps[step].goal {
                crate::guide::Goal::Run(Action::Press(_)) => None,
                crate::guide::Goal::Run(action) => Some(action),
                _ => None,
            };
            layer.push(el().key("lesson-dim").absolute(0.0, 0.0).w_full().h_full().fill(dim).a11y_hidden().on_press_tx(|_: &mut App, _: &mut Tx| {}));
            layer.push(col().key("lesson-centre").absolute(0.0, 0.0).w_full().h_full().center().child(bubble(s, k, lesson.steps[step].id, step, total, command)));
        }
    }
}

/// Where a lesson card goes when the part it explains is inside a dialog: beside the dialog at the
/// part's height, right first, so the card never covers the dialog it explains. None when the
/// window has no room on either side.
fn beside(dialog: [f32; 4], top: f32, window: [f32; 2], tall: f32) -> Option<(f32, f32)> {
    let [dx, _, dw, _] = dialog;
    let gap = space::S3;
    let y = top.min(window[1] - tall - gap).max(gap);
    if window[0] - (dx + dw) >= CARD + 2.0 * gap {
        Some((dx + dw + gap, y))
    } else if dx >= CARD + 2.0 * gap {
        Some((dx - gap - CARD, y))
    } else {
        None
    }
}

thread_local! {
    /// Where the last frame found the lesson: its id and the step on screen, or None when done.
    static SETTLED: std::cell::Cell<Option<(&'static str, Option<usize>)>> = const { std::cell::Cell::new(None) };
}

/// Keeps where the last frame found the lesson, so a step that was done stays passed when the
/// window moves on (a template's Use leaves the Templates tab), Next counts from the step on
/// screen, and a finished lesson ends. Called by the lesson's own buttons and by the poll.
pub(crate) fn settle_lesson(app: &mut App) -> bool {
    let Some((id, reached)) = SETTLED.with(|settled| settled.take()) else { return false };
    match (reached, app.lesson.as_mut()) {
        (Some(step), Some((lesson, at))) if lesson.id == id && *at != step => {
            *at = step;
            true
        }
        (None, Some((lesson, _))) if lesson.id == id => {
            if app.course.is_some() {
                app.next_course_lesson();
            } else {
                app.lesson = None;
            }
            true
        }
        _ => false,
    }
}

thread_local! {
    /// The part the lesson rings now, and the last one scrolled into view.
    static REVEAL: std::cell::RefCell<(Option<String>, Option<String>)> = const { std::cell::RefCell::new((None, None)) };
}

/// Scrolls the part a lesson rings into view the first time it is rung, so a control at the
/// foot of a list is never ringed off screen. Called from the poll, which has a `Tx`.
pub(crate) fn reveal_ring(lesson_on: bool, tx: &mut Tx) -> bool {
    REVEAL.with(|reveal| {
        let mut reveal = reveal.borrow_mut();
        if !lesson_on {
            *reveal = (None, None);
            return false;
        }
        let (want, done) = &mut *reveal;
        match want.clone() {
            Some(key) if done.as_ref() != Some(&key) => {
                tx.scroll_into_view(key.clone());
                *done = Some(key);
                true
            }
            _ => false,
        }
    })
}

thread_local! {
    /// The clip of the step on screen, decoded once: the window's image cache plays its frames.
    static CLIPS: std::cell::RefCell<std::collections::HashMap<String, rune::Image>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The step's recorded clip, when it has one and the window can draw pictures.
fn clip_of(s: &App, id: &str) -> Option<rune::Image> {
    let bytes = crate::guide::lessons::clip(id)?;
    let images = s.images.as_ref()?;
    // Only the clip on screen is kept: a decoded clip is about 6 MB of frames, and a budget PC
    // walking through every lesson must not hold them all.
    Some(CLIPS.with(|clips| {
        let mut clips = clips.borrow_mut();
        clips.retain(|kept, _| kept == id);
        clips.entry(id.to_string()).or_insert_with(|| images.load(bytes)).clone()
    }))
}

/// The clip's size in the bubble: the card's width inside its padding, at 16 to 9.
const CLIP_W: f32 = CARD - 2.0 * space::S4;

/// The step's bubble: where it is in the lesson, its words, its clip of the move, Skip, and the
/// search when the control is not on screen. No Next: the step ends on the person's own press.
fn bubble(s: &App, k: &Kit, id: &str, step: usize, total: usize, search: Option<Action>) -> El {
    let (main, more) = crate::guide::lessons::words(id);
    // With no control on screen the card points at the search and names what to find there.
    let main = match search {
        Some(Action::Run(command)) => t::guide_find(command.label()),
        _ => main,
    };
    let main = main.as_str();
    let mut buttons = row().w_full().gap(space::S2).center_y().child(row().w(BUTTON).child(btn(t::TOUR_SKIP()).icon(&icons::CLOSE).ghost().grow().focusable().build(k, |s, _| s.end_lesson()))).child(spacer());
    if s.course.is_some() {
        buttons = buttons.child(row().w(BUTTON).child(btn(t::TOUR_END()).ghost().grow().focusable().build(k, |s, _| s.end_tour()).key("tour-end")));
    }
    if let Some(command) = search {
        buttons = buttons.child(btn(t::GUIDE_OPEN_SEARCH()).icon(&rune_kit::glyphs::SEARCH).primary().focusable().build(k, move |s, _| s.lesson_search(command)).key("lesson-search"));
    }
    let explains = s.lesson.is_some_and(|(lesson, _)| matches!(lesson.steps.get(step).map(|step| step.goal), Some(crate::guide::Goal::Point(_))));
    if explains {
        buttons = buttons.child(row().w(BUTTON).child(btn(t::TOUR_NEXT()).icon(&icons::ARROW_RIGHT).primary().grow().focusable().build(k, |s, _| s.lesson_next()).key("lesson-next")));
    }
    col()
        .key("lesson-card")
        .role("dialog")
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
                .children(s.course.filter(|started| *started > 0).and_then(|started| COURSE.get(started - 1)).map(|feature| {
                    let started = s.course.unwrap_or(1);
                    text(t::course_part(started, COURSE.len(), feature.name())).key("course-part").font(type_::SMALL_STRONG).color(theme::accent())
                }))
                .child(text(t::tour_place(step + 1, total)).key(format!("lesson-step-{id}")).font(type_::SMALL).color(k.c.text_muted))
                .child(crate::ui::widgets::wrapped(main).font(type_::TITLE).color(k.c.text_base).wrap())
                .child(crate::ui::widgets::wrapped(more).font(type_::BODY).color(k.c.text_muted).wrap()),
        )
        .children(clip_of(s, id).map(|clip| {
            el().key("lesson-clip").size(CLIP_W, (CLIP_W * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, k.c.border_base).pixels_fit(clip.pixels(), rune::SurfaceFit::Fill).a11y_hidden()
        }))
        .child(buttons)
}

/// A question mark in a ring, on the 16 point grid of the window's own icons.
static HELP_MARK: rune::Icon = rune::Icon {
    name: "g-help",
    view: 16.0,
    body: r#"<circle cx="8" cy="8" r="6.5" stroke="currentColor"/><path d="M6.2 6.3C6.4 5.4 7.1 4.8 8 4.8C9 4.8 9.8 5.5 9.8 6.4C9.8 7.6 8 7.9 8 9.2" stroke="currentColor" stroke-linecap="round"/><circle cx="8" cy="11.3" r="0.7" fill="currentColor"/>"#,
};

/// A feature's help mark: a small round mark in or beside the feature, found again whenever it
/// is needed (AirOps' and Evernote's info marks in the research). It opens the feature's card.
pub(crate) fn help_mark(s: &App, k: &Kit, feature: Feature) -> El {
    let on = s.help == Some(feature);
    btn(t::GUIDE_HELP()).icon(&HELP_MARK).icon_only().ghost().small().active(on).focusable().build(k, move |s, _| s.toggle_help(feature)).key(feature.key())
}

/// A dialog's help, as the first row of its body (a dialog has no room for a mark in its title):
/// what the dialog is for and Show me, which starts its lesson where the dialog already is.
pub(crate) fn help_row(k: &Kit, feature: Feature) -> El {
    row()
        .key(feature.key())
        .w_full()
        .gap(space::S3)
        .padding(space::S3)
        .radius(k.radius.md)
        .fill(k.c.layer_03)
        .center_y()
        .child(icon(&HELP_MARK, 16.0).color(k.c.text_muted).a11y_hidden())
        .child(crate::ui::widgets::wrapped(feature.about()).font(type_::SMALL).color(k.c.text_muted).wrap().grow(1.0))
        .child(btn(t::GUIDE_SHOW_ME()).small().focusable().build(k, move |s, _| s.show_me(feature)))
}

/// The open help card under its mark: what the feature is for, Show me, and Close.
fn help_card(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(feature) = s.help else { return };
    let buttons = row()
        .w_full()
        .gap(space::S2)
        .center_y()
        .child(spacer())
        .child(btn(t::BTN_CLOSE()).icon(&icons::CLOSE).ghost().focusable().build(k, |s, _| s.help = None))
        .child(btn(t::GUIDE_SHOW_ME()).icon(&icons::ARROW_RIGHT).primary().focusable().build(k, move |s, _| s.show_me(feature)).key("help-show-me"));
    let card = col()
        .key("help-card")
        .role("dialog")
        .label(feature.name())
        .w(CARD)
        .padding(space::S4)
        .gap(space::S3)
        .radius(k.radius.lg)
        .fill(k.c.layer_04)
        .border(1.0, k.c.border_strong)
        .shadow(8.0, 24.0, rune::Color::BLACK.alpha(0.4))
        .child(text(feature.name()).font(type_::TITLE).color(k.c.text_base))
        .child(crate::ui::widgets::wrapped(feature.about()).font(type_::BODY).color(k.c.text_muted).wrap())
        .child(buttons);
    layer.push(card.anchored(feature.key(), Side::Bottom, Align::End, space::S2));
}

/// What's new: the release's features as a list, and the picked one's clip playing beside it
/// with what it is for and Try (VS Code's walkthrough layout in the research). One clip plays,
/// so one clip is decoded, and the list fits any window.
fn news_view(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let Some(picked) = s.news else { return };
    let running = env!("CARGO_PKG_VERSION");
    // After an update: what came since the last one seen. Opened by hand with nothing unseen:
    // the newest release's entries, so the panel is never empty.
    let unseen = crate::guide::news::unseen(&s.settings.news_seen, running);
    let (version, shown) = match crate::guide::news::RELEASES.first() {
        Some((latest, features)) if unseen.is_empty() => (*latest, features.to_vec()),
        _ => (crate::guide::news::newest(running).unwrap_or(running), unseen),
    };
    let Some(&current) = shown.get(picked.min(shown.len().saturating_sub(1))) else { return };
    let mut list = col().key("news-list").role("list").w(NEWS_LIST_W).gap(2.0).focus_group().focus_follows();
    for (at, feature) in shown.iter().copied().enumerate() {
        let on = feature == current;
        list = list.child(
            row()
                .key(format!("news-{}", feature.key()))
                .role("listitem")
                .label(feature.name())
                .w_full()
                .h(40.0)
                .px(space::S3)
                .gap(space::S2)
                .center_y()
                .radius(k.radius.md)
                .fill(if on { k.c.layer_03 } else { rune::Color::TRANSPARENT })
                .hover(k.c.hover)
                .cursor(rune::Cursor::Pointer)
                .focusable(true)
                .on_press(move |s: &mut App| s.news = Some(at))
                .child(icon(crate::ui::tabs::glyph(feature.tab()), 16.0).color(if on { k.c.text_base } else { k.c.text_muted }).a11y_hidden())
                .child(text(feature.name()).font(if on { type_::BODY_STRONG } else { type_::BODY }).color(k.c.text_base)),
        );
    }
    let mut detail = col().key("news-detail").grow(1.0).gap(space::S3);
    detail = match clip_of(s, current.clip_step()) {
        Some(clip) => detail.child(el().key("news-clip").size(NEWS_CLIP_W, (NEWS_CLIP_W * 9.0 / 16.0).round()).radius(k.radius.md).clip_rounded(k.radius.md).border(1.0, k.c.border_base).pixels_fit(clip.pixels(), rune::SurfaceFit::Fill).a11y_hidden()),
        None => detail,
    };
    detail = detail
        .child(text(current.name()).font(type_::TITLE).color(k.c.text_base))
        .child(crate::ui::widgets::wrapped(current.about()).font(type_::BODY).color(k.c.text_muted).wrap())
        .child(row().child(btn(t::WHATSNEW_TRY()).icon(&icons::ARROW_RIGHT).primary().focusable().build(k, move |s, _| s.try_news(current)).key("news-try")));
    let card = col()
        .key("news-card")
        .role("dialog")
        .label(t::CMD_WHATSNEW())
        .w(NEWS_W)
        .padding(space::S5)
        .gap(space::S4)
        .radius(k.radius.lg)
        .fill(k.c.layer_04)
        .border(1.0, k.c.border_strong)
        .shadow(8.0, 24.0, rune::Color::BLACK.alpha(0.4))
        .child(text(t::whats_new_title(version)).font(type_::HEADING).color(k.c.text_base))
        .child(row().w_full().gap(space::S5).child(list).child(detail))
        .child(row().w_full().child(spacer()).child(btn(t::BTN_CLOSE()).icon(&icons::CLOSE).focusable().build(k, |s, _| s.close_news()).key("news-close")))
        .modal(true, |s: &mut App| s.close_news());
    layer.push(el().key("news-dim").absolute(0.0, 0.0).w_full().h_full().fill(rune::Color::BLACK.alpha(0.45)).a11y_hidden());
    layer.push(col().key("news-centre").absolute(0.0, 0.0).w_full().h_full().center().child(card));
}

/// A downloaded update offered: what it is, that restarting is short, Later and Restart now
/// (Sparkle's install and remind-later choices; the button in the top bar keeps it findable).
fn update_card(s: &App, k: &Kit, layer: &mut Vec<El>) {
    let (true, Some(version)) = (s.update_card, s.update_waiting.as_ref()) else { return };
    let body = col()
        .w_full()
        .px(space::S4)
        .pb(space::S4)
        .gap(space::S4)
        .child(crate::ui::widgets::wrapped(t::UPDATE_CARD_HINT()).font(type_::BODY).color(k.c.text_muted).wrap())
        .child(
            row()
                .w_full()
                .gap(space::S2)
                .child(rune::spacer())
                .child(btn(t::UPDATE_LATER()).focusable().build(k, |s, _| s.update_card = false).key("update-later"))
                .child(btn(t::UPDATE_RESTART()).icon(&icons::IMPORT).primary().focusable().build(k, |s, tx| s.restart_to_update(tx)).key("update-restart").autofocus()),
        );
    rune_kit::Dialog::new("update", t::update_ready(version)).open(true).width(440.0).fit().on_close(|s: &mut App| s.update_card = false).child(body).build(k, layer);
}

/// What's new's width, its list's width and the picked entry's clip.
const NEWS_W: f32 = 760.0;
const NEWS_LIST_W: f32 = 200.0;
const NEWS_CLIP_W: f32 = 480.0;

/// The dim around the part a stop points at, the ring, and the card.
pub(crate) fn view(s: &App, cx: &mut Cx<App>, k: &Kit, layer: &mut Vec<El>) {
    update_card(s, k, layer);
    news_view(s, k, layer);
    help_card(s, k, layer);
    lesson_view(s, cx, k, layer);
    let Some(at) = s.tour else { return };
    let stop = &STOPS[at.min(STOP_COUNT - 1)];
    let dim = rune::Color::BLACK.alpha(0.38);
    let card = card(k, at, stop);
    let shown = {
        let layout = |key: &str| cx.layout(key);
        Runner::starting_at(BASICS, at).advance(&Window { app: s, layout: &layout })
    };
    let target = match shown {
        Show::At { key, .. } => cx.layout(&key).map(|rect| (key, rect)),
        Show::Search { .. } | Show::Finished => None,
    };
    match target {
        Some((key, [x, y, w, h])) => {
            let (x0, y0) = ((x - PAD).max(0.0), (y - PAD).max(0.0));
            let (x1, y1) = (x + w + PAD, y + h + PAD);
            let strip = || el().fill(dim).a11y_hidden();
            // Four strips leave the part itself undimmed.
            layer.push(strip().key("tour-dim-top").pin(Some(0.0), Some(0.0), Some(0.0), None).h(y0));
            layer.push(strip().key("tour-dim-bottom").pin(Some(0.0), Some(y1), Some(0.0), Some(0.0)));
            layer.push(strip().key("tour-dim-left").pin(Some(0.0), Some(y0), None, None).w(x0).h(y1 - y0));
            layer.push(strip().key("tour-dim-right").pin(Some(x1), Some(y0), Some(0.0), None).h(y1 - y0));
            layer.push(el().key("tour-ring").absolute(x0, y0).size(x1 - x0, y1 - y0).radius(k.radius.lg).border(3.0, theme::accent()).a11y_hidden());
            layer.push(card.anchored(&key, stop.side, stop.align, space::S3));
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
    fn a_card_sits_beside_a_dialog_with_room_and_never_off_the_window() {
        let window = [1366.0, 768.0];
        let (x, y) = beside([258.0, 20.0, 483.0, 700.0], 100.0, window, 200.0).expect("room on the right");
        assert!(x > 258.0 + 483.0 && x + CARD <= window[0] && y == 100.0);
        let (_, low) = beside([258.0, 20.0, 483.0, 700.0], 700.0, window, 200.0).unwrap();
        assert!(low + 200.0 <= window[1], "a part low in the window pulls the card up");
        assert_eq!(beside([403.0, 80.0, 560.0, 600.0], 100.0, window, 200.0), None, "no room either side");
    }

    #[test]
    fn a_church_tour_shows_the_practice_library_and_gives_the_church_its_own_back() {
        let folder = std::env::temp_dir().join(format!("artosia-tour-room-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder for the church's library");
        let mut app = App::load_in(Some(folder.clone()));
        assert!(app.songs.is_empty(), "a new church starts with no songs");
        app.start_tour();
        assert!(!app.songs.is_empty(), "the tour shows the practice songs");
        assert!(app.folder.is_none() && app.church.is_some(), "the church's app waits aside");
        app.end_tour();
        assert!(app.songs.is_empty(), "the church's own library comes back, with nothing of the tour in it");
        assert_eq!(app.folder.as_ref(), Some(&folder));
        assert!(app.settings.toured, "the tour counts as seen");
        assert!(app.store.songs().unwrap_or_default().is_empty(), "nothing the tour showed was saved");
        drop(app);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_whole_tour_runs_every_lesson_after_the_stops_and_ends_on_end_tour() {
        let mut app = App::practice();
        app.start_tour();
        assert_eq!((app.tour, app.course), (Some(0), Some(0)));
        for _ in 0..STOP_COUNT {
            app.tour_next();
        }
        assert!(app.tour.is_none(), "the stops are done");
        assert_eq!(app.lesson.map(|(lesson, _)| lesson.id), Some(COURSE[0].lesson().id), "the first feature's lesson follows");
        app.end_lesson();
        assert_eq!(app.lesson.map(|(lesson, _)| lesson.id), Some(COURSE[1].lesson().id), "Skip goes to the next lesson");
        app.end_tour();
        assert!(app.lesson.is_none() && app.course.is_none(), "End tour ends all of it");
    }

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
