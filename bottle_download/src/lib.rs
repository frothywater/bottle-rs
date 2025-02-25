mod error;
mod harvest;
mod thumb;

pub use error::Error;
pub use harvest::*;

use std::path::PathBuf;

/// A task to download an image from a URL to a file.
#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub root_dir: PathBuf,
    pub subdir: PathBuf,
    pub filename: PathBuf,
    pub image_id: i32,
}

/// Local image means an already downloaded image on disk.
/// But the image dimension may be not yet known.
/// NOTE: All paths are relative to the root directory
#[derive(Debug, Clone)]
pub struct LocalImage {
    pub filename: String,
    pub relpath: String,
    pub thumbnail_relpath: Option<String>,
    pub small_thumbnail_relpath: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub size: u64,
}
