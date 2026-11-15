//! Import a PowerPoint (OVPPTX1, PPTXGALLERY1): one gallery offers the slides as made (pictures
//! drawn by the installed PowerPoint or LibreOffice, on a worker) and, after it, each template
//! family that holds every redrawn card; a redraw proposes the birthday table for Members and
//! the money table for Offerings. Nothing is written until the person presses Import.

use std::path::{Path, PathBuf};

use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, Fill, LINES_PER_SLIDE, NoticeStyle, Person, announcement_slide, even_chunks};
use artosia_core::giving::{Fund, Gift, Method};
use artosia_office::{Money, TableKind};
use artosia_slides::warta::{self, BODY_FLOOR, Field, Pics};

use crate::state::{App, Arrived};
use crate::strings as t;

/// How wide each slide is drawn: a card shows it full screen.
const EXPORT_WIDTH: u32 = 1920;

/// The tile picked in the gallery: the file as made, or cards in one template family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    AsMade,
    Redraw(NoticeStyle),
}

/// Where the slides as pictures stand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Made {
    /// The worker is drawing; `done` pictures are in the folder.
    Drawing { done: usize },
    /// One picture per visible slide, in order, in the temp folder.
    Ready(Vec<PathBuf>),
    Stopped,
    NoRenderer,
    Failed,
}

/// What a redraw offers to write besides the cards.
pub enum Kind {
    /// People not in Members yet, repeats left out.
    Members(Vec<Person>),
    /// A money table, and how many of its gifts are not in Offerings yet.
    Offerings { money: Money, new: usize },
}

pub struct Proposal {
    pub kind: Kind,
    pub on: bool,
}

impl Proposal {
    /// How many people or gifts it would add.
    pub fn count(&self) -> usize {
        match &self.kind {
            Kind::Members(people) => people.len(),
            Kind::Offerings { new, .. } => *new,
        }
    }
}

/// The file in the dialog.
pub struct PptxImport {
    pub path: PathBuf,
    pub file: String,
    /// Visible slides.
    pub slides: usize,
    /// The cards a redraw makes, in the default family; a card with a picture holds a marker path.
    pub cards: Vec<Announcement>,
    /// The families that hold every card, the default first; worked out on a worker, empty until it answers.
    pub families: Vec<NoticeStyle>,
    /// The worker has not yet said which families hold the cards.
    pub sorting: bool,
    pub proposals: Vec<Proposal>,
    pub pick: Pick,
    pub made: Made,
    /// Which export this dialog waits for; a later one leaves the earlier one's folder to be cleaned.
    pub round: u64,
    /// Ends the export that is drawing, when the person cancels or leaves the dialog.
    pub cancel: artosia_office::Cancel,
}

impl PptxImport {
    /// The first family that holds the cards, or As made when none does.
    fn redraw_pick(&self) -> Pick {
        self.families.first().map_or(Pick::AsMade, |style| Pick::Redraw(*style))
    }

    /// Whether Import can be pressed now.
    pub fn ready(&self) -> bool {
        match self.pick {
            Pick::AsMade => matches!(self.made, Made::Ready(_)),
            Pick::Redraw(_) => true,
        }
    }
}

/// The export function; a test swaps it for one that draws nothing.
pub type Exporter = fn(&Path, &Path, u32, &artosia_office::Cancel) -> Result<artosia_office::Exported, artosia_office::Error>;

/// The number of the latest export of this process, so no two dialogs share a folder.
static ROUNDS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn folder_of(round: u64) -> PathBuf {
    std::env::temp_dir().join(format!("artosia-pptx-{}-{round}", std::process::id()))
}

/// How many slides the folder holds so far.
fn drawn_in(folder: &Path) -> usize {
    std::fs::read_dir(folder).map_or(0, |entries| entries.flatten().filter(|entry| entry.file_name().to_string_lossy().starts_with("slide-")).filter(|entry| entry.path().extension().is_some_and(|extension| extension == "png")).count())
}

/// The card a slide of the file makes, in `style`; None for a slide with nothing on it.
pub(crate) fn card_of(draft: &artosia_office::Draft, image: String, style: NoticeStyle) -> Option<Announcement> {
    if draft.title.trim().is_empty() && draft.body.trim().is_empty() && image.is_empty() {
        return None;
    }
    Some(Announcement { id: 0, title: draft.title.clone(), body: draft.body.clone(), seconds: 10, image, fill: Fill::Nothing, passage: String::new(), style: Some(style), ..Announcement::new() })
}

