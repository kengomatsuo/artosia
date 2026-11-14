//! The Members window's state and what its buttons do (OVPEOPLE1). The church's people live in
//! `App::people`, which the birthday card reads; this holds the filter, the sort, the dialog for one
//! person and the import of a list, and writes the library.

use std::cmp::Ordering;
use std::path::Path;

use artosia_core::calendar::Date;
use artosia_core::deck::Person;
use artosia_core::people::Group;
use artosia_office::{Column, Found, Mapping};
use rune::TextEdit;
use rune_kit::{SortDir, TableState};

use crate::state::{App, Tab, Undo};
use crate::strings as t;

/// The column the table opens sorted by.
pub const FIRST_SORT: &str = "next";
/// Rows of a table file the import dialog shows.
pub const PREVIEW: usize = 5;
/// The oldest year of birth the year list offers, counted back from this year.
pub const YEARS: i32 = 110;

/// The part of the person dialog whose list is open. One at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    None,
    Day,
    Month,
    Year,
}

/// The person in the dialog: a new one (`id` 0) or a saved one being changed.
pub struct PersonForm {
    pub id: i64,
    pub name: TextEdit,
    pub day: u8,
    pub month: u8,
    /// 0 when it is not known.
    pub year: u16,
    pub open: Part,
}

impl PersonForm {
    pub fn ready(&self) -> bool {
        !self.name.text.trim().is_empty()
    }

    fn person(&self) -> Person {
        Person { id: self.id, name: self.name.text.trim().to_string(), day: self.day, month: self.month, year: self.year }
    }

    /// The days the month has; a 29th is allowed while the year is unknown or a leap year.
    pub fn days(&self) -> u8 {
        let year = if self.year == 0 { 2024 } else { i32::from(self.year) };
        Date::month_days(year, self.month)
    }
}

/// A list brought in from a file, waiting for its columns to be confirmed and for Replace or Add.
pub struct Import {
    /// The file's name, for the dialog.
    pub file: String,
    /// The rows of a spreadsheet or csv file, and the role of each column. A Word or PowerPoint
    /// file has none: its tables are read as they are.
    pub table: Option<(Vec<Vec<String>>, Mapping)>,
    /// The people the roles give, and the rows they could not.
    pub found: Found,
    /// Replace the list, else add to it.
    pub replace: bool,
    /// The column whose role list is open.
    pub open: Option<usize>,
}

/// The Members tab and what its dialogs hold.
pub struct Window {
    pub table: TableState,
    /// The people shown, as places in `App::people`, filtered and in the sort's order.
    pub view: Vec<usize>,
    pub search: TextEdit,
    pub month: Option<u8>,
    pub form: Option<PersonForm>,
    pub import: Option<Import>,
    /// The day the view was last ordered on, so a new day re-orders the next birthdays.
    pub on: Date,
}

impl Default for Window {
    fn default() -> Window {
        let mut table = TableState::new().key("people-table");
        table.set_sort(FIRST_SORT, SortDir::Ascending);
        Window { table, view: Vec::new(), search: TextEdit::new(""), month: None, form: None, import: None, on: Date { year: 1970, month: 1, day: 1 } }
    }
}

/// None sorts after everything, whichever way the column goes.
fn known<T: Ord>(a: Option<T>, b: Option<T>, dir: SortDir) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => dir.apply(a.cmp(&b)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn group_rank(group: Group) -> u8 {
    match group {
        Group::Child => 0,
        Group::Youth => 1,
        Group::Adult => 2,
    }
}

/// How two people stand in a column's order.
fn order(column: &str, dir: SortDir, a: &Person, b: &Person, today: Date) -> Ordering {
    let by = match column {
        "name" => dir.apply(a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        "born" => known((a.year != 0).then_some((a.year, a.month, a.day)), (b.year != 0).then_some((b.year, b.month, b.day)), dir),
        "group" => known(a.group(today).map(group_rank), b.group(today).map(group_rank), dir),
        "age" => known(a.age(today), b.age(today), dir),
        _ => dir.apply(a.days_until(today).cmp(&b.days_until(today))),
    };
    by.then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())).then(a.id.cmp(&b.id))
}

/// The same person is not listed twice: name in any letter case, and day and month.
pub(crate) fn same(a: &Person, b: &Person) -> bool {
    a.name.to_lowercase() == b.name.to_lowercase() && (a.day, a.month) == (b.day, b.month)
}

