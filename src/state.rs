use md5::{Digest, Md5};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct State {
    pub db: Arc<r2d2::Pool<SqliteConnectionManager>>,
    pub credential: Option<String>,
}

impl State {
    pub fn new(db: r2d2::Pool<SqliteConnectionManager>) -> Self {
        State {
            db: Arc::new(db),
            credential: None,
        }
    }

    pub fn set_credential(mut self, username: String, password: String) -> Self {
        let mut hasher = Md5::new();
        hasher.update(format!("{}:{}", username, password));
        self.credential = Some(format!("{:2x}", hasher.finalize()));
        self
    }
}
