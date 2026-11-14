//! The announcement editor's stage (NOTICEEDIT1, `editing-mocks` E1a to E1d): the slide large, a
//! box of it picked with its outline, handles and small bar, the strip above it, the sheet of a
//! data part under it and the card that offers the second slide. Every number is the mock's
//! (`build/spec.json`); a colour comes from the window's own tokens.

use artosia_core::deck::{Align as Face, Fill, Template};
use artosia_slides::warta::{Field, Fit, TextBox};
use rune::{Align, Color, Cursor, El, Key, KeyEvent, ScrollSpec, TextStyle, Tx, col, el, icon, row, spacer, text, text_input};
use rune_kit::{Kit, ScrollBar, Select, SelectAppearance, SegmentedControl};

use super::announcements::{SECONDS, calendar_chips, edit_of, escape_closes, holds, passage_field, picture_rows, template_chips, window_panel};
use crate::edit_slide::{Overflow, Stage};
use crate::state::{App, Notices, Panel, Tab};
use crate::strings as t;
use crate::templates::Target;
use crate::theme::{self, icons, space, type_};
use crate::ui::colour::ColourFor;
use crate::ui::widgets::{btn, divider, enter_breaks_line, enter_key, section_label, vertical_divider};

/// The strip: its height, and what its controls stand in.
const STRIP: f32 = 38.0;
const CTRL: f32 = 26.0;
/// The height of the header above the strip, the strip's row and the bar under the stage.
pub const HEAD: f32 = 46.0;
pub const BAR: f32 = 48.0;
/// The space a stage keeps around its slide, and under a slide with a sheet or the card of E1d.
const AROUND: f32 = 30.0;
const SHEET: f32 = 192.0;
/// The sheet of the card's own settings has two rows of chips more than a data part's.
const SHEET_MORE: f32 = 236.0;
const OFFER: f32 = 84.0;
/// The selection outline's width and a handle's radius.
const OUTLINE: f32 = 1.5;
/// The margin of the strip, the sheet and the card from the stage's side edges.
const GUTTER: f32 = 15.0;
const HANDLE: f32 = 4.5;

/// The size the slide is drawn at, from what the stage leaves it.
fn slide_size(avail: [f32; 2], sheet: Option<f32>, offer: bool) -> [f32; 2] {
    let [w, h] = avail;
    let (reserve, around) = match (sheet, offer) {
        (Some(sheet), _) => (sheet + 4.0, 12.0),
        (_, true) => (OFFER + 22.0, 0.0),
        _ => (0.0, 2.0 * AROUND),
    };
    let high = (h - reserve - around).max(90.0);
    let wide = (w - 2.0 * AROUND).max(160.0);
    let (sw, sh) = if wide * 9.0 / 16.0 <= high { (wide, wide * 9.0 / 16.0) } else { (high * 16.0 / 9.0, high) };
    [sw.round(), sh.round()]
}

/// A word of the box a strip control acts on.
fn role_name(field: Field) -> &'static str {
    match field {
        Field::Title => t::FIELD_TITLE(),
        Field::Body => t::EDIT_ROLE_BODY(),
        Field::Rows => t::EDIT_PART_ROWS(),
        Field::Names => t::EDIT_PART_NAMES(),
        Field::Funds => t::EDIT_PART_FUNDS(),
        Field::Account => t::EDIT_PART_ACCOUNT(),
        Field::Verse => t::EDIT_PART_VERSE(),
    }
}

/// The window a data part is read from, as the tab that holds it.
fn source_tab(field: Field) -> Tab {
    match field {
        Field::Rows => Tab::Calendar,
        Field::Names => Tab::People,
        Field::Funds | Field::Account => Tab::Giving,
        _ => Tab::Plan,
    }
}

/// The part a card of this kind reads from a window, for the sheet of a window that holds nothing yet.
fn part_of(fill: Fill) -> Option<Field> {
    match fill {
        Fill::Schedule => Some(Field::Rows),
        Fill::Birthdays => Some(Field::Names),
        Fill::Offerings => Some(Field::Funds),
        Fill::Transfer => Some(Field::Account),
        Fill::Passage => Some(Field::Verse),
        Fill::Nothing => None,
    }
}

fn source_label(field: Field) -> &'static str {
    match field {
        Field::Rows => t::EDIT_READ_CAL(),
        Field::Names => t::EDIT_READ_MEMBERS(),
        Field::Funds => t::EDIT_READ_OFFERINGS(),
        Field::Account => t::NOTICE_READ_ACCOUNT(),
        _ => t::NOTICE_READ_BIBLE(),
    }
}

fn open_label(field: Field) -> &'static str {
    match field {
        Field::Rows => t::EDIT_OPEN_CALENDAR(),
        Field::Names => t::EDIT_OPEN_MEMBERS(),
        _ => t::EDIT_OPEN_OFFERINGS(),
    }
}

fn rgb_of(c: [u8; 3]) -> Color {
    artosia_slides::look::rgb(c)
}

/// A control of the strip: a label, an icon or both, in the mock's 26 high rounded box.
struct Tool<'a> {
    key: String,
    label: &'a str,
    glyph: Option<&'static rune::Icon>,
    shown: Option<&'a str>,
    lit: bool,
    warn: bool,
    ghost: bool,
    solid: bool,
    tall: bool,
}

fn tool<'a>(key: impl Into<String>, label: &'a str) -> Tool<'a> {
    Tool { key: key.into(), label, glyph: None, shown: None, lit: false, warn: false, ghost: false, solid: false, tall: false }
}