impl App {
    /// Orders and filters the people for the table, and keeps the day it was done on.
    pub fn refresh_members(&mut self) {
        let today = self.civil_today();
        let needle = self.members.search.text.trim().to_lowercase();
        let month = self.members.month;
        let mut view: Vec<usize> = (0..self.people.len()).filter(|at| month.is_none_or(|month| self.people[*at].month == month) && (needle.is_empty() || self.people[*at].name.to_lowercase().contains(&needle))).collect();
        let (column, dir) = self.members.table.sort().map_or((FIRST_SORT.to_string(), SortDir::Ascending), |(column, dir)| (column.to_string(), dir));
        view.sort_by(|a, b| order(&column, dir, &self.people[*a], &self.people[*b], today));
        self.members.view = view;
        self.members.on = today;
    }

    /// Reads the people from the library again, after a write.
    pub(crate) fn reload_people(&mut self) {
        self.people = self.store.people().unwrap_or_default();
        self.refresh_members();
        self.refresh_decks();
    }

    /// The person a row of the table is.
    pub fn person_at(&self, row: usize) -> Option<&Person> {
        self.members.view.get(row).and_then(|at| self.people.get(*at))
    }

    /// A month of the strip: shows that month's people, or all of them when it was the one shown.
    pub fn members_month(&mut self, month: u8) {
        self.members.month = if self.members.month == Some(month) { None } else { Some(month) };
        self.refresh_members();
    }

    pub fn members_clear_month(&mut self) {
        self.members.month = None;
        self.refresh_members();
    }

    /// The announcement that reads the list: the saved card for this month's birthdays, else the week's.
    pub fn birthday_card(&self) -> Option<&artosia_core::deck::Announcement> {
        use artosia_core::deck::Fill;
        [Fill::BirthdaysMonth, Fill::BirthdaysWeek].into_iter().find_map(|fill| self.announcements.iter().find(|card| card.fill == fill))
    }

    /// "Open the card" beside the side panel's slide: the announcement editor on that card.
    pub fn open_birthday_card(&mut self) {
        let card = self.birthday_card().cloned();
        self.open_tab(Tab::Notices);
        if card.is_some() {
            self.notices = Some(App::notices_for(card.as_ref()));
        }
    }

    // ---- One person ----

    /// A new person, born on today's day and month until the pickers say otherwise.
    pub fn new_person(&mut self) {
        let today = self.civil_today();
        let mut name = TextEdit::new("");
        name.focused = true;
        self.members.form = Some(PersonForm { id: 0, name, day: today.day, month: today.month, year: 0, open: Part::None });
        self.close_palette();
    }

    pub fn edit_person(&mut self, id: i64) {
        let Some(person) = self.people.iter().find(|person| person.id == id) else { return };
        let mut name = TextEdit::new(person.name.clone());
        name.focused = true;
        self.members.form = Some(PersonForm { id, name, day: person.day, month: person.month, year: person.year, open: Part::None });
    }

    pub fn close_person(&mut self) {
        self.members.form = None;
    }

    pub fn person_open(&mut self, part: Part) {
        if let Some(form) = self.members.form.as_mut() {
            form.open = if form.open == part { Part::None } else { part };
        }
    }

    pub fn person_close_part(&mut self) {
        if let Some(form) = self.members.form.as_mut() {
            form.open = Part::None;
        }
    }

    pub fn person_day(&mut self, day: u8) {
        if let Some(form) = self.members.form.as_mut() {
            form.day = day.clamp(1, form.days());
            form.open = Part::None;
        }
    }

    /// A month with fewer days brings the day down to its last.
    pub fn person_month(&mut self, month: u8) {
        if let Some(form) = self.members.form.as_mut() {
            form.month = month.clamp(1, 12);
            form.day = form.day.min(form.days());
            form.open = Part::None;
        }
    }

    pub fn person_year(&mut self, year: u16) {
        if let Some(form) = self.members.form.as_mut() {
            form.year = year;
            form.day = form.day.min(form.days());
            form.open = Part::None;
        }
    }

    pub fn save_person(&mut self) {
        let Some(mut person) = self.members.form.as_ref().filter(|form| form.ready()).map(PersonForm::person) else { return };
        if self.store.save_person(&mut person).is_ok() {
            self.members.form = None;
            self.reload_people();
            self.members.table.select(person.id.to_string());
        }
    }

    pub fn delete_person(&mut self, id: i64) {
        let Some(person) = self.people.iter().find(|person| person.id == id).cloned() else { return };
        if self.store.delete_person(id).is_ok() {
            self.members.form = None;
            self.members.table.clear_selection();
            self.reload_people();
            self.say(t::person_deleted(&person.name), Some(Undo::Person(person)));
        }
    }

