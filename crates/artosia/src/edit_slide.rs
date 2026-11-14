//! Editing an announcement on its slide (NOTICEEDIT1, `editing-mocks` E1): which box is picked,
//! the caret in it, the strip's controls, the history of committed edits and the second slide.
//! What a box is and whether its words fit is `artosia_slides::warta::layout`; this file decides
//! what a press on one of them changes in the card.

use artosia_core::deck::{Align, Announcement, Face, NoticeStyle, Slide, announcement_pages};
pub use artosia_slides::warta::Field;
use artosia_slides::warta::{self, Layout, Pics};
use rune::TextEdit;

use crate::state::{App, Notices, Panel, Snapshot, Step, Undo};
use crate::strings as t;
use crate::templates::Target;

/// What keeps a card's words from being saved as they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    None,
    /// A title that needs more than two lines at its floor: it never continues on a second slide (D5).
    Title,
    /// A body that does not fit at its floor: it can go on a second slide.
    Body,
}

/// The slide the form would give, and how it is laid out when its style has boxes.
pub struct Stage {
    pub card: Announcement,
    /// The slides the card gives; the first is what the room would see.
    pub shown: Vec<Slide>,
    /// The first slide before a long body is cut, which is what the boxes are worked out on.
    pub whole: Option<Slide>,
    pub style: NoticeStyle,
    pub layout: Option<Layout>,
}

impl Stage {
    /// What keeps these words from being saved as they are.
    pub fn overflow(&self) -> Overflow {
        let Some(layout) = self.layout.as_ref() else { return Overflow::None };
        if layout.title.is_some_and(|fit| !fit.fits) {
            Overflow::Title
        } else if layout.body.is_some_and(|fit| !fit.fits) {
            Overflow::Body
        } else {
            Overflow::None
        }
    }
}

/// The least a title or a body is stepped by, in the 1080 high space of the slide.
const TITLE_STEP: f32 = 4.0;
const BODY_STEP: f32 = 2.0;

fn snapshot_of(form: &Notices) -> Snapshot {
    Snapshot { title: form.title.text.clone(), body: form.body.text.clone(), title_face: form.title_face, body_face: form.body_face }
}

fn put_back(form: &mut Notices, snapshot: &Snapshot) {
    form.title = TextEdit::new(snapshot.title.clone());
    form.body = TextEdit::new(snapshot.body.clone());
    (form.title_face, form.body_face) = (snapshot.title_face, snapshot.body_face);
}

impl App {
    /// The words of the card in the form, as one step of the history.
    fn notice_snapshot(&self) -> Option<Snapshot> {
        self.notices.as_ref().map(snapshot_of)
    }

