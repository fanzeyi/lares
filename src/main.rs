use std::convert::TryInto;
use tide::Request;

#[macro_use]
mod error;
mod model;
mod state;

use crate::model::Database;
use crate::state::State;

fn handle_groups(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_feeds(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_favicons(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_items(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

fn handle_links(request: Request<State>) -> Result<impl Into<tide::Response>, tide::Error> {
    Ok("")
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let db = Database::new("lares.db".as_ref()).unwrap();
    let state = State::new(db);

    tide::log::start();
    let mut app = tide::with_state(state);
    app.at("/").get(|request: Request<State>| async move {
        let query = request
            .url()
            .query()
            .ok_or_else(|| error!(400, "unable to parse request"))?;
        let method = query
            .strip_prefix("api&")
            .ok_or_else(|| error!(400, "unable to parse request"))?;

        let resp = match method {
            "groups" => handle_groups(request)?.into(),
            "feeds" => handle_feeds(request)?.into(),
            "favicons" => handle_favicons(request)?.into(),
            "items" => handle_items(request)?.into(),
            "links" => handle_links(request)?.into(),
            _ => bail!(400, "unsupported method"),
        };

        Ok(resp)
    });
    app.listen("127.0.0.1:4000").await?;

    Ok(())
}
