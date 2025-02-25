use chrono::NaiveDateTime;
use diesel::prelude::*;

use bottle_core::schema::*;

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = yandere_post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YanderePost {
    pub id: i64,
    pub tags: String,
    pub creator_id: Option<i64>,
    pub author: String,
    pub url: String,
    pub thumbnail_url: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub file_ext: String,
    pub rating: String,
    pub md5: String,
    pub source: String,
    pub has_children: bool,
    pub parent_id: Option<i64>,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = yandere_post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewYanderePost {
    pub id: i64,
    pub tags: String,
    pub creator_id: Option<i64>,
    pub author: String,
    pub url: String,
    pub thumbnail_url: String,
    pub width: i32,
    pub height: i32,
    pub file_size: i64,
    pub file_ext: String,
    pub rating: String,
    pub md5: String,
    pub source: String,
    pub has_children: bool,
    pub parent_id: Option<i64>,
    pub created_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(table_name = yandere_pool)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YanderePool {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub user_id: i64,
    pub post_count: i32,
    pub created_date: NaiveDateTime,
    pub updated_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(primary_key(pool_id, post_id))]
#[diesel(table_name = yandere_pool_post)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YanderePoolPost {
    pub pool_id: i64,
    pub post_id: i64,
    pub sequence: String,
    pub prev_post_id: Option<i64>,
    pub next_post_id: Option<i64>,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(primary_key(name))]
#[diesel(table_name = yandere_tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YandereTag {
    pub name: String,
    pub type_: String,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Debug, Clone)]
#[diesel(primary_key(post_id, tag_name))]
#[diesel(table_name = yandere_post_tag)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YanderePostTag {
    pub post_id: i64,
    pub tag_name: String,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone)]
#[diesel(table_name = yandere_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YandereWatchList {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub kind: String,
    pub search_query: Option<String>,
    pub pool_id: Option<i32>,
    pub reached_end: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = yandere_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewYandereWatchList {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub kind: String,
    pub search_query: Option<String>,
    pub pool_id: Option<i32>,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = yandere_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YandereWatchListUpdate {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = yandere_watch_list_history)]
#[diesel(belongs_to(YandereWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YandereWatchListHistory {
    pub id: i32,
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub updated_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = yandere_watch_list_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewYandereWatchListHistory {
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
}

#[derive(Queryable, Selectable, Insertable, Identifiable, Associations, Debug, Clone)]
#[diesel(primary_key(watch_list_id, post_id))]
#[diesel(table_name = yandere_watch_list_post)]
#[diesel(belongs_to(YandereWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct YandereWatchListPost {
    pub watch_list_id: i32,
    pub post_id: i64,
    pub sort_index: Option<i32>,
}
