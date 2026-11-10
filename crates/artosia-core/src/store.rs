//! The library on disk: one SQLite file holding songs, plans, announcements and settings.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::deck::{Announcement, Fill, Person};
use crate::plan::Plan;
use crate::song::Song;

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
            ("announcements", "image", "text not null default ''"),
            ("announcements", "fill", "text not null default ''"),
        ];
        for (table, column, definition) in added {
            let present = db.prepare(&format!("select 1 from pragma_table_info('{table}') where name = ?1"))?.exists([column])?;
            if !present {
                db.execute_batch(&format!("alter table {table} add column {column} {definition}"))?;
            }
        }
        Ok(Store { db })
    }

    pub(crate) fn db(&self) -> &Connection {
        &self.db
    }

    pub fn songs(&self) -> Result<Vec<Song>> {
        let mut statement = self.db.prepare(
            "select id, title, alt_title, author, copyright, ccli, song_key, number, lyrics, second, play_order, source, edited
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
                second: row.get(9)?,
                order: row.get(10)?,
                source: row.get(11)?,
                edited: row.get(12)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Inserts a song with id 0, otherwise updates it; the song's id is set either way.
    pub fn save_song(&self, song: &mut Song) -> Result<()> {
        let values = params![
            song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number, song.lyrics,
            song.second, song.order, song.source, song.edited
        ];
        if song.id == 0 {
            self.db.execute(
                "insert into songs (title, alt_title, author, copyright, ccli, song_key, number, lyrics, second, play_order, source, edited)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                values,
            )?;
            song.id = self.db.last_insert_rowid();
        } else {
            self.db.execute(
                "update songs set title = ?1, alt_title = ?2, author = ?3, copyright = ?4, ccli = ?5, song_key = ?6,
                 number = ?7, lyrics = ?8, second = ?9, play_order = ?10, edited = ?11 where id = ?12",
                params![
                    song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                    song.lyrics, song.second, song.order, song.edited, song.id
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
            let mut by_source = transaction.prepare("select id, edited, title, alt_title, lyrics, second, play_order from songs where source = ?1")?;
            let mut replace = transaction.prepare(
                "update songs set title = ?1, alt_title = ?2, author = ?3, copyright = ?4, ccli = ?5, song_key = ?6,
                 number = ?7, lyrics = ?8, second = ?9, play_order = ?10 where id = ?11",
            )?;
            let mut exists = transaction.prepare("select id, lyrics from songs where title = ?1 collate nocase and author = ?2 collate nocase")?;
            let opening = |lyrics: &str| -> String { crate::text::normalize(lyrics).chars().take(60).collect() };
            let mut insert = transaction.prepare(
                "insert into songs (title, alt_title, author, copyright, ccli, song_key, number, lyrics, second, play_order, source)
                 values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for song in songs.iter_mut() {
                if !song.source.is_empty() {
                    type Known = (i64, bool, String, String, String, String, String);
                    let known: Option<Known> = by_source
                        .query_row([&song.source], |row| {
                            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
                        })
                        .optional()?;
                    if let Some((id, edited, title, alt_title, lyrics, second, order)) = known {
                        song.id = id;
                        let same = title == song.title && alt_title == song.alt_title && lyrics == song.lyrics && second == song.second && order == song.order;
                        if same {
                            // Nothing changed in the library: not an update, and no edit to protect.
                        } else if edited {
                            done.kept += 1;
                        } else {
                            done.updated += replace.execute(params![
                                song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                                song.lyrics, song.second, song.order, id
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
                    song.lyrics, song.second, song.order, song.source
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
            "insert or replace into songs (id, title, alt_title, author, copyright, ccli, song_key, number, lyrics, second, play_order, source, edited)
             values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                song.id, song.title, song.alt_title, song.author, song.copyright, song.ccli, song.key, song.number,
                song.lyrics, song.second, song.order, song.source, song.edited
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

    pub fn delete_plan(&self, id: i64) -> Result<()> {
        self.db.execute("delete from plans where id = ?1", [id])?;
        Ok(())
    }

    pub fn announcements(&self) -> Result<Vec<Announcement>> {
        let mut statement = self.db.prepare("select id, title, body, seconds, image, fill from announcements order by position, id")?;
        let rows = statement.query_map([], |row| {
            let fill: String = row.get(5)?;
            Ok(Announcement { id: row.get(0)?, title: row.get(1)?, body: row.get(2)?, seconds: row.get(3)?, image: row.get(4)?, fill: Fill::from_code(&fill) })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_announcement(&self, announcement: &mut Announcement) -> Result<()> {
        if announcement.id == 0 {
            self.db.execute(
                "insert into announcements (position, title, body, seconds, image, fill)
                 values ((select coalesce(max(position), 0) + 1 from announcements), ?1, ?2, ?3, ?4, ?5)",
                params![announcement.title, announcement.body, announcement.seconds, announcement.image, announcement.fill.code()],
            )?;
            announcement.id = self.db.last_insert_rowid();
        } else {
            self.db.execute(
                "update announcements set title = ?1, body = ?2, seconds = ?3, image = ?4, fill = ?5 where id = ?6",
                params![announcement.title, announcement.body, announcement.seconds, announcement.image, announcement.fill.code(), announcement.id],
            )?;
        }
        Ok(())
    }

    /// Everyone whose birthday is kept, by date.
    pub fn people(&self) -> Result<Vec<Person>> {
        let mut statement = self.db.prepare("select id, name, day, month from people order by month, day, name collate nocase")?;
        let rows = statement.query_map([], |row| Ok(Person { id: row.get(0)?, name: row.get(1)?, day: row.get(2)?, month: row.get(3)? }))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Replaces the whole birthday list, as a fresh import of the church's file does.
    pub fn replace_people(&self, people: &[Person]) -> Result<()> {
        self.db.execute_batch("begin")?;
        let done = (|| -> Result<()> {
            self.db.execute("delete from people", [])?;
            for person in people {
                self.db.execute("insert into people (name, day, month) values (?1, ?2, ?3)", params![person.name, person.day, person.month])?;
            }
            Ok(())
        })();
        self.db.execute_batch(if done.is_ok() { "commit" } else { "rollback" })?;
        done
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
    use super::*;
    use crate::plan::PlanEntry;

    fn store() -> Store {
        Store::in_memory().unwrap_or_else(|error| panic!("{error}"))
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
    fn people_are_replaced_whole_and_read_by_date() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let person = |name: &str, month: u8, day: u8| Person { id: 0, name: name.into(), day, month };
        assert!(store.replace_people(&[person("Later", 9, 2), person("Sooner", 2, 14)]).is_ok());
        assert!(store.replace_people(&[person("Only", 5, 5), person("First", 1, 1)]).is_ok());
        let names: Vec<String> = store.people().unwrap_or_default().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["First", "Only"]);
    }

    #[test]
    fn an_announcement_keeps_its_picture_and_what_fills_it() {
        let store = Store::in_memory().unwrap_or_else(|_| unreachable!());
        let mut item = Announcement { title: "Birthdays".into(), image: "cake.png".into(), fill: Fill::BirthdaysWeek, seconds: 12, ..Announcement::default() };
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
