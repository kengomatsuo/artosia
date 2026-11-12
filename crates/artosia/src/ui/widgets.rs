//! The window's own small parts. Buttons here take keyboard focus only where asked: after a
//! click on Black, Space must still mean "next slide", not "press Black again". Dialogs and
//! the editors ask for it, so Tab reaches every button in them.

use crate::strings as t;
use rune::{Color, Cursor, El, Icon, Span, TextStyle, Tx, el, icon, rich, row, text};
use rune_kit::Kit;

use crate::state::App;
use crate::theme::{self, icons, type_};

/// Enter presses the button that has focus, as Space does. Rune follows AppKit, where Return
/// belongs to the default button and only Space presses the focused one; this program runs
/// on Windows, where a focused button takes both (Microsoft, Keyboard accessibility: "For
/// controls that can be invoked, implement keyboard event handlers for the Spacebar and Enter
/// keys"; WAI-ARIA button pattern: "Enter: Activates the button"). A keys-only tester took
/// every dialog button for broken (2026-11-11). Hung on each element that asks for focus.
pub fn enter_key(s: &mut App, ev: &rune::KeyEvent, tx: &mut Tx) -> bool {
    // The tour and the first run own Enter: it means the next card, or the page's primary button, even while another button holds the focus.
    if ev.key != rune::Key::Enter || ev.mods != rune::Mods::default() || s.tour.is_some() || s.onboard.is_some() {
        return false;
    }
    if !tx.key_repeat() {
        tx.send_key(rune::KeyEvent { key: rune::Key::Text(" ".into()), mods: rune::Mods::default() });
    }
    true
}

/// Enter starts a new line in a box of several lines. Rune at the pinned rev treats such a
/// box as a chat composer: Return submits and only Shift Return breaks the line
/// (`edit/input.rs`, `Block::Paragraph => out.submit = true`), so Enter did nothing at all in
/// the lyrics (found pressing real keys, 2026-11-11). Every `TextArea` calls this first in its
/// `on_key` and passes the key on as Shift Return, which the field turns into the line break.
pub fn enter_breaks_line(ev: &rune::KeyEvent, tx: &mut Tx) -> bool {
    if ev.key != rune::Key::Enter || ev.mods != rune::Mods::default() {
        return false;
    }
    tx.send_key(rune::KeyEvent { key: rune::Key::Enter, mods: rune::Mods { shift: true, ..rune::Mods::default() } });
    true
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Plain,
    Primary,
    Ghost,
    /// A switch drawn as a button: filled with the warning colour while on.
    Toggle(bool),
    /// Filled with the accent: running now.
    Active,
}

pub struct Btn {
    label: String,
    icon: Option<&'static Icon>,
    kind: Kind,
    hint: Option<String>,
    /// An arrow key drawn as a key cap before the label, for a button an arrow key presses.
    key_icon: Option<&'static Icon>,
    height: f32,
    grow: bool,
    disabled: bool,
    help: Option<String>,
    icon_only: bool,
    focus: bool,
}

/// Text for a box that wraps. Every wrapping text of the window starts here, so a Korean word
/// is never cut in two (`artosia_core::text::keep_words`).
pub fn wrapped(words: impl Into<String>) -> El {
    text(artosia_core::text::keep_words(&words.into()).into_owned())
}

pub fn btn(label: impl Into<String>) -> Btn {
    Btn {
        label: label.into(),
        icon: None,
        kind: Kind::Plain,
        hint: None,
        key_icon: None,
        height: 32.0,
        grow: false,
        disabled: false,
        help: None,
        icon_only: false,
        focus: false,
    }
}

