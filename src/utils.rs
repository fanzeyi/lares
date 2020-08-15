use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_timestamp() -> u128 {
    let start = SystemTime::now();
    start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}

pub fn comma_join_vec<T: IntoIterator<Item = U>, U: ToString>(items: T) -> String {
    items
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