/// The slides a card gives, a long list cut into slides of `LINES_PER_SLIDE` rows as the loop does.
fn pages_of(card: &Announcement) -> Vec<artosia_core::deck::Slide> {
    let lines: Vec<&str> = card.body.lines().collect();
    let bodies: Vec<String> = if lines.len() <= LINES_PER_SLIDE { vec![card.body.clone()] } else { even_chunks(&lines, LINES_PER_SLIDE).map(|group| group.join("\n")).collect() };
    bodies
        .into_iter()
        .map(|body| {
            let mut slide = announcement_slide(&Announcement { body, ..card.clone() });
            (slide.style, slide.colourway) = (card.style, card.colourway.clone());
            slide
        })
        .collect()
}

/// Whether `style` holds every card: no card is past its floor beyond the continuation slides the
/// family already makes (`warta::paginate` and `holds`), and a card with a picture is laid out with
/// its picture. The look's own Hairline, Ledger and Band do not continue, so there the words must
/// be set at the poster floor or larger and inside the screen.
pub fn fits_family(cards: &[Announcement], style: NoticeStyle) -> bool {
    cards.iter().all(|card| {
        let card = Announcement { style: Some(style), colourway: String::new(), ..card.clone() };
        pages_of(&card).iter().all(|slide| {
            if style.is_poster() {
                return warta::paginate(slide, style).iter().all(|page| warta::holds(page, style));
            }
            let layout = artosia_slides::notice::layout(slide, &artosia_slides::look::Look::default(), &Pics::default(), false);
            layout.boxes.iter().all(|found| found.x >= 0.0 && found.y >= 0.0 && found.x + found.w <= 1920.5 && found.y + found.h <= 1080.5 && (found.field != Field::Body || found.size >= BODY_FLOOR))
        })
    })
}

/// The families that hold every card, the default first.
fn families_for(cards: &[Announcement]) -> Vec<NoticeStyle> {
    if cards.is_empty() {
        return Vec::new();
    }
    NoticeStyle::ALL.iter().copied().filter(|style| fits_family(cards, *style)).collect()
}

/// The gift as the library would hold it, for telling a repeat from a new one: day, giver, amount.
fn same_gift(known: &Gift, date: Date, giver: &str, amount: i64) -> bool {
    known.date == date && known.giver.to_lowercase() == giver.to_lowercase() && known.amount == amount
}

impl App {
    fn gift_date(&self, date: Option<(u8, u8, u16)>) -> Date {
        let today = self.civil_today();
        match date {
            Some((day, month, year)) => Date { year: if year == 0 { today.year } else { i32::from(year) }, month, day },
            None => today,
        }
    }

    fn is_known_gift(&self, gift: &artosia_office::Gift) -> bool {
        let date = self.gift_date(gift.date);
        self.gifts.iter().any(|known| same_gift(known, date, &gift.giver, gift.amount))
    }

    /// What the tables of a file offer: the people not yet in Members and each money table.
    fn proposals_of(&self, deck: &artosia_office::Pptx) -> Vec<Proposal> {
        let mut found: Vec<artosia_office::Birthday> = Vec::new();
        let mut proposals = Vec::new();
        for mapped in artosia_office::redraw(deck) {
            match mapped.kind {
                TableKind::Birthdays(table) => {
                    found.extend(table.people);
                }
                TableKind::Money(money) => {
                    let new = money.gifts.iter().filter(|gift| !self.is_known_gift(gift)).count();
                    proposals.push(Proposal { kind: Kind::Offerings { money, new }, on: true });
                }
                TableKind::Other => {}
            }
        }
        let people: Vec<Person> = artosia_office::dedup(found)
            .into_iter()
            .map(|person| Person { id: 0, name: person.name, day: person.day, month: person.month, year: person.year })
            .filter(|person| !self.people.iter().any(|known| crate::people::same(known, person)))
            .collect();
        if !people.is_empty() {
            proposals.insert(0, Proposal { kind: Kind::Members(people), on: true });
        }
        proposals
    }