impl<'a> Tool<'a> {
    fn icon(mut self, glyph: &'static rune::Icon) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// A word on the control; without one it is an icon alone and `label` is its name.
    fn word(mut self, shown: &'a str) -> Self {
        self.shown = Some(shown);
        self
    }

    fn lit(mut self, lit: bool) -> Self {
        self.lit = lit;
        self
    }

    /// The one answer a card asks for: ink on the page's own light, as the mock's Continue.
    fn solid(mut self) -> Self {
        self.solid = true;
        self
    }

    /// The mock's 32 high control of the card under the slide.
    fn tall(mut self) -> Self {
        self.tall = true;
        self
    }

    fn warn(mut self) -> Self {
        self.warn = true;
        self
    }

    fn quiet(mut self) -> Self {
        self.ghost = true;
        self
    }

    fn build(self, k: &Kit, enabled: bool, press: impl Fn(&mut App) + 'static) -> El {
        let (fill, ink, edge) = if self.warn {
            (theme::warn().alpha(0.18), theme::warn_ink(), Some(theme::warn()))
        } else if self.solid {
            (k.c.text_base, theme::surf().chrome, None)
        } else if self.lit {
            (theme::accent().alpha(0.18), theme::accent(), None)
        } else if self.ghost {
            (Color::TRANSPARENT, k.c.text_base, None)
        } else {
            (k.c.layer_01, k.c.text_base, Some(k.c.border_base))
        };
        let mut e = row().key(self.key).role("button").label(self.label.to_string()).h(if self.tall { 32.0 } else { CTRL }).px(if self.shown.is_some() { 11.0 } else { 6.0 }).gap(space::S2).center().radius(6.0).fill(fill).cursor(Cursor::Pointer).focusable(enabled).on_key(enter_key);
        if let Some(edge) = edge.filter(|_| !self.lit) {
            e = e.border(1.0, edge);
        }
        if let Some(glyph) = self.glyph {
            e = e.child(icon(glyph, 15.0).color(ink).a11y_hidden());
        }
        if let Some(word) = self.shown {
            e = e.child(text(word.to_string()).font(type_::BODY_STRONG).color(ink).a11y_hidden());
        }
        if enabled {
            e.hover(k.c.hover).on_press(move |s: &mut App| press(s))
        } else {
            e.layer_opacity(0.45).disabled(true)
        }
    }
}

fn thin_divider(k: &Kit) -> El {
    el().w(1.0).h(20.0).fill(k.c.border_base).a11y_hidden()
}

/// The strip above the slide: history, the picked box's role and size, bold, italic, alignment and
/// Apply style to. A data part leaves the text tools dim and offers its window instead.
fn strip(s: &App, k: &Kit, form: &Notices, stage: &Stage, width: f32, layer: &mut Vec<El>) -> El {
    let layout = stage.layout.as_ref();
    let picked = form.slide.selected;
    let field = picked.unwrap_or(Field::Title);
    let typed = field.typed();
    let fit: Option<Fit> = layout.and_then(|layout| if field == Field::Body { layout.body } else { layout.title });
    // Size, bold, italic and alignment are set on a typed box of the styles that draw their own boxes.
    let tools = typed && fit.is_some();
    let face = if field == Field::Body { form.body_face } else { form.title_face };
    let undoable = !form.slide.undo.is_empty() || form.slide.editing;
    let mut bar = row().key("notice-strip").w(width).h(STRIP).px(space::S2).gap(space::S2).center_y().radius(10.0).fill(theme::surf().raised).border(1.0, k.c.border_muted);
    bar = bar
        .child(tool("strip-undo", t::BTN_UNDO()).icon(&icons::UNDO_STEP).quiet().build(k, undoable, |s| s.notice_undo()))
        .child(tool("strip-redo", t::EDIT_REDO()).icon(&icons::REDO).quiet().build(k, !form.slide.redo.is_empty(), |s| s.notice_redo()))
        .child(thin_divider(k));
    let (title_name, body_name) = (role_name(Field::Title), role_name(Field::Body));
    let role = role_name(field);
    bar = bar.child(
        tool("strip-role", role)
            .word(role)
            .lit(true)
            .build(k, layout.is_some_and(|layout| layout.of(Field::Body).is_some()), move |s| s.notice_select(Some(if field == Field::Title { Field::Body } else { Field::Title }))),
    );
    let _ = (title_name, body_name);
    bar = bar.child(thin_divider(k)).child(text(t::EDIT_SIZE()).font(type_::BODY).color(k.c.text_muted));
    let shown = fit.map(|fit| format!("{}", fit.size.round() as i32)).unwrap_or_else(|| "-".to_string());
    let stepper = row()
        .gap(2.0)
        .center_y()
        .child(tool("strip-smaller", t::EDIT_SIZE()).icon(&icons::MINUS).build(k, tools && fit.is_some_and(|fit| fit.size > fit.floor), |s| s.notice_size(-1)))
        .child(row().key("strip-size").role("status").label(t::EDIT_SIZE()).w(48.0).h(CTRL).center().radius(6.0).fill(k.c.layer_01).border(1.0, k.c.border_base).child(text(shown).font(type_::BODY).color(if tools { k.c.text_base } else { k.c.text_faint })))
        .child(tool("strip-larger", t::EDIT_SIZE()).icon(&icons::PLUS).build(k, tools && fit.is_some_and(|fit| fit.ceiling < fit.max), |s| s.notice_size(1)));
    bar = bar.child(stepper);
    bar = bar.child(if fit.is_some_and(|fit| fit.at_floor() && !fit.fits) {
        tool("strip-fit", t::EDIT_SMALLEST()).icon(&icons::ALERT).word(t::EDIT_SMALLEST()).warn().build(k, true, |s| s.notice_fit())
    } else {
        tool("strip-fit", t::EDIT_FIT()).icon(&icons::FIT).word(t::EDIT_FIT()).lit(tools).build(k, tools, |s| s.notice_fit())
    });
    bar = bar
        .child(thin_divider(k))
        .child(tool("strip-bold", t::EDIT_BOLD()).icon(&icons::BOLD).lit(tools && face.bold).build(k, tools, |s| s.notice_toggle_bold()))
        .child(tool("strip-italic", t::EDIT_ITALIC()).icon(&icons::ITALIC).lit(tools && face.italic).build(k, tools, |s| s.notice_toggle_italic()))
        .child(thin_divider(k));
    let aligns = row().key("strip-align").role("radiogroup").label(t::EDIT_ALIGN_LEFT()).h(CTRL).gap(0.0).padding(2.0).radius(6.0).fill(k.c.layer_01).border(1.0, k.c.border_base).center_y();
    let place = |key: &str, label: &str, glyph: &'static rune::Icon, align: Face| {
        let on = tools && face.align == align;
        let e = row().key(key.to_string()).role("radio").label(label.to_string()).checked(on).w(28.0).h(22.0).center().radius(5.0).cursor(Cursor::Pointer).focusable(tools).on_key(enter_key).child(icon(glyph, 14.0).color(if on { k.c.text_base } else { k.c.text_muted }).a11y_hidden());
        if tools { e.fill(if on { theme::accent().alpha(0.2) } else { Color::TRANSPARENT }).hover(k.c.hover).on_press(move |s: &mut App| s.notice_align(align)) } else { e.layer_opacity(0.45) }
    };
    bar = bar.child(aligns.child(place("strip-left", t::EDIT_ALIGN_LEFT(), &icons::ALIGN_L, Face::Start)).child(place("strip-centre", t::EDIT_ALIGN_CENTRE(), &icons::ALIGN_C, Face::Centre)).child(place("strip-right", t::EDIT_ALIGN_RIGHT(), &icons::ALIGN_R, Face::End)));
    bar = bar.child(spacer());
    if !typed {
        let tab = source_tab(field);
        bar = bar.child(tool("strip-open", open_label(field)).icon(&icons::EXTERNAL).word(open_label(field)).build(k, true, move |s| s.open_tab(tab)));
    } else {
        let names = [t::TPL_APPLY_SLIDE(), t::TPL_APPLY_ALL(), t::TPL_APPLY_PLAN()];
        let at = match form.slide.apply {
            Target::Slide => 0,
            Target::Every => 1,
            Target::Plan => 2,
        };
        bar = bar.child(text(t::TPL_APPLY()).font(type_::BODY).color(k.c.text_muted)).child(
            Select::new("notice-apply", &names)
                .label(t::TPL_APPLY())
                .appearance(SelectAppearance::Field)
                .width(176.0)
                .current(Some(names[at]))
                .open(form.slide.apply_open)
                .on_toggle(|s: &mut App| {
                    if let Some(form) = s.notices.as_mut() {
                        form.slide.apply_open = !form.slide.apply_open;
                    }
                })
                .on_close(|s: &mut App| {
                    if let Some(form) = s.notices.as_mut() {
                        form.slide.apply_open = false;
                    }
                })
                .on_choose(|s: &mut App, picked: &str, _: &mut Tx| {
                    let target = if picked == t::TPL_APPLY_ALL() { Target::Every } else if picked == t::TPL_APPLY_PLAN() { Target::Plan } else { Target::Slide };
                    s.notice_apply(target);
                })
                .build(k, layer),
        );
    }
    let _ = s;
    bar
}

