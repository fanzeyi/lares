use fake::faker::filesystem::en::DirPath;
use fake::faker::lorem::en::{Word, Words};
use fake::faker::name::en::Name;
use fake::{Dummy, Fake, Faker};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Row, NO_PARAMS};
use serde::Serialize;
use std::path::Path;

use crate::error::Result;

pub trait Model: Sized {
    const TABLE: &'static str;
    fn from_row(row: &Row) -> rusqlite::Result<Self>;
    fn set_id(&mut self, id: u32);
}

pub trait ModelExt<T> {
    fn get(conn: &Connection, id: u32) -> Result<T>;
    fn all(conn: &Connection) -> Result<Vec<T>>;
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
}

#[derive(Debug, Serialize, Dummy)]
pub struct Group {
    pub id: u32,
    #[dummy(faker = "Name()")]
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

    pub fn fake() -> Self {
        let domain: String = Word().fake();

        Feed {
            id: 0,
            title: Words(10..15).fake::<Vec<String>>().join(" "),
            url: format!("http://www.{}.com{}", domain, DirPath().fake::<String>()),
            site_url: format!("http://www.{}.com/", domain),
            is_spark: true,
            last_updated_on_time: 1596610160,
        }
    }

    pub fn insert(mut self, conn: &Connection) -> Result<Self> {
        self.id = conn
            .prepare("INSERT INTO `feed` (title, url, site_url, is_spark, last_updated) VALUES (?1, ?2, ?3, ?4, ?5)")?
            .insert(params![self.title, self.url, self.site_url, self.is_spark, self.last_updated_on_time])? as u32;
        Ok(self)
    }
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
        ser.serialize_str(
            &ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(","),
        )
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

    pub fn all(conn: &Connection) -> Result<Vec<Self>> {
        let mut indices = conn
            .prepare("SELECT group_id, feed_id FROM `feed_group`")?
            .query_map(NO_PARAMS, |row| {
                Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
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

    pub fn insert(self, conn: &Connection) -> Result<()> {
        let mut insert =
            conn.prepare("INSERT INTO `feed_group` (group_id, feed_id) VALUES (?1, ?2)")?;
        for feed_id in self.feed_ids.iter() {
            // maybe take care of `StatementChangedRows` when unique-ness is violated
            let _ = insert.execute(params![self.group_id, feed_id]);
        }
        Ok(())
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
}

#[derive(Debug)]
pub struct Item {
    id: u32,
    feed_id: u32,
    title: String,
    author: String,
    html: String,
    url: String,
    is_saved: bool,
    is_read: bool,
    created_on_time: u32,
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
}

pub fn get_pool(path: &Path) -> Result<r2d2::Pool<SqliteConnectionManager>> {
    let manager = SqliteConnectionManager::file(path);
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

#[test]
fn test_fill_group_fixture() {
    let pool = get_pool("lares.db".as_ref()).unwrap();
    let conn = pool.get().unwrap();
    let group = Faker.fake::<Group>();
    group.insert(&conn).unwrap();
}

#[test]
fn test_fill_feed_fixture() {
    let pool = get_pool("lares.db".as_ref()).unwrap();
    let conn = pool.get().unwrap();
    let feed = Feed::fake();
    feed.insert(&conn).unwrap();
}
