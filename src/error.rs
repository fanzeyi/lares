use thiserror::Error;

macro_rules! error {
    ($code: expr, $($t:tt)*) => {
        ::tide::Error::from_str(
            ($code as u16).try_into().unwrap_or(::tide::StatusCode::BadRequest),
            format!($($t)*)
        );
    };
}

macro_rules! bail {
    ($code: expr, $($t:tt)*) => {
        return Err(error!($code, $($t)*));
    };
}

macro_rules! _ensure {
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

    #[error("HTTP error")]
    SurfError(#[from] surf::Exception),

    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("Feed error")]
    FeedError(#[from] feed_rs::parser::ParseFeedError),

    #[error("XML error")]
    XmlError(#[from] quick_xml::Error),

    #[error("XML error at position {position}: {source}")]
    XmlErrorWithPosition {
        #[source]
        source: quick_xml::Error,
        position: usize,
    },

    #[error("url parsing error")]
    UrlError(#[from] url::ParseError),

    #[error("{}", _0)]
    Message(String),
}

impl Error {
    pub fn message(msg: String) -> Self {
        Error::Message(msg)
    }
}

impl From<(quick_xml::Error, usize)> for Error {
    fn from((source, position): (quick_xml::Error, usize)) -> Self {
        Error::XmlErrorWithPosition { source, position }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