    /// The pictures a slide shows, as the previews hand them to the drawing.
    fn pics_of(&self, slide: &Slide) -> Pics<'_> {
        let pixels = |path: &String| (!path.is_empty()).then(|| self.slide_pictures.get(path)).flatten().map(|image| image.pixels());
        Pics { photo: pixels(&slide.image), photo2: pixels(&slide.image2), qris: pixels(&slide.qris) }
    }

    /// What the form gives on the stage; None while it has no words.
    pub fn notice_stage(&self) -> Option<Stage> {
        let card = self.notice_draft()?;
        let bible = self.bible().map(|bible| &**bible);
        let whole = announcement_pages(std::slice::from_ref(&card), &self.people, &self.events, &self.giving_data(), &self.today(), bible, self.settings.verses_per_slide).into_iter().next().map(|(_, slide)| slide);
        let style = whole.as_ref().and_then(|slide| slide.style).unwrap_or(self.look().notice_style);
        let typing = self.notices.as_ref().is_some_and(|form| form.slide.editing);
        let layout = whole.as_ref().map(|slide| artosia_slides::notice::layout(slide, &self.look(), &self.pics_of(slide), typing));
        // A key typed into a text that does not fit draws the slide as it is, not cut into the slides of the loop: that is
        // worked out when the caret leaves, so a key costs the layout and one drawing.
        let overfull = layout.as_ref().is_some_and(|layout| layout.body.is_some_and(|fit| !fit.fits));
        let shown = if typing && overfull { whole.iter().cloned().collect() } else { self.card_slides(&card) };
        Some(Stage { card, shown, whole, style, layout })
    }

    /// What keeps the form's words from being saved as they are.
    pub fn notice_overflow(&self) -> Overflow {
        self.notice_stage().map_or(Overflow::None, |stage| stage.overflow())
    }

    /// Puts the card in the form away before another opens or the editor closes: the caret ends, a body
    /// that does not fit goes on a second slide first (D4), and a title too long is kept as it is.
    pub fn settle_notice(&mut self) {
        self.notice_end_edit();
        match self.notice_overflow() {
            Overflow::Body => self.notice_continue(),
            Overflow::Title => self.write_notice(),
            Overflow::None => self.save_notice(),
        }
    }

    /// The toast's Undo of a second slide: the card added goes, and the card is as it was. An editor
    /// still open on it takes its own step back, so its history stays true.
    pub fn undo_continue(&mut self, before: Announcement, added: i64) {
        let open_on_it = self.notices.as_ref().is_some_and(|form| form.id == before.id && matches!(form.slide.undo.last(), Some(Step::Continued { added: own, .. }) if own.id == added));
        if open_on_it {
            self.notice_undo();
            return;
        }
        if self.store.delete_announcement(added).is_ok() {
            self.reload_announcements();
        }
        let _ = self.apply_card(before);
    }

    /// Picks a box, or none. A box from a window opens that window's panel under the slide.
    pub fn notice_select(&mut self, field: Option<Field>) {
        if self.notices.as_ref().is_some_and(|form| form.slide.editing && form.slide.selected != field) {
            self.notice_end_edit();
        }
        let Some(form) = self.notices.as_mut() else { return };
        form.slide.selected = field;
        form.slide.panel = match (field, form.slide.panel) {
            (Some(field), _) if !field.typed() => Panel::Data,
            (_, Panel::Data) => Panel::None,
            (_, panel) => panel,
        };
        form.slide.apply_open = false;
    }

    /// Enter puts the caret in the box that is picked; false when none is, so the key goes on.
    pub fn notice_enter(&mut self) -> bool {
        match self.notices.as_ref().and_then(|form| form.slide.selected).filter(|field| field.typed()) {
            Some(field) if self.notices.as_ref().is_some_and(|form| !form.slide.editing) => {
                self.notice_begin_edit(field);
                true
            }
            _ => false,
        }
    }

    /// Esc backs out one level: the caret, then the picked box and the sheet. False when there is
    /// nothing left to back out of, so the editor itself closes.
    pub fn notice_escape(&mut self) -> bool {
        let Some(form) = self.notices.as_ref() else { return false };
        if form.slide.editing {
            self.notice_end_edit();
            return true;
        }
        if form.slide.selected.is_some() || form.slide.panel != Panel::None || form.slide.apply_open {
            if let Some(form) = self.notices.as_mut() {
                (form.slide.selected, form.slide.panel, form.slide.apply_open) = (None, Panel::None, false);
            }
            return true;
        }
        false
    }

    /// A press on a box: the first picks it, a second on a typed box puts the caret in it.
    pub fn notice_press_box(&mut self, field: Field) {
        let again = self.notices.as_ref().is_some_and(|form| form.slide.selected == Some(field) && !form.slide.editing);
        if again && field.typed() {
            self.notice_begin_edit(field);
        } else {
            self.notice_select(Some(field));
        }
    }

    /// The pointer is over a box, or left it.
    pub fn notice_hover(&mut self, field: Field, inside: bool) {
        if let Some(form) = self.notices.as_mut() {
            form.slide.hover = if inside { Some(field) } else { form.slide.hover.filter(|own| *own != field) };
        }
    }

    /// The small bar's three dots: the card's template, style, title colour and pictures under the slide.
    pub fn notice_more(&mut self) {
        if let Some(form) = self.notices.as_mut() {
            form.slide.panel = if form.slide.panel == Panel::More { Panel::None } else { Panel::More };
        }
    }

    /// Done on a sheet: it folds away and nothing is picked.
    pub fn notice_done_panel(&mut self) {
        self.notice_end_edit();
        if let Some(form) = self.notices.as_mut() {
            form.slide.panel = Panel::None;
            form.slide.selected = None;
        }
    }

    /// The list of cards over the rail, or the rail again.
    pub fn notice_list(&mut self, open: bool) {
        if let Some(form) = self.notices.as_mut() {
            form.slide.list = open;
        }
    }

    /// The caret goes into a typed box; Esc or a press outside ends it.
    pub fn notice_begin_edit(&mut self, field: Field) {
        if !field.typed() {
            self.notice_select(Some(field));
            return;
        }
        self.notice_select(Some(field));
        let Some(form) = self.notices.as_mut() else { return };
        let began = snapshot_of(form);
        let edit = if field == Field::Title { &mut form.title } else { &mut form.body };
        edit.focused = true;
        let end = edit.text.len();
        edit.set_selection(end, end);
        form.slide.editing = true;
        form.slide.began = Some(began);
    }

    /// Ends the caret; what was typed is one step of the history.
    pub fn notice_end_edit(&mut self) {
        let Some(form) = self.notices.as_mut() else { return };
        if !form.slide.editing {
            return;
        }
        form.slide.editing = false;
        form.title.focused = false;
        form.body.focused = false;
        let now = snapshot_of(form);
        if let Some(began) = form.slide.began.take().filter(|began| *began != now) {
            form.slide.undo.push(Step::Words(began));
            form.slide.redo.clear();
        }
    }

    /// Changes the card through `change` as one step of the history, when it changes anything.
    fn notice_change(&mut self, change: impl FnOnce(&mut Notices)) {
        let Some(form) = self.notices.as_mut() else { return };
        let before = snapshot_of(form);
        change(form);
        if snapshot_of(form) != before {
            form.slide.undo.push(Step::Words(before));
            form.slide.redo.clear();
        }
    }

    /// The typed box the strip acts on: the one picked, else the title.
    fn notice_target(&self) -> Field {
        self.notices.as_ref().and_then(|form| form.slide.selected).filter(|field| field.typed()).unwrap_or(Field::Title)
    }

    fn face_mut(form: &mut Notices, field: Field) -> &mut Face {
        if field == Field::Body { &mut form.body_face } else { &mut form.title_face }
    }

    /// The fit of the typed box the strip acts on.
    pub fn notice_fit_of(&self, field: Field) -> Option<warta::Fit> {
        let layout = self.notice_stage()?.layout?;
        if field == Field::Body { layout.body } else { layout.title }
    }

    /// Steps the box's ceiling by `by` steps; the text still shrinks to what fits under it.
    pub fn notice_size(&mut self, by: i32) {
        let field = self.notice_target();
        let Some(fit) = self.notice_fit_of(field) else { return };
        let step = if field == Field::Body { BODY_STEP } else { TITLE_STEP };
        let ceiling = if by < 0 { (fit.size + step * by as f32).max(fit.floor) } else { (fit.ceiling + step * by as f32).min(fit.max) };
        // The style's own ceiling is stored as none, so a later style keeps its own.
        let stored = if ceiling >= fit.max { 0 } else { ceiling as u16 };
        self.notice_change(|form| App::face_mut(form, field).size = stored);
    }

    /// Fit: the style's own ceiling again, so the text is as large as it can be and still fit.
    pub fn notice_fit(&mut self) {
        let field = self.notice_target();
        self.notice_change(|form| App::face_mut(form, field).size = 0);
    }

    pub fn notice_toggle_bold(&mut self) {
        let field = self.notice_target();
        self.notice_change(|form| {
            let face = App::face_mut(form, field);
            face.bold = !face.bold;
        });
    }

    pub fn notice_toggle_italic(&mut self) {
        let field = self.notice_target();
        self.notice_change(|form| {
            let face = App::face_mut(form, field);
            face.italic = !face.italic;
        });
    }

    pub fn notice_align(&mut self, align: Align) {
        let field = self.notice_target();
        self.notice_change(|form| App::face_mut(form, field).align = align);
    }

    /// Picks the box after or before the picked one, among those that take words. After the last
    /// one the pick is let go, so the next Tab reaches the strip's controls.
    pub fn notice_walk(&mut self, forward: bool) -> bool {
        let Some(stage) = self.notice_stage() else { return false };
        let boxes: Vec<Field> = stage.layout.map(|layout| layout.boxes.iter().map(|found| found.field).collect()).unwrap_or_default();
        let at = self.notices.as_ref().and_then(|form| form.slide.selected).and_then(|picked| boxes.iter().position(|field| *field == picked));
        let next = match (at, forward) {
            (None, true) => boxes.first().copied(),
            (None, false) => boxes.last().copied(),
            (Some(at), true) => boxes.get(at + 1).copied(),
            (Some(at), false) => at.checked_sub(1).and_then(|before| boxes.get(before)).copied(),
        };
        self.notice_select(next);
        next.is_some()
    }

    /// Undo: the last committed edit, or the second slide taken away again (N1).
    pub fn notice_undo(&mut self) {
        self.notice_end_edit();
        let Some(step) = self.notices.as_mut().and_then(|form| form.slide.undo.pop()) else { return };
        let Some(now) = self.notice_snapshot() else { return };
        match step {
            Step::Words(before) => {
                if let Some(form) = self.notices.as_mut() {
                    form.slide.redo.push(Step::Words(now));
                    put_back(form, &before);
                }
            }
            Step::Continued { before, after, added } => {
                if self.store.delete_announcement(added.id).is_ok() {
                    self.reload_announcements();
                }
                // The toast that offered this same Undo has nothing left to take back.
                if self.toast.as_ref().is_some_and(|toast| matches!(&toast.undo, Some(Undo::Continued { added: own, .. }) if *own == added.id)) {
                    self.toast = None;
                }
                if let Some(form) = self.notices.as_mut() {
                    form.slide.redo.push(Step::Continued { before: before.clone(), after, added });
                    form.slide.continued = None;
                    put_back(form, &before);
                }
                // What was stored is the part that stayed; the whole is stored again.
                self.write_notice();
            }
        }
    }

    pub fn notice_redo(&mut self) {
        self.notice_end_edit();
        let Some(step) = self.notices.as_mut().and_then(|form| form.slide.redo.pop()) else { return };
        let Some(now) = self.notice_snapshot() else { return };
        match step {
            Step::Words(after) => {
                if let Some(form) = self.notices.as_mut() {
                    form.slide.undo.push(Step::Words(now));
                    put_back(form, &after);
                }
            }
            Step::Continued { before, after, mut added } => {
                added.id = 0;
                if self.store.save_announcement(&mut added).is_ok() {
                    self.place_after_form(added.id);
                }
                if let Some(form) = self.notices.as_mut() {
                    form.slide.undo.push(Step::Continued { before, after: after.clone(), added });
                    put_back(form, &after);
                }
                self.write_notice();
            }
        }
    }

    /// Puts card `id` right after the card in the form.
    fn place_after_form(&mut self, id: i64) {
        let Some(form_id) = self.notices.as_ref().map(|form| form.id) else { return };
        let mut ids: Vec<i64> = self.store.announcements().unwrap_or_default().iter().map(|card| card.id).filter(|own| *own != id).collect();
        let at = ids.iter().position(|own| *own == form_id).map_or(ids.len(), |at| at + 1);
        ids.insert(at, id);
        let _ = self.store.order_announcements(&ids);
        self.reload_announcements();
    }

    /// Continue on a second slide: the body keeps what holds at the floor and the rest goes on a card
    /// right after, with the same title and style, ten seconds long. One write, one Undo (N5).
    pub fn notice_continue(&mut self) {
        let Some(stage) = self.notice_stage() else { return };
        let (Some(whole), Some(before)) = (stage.whole.as_ref(), self.notice_snapshot()) else { return };
        let Some((first, rest)) = warta::cut(whole, stage.style) else { return };
        self.notice_end_edit();
        // The card as it is stored comes first, so the new one has something to stand after.
        let mut kept = stage.card.clone();
        kept.body = first.text.clone();
        let Ok(id) = self.apply_card(kept) else {
            self.say(t::NOTICE_CONTINUE_FAILED(), None);
            return;
        };
        let mut added = Announcement { id: 0, body: rest.text.clone(), seconds: 10, ..stage.card.clone() };
        if self.store.save_announcement(&mut added).is_err() {
            // None of it is kept: the card goes back as it was.
            let mut again = stage.card.clone();
            again.id = id;
            let _ = self.apply_card(again);
            self.say(t::NOTICE_CONTINUE_FAILED(), None);
            return;
        }
        if let Some(form) = self.notices.as_mut() {
            form.id = id;
            form.body = TextEdit::new(first.text.clone());
        }
        self.place_after_form(added.id);
        let Some(after) = self.notice_snapshot() else { return };
        if let Some(form) = self.notices.as_mut() {
            form.slide.undo.push(Step::Continued { before, after, added: Box::new(added.clone()) });
            form.slide.redo.clear();
            form.slide.continued = Some(added.id);
        }
        self.say(t::NOTICE_CONTINUED(), Some(Undo::Continued { before: Box::new(stage.card.clone()), added: added.id }));
    }

    /// "Apply style to": the picked box's setting, and the card's style, on this card, on every card,
    /// or on every card and the plan's look.
    pub fn notice_apply(&mut self, target: Target) {
        let Some(form) = self.notices.as_ref() else { return };
        let (field, style, title_face, body_face) = (self.notice_target(), form.style, form.title_face, form.body_face);
        if let Some(form) = self.notices.as_mut() {
            form.slide.apply = target;
            form.slide.apply_open = false;
        }
        if target == Target::Slide {
            return;
        }
        self.write_notice();
        let ids: Vec<i64> = self.announcements.iter().map(|card| card.id).collect();
        let mut changed = 0;
        for id in ids {
            let Some(mut card) = self.announcements.iter().find(|card| card.id == id).cloned() else { continue };
            if let Some(style) = style {
                card.style = Some(style);
            }
            if field == Field::Title {
                card.title_face = title_face;
            } else {
                card.body_face = body_face;
            }
            if self.store.save_announcement(&mut card).is_ok() {
                changed += 1;
            }
        }
        if target == Target::Plan
            && let Some(style) = style
        {
            self.use_own_look();
            let mut look = self.look();
            look.notice_style = style;
            self.set_look(look);
        }
        self.reload_announcements();
        self.say(if target == Target::Plan { t::TPL_APPLIED_PLAN().to_string() } else { t::style_applied(changed) }, None);
    }
}

