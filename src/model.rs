use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Row, NO_PARAMS};
use serde::{ser::SerializeSeq, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use crate::error::{Error, Result};

pub trait Model: Sized {
    const TABLE: &'static str;
    fn from_row(row: &Row) -> rusqlite::Result<Self>;
    fn set_id(&mut self, id: u32);
    fn get_id(&self) -> u32;
}

pub trait ModelExt<T> {
    fn get(conn: &Connection, id: u32) -> Result<T>;
    fn all(conn: &Connection) -> Result<Vec<T>>;
    fn delete(self, conn: &Connection) -> Result<T>;
    fn count(conn: &Connection) -> Result<u32>;
}

impl<T: Model + Sized> ModelExt<T> for T {
    fn get(conn: &Connection, id: u32) -> Result<T> {
        conn.query_row(
            &format!("SELECT * FROM `{}` WHERE `id` = ?1", Self::TABLE),
            params![id],
            Self::from_row,
        )
        .map_err(|e| e.into())
    }

    fn all(conn: &Connection) -> Result<Vec<Self>> {
        Ok(conn
            .prepare(&format!("SELECT * FROM `{}`", Self::TABLE))?
            .query_map(NO_PARAMS, Self::from_row)?
            .collect::<Result<_, _>>()?)
    }

    fn delete(self, conn: &Connection) -> Result<T> {
        conn.execute(
            &format!("DELETE FROM `{}` WHERE `id` = ?1", Self::TABLE),
            params![self.get_id()],
        )?;
        Ok(self)
    }

    fn count(conn: &Connection) -> Result<u32> {
        Ok(conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", Self::TABLE),
            NO_PARAMS,
            |row| row.get::<_, u32>(0),
        )?)
    }
}

#[derive(Debug, Serialize)]
pub struct Group {
    pub id: u32,
    pub title: String,
}

impl Group {
    pub fn new(title: String) -> Self {
        Self { id: 0, title }
    }

    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `group` (
            id INTEGER PRIMARY KEY,
            title TEXT UNIQUE ON CONFLICT IGNORE
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn insert(mut self, conn: &Connection) -> Result<Self> {
        self.id = conn
            .prepare("INSERT INTO `group` (title) VALUES (?1)")?
            .insert(params![self.title])? as u32;
        Ok(self)
    }

    pub fn get_by_name(conn: &Connection, title: &str) -> Result<Self> {
        conn.prepare("SELECT * FROM `group` WHERE `title` = ?1")?
            .query_row(params![title], Self::from_row)
            .map_err(Into::into)
    }

    pub fn get_feeds(&self, conn: &Connection) -> Result<Vec<Feed>> {
        Ok(conn
            .prepare(
                r"
        SELECT * 
        FROM `feed` 
        WHERE `id` IN (
            SELECT `feed_id` FROM `feed_group` WHERE `group_id` = ?1
        )",
            )?
            .query_map(params![self.id], Feed::from_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn add_feed(&self, conn: &Connection, mut feed: Feed) -> Result<Feed> {
        let feed_group = FeedGroup::new(self.id, feed.id);
        feed_group.insert(&conn)?;
        conn.execute(
            "UPDATE `feed` SET `is_spark` = 0 WHERE `id` = ?1",
            params![feed.id],
        )?;
        feed.is_spark = false;
        Ok(feed)
    }

    pub fn read(&self, before: Option<u32>) {}
}

impl Model for Group {
    const TABLE: &'static str = "group";

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
        })
    }

    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug, Serialize)]
pub struct Feed {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub site_url: String,
    pub is_spark: bool,
    pub last_updated_on_time: u32,
}

