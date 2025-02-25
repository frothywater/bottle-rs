// Database model definitions.
//
// Notes: Didn't involve much association feature here, like `belongs_to`,
// since I usually directly build the query instead of starting from a parent object.

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;

use bottle_core::schema::*;

// MARK: Library

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = work)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Work {
    pub id: i32,
    pub source: Option<String>,
    pub post_id: Option<String>,
    pub post_id_int: Option<i64>,
    /// If the work represents one single image within some post, this field records the index of the image within the post.
    /// Or if the work represents a whole post, this field is None.
    pub page_index: Option<i32>,
    pub as_archive: bool,
    pub image_count: i32,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub favorite: bool,
    pub rating: i32,
    pub thumbnail_path: Option<String>,
    pub small_thumbnail_path: Option<String>,
    pub added_date: NaiveDateTime,
    pub modified_date: NaiveDateTime,
    pub viewed_date: Option<NaiveDateTime>,
}

#[derive(Insertable, Debug, Clone, Default)]
#[diesel(table_name = work)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewWork {
    pub source: Option<String>,
    pub post_id: Option<String>,
    pub post_id_int: Option<i64>,
    /// If the work represents one single image within some post, this field records the index of the image within the post.
    /// Or if the work represents a whole post, this field is None.
    pub page_index: Option<i32>,
    pub as_archive: bool,
    pub image_count: i32,
    pub name: Option<String>,
    pub caption: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone, Serialize)]
#[diesel(table_name = image)]
#[diesel(belongs_to(Work))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Image {
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

#[derive(Insertable, Debug, Clone, Default)]
#[diesel(table_name = image)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewImage {
    pub work_id: i32,
    /// If the work which the image belongs to already represents one single image within some post, this field is None.
    pub page_index: Option<i32>,
    pub filename: String,
    pub remote_url: Option<String>,
    pub path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: Option<i32>,
}

#[derive(AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = image)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ImageUpdate {
    pub path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub small_thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub size: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = album)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Album {
    pub id: i32,
    pub name: String,
    pub folder_id: Option<i32>,
    pub position: i32,
    pub added_date: NaiveDateTime,
    pub modified_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone, Default)]
#[diesel(table_name = album)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewAlbum {
    pub name: String,
    pub folder_id: Option<i32>,
    pub position: i32,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = album)]
#[diesel(treat_none_as_null = true)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AlbumUpdate {
    pub folder_id: Option<i32>,
    pub position: i32,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = album_work)]
#[diesel(primary_key(album_id, work_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AlbumWork {
    pub album_id: i32,
    pub work_id: i32,
    pub position: i32,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Serialize)]
#[diesel(table_name = folder)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Folder {
    pub id: i32,
    pub name: String,
    pub parent_id: Option<i32>,
    pub position: i32,
    pub added_date: NaiveDateTime,
    pub modified_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone, Default)]
#[diesel(table_name = folder)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewFolder {
    pub name: String,
    pub parent_id: Option<i32>,
    pub position: i32,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = folder)]
#[diesel(treat_none_as_null = true)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FolderUpdate {
    pub parent_id: Option<i32>,
    pub position: i32,
}