    /// A PowerPoint was chosen: reads it, shows the dialog and starts drawing its slides on a worker.
    pub fn open_pptx_import(&mut self, path: &Path) {
        let deck = match artosia_office::Pptx::open(path) {
            Ok(deck) => deck,
            Err(_) => {
                self.say(t::could_not_read(t::WHY_UNREADABLE()), None);
                return;
            }
        };
        self.save_notice();
        let default = Announcement::new().style.unwrap_or(NoticeStyle::SerifItalic);
        let cards: Vec<Announcement> = artosia_office::to_announcements(&deck).iter().filter_map(|draft| card_of(draft, draft.picture.as_ref().map(|_| "picture".to_string()).unwrap_or_default(), default)).collect();
        let slides = deck.slides.iter().filter(|slide| !slide.hidden).count();
        let proposals = self.proposals_of(&deck);
        let round = ROUNDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let file = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
        self.pptx = Some(PptxImport { path: path.to_path_buf(), file, slides, cards: cards.clone(), families: Vec::new(), sorting: true, proposals, pick: Pick::AsMade, made: Made::Drawing { done: 0 }, round, cancel: artosia_office::Cancel::new() });
        let cancel = self.pptx.as_ref().map(|import| import.cancel.clone()).unwrap_or_default();
        // Which families hold the cards takes a second on a big file, so it is worked out beside the export.
        let (inbox, waker) = (self.inbox.clone(), self.waker.clone());
        std::thread::spawn(move || {
            let families = families_for(&cards);
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::PptxFamilies(round, families));
            }
            if let Some(wake) = &waker {
                wake();
            }
        });
        let (inbox, waker, export, from, folder) = (self.inbox.clone(), self.waker.clone(), self.pptx_exporter, path.to_path_buf(), folder_of(round));
        std::thread::spawn(move || {
            let made = match export(&from, &folder, EXPORT_WIDTH, &cancel) {
                Ok(done) => Made::Ready(done.files),
                Err(artosia_office::Error::NoRenderer) => Made::NoRenderer,
                Err(artosia_office::Error::Cancelled) => Made::Stopped,
                Err(_) => Made::Failed,
            };
            if let Ok(mut inbox) = inbox.lock() {
                inbox.push(Arrived::Pptx(round, made));
            }
            if let Some(wake) = &waker {
                wake();
            }
        });
    }

    /// The worker finished. An answer nobody waits for any more is thrown away with its pictures.
    pub fn pptx_arrived(&mut self, round: u64, made: Made) {
        match self.pptx.as_mut() {
            Some(import) if import.round == round && matches!(import.made, Made::Drawing { .. }) => {
                if !matches!(made, Made::Ready(_)) {
                    import.pick = import.redraw_pick();
                }
                import.made = made;
            }
            _ => {
                let _ = std::fs::remove_dir_all(folder_of(round));
            }
        }
    }

    /// The worker named the families that hold the cards. When As made cannot be had, the first of them is picked.
    pub fn pptx_families_arrived(&mut self, round: u64, families: Vec<NoticeStyle>) {
        let Some(import) = self.pptx.as_mut().filter(|import| import.round == round) else { return };
        import.families = families;
        import.sorting = false;
        if import.pick == Pick::AsMade && !matches!(import.made, Made::Ready(_) | Made::Drawing { .. }) {
            import.pick = import.redraw_pick();
        }
    }

    /// Counts the pictures written so far, for the bar. True when the count moved.
    pub fn watch_pptx(&mut self, wake: &mut f64) -> bool {
        let Some(import) = self.pptx.as_mut() else { return false };
        let Made::Drawing { done } = import.made else { return false };
        *wake = wake.min(0.3);
        let now = drawn_in(&folder_of(import.round)).min(import.slides);
        import.made = Made::Drawing { done: now };
        now != done
    }

    /// The bar's Cancel: the answer is no longer waited for, and Redraw is what is left.
    pub fn stop_pptx_export(&mut self) {
        if let Some(import) = self.pptx.as_mut().filter(|import| matches!(import.made, Made::Drawing { .. })) {
            import.cancel.cancel();
            import.made = Made::Stopped;
            import.pick = import.redraw_pick();
        }
    }

    /// A tile is picked: As made once its pictures are coming, or a family that holds the cards.
    pub fn pptx_pick(&mut self, pick: Pick) {
        let Some(import) = self.pptx.as_mut() else { return };
        let allowed = match pick {
            Pick::AsMade => matches!(import.made, Made::Ready(_) | Made::Drawing { .. }),
            Pick::Redraw(style) => import.families.contains(&style),
        };
        if allowed {
            import.pick = pick;
        }
    }

    pub fn pptx_tick(&mut self, at: usize) {
        if let Some(proposal) = self.pptx.as_mut().and_then(|import| import.proposals.get_mut(at)) {
            proposal.on = !proposal.on;
        }
    }

    /// The dialog is left: what was drawn goes, and a worker still drawing is cleaned up after.
    pub fn close_pptx_import(&mut self) {
        let Some(import) = self.pptx.take() else { return };
        // A worker still drawing is told to stop; the folder is removed when it answers.
        import.cancel.cancel();
        if !matches!(import.made, Made::Drawing { .. }) {
            let _ = std::fs::remove_dir_all(folder_of(import.round));
        }
    }

    /// Writes the choice: the pictures as cards, or the redrawn cards and the ticked proposals.
    pub fn apply_pptx_import(&mut self) {
        let Some(import) = self.pptx.take() else { return };
        if !import.ready() {
            self.pptx = Some(import);
            return;
        }
        let (mut members, mut gifts) = (0, 0);
        let counts = match (&import.pick, &import.made) {
            (Pick::AsMade, Made::Ready(files)) => Some(self.pictures_as_cards(&import.path, files)),
            (Pick::Redraw(style), _) => self.redraw_cards(&import.path, *style),
            _ => None,
        };
        if matches!(import.pick, Pick::Redraw(_)) {
            for proposal in &import.proposals {
                match (&proposal.kind, proposal.on) {
                    (Kind::Members(people), true) => members += self.add_members(people),
                    (Kind::Offerings { money, .. }, true) => gifts += self.add_gifts(money),
                    _ => {}
                }
            }
        }
        let _ = std::fs::remove_dir_all(folder_of(import.round));
        let Some((added, lost)) = counts else { return };
        let mut said = t::pptx_imported(added, lost);
        if members + gifts > 0 {
            said = format!("{said}. {}", t::pptx_also(members, gifts));
        }
        self.say(said, None);
    }

    /// Copies each drawn slide into the library's media folder and makes a picture card of it.
    fn pictures_as_cards(&mut self, path: &Path, files: &[PathBuf]) -> (usize, usize) {
        self.save_notice();
        let folder = self.media_folder();
        let stem = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_else(|| "slides".to_string());
        // Each import has its own names, so cards of an earlier import keep their pictures.
        let stamp = crate::state::unix_now() as u64;
        let (mut added, mut lost) = (0, 0);
        for (at, file) in files.iter().enumerate() {
            let kept = folder.join(format!("{stem}-{stamp}-{:03}.png", at + 1));
            if std::fs::create_dir_all(&folder).is_err() || std::fs::copy(file, &kept).is_err() {
                lost += 1;
                continue;
            }
            let mut card = Announcement { seconds: 10, image: kept.to_string_lossy().to_string(), fill: Fill::Nothing, ..Announcement::new() };
            if self.store.save_announcement(&mut card).is_ok() {
                added += 1;
            }
        }
        self.show_new_cards();
        (added, lost)
    }

    /// The list is read again, and an open editor moves to the last card.
    pub(crate) fn show_new_cards(&mut self) {
        self.reload_announcements();
        if self.notices.is_some() {
            self.notices = Some(App::notices_for(self.announcements.last()));
        }
    }

    /// People not in Members yet; how many were added.
    fn add_members(&mut self, people: &[Person]) -> usize {
        let fresh: Vec<Person> = people.iter().filter(|person| !self.people.iter().any(|known| crate::people::same(known, person))).cloned().collect();
        if self.store.add_people(&fresh).is_ok() {
            self.members.table.clear_selection();
            self.reload_people();
            return fresh.len();
        }
        0
    }

    /// The fund a table's word names: one the church has, the first fund for no word, else a new one.
    fn fund_named(&mut self, name: &str) -> i64 {
        let name = name.trim();
        let found = if name.is_empty() { self.funds.first() } else { self.funds.iter().find(|fund| fund.name.to_lowercase() == name.to_lowercase()) };
        if let Some(fund) = found {
            return fund.id;
        }
        let mut fund = Fund { id: 0, name: name.to_string(), colour: crate::calendar::next_colour(self.funds.len()) };
        if self.store.save_fund(&mut fund).is_ok() {
            self.funds.push(fund.clone());
        }
        fund.id
    }

    /// The gifts of a money table that Offerings does not hold yet; how many were added.
    fn add_gifts(&mut self, money: &Money) -> usize {
        let mut added = 0;
        let fresh: Vec<artosia_office::Gift> = money.gifts.iter().filter(|gift| !self.is_known_gift(gift)).cloned().collect();
        for gift in fresh {
            let lower = gift.method.to_lowercase();
            let method = if ["transfer", "tf", "bank"].iter().any(|word| lower.contains(word)) { Method::Transfer } else { Method::Cash };
            let mut kept = Gift { id: 0, date: self.gift_date(gift.date), giver: gift.giver.clone(), fund: self.fund_named(&gift.fund), method, amount: gift.amount };
            if self.store.save_gift(&mut kept).is_ok() {
                added += 1;
            }
        }
        if added > 0 {
            self.reload_gifts();
        }
        added
    }
}