impl Feed {
    pub fn new(title: String, url: String, site_url: String) -> Self {
        Feed {
            id: 0,
            title,
            url,
            site_url,
            is_spark: true,
            last_updated_on_time: crate::utils::unix_timestamp() as u32,
        }
    }

    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `feed` (
            id INTEGER PRIMARY KEY,
            title TEXT,
            url TEXT,
            site_url TEXT,
            is_spark BOOLEAN,
            last_updated DATETIME
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn insert(mut self, conn: &Connection) -> Result<Self> {
        self.id = conn
            .prepare("INSERT INTO `feed` (title, url, site_url, is_spark, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)")?
            .insert(params![self.title, self.url, self.site_url, self.is_spark, self.last_updated_on_time])? as u32;
        Ok(self)
    }

    pub fn items(&self, conn: &Connection) -> Result<Vec<Item>> {
        Ok(conn
            .prepare("SELECT * FROM `item` WHERE `feed_id` = ?1 ORDER BY `id` DESC LIMIT 10")?
            .query_map(params![self.id], Item::from_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub async fn crawl(&self, state: crate::state::State) -> Result<()> {
        let exist_urls = {
            let conn = state.db.get()?;
            self.items(&conn)?
                .into_iter()
                .map(|item| item.url)
                .collect::<HashSet<String>>()
        };
        let content = surf::get(&self.url).await?.body_bytes().await?;
        let channel = rss::Channel::read_from(&content[..])?;

        let mut items = Vec::new();
        for item in channel.items() {
            if let Some(link) = item.link() {
                if exist_urls.contains(link) {
                    break;
                }

                items.push(Item {
                    feed_id: self.id,
                    title: item.title().unwrap_or_default().to_owned(),
                    author: item.author().unwrap_or_default().to_owned(),
                    html: item.content().unwrap_or_default().to_owned(),
                    url: link.to_owned(),
                    is_saved: false,
                    is_read: false,
                    ..Default::default()
                });
            }
        }

        {
            let conn = state.db.get()?;
            Item::insert_multi(&conn, items)?;
        }

        Ok(())
    }

    pub fn read(&self, before: Option<u32>) {}
}

impl Model for Feed {
    const TABLE: &'static str = "feed";

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            url: row.get(2)?,
            site_url: row.get(3)?,
            is_spark: row.get(4)?,
            last_updated_on_time: row.get(5)?,
        })
    }

    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

impl std::fmt::Display for Feed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "ID: {}", self.id)?;
        writeln!(f, "Name: {}", self.title)?;
        writeln!(f, "Feed URL: {}", self.url)?;
        writeln!(f, "Site URL: {}", self.site_url)
    }
}

#[derive(Debug, Serialize)]
pub struct FeedGroup {
    pub group_id: u32,
    #[serde(serialize_with = "FeedGroup::serialize_json")]
    pub feed_ids: Vec<u32>,
}

impl FeedGroup {
    pub fn new(group_id: u32, feed_id: u32) -> Self {
        Self {
            group_id,
            feed_ids: vec![feed_id],
        }
    }

