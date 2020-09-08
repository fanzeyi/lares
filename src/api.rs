use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::convert::TryInto;
use std::future::Future;
use std::pin::Pin;
use tide::{log, Request};

use crate::model::{Feed, FeedGroup, Group, Item, ModelExt};
use crate::state::State;
use crate::utils::comma_join_vec;

const API_VERSION: &'static str = "2";

fn handle_groups(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("requesting groups");
    let (groups, feed_groups) = {
        let conn = request.state().db.get()?;
        (Group::all(&conn)?, FeedGroup::all(&conn)?)
    };

    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "groups": groups,
        "feeds_groups": feed_groups,
    }))
}

fn handle_feeds(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("requesting feeds");
    let (feeds, feed_groups) = {
        let conn = request.state().db.get()?;
        (Feed::all(&conn)?, FeedGroup::all(&conn)?)
    };

    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "feeds": feeds,
        "feeds_groups": feed_groups,
    }))
}

fn handle_items(
    request: Request<State>,
    since_id: Option<u32>,
) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("requesting items (since = {:?})", since_id);
    let (count, items) = {
        let conn = request.state().db.get()?;
        (Item::count(&conn)?, Item::select(&conn, since_id, false)?)
    };
    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "total_items": count,
        "items": items,
    }))
}

fn handle_unread_item_ids(
    request: Request<State>,
) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("requesting unread item ids");
    let item_ids = {
        let conn = request.state().db.get()?;
        Item::unread(&conn)?
    };
    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "unread_item_ids": comma_join_vec(item_ids),
    }))
}

fn handle_saved_item_ids(
    request: Request<State>,
) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("requesting saved item ids");
    let item_ids = {
        let conn = request.state().db.get()?;
        Item::saved(&conn)?
    };
    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
        "unread_item_ids": comma_join_vec(item_ids),
    }))
}

fn handle_write_form(
    request: Request<State>,
    form: WriteForm,
) -> Result<impl Into<tide::Response>, tide::Error> {
    log::info!("writing form: {:?}", form);
    match form.mark {
        MarkType::Item => {
            let item = {
                let conn = request.state().db.get()?;
                Item::get(&conn, form.id as u32)?
            };

            let conn = request.state().db.get()?;
            match form.r#as {
                Action::Read => {
                    item.read(&conn)?;
                }
                Action::Saved => {
                    item.save(&conn)?;
                }
                Action::Unsaved => {
                    item.unsave(&conn)?;
                }
            }
            handle_ok(request)
        }
        MarkType::Feed => {
            let feed = {
                let conn = request.state().db.get()?;
                Feed::get(&conn, form.id as u32)?
            };

            match form.r#as {
                Action::Saved | Action::Unsaved => bail!(400, "invalid as"),
                Action::Read => {
                    feed.read(form.before);
                    handle_ok(request)
                }
            }
        }
        MarkType::Group => {
            if form.id == 0 {
                // TODO: Kindling
                bail!(400, "unimplemented");
            } else if form.id == -1 {
                // TODO: Sparks
                bail!(400, "unimplemented");
            };

            let group = {
                let conn = request.state().db.get()?;
                Group::get(&conn, form.id as u32)?
            };

            match form.r#as {
                Action::Saved | Action::Unsaved => bail!(400, "invalid as"),
                Action::Read => {
                    group.read(form.before);
                    handle_ok(request)
                }
            }
        }
    }
}

fn handle_ok(_request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok(json!({
        "api_version": API_VERSION,
        "auth": 1,
    }))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum MarkType {
    Item,
    Feed,
    Group,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Action {
    Read,
    Saved,
    Unsaved,
}

#[derive(Deserialize, Debug)]
struct WriteForm {
    mark: MarkType,
    r#as: Action,
    id: i32,
    before: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct Auth {
    api_key: String,
}

macro_rules! unwrap_or_return {
    ($result: expr, $ret: expr) => {
        match $result {
            Some(v) => v,
            None => return $ret,
        }
    };
}

fn auth_error() -> tide::Result {
    Ok(json!({
        "api_version": API_VERSION,
        "auth": 0,
    })
    .into())
}

fn auth(
    mut request: Request<State>,
    next: tide::Next<'_, State>,
) -> Pin<Box<dyn Future<Output = tide::Result> + Send + '_>> {
    Box::pin(async move {
        let credential = unwrap_or_return!(
            request.state().credential.clone(),
            Ok(next.run(request).await)
        );
        let body = unwrap_or_return!(request.take_body().into_string().await.ok(), auth_error());
        // We need this to avoid taking away body from subsequent processing.
        request.set_body(body.clone());

        let auth = unwrap_or_return!(serde_urlencoded::from_str::<Auth>(&body).ok(), auth_error());
        if auth.api_key == credential {
            Ok(next.run(request).await)
        } else {
            auth_error()
        }
    })
}

pub fn make_app(state: State) -> tide::Server<State> {
    let mut app = tide::with_state(state);
    app.with(auth)
        .at("/")
        .get(|mut request: Request<State>| async move {
            let _ = request.body_string().await;
            Ok("")
        })
        .post(|mut request: Request<State>| async move {
            let form = request.body_form::<WriteForm>().await;
            let query = request.url().query_pairs().collect::<HashMap<_, _>>();

            let resp = if query.contains_key("groups") {
                handle_groups(request)?.into()
            } else if query.contains_key("feeds") {
                handle_feeds(request)?.into()
            } else if query.contains_key("items") {
                let since_id = query.get("since_id").and_then(|x| x.parse().ok());
                handle_items(request, since_id)?.into()
            } else if query.contains_key("unread_item_ids") {
                handle_unread_item_ids(request)?.into()
            } else if query.contains_key("saved_item_ids") {
                handle_saved_item_ids(request)?.into()
            } else if let Ok(form) = form {
                handle_write_form(request, form)?.into()
            } else {
                handle_ok(request)?.into()
            };

            Ok(resp)
        });

    app
}
