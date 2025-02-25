// The Library part of Bottle.
// Many bare functions here mainly to operate Work and images.

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Remote work means a work that is not downloaded yet.
/// It can be earlier fetched by a community plugin, or manually added by the user.
/// If the work is manually added, most fields can be None.
#[derive(Debug, Clone, Default)]
pub struct RemoteWork {
    pub source: Option<String>,
    pub post_id: Option<String>,
    pub post_id_int: Option<i64>,
    pub page_index: Option<i32>,
    pub media_count: i32,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub images: Vec<RemoteImage>,
}

impl std::fmt::Display for RemoteWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Post {}{}{}",
            self.post_id
                .clone()
                .or(self.post_id_int.map(|i| i.to_string()))
                .unwrap_or_default(),
            self.page_index.map(|i| format!(":{}", i)).unwrap_or_default(),
            self.source.as_ref().map(|s| format!("@{}", s)).unwrap_or_default()
        )
    }
}

/// Remote image means an image of a work that is not downloaded yet.
/// It can be earlier fetched by a community plugin, or manually added by the user.
#[derive(Debug, Clone)]
pub struct RemoteImage {
    pub filename: String,
    pub url: String,
    /// If the work which the image belongs to already represents one single image within some post, this field is None.
    pub page_index: Option<i32>,
}

/// A unified app response of a work.
#[derive(Debug, Clone, Serialize)]
pub struct WorkView {
    pub id: i32,
    pub community: Option<String>,
    pub post_id: Option<String>,
    /// If the work represents one single image within some post, this field records the index of the image within the post.
    /// Or if the work represents a whole post, this field is None.
    pub page_index: Option<i32>,
    pub as_archive: bool,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub favorite: bool,
    pub rating: i32,
    pub thumbnail_path: Option<String>,
    pub small_thumbnail_path: Option<String>,
    pub added_date: DateTime<Utc>,
    pub modified_date: DateTime<Utc>,
    pub viewed_date: Option<DateTime<Utc>>,
}

/// A unified app response of an image.
#[derive(Debug, Clone, Serialize)]
pub struct ImageView {
    pub id: i32,
    pub work_id: i32,
    /// If the work which the image belongs to already represents one single image within some post, this field is None.
    pub page_index: Option<i32>,
    pub filename: String,
    pub remote_url: Option<String>,
    pub path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub small_thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: Option<i32>,
}

/// A unified app response of an album.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumView {
    pub id: i32,
    pub name: String,
    pub folder_id: Option<i32>,
    pub position: i32,
    pub added_date: DateTime<Utc>,
    pub modified_date: DateTime<Utc>,
}

/// A unified app response of a folder.
#[derive(Debug, Clone, Serialize)]
pub struct FolderView {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub position: i32,
    pub added_date: DateTime<Utc>,
    pub modified_date: DateTime<Utc>,
}