/// A line of the slide's text, drawn faint under the box where the next slide would start.
fn ghost_text(line: &artosia_slides::warta::Line, like: &TextBox, u: f32) -> El {
    let style = TextStyle::new((line.size * u).max(4.0), line.weight).family(line.family).lh((line.size * u * 1.3).ceil());
    let style = if line.italic { style.italic() } else { style };
    text(line.text.clone()).font(style).color(rgb_of(like.ink).alpha(0.35)).absolute(line.x * u, line.top * u).inert()
}

/// The yellow diagonal hatch the mock draws behind the lines that do not fit.
fn hatch(w: f32, h: f32) -> El {
    let mut field = el().size(w, h).clip_rounded(0.0).a11y_hidden();
    let tall = (w + h) * 1.5;
    for at in 0..((w + h) / 14.0).ceil() as i32 {
        field = field.child(el().absolute(at as f32 * 14.0 - h / 2.0, h / 2.0 - tall / 2.0).size(3.0, tall).fill(theme::warn().alpha(0.35)).rotate(std::f32::consts::FRAC_PI_4));
    }
    field
}

fn handles(w: f32, h: f32) -> Vec<El> {
    let at = [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (0.0, 0.5), (1.0, 0.5), (0.0, 1.0), (0.5, 1.0), (1.0, 1.0)];
    at.iter()
        .enumerate()
        .map(|(n, (fx, fy))| el().key(format!("handle-{n}")).absolute(fx * w - HANDLE, fy * h - HANDLE).size(2.0 * HANDLE, 2.0 * HANDLE).radius(HANDLE).fill(Color::WHITE).border(OUTLINE, theme::accent()).a11y_hidden())
        .collect()
}