    pub fn restore_person(&mut self, mut person: Person) {
        person.id = 0;
        if self.store.save_person(&mut person).is_ok() {
            self.reload_people();
        }
    }

    /// Ctrl N on the Members tab, and "Add person" in the search.
    pub fn add_person_here(&mut self) {
        self.open_tab(Tab::People);
        self.new_person();
    }

    // ---- Import ----

    /// Reads a list from a Word, PowerPoint, Excel or csv file; says why when it cannot.
    fn read_people(&mut self, path: &Path) -> Option<Import> {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let rows = match extension.as_str() {
            "xlsx" => artosia_office::xlsx_table(path).map(|table| table.rows),
            "csv" | "txt" => std::fs::read(path).map(|bytes| artosia_office::csv_from_bytes(&bytes).rows).map_err(artosia_office::Error::from),
            _ => {
                let found = match extension.as_str() {
                    "pptx" => artosia_office::Pptx::open(path).map(|deck| artosia_office::from_pptx(&deck)),
                    _ => artosia_office::Docx::open(path).map(|doc| artosia_office::from_docx(&doc)),
                };
                return match found {
                    Ok(found) if !found.people.is_empty() => Some(Import { file, table: None, found, replace: false, open: None }),
                    Ok(_) => {
                        self.say(t::BIRTHDAYS_NONE_FOUND(), None);
                        None
                    }
                    Err(error) => {
                        self.say(t::could_not_read(&error.to_string()), None);
                        None
                    }
                };
            }
        };
        match rows {
            Ok(rows) if !rows.is_empty() => {
                let mapping = artosia_office::guess(&rows);
                let found = artosia_office::read(&rows, &mapping);
                Some(Import { file, table: Some((rows, mapping)), found, replace: false, open: None })
            }
            Ok(_) => {
                self.say(t::BIRTHDAYS_NONE_FOUND(), None);
                None
            }
            Err(error) => {
                self.say(t::could_not_read(&error.to_string()), None);
                None
            }
        }
    }

    /// Opens the Members tab on a list brought in from `path`, to confirm its columns.
    pub fn open_import(&mut self, path: &Path) {
        if let Some(import) = self.read_people(path) {
            self.open_tab(Tab::People);
            self.members.import = Some(import);
        }
    }

    /// `--import-birthdays <file>`: replaces the list at once, as a start from a script wants.
    pub fn import_birthdays(&mut self, path: &Path) {
        if let Some(import) = self.read_people(path) {
            self.members.import = Some(Import { replace: true, ..import });
            self.apply_import();
        }
    }

    pub fn close_import(&mut self) {
        self.members.import = None;
    }

    fn reread(import: &mut Import) {
        if let Some((rows, mapping)) = &import.table {
            import.found = artosia_office::read(rows, mapping);
        }
    }

    pub fn import_open(&mut self, column: usize) {
        if let Some(import) = self.members.import.as_mut() {
            import.open = if import.open == Some(column) { None } else { Some(column) };
        }
    }

    pub fn import_close_part(&mut self) {
        if let Some(import) = self.members.import.as_mut() {
            import.open = None;
        }
    }

    /// Gives a column a role; a role belongs to one column, so another that had it is ignored.
    pub fn import_role(&mut self, column: usize, role: Column) {
        let Some(import) = self.members.import.as_mut() else { return };
        import.open = None;
        if let Some((_, mapping)) = import.table.as_mut().filter(|(_, mapping)| column < mapping.columns.len()) {
            for (at, held) in mapping.columns.iter_mut().enumerate() {
                if at != column && role != Column::Ignore && *held == role {
                    *held = Column::Ignore;
                }
            }
            mapping.columns[column] = role;
        }
        App::reread(import);
    }

    pub fn import_header(&mut self, header: bool) {
        let Some(import) = self.members.import.as_mut() else { return };
        if let Some((_, mapping)) = import.table.as_mut() {
            mapping.header = header;
        }
        App::reread(import);
    }

    pub fn import_replace(&mut self, replace: bool) {
        if let Some(import) = self.members.import.as_mut() {
            import.replace = replace;
        }
    }

