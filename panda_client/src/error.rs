use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Rate Limit: {0}")]
    RateLimit(String),
    #[error("Invalid HTML: {0}")]
    InvalidHTML(String),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Network Error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Cannot parse URL: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("Cannot parse date: {0}")]
    DateError(#[from] chrono::ParseError),
    #[error("Cannot parse integer: {0}")]
    IntegerError(#[from] std::num::ParseIntError),
    #[error("Parsing error: {0}")]
    ParsingError(#[from] bottle_util::ParsingError),
}
