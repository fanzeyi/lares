use anyhow::Result;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, NO_PARAMS};
use std::path::Path;

#[derive(Debug)]
pub struct Group {
    id: u32,
    title: String,
}

impl Group {
    pub fn create_table(conn: &Connection) -> Result<()> {
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS `group` (
            id INTEGER PRIMARY KEY,
            title TEXT
        )
        "#,
            NO_PARAMS,
        )?;
        Ok(())
    }

    pub fn get(conn: &Connection, id: u32) -> Result<Self> {
        conn.query_row(
            "SELECT id, title FROM `group` WHERE `id` = ?1",
            params![id],
            |row| {
                Ok(Self {
                    id: row.get(0)?,
                    title: row.get(1)?,
                })
            },
        )
        .map_err(|e| e.into())
    }
}

#[derive(Debug)]
pub struct Feed {
    id: u32,
    title: String,
    url: String,
    site_url: String,
    is_spark: bool,
    last_updated_on_time: u32,
}

impl Feed {
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

    pub fn get(conn: &Connection, id: u32) -> Result<Self> {
        conn.query_row(
            "SELECT id, title, url, site_url, is_spark, last_updated FROM `feed` WHERE `id` = ?1",
            params![id],
            |row| {
                Ok(Self {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    url: row.get(2)?,
                    site_url: row.get(3)?,
                    is_spark: row.get(4)?,
                    last_updated_on_time: row.get(5)?,
                })
            },
        )
        .map_err(|e| e.into())
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

    pub fn get(conn: &Connection, id: u32) -> Result<Self> {
        conn.query_row(
            "SELECT id, data FROM `favicon` WHERE `id` = ?1",
            params![id],
            |row| {
                Ok(Self {
                    id: row.get(0)?,
                    data: row.get(1)?,
                })
            },
        )
        .map_err(|e| e.into())
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

    pub fn get(conn: &Connection, id: u32) -> Result<Self> {
        conn.query_row(
            r#"SELECT
                id, feed_id, title, author, html, url, is_saved, is_read, created
            FROM `item` WHERE `id` = ?1"#,
            params![id],
            |row| {
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
            },
        )
        .map_err(|e| e.into())
    }
}

pub struct Database {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = r2d2::Pool::new(manager)?;

        {
            let conn = pool.get()?;

            Group::create_table(&conn)?;
            Feed::create_table(&conn)?;
            Favicon::create_table(&conn)?;
            Item::create_table(&conn)?;
        }

        Ok(Database { pool })
    }
}
