use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("Not logged in: {0}")]
    NotLoggedIn(String),
    #[error("Rate Limit: {0}")]
    RateLimit(String),
    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Object `{0}` not found")]
    ObjectNotFound(String),
    #[error("Object `{0}` is not complete")]
    ObjectNotComplete(String),
    #[error("Object `{0}` already exists")]
    ObjectAlreadyExists(String),
    #[error("Unknown field: {0}")]
    UnknownField(String),

    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("Cannot encode/decode JSON: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("Cannot parse URL: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("Cannot parse path: {0}")]
    PathError(#[from] std::path::StripPrefixError),
    #[error("Cannot parse date: {0}")]
    DateError(#[from] chrono::ParseError),
    #[error("Cannot parse integer: {0}")]
    IntegerError(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
