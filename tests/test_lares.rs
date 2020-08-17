use anyhow::Result;
use assert_cmd::Command;
use async_std::task::{self, JoinHandle};
use rand::distributions::Alphanumeric;
use rand::Rng;
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

struct Lares {
    db: NamedTempFile,
}

impl Lares {
    fn new() -> Result<Self> {
        let db = tempfile::Builder::new().suffix(".db").tempfile()?;
        Ok(Self { db })
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
        app.at("/")
            .serve_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures"))?;

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
