use either::Either;

use crate::client::HttpClient;
use crate::error::Result;
use crate::find::find_rel_alternates;

pub struct RemoteFeed {
    url: String,
    feed: feed_rs::model::Feed,
}

impl RemoteFeed {
    pub async fn new(url: &str) -> Result<Self> {
        let bytes = HttpClient::get(url).await?;
        let feed = feed_rs::parser::parse(&bytes[..])?;

        Ok(RemoteFeed {
            url: url.to_owned(),
            feed,
        })
    }

    /// Attempts to fetch and parse feed from the given url
    pub async fn try_new(url: &str) -> Result<Either<Self, Vec<String>>> {
        let bytes = HttpClient::get(url).await?;
        match feed_rs::parser::parse(&bytes[..]) {
            Ok(feed) => Ok(Either::Left(RemoteFeed {
                url: url.to_owned(),
                feed,
            })),
            Err(_) => Ok(Either::Right(find_rel_alternates(&bytes[..])?)),
        }
    }

    pub fn get_title(&self) -> Option<String> {
        self.feed.title.as_ref().map(|t| t.content.clone())
    }

    pub fn get_site_url(&self) -> Option<String> {
        self.feed
            .links
            .iter()
            .map(|l| l.href.as_str())
            .filter(|&link| link != self.url)
            .next()
            .map(|x| x.to_owned())
    }

    pub fn get_url(&self) -> &str {
        &self.url
    }
}
