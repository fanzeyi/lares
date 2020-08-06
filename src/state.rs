use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct State {
    pub db: Arc<r2d2::Pool<SqliteConnectionManager>>,
}

impl State {
    pub fn new(db: r2d2::Pool<SqliteConnectionManager>) -> Self {
        State { db: Arc::new(db) }
    }
}
