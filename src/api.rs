use serde_json::json;
use std::convert::TryInto;
use tide::Request;

use crate::model::{Favicon, Feed, FeedGroup, Group, Item, ModelExt};
use crate::state::State;

const API_VERSION: &'static str = "2";

fn handle_groups(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    let (groups, feed_groups) = {
        let conn = request.state().db.as_ref().get()?;
        (Group::all(&conn)?, FeedGroup::all(&conn)?)
    };

    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "groups": groups,
        "feed_groups": feed_groups,
    }))
}

fn handle_feeds(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    let (feeds, feed_groups) = {
        let conn = request.state().db.as_ref().get()?;
        (Feed::all(&conn)?, dbg!(FeedGroup::all(&conn))?)
    };

    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "feeds": feeds,
        "feed_groups": feed_groups,
    }))
}

fn handle_favicons(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_items(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_ok(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok(json!({
        "api_version": "2",
        "auth": 1,
    }))
}

pub fn make_app(state: State) -> tide::Server<State> {
    tide::log::start();

    let mut app = tide::with_state(state);
    app.at("/")
        .get(|mut request: Request<State>| async move {
            dbg!(&request);
            dbg!(request.body_string().await);
            Ok("")
        })
        .post(|mut request: Request<State>| async move {
            dbg!(request.body_string().await);
            let query = dbg!(&request)
                .url()
                .query()
                .ok_or_else(|| error!(400, "unable to parse request"))?;
            let method = dbg!(query)
                .strip_prefix("api")
                .ok_or_else(|| error!(400, "unable to parse request"))?;

            let resp = match method {
                "&groups" => handle_groups(request)?.into(),
                "&feeds" => handle_feeds(request)?.into(),
                "&favicons" => handle_favicons(request)?.into(),
                "&items" => handle_items(request)?.into(),
                "" => handle_ok(request)?.into(),
                _ => bail!(400, "unsupported method"),
            };

            Ok(resp)
        });

    app
}
