use log::{debug, info, warn};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;
use std::{collections::HashMap, io::BufRead};

use crate::error::Result;
use crate::model::Feed;
use crate::remote::RemoteFeed;

#[derive(Debug)]
pub struct ImportedFeed {
    rss_url: String,
    title: Option<String>,
    site_url: Option<String>,
}

impl ImportedFeed {
    fn new(rss_url: String, title: Option<String>, site_url: Option<String>) -> Self {
        ImportedFeed {
            rss_url,
            title,
            site_url,
        }
    }

    /// Update title and site_url from rss_url when they are unspecified
    pub async fn update(&mut self) -> Result<()> {
        if self.title.is_some() && self.site_url.is_some() {
            // nothing to fetch
            return Ok(());
        }

        info!("fetching missing metadata for feed {}", self);
        let remote = RemoteFeed::new(&self.rss_url).await?;

        if self.title.is_none() {
            self.title = remote.get_title();
        }

        if self.site_url.is_none() {
            self.site_url = remote.get_site_url();
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        url::Url::parse(&self.rss_url)?;
        if let Some(url) = self.site_url.as_ref() {
            url::Url::parse(url)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for ImportedFeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.rss_url)?;

        if self.title.is_some() || self.site_url.is_some() {
            write!(f, "(")?;

            if let Some(title) = self.title.as_ref() {
                write!(f, "\"{}\"", title)?;
            }

            if let Some(site_url) = self.site_url.as_ref() {
                if self.title.is_some() {
                    write!(f, ", ")?;
                }

                write!(f, "{}", site_url)?;
            }

            write!(f, ")")?;
        }

        Ok(())
    }
}

impl From<ImportedFeed> for Feed {
    fn from(feed: ImportedFeed) -> Feed {
        Feed::new(
            feed.title.unwrap_or_default(),
            feed.rss_url,
            feed.site_url.unwrap_or_default(),
        )
    }
}

type ImportResult = Result<Vec<(Option<String>, Vec<ImportedFeed>)>>;

pub fn from_file(path: &Path) -> ImportResult {
    let reader = Reader::from_file(&path)?;
    from_reader(reader)
}

fn from_reader<B: BufRead>(mut reader: Reader<B>) -> ImportResult {
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut feeds: Option<Vec<ImportedFeed>> = None;
    let mut group: Option<String> = None;
    let mut result: Vec<(Option<String>, Vec<ImportedFeed>)> = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                debug!("start tag <{}>", String::from_utf8_lossy(e.name()));

                if e.name() != b"outline" {
                    continue;
                }

                let attrs = e
                    .attributes()
                    .filter_map(|attr| {
                        if let Ok(attr) = attr {
                            Some((attr.key, attr.value))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<_, _>>();

                if group.is_some() {
                    warn!("possible malformed OPML file: previous group tag is not closed.");
                    continue;
                }

                if let Some(title) = attrs.get(&b"title"[..]).or_else(|| attrs.get(&b"text"[..])) {
                    let name = String::from_utf8_lossy(title).into_owned();
                    info!("processing group: {}", &name);
                    group = Some(name);
                }
            }
            Ok(Event::End(ref e)) => {
                debug!("end tag </{}>", String::from_utf8_lossy(e.name()));
                if e.name() == b"outline" {
                    if feeds.as_ref().map(|x| !x.is_empty()).unwrap_or(false) {
                        info!("processed group: {:?}", group.as_ref());
                        result.push((group.take(), feeds.take().unwrap()));
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tagname = String::from_utf8_lossy(e.name());
                debug!("self-closing tag <{} />", tagname);

                if e.name() != b"outline" {
                    continue;
                }

                let attrs = e
                    .attributes()
                    .filter_map(|attr| {
                        if let Ok(attr) = attr {
                            Some((attr.key, attr.value))
                        } else {
                            None
                        }
                    })
                    .collect::<HashMap<_, _>>();

                if feeds.is_none() {
                    feeds = Some(Vec::new());
                }

                let url = match attrs.get(&b"xmlUrl"[..]) {
                    Some(url) => url,
                    None => {
                        warn!(
                            "outline item does not contain feed URL, skipping: <{} {} />",
                            tagname,
                            String::from_utf8_lossy(e.attributes_raw()),
                        );
                        continue;
                    }
                };

                let title = attrs.get(&b"title"[..]).or_else(|| attrs.get(&b"text"[..]));
                let site_url = attrs.get(&b"htmlUrl"[..]);

                let rss_url = String::from_utf8_lossy(url).into_owned();
                let title = title.map(|x| String::from_utf8_lossy(x).into_owned());
                let site_url = site_url.map(|x| String::from_utf8_lossy(x).into_owned());

                info!(
                    "importing feed {} (\"{:?}\", site: {:?})",
                    rss_url, title, site_url
                );
                feeds
                    .as_mut()
                    .unwrap()
                    .push(ImportedFeed::new(rss_url, title, site_url));
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err((e, reader.buffer_position()).into());
            }
            _ => (),
        }
    }

    // no group catch
    if feeds.as_ref().map(|x| !x.is_empty()).unwrap_or(false) {
        info!("processed group: {:?}", group.as_ref());
        result.push((group.take(), feeds.take().unwrap()));
    }

    Ok(result)
}