/// The small bar over a picked box: Edit, Fit and more, or for a data part Edit and its window.
fn float_bar(k: &Kit, field: Field, b: &TextBox, u: f32, fits_to: Option<Fit>) -> El {
    let part = |key: &str, glyph: &'static rune::Icon, word: &str, press: Box<dyn Fn(&mut App)>| {
        row().key(key.to_string()).role("button").label(word.to_string()).h(30.0).px(space::S2).gap(6.0).center_y().cursor(Cursor::Pointer).focusable(true).on_key(enter_key).hover(k.c.hover).radius(8.0).child(icon(glyph, 14.0).color(k.c.text_base).a11y_hidden()).child(text(word.to_string()).font(type_::SMALL_STRONG).color(k.c.text_base).a11y_hidden()).on_press(move |s: &mut App| press(s))
    };
    let dots = row().key("bar-more").role("button").label(t::EDIT_MORE()).h(30.0).px(space::S2).center().cursor(Cursor::Pointer).focusable(true).on_key(enter_key).hover(k.c.hover).radius(8.0).child(icon(&icons::DOTS, 14.0).color(k.c.text_base).a11y_hidden()).on_press(|s: &mut App| s.notice_more());
    let sep = || el().w(1.0).h(16.0).fill(k.c.border_base).a11y_hidden();
    let mut bar = row().key("slide-bar").h(30.0).radius(8.0).fill(theme::surf().raised).border(1.0, k.c.border_base).center_y().shadow(2.0, 8.0, Color::BLACK.alpha(0.3));
    bar = bar.child(part("bar-edit", &icons::PENCIL, t::BTN_EDIT(), Box::new(move |s| s.notice_begin_edit(field))));
    if field.typed() {
        if fits_to.is_some() {
            bar = bar.child(sep()).child(part("bar-fit", &icons::FIT, t::EDIT_FIT(), Box::new(|s| s.notice_fit())));
        }
        bar = bar.child(sep()).child(dots);
    } else {
        let tab = source_tab(field);
        bar = bar.child(sep()).child(part("bar-source", &icons::EXTERNAL, match tab {
            Tab::Calendar => t::TAB_CALENDAR(),
            Tab::People => t::TAB_PEOPLE(),
            _ => t::TAB_GIVING(),
        }, Box::new(move |s| s.open_tab(tab))));
    }
    let (x, w) = (b.x * u, b.w * u);
    // Six points over the box, and under it when the box is at the slide's top edge.
    let above = b.y * u >= 36.0;
    let y = if above { b.y * u - 36.0 } else { (b.y + b.h) * u + 6.0 };
    row().key("slide-bar-row").absolute(x, y).w(w).justify_end().child(bar)
}

/// "Shrunk to fit" with its meter, under the box's lower right corner.
fn shrunk_pill(fit: Fit, b: &TextBox, u: f32) -> El {
    let share = if fit.ceiling > fit.floor { ((fit.size - fit.floor) / (fit.ceiling - fit.floor)).clamp(0.0, 1.0) } else { 1.0 };
    let accent = theme::accent();
    let meter = el()
        .key("shrunk-meter")
        .w(54.0)
        .h(6.0)
        .radius(3.0)
        .fill(accent.alpha(0.3))
        .child(el().w(54.0 * share).h(6.0).radius(3.0).fill(accent))
        .child(el().absolute(54.0 * share - 5.0, -2.0).size(10.0, 10.0).radius(5.0).fill(Color::WHITE).border(OUTLINE, accent))
        .a11y_hidden();
    let pill = row()
        .key("shrunk-pill")
        .role("status")
        .label(t::EDIT_SHRUNK())
        .h(24.0)
        .px(space::S3)
        .gap(space::S2)
        .center_y()
        .radius(12.0)
        .fill(accent.alpha(0.16))
        .border(1.0, accent)
        .child(icon(&icons::FIT, 15.0).color(accent).a11y_hidden())
        .child(text(t::EDIT_SHRUNK()).font(type_::SMALL_STRONG).color(accent).a11y_hidden())
        .child(meter);
    row().key("shrunk-row").absolute(b.x * u, (b.y + b.h) * u + 3.0).w(b.w * u).justify_end().child(pill)
}

