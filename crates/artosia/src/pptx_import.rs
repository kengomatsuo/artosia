//! Import a PowerPoint (OVPPTX1): the dialog offers the slides as made (pictures drawn by the
//! installed PowerPoint or LibreOffice, on a worker) or redrawn (their words as announcement
//! cards), and a redraw proposes the birthday table for Members and the money table for
//! Offerings. Nothing is written until the person presses Import.

use std::path::{Path, PathBuf};

use artosia_core::calendar::Date;
use artosia_core::deck::{Announcement, Fill, Person};
use artosia_core::giving::{Fund, Gift, Method};
use artosia_office::{Money, TableKind};

use crate::state::{App, Arrived};
use crate::strings as t;

/// How wide each slide is drawn: a card shows it full screen.
const EXPORT_WIDTH: u32 = 1920;

/// The two ways a file comes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    AsMade,
    Redraw,
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
    /// The cards a redraw makes: title and first line of each.
    pub cards: Vec<(String, String)>,
    pub proposals: Vec<Proposal>,
    pub way: Way,
    pub made: Made,
    /// Which export this dialog waits for; a later one leaves the earlier one's folder to be cleaned.
    pub round: u64,
    /// Ends the export that is drawing, when the person cancels or leaves the dialog.
    pub cancel: artosia_office::Cancel,
}

impl PptxImport {
    /// Whether Import can be pressed now.
    pub fn ready(&self) -> bool {
        match self.way {
            Way::AsMade => matches!(self.made, Made::Ready(_)),
            Way::Redraw => true,
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
        let cards = artosia_office::to_announcements(&deck).into_iter().map(|draft| (draft.title, draft.body.lines().find(|line| !line.trim().is_empty()).unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" "))).collect();
        let slides = deck.slides.iter().filter(|slide| !slide.hidden).count();
        let proposals = self.proposals_of(&deck);
        let round = ROUNDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        let file = path.file_name().map(|name| name.to_string_lossy().to_string()).unwrap_or_default();
        self.pptx = Some(PptxImport { path: path.to_path_buf(), file, slides, cards, proposals, way: Way::AsMade, made: Made::Drawing { done: 0 }, round, cancel: artosia_office::Cancel::new() });
        let cancel = self.pptx.as_ref().map(|import| import.cancel.clone()).unwrap_or_default();
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
                    import.way = Way::Redraw;
                }
                import.made = made;
            }
            _ => {
                let _ = std::fs::remove_dir_all(folder_of(round));
            }
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
            import.way = Way::Redraw;
        }
    }

    pub fn pptx_way(&mut self, way: Way) {
        if let Some(import) = self.pptx.as_mut().filter(|import| way == Way::Redraw || matches!(import.made, Made::Ready(_) | Made::Drawing { .. })) {
            import.way = way;
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
        let counts = match (&import.way, &import.made) {
            (Way::AsMade, Made::Ready(files)) => Some(self.pictures_as_cards(&import.path, files)),
            _ => self.redraw_cards(&import.path),
        };
        if import.way == Way::Redraw {
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
            let mut card = Announcement { seconds: 10, image: kept.to_string_lossy().to_string(), fill: Fill::Nothing, ..Announcement::default() };
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
    /// whose printed total is not its rows' sum) or "none" (no program to draw the slides). All made up.
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
        let cards = titles.iter().map(|title| (title.to_string(), "Made-up line of words".to_string())).collect();
        let gift = |giver: &str, fund: &str, amount: i64| artosia_office::Gift { date: Some((2, 11, 2026)), giver: giver.into(), fund: fund.into(), method: String::new(), amount };
        let money = |printed: i64| Money { gifts: vec![gift("Sample Gamma", "General offering", 200_000), gift("Sample Delta", "Building", 300_000), gift("", "General offering", 150_000)], printed_total: Some(printed), unread: 0 };
        let person = |name: &str, day: u8, month: u8| Person { id: 0, name: name.into(), day, month, year: 0 };
        let people = vec![person("Sample Alpha", 5, 1), person("Sample Beta", 17, 3), person("Sample Epsilon", 9, 11), person("Sample Zeta", 21, 6)];
        let (made, way, proposals) = match kind {
            "money" => (Made::Ready(files), Way::Redraw, vec![Proposal { kind: Kind::Offerings { money: money(700_000), new: 3 }, on: true }]),
            "none" => (Made::NoRenderer, Way::Redraw, vec![Proposal { kind: Kind::Members(people), on: true }, Proposal { kind: Kind::Offerings { money: money(650_000), new: 3 }, on: true }]),
            _ => (Made::Ready(files), Way::AsMade, vec![Proposal { kind: Kind::Members(people), on: true }, Proposal { kind: Kind::Offerings { money: money(650_000), new: 3 }, on: true }]),
        };
        self.pptx = Some(PptxImport { path: PathBuf::from("Bulletin.pptx"), file: "Bulletin.pptx".into(), slides: 8, cards, proposals, way, made, round: 0, cancel: artosia_office::Cancel::new() });
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

    fn wait_for(app: &mut App) {
        for _ in 0..200 {
            let arrived: Vec<Arrived> = app.inbox.lock().map(|mut inbox| std::mem::take(&mut *inbox)).unwrap_or_default();
            if let Some(Arrived::Pptx(round, made)) = arrived.into_iter().next() {
                app.pptx_arrived(round, made);
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
        assert_eq!(import.cards[0], (String::new(), "No Nama Tanggal Lahir".to_string()), "a table slide is named by its first row, cells joined with spaces");
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
        app.pptx_way(Way::Redraw);
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
        app.pptx_way(Way::Redraw);
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
        assert_eq!((import.made.clone(), import.way, import.ready()), (Made::NoRenderer, Way::Redraw, true));
        app.pptx_way(Way::AsMade);
        assert_eq!(app.pptx.as_ref().map(|import| import.way), Some(Way::Redraw), "As made cannot be chosen");
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
        assert_eq!(app.pptx.as_ref().map(|import| (import.made.clone(), import.way)), Some((Made::Stopped, Way::Redraw)));
        wait_for(&mut app);
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