impl Btn {
    pub fn icon(mut self, icon: &'static Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn primary(mut self) -> Self {
        self.kind = Kind::Primary;
        self
    }
    pub fn ghost(mut self) -> Self {
        self.kind = Kind::Ghost;
        self
    }
    pub fn toggle(mut self, on: bool) -> Self {
        self.kind = Kind::Toggle(on);
        self
    }
    /// Fills the button with the accent while what it stands for is running, as a stream is.
    /// Red stays for what is on the projector.
    pub fn active(mut self, on: bool) -> Self {
        if on {
            self.kind = Kind::Active;
        }
        self
    }
    /// A key cap after the label, such as "F1".
    pub fn hint(mut self, keys: impl Into<String>) -> Self {
        self.hint = Some(keys.into());
        self
    }
    pub fn tall(mut self) -> Self {
        self.height = 44.0;
        self
    }
    pub fn small(mut self) -> Self {
        self.height = 26.0;
        self
    }
    pub fn grow(mut self) -> Self {
        self.grow = true;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }
    /// Lets Tab reach the button, for dialogs and editors where Space is not "next slide".
    pub fn focusable(mut self) -> Self {
        self.focus = true;
        self
    }
    /// Shows only the icon; the label still names the button for a screen reader and a tooltip.
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }

    pub fn build(self, k: &Kit, press: impl Fn(&mut App, &mut Tx) + 'static) -> El {
        let c = &k.c;
        let (fill, ink, border) = match self.kind {
            Kind::Plain => (c.layer_02, c.text_base, Some(c.border_base)),
            Kind::Primary => (c.text_base, c.page, None),
            Kind::Ghost => (Color::TRANSPARENT, c.text_muted, None),
            Kind::Toggle(true) => (theme::warn(), Color::BLACK, None),
            Kind::Toggle(false) => (c.layer_02, c.text_base, Some(c.border_base)),
            Kind::Active => (theme::accent(), Color::WHITE, None),
        };
        let dark_fill = matches!(self.kind, Kind::Primary | Kind::Toggle(true) | Kind::Active);
        let (hover, pressed) = if dark_fill { (Color::BLACK.alpha(0.08), Color::BLACK.alpha(0.16)) } else { (c.hover, c.pressed) };
        let font = if self.height >= 44.0 { type_::TITLE } else { type_::BODY_STRONG };
        let pad = if self.icon_only && self.hint.is_none() { 0.0 } else if self.icon_only { 6.0 } else if self.height <= 26.0 { 8.0 } else { 12.0 };
        let mut e = row().role("button").label(self.label.clone()).h(self.height).px(pad).gap(6.0).center().radius(k.radius.md).fill(fill);
        // An icon alone is a square; with its key beside it the pair is as wide as it needs.
        if self.icon_only && self.hint.is_none() {
            e = e.w(self.height);
        } else if self.icon_only {
            e = e.px(6.0);
        }
        if let Some(border) = border {
            e = e.border(1.0, border);
        }
        if let Some(i) = self.icon {
            e = e.child(icon(i, 16.0).color(ink).a11y_hidden());
        }
        if let Some(i) = self.key_icon {
            e = e.child(el().size(20.0, 18.0).center().radius(4.0).fill(ink.alpha(0.14)).child(icon(i, 12.0).color(ink)).a11y_hidden());
        }
        if !self.icon_only {
            e = e.child(text(self.label.clone()).font(font).color(ink).truncate().a11y_hidden());
        }
        if let Some(hint) = self.hint {
            // A key cap on a filled button is inked for that fill, whatever the window's theme.
            e = e.child(match self.kind {
                Kind::Primary | Kind::Active => keycap_on(&hint, ink),
                Kind::Toggle(true) => keycap_on(&hint, Color::BLACK),
                _ => keycap(k, &hint, false),
            });
        }
        if self.grow {
            // A grown button shares its row; a label too long for its share is cut, never pushed past the edge.
            e = e.grow(1.0).min_w(0.0);
        }
        let tip = self.help.or(self.icon_only.then(|| self.label.clone()));
        if let Some(tip) = tip {
            e = e.help(tip);
        }
        if self.disabled {
            return e.layer_opacity(0.45).disabled(true);
        }
        if self.focus {
            e = e.focusable(true).on_key(crate::ui::widgets::enter_key);
        }
        e.hover(hover).press(pressed).cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx: &mut Tx| {
            press(s, tx);
        })
    }
}

/// Keys as a keyboard prints them: an arrow key is its arrow, the space bar is a bar, and
/// every other key is its name (owner, 2026-11-11: "show the exact thing showing on the
/// keyboard"). `spec` is the same in every language: "Ctrl Up Down", "Shift Tab", "Space".
/// A word in lower case ("to") is set as plain text between the keys.
pub fn keycaps(k: &Kit, spec: &str, on: Option<Color>) -> El {
    let (fill, ink) = match on {
        Some(ink) => (ink.alpha(0.14), ink.alpha(0.85)),
        None => (k.c.layer_03, k.c.text_muted),
    };
    let mut out = row().gap(3.0).center_y().a11y_hidden();
    for token in spec.split([' ', ',']).filter(|token| !token.is_empty()) {
        let cap = || el().h(18.0).min_w(18.0).px(4.0).center().radius(4.0).fill(fill);
        let arrow = |glyph: &'static Icon| cap().child(icon(glyph, 12.0).color(ink));
        out = out.child(match token {
            "Up" => arrow(&icons::ARROW_UP),
            "Down" => arrow(&icons::ARROW_DOWN),
            "Left" => arrow(&icons::ARROW_LEFT),
            "Right" => arrow(&icons::ARROW_RIGHT),
            "Space" => cap().w(34.0).child(icon(&icons::SPACE_BAR, 14.0).color(ink)),
            // The two little words between key caps are words of the language.
            "to" => text(t::KEYS_TO()).font(type_::TINY).color(ink),
            "or" => text(t::KEYS_OR()).font(type_::TINY).color(ink),
            word if word.chars().all(|c| c.is_lowercase()) && word.len() > 1 => text(word.to_string()).font(type_::TINY).color(ink),
            name => cap().child(text(name.to_string()).font(type_::TINY).color(ink)),
        });
    }
    out
}

