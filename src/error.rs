use thiserror::Error;

macro_rules! error {
    ($code: expr, $($t:tt)*) => {
        ::tide::Error::from_str(($code as u16).try_into().unwrap(), format!($($t)*));
    };
}

macro_rules! bail {
    ($code: expr, $($t:tt)*) => {
        return Err(error!($code, $($t)*));
    };
}

macro_rules! ensure {
    ($cond: expr, $code: expr, $($t:tt)*) => {
        if !($cond) {
            bail!($code, $($t)*);
        }
    };
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Database error")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Database pool error")]
    R2D2Error(#[from] r2d2::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
