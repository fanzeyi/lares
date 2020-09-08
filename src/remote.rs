use crate::error::Result;

pub struct RemoteFeed {
    url: String,
    feed: feed_rs::model::Feed,
}

impl RemoteFeed {
    pub async fn new(url: &str) -> Result<Self> {
        let bytes = surf::get(url).await?.body_bytes().await?;
        let feed = feed_rs::parser::parse(&bytes[..])?;

        Ok(RemoteFeed {
            url: url.to_owned(),
            feed,
        })
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
}