/// The words that sit over a box while the caret is in it: the same face at the same size, on what
/// the slide's words stand on, so the slide's own words under it never show through.
fn edit_field(k: &Kit, field: Field, b: &TextBox, u: f32, form: &Notices) -> El {
    let pad = b.pad;
    let ink = rgb_of(b.ink);
    let ground = b.ground.map_or(theme::surf().chrome, rgb_of);
    let pitch = ((b.h - 2.0 * pad) / b.lines.max(1) as f32 * u).max(8.0);
    let style = TextStyle::new((b.size * u).max(4.0), b.weight).family(b.family).lh(pitch.round()).tracking(b.tracking * u);
    let style = if b.italic { style.italic() } else { style };
    let edit = if field == Field::Title { &form.title } else { &form.body };
    let accessor: fn(&mut App) -> &mut rune::TextEdit = if field == Field::Title { |s| edit_of(s, |form| &mut form.title) } else { |s| edit_of(s, |form| &mut form.body) };
    let face = if field == Field::Title { form.title_face } else { form.body_face };
    // The card's own alignment when it chose one, else the way the slide draws the box, so nothing moves when the caret goes in.
    let drawn = if face.align == Face::Start { b.align } else { face.align };
    let align = match drawn {
        Face::Start => Align::Start,
        Face::Centre => Align::Center,
        Face::End => Align::End,
    };
    let input = text_input(edit, style)
        .key(if field == Field::Title { "slide-title" } else { "slide-body" })
        .label(role_name(field))
        .w((b.w - 2.0 * pad) * u)
        .color(ink)
        .align(align)
        .caret(ink, k.selection())
        .on_edit(move |s: &mut App| accessor(s))
        .on_submit(move |s: &mut App, tx: &mut Tx| {
            if field == Field::Title {
                s.notice_end_edit();
                tx.blur();
            } else if let Some(form) = s.notices.as_mut() {
                form.body.insert("\n");
            }
        })
        .on_input_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| match ev.key {
            Key::Escape => {
                s.notice_end_edit();
                tx.blur();
                true
            }
            Key::Tab => {
                s.notice_end_edit();
                s.notice_walk(!ev.mods.shift);
                tx.blur();
                true
            }
            _ => false,
        });
    el().key("slide-field").absolute(b.x * u, b.y * u).size(b.w * u, b.h * u).fill(ground).padding(pad * u).child(input)
}

/// The slide with what is drawn over it: boxes, the picked one's outline, handles and bar, the pill,
/// and the lines that do not fit.
fn canvas(s: &App, k: &Kit, form: &Notices, stage: &Stage, overflow: Overflow, size: [f32; 2]) -> El {
    let [w, h] = size;
    let u = h / 1080.0;
    let picture = stage.shown.first().cloned();
    let base = el().key("slide-picture").size(w, h).child(super::thumb(s, &s.preview_state(picture), w)).clip_rounded(0.0);
    let mut layer = el().key("slide-stage").role("img").label(t::ANNOUNCEMENT_PREVIEW()).size(w, h).child(base).child(el().key("slide-ground").absolute(0.0, 0.0).size(w, h).on_press(|s: &mut App| s.notice_select(None)).a11y_hidden());
    let Some(layout) = stage.layout.as_ref() else { return layer };
    let (picked, editing, hover) = (form.slide.selected, form.slide.editing, form.slide.hover);
    for b in &layout.boxes {
        let field = b.field;
        let (x, y, bw, bh) = (b.x * u, b.y * u, b.w * u, b.h * u);
        let on = picked == Some(field);
        let mut e = el().key(format!("slide-box-{field:?}")).role("button").label(role_name(field)).absolute(x, y).size(bw, bh).cursor(if on && field.typed() { Cursor::Text } else { Cursor::Pointer }).on_press(move |s: &mut App| s.notice_press_box(field)).on_hover_change(move |s: &mut App, inside: bool, _: &mut Tx| s.notice_hover(field, inside));
        let warn = on && field == Field::Body && overflow == Overflow::Body || on && field == Field::Title && overflow == Overflow::Title;
        if warn {
            e = e.border(OUTLINE, theme::warn()).border_dash(5.0, 4.0);
        } else if on {
            e = e.border(OUTLINE, theme::accent());
        } else if hover == Some(field) {
            e = e.border(1.0, theme::accent().alpha(0.7));
        }
        if on && !editing {
            e = e.children(handles(bw, bh));
        }
        layer = layer.child(e);
        if !field.typed() && on {
            // The label that names what a data part reads, over its top edge.
            let label = row().key("read-label").h(22.0).px(space::S3).gap(6.0).center_y().radius(11.0).fill(theme::accent()).child(icon(&icons::CALENDAR, 13.0).color(Color::WHITE).a11y_hidden()).child(text(source_label(field)).font(type_::SMALL_STRONG).color(Color::WHITE).a11y_hidden());
            layer = layer.child(row().key("read-label-row").absolute(x, y + 12.0).w(bw).justify_center().child(label));
        }
    }
    // A data part wears its label even before it is picked, so a person sees which words are not typed.
    for b in layout.boxes.iter().filter(|b| !b.field.typed() && picked != Some(b.field)) {
        let label = row().key("read-label").h(22.0).px(space::S3).gap(6.0).center_y().radius(11.0).fill(theme::accent().alpha(0.85)).child(text(source_label(b.field)).font(type_::SMALL_STRONG).color(Color::WHITE).a11y_hidden());
        layer = layer.child(row().key(format!("read-label-{:?}", b.field)).absolute(b.x * u, b.y * u + 12.0).w(b.w * u).justify_center().child(label));
    }
    // The part that does not fit: its lines faint under the box, the hatch, and where the next slide starts.
    if let (Some(body), false) = (layout.of(Field::Body), layout.ghost.is_empty()) {
        let (top, last) = (layout.ghost[0].top, layout.ghost[layout.ghost.len() - 1].bottom);
        layer = layer.child(el().key("ghost-hatch").absolute(body.x * u, top * u).size(body.w * u, ((last - top) * u).max(4.0)).child(hatch(body.w * u, ((last - top) * u).max(4.0))));
        for line in &layout.ghost {
            layer = layer.child(ghost_text(line, body, u));
        }
        if let Some(y) = layout.split_y {
            let warn = theme::warn();
            layer = layer
                .child(el().key("split-line").absolute((body.x - 4.0) * u, y * u).size((body.w + 8.0) * u + 36.0, 1.5).fill(warn).a11y_hidden())
                .child(
                    row()
                        .key("split-tag")
                        .absolute((body.x + body.w) * u + 36.0, y * u - 10.0)
                        .h(20.0)
                        .px(space::S2)
                        .center()
                        .radius(5.0)
                        .fill(warn)
                        .child(text(t::EDIT_NEXT_STARTS()).font(type_::SMALL_STRONG).color(Color::BLACK).a11y_hidden()),
                );
        }
    }
    // The shrink pill and the picked box's bar come last, over everything.
    if let Some(field) = picked.filter(|_| editing || true)
        && let Some(b) = layout.of(field)
    {
        let fit = if field == Field::Body { layout.body } else if field == Field::Title { layout.title } else { None };
        if editing && field.typed() {
            layer = layer.child(edit_field(k, field, b, u, form));
        }
        if let Some(fit) = fit.filter(|fit| editing && fit.shrunk() && fit.fits) {
            layer = layer.child(shrunk_pill(fit, b, u));
        }
        if !editing {
            layer = layer.child(float_bar(k, field, b, u, fit));
        }
    }
    layer
}