/// One key or chord in a small box.
pub fn keycap(k: &Kit, keys: &str, on_light: bool) -> El {
    keycaps(k, keys, on_light.then_some(Color::BLACK))
}

/// A key cap on a filled button, inked like the button's label.
fn keycap_on(keys: &str, ink: Color) -> El {
    keycaps(theme::kit(), keys, Some(ink))
}

/// How a tile is filled.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Plain,
    /// The one preferred action of its strip.
    Primary,
    /// A state that is on, in the colour that state has: yellow hides, red is live.
    On(Color, Color),
}

/// A button of the strips under the lists: an icon over one word, its key in a corner. Tiles
/// of one set are the same size, which comes from the strip and never from the word, so a
/// longer language changes nothing; the preferred one differs by fill, never by size (Apple's
/// guidelines on buttons: "Use style, not size, to visually distinguish the preferred choice").
pub struct Tile {
    label: String,
    icon: Option<&'static Icon>,
    /// The icon is an arrow key itself and is drawn as a key.
    key_face: bool,
    /// Icon, word and key side by side: for a tile much wider than it is high.
    wide: bool,
    keys: Option<&'static str>,
    tone: Tone,
    height: f32,
    /// The tile is too narrow for its key to sit beside its icon: the key gets a line of its own.
    narrow: bool,
    disabled: bool,
    help: Option<String>,
}

pub fn tile(label: impl Into<String>) -> Tile {
    Tile { label: label.into(), icon: None, key_face: false, wide: false, keys: None, tone: Tone::Plain, height: 64.0, narrow: false, disabled: false, help: None }
}

impl Tile {
    pub fn icon(mut self, icon: &'static Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// An arrow key as the tile's icon, drawn as the key it is.
    pub fn arrow_key(mut self, icon: &'static Icon) -> Self {
        self.icon = Some(icon);
        self.key_face = true;
        self
    }
    pub fn keys(mut self, keys: &'static str) -> Self {
        self.keys = Some(keys);
        self
    }
    /// Lays the icon, the word and the key in one line.
    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }
    pub fn primary(mut self) -> Self {
        self.tone = Tone::Primary;
        self
    }
    /// Filled while the state it names is on.
    pub fn on(mut self, on: bool, fill: Color, ink: Color) -> Self {
        if on {
            self.tone = Tone::On(fill, ink);
        }
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
    /// For a tile under about 76 points wide: the key in the corner gets its own line above the icon.
    pub fn narrow(mut self, narrow: bool) -> Self {
        self.narrow = narrow;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn help(mut self, text: impl Into<String>) -> Self {
        self.help = Some(text.into());
        self
    }

    pub fn build(self, k: &Kit, press: impl Fn(&mut App, &mut Tx) + 'static) -> El {
        let c = &k.c;
        let (fill, ink, border) = match self.tone {
            Tone::Plain => (c.layer_02, c.text_base, Some(c.border_base)),
            Tone::Primary => (c.text_base, c.page, None),
            Tone::On(fill, ink) => (fill, ink, None),
        };
        let filled = self.tone != Tone::Plain;
        let (hover, pressed) = if filled { (Color::BLACK.alpha(0.08), Color::BLACK.alpha(0.16)) } else { (c.hover, c.pressed) };
        let name = self.help.clone().unwrap_or_else(|| self.label.clone());
        if self.wide {
            let mut e = row().role("button").label(name).h(self.height).grow(1.0).min_w(0.0).px(12.0).gap(8.0).center_y().radius(k.radius.md).fill(fill);
            if let Some(border) = border {
                e = e.border(1.0, border);
            }
            if let Some(glyph) = self.icon {
                e = e.child(icon(glyph, 18.0).color(ink).a11y_hidden());
            }
            e = e.child(text(self.label.clone()).font(type_::BODY_STRONG).color(ink).truncate().grow(1.0).min_w(0.0).a11y_hidden());
            if let Some(keys) = self.keys {
                e = e.child(keycaps(k, keys, filled.then_some(ink)));
            }
            if let Some(tip) = self.help {
                e = e.help(tip);
            }
            if self.disabled {
                return e.layer_opacity(0.45).disabled(true);
            }
            return e.hover(hover).press(pressed).cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx: &mut Tx| press(s, tx));
        }
        let mut e = rune::col().role("button").label(name).h(self.height).grow(1.0).min_w(0.0).px(3.0).gap(4.0).center().radius(k.radius.md).fill(fill);
        if let Some(border) = border {
            e = e.border(1.0, border);
        }
        if self.narrow && self.keys.is_some() {
            e = e.child(el().h(14.0));
        }
        if let Some(glyph) = self.icon {
            e = e.child(if self.key_face {
                el().size(28.0, 24.0).center().radius(5.0).fill(ink.alpha(0.14)).child(icon(glyph, 16.0).color(ink)).a11y_hidden()
            } else {
                icon(glyph, 20.0).color(ink).a11y_hidden()
            });
        }
        // One word on one line. Two short words that do not fit side by side take two lines,
        // a size smaller: the tile's width is the strip's, never the word's.
        let long = self.label.chars().count() > 8;
        let label = match (long, self.label.contains(' ')) {
            (true, true) => crate::ui::widgets::wrapped(self.label.clone()).font(type_::TINY).color(ink).align(rune::Align::Center).wrap().max_lines(2),
            (true, false) => text(self.label.clone()).font(type_::TINY).color(ink).truncate(),
            _ => text(self.label.clone()).font(type_::SMALL_STRONG).color(ink).truncate(),
        };
        e = e.child(el().w_full().center().child(label.a11y_hidden()));
        if let Some(keys) = self.keys {
            // The key sits in the corner, where it never changes the tile's size.
            e = e.child(el().pin(None, Some(5.0), Some(5.0), None).child(keycaps(k, keys, filled.then_some(ink))));
        }
        if let Some(tip) = self.help {
            e = e.help(tip);
        }
        if self.disabled {
            return e.layer_opacity(0.45).disabled(true);
        }
        e.hover(hover).press(pressed).cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx: &mut Tx| press(s, tx))
    }
}