    pub fn serialize_json<S: serde::Serializer>(ids: &Vec<u32>, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&crate::utils::comma_join_vec(ids))
    }

    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `feed_group` (
            id INTEGER PRIMARY KEY,
            group_id INTEGER,
            feed_id INTEGER,
            UNIQUE(group_id, feed_id) ON CONFLICT IGNORE
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }

    fn fold_group(mut indices: Vec<(u32, u32)>) -> Result<Vec<Self>> {
        indices.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        Ok(indices
            .into_iter()
            .fold(Vec::<(u32, Vec<u32>)>::new(), |mut accu, item| {
                if accu.last().map(|x| x.0 != item.0).unwrap_or(true) {
                    accu.push((item.0, Vec::new()));
                }
                // unwrap(): logically impossible to fail
                accu.last_mut().unwrap().1.push(item.1);
                accu
            })
            .into_iter()
            .map(|(group_id, feed_ids)| Self { group_id, feed_ids })
            .collect())
    }

    pub fn all(conn: &Connection) -> Result<Vec<Self>> {
        let indices = conn
            .prepare("SELECT group_id, feed_id FROM `feed_group`")?
            .query_map(NO_PARAMS, |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Self::fold_group(indices)
    }

    pub fn get_by_group(conn: &Connection, group_id: u32) -> Result<Self> {
        let indices = conn
            .prepare("SELECT group_id, feed_id FROM `feed_group` WHERE `group_id` = ?1")?
            .query_map(params![group_id], |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let result = Self::fold_group(indices)?;
        result
            .into_iter()
            .next()
            .ok_or_else(|| Error::message("unable to find group".to_owned()))
    }

    pub fn insert(self, conn: &Connection) -> Result<()> {
        let mut insert =
            conn.prepare("INSERT INTO `feed_group` (group_id, feed_id) VALUES (?1, ?2)")?;
        for feed_id in self.feed_ids.iter() {
            // maybe take care of `StatementChangedRows` when unique-ness is violated
            let _ = insert.execute(params![self.group_id, feed_id]);
        }
        Ok(())
    }

    pub fn delete(mut self, conn: &Connection) -> Result<Self> {
        let rarray = Rc::new(
            self.feed_ids
                .iter()
                .map(|&s| s as i64)
                .map(rusqlite::types::Value::from)
                .collect::<Vec<_>>(),
        );
        conn.prepare("UPDATE `feed` SET `is_spark` = 1 WHERE `id` IN rarray(?) ")?
            .execute(&[&rarray])?;
        conn.execute(
            "DELETE FROM `feed_group` WHERE `group_id` = ?1",
            params![self.group_id],
        )?;
        self.feed_ids.clear();
        Ok(self)
    }
}

#[derive(Debug)]
pub struct Favicon {
    id: u32,
    data: String,
}

impl Favicon {
    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `favicon` (
            id INTEGER PRIMARY KEY,
            data BLOB
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }
}

impl Model for Favicon {
    const TABLE: &'static str = "favicon";

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            data: row.get(1)?,
        })
    }

    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Item {
    pub id: u32,
    pub feed_id: u32,
    pub title: String,
    pub author: String,
    pub html: String,
    pub url: String,
    pub is_saved: bool,
    pub is_read: bool,
    pub created_on_time: Option<u32>,
}

impl Item {
    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `item` (
            id INTEGER PRIMARY KEY,
            feed_id INTEGER,
            title TEXT,
            author TEXT,
            html BLOB,
            url TEXT,
            is_saved BOOLEAN,
            is_read BOOLEAN,
            created DATETIME
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn insert_multi(conn: &Connection, items: Vec<Item>) -> Result<()> {
        let mut stmt = conn.prepare(
            r"
        INSERT INTO `item` (feed_id, title, author, html, url, is_saved, is_read)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;

        for item in items.into_iter() {
            stmt.execute(params![
                item.feed_id,
                item.title,
                item.author,
                item.html,
                item.url,
                item.is_saved,
                item.is_read
            ])?;
        }

        stmt.finalize()?;
        Ok(())
    }

    pub fn select(conn: &Connection, offset: Option<u32>, desc: bool) -> Result<Vec<Self>> {
        let mut stmt = "SELECT * FROM `item` ORDER BY `id` ".to_owned();
        if desc {
            stmt.push_str("DESC");
        }
        stmt.push_str(" LIMIT 50");
        if let Some(offset) = offset {
            stmt.push_str(&format!(" OFFSET {}", offset));
        }
        Ok(conn
            .prepare(dbg!(&stmt))?
            .query_map(NO_PARAMS, Self::from_row)?
            .collect::<Result<_, _>>()?)
    }

    pub fn unread(conn: &Connection) -> Result<Vec<u32>> {
        Ok(conn
            .prepare("SELECT id FROM `item` WHERE `is_read` = 0")?
            .query_map(NO_PARAMS, |row| row.get(0))?
            .collect::<Result<_, _>>()?)
    }

    pub fn saved(conn: &Connection) -> Result<Vec<u32>> {
        Ok(conn
            .prepare("SELECT id FROM `item` WHERE `is_saved` = 1")?
            .query_map(NO_PARAMS, |row| row.get(0))?
            .collect::<Result<_, _>>()?)
    }

    pub fn read(mut self, conn: &Connection) -> Result<Self> {
        conn.execute(
            "UPDATE `item` SET `is_read` = 1 WHERE `id` = ?1",
            params![self.id],
        )?;
        self.is_read = true;
        Ok(self)
    }

    pub fn save(mut self, conn: &Connection) -> Result<Self> {
        conn.execute(
            "UPDATE `item` SET `is_saved` = 1 WHERE `id` = ?1",
            params![self.id],
        )?;
        self.is_saved = true;
        Ok(self)
    }

    pub fn unsave(mut self, conn: &Connection) -> Result<Self> {
        conn.execute(
            "UPDATE `item` SET `is_saved` = 0 WHERE `id` = ?1",
            params![self.id],
        )?;
        self.is_saved = false;
        Ok(self)
    }
}

impl Model for Item {
    const TABLE: &'static str = "item";

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            feed_id: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            html: row.get(4)?,
            url: row.get(5)?,
            is_saved: row.get(6)?,
            is_read: row.get(7)?,
            created_on_time: row.get(8)?,
        })
    }

    fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    fn get_id(&self) -> u32 {
        self.id
    }
}