/// The part of a slide's text a data sheet lists, a line to a row and a tab to a column.
fn data_rows(stage: &Stage) -> Vec<Vec<String>> {
    let Some(slide) = stage.whole.as_ref() else { return Vec::new() };
    artosia_core::deck::notice_body(&slide.text).rows
}

/// The sheet of a data part: the window's own pickers on the left, what it holds on the right.
fn data_sheet(s: &App, k: &Kit, form: &Notices, stage: &Stage, field: Field, width: f32) -> El {
    let tab = source_tab(field);
    let head = row()
        .w_full()
        .h(34.0)
        .px(space::S4)
        .gap(space::S2)
        .center_y()
        .child(text(role_name(field)).font(type_::TITLE).color(k.c.text_base).grow(1.0))
        .child(tool("sheet-open", open_label(field)).icon(&icons::EXTERNAL).word(open_label(field)).build(k, tab != Tab::Plan, move |s| s.open_tab(tab)))
        .child(btn(t::BTN_DONE()).primary().small().focusable().build(k, |s, _| s.notice_done_panel()));
    let template = form.fill.template();
    let mut left = col().key("sheet-pickers").role("group").label(t::ANNOUNCEMENT_FILL()).w((width * 0.5).max(300.0)).gap(space::S3).px(space::S4).py(space::S2);
    let window = stage.card.window;
    match template {
        Template::Schedule => left = left.child(window_panel(s, k, form, window, false)).child(calendar_chips(s, k, form)),
        Template::Birthdays => left = left.child(window_panel(s, k, form, window, true)),
        Template::Passage => left = left.child(passage_field(s, k, form)),
        _ => left = left.children(holds(s, k, Some(&stage.card))),
    }
    let rows = data_rows(stage);
    let mut list = col().key("sheet-rows").grow(1.0).min_w(0.0).gap(2.0).px(space::S4).py(space::S2);
    if form.fill == Fill::Schedule {
        list = list.child(
            row()
                .h(18.0)
                .gap(space::S3)
                .child(text(crate::strings::PEOPLE_DAY()).font(type_::SMALL).color(k.c.text_muted).w(108.0))
                .child(text(t::CAL_START()).font(type_::SMALL).color(k.c.text_muted).w(76.0))
                .child(text(t::CAL_EVENT_NAME()).font(type_::SMALL).color(k.c.text_muted)),
        );
    }
    for (at, cells) in rows.iter().enumerate() {
        let mut line = row().key(format!("sheet-row-{at}")).h(22.0).gap(space::S3).center_y();
        for (column, cell) in cells.iter().enumerate() {
            let fixed = if form.fill == Fill::Schedule { [108.0, 76.0].get(column).copied() } else { None };
            let cell = text(cell.clone()).font(type_::BODY).color(if column == 0 && form.fill == Fill::Schedule { k.c.text_muted } else { k.c.text_base }).truncate();
            line = line.child(match fixed {
                Some(w) => cell.w(w),
                None => cell.min_w(0.0).grow(1.0),
            });
        }
        list = list.child(line);
    }
    let body = row().w_full().grow(1.0).min_h(0.0).child(col().h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(left).scroll_bar(k)).child(vertical_divider(k)).child(col().grow(1.0).min_w(0.0).h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(list).scroll_bar(k));
    col().key("notice-sheet").w(width).h(SHEET).radius(12.0).fill(k.c.layer_02).border(1.0, k.c.border_base).child(head).child(divider(k)).child(body)
}

/// The sheet for what the strip has no room for: the card's template, style, title colour and pictures.
fn more_sheet(s: &App, k: &Kit, form: &Notices, width: f32) -> El {
    // Same body as the other sheets; only its height differs.
    let head = row().w_full().h(34.0).px(space::S4).gap(space::S2).center_y().child(text(t::EDIT_CARD_SETTINGS()).font(type_::TITLE).color(k.c.text_base).grow(1.0)).child(btn(t::BTN_DONE()).primary().small().focusable().build(k, |s, _| s.notice_done_panel()));
    let colours = crate::ui::colour::rows(s, k, ColourFor::NoticeTitle, t::NOTICE_TITLE_COLOUR(), &crate::ui::look::text_colours());
    let left = col().w(width * 0.45).gap(space::S2).px(space::S4).py(space::S2).child(template_chips(s, k, form));
    let right = col().grow(1.0).min_w(0.0).gap(space::S2).px(space::S4).py(space::S2).children(colours).children(picture_rows(k, form));
    let body = row().w_full().grow(1.0).min_h(0.0).child(col().h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(left).scroll_bar(k)).child(vertical_divider(k)).child(col().grow(1.0).min_w(0.0).h_full().min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(theme::size::FADE).child(right).scroll_bar(k));
    col().key("notice-sheet").w(width).h(SHEET_MORE).radius(12.0).fill(k.c.layer_02).border(1.0, k.c.border_base).child(head).child(divider(k)).child(body)
}

