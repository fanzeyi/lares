use crate::error::Result;
use crate::model::{Feed, ModelExt};
use crate::state::State;
use async_std::stream;
use async_std::stream::StreamExt;
use async_std::task;
use futures::future::join_all;
use std::time::Duration;

pub struct Crawler {
    state: State,
    interval_secs: u64,
}

impl Crawler {
    pub fn new(state: State, interval_secs: u64) -> Self {
        Crawler {
            state,
            interval_secs,
        }
    }

    async fn crawl(&self) -> Result<()> {
        let feeds = {
            let conn = self.state.db.get()?;
            Feed::all(&conn)?
        };

        let _ = join_all(feeds.into_iter().map(|feed| {
            let state = self.state.clone();
            task::spawn(async move { feed.crawl(state).await })
        }))
        .await;
        Ok(())
    }

    pub async fn runloop(self) -> Result<()> {
        let mut interval = stream::interval(Duration::from_secs(self.interval_secs));
        while let Some(_) = interval.next().await {
            match self.crawl().await {
                Ok(_) => (),
                Err(e) => eprintln!("error: {:?}", e),
            }
        }
        Ok(())
    }
}
