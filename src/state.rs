use std::sync::Arc;

use crate::model::Database;

#[derive(Clone)]
pub struct State {
    db: Arc<Database>,
}

impl State {
    pub fn new(db: Database) -> Self {
        State { db: Arc::new(db) }
    }
}
