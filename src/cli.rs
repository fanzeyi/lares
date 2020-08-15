use crate::model::{Feed, FeedGroup, Group, ModelExt};
use crate::state::State;
use anyhow::{anyhow, Context, Result};
use async_std::prelude::FutureExt;
use prettytable::{cell, format, row, Table};
use structopt::StructOpt;

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

    async fn add(state: State, url: String, group: Option<String>) -> Result<()> {
        let bytes = surf::get(&url).await.unwrap().body_bytes().await.unwrap();
        let channel = rss::Channel::read_from(&bytes[..])?;
        let feed = Feed::new(channel.title().to_owned(), url, channel.link().to_owned());
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
        let feed = feed.delete(&conn)?;
        println!("Feed deleted!\n{}", feed);
        // TODO: delete related items
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

    async fn run(self, state: State) -> Result<()> {
        match self {
            Self::List => Self::list(state),
            Self::Add { url, group } => Self::add(state, url, group).await,
            Self::Delete { id } => Self::delete(state, id),
            Self::Crawl { id } => Self::crawl(state, id).await,
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
        group.add_feed(&conn, feed)?;
        Ok(())
    }

    fn delete(state: State, group: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::get_by_name(&conn, &group)
            .with_context(|| anyhow!("Unable to find group '{}'", group))?;
        let feed_groups = FeedGroup::get_by_group(&conn, group.id)?;
        if feed_groups.feed_ids.len() != 0 {
            println!("Warning: there are still feeds belong to this group");
        }
        feed_groups.delete(&conn)?;
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
#[structopt(name = "lares", about = "Minified RSS service")]
pub enum Options {
    /// Manages feeds
    Feed(FeedCommand),
    /// Manages group
    Group(GroupCommand),
    /// Starts web server
    Server {
        #[structopt(short = "H", long = "host", default_value = "127.0.0.1")]
        /// Specifies host of server
        host: String,

        #[structopt(short = "p", long = "port", default_value = "4000")]
        /// Specifies port of server
        port: u32,
    },
}

impl Options {
    async fn server(state: State, host: String, port: u32) -> Result<()> {
        let app = crate::api::make_app(state.clone());
        let crwaler = crate::crawler::Crawler::new(state);
        let (web, crawl) = app
            .listen(format!("{}:{}", host, port))
            .join(crwaler.runloop())
            .await;
        web.unwrap();
        crawl.unwrap();
        Ok(())
    }

    pub async fn run(self, state: State) -> Result<()> {
        match self {
            Options::Feed(cmd) => cmd.run(state).await,
            Options::Group(cmd) => cmd.run(state).await,
            Options::Server { host, port } => Self::server(state, host, port).await,
        }
    }
}
