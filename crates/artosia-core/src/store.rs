//! The library on disk: one SQLite file holding songs, plans, announcements and settings.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::calendar::{Calendar, Change, Date, Event, Override, Repeat};
use crate::deck::{Announcement, Fill, NoticeStyle, Person};
use crate::window::{Range, Window};
use crate::giving::{Account, Fund, Gift, Method};
use crate::plan::Plan;
use crate::song::{Song, Translation};

pub struct Store {
    db: Connection,
}

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sqlite(error) => write!(f, "library database: {error}"),
            StoreError::Json(error) => write!(f, "library data: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::Sqlite(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        StoreError::Json(error)
    }
}

type Result<T> = std::result::Result<T, StoreError>;

/// The further languages as the JSON a row keeps; `[]` for none, so an empty column always
/// means a row from before languages existed.
/// A title colour kept as six hex digits; anything else is no colour.
fn colour_of_code(code: &str) -> Option<[u8; 3]> {
    let byte = |at: usize| code.get(at..at + 2).and_then(|pair| u8::from_str_radix(pair, 16).ok());
    (code.len() == 6).then(|| Some([byte(0)?, byte(2)?, byte(4)?])).flatten()
}

fn texts_json(translations: &[Translation]) -> Result<String> {
    Ok(serde_json::to_string(translations)?)
}

/// The first further language's words, which an older Artosia reads from `second`; a library
/// saved here keeps writing it, so opening it with the older program still shows that language.
fn second_of(translations: &[Translation]) -> &str {
    translations.first().map_or("", |translation| translation.text.as_str())
}

fn translations_of(json: &str) -> Vec<Translation> {
    serde_json::from_str(json).unwrap_or_default()
}

/// What an import did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Imported {
    pub added: usize,
    /// Replaced in place because the library's copy changed.
    pub updated: usize,
    /// Left alone because they were edited in Artosia.
    pub kept: usize,
}