    /// Writes what the dialog found: replaces the list (Undo brings the old one back) or adds to it.
    pub fn apply_import(&mut self) {
        let Some(import) = self.members.import.take() else { return };
        let found: Vec<Person> = artosia_office::dedup(import.found.people.clone()).into_iter().map(|person| Person { id: 0, name: person.name, day: person.day, month: person.month, year: person.year }).collect();
        if found.is_empty() {
            self.members.import = Some(import);
            return;
        }
        let written = if import.replace {
            let before = self.people.clone();
            self.store.replace_people(&found).map(|()| Some(Undo::People(before)))
        } else {
            let fresh: Vec<Person> = found.into_iter().filter(|person| !self.people.iter().any(|known| same(known, person))).collect();
            self.store.add_people(&fresh).map(|()| None)
        };
        if let Ok(undo) = written {
            self.members.table.clear_selection();
            self.reload_people();
            self.say(t::birthdays_imported(self.people.len(), import.found.unread), undo);
        } else {
            self.members.import = Some(import);
        }
    }

    pub fn restore_people(&mut self, people: Vec<Person>) {
        if self.store.replace_people(&people).is_ok() {
            self.reload_people();
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn add(app: &mut App, name: &str, day: u8, month: u8, year: u16) -> i64 {
        app.new_person();
        if let Some(form) = app.members.form.as_mut() {
            form.name = TextEdit::new(name);
        }
        app.person_month(month);
        app.person_year(year);
        app.person_day(day);
        app.save_person();
        app.people.iter().find(|person| person.name == name).map(|person| person.id).expect("saved")
    }

    fn names(app: &App) -> Vec<String> {
        app.members.view.iter().map(|at| app.people[*at].name.clone()).collect()
    }

    #[test]
    fn the_practice_library_lists_about_thirty_made_up_names_by_next_birthday() {
        let mut app = App::practice();
        app.refresh_members();
        assert!((28..=32).contains(&app.people.len()), "{} people", app.people.len());
        assert_eq!(app.members.view.len(), app.people.len());
        let today = app.civil_today();
        let days: Vec<u32> = app.members.view.iter().map(|at| app.people[*at].days_until(today)).collect();
        assert!(days.windows(2).all(|pair| pair[0] <= pair[1]), "the soonest birthday first");
        assert_eq!(days[0], 0, "the practice list has a birthday today");
    }

    #[test]
    fn the_table_filters_by_month_and_by_name_and_sorts_by_a_column() {
        let mut app = App::practice();
        add(&mut app, "Zed Filter", 3, 7, 1990);
        add(&mut app, "Abe Filter", 20, 7, 0);
        app.members_month(7);
        assert!(names(&app).iter().all(|name| name.contains("Filter") || app.people.iter().any(|p| &p.name == name && p.month == 7)));
        assert!(names(&app).contains(&"Zed Filter".to_string()));
        app.members.search = TextEdit::new("filter");
        app.refresh_members();
        assert_eq!(names(&app).len(), 2);
        app.members.table.set_sort("name", SortDir::Ascending);
        app.refresh_members();
        assert_eq!(names(&app), ["Abe Filter", "Zed Filter"]);
        app.members.table.set_sort("name", SortDir::Descending);
        app.refresh_members();
        assert_eq!(names(&app), ["Zed Filter", "Abe Filter"]);
        // An unknown year is last in the age column whichever way it goes.
        for dir in [SortDir::Ascending, SortDir::Descending] {
            app.members.table.set_sort("age", dir);
            app.refresh_members();
            assert_eq!(names(&app).last().map(String::as_str), Some("Abe Filter"));
        }
        // The month strip's press again shows everyone.
        app.members_month(7);
        app.members.search = TextEdit::new("");
        app.refresh_members();
        assert_eq!(app.members.view.len(), app.people.len());
    }

    #[test]
    fn the_dialog_adds_edits_and_deletes_with_undo() {
        let mut app = App::practice();
        let before = app.people.len();
        app.new_person();
        app.save_person();
        assert!(app.members.form.is_some() && app.people.len() == before, "a person with no name is not saved");
        let id = add(&mut app, "Dialog Person", 31, 1, 0);
        assert!(app.members.form.is_none() && app.people.len() == before + 1);
        // February has no 31st: the month brings the day down, and a leap year keeps the 29th.
        app.edit_person(id);
        app.person_month(2);
        assert_eq!(app.members.form.as_ref().map(|form| form.day), Some(29), "no year yet, so a leap day is allowed");
        app.person_year(2023);
        assert_eq!(app.members.form.as_ref().map(|form| form.day), Some(28));
        app.person_day(40);
        assert_eq!(app.members.form.as_ref().map(|form| form.day), Some(28));
        app.save_person();
        let saved = app.people.iter().find(|person| person.id == id).expect("kept");
        assert_eq!((saved.day, saved.month, saved.year), (28, 2, 2023));
        app.delete_person(id);
        assert_eq!(app.people.len(), before);
        assert!(app.toast.as_ref().is_some_and(|toast| toast.undo.is_some()));
        app.undo();
        assert!(app.people.iter().any(|person| person.name == "Dialog Person" && person.year == 2023));
        assert!(app.members.view.len() == app.people.len(), "the table follows");
    }

    #[test]
    fn a_new_day_orders_the_next_birthdays_again() {
        let mut app = App::practice();
        app.refresh_members();
        let first = names(&app)[0].clone();
        assert_eq!(app.members.on, app.civil_today());
        app.now += 86_400.0 * 3.0;
        assert_ne!(app.members.on, app.civil_today(), "the view was ordered on the day before");
        app.refresh_members();
        assert_eq!(app.members.on, app.civil_today());
        assert_ne!(names(&app)[0], first, "the person born today is no longer first");
    }

    fn csv(name: &str, text: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("artosia-members-{}-{name}", std::process::id()));
        std::fs::write(&path, text).expect("a temp file");
        path
    }

    #[test]
    fn a_csv_list_is_mapped_by_its_headings_and_adds_without_repeats() {
        let mut app = App::practice();
        let before = app.people.len();
        let path = csv("add.csv", "Nama;Tgl;Bulan;Tahun\nImport Satu;5;Januari;1991\nImport Dua;17;2;\nImport Tiga;31;2;1980\n");
        app.open_import(&path);
        assert_eq!(app.tab, Tab::People);
        let import = app.members.import.as_ref().expect("the dialog is open");
        assert_eq!(import.table.as_ref().map(|(_, mapping)| (mapping.header, mapping.columns.clone())), Some((true, vec![Column::Name, Column::Day, Column::Month, Column::Year])));
        assert_eq!((import.found.people.len(), import.found.unread), (2, 1), "31 February is skipped");
        // Taking the year away from its column leaves those two people with no year.
        app.import_role(3, Column::Ignore);
        assert!(app.members.import.as_ref().is_some_and(|import| import.found.people.iter().all(|person| person.year == 0)));
        app.import_role(3, Column::Year);
        app.apply_import();
        assert!(app.members.import.is_none());
        assert_eq!(app.people.len(), before + 2);
        assert!(app.people.iter().any(|person| person.name == "Import Satu" && person.year == 1991));
        // The same file again adds nobody.
        app.open_import(&path);
        app.apply_import();
        assert_eq!(app.people.len(), before + 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replacing_takes_the_old_list_out_and_undo_brings_it_back() {
        let mut app = App::practice();
        let before = app.people.clone();
        let path = csv("replace.csv", "Name,Birth date\nOnly One,1990-03-04\n");
        app.open_import(&path);
        app.import_replace(true);
        app.apply_import();
        assert_eq!(app.people.iter().map(|p| (p.name.as_str(), p.day, p.month, p.year)).collect::<Vec<_>>(), [("Only One", 4, 3, 1990)]);
        app.undo();
        assert_eq!(app.people.len(), before.len());
        assert!(before.iter().all(|old| app.people.iter().any(|person| person.name == old.name && (person.day, person.month, person.year) == (old.day, old.month, old.year))));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_file_with_nothing_to_read_opens_no_dialog() {
        let mut app = App::practice();
        let path = csv("empty.csv", "");
        app.open_import(&path);
        assert!(app.members.import.is_none());
        assert_eq!(app.toast.as_ref().map(|toast| toast.text.as_str()), Some(t::BIRTHDAYS_NONE_FOUND()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn six_hundred_people_are_ordered_in_a_frame() {
        let mut app = App::practice();
        let many: Vec<Person> = (0..600).map(|n| Person { name: format!("Person {n:03}"), day: (n % 28 + 1) as u8, month: (n % 12 + 1) as u8, year: if n % 7 == 0 { 0 } else { 1940 + (n % 80) as u16 }, ..Person::default() }).collect();
        assert!(app.store.replace_people(&many).is_ok());
        app.people = app.store.people().unwrap_or_default();
        for column in ["name", "born", "next", "group", "age"] {
            app.members.table.set_sort(column, SortDir::Descending);
            // The fastest of five: one run on a busy machine took 34 ms while other builds ran.
            let took = (0..5)
                .map(|_| {
                    let started = std::time::Instant::now();
                    app.refresh_members();
                    started.elapsed()
                })
                .min()
                .unwrap_or_default();
            eprintln!("members: 600 people ordered by {column} in {took:?}");
            assert!(took.as_millis() < 16, "{column} took {took:?}");
        }
        assert_eq!(app.members.view.len(), 600);
    }
}
