use chrono::NaiveDateTime;
use diesel::prelude::*;

use bottle_core::schema::*;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = panda_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaAccount {
    pub id: i32,
    pub cookies: String,
    pub name: Option<String>,
    pub username: Option<String>,
}

#[derive(Insertable, Debug, Clone, Default)]
#[diesel(table_name = panda_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPandaAccount {
    pub cookies: String,
    pub name: Option<String>,
    pub username: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, Default)]
#[diesel(table_name = panda_gallery)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaGallery {
    pub id: i64,
    pub token: String,
    pub title: String,
    pub thumbnail_url: String,
    pub category: i32,
    pub uploader: String,
    pub rating: f32,
    pub media_count: i32,
    pub english_title: Option<String>,
    pub parent: Option<String>,
    pub visible: Option<bool>,
    pub language: Option<String>,
    pub file_size: Option<i32>,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
    pub stale: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = panda_gallery)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPandaGallery {
    pub id: i64,
    pub token: String,
    pub title: String,
    pub thumbnail_url: String,
    pub category: i32,
    pub uploader: String,
    pub rating: f32,
    pub media_count: i32,
    pub created_date: NaiveDateTime,
}

#[derive(AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = panda_gallery)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaGalleryUpdate {
    pub media_count: Option<i32>,
    pub english_title: Option<String>,
    pub rating: Option<f32>,
    pub parent: Option<String>,
    pub visible: Option<bool>,
    pub language: Option<String>,
    pub file_size: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone, Default)]
#[diesel(primary_key(gallery_id, media_index))]
#[diesel(table_name = panda_media)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaMedia {
    pub gallery_id: i64,
    pub media_index: i32,
    pub token: String,
    pub thumbnail_url: Option<String>,
    pub url: Option<String>,
    pub filename: Option<String>,
    pub file_size: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = panda_media)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaMediaUpdate {
    pub url: Option<String>,
    pub filename: Option<String>,
    pub file_size: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = panda_tag)]
#[diesel(primary_key(namespace, name))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaTag {
    pub namespace: String,
    pub name: String,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(primary_key(gallery_id, namespace, name))]
#[diesel(table_name = panda_gallery_tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaGalleryTag {
    pub gallery_id: i64,
    pub namespace: String,
    pub name: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = panda_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaWatchList {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub query: Option<String>,
    pub reached_end: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = panda_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPandaWatchList {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub query: Option<String>,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = panda_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaWatchListUpdate {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(primary_key(watch_list_id, gallery_id))]
#[diesel(table_name = panda_watch_list_gallery)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaWatchListGallery {
    pub watch_list_id: i32,
    pub gallery_id: i64,
    pub sort_index: Option<i32>,
    pub stale: bool,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = panda_watch_list_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PandaWatchListHistory {
    pub id: i32,
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub prev_offset: Option<String>,
    pub next_offset: Option<String>,
    pub updated_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = panda_watch_list_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPandaWatchListHistory {
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub prev_offset: Option<String>,
    pub next_offset: Option<String>,
}
