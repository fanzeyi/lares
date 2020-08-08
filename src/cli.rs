use crate::model::{Feed, FeedGroup, Group, ModelExt};
use crate::state::State;
use anyhow::{anyhow, Context, Result};
use prettytable::{cell, format, row, Cell, Row, Table};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum FeedCommand {
    List,
    Add { url: String },
    Delete { id: u32 },
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

    async fn add(state: State, url: String) -> Result<()> {
        let bytes = surf::get(&url).await.unwrap().body_bytes().await.unwrap();
        let channel = rss::Channel::read_from(&bytes[..])?;
        let feed = Feed::new(channel.title().to_owned(), url, channel.link().to_owned());
        {
            let conn = state.db.get()?;
            feed.insert(&conn)?;
        }
        Ok(())
    }

    fn delete(state: State, id: u32) -> Result<()> {
        Ok(())
    }

    async fn run(self, state: State) -> Result<()> {
        match self {
            Self::List => Self::list(state),
            Self::Add { url } => Self::add(state, url).await,
            Self::Delete { id } => Self::delete(state, id),
        }
    }
}

#[derive(Debug, StructOpt)]
pub enum GroupCommand {
    List,
    Add { name: String },
    AddFeed { id: u32, group: String },
    Delete { name: String },
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
        let _ = Feed::get(&conn, feed_id)
            .with_context(|| anyhow!("Unable to find feed with id = {}", feed_id))?;
        let feed_group = FeedGroup::new(group.id, feed_id);
        feed_group.insert(&conn)?;
        Ok(())
    }

    fn delete(state: State, name: String) -> Result<()> {
        Ok(())
    }

    async fn run(self, state: State) -> Result<()> {
        match self {
            Self::List => Self::list(state),
            Self::Add { name } => Self::add(state, name),
            Self::AddFeed { id, group } => Self::add_feed(state, id, group),
            Self::Delete { name } => Self::delete(state, name),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "lares", about = "Minified RSS service")]
pub enum Options {
    Feed(FeedCommand),
    Group(GroupCommand),
    Server,
}

impl Options {
    async fn server(state: State) -> Result<()> {
        let app = crate::api::make_app(state);
        app.listen("127.0.0.1:4000").await?;
        Ok(())
    }

    pub async fn run(self, state: State) -> Result<()> {
        match self {
            Options::Feed(cmd) => cmd.run(state).await,
            Options::Group(cmd) => cmd.run(state).await,
            Options::Server => Self::server(state).await,
        }
    }
}
