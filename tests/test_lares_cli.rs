use anyhow::Result;
use assert_cmd::Command;
use async_std::task::{self, JoinHandle};
use chrono::TimeZone;
use lares::model::ModelExt;
use r2d2_sqlite::SqliteConnectionManager;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use tempfile::NamedTempFile;

fn get_available_port() -> Result<u16> {
    let listen = TcpListener::bind("127.0.0.1:0")?;
    Ok(listen.local_addr()?.port())
}

fn rand_string() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .collect()
}

fn get_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

struct Lares {
    db: NamedTempFile,
    pub pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Lares {
    fn new() -> Result<Self> {
        let db = tempfile::Builder::new().suffix(".db").tempfile()?;
        let pool = lares::model::get_pool(db.path())?;
        Ok(Self { db, pool })
    }

    fn cmd(&self) -> Result<Command> {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
        cmd.env("LARES_DATABASE", self.db.path());
        Ok(cmd)
    }

    fn run_fixture_server(&self) -> Result<(String, JoinHandle<()>)> {
        let port = get_available_port()?;
        let addr = format!("127.0.0.1:{}", port);

        let mut app = tide::new();
        app.at("/").serve_dir(get_fixtures_dir())?;

        let web = task::spawn({
            let addr = addr.clone();
            async move {
                app.listen(addr).await.unwrap();
            }
        });

        Ok((format!("http://{}", addr), web))
    }
}

#[test]
fn test_basic() -> Result<()> {
    let lares = Lares::new()?;
    lares.cmd()?.output().unwrap();
    Ok(())
}

#[test]
fn test_group_basic() -> Result<()> {
    let lares = Lares::new()?;

    let name1 = rand_string();
    lares.cmd()?.args(&["group", "add", &name1]).unwrap();
    lares.cmd()?.args(&["group", "add", &name1]).unwrap_err();

    let name2 = rand_string();
    lares.cmd()?.args(&["group", "add", &name2]).unwrap();

    let result = lares.cmd()?.args(&["group", "list"]).output()?;
    let stdout = String::from_utf8(result.stdout)?;

    assert!(stdout.contains(&name1));
    assert!(stdout.contains(&name2));

    lares.cmd()?.args(&["group", "delete", &name1]).unwrap();

    let result = lares.cmd()?.args(&["group", "list"]).output()?;
    let stdout = String::from_utf8(result.stdout)?;

    assert!(!stdout.contains(&name1));
    assert!(stdout.contains(&name2));

    Ok(())
}

#[test]
fn test_feed_basic() -> Result<()> {
    let lares = Lares::new()?;
    let (addr, _server) = lares.run_fixture_server()?;

    let rust = format!("{}/rust.xml", addr);
    let result = lares.cmd()?.args(&["feed", "add", &rust]).output()?;
    let stdout = String::from_utf8(result.stdout)?;

    assert!(stdout.contains(&format!("Feed URL: {}", rust)));
    assert!(stdout.contains("Site URL: https://blog.rust-lang.org/"));

    // duplicate feed
    lares.cmd()?.args(&["feed", "add", &rust]).unwrap_err();

    let result = lares.cmd()?.args(&["feed", "list"]).output()?;
    let stdout = String::from_utf8(result.stdout)?;
    assert!(stdout.contains(&rust));

    lares.cmd()?.args(&["feed", "delete", "1"]).unwrap();

    let result = lares.cmd()?.args(&["feed", "list"]).output()?;
    let stdout = String::from_utf8(result.stdout)?;
    assert!(!stdout.contains(&rust));

    Ok(())
}

#[test]
fn test_crawl() -> Result<()> {
    let lares = Lares::new()?;
    let (addr, _server) = lares.run_fixture_server()?;

    let rust = format!("{}/rust.xml", addr);
    let _ = lares.cmd()?.args(&["feed", "add", &rust]).output()?;

    // check database before crawling
    let conn = lares.pool.get()?;
    assert!(lares::model::Item::all(&conn)?.is_empty());

    lares.cmd()?.args(&["feed", "crawl", "1"]).unwrap();

    let items = lares::model::Item::all(&conn)?;
    assert_eq!(items.len(), 10);
    assert_eq!(items[9].title, "Announcing Rust 1.45.2");
    assert_eq!(items[0].title, "Five Years of Rust");
    assert_eq!(
        items[9].url,
        "https://blog.rust-lang.org/2020/08/03/Rust-1.45.2.html"
    );
    assert_eq!(items[9].is_read, false);
    assert_eq!(items[9].is_saved, false);
    assert_eq!(items[9].author, "The Rust Release Team");
    assert_eq!(
        items[9].created_on_time,
        chrono::Utc.ymd(2020, 08, 03).and_hms(0, 0, 0)
    );

    Ok(())
}

#[test]
fn test_import() -> Result<()> {
    let lares = Lares::new()?;
    let opml = get_fixtures_dir().join("normal.opml");
    lares.cmd()?.args(&["feed", "import"]).arg(&opml).unwrap();

    let conn = lares.pool.get()?;
    let groups = lares::model::Group::all(&conn)?;
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].title, "Group 1 Title");
    assert_eq!(groups[1].title, "Group 2 Text");

    let feeds = lares::model::Feed::all(&conn)?;
    assert_eq!(feeds.len(), 4);
    assert_eq!(feeds[0].title, "Feed 1 Title");
    assert_eq!(feeds[1].title, "Feed 2 Text");
    assert_eq!(feeds[2].title, "Feed 3 Title");
    assert_eq!(feeds[3].title, "Feed 4 Text");

    Ok(())
}

#[test]
fn test_import_fill_missing() -> Result<()> {
    let lares = Lares::new()?;
    let (addr, _server) = lares.run_fixture_server()?;

    let missing = get_fixtures_dir().join("missing.opml");
    let generated = std::fs::read_to_string(missing)?.replace("__REPLACE__", &addr);
    let mut opml = tempfile::NamedTempFile::new()?;
    opml.as_file_mut().write_all(generated.as_bytes())?;
    opml.as_file_mut().flush()?;

    lares
        .cmd()?
        .args(&["feed", "import"])
        .arg(opml.path())
        .unwrap();

    let conn = lares.pool.get()?;
    let feeds = lares::model::Feed::all(&conn)?;
    assert_eq!(feeds.len(), 1);
    assert_eq!(feeds[0].title, "Rust Blog");
    assert_eq!(feeds[0].site_url, "https://blog.rust-lang.org/feed.xml");
    Ok(())
}

#[test]
fn test_import_flat() -> Result<()> {
    let lares = Lares::new()?;
    let opml = get_fixtures_dir().join("flat.opml");
    lares.cmd()?.args(&["feed", "import"]).arg(&opml).unwrap();

    let conn = lares.pool.get()?;
    let groups = lares::model::Group::all(&conn)?;
    assert_eq!(groups.len(), 0);

    let feeds = lares::model::Feed::all(&conn)?;
    assert_eq!(feeds.len(), 2);
    assert_eq!(feeds[0].title, "Feed 1 Title");
    assert_eq!(feeds[1].title, "Feed 2 Title");

    Ok(())
}
