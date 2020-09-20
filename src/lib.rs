use structopt::StructOpt;

#[macro_use]
mod error;

mod api;
mod cli;
mod client;
mod crawler;
mod find;
pub mod model;
mod opml;
mod remote;
mod state;
mod utils;

pub async fn cli() -> anyhow::Result<()> {
    cli::Options::from_args().run().await
}