impl App {
    /// A plain card in the editorial style with two made-up pictures, opened on its slide as the
    /// mock draws it; `long` gives it a body that does not fit at the smallest size.
    pub fn demo_plain_notice(&mut self, long: bool) {
        let folder = std::env::temp_dir().join("artosia-edit-demo");
        let _ = std::fs::create_dir_all(&folder);
        // Two soft gradients stand for photographs: a warm one with light spots and a dusk.
        let picture = |name: &str, top: [f32; 3], bottom: [f32; 3]| -> String {
            let file = folder.join(name);
            let made = image::RgbaImage::from_fn(480, 320, |x, y| {
                let t = y as f32 / 320.0;
                let glow = ((x as f32 * 0.07).sin() * (y as f32 * 0.09).cos()).max(0.0) * 40.0;
                let at = |c: usize| (top[c] + (bottom[c] - top[c]) * t + glow * (1.0 - t)).clamp(0.0, 255.0) as u8;
                image::Rgba([at(0), at(1), at(2), 255])
            });
            let _ = made.save(&file);
            file.to_string_lossy().to_string()
        };
        let lead = "Sabtu 12 – Minggu 13 Desember 2026, Wisma Bukit Doa, Cipanas";
        let body = if long {
            format!("{lead}
Pendaftaran di meja informasi sampai 6 Desember. Biaya Rp 350.000 per orang, anak di bawah lima tahun gratis. Bawa perlengkapan tidur, alat mandi dan Alkitab sendiri. Bus berangkat Sabtu pukul 06.00 dari halaman gereja dan kembali Minggu sore. Peserta membawa uang saku sendiri dan menjaga ketertiban di perjalanan dan di tempat retret. Kolekte khusus untuk retret dikumpulkan pada ibadah Minggu depan di kotak dekat pintu masuk.")
        } else {
            format!("{lead}
Pendaftaran di meja informasi sampai 6 Desember")
        };
        let mut card = Announcement {
            title: "Retret keluarga jemaat".into(),
            body,
            seconds: 10,
            style: Some(NoticeStyle::Greige),
            image: picture("warm.png", [120.0, 70.0, 40.0], [20.0, 16.0, 14.0]),
            image2: picture("dusk.png", [60.0, 40.0, 120.0], [240.0, 150.0, 90.0]),
            ..Announcement::default()
        };
        let _ = self.store.save_announcement(&mut card);
        self.reload_announcements();
        self.edit_announcements();
        self.pick_notice(card.id);
    }

    /// The same card with the caret in its title and words typed that make it shrink.
    pub fn demo_plain_notice_typing(&mut self) {
        self.demo_plain_notice(false);
        if let Some(form) = self.notices.as_mut() {
            form.title = TextEdit::new("Retret keluarga jemaat Cipanas");
        }
        self.notice_begin_edit(Field::Title);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use artosia_core::deck::Fill;

    fn open(app: &mut App, body: &str) -> i64 {
        let mut card = Announcement { title: "Retret keluarga jemaat".into(), body: body.into(), seconds: 10, style: Some(NoticeStyle::Greige), ..Announcement::default() };
        app.store.save_announcement(&mut card).expect("saved");
        app.reload_announcements();
        app.edit_announcements();
        app.pick_notice(card.id);
        card.id
    }

    const LONG: &str = "Pendaftaran di meja informasi sampai 6 Desember Biaya Rp 350.000 per orang anak di bawah lima tahun gratis Bawa perlengkapan tidur alat mandi dan Alkitab sendiri Bus berangkat Sabtu pukul 06.00 dari halaman gereja dan kembali Minggu sore";

    #[test]
    fn typing_in_a_box_is_one_step_of_the_history_and_undo_puts_the_words_back() {
        let mut app = App::practice();
        open(&mut app, "Sabtu 12 Desember");
        app.notice_begin_edit(Field::Title);
        assert!(app.notices.as_ref().is_some_and(|form| form.slide.editing && form.title.focused));
        app.notices.as_mut().expect("open").title = TextEdit::new("Retret pemuda");
        app.notice_end_edit();
        let form = app.notices.as_ref().expect("open");
        assert_eq!((form.slide.undo.len(), form.title.text.as_str()), (1, "Retret pemuda"));
        app.notice_undo();
        assert_eq!(app.notices.as_ref().map(|form| form.title.text.as_str()), Some("Retret keluarga jemaat"));
        app.notice_redo();
        assert_eq!(app.notices.as_ref().map(|form| form.title.text.as_str()), Some("Retret pemuda"));
        // An edit that changed nothing makes no step.
        app.notice_begin_edit(Field::Body);
        app.notice_end_edit();
        assert_eq!(app.notices.as_ref().map(|form| form.slide.undo.len()), Some(1));
    }

    #[test]
    fn a_data_box_opens_its_panel_and_is_never_typed_in() {
        let mut app = App::practice();
        app.demo_notices("schedule");
        app.notice_begin_edit(Field::Rows);
        let form = app.notices.as_ref().expect("open");
        assert!(!form.slide.editing && form.slide.panel == Panel::Data && form.slide.selected == Some(Field::Rows));
        app.notice_select(Some(Field::Title));
        assert_eq!(app.notices.as_ref().map(|form| form.slide.panel), Some(Panel::None));
    }

    #[test]
    fn the_strip_sets_a_ceiling_bold_italic_and_alignment_each_as_a_step() {
        let mut app = App::practice();
        open(&mut app, "Sabtu 12 Desember");
        app.notice_select(Some(Field::Title));
        let before = app.notice_fit_of(Field::Title).expect("a poster card is fitted");
        app.notice_size(-1);
        let after = app.notice_fit_of(Field::Title).expect("fit");
        assert!(after.size < before.size && after.size >= after.floor, "{before:?} then {after:?}");
        app.notice_toggle_bold();
        app.notice_align(Align::Centre);
        let form = app.notices.as_ref().expect("open");
        assert!(form.title_face.bold && form.title_face.align == Align::Centre && form.title_face.size > 0 && form.slide.undo.len() == 3);
        app.notice_fit();
        assert_eq!(app.notices.as_ref().map(|form| form.title_face.size), Some(0));
        app.notice_undo();
        assert!(app.notices.as_ref().is_some_and(|form| form.title_face.size > 0));
    }

    #[test]
    fn a_body_past_the_floor_blocks_saving_until_it_continues_on_a_second_card() {
        let mut app = App::practice();
        let id = open(&mut app, &std::iter::repeat_n(LONG, 6).collect::<Vec<_>>().join(" "));
        assert_eq!(app.notice_overflow(), Overflow::Body);
        let words = app.notices.as_ref().map(|form| form.body.text.split_whitespace().count()).unwrap_or_default();
        let cards = app.announcements.len();
        // Changing the words does not save a clipped slide.
        app.notices.as_mut().expect("open").title = TextEdit::new("Retret baru");
        app.save_notice();
        assert!(app.announcements.iter().all(|card| card.title != "Retret baru"));
        app.notice_continue();
        assert_eq!(app.announcements.len(), cards + 1);
        assert_eq!(app.notice_overflow(), Overflow::None);
        let at = app.announcements.iter().position(|card| card.id == id).expect("kept");
        let (first, second) = (&app.announcements[at], &app.announcements[at + 1]);
        assert_eq!((first.title.as_str(), second.title.as_str(), second.seconds), ("Retret baru", "Retret baru", 10));
        assert_eq!(first.body.split_whitespace().count() + second.body.split_whitespace().count(), words, "no word is lost");
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        // One Undo takes the new card away and puts the whole body back.
        app.notice_undo();
        assert_eq!(app.announcements.len(), cards);
        assert_eq!(app.notices.as_ref().map(|form| form.body.text.split_whitespace().count()), Some(words));
        app.notice_redo();
        assert_eq!(app.announcements.len(), cards + 1);
    }

    #[test]
    fn leaving_with_a_body_that_does_not_fit_continues_it_and_saves() {
        let mut app = App::practice();
        open(&mut app, &std::iter::repeat_n(LONG, 3).collect::<Vec<_>>().join(" "));
        let cards = app.announcements.len();
        app.close_announcements();
        assert_eq!(app.announcements.len(), cards + 1);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        assert!(app.notices.is_none());
    }

    #[test]
    fn a_title_that_needs_three_lines_is_never_continued() {
        let mut app = App::practice();
        open(&mut app, "Sabtu");
        app.notices.as_mut().expect("open").title = TextEdit::new("Retret keluarga jemaat bersama seluruh pemuda dan anak anak dari semua cabang pelayanan di kota ini dan sekitarnya");
        assert_eq!(app.notice_overflow(), Overflow::Title);
        let cards = app.announcements.len();
        app.notice_continue();
        assert_eq!(app.announcements.len(), cards, "a title does not go on a second slide");
    }

    #[test]
    fn the_console_and_a_phone_write_a_card_through_one_function_and_the_revision_moves() {
        use artosia_remote::{RemoteCard, RemoteState};
        let mut app = App::practice();
        let id = open(&mut app, "Sabtu 12 Desember");
        let rev = |app: &App| {
            let mut state = RemoteState::default();
            app.remote_rows(&mut state);
            state.cards.iter().find(|card: &&RemoteCard| card.id == id).expect("row").rev()
        };
        let opened = rev(&app);
        app.notices.as_mut().expect("open").title = TextEdit::new("Dari konsol");
        app.save_notice();
        assert_ne!(rev(&app), opened, "the console's save moved the revision");
        assert_eq!(app.apply_card_text(id, opened, Some("Dari telepon"), None), Err(crate::phone_edit::Refused::Old), "the phone that opened it before is refused");
        let now = rev(&app);
        app.notices = None;
        assert_eq!(app.apply_card_text(id, now, Some("Dari telepon"), None), Ok(()));
        assert_ne!(rev(&app), now);
        assert_eq!(app.apply_card(Announcement { title: String::new(), fill: Fill::Nothing, ..Announcement::default() }), Err(crate::phone_edit::Refused::Invalid));
    }

    #[test]
    fn a_new_card_starts_from_words_to_replace_and_is_not_kept_until_they_change() {
        let mut app = App::practice();
        let cards = app.announcements.len();
        app.new_notice("", "");
        let form = app.notices.as_ref().expect("open");
        assert!(form.slide.editing && !form.title.text.is_empty() && form.title.selected() == form.title.text);
        app.save_notice();
        assert_eq!(app.announcements.len(), cards, "the template's own words are not a card");
        app.notices.as_mut().expect("open").title = TextEdit::new("Pengumuman baru");
        app.save_notice();
        assert_eq!(app.announcements.len(), cards + 1);
    }

    #[test]
    fn opening_another_card_continues_a_body_that_does_not_fit_and_its_toast_still_undoes_it() {
        let mut app = App::practice();
        let id = open(&mut app, &std::iter::repeat_n(LONG, 6).collect::<Vec<_>>().join(" "));
        let cards = app.announcements.len();
        let other = app.announcements.iter().find(|card| card.id != id).map(|card| card.id).expect("another card");
        app.pick_notice(other);
        assert_eq!(app.announcements.len(), cards + 1, "the long body went on a second card before the editor moved");
        app.undo();
        assert_eq!(app.announcements.len(), cards);
        assert!(app.announcements.iter().any(|card| card.id == id && card.body.split_whitespace().count() > 150), "the card has its whole body back");
    }

    #[test]
    fn a_card_whose_window_is_empty_keeps_its_sheet_open_and_selectable_parts_say_so() {
        let mut app = App::practice();
        app.events.clear();
        app.demo_notices("schedule");
        app.events.clear();
        app.refresh_decks();
        let stage = app.notice_stage().expect("the card has a title");
        assert!(stage.shown.is_empty(), "no events, no slide: the stage says so instead of drawing an empty one");
    }

    #[test]
    fn apply_gives_the_picked_boxs_setting_to_this_card_or_to_every_card() {
        let mut app = App::practice();
        let id = open(&mut app, "Sabtu");
        app.notice_select(Some(Field::Title));
        app.notice_toggle_bold();
        app.notice_apply(Target::Slide);
        app.save_notice();
        let bold = |app: &App| app.announcements.iter().filter(|card| card.title_face.bold).map(|card| card.id).collect::<Vec<_>>();
        assert_eq!(bold(&app), [id]);
        app.notice_apply(Target::Every);
        assert_eq!(bold(&app).len(), app.announcements.len());
    }
}
