use anyhow::{anyhow, Context, Result};
use async_std::prelude::FutureExt;
use either::Either;
use futures::stream::{self, StreamExt};
use log::{info, warn};
use prettytable::{cell, format, row, Table};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use structopt::StructOpt;

use crate::model::{Feed, FeedGroup, Group, Item, ModelExt};
use crate::opml;
use crate::remote::RemoteFeed;
use crate::state::State;

#[derive(Debug, StructOpt)]
pub enum FeedCommand {
    /// Lists all feeds
    List,

    /// Adds a new feed
    Add {
        url: String,
        #[structopt(short = "g", long = "group")]
        group: Option<String>,
    },

    /// Deletes a feed
    Delete { id: u32 },

    /// Crawls a feed manually
    Crawl { id: u32 },

    /// Imports OPML file
    Import { file: PathBuf },
}

impl FeedCommand {
    fn list(state: State) -> Result<()> {
        let feeds = {
            let conn = state.db.get()?;
            Feed::all(&conn)?
        };
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["id", "name", "feed url"]);

        for feed in feeds.into_iter() {
            table.add_row(row![feed.id, feed.title, feed.url]);
        }

        table.printstd();
        Ok(())
    }

    async fn select_remotes(state: &State, candidates: Vec<String>) -> Result<RemoteFeed> {
        if candidates.is_empty() {
            return Err(anyhow!(
                "Supplied URL is not a feed, and we can't find any potential candidate in the page."
            ));
        }

        let length = candidates.len();
        if length == 1 {
            let url = candidates.first().unwrap();
            log::info!(
                "Supplied URL is not a feed, but we found a potential candidate: {}",
                url
            );
            return Ok(RemoteFeed::new(url).await?);
        }

        println!(
            "Supplied URL is not a feed, but we found {} potential candidates. Please select one:",
            length
        );

        for (idx, url) in candidates.iter().enumerate() {
            println!("{}) {}", idx, url);
        }

        let stdin = io::stdin();
        loop {
            print!("select (0-{}, c to cancel): ", length - 1);
            io::stdout().flush()?;

            let mut selection = String::new();
            stdin.lock().read_line(&mut selection)?;

            let selection = selection.trim();
            if selection == "c" {
                break Err(anyhow!("No selection was made."));
            }

            match selection.parse::<usize>() {
                Ok(select) if select < length => {
                    let url = candidates.get(select).unwrap();

                    let feed = {
                        let conn = state.db.get()?;
                        Feed::get_by_url(&conn, &url)?
                    };

                    if feed.is_some() {
                        println!("Error: Invalid selection: selected feed already exists");
                        continue;
                    }

                    match RemoteFeed::new(candidates.get(select).unwrap()).await {
                        Ok(feed) => break Ok(feed),
                        Err(e) => println!("Error: Selection is not a feed: {}", e),
                    }
                }
                Ok(_) => {
                    println!("Error: Invalid selection: out of range");
                }
                Err(e) => {
                    println!("Error: Invalid selection: {}", e);
                }
            }
        }
    }

    async fn add(state: State, url: String, group: Option<String>) -> Result<()> {
        let feed = {
            let conn = state.db.get()?;
            Feed::get_by_url(&conn, &url)?
        };

        if feed.is_some() {
            return Err(anyhow!("Feed `{}` already exists!", url));
        }

        let remote = match RemoteFeed::try_new(&url).await? {
            Either::Left(remote) => remote,
            Either::Right(candidates) => Self::select_remotes(&state, candidates).await?,
        };

        let url = remote.get_url().to_owned();

        let feed = Feed::new(
            remote
                .get_title()
                .ok_or_else(|| anyhow!("Feed doesn't have a title"))?,
            url.clone(),
            remote.get_site_url().unwrap_or(url),
        );
        let feed = {
            let conn = state.db.get()?;
            feed.insert(&conn)?
        };
        println!("Feed added!\n{}", feed);

        if let Some(group) = group {
            let conn = state.db.get()?;
            let group = Group::get_by_name(&conn, &group)
                .with_context(|| anyhow!("Unable to find group '{}'", group))?;
            group.add_feed(&conn, feed)?;

            println!("Feed added to group {}", group.title);
        }
        Ok(())
    }

    fn delete(state: State, id: u32) -> Result<()> {
        let conn = state.db.get()?;
        let feed = Feed::get(&conn, id)?;
        FeedGroup::delete_by_feed(&conn, feed.id)?;
        Item::delete_by_feed(&conn, feed.id)?;
        let feed = feed.delete(&conn)?;
        println!("Feed deleted!\n{}", feed);
        Ok(())
    }

    async fn crawl(state: State, id: u32) -> Result<()> {
        let feed = {
            let conn = state.db.get()?;
            Feed::get(&conn, id)?
        };

        feed.crawl(state).await?;
        Ok(())
    }

    async fn import(state: State, file: PathBuf) -> Result<()> {
        let imports = opml::from_file(&file)?;

        let imports: Vec<_> = stream::iter(imports)
            .then(|(group, feeds)| async move {
                // normalize feeds
                let feeds = stream::iter(feeds)
                    .filter_map(|mut feed| async move {
                        if let Err(e) = feed.update().await {
                            warn!("failed to update feed {}: {:?}", feed, e);
                        }

                        if let Err(e) = feed.validate() {
                            warn!("invalid feed ({}): {:?}", feed, e);
                            None
                        } else {
                            Some(feed)
                        }
                    })
                    .map(Feed::from)
                    .collect::<Vec<Feed>>()
                    .await;

                (group, feeds)
            })
            .collect()
            .await;

        let conn = state.db.get()?;
        for (group, feeds) in imports.into_iter() {
            let group = group.and_then(|title| {
                if let Ok(group) = Group::get_by_name(&conn, &title) {
                    Some(group)
                } else {
                    let group = Group::new(title.clone());
                    match group.insert(&conn) {
                        Ok(group) => Some(group),
                        Err(e) => {
                            warn!("unable to create group {}: {:?}", title, e);
                            None
                        }
                    }
                }
            });

            for feed in feeds {
                let feed = match feed.insert(&conn) {
                    Err(e) => {
                        warn!("unable to create feed: {:?}", e);
                        continue;
                    }
                    Ok(feed) => feed,
                };

                if let Some(group) = group.as_ref() {
                    if let Err(e) = group.add_feed(&conn, feed) {
                        warn!("unable to add feed to group {:?}: {:?}", group, e);
                        continue;
                    }
                }
            }
        }

        info!("import completed.");

        Ok(())
    }

    async fn run(self, state: State) -> Result<()> {
        match self {
            Self::List => Self::list(state),
            Self::Add { url, group } => Self::add(state, url, group).await,
            Self::Delete { id } => Self::delete(state, id),
            Self::Crawl { id } => Self::crawl(state, id).await,
            Self::Import { file } => Self::import(state, file).await,
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum GroupCommand {
    /// Lists all groups
    List,

    /// Adds a group
    Add { name: String },

    /// Adds a feed to group
    AddFeed { id: u32, group: String },

    /// Deletes a group
    Delete { name: String },

    /// Prints the content of a group
    Show { name: String },
}

impl GroupCommand {
    fn list(state: State) -> Result<()> {
        let groups = {
            let conn = state.db.get()?;
            Group::all(&conn)?
        };
        let mut table = Table::new();
        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(row!["id", "name"]);

        for group in groups.into_iter() {
            table.add_row(row![group.id, group.title]);
        }

        table.printstd();
        Ok(())
    }

    fn add(state: State, name: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::new(name.clone());
        group
            .insert(&conn)
            .with_context(|| anyhow!("Unable to create group '{}'.", name))?;
        println!("Group '{}' added.", name);
        Ok(())
    }

    fn add_feed(state: State, feed_id: u32, group: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::get_by_name(&conn, &group)
            .with_context(|| anyhow!("Unable to find group '{}'", group))?;
        let feed = Feed::get(&conn, feed_id)
            .with_context(|| anyhow!("Unable to find feed with id = {}", feed_id))?;
        // if let Ok((_, group_id)) = FeedGroup::get_by_feed(&conn, feed_id) {}
        group.add_feed(&conn, feed)?;
        Ok(())
    }

    fn delete(state: State, group: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::get_by_name(&conn, &group)
            .with_context(|| anyhow!("Unable to find group '{}'", group))?;
        if let Ok(feed_groups) = FeedGroup::get_by_group(&conn, group.id) {
            if feed_groups.feed_ids.len() != 0 {
                println!("Warning: there are still feeds belong to this group");
            }
            feed_groups.delete(&conn)?;
        }
        let group = group.delete(&conn)?;
        println!("Group {} deleted", group.title);
        Ok(())
    }

    fn show(state: State, group: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::get_by_name(&conn, &group)
            .with_context(|| anyhow!("Unable to find group '{}'", group))?;
        let feeds = group.get_feeds(&conn)?;
        println!("Group {}:\n", group.title);
        for feed in feeds.iter() {
            println!("{}", feed);
        }
        Ok(())
    }

    async fn run(self, state: State) -> Result<()> {
        match self {
            Self::List => Self::list(state),
            Self::Add { name } => Self::add(state, name),
            Self::AddFeed { id, group } => Self::add_feed(state, id, group),
            Self::Delete { name } => Self::delete(state, name),
            Self::Show { name } => Self::show(state, name),
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct ServerConfig {
    #[structopt(short = "H", long = "host", default_value = "127.0.0.1")]
    /// Specifies server host
    host: String,

    #[structopt(short = "p", long = "port", default_value = "4000")]
    /// Specifies alternate port
    port: u32,

    #[structopt(short = "u", long = "username", requires = "password")]
    /// Specifies authentication username
    username: Option<String>,

    #[structopt(short = "P", long = "password", requires = "username")]
    /// Specifies authentication password
    password: Option<String>,

    #[structopt(short = "i", long = "interval", default_value = "30")]
    /// Specifies crawl interval (unit: minutes)
    interval: u32,
}

#[derive(Debug, StructOpt)]
pub enum SubCommand {
    /// Manages feeds
    Feed(FeedCommand),
    /// Manages group
    Group(GroupCommand),
    /// Starts web server
    Server(ServerConfig),
}

#[derive(StructOpt, Debug)]
#[structopt(name = "lares", about = "Minimal RSS service")]
pub struct Options {
    #[structopt(
        short = "d",
        long = "database",
        env = "LARES_DATABASE",
        default_value = "lares.db"
    )]
    database: PathBuf,

    #[structopt(long)]
    debug: bool,

    #[structopt(subcommand)]
    command: SubCommand,
}

impl Options {
    async fn server(mut state: State, config: ServerConfig) -> Result<()> {
        if let Some(username) = config.username {
            if let Some(password) = config.password {
                state = state.set_credential(username, password);
            }
        }

        let app = crate::api::make_app(state.clone());
        let crawl_interval = ((config.interval) * 60) as u64;
        let crwaler = crate::crawler::Crawler::new(state, crawl_interval);
        let (web, crawl) = app
            .listen(format!("{}:{}", config.host, config.port))
            .join(crwaler.runloop())
            .await;
        (web?, crawl?);
        Ok(())
    }

    pub async fn run(self) -> Result<()> {
        let pool = crate::model::get_pool(&self.database)?;
        let state = crate::state::State::new(pool);

        if self.debug {
            femme::with_level(log::LevelFilter::Debug);
        } else {
            femme::with_level(log::LevelFilter::Info);
        }

        match self.command {
            SubCommand::Feed(cmd) => cmd.run(state).await,
            SubCommand::Group(cmd) => cmd.run(state).await,
            SubCommand::Server(config) => Self::server(state, config).await,
        }
    }
}