/// A hint line: a key cap and what it does.
pub fn key_hint(k: &Kit, keys: &str, does: &str) -> El {
    row().gap(6.0).center_y().child(keycap(k, keys, false)).child(text(does).font(type_::SMALL).color(k.c.text_muted))
}

/// The small heading over a panel section, in sentence case.
pub fn section_label(k: &Kit, label: &str) -> El {
    text(label).font(type_::SMALL_STRONG).color(k.c.text_muted)
}

/// A coloured pill naming a song section.
pub fn tag_chip(label: &str, color: Color) -> El {
    el().px(7.0).h(20.0).center().radius(6.0).fill(theme::tint_of(color, false)).child(text(label).font(type_::TINY).color(theme::ink_of(color)))
}

/// A section's chip: its whole name in the window's language, in the colour of its kind.
/// "V1" and "C" were a code to learn, and two native readers each said a volunteer who reads
/// Bait and Reff everywhere else never learns that C means Reff (2026-11-11). The code stays
/// only where it is typed (the song order) and where it is a key, drawn as a key.
pub fn section_chip(tag: artosia_core::song::Tag) -> El {
    tag_chip(&t::section_label(tag), artosia_slides::stage::group_color(artosia_core::deck::tag_group(tag)))
}

/// A filled pill for a state, such as "Live".
pub fn state_pill(label: &str, fill: Color, ink: Color) -> El {
    el().px(7.0).h(20.0).center().radius(6.0).fill(fill).child(text(label).font(type_::TINY).color(ink))
}

/// Text with the matched ranges set bold on a tint, for search results.
pub fn highlighted(content: &str, ranges: &[std::ops::Range<usize>], style: TextStyle, ink: Color, mark: Color) -> El {
    let spans: Vec<Span> = ranges
        .iter()
        .filter(|range| range.end <= content.len() && content.is_char_boundary(range.start) && content.is_char_boundary(range.end))
        .map(|range| {
            let mut span = Span::new(range.start, range.end).weight(700.0);
            span.color = Some(mark);
            span
        })
        .collect();
    rich(content, spans).font(style).color(ink)
}

pub fn divider(k: &Kit) -> El {
    el().h(1.0).w_full().fill(k.c.border_muted).a11y_hidden()
}

pub fn vertical_divider(k: &Kit) -> El {
    el().w(1.0).h_full().fill(k.c.border_muted).a11y_hidden()
}