const SCHEMA: &str = "
create table if not exists songs (
    id integer primary key,
    title text not null,
    alt_title text not null default '',
    author text not null default '',
    copyright text not null default '',
    ccli text not null default '',
    song_key text not null default '',
    number integer not null default 0,
    lyrics text not null default '',
    second text not null default '',
    play_order text not null default ''
);
create table if not exists plans (
    id integer primary key,
    name text not null,
    body text not null
);
create table if not exists announcements (
    id integer primary key,
    position integer not null default 0,
    title text not null,
    body text not null default '',
    seconds integer not null default 10
);
create table if not exists people (
    id integer primary key,
    name text not null,
    day integer not null,
    month integer not null
);
create table if not exists calendars (
    id integer primary key,
    name text not null,
    colour integer not null default 0
);
create table if not exists events (
    id integer primary key,
    calendar integer not null,
    name text not null,
    date text not null,
    start integer not null default 0,
    finish integer not null default 0,
    repeat text not null default 'once',
    until text not null default '',
    skipped text not null default ''
);
-- One row per changed date of a series, keyed by the date the rule gave it (kind 'moved' or 'cancelled').
-- `events.skipped` is still written with the cancelled dates, so an older Artosia reads them.
create table if not exists event_overrides (
    event_id integer not null,
    original text not null,
    kind text not null,
    date text not null default '',
    start integer not null default 0,
    finish integer not null default 0,
    name text not null default '',
    primary key (event_id, original)
);
create table if not exists funds (
    id integer primary key,
    name text not null,
    colour integer not null default 0
);
create table if not exists gifts (
    id integer primary key,
    date text not null,
    giver text not null default '',
    fund integer not null,
    method text not null default 'cash',
    amount integer not null default 0
);
create table if not exists settings (
    key text primary key,
    value text not null
);
-- What the search learns from (docs/palette-suggestions.md). Ids and folded typed text only:
-- no lyrics, no titles, no announcement text, nothing typed into a message.
create table if not exists palette_picks (
    id integer primary key,
    picked_at integer not null,
    service_date text not null,
    weekday integer not null,
    plan_id integer,
    kind text not null,
    target text not null,
    typed text not null default '',
    shown_rank integer,
    action text not null
);
create index if not exists palette_picks_typed on palette_picks (typed, target);
create index if not exists palette_picks_when on palette_picks (picked_at);
create table if not exists service_songs (
    id integer primary key,
    service_date text not null,
    weekday integer not null,
    plan_id integer,
    seq integer not null,
    song_id integer not null,
    shown_at integer not null,
    typed text not null default '',
    start_tag text not null default '',
    from_start integer not null default 1
);
create index if not exists service_songs_song on service_songs (song_id, service_date);
create table if not exists service_passages (
    id integer primary key,
    service_date text not null,
    plan_id integer,
    seq integer not null,
    reference text not null,
    shown_at integer not null,
    typed text not null default ''
);
";

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        let db = Connection::open(path)?;
        Store::prepare(db)
    }

    pub fn in_memory() -> Result<Store> {
        Store::prepare(Connection::open_in_memory()?)
    }

    fn prepare(db: Connection) -> Result<Store> {
        db.execute_batch(SCHEMA)?;
        // Libraries made before a column existed gain it here.
        let added = [
            ("songs", "source", "text not null default ''"),
            ("songs", "edited", "integer not null default 0"),
            ("songs", "lang", "text not null default ''"),
            ("songs", "texts", "text not null default ''"),
            ("people", "year", "integer not null default 0"),
            ("announcements", "image", "text not null default ''"),
            ("announcements", "fill", "text not null default ''"),
            ("announcements", "passage", "text not null default ''"),
            ("announcements", "style", "text not null default ''"),
            ("announcements", "title_colour", "text not null default ''"),
            ("announcements", "calendars", "text not null default ''"),
            ("announcements", "month", "integer not null default 0"),
            ("announcements", "window", "text not null default ''"),
            ("announcements", "highlight", "text not null default ''"),
        ];
        for (table, column, definition) in added {
            let present = db.prepare(&format!("select 1 from pragma_table_info('{table}') where name = ?1"))?.exists([column])?;
            if !present {
                db.execute_batch(&format!("alter table {table} add column {column} {definition}"))?;
            }
        }
        // A library written by 0.1.16 holds one extra language in `second`; it becomes the first
        // of `texts`. A row with `texts` still empty has not been moved (a saved row writes at
        // least `[]`), and `second` stays in place, unread, so nothing is ever lost.
        let old: Vec<(i64, String)> = {
            let mut statement = db.prepare("select id, second from songs where texts = '' and second != ''")?;
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect::<rusqlite::Result<_>>()?
        };
        for (id, second) in old {
            let moved = vec![Translation { lang: String::new(), text: second }];
            db.execute("update songs set texts = ?1 where id = ?2", params![serde_json::to_string(&moved)?, id])?;
        }
        Ok(Store { db })
    }

    pub(crate) fn db(&self) -> &Connection {
        &self.db
    }

    pub fn songs(&self) -> Result<Vec<Song>> {
        let mut statement = self.db.prepare(
            "select id, title, alt_title, author, copyright, ccli, song_key, number, lyrics, play_order, source, edited, lang, texts
             from songs order by title collate nocase",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Song {
                id: row.get(0)?,
                title: row.get(1)?,
                alt_title: row.get(2)?,
                author: row.get(3)?,
                copyright: row.get(4)?,
                ccli: row.get(5)?,
                key: row.get(6)?,
                number: row.get(7)?,
                lyrics: row.get(8)?,
                order: row.get(9)?,
                source: row.get(10)?,
                edited: row.get(11)?,
                lang: row.get(12)?,
                translations: translations_of(&row.get::<_, String>(13)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts a song with id 0, otherwise updates it; the song's id is set either way.
    pub fn save_song(&self, song: &mut Song) -> Result<()> {
        let texts = texts_json(&song.translations)?;
        let second = second_of(&song.translations);
        let values = params![
            song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number, song.lyrics,
            texts, song.order, song.source, song.edited, song.lang, second
        ];
        if song.id == 0 {
            self.db.execute(
                "insert into songs (title, alt_title, author, copyright, ccli, song_key, number, lyrics, texts, play_order, source, edited, lang, second)\n                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                values,
            )?;
            song.id = self.db.last_insert_rowid();
        } else {
            self.db.execute(
                "update songs set title = ?1, alt_title = ?2, author = ?3, copyright = ?4, ccli = ?5, song_key = ?6,\n                 number = ?7, lyrics = ?8, texts = ?9, play_order = ?10, edited = ?11, lang = ?12, second = ?13 where id = ?14",
                params![
                    song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                    song.lyrics, texts, song.order, song.edited, song.lang, second, song.id
                ],
            )?;
        }
        Ok(())
    }

    /// Adds many songs in one transaction. A song that carries a source id replaces the song
    /// imported earlier under that id, so a renamed or corrected song updates in place, unless
    /// that song was edited in Artosia: an edit made here wins over the library it came from.
    /// A song without a source id is skipped when the library already holds one with its
    /// title, author and opening words; items that only share a title, such as three weekly
    /// notices, all come in.
    pub fn import_songs(&mut self, songs: &mut [Song]) -> Result<Imported> {
        let transaction = self.db.transaction()?;
        let mut done = Imported::default();
        {
            let mut by_source = transaction.prepare("select id, edited, title, alt_title, lyrics, texts, play_order, lang from songs where source = ?1")?;
            let mut replace = transaction.prepare(
                "update songs set title = ?1, alt_title = ?2, author = ?3, copyright = ?4, ccli = ?5, song_key = ?6,\n                 number = ?7, lyrics = ?8, texts = ?9, play_order = ?10, lang = ?11, second = ?12 where id = ?13",
            )?;
            let mut exists = transaction.prepare("select id, lyrics from songs where title = ?1 collate nocase and author = ?2 collate nocase")?;
            let opening = |lyrics: &str| -> String { crate::text::normalize(lyrics).chars().take(60).collect() };
            let mut insert = transaction.prepare(
                "insert into songs (title, alt_title, author, copyright, ccli, song_key, number, lyrics, texts, play_order, source, lang, second)\n                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for song in songs.iter_mut() {
                if !song.source.is_empty() {
                    type Known = (i64, bool, String, String, String, String, String, String);
                    let known: Option<Known> = by_source
                        .query_row([&song.source], |row| {
                            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
                        })
                        .optional()?;
                    if let Some((id, edited, title, alt_title, lyrics, texts, order, lang)) = known {
                        song.id = id;
                        let same = title == song.title
                            && alt_title == song.alt_title
                            && lyrics == song.lyrics
                            && translations_of(&texts) == song.translations
                            && lang == song.lang
                            && order == song.order;
                        if same {
                            // Nothing changed in the library: not an update, and no edit to protect.
                        } else if edited {
                            done.kept += 1;
                        } else {
                            done.updated += replace.execute(params![
                                song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                                song.lyrics, texts_json(&song.translations)?, song.order, song.lang, second_of(&song.translations), id
                            ])?;
                        }
                        continue;
                    }
                }
                let same: Vec<(i64, String)> = exists
                    .query_map(params![song.title, song.author], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                let wanted = opening(&song.lyrics);
                if let Some((id, _)) = same.iter().find(|(_, lyrics)| opening(lyrics) == wanted) {
                    song.id = *id;
                    continue;
                }
                insert.execute(params![
                    song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                    song.lyrics, texts_json(&song.translations)?, song.order, song.source, song.lang, second_of(&song.translations)
                ])?;
                song.id = transaction.last_insert_rowid();
                done.added += 1;
            }
        }
        transaction.commit()?;
        Ok(done)
    }

    /// Puts a deleted song back under its old id, so plans that name it still find it.
    pub fn restore_song(&self, song: &Song) -> Result<()> {
        self.db.execute(
            "insert or replace into songs (id, title, alt_title, author, copyright, ccli, song_key, number, lyrics, texts, play_order, source, edited, lang, second)\n             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                song.id, song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                song.lyrics, texts_json(&song.translations)?, song.order, song.source, song.edited, song.lang, second_of(&song.translations)
            ],
        )?;
        Ok(())
    }

    pub fn delete_song(&self, id: i64) -> Result<()> {
        self.db.execute("delete from songs where id = ?1", [id])?;
        Ok(())
    }

    pub fn plans(&self) -> Result<Vec<Plan>> {
        let mut statement = self.db.prepare("select id, body from plans order by id desc")?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
        let mut plans = Vec::new();
        for row in rows {
            let (id, body) = row?;
            let mut plan: Plan = serde_json::from_str(&body)?;
            plan.id = id;
            plans.push(plan);
        }
        Ok(plans)
    }

    pub fn save_plan(&self, plan: &mut Plan) -> Result<()> {
        let body = serde_json::to_string(plan)?;
        if plan.id == 0 {
            self.db.execute("insert into plans (name, body) values (?1, ?2)", params![plan.name, body])?;
            plan.id = self.db.last_insert_rowid();
        } else {
            self.db.execute("update plans set name = ?1, body = ?2 where id = ?3", params![plan.name, body, plan.id])?;
        }
        Ok(())
    }

    /// Puts a deleted plan back under its old id, so the learning log still points at it.
    pub fn restore_plan(&self, plan: &Plan) -> Result<()> {
        let body = serde_json::to_string(plan)?;
        self.db.execute("insert or replace into plans (id, name, body) values (?1, ?2, ?3)", params![plan.id, plan.name, body])?;
        Ok(())
    }

    pub fn delete_plan(&self, id: i64) -> Result<()> {
        self.db.execute("delete from plans where id = ?1", [id])?;
        Ok(())
    }

    pub fn announcements(&self) -> Result<Vec<Announcement>> {
        let mut statement = self.db.prepare("select id, title, body, seconds, image, fill, passage, style, title_colour, calendars, month, window, highlight from announcements order by position, id")?;
        let rows = statement.query_map([], |row| {
            let (fill, style, colour, calendars): (String, String, String, String) = (row.get(5)?, row.get(7)?, row.get(8)?, row.get(9)?);
            let (month, window, highlight): (i64, String, String) = (row.get(10)?, row.get(11)?, row.get(12)?);
            Ok(Announcement {
                id: row.get(0)?,
                title: row.get(1)?,
                body: row.get(2)?,
                seconds: row.get(3)?,
                image: row.get(4)?,
                fill: Fill::from_code(&fill),
                passage: row.get(6)?,
                style: NoticeStyle::from_code(&style),
                title_colour: colour_of_code(&colour),
                calendars: calendars.split(',').filter_map(|id| id.parse().ok()).collect(),
                // A card saved before windows had its days in its data source's word and a month flag.
                window: Window::from_codes(&window, &highlight).unwrap_or_else(|| Window::migrated(&fill, month != 0)),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_announcement(&self, announcement: &mut Announcement) -> Result<()> {
        let style = announcement.style.map_or("", NoticeStyle::code);
        let colour = announcement.title_colour.map_or(String::new(), |[r, g, b]| format!("{r:02x}{g:02x}{b:02x}"));
        let calendars = announcement.calendars.iter().map(i64::to_string).collect::<Vec<_>>().join(",");
        let (window, highlight) = announcement.window.codes();
        // `month` is still written, so an older Artosia reads a card of the month as it did.
        let month = announcement.window.range == Range::Month(0);
        if announcement.id == 0 {
            self.db.execute(
                "insert into announcements (position, title, body, seconds, image, fill, passage, style, title_colour, calendars, month, window, highlight)
                 values ((select coalesce(max(position), 0) + 1 from announcements), ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![announcement.title, announcement.body, announcement.seconds, announcement.image, announcement.fill.code(), announcement.passage, style, colour, calendars, month, window, highlight],
            )?;
            announcement.id = self.db.last_insert_rowid();
        } else {
            self.db.execute(
                "update announcements set title = ?1, body = ?2, seconds = ?3, image = ?4, fill = ?5, passage = ?6, style = ?7, title_colour = ?8, calendars = ?9, month = ?10, window = ?11, highlight = ?12 where id = ?13",
                params![announcement.title, announcement.body, announcement.seconds, announcement.image, announcement.fill.code(), announcement.passage, style, colour, calendars, month, window, highlight, announcement.id],
            )?;
        }
        Ok(())
    }

    /// Everyone whose birthday is kept, by date.
    pub fn people(&self) -> Result<Vec<Person>> {
        let mut statement = self.db.prepare("select id, name, day, month, year from people order by month, day, name collate nocase")?;
        let rows = statement.query_map([], |row| Ok(Person { id: row.get(0)?, name: row.get(1)?, day: row.get(2)?, month: row.get(3)?, year: row.get(4)? }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn insert_person(&self, person: &Person) -> Result<i64> {
        self.db.execute("insert into people (name, day, month, year) values (?1, ?2, ?3, ?4)", params![person.name, person.day, person.month, person.year])?;
        Ok(self.db.last_insert_rowid())
    }

    /// Replaces the whole birthday list, as a fresh import of the church's file does.
    pub fn replace_people(&self, people: &[Person]) -> Result<()> {
        self.db.execute_batch("begin")?;
        let done = (|| -> Result<()> {
            self.db.execute("delete from people", [])?;
            for person in people {
                self.insert_person(person)?;
            }
            Ok(())
        })();
        self.db.execute_batch(if done.is_ok() { "commit" } else { "rollback" })?;
        done
    }

    /// Adds people to the list, as the dialog or an import that keeps the list does.
    pub fn add_people(&self, people: &[Person]) -> Result<()> {
        self.db.execute_batch("begin")?;
        let done = people.iter().try_for_each(|person| self.insert_person(person).map(drop));
        self.db.execute_batch(if done.is_ok() { "commit" } else { "rollback" })?;
        done
    }

    /// Inserts a person with id 0, otherwise updates them; the id is set either way.
    pub fn save_person(&self, person: &mut Person) -> Result<()> {
        if person.id == 0 {
            person.id = self.insert_person(person)?;
        } else {
            self.db.execute("update people set name = ?1, day = ?2, month = ?3, year = ?4 where id = ?5", params![person.name, person.day, person.month, person.year, person.id])?;
        }
        Ok(())
    }

    pub fn delete_person(&self, id: i64) -> Result<()> {
        self.db.execute("delete from people where id = ?1", [id])?;
        Ok(())
    }

    /// Puts the announcements in the order given; ids left out keep their place after them.
    pub fn order_announcements(&self, ids: &[i64]) -> Result<()> {
        for (position, id) in ids.iter().enumerate() {
            self.db.execute("update announcements set position = ?1 where id = ?2", params![position as i64 + 1, id])?;
        }
        Ok(())
    }

    pub fn delete_announcement(&self, id: i64) -> Result<()> {
        self.db.execute("delete from announcements where id = ?1", [id])?;
        Ok(())
    }

    /// The church's calendars, in the order they were made.
    pub fn calendars(&self) -> Result<Vec<Calendar>> {
        let mut statement = self.db.prepare("select id, name, colour from calendars order by id")?;
        let rows = statement.query_map([], |row| {
            let colour: u32 = row.get(2)?;
            Ok(Calendar { id: row.get(0)?, name: row.get(1)?, colour: [(colour >> 16) as u8, (colour >> 8) as u8, colour as u8] })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts a calendar with id 0, otherwise updates it; the id is set either way.
    pub fn save_calendar(&self, calendar: &mut Calendar) -> Result<()> {
        let [r, g, b] = calendar.colour;
        let colour = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        if calendar.id == 0 {
            self.db.execute("insert into calendars (name, colour) values (?1, ?2)", params![calendar.name, colour])?;
            calendar.id = self.db.last_insert_rowid();
        } else {
            self.db.execute("update calendars set name = ?1, colour = ?2 where id = ?3", params![calendar.name, colour, calendar.id])?;
        }
        Ok(())
    }

    /// Deletes a calendar and the events in it.
    pub fn delete_calendar(&self, id: i64) -> Result<()> {
        self.db.execute("delete from event_overrides where event_id in (select id from events where calendar = ?1)", [id])?;
        self.db.execute("delete from events where calendar = ?1", [id])?;
        self.db.execute("delete from calendars where id = ?1", [id])?;
        Ok(())
    }

    pub fn events(&self) -> Result<Vec<Event>> {
        let mut rows = self.db.prepare("select event_id, original, kind, date, start, finish, name from event_overrides")?;
        let mut changes: std::collections::HashMap<i64, Vec<Override>> = std::collections::HashMap::new();
        for found in rows.query_map([], |row| {
            let (id, original, kind, date, name): (i64, String, String, String, String) = (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(6)?);
            Ok((id, original, kind, date, row.get::<_, u16>(4)?, row.get::<_, u16>(5)?, name))
        })? {
            let (id, original, kind, date, start, end, name) = found?;
            let Some(original) = Date::parse(&original) else { continue };
            let change = match (kind.as_str(), Date::parse(&date)) {
                ("moved", Some(date)) => Change::Moved { date, start, end, name: (!name.is_empty()).then_some(name) },
                ("cancelled", _) => Change::Cancelled,
                _ => continue,
            };
            changes.entry(id).or_default().push(Override { original, change });
        }
        let mut statement = self.db.prepare("select id, calendar, name, date, start, finish, repeat, until, skipped from events order by id")?;
        let rows = statement.query_map([], |row| {
            let (date, repeat, until, skipped): (String, String, String, String) = (row.get(3)?, row.get(6)?, row.get(7)?, row.get(8)?);
            let id: i64 = row.get(0)?;
            let mut overrides = changes.get(&id).cloned().unwrap_or_default();
            // A date an older Artosia skipped has no row: it reads as cancelled.
            for original in skipped.split(',').filter_map(Date::parse) {
                if !overrides.iter().any(|item| item.original == original) {
                    overrides.push(Override { original, change: Change::Cancelled });
                }
            }
            overrides.sort_by_key(|item| item.original);
            Ok(Event {
                id,
                calendar: row.get(1)?,
                name: row.get(2)?,
                date: Date::parse(&date).unwrap_or(Date { year: 1970, month: 1, day: 1 }),
                start: row.get(4)?,
                end: row.get(5)?,
                repeat: Repeat::from_code(&repeat),
                until: Date::parse(&until),
                overrides,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts an event with id 0, otherwise updates it; the id is set either way.
    pub fn save_event(&self, event: &mut Event) -> Result<()> {
        let until = event.until.map(Date::code).unwrap_or_default();
        let skipped = event.overrides.iter().filter(|item| item.change == Change::Cancelled).map(|item| item.original.code()).collect::<Vec<_>>().join(",");
        if event.id == 0 {
            self.db.execute(
                "insert into events (calendar, name, date, start, finish, repeat, until, skipped) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![event.calendar, event.name, event.date.code(), event.start, event.end, event.repeat.code(), until, skipped],
            )?;
            event.id = self.db.last_insert_rowid();
        } else {
            self.db.execute(
                "update events set calendar = ?1, name = ?2, date = ?3, start = ?4, finish = ?5, repeat = ?6, until = ?7, skipped = ?8 where id = ?9",
                params![event.calendar, event.name, event.date.code(), event.start, event.end, event.repeat.code(), until, skipped, event.id],
            )?;
        }
        self.db.execute("delete from event_overrides where event_id = ?1", [event.id])?;
        for item in &event.overrides {
            let (kind, date, start, end, name) = match &item.change {
                Change::Cancelled => ("cancelled", String::new(), 0, 0, String::new()),
                Change::Moved { date, start, end, name } => ("moved", date.code(), *start, *end, name.clone().unwrap_or_default()),
            };
            self.db.execute("insert into event_overrides (event_id, original, kind, date, start, finish, name) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)", params![event.id, item.original.code(), kind, date, start, end, name])?;
        }
        Ok(())
    }

    pub fn delete_event(&self, id: i64) -> Result<()> {
        self.db.execute("delete from event_overrides where event_id = ?1", [id])?;
        self.db.execute("delete from events where id = ?1", [id])?;
        Ok(())
    }

    // ---- Offerings ----

    /// The church's funds, in the order they were made.
    pub fn funds(&self) -> Result<Vec<Fund>> {
        let mut statement = self.db.prepare("select id, name, colour from funds order by id")?;
        let rows = statement.query_map([], |row| {
            let colour: u32 = row.get(2)?;
            Ok(Fund { id: row.get(0)?, name: row.get(1)?, colour: [(colour >> 16) as u8, (colour >> 8) as u8, colour as u8] })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts a fund with id 0, otherwise updates it; the id is set either way.
    pub fn save_fund(&self, fund: &mut Fund) -> Result<()> {
        let [r, g, b] = fund.colour;
        let colour = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        if fund.id == 0 {
            self.db.execute("insert into funds (name, colour) values (?1, ?2)", params![fund.name, colour])?;
            fund.id = self.db.last_insert_rowid();
        } else {
            self.db.execute("update funds set name = ?1, colour = ?2 where id = ?3", params![fund.name, colour, fund.id])?;
        }
        Ok(())
    }

    /// Deletes a fund and the gifts given to it.
    pub fn delete_fund(&self, id: i64) -> Result<()> {
        self.db.execute("delete from gifts where fund = ?1", [id])?;
        self.db.execute("delete from funds where id = ?1", [id])?;
        Ok(())
    }

    /// Every gift, the newest date first.
    pub fn gifts(&self) -> Result<Vec<Gift>> {
        let mut statement = self.db.prepare("select id, date, giver, fund, method, amount from gifts order by date desc, id")?;
        let rows = statement.query_map([], |row| {
            let (date, method): (String, String) = (row.get(1)?, row.get(4)?);
            Ok(Gift { id: row.get(0)?, date: Date::parse(&date).unwrap_or(Date { year: 1970, month: 1, day: 1 }), giver: row.get(2)?, fund: row.get(3)?, method: Method::from_code(&method), amount: row.get(5)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts a gift with id 0, otherwise updates it; the id is set either way.
    pub fn save_gift(&self, gift: &mut Gift) -> Result<()> {
        if gift.id == 0 {
            self.db.execute("insert into gifts (date, giver, fund, method, amount) values (?1, ?2, ?3, ?4, ?5)", params![gift.date.code(), gift.giver, gift.fund, gift.method.code(), gift.amount])?;
            gift.id = self.db.last_insert_rowid();
        } else {
            self.db.execute("update gifts set date = ?1, giver = ?2, fund = ?3, method = ?4, amount = ?5 where id = ?6", params![gift.date.code(), gift.giver, gift.fund, gift.method.code(), gift.amount, gift.id])?;
        }
        Ok(())
    }

    pub fn delete_gift(&self, id: i64) -> Result<()> {
        self.db.execute("delete from gifts where id = ?1", [id])?;
        Ok(())
    }

    /// The transfer account, kept once; empty until it is written.
    pub fn account(&self) -> Account {
        self.setting("transfer_account").unwrap_or_default()
    }

    pub fn set_account(&self, account: &Account) -> Result<()> {
        self.set_setting("transfer_account", account)
    }

    /// Reads a setting, or None when it was never saved or no longer parses.
    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let value: Option<String> =
            self.db.query_row("select value from settings where key = ?1", [key], |row| row.get(0)).optional().ok().flatten();
        value.and_then(|json| serde_json::from_str(&json).ok())
    }

    pub fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.db.execute(
            "insert into settings (key, value) values (?1, ?2) on conflict(key) do update set value = excluded.value",
            params![key, json],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    /// What the first run's import costs the window, measured on a real library named by
    /// `ARTOSIA_SONG_ZIP` (it does nothing without the variable): the songs go in forty at a
    /// time, one batch a frame, on a library kept on disk, and the search is then built once.
    #[test]
    fn a_real_library_goes_in_forty_at_a_time() {
        let Some(zip) = std::env::var_os("ARTOSIA_SONG_ZIP") else { return };
        let folder = crate::import::as_folder(Path::new(&zip)).unwrap_or_else(|error| panic!("{error}"));
        let (mut songs, _) = crate::openlyrics::read_folder(&folder);
        let _ = std::fs::remove_dir_all(&folder);
        let path = std::env::temp_dir().join(format!("artosia-store-batches-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut store = Store::open(&path).unwrap_or_else(|error| panic!("{error}"));
        let mut took: Vec<f64> = Vec::new();
        for batch in songs.chunks_mut(40) {
            let start = std::time::Instant::now();
            store.import_songs(batch).unwrap_or_else(|error| panic!("{error}"));
            took.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        took.sort_by(f64::total_cmp);
        let start = std::time::Instant::now();
        let all = store.songs().unwrap_or_default();
        let read = start.elapsed().as_secs_f64() * 1000.0;
        let start = std::time::Instant::now();
        let index = crate::search::SongIndex::build(&all);
        let built = start.elapsed().as_secs_f64() * 1000.0;
        std::hint::black_box(index);
        drop(store);
        let _ = std::fs::remove_file(&path);
        println!(
            "{} songs in {} batches: median {:.1} ms, slowest {:.1} ms, all {:.0} ms; read back {:.0} ms; search built {:.0} ms",
            all.len(), took.len(), took[took.len() / 2], took[took.len() - 1], took.iter().sum::<f64>(), read, built
        );
    }

    use super::*;
    use crate::plan::PlanEntry;
    use crate::song::Translation;

    fn store() -> Store {
        Store::in_memory().unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn funds_gifts_and_the_account_round_trip_and_a_fund_takes_its_gifts_with_it() {
        let store = store();
        let mut tithe = Fund { id: 0, name: "Tithe".into(), colour: [1, 2, 3] };
        let mut missions = Fund { id: 0, name: "Missions".into(), colour: [200, 100, 50] };
        store.save_fund(&mut tithe).unwrap();
        store.save_fund(&mut missions).unwrap();
        let day = Date { year: 2026, month: 11, day: 8 };
        let mut first = Gift { id: 0, date: day, giver: String::new(), fund: tithe.id, method: Method::Cash, amount: 123_456_789_012 };
        let mut second = Gift { id: 0, date: Date { year: 2026, month: 11, day: 15 }, giver: "A. Giver".into(), fund: missions.id, method: Method::Transfer, amount: 5 };
        store.save_gift(&mut first).unwrap();
        store.save_gift(&mut second).unwrap();
        assert_eq!(store.gifts().unwrap(), vec![second.clone(), first.clone()], "newest date first");
        first.amount = 7;
        store.save_gift(&mut first).unwrap();
        assert_eq!(store.gifts().unwrap()[1].amount, 7);
        missions.colour = [9, 9, 9];
        store.save_fund(&mut missions).unwrap();
        assert_eq!(store.funds().unwrap()[1].colour, [9, 9, 9]);
        assert_eq!(store.account(), Account::default());
        let account = Account { bank: "Bank Contoh".into(), number: "0123".into(), holder: "Gereja Contoh".into(), note: String::new() };
        store.set_account(&account).unwrap();
        assert_eq!(store.account(), account);
        store.delete_gift(first.id).unwrap();
        assert_eq!(store.gifts().unwrap(), vec![second.clone()]);
        store.delete_fund(missions.id).unwrap();
        assert!(store.gifts().unwrap().is_empty());
        assert_eq!(store.funds().unwrap(), vec![tithe]);
    }

    #[test]
    fn calendars_and_events_round_trip_and_a_calendar_takes_its_events_with_it() {
        use crate::calendar::{Position, Repeat};
        let store = store();
        let mut services = Calendar { id: 0, name: "Services".into(), colour: [18, 52, 86] };
        let mut youth = Calendar { id: 0, name: "Youth".into(), colour: [255, 0, 1] };
        store.save_calendar(&mut services).unwrap();
        store.save_calendar(&mut youth).unwrap();
        assert!(services.id > 0 && youth.id > services.id);
        let day = |year, month, day| Date::new(year, month, day).unwrap();
        let mut weekly = Event { id: 0, calendar: services.id, name: "Morning".into(), date: day(2026, 1, 4), start: 390, end: 480, repeat: Repeat::Weekly(1, 2), until: Some(day(2027, 1, 3)), overrides: vec![Override { original: day(2026, 11, 1), change: Change::Cancelled }, Override { original: day(2026, 12, 27), change: Change::Moved { date: day(2026, 12, 29), start: 600, end: 660, name: Some("Evening".into()) } }] };
        let mut monthly = Event { id: 0, calendar: youth.id, name: "Night".into(), date: day(2026, 1, 1), start: 1110, end: 1170, repeat: Repeat::Monthly(Position::Last, 5, 3), until: None, overrides: Vec::new() };
        store.save_event(&mut weekly).unwrap();
        store.save_event(&mut monthly).unwrap();
        assert_eq!(store.calendars().unwrap(), vec![services.clone(), youth.clone()]);
        assert_eq!(store.events().unwrap(), vec![weekly.clone(), monthly.clone()]);
        weekly.name = "Early".into();
        weekly.until = None;
        weekly.overrides.clear();
        store.save_event(&mut weekly).unwrap();
        services.colour = [1, 2, 3];
        store.save_calendar(&mut services).unwrap();
        assert_eq!(store.events().unwrap(), vec![weekly.clone(), monthly.clone()]);
        assert_eq!(store.calendars().unwrap()[0].colour, [1, 2, 3]);
        store.delete_event(monthly.id).unwrap();
        assert_eq!(store.events().unwrap().len(), 1);
        store.delete_calendar(services.id).unwrap();
        assert!(store.events().unwrap().is_empty(), "the events of a deleted calendar go with it");
        assert_eq!(store.calendars().unwrap(), vec![youth]);
    }

    #[test]
    fn a_date_an_older_artosia_skipped_reads_as_cancelled_and_stays_in_step() {
        let store = store();
        let day = |year, month, day| Date::new(year, month, day).unwrap();
        let mut event = Event { id: 0, calendar: 1, name: "Old".into(), date: day(2026, 1, 4), start: 540, end: 600, repeat: Repeat::Weekly(1, 1), until: None, overrides: Vec::new() };
        store.save_event(&mut event).unwrap();
        // What an older program wrote: a skipped column and no override rows.
        store.db.execute("update events set skipped = '2026-11-22,2026-11-01' where id = ?1", [event.id]).unwrap();
        let read = store.events().unwrap();
        assert_eq!(read[0].overrides, vec![Override { original: day(2026, 11, 1), change: Change::Cancelled }, Override { original: day(2026, 11, 22), change: Change::Cancelled }]);
        // Saving writes rows and the column together, and bringing a date back clears both.
        let mut again = read[0].clone();
        again.set_change(day(2026, 11, 1), None);
        store.save_event(&mut again).unwrap();
        assert_eq!(store.events().unwrap()[0].overrides, vec![Override { original: day(2026, 11, 22), change: Change::Cancelled }]);
        let skipped: String = store.db.query_row("select skipped from events where id = ?1", [event.id], |row| row.get(0)).unwrap();
        assert_eq!(skipped, "2026-11-22");
        store.delete_event(event.id).unwrap();
        let rows: i64 = store.db.query_row("select count(*) from event_overrides", [], |row| row.get(0)).unwrap();
        assert_eq!(rows, 0, "a deleted event leaves no changes behind");
    }

    #[test]
    fn songs_round_trip_and_sort_by_title() {
        let store = store();
        let mut b = Song { title: "beta".into(), lyrics: "[1]\nline\n".into(), ..Song::default() };
        let mut a = Song { title: "Alpha".into(), ..Song::default() };
        assert!(store.save_song(&mut b).is_ok() && store.save_song(&mut a).is_ok());
        assert!(a.id != 0 && a.id != b.id);
        b.author = "Someone".into();
        assert!(store.save_song(&mut b).is_ok());
        let songs = store.songs().unwrap_or_default();
        assert_eq!(songs.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Alpha", "beta"]);
        assert_eq!(songs[1].author, "Someone");
        assert!(store.delete_song(a.id).is_ok());
        assert_eq!(store.songs().unwrap_or_default().len(), 1);
    }

    #[test]
    fn importing_twice_adds_nothing_new() {
        let mut store = store();
        let mut batch = vec![Song { title: "One".into(), ..Song::default() }, Song { title: "Two".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut batch).unwrap_or_default().added, 2);
        let mut again = vec![Song { title: "one".into(), ..Song::default() }, Song { title: "Three".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut again).unwrap_or_default().added, 1);
        assert_eq!(again[0].id, batch[0].id);
        // The same title with other words is another item, not a duplicate.
        let mut notice = vec![Song { title: "One".into(), lyrics: "[1]
Other words
".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut notice).unwrap_or_default().added, 1);
    }

    #[test]
    fn a_song_with_a_source_id_updates_in_place_when_renamed() {
        let mut store = store();
        let mut first = vec![Song { source: "s0001-old".into(), title: "Old title".into(), lyrics: "[1]\nWords\n".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut first).unwrap_or_default(), Imported { added: 1, updated: 0, kept: 0 });
        let mut renamed = vec![Song { source: "s0001-old".into(), title: "New title".into(), lyrics: "[1]\nBetter words\n".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut renamed).unwrap_or_default(), Imported { added: 0, updated: 1, kept: 0 });
        // The same copy again changes nothing and counts as nothing.
        assert_eq!(store.import_songs(&mut renamed).unwrap_or_default(), Imported::default());
        let songs = store.songs().unwrap_or_default();
        assert_eq!(songs.len(), 1);
        assert_eq!((songs[0].id, songs[0].title.as_str(), songs[0].lyrics.as_str()), (first[0].id, "New title", "[1]\nBetter words\n"));
    }

    #[test]
    fn a_song_edited_here_survives_a_later_import() {
        let mut store = store();
        let mut first = vec![Song { source: "s0002".into(), title: "Hymn".into(), lyrics: "[1]\nLibrary words\n".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut first).unwrap_or_default().added, 1);
        let mut mine = store.songs().unwrap_or_default().remove(0);
        mine.lyrics = "[1]\nOur church's words\n".into();
        mine.edited = true;
        assert!(store.save_song(&mut mine).is_ok());
        let mut corrected = vec![Song { source: "s0002".into(), title: "Hymn".into(), lyrics: "[1]\nLibrary words, corrected\n".into(), ..Song::default() }];
        assert_eq!(store.import_songs(&mut corrected).unwrap_or_default(), Imported { added: 0, updated: 0, kept: 1 });
        let songs = store.songs().unwrap_or_default();
        assert_eq!(songs[0].lyrics, "[1]\nOur church's words\n");
        assert!(songs[0].edited);
    }

    /// The schema 0.1.16 wrote, with a song that has a second language in `second`.
    const LIBRARY_0_1_16: &str = "\n        create table songs (id integer primary key, title text not null, alt_title text not null default '',\n            author text not null default '', copyright text not null default '', ccli text not null default '',\n            song_key text not null default '', number integer not null default 0, lyrics text not null default '',\n            second text not null default '', play_order text not null default '', source text not null default '',\n            edited integer not null default 0);\n        insert into songs (title, lyrics, second, source) values ('Two tongues', '[1]\nfirst words\n', '[1]\nsecond words\n', 'old-1');\n        insert into songs (title, lyrics) values ('One tongue', '[1]\nonly words\n');";

    #[test]
    fn a_library_from_0_1_16_opens_with_its_second_text_as_a_second_language() {
        let path = std::env::temp_dir().join(format!("artosia-store-old-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        rusqlite::Connection::open(&path).and_then(|db| db.execute_batch(LIBRARY_0_1_16)).expect("an old library is written");
        let songs = Store::open(&path).unwrap_or_else(|error| panic!("{error}")).songs().unwrap_or_default();
        let two = songs.iter().find(|song| song.title == "Two tongues").expect("the song is still there");
        assert_eq!(two.lyrics, "[1]\nfirst words\n");
        assert_eq!(two.translations, [Translation { lang: String::new(), text: "[1]\nsecond words\n".into() }]);
        assert_eq!(two.texts().count(), 2);
        let one = songs.iter().find(|song| song.title == "One tongue").expect("the song is still there");
        assert!(one.translations.is_empty());
        // Opened a second time nothing moves again, and a saved change to the languages stays.
        let store = Store::open(&path).unwrap_or_else(|error| panic!("{error}"));
        let mut again = store.songs().unwrap_or_default().into_iter().find(|song| song.title == "Two tongues").expect("there");
        again.lang = "id".into();
        again.translations[0].lang = "en".into();
        again.translations.push(Translation { lang: "zh-Hans".into(), text: "[1]\nthird words\n".into() });
        assert!(store.save_song(&mut again).is_ok());
        drop(store);
        let back = Store::open(&path).unwrap_or_else(|error| panic!("{error}")).songs().unwrap_or_default();
        let back = back.iter().find(|song| song.title == "Two tongues").expect("there");
        assert_eq!((back.lang.as_str(), back.translations.len(), back.translations[1].lang.as_str()), ("id", 2, "zh-Hans"));
        // An older Artosia reads `second`: it holds the first further language, as it did.
        let older = rusqlite::Connection::open(&path).expect("opens");
        let second = |title: &str| older.query_row("select second from songs where title = ?1", [title], |row| row.get::<_, String>(0)).expect("a row");
        assert_eq!(second("Two tongues"), "[1]\nsecond words\n");
        drop(older);
        // A song saved new, restored, or imported writes it too.
        let store = Store::open(&path).unwrap_or_else(|error| panic!("{error}"));
        let mut fresh = Song { title: "Fresh".into(), lyrics: "[1]\na\n".into(), translations: vec![Translation { lang: "en".into(), text: "[1]\nb\n".into() }], ..Song::default() };
        assert!(store.save_song(&mut fresh).is_ok());
        let mut imported = [Song { title: "Imported".into(), source: "s1".into(), lyrics: "[1]\na\n".into(), translations: fresh.translations.clone(), ..Song::default() }];
        let mut store = store;
        assert!(store.import_songs(&mut imported).is_ok());
        assert!(store.delete_song(fresh.id).is_ok());
        assert!(store.restore_song(&fresh).is_ok());
        drop(store);
        let older = rusqlite::Connection::open(&path).expect("opens");
        let second = |title: &str| older.query_row("select second from songs where title = ?1", [title], |row| row.get::<_, String>(0)).expect("a row");
        assert_eq!((second("Fresh"), second("Imported")), ("[1]\nb\n".to_string(), "[1]\nb\n".to_string()));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_import_updates_the_languages_of_a_song_and_leaves_an_edited_song_alone() {
        let mut store = store();
        let song = |english: &str| Song {
            source: "s0003".into(),
            title: "Hymn".into(),
            lyrics: "[1]\nKata\n".into(),
            lang: "id".into(),
            translations: vec![Translation { lang: "en".into(), text: english.into() }],
            ..Song::default()
        };
        assert_eq!(store.import_songs(&mut [song("[1]\nWords\n")]).unwrap_or_default().added, 1);
        // Only a further language changed in the library: that is an update.
        assert_eq!(store.import_songs(&mut [song("[1]\nBetter words\n")]).unwrap_or_default(), Imported { added: 0, updated: 1, kept: 0 });
        assert_eq!(store.songs().unwrap_or_default()[0].translations[0].text, "[1]\nBetter words\n");
        // Edited here, it is kept whatever the library says about any of its languages.
        let mut mine = store.songs().unwrap_or_default().remove(0);
        mine.translations.push(Translation { lang: "zh-Hans".into(), text: "[1]\nOurs\n".into() });
        mine.edited = true;
        assert!(store.save_song(&mut mine).is_ok());
        assert_eq!(store.import_songs(&mut [song("[1]\nCorrected\n")]).unwrap_or_default(), Imported { added: 0, updated: 0, kept: 1 });
        let kept = store.songs().unwrap_or_default().remove(0);
        assert_eq!((kept.translations.len(), kept.translations[0].text.as_str()), (2, "[1]\nBetter words\n"));
    }

    #[test]
    fn plans_and_settings_round_trip() {
        let store = store();
        let mut plan = Plan::named("Sunday");
        plan.push(PlanEntry::Heading { title: "Worship".into() });
        assert!(store.save_plan(&mut plan).is_ok());
        plan.push(PlanEntry::Announcements);
        assert!(store.save_plan(&mut plan).is_ok());
        let plans = store.plans().unwrap_or_default();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].items.len(), 2);
        assert!(store.set_setting("lines", &4usize).is_ok());
        assert_eq!(store.setting::<usize>("lines"), Some(4));
        assert_eq!(store.setting::<usize>("missing"), None);
    }

    #[test]
    fn the_plan_a_library_already_holds_becomes_the_first_of_many() {
        let store = store();
        // A row written before plans had a day: only the old fields in its body.
        let old = r#"{"id":0,"name":"Today","items":[{"id":1,"entry":"Announcements"}],"next_id":2}"#;
        store.db().execute("insert into plans (name, body) values ('Today', ?1)", [old]).expect("insert");
        let mut second = Plan::named("Youth");
        second.date = "2026-11-14".into();
        second.used = 99;
        assert!(store.save_plan(&mut second).is_ok());
        let plans = store.plans().unwrap_or_default();
        assert_eq!(plans.len(), 2);
        let first = plans.iter().find(|plan| plan.name == "Today").expect("the old plan is still there");
        assert_eq!((first.items.len(), first.date.as_str(), first.used), (1, "", 0));
        let youth = plans.iter().find(|plan| plan.name == "Youth").expect("the new plan is there");
        assert_eq!((youth.date.as_str(), youth.used), ("2026-11-14", 99));
        // A deleted plan comes back under its own id.
        assert!(store.delete_plan(youth.id).is_ok());
        assert_eq!(store.plans().unwrap_or_default().len(), 1);
        assert!(store.restore_plan(youth).is_ok());
        let back = store.plans().unwrap_or_default();
        assert!(back.iter().any(|plan| plan.id == youth.id && plan.name == "Youth"));
    }

    #[test]
    fn people_are_replaced_whole_and_read_by_date() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let person = |name: &str, month: u8, day: u8| Person { name: name.into(), day, month, ..Person::default() };
        assert!(store.replace_people(&[person("Later", 9, 2), person("Sooner", 2, 14)]).is_ok());
        assert!(store.replace_people(&[person("Only", 5, 5), person("First", 1, 1)]).is_ok());
        let names: Vec<String> = store.people().unwrap_or_default().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["First", "Only"]);
    }

    #[test]
    fn a_person_keeps_a_birth_year_and_is_added_edited_and_deleted_alone() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let mut ana = Person { name: "Ana".into(), day: 3, month: 4, year: 1988, ..Person::default() };
        assert!(store.save_person(&mut ana).is_ok() && ana.id != 0);
        assert!(store.add_people(&[Person { name: "Budi".into(), day: 1, month: 1, ..Person::default() }]).is_ok());
        ana.year = 0;
        ana.name = "Ana K".into();
        assert!(store.save_person(&mut ana).is_ok());
        let all = store.people().unwrap_or_default();
        assert_eq!(all.iter().map(|p| (p.name.as_str(), p.year)).collect::<Vec<_>>(), [("Budi", 0), ("Ana K", 0)]);
        assert!(store.delete_person(ana.id).is_ok());
        assert_eq!(store.people().unwrap_or_default().len(), 1);
    }

    #[test]
    fn a_library_made_before_the_year_column_gains_it() {
        let folder = std::env::temp_dir().join(format!("artosia-people-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap_or_default();
        let path = folder.join("library.db");
        let db = Connection::open(&path).unwrap_or_else(|_| unreachable!());
        assert!(db.execute_batch("create table people (id integer primary key, name text not null, day integer not null, month integer not null); insert into people (name, day, month) values ('Old', 5, 6);").is_ok());
        drop(db);
        let store = Store::open(&path).unwrap_or_else(|_| unreachable!());
        assert_eq!(store.people().unwrap_or_default().iter().map(|p| (p.name.as_str(), p.year)).collect::<Vec<_>>(), [("Old", 0)]);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn an_announcement_keeps_its_picture_and_what_fills_it() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let mut item = Announcement { title: "Birthdays".into(), image: "cake.png".into(), fill: Fill::Birthdays, seconds: 12, ..Announcement::default() };
        assert!(store.save_announcement(&mut item).is_ok());
        assert_eq!(store.announcements().unwrap_or_default(), [item]);
    }

    #[test]
    fn a_card_keeps_its_style_title_colour_calendars_and_window() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let mut item = Announcement { title: "Week".into(), fill: Fill::Schedule, style: Some(NoticeStyle::Band), title_colour: Some([1, 171, 255]), calendars: vec![3, 7], window: Window { range: Range::Month(1), highlight: Some(Range::Past(7)) }, ..Announcement::default() };
        assert!(store.save_announcement(&mut item).is_ok());
        assert_eq!(store.announcements().unwrap_or_default(), [item.clone()]);
        // A card saved with none of them reads back as the look it always had.
        let mut old = Announcement { title: "Old".into(), ..Announcement::default() };
        assert!(store.save_announcement(&mut old).is_ok());
        let kept = store.announcements().unwrap_or_default();
        assert_eq!((kept[1].style, kept[1].title_colour, kept[1].calendars.len(), kept[1].window), (None, None, 0, Window::default()));
    }

    #[test]
    fn a_card_saved_before_windows_opens_with_the_days_it_read() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        // Rows as 0.1.x wrote them: a data source word and the month flag, no window columns filled.
        for (title, fill, month) in [("Week", "birthdays-week", 0), ("Month", "birthdays-month", 0), ("Cal week", "schedule-week", 0), ("Cal month", "schedule-week", 1)] {
            store.db.execute("insert into announcements (title, fill, month) values (?1, ?2, ?3)", params![title, fill, month]).unwrap();
        }
        let cards = store.announcements().unwrap_or_default();
        assert_eq!(cards.iter().map(|card| (card.fill, card.window.range)).collect::<Vec<_>>(), [(Fill::Birthdays, Range::Coming(7)), (Fill::Birthdays, Range::Month(0)), (Fill::Schedule, Range::Coming(7)), (Fill::Schedule, Range::Month(0))]);
        // Saved again, a card keeps its window and no longer needs the old words.
        let mut moved = cards[1].clone();
        moved.window.highlight = Some(Range::Past(7));
        assert!(store.save_announcement(&mut moved).is_ok());
        assert_eq!(store.announcements().unwrap_or_default()[1].window, moved.window);
    }

    #[test]
    fn a_passage_card_keeps_its_reference() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let mut item = Announcement { title: "Verse of the week".into(), fill: Fill::Passage, passage: "Mazmur 23:1-3".into(), seconds: 12, ..Announcement::default() };
        assert!(store.save_announcement(&mut item).is_ok());
        item.passage = "Mazmur 23:4".into();
        assert!(store.save_announcement(&mut item).is_ok());
        assert_eq!(store.announcements().unwrap_or_default(), [item]);
    }

    #[test]
    fn announcements_keep_their_order() {
        let store = store();
        for title in ["First", "Second"] {
            let mut item = Announcement { title: title.into(), seconds: 8, ..Announcement::default() };
            assert!(store.save_announcement(&mut item).is_ok());
        }
        let mut ids: Vec<i64> = store.announcements().unwrap_or_default().iter().map(|a| a.id).collect();
        ids.reverse();
        assert!(store.order_announcements(&ids).is_ok());
        let reversed: Vec<i64> = store.announcements().unwrap_or_default().iter().map(|a| a.id).collect();
        assert_eq!(reversed, ids);
        ids.reverse();
        assert!(store.order_announcements(&ids).is_ok());
        let titles: Vec<String> = store.announcements().unwrap_or_default().into_iter().map(|a| a.title).collect();
        assert_eq!(titles, ["First", "Second"]);
    }
}