pub fn get_pool(path: &Path) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        rusqlite::vtab::array::load_module(&c)?;
        Ok(())
    });
    let pool = r2d2::Pool::new(manager)?;

    {
        let conn = pool.get()?;

        Group::create_table(&conn)?;
        Feed::create_table(&conn)?;
        FeedGroup::create_table(&conn)?;
        Favicon::create_table(&conn)?;
        Item::create_table(&conn)?;
    }

    Ok(pool)
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_test_feed(i: u32) -> Feed {
        Feed::new(
            format!("feed {}", i),
            format!("http://{}.example.com/feed", i),
            format!("http://{}.example.com/", i),
        )
    }

    fn make_test_group(i: u32) -> Group {
        Group::new(format!("group {}", i))
    }

    #[test]
    fn test_group() -> Result<()> {
        let conn = Connection::open_in_memory().unwrap();
        Group::create_table(&conn).unwrap();
        Feed::create_table(&conn).unwrap();
        FeedGroup::create_table(&conn).unwrap();

        // prepare
        let group = make_test_group(1).insert(&conn).unwrap();
        let feed1 = make_test_feed(1).insert(&conn).unwrap();
        let feed2 = make_test_feed(2).insert(&conn).unwrap();
        assert!(feed2.is_spark);

        let _feed1 = group.add_feed(&conn, feed1).unwrap();
        let feed2 = group.add_feed(&conn, feed2).unwrap();

        let result = Group::get_by_name(&conn, "group 1").unwrap();
        assert_eq!(result.id, group.id);

        let feeds = result.get_feeds(&conn).unwrap();
        assert_eq!(feeds.len(), 2);
        let feed2_new = feeds
            .into_iter()
            .filter(|x| x.id == feed2.id)
            .next()
            .unwrap();
        assert!(!feed2_new.is_spark);
        Ok(())
    }

    #[test]
    fn test_feed_group() {
        let conn = Connection::open_in_memory().unwrap();
        rusqlite::vtab::array::load_module(&conn).unwrap();
        Group::create_table(&conn).unwrap();
        Feed::create_table(&conn).unwrap();
        FeedGroup::create_table(&conn).unwrap();

        for group_id in 1..3 {
            let group = make_test_group(group_id).insert(&conn).unwrap();
            for feed_id in 1..(group_id + 5) {
                let feed = make_test_feed(feed_id).insert(&conn).unwrap();
                group.add_feed(&conn, feed).unwrap();
            }
        }

        let group = FeedGroup::get_by_group(&conn, 1).unwrap();
        assert_eq!(group.feed_ids.len(), 5);

        let groups = FeedGroup::all(&conn).unwrap();
        for group in groups.iter() {
            assert_eq!(group.feed_ids.len() as u32, 5 + group.group_id - 1);
        }

        let first = groups.into_iter().next().unwrap();
        let feed_id = first.feed_ids[0];
        first.delete(&conn).unwrap();

        let feed = Feed::get(&conn, feed_id).unwrap();
        assert!(feed.is_spark);
    }
}