impl App {
    /// Fills the dialog for a headless render: "made" (pictures and both proposals), "money" (a table
    /// whose printed total is not its rows' sum), "none" (no program to draw the slides), "drawing"
    /// (the pictures not there yet) or "long" (a table some families cannot hold). All made up.
    pub fn demo_pptx(&mut self, kind: &str) {
        let folder = std::env::temp_dir().join("artosia-pptx-demo");
        let _ = std::fs::create_dir_all(&folder);
        let files: Vec<PathBuf> = (1..=8u32)
            .map(|n| {
                let file = folder.join(format!("slide-{n:03}.png"));
                let tint = |at: u32| (40 + n * 20 + at * 30).min(255) as u8;
                let _ = image::RgbaImage::from_fn(320, 180, |x, y| image::Rgba([tint(x / 107), tint(y / 60), 150, 255])).save(&file);
                file
            })
            .collect();
        let titles = ["Sunday times", "Youth night", "Birthdays", "Offering report", "Prayer meeting", "Thank you", "Next week", "Church picnic"];
        let mut cards: Vec<Announcement> = titles.iter().map(|title| Announcement { title: title.to_string(), body: "Made-up line of words\nA second made-up line".to_string(), seconds: 10, ..Announcement::new() }).collect();
        if kind == "long" {
            let rows: Vec<String> = (1..=10).map(|n| format!("Sample giver number {n} with a long made-up name\tGeneral offering for the building fund\t{n}00.000")).collect();
            cards.push(Announcement { title: "Offering report".into(), body: rows.join("\n"), seconds: 10, ..Announcement::new() });
        }
        let families = families_for(&cards);
        let redraw = Pick::Redraw(families.first().copied().unwrap_or(NoticeStyle::SerifItalic));
        let gift = |giver: &str, fund: &str, amount: i64| artosia_office::Gift { date: Some((2, 11, 2026)), giver: giver.into(), fund: fund.into(), method: String::new(), amount };
        let money = |printed: i64| Money { gifts: vec![gift("Sample Gamma", "General offering", 200_000), gift("Sample Delta", "Building", 300_000), gift("", "General offering", 150_000)], printed_total: Some(printed), unread: 0 };
        let person = |name: &str, day: u8, month: u8| Person { id: 0, name: name.into(), day, month, year: 0 };
        let people = vec![person("Sample Alpha", 5, 1), person("Sample Beta", 17, 3), person("Sample Epsilon", 9, 11), person("Sample Zeta", 21, 6)];
        let (made, pick, proposals) = match kind {
            "money" => (Made::Ready(files), redraw, vec![Proposal { kind: Kind::Offerings { money: money(700_000), new: 3 }, on: true }]),
            "drawing" => (Made::Drawing { done: 3 }, Pick::AsMade, Vec::new()),
            "none" => (Made::NoRenderer, redraw, vec![Proposal { kind: Kind::Members(people), on: true }, Proposal { kind: Kind::Offerings { money: money(650_000), new: 3 }, on: true }]),
            "long" => (Made::Ready(files), redraw, Vec::new()),
            _ => (Made::Ready(files), Pick::AsMade, vec![Proposal { kind: Kind::Members(people), on: true }, Proposal { kind: Kind::Offerings { money: money(650_000), new: 3 }, on: true }]),
        };
        self.pptx = Some(PptxImport { path: PathBuf::from("Bulletin.pptx"), file: "Bulletin.pptx".into(), slides: 8, cards, families, sorting: false, proposals, pick, made, round: 0, cancel: artosia_office::Cancel::new() });
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A one-slide-per-table deck with made-up names and gifts, built as the office crate's own tests do.
    fn deck(path: &Path, total: &str) {
        let cell = |text: &str| format!("<a:tc><a:txBody><a:p><a:r><a:t>{text}</a:t></a:r></a:p></a:txBody></a:tc>");
        let row = |cells: &[&str]| format!("<a:tr>{}</a:tr>", cells.iter().map(|text| cell(text)).collect::<String>());
        let table = |rows: String| format!(r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="t"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="8000000" cy="1500000"/></p:xfrm><a:graphic><a:graphicData><a:tbl>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#);
        let slide = |tree: String| format!(r#"<?xml version="1.0"?><p:sld xmlns:a="a" xmlns:p="p" xmlns:r="r"><p:cSld><p:spTree>{tree}</p:spTree></p:cSld></p:sld>"#);
        let birthdays = slide(table([row(&["No", "Nama", "Tanggal Lahir"]), row(&["1", "Sample Alpha", "5 Januari"]), row(&["2", "Sample Beta", "17 Maret"])].concat()));
        let money = slide(table([row(&["Nama", "Persembahan", "Jumlah"]), row(&["Sample Gamma", "Persembahan Umum", "Rp 200.000"]), row(&["Sample Delta", "Pembangunan", "Rp 300.000"]), row(&["Total", "", total])].concat()));
        let presentation = r#"<?xml version="1.0"?><p:presentation xmlns:p="p" xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId1"/><p:sldId id="257" r:id="rId2"/></p:sldIdLst><p:sldSz cx="12192000" cy="6858000"/></p:presentation>"#;
        let head = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#;
        let rel = |id: &str, kind: &str, target: &str| format!(r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}" Target="{target}"/>"#);
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).expect("file"));
        let options = zip::write::SimpleFileOptions::default();
        let parts: [(&str, String); 5] = [
            ("_rels/.rels", format!("{head}{}</Relationships>", rel("rId1", "officeDocument", "ppt/presentation.xml"))),
            ("ppt/presentation.xml", presentation.to_string()),
            ("ppt/_rels/presentation.xml.rels", format!("{head}{}{}</Relationships>", rel("rId1", "slide", "slides/slide1.xml"), rel("rId2", "slide", "slides/slide2.xml"))),
            ("ppt/slides/slide1.xml", birthdays),
            ("ppt/slides/slide2.xml", money),
        ];
        for (name, body) in parts {
            writer.start_file(name, options).expect("part");
            writer.write_all(body.as_bytes()).expect("bytes");
        }
        writer.finish().expect("zip");
    }

    /// Draws a flat grey picture per slide, as a renderer would, and counts none of the real programs.
    fn fake_export(_: &Path, out: &Path, _: u32, _: &artosia_office::Cancel) -> Result<artosia_office::Exported, artosia_office::Error> {
        std::fs::create_dir_all(out)?;
        let files: Vec<PathBuf> = (1..=2)
            .map(|n| {
                let file = out.join(format!("slide-{n:03}.png"));
                image::RgbaImage::from_pixel(32, 18, image::Rgba([40 * n as u8, 90, 160, 255])).save(&file).expect("png");
                file
            })
            .collect();
        Ok(artosia_office::Exported { tool: artosia_office::Tool::PowerPoint, files })
    }

    fn no_renderer(_: &Path, _: &Path, _: u32, _: &artosia_office::Cancel) -> Result<artosia_office::Exported, artosia_office::Error> {
        Err(artosia_office::Error::NoRenderer)
    }

    /// Waits for both answers of an opened dialog: the pictures and the families that hold the cards.
    fn wait_for(app: &mut App) {
        let (mut made, mut sorted) = (false, false);
        for _ in 0..500 {
            let arrived: Vec<Arrived> = app.inbox.lock().map(|mut inbox| std::mem::take(&mut *inbox)).unwrap_or_default();
            for item in arrived {
                match item {
                    Arrived::Pptx(round, answer) => {
                        app.pptx_arrived(round, answer);
                        made = true;
                    }
                    Arrived::PptxFamilies(round, families) => {
                        app.pptx_families_arrived(round, families);
                        sorted = true;
                    }
                    _ => {}
                }
            }
            if made && sorted {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("the export did not answer");
    }

    fn file(name: &str, total: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("artosia-pptx-test-{}-{name}.pptx", std::process::id()));
        deck(&path, total);
        path
    }

    #[test]
    fn a_file_is_read_into_proposals_and_nothing_is_written_until_import() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("proposals", "Rp 500.000");
        let (people, gifts, cards) = (app.people.len(), app.gifts.len(), app.announcements.len());
        app.open_pptx_import(&path);
        wait_for(&mut app);
        let import = app.pptx.as_ref().expect("the dialog is open");
        assert_eq!((import.slides, import.cards.len()), (2, 2));
        assert_eq!((import.cards[0].title.as_str(), import.cards[0].body.lines().next()), ("", Some("No\tNama\tTanggal Lahir")), "a table slide is one row per line, cells joined by a tab");
        assert_eq!(import.families.first(), Some(&NoticeStyle::SerifItalic), "the default family comes first");
        assert!(matches!(&import.made, Made::Ready(files) if files.len() == 2));
        let counts: Vec<usize> = import.proposals.iter().map(Proposal::count).collect();
        assert_eq!(counts, vec![2, 2], "two people for Members, two gifts for Offerings");
        assert!(matches!(&import.proposals[1].kind, Kind::Offerings { money, .. } if money.total_matches() == Some(true)));
        assert_eq!((app.people.len(), app.gifts.len(), app.announcements.len()), (people, gifts, cards), "nothing is written before Import");
        app.close_pptx_import();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_money_table_whose_total_differs_is_marked() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("differs", "Rp 450.000");
        app.open_pptx_import(&path);
        let import = app.pptx.as_ref().expect("open");
        let Kind::Offerings { money, .. } = &import.proposals[1].kind else { panic!("a money table") };
        assert_eq!((money.total_matches(), money.printed_total, money.sum()), (Some(false), Some(450_000), 500_000));
        wait_for(&mut app);
        app.close_pptx_import();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn as_made_copies_each_picture_into_the_library_in_order_and_cleans_the_temp_folder() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("made", "Rp 500.000");
        let before = app.announcements.len();
        app.open_pptx_import(&path);
        wait_for(&mut app);
        let folder = folder_of(app.pptx.as_ref().expect("open").round);
        assert!(folder.is_dir());
        app.apply_pptx_import();
        assert!(app.pptx.is_none());
        assert!(!folder.exists(), "the temp folder is cleaned");
        let made: Vec<&Announcement> = app.announcements.iter().skip(before).collect();
        assert_eq!(made.len(), 2);
        for card in &made {
            assert!(Path::new(&card.image).is_file() && !Path::new(&card.image).starts_with(std::env::temp_dir().join(format!("artosia-pptx-{}", std::process::id()))));
            assert!(card.title.is_empty() && card.body.is_empty());
        }
        assert!(made[0].image.ends_with("-001.png") && made[1].image.ends_with("-002.png"));
        for card in &made {
            let _ = std::fs::remove_file(&card.image);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_redraw_adds_the_ticked_people_and_gifts_once_and_leaves_repeats_out() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("redraw", "Rp 500.000");
        let (people, gifts, cards) = (app.people.len(), app.gifts.len(), app.announcements.len());
        app.open_pptx_import(&path);
        wait_for(&mut app);
        app.pptx_pick(Pick::Redraw(NoticeStyle::SerifItalic));
        app.apply_pptx_import();
        assert_eq!((app.people.len(), app.gifts.len(), app.announcements.len()), (people + 2, gifts + 2, cards + 2));
        assert!(app.funds.iter().any(|fund| fund.name == "Pembangunan"), "an unknown fund is made");
        // The same file again offers nothing that is already there.
        app.open_pptx_import(&path);
        let counts: Vec<usize> = app.pptx.as_ref().expect("open").proposals.iter().map(Proposal::count).collect();
        assert_eq!(counts, vec![0], "only the money table is listed, with none new");
        wait_for(&mut app);
        app.close_pptx_import();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn an_unticked_proposal_is_not_written() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("unticked", "Rp 500.000");
        let (people, gifts) = (app.people.len(), app.gifts.len());
        app.open_pptx_import(&path);
        wait_for(&mut app);
        app.pptx_pick(Pick::Redraw(NoticeStyle::SerifItalic));
        app.pptx_tick(0);
        app.apply_pptx_import();
        assert_eq!((app.people.len(), app.gifts.len()), (people, gifts + 2));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn with_no_renderer_as_made_is_unavailable_and_redraw_stays() {
        let mut app = App::practice();
        app.pptx_exporter = no_renderer;
        let path = file("none", "Rp 500.000");
        app.open_pptx_import(&path);
        wait_for(&mut app);
        let import = app.pptx.as_ref().expect("open");
        assert_eq!((import.made.clone(), import.pick, import.ready()), (Made::NoRenderer, Pick::Redraw(NoticeStyle::SerifItalic), true));
        app.pptx_pick(Pick::AsMade);
        assert_eq!(app.pptx.as_ref().map(|import| import.pick), Some(Pick::Redraw(NoticeStyle::SerifItalic)), "As made cannot be chosen");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_stopped_export_is_cleaned_when_it_answers_late() {
        let mut app = App::practice();
        app.pptx_exporter = fake_export;
        let path = file("stopped", "Rp 500.000");
        app.open_pptx_import(&path);
        let round = app.pptx.as_ref().expect("open").round;
        app.stop_pptx_export();
        assert_eq!(app.pptx.as_ref().map(|import| (import.made.clone(), import.sorting)), Some((Made::Stopped, true)));
        wait_for(&mut app);
        assert_eq!(app.pptx.as_ref().map(|import| import.pick), Some(Pick::Redraw(NoticeStyle::SerifItalic)), "with no pictures the first family that holds the cards is picked");
        assert!(!folder_of(round).exists(), "the late pictures are removed");
        assert_eq!(app.pptx.as_ref().map(|import| import.made.clone()), Some(Made::Stopped));
        let _ = std::fs::remove_file(path);
    }

    /// An export that draws until it is told to stop, as a slow PowerPoint does.
    fn slow_export(_: &Path, _: &Path, _: u32, cancel: &artosia_office::Cancel) -> Result<artosia_office::Exported, artosia_office::Error> {
        for _ in 0..500 {
            if cancel.is_cancelled() {
                return Err(artosia_office::Error::Cancelled);
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(artosia_office::Error::Export("never told to stop".into()))
    }

    #[test]
    fn cancel_and_leaving_the_dialog_both_signal_the_export() {
        let mut app = App::practice();
        app.pptx_exporter = slow_export;
        let path = file("signal", "Rp 500.000");
        app.open_pptx_import(&path);
        let cancel = app.pptx.as_ref().expect("open").cancel.clone();
        assert!(!cancel.is_cancelled());
        app.stop_pptx_export();
        assert!(cancel.is_cancelled(), "the bar's Cancel reaches the export");
        let start = std::time::Instant::now();
        wait_for(&mut app);
        assert!(start.elapsed() < std::time::Duration::from_secs(2), "the export ended at once");
        assert_eq!(app.pptx.as_ref().map(|import| import.made.clone()), Some(Made::Stopped));
        app.open_pptx_import(&path);
        let second = app.pptx.as_ref().expect("open").cancel.clone();
        app.close_pptx_import();
        assert!(second.is_cancelled(), "leaving the dialog reaches it too");
        wait_for(&mut app);
        let _ = std::fs::remove_file(path);
    }

    /// A card of `rows` table rows, `width` characters in each of its two cells.
    fn table_card(rows: usize, width: usize) -> Announcement {
        let body = (0..rows).map(|row| format!("{}\t{}", "Aaaa bbbb ".repeat(width / 10 + 1).trim_end().chars().take(width).collect::<String>(), row)).collect::<Vec<_>>().join("\n");
        Announcement { title: "Report".into(), body, seconds: 10, fill: Fill::Nothing, ..Announcement::new() }
    }

    #[test]
    fn a_family_is_offered_only_when_every_card_holds_in_it() {
        let short = [Announcement { title: "Picnic".into(), body: "On the field\nAfter the service".into(), seconds: 10, ..Announcement::new() }];
        let offered: Vec<NoticeStyle> = NoticeStyle::ALL.iter().copied().filter(|style| fits_family(&short, *style)).collect();
        assert_eq!(offered.first(), Some(&NoticeStyle::SerifItalic), "the default family is first");
        assert!(offered.len() >= 10, "a short card holds in most families: {offered:?}");
        // A table of very long cells is past the floor in some family and not in another.
        let long = [table_card(10, 160)];
        let held: Vec<NoticeStyle> = NoticeStyle::ALL.iter().copied().filter(|style| fits_family(&long, *style)).collect();
        println!("long table holds in {} of {}: {held:?}", held.len(), NoticeStyle::ALL.len());
        assert!(!held.is_empty() && held.len() < NoticeStyle::ALL.len(), "some family is left out and some remain: {held:?}");
        assert!(held.len() < offered.len(), "the long table narrows the offer");
        // One card that cannot hold takes the family out for the whole file.
        let both = [short[0].clone(), long[0].clone()];
        assert!(NoticeStyle::ALL.iter().all(|style| fits_family(&both, *style) == (fits_family(&short, *style) && fits_family(&long, *style))));
        // A card with a picture goes through the same check with its picture laid out.
        let picture = [Announcement { image: "picture".into(), seconds: 10, ..Announcement::new() }];
        assert!(NoticeStyle::ALL.iter().any(|style| fits_family(&picture, *style)));
    }

    #[test]
    fn working_out_the_offer_for_a_big_file_costs_what_is_measured() {
        let cards: Vec<Announcement> = (0..30).map(|at| Announcement { title: format!("Card {at}"), body: "A made-up line of words\nAnother made-up line".into(), seconds: 10, ..Announcement::new() }).collect();
        let start = std::time::Instant::now();
        let offered = families_for(&cards);
        let spent = start.elapsed();
        println!("offer for 30 cards: {spent:?}, {} families", offered.len());
        assert!(spent < std::time::Duration::from_secs(3), "{spent:?}");
    }

    #[test]
    fn counting_the_pictures_written_costs_microseconds() {
        let folder = std::env::temp_dir().join(format!("artosia-pptx-count-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("folder");
        for n in 0..40 {
            std::fs::write(folder.join(format!("slide-{n:03}.png")), b"x").expect("file");
        }
        let start = std::time::Instant::now();
        for _ in 0..100 {
            assert_eq!(drawn_in(&folder), 40);
        }
        let each = start.elapsed() / 100;
        println!("count of 40 slides: {each:?}");
        assert!(each < std::time::Duration::from_millis(2), "{each:?}");
        let _ = std::fs::remove_dir_all(folder);
    }
}
