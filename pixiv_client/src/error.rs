use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid field: {0}")]
    InvalidField(String),
    #[error("Cannot encode/decode JSON: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Network Error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Cannot parse URL: {0}")]
    UrlError(#[from] url::ParseError),
}