/// The words of a style with no boxes of its own, changed in a sheet: its title, and its body when the
/// card holds typed words.
fn words_sheet(s: &App, k: &Kit, form: &Notices, width: f32, missing: (bool, bool)) -> El {
    let head = row().w_full().h(34.0).px(space::S4).gap(space::S2).center_y().child(text(t::EDIT_WORDS()).font(type_::TITLE).color(k.c.text_base).grow(1.0));
    let _ = s;
    let title = col().key("words-title").w_full().gap(space::S1).child(section_label(k, t::FIELD_TITLE())).child(
        rune_kit::TextInput::new(&form.title)
            .width(None)
            .label(t::FIELD_TITLE())
            .placeholder(t::ANNOUNCEMENT_TITLE_HINT())
            .on_edit(|s: &mut App| edit_of(s, |form| &mut form.title))
            .on_key(escape_closes)
            .build(k),
    );
    let mut fields = col().w_full().gap(space::S2).px(space::S4).py(space::S2).children(missing.0.then_some(title));
    if missing.1 {
        fields = fields.child(col().w_full().gap(space::S1).child(section_label(k, t::ANNOUNCEMENT_BODY())).child(
            rune_kit::TextArea::new(&form.body)
                .width(None)
                .label(t::ANNOUNCEMENT_BODY())
                .placeholder(t::ANNOUNCEMENT_BODY_HINT())
                .auto_grow(2, 5)
                .on_edit(|s: &mut App| edit_of(s, |form| &mut form.body))
                .on_key(|s: &mut App, ev: &KeyEvent, tx: &mut Tx| enter_breaks_line(ev, tx) || escape_closes(s, ev, tx))
                .build(k),
        ));
    }
    col().key("notice-sheet").w(width).h(SHEET).radius(12.0).fill(k.c.layer_02).border(1.0, k.c.border_base).child(head).child(divider(k)).child(col().w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(fields).scroll_bar(k))
}

/// The card under the slide when a text does not fit at its smallest size: the offer of a second slide
/// for a body, and only Undo for a title, which never continues.
fn offer_card(k: &Kit, overflow: Overflow, width: f32) -> El {
    let warn = theme::warn();
    let mut actions = row().gap(space::S2).center_y();
    if overflow == Overflow::Body {
        actions = actions.child(
            tool("offer-continue", t::EDIT_CONTINUE())
                .icon(&icons::SPLIT)
                .word(t::EDIT_CONTINUE())
                .solid()
                .tall()
                .build(k, true, |s| s.notice_continue())
                .focus_ring(2.0, 3.0, theme::accent()),
        );
    }
    actions = actions.child(tool("offer-undo", t::BTN_UNDO()).icon(&icons::UNDO_STEP).word(t::BTN_UNDO()).tall().build(k, true, |s| s.notice_undo()));
    if overflow == Overflow::Body {
        actions = actions.child(text(t::EDIT_ADDS_SLIDE()).font(type_::SMALL).color(k.c.text_muted));
    }
    col()
        .key("offer-card")
        .role("alert")
        .w(width)
        .h(OFFER)
        .px(space::S3)
        .py(space::S3)
        .gap(space::S3)
        .radius(12.0)
        .fill(k.c.layer_02)
        .border(1.0, warn)
        .child(row().gap(space::S2).center_y().child(icon(&icons::ALERT, 17.0).color(warn).a11y_hidden()).child(text(t::EDIT_NO_FIT_LONG()).font(type_::TITLE).color(theme::warn_ink())))
        .child(actions)
}

/// What is under the stage: Delete, the seconds, Show now and Save with its two reasons to wait.
fn bottom(s: &App, k: &Kit, form: &Notices, stage: Option<&Stage>, overflow: Overflow, width: f32) -> El {
    let id = form.id;
    let slides = stage.map_or(0, |stage| stage.shown.len());
    let stored = s.announcements.iter().find(|card| card.id == id);
    let unsaved = stage.is_some_and(|stage| stored != Some(&stage.card));
    let at = SECONDS.iter().position(|seconds| *seconds == form.seconds);
    let labels: Vec<String> = SECONDS.iter().map(|seconds| seconds.to_string()).collect();
    let mut bar = row().key("notice-bar").w(width).h(BAR).px(GUTTER).gap(space::S3).center_y().fill(theme::surf().work).border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_muted);
    if id != 0 {
        bar = bar.child(btn(t::BTN_DELETE()).icon(&icons::TRASH).ghost().focusable().build(k, move |s, _| s.delete_announcement(id)));
    }
    bar = bar.child(spacer());
    if overflow != Overflow::None {
        bar = bar.child(row().gap(6.0).center_y().child(icon(&icons::ALERT, 15.0).color(theme::warn_ink()).a11y_hidden()).child(text(t::EDIT_CANT_SAVE()).font(type_::BODY_STRONG).color(theme::warn_ink())));
    } else {
        bar = bar.child(text(t::ANNOUNCEMENT_SECONDS()).font(type_::BODY).color(k.c.text_muted)).child(SegmentedControl::new(&labels, at).label(t::ANNOUNCEMENT_SECONDS()).fit().on_pick(|s: &mut App, picked, _: &mut Tx| s.set_notice_seconds(SECONDS[picked.min(SECONDS.len() - 1)])).build(k));
    }
    if unsaved && overflow == Overflow::None {
        bar = bar.child(row().key("not-saved").role("status").h(22.0).px(space::S2).center().radius(11.0).fill(theme::warn().alpha(0.2)).child(text(t::EDIT_NOT_SAVED()).font(type_::SMALL_STRONG).color(theme::warn_ink())));
    }
    bar.child(btn(t::BTN_SHOW_NOW()).icon(&icons::PLAY).disabled(slides == 0 || overflow != Overflow::None).focusable().build(k, |s, _| s.show_form_notice_now()))
        .child(btn(t::BTN_SAVE()).primary().hint(t::KEY_SAVE()).disabled(overflow != Overflow::None).focusable().build(k, |s, _| s.save_notice()))
}

