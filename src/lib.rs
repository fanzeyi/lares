use structopt::StructOpt;

#[macro_use]
mod error;

mod api;
mod cli;
mod model;
mod state;
mod utils;

pub async fn cli() -> anyhow::Result<()> {
    let pool = crate::model::get_pool("lares.db".as_ref()).unwrap();
    let state = crate::state::State::new(pool);

    cli::Options::from_args().run(state).await
}
