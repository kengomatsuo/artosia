//! The window's own small parts. Buttons here take keyboard focus only where asked: after a
//! click on Black, Space must still mean "next slide", not "press Black again". Dialogs and
//! the editors ask for it, so Tab reaches every button in them.

use rune::{Color, Cursor, El, Icon, Span, TextStyle, Tx, el, icon, rich, row, text};
use rune_kit::Kit;

use crate::state::App;
use crate::theme::{self, type_};

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
    height: f32,
    grow: bool,
    disabled: bool,
    help: Option<String>,
    icon_only: bool,
    focus: bool,
}

pub fn btn(label: impl Into<String>) -> Btn {
    Btn {
        label: label.into(),
        icon: None,
        kind: Kind::Plain,
        hint: None,
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
    /// A height worked out by the caller, for controls that fill the strip beside a picture.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
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
        let pad = if self.icon_only { 0.0 } else if self.height <= 26.0 { 8.0 } else { 12.0 };
        let mut e = row().role("button").label(self.label.clone()).h(self.height).px(pad).gap(6.0).center().radius(k.radius.md).fill(fill);
        if self.icon_only {
            e = e.w(self.height);
        }
        if let Some(border) = border {
            e = e.border(1.0, border);
        }
        if let Some(i) = self.icon {
            e = e.child(icon(i, 16.0).color(ink).a11y_hidden());
        }
        if !self.icon_only {
            e = e.child(text(self.label.clone()).font(font).color(ink).truncate().a11y_hidden());
        }
        if let Some(hint) = self.hint {
            // A key cap on a filled button is inked for that fill, whatever the window's theme.
            e = e.child(match self.kind {
                Kind::Primary => keycap_on(&hint, ink),
                Kind::Toggle(true) => keycap_on(&hint, Color::BLACK),
                _ => keycap(k, &hint, false),
            });
        }
        if self.grow {
            e = e.grow(1.0);
        }
        let tip = self.help.or(self.icon_only.then(|| self.label.clone()));
        if let Some(tip) = tip {
            e = e.help(tip);
        }
        if self.disabled {
            return e.layer_opacity(0.45).disabled(true);
        }
        if self.focus {
            e = e.focusable(true);
        }
        e.hover(hover).press(pressed).cursor(Cursor::Pointer).on_press_tx(move |s: &mut App, tx: &mut Tx| {
            press(s, tx);
        })
    }
}

/// A key name in a small box, as printed on a key.
pub fn keycap(k: &Kit, keys: &str, on_light: bool) -> El {
    let (fill, ink) = if on_light { (Color::BLACK.alpha(0.12), Color::BLACK.alpha(0.72)) } else { (k.c.layer_03, k.c.text_muted) };
    el().px(5.0).h(18.0).center().radius(4.0).fill(fill).child(text(keys).font(type_::TINY).color(ink)).a11y_hidden()
}

/// A key cap on a filled button, inked like the button's label.
fn keycap_on(keys: &str, ink: Color) -> El {
    el().px(5.0).h(18.0).center().radius(4.0).fill(ink.alpha(0.14)).child(text(keys).font(type_::TINY).color(ink.alpha(0.8))).a11y_hidden()
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