/// The empty stage of a card with no words: what it is waiting for, and how to give it some.
fn empty(s: &App, k: &Kit, form: &Notices, size: [f32; 2]) -> El {
    let note = match form.fill.template() {
        Template::Plain => t::ANNOUNCEMENT_BODY_HINT(),
        Template::Schedule => t::NOTICE_EMPTY_CALENDAR(),
        Template::Birthdays => t::NOTICE_EMPTY_MEMBERS(),
        Template::Offerings => t::GIVING_EMPTY(),
        Template::Transfer => t::GIVING_ACCOUNT_NONE(),
        Template::Passage => t::PASSAGE_NO_BIBLE(),
    };
    let _ = s;
    el().key("notice-empty").size(size[0], size[1]).radius(k.radius.lg).fill(k.c.layer_02).border(1.0, k.c.border_base).px(space::S4).center().child(text(note).font(type_::BODY).color(k.c.text_muted).wrap().w(size[0] - 2.0 * space::S4))
}

/// Everything right of the rail: the strip, the stage with its sheet or offer, and the bar.
pub fn main(s: &App, k: &Kit, form: &Notices, size: [f32; 2], layer: &mut Vec<El>) -> El {
    let [width, height] = size;
    let stage = s.notice_stage();
    let overflow = stage.as_ref().map_or(Overflow::None, Stage::overflow);
    let panel = form.slide.panel;
    // A part with no box on the slide, because it has no words yet, is typed in the Words sheet instead.
    let missing = stage.as_ref().and_then(|stage| stage.layout.as_ref()).map_or((false, false), |layout| (layout.of(Field::Title).is_none(), form.fill == Fill::Nothing && layout.of(Field::Body).is_none()));
    let stage_h = (height - STRIP - 6.0 - BAR).max(160.0);
    let inner = width - 2.0 * GUTTER;
    // The sheet of a data part, or the card's own, or the words of a style with no boxes of its own.
    // A window with nothing in it says so in the slide's place, and its sheet is open, where the days are changed (C6).
    let blank = stage.as_ref().is_some_and(|stage| stage.shown.is_empty());
    let data = stage.as_ref().and(form.slide.selected).filter(|field| !field.typed()).or_else(|| blank.then(|| part_of(form.fill)).flatten());
    let showing = if panel == Panel::More { Some(Panel::More) } else if data.is_some() && (panel == Panel::Data || blank) { Some(Panel::Data) } else if missing.0 || missing.1 { Some(Panel::None) } else { None };
    let offer = overflow != Overflow::None;
    let slide = slide_size([width, stage_h], showing.map(|shown| if shown == Panel::More { SHEET_MORE } else { SHEET }), offer);
    let mut column = col().key("notice-main").w(width).h_full().min_h(0.0).fill(theme::surf().work);
    if let Some(stage) = stage.as_ref() {
        column = column.child(col().w_full().px(GUTTER).pb(6.0).child(strip(s, k, form, stage, inner, layer)));
    } else {
        column = column.child(el().h(STRIP + 6.0));
    }
    let slide_area = match &stage {
        Some(stage) if !blank => canvas(s, k, form, stage, overflow, slide),
        _ => empty(s, k, form, slide),
    };
    let mut stage_el = col().key("notice-stage").w_full().grow(1.0).min_h(0.0).fill(theme::surf().chrome).items_center().on_press(|s: &mut App| s.notice_select(None));
    stage_el = stage_el.child(el().h(if showing.is_some() { 6.0 } else if offer { 2.0 } else { AROUND })).child(slide_area);
    if offer {
        stage_el = stage_el.child(el().h(12.0)).child(offer_card(k, overflow, 540.0_f32.min(inner)));
    }
    if let Some(shown) = showing {
        stage_el = stage_el.child(el().grow(1.0).min_h(4.0));
        let sheet = match (shown, data) {
            (Panel::More, _) => more_sheet(s, k, form, inner),
            (Panel::Data, Some(field)) => stage.as_ref().map_or_else(el, |stage| data_sheet(s, k, form, stage, field, inner)),
            _ => words_sheet(s, k, form, inner, missing),
        };
        stage_el = stage_el.child(sheet).child(el().h(4.0));
    }
    column.child(stage_el).child(bottom(s, k, form, stage.as_ref(), overflow, width))
}
