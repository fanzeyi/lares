use chrono::{DateTime, Utc};
use serde::Serializer;

pub fn comma_join_vec<T: IntoIterator<Item = U>, U: ToString>(items: T) -> String {
    items
        .into_iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub fn serialize_timestamp<S: Serializer>(
    val: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(val.timestamp())
}
