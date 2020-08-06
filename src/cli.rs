use crate::model::{Feed, FeedGroup, Group, ModelExt};
use crate::state::State;
use anyhow::{anyhow, Context, Result};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "lares", about = "Minified RSS service")]
pub enum Options {
    AddFeed {
        #[structopt(short = "g", long = "group")]
        group: Option<String>,

        #[structopt(name = "URL")]
        feed: String,
    },

    AddGroup {
        #[structopt(name = "NAME")]
        name: String,
    },

    AddFeedGroup {
        #[structopt(short = "g", long = "group")]
        group: String,

        #[structopt(name = "ID")]
        feed_id: u32,
    },
}

impl Options {
    fn add_feed(state: State, feed: String, group: Option<String>) -> Result<()> {
        let conn = state.db.get()?;
        Ok(())
    }

    fn add_group(state: State, name: String) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::new(name.clone());
        group
            .insert(&conn)
            .with_context(|| anyhow!("Unable to create group '{}'.", name))?;
        println!("Group '{}' added.", name);
        Ok(())
    }

    fn add_feed_group(state: State, group: String, feed_id: u32) -> Result<()> {
        let conn = state.db.get()?;
        let group = Group::get_by_name(&conn, &group)
            .with_context(|| anyhow!("Unable to find group '{}'", group))?;
        let _ = Feed::get(&conn, feed_id)
            .with_context(|| anyhow!("Unable to find feed with id = {}", feed_id))?;
        let feed_group = FeedGroup::new(group.id, feed_id);
        feed_group.insert(&conn)?;
        Ok(())
    }

    pub async fn run(self, state: crate::state::State) -> Result<()> {
        match self {
            Self::AddFeed { feed, group } => Self::add_feed(state, feed, group),
            Self::AddGroup { name } => Self::add_group(state, name),
            Self::AddFeedGroup { group, feed_id } => Self::add_feed_group(state, group, feed_id),
        }
    }
}
