use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_timestamp() -> u128 {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}
