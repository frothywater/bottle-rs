use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Network Error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("JPEG error: {0}")]
    JPEGError(#[from] jpeg_encoder::EncodingError),
    #[error("Incomplete download: {0}")]
    IncompleteDownload(String),
}
