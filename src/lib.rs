use structopt::StructOpt;

mod cli;
#[macro_use]
mod error;
mod api;
mod model;
mod state;

pub async fn server() -> Result<(), std::io::Error> {
    let pool = crate::model::get_pool("lares.db".as_ref()).unwrap();
    let state = crate::state::State::new(pool);
    let app = crate::api::make_app(state);
    app.listen("127.0.0.1:4000").await?;
    Ok(())
}

pub async fn cli() -> anyhow::Result<()> {
    let pool = crate::model::get_pool("lares.db".as_ref()).unwrap();
    let state = crate::state::State::new(pool);

    cli::Options::from_args().run(state).await
}
