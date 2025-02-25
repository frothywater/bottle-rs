use chrono::NaiveDateTime;
use diesel::prelude::*;

use bottle_core::schema::*;

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
    pub description: String,
    pub url: Option<String>,
    pub pawoo_url: Option<String>,
    pub twitter_username: Option<String>,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPixivUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
    pub description: String,
    pub url: Option<String>,
    pub pawoo_url: Option<String>,
    pub twitter_username: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivAccount {
    pub id: i32,
    pub refresh_token: String,
    pub access_token: Option<String>,
    pub expiry: Option<NaiveDateTime>,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPixivAccount {
    pub refresh_token: String,
}

#[derive(AsChangeset, Debug, Clone, Default)]
#[diesel(table_name = pixiv_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivAccountUpdate {
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    pub expiry: Option<NaiveDateTime>,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = pixiv_illust)]
#[diesel(belongs_to(PixivUser, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivIllust {
    pub id: i64,
    pub user_id: i64,
    pub type_: String,
    pub title: String,
    pub caption: String,
    pub restrict: bool,
    pub sanity_level: i32,
    pub series_id: Option<i64>,
    pub series_title: Option<String>,
    pub thumbnail_url: String,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_illust)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPixivIllust {
    pub id: i64,
    pub user_id: i64,
    pub type_: String,
    pub title: String,
    pub caption: String,
    pub restrict: bool,
    pub sanity_level: i32,
    pub series_id: Option<i64>,
    pub series_title: Option<String>,
    pub thumbnail_url: String,
    pub created_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_media)]
#[diesel(primary_key(illust_id, page))]
#[diesel(belongs_to(PixivIllust, foreign_key = illust_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivMedia {
    pub illust_id: i64,
    pub page: i32,
    pub square_medium_url: String,
    pub medium_url: String,
    pub large_url: String,
    pub original_url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Associations, Debug, Clone)]
#[diesel(table_name = pixiv_illust_tag)]
#[diesel(primary_key(illust_id, tag))]
#[diesel(belongs_to(PixivIllust, foreign_key = illust_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivIllustTag {
    pub illust_id: i64,
    pub tag: String,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = pixiv_watch_list)]
#[diesel(belongs_to(PixivAccount, foreign_key = account_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivWatchList {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub restriction: Option<String>,
    pub user_id: Option<i64>,
    pub search_query: Option<String>,
    pub bookmark_tag: Option<String>,
    pub illust_type: Option<String>,
    pub reached_end: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPixivWatchList {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub restriction: Option<String>,
    pub user_id: Option<i64>,
    pub search_query: Option<String>,
    pub bookmark_tag: Option<String>,
    pub illust_type: Option<String>,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = pixiv_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivWatchListUpdate {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, Debug, Clone)]
#[diesel(primary_key(watch_list_id, illust_id))]
#[diesel(table_name = pixiv_watch_list_illust)]
#[diesel(belongs_to(PixivWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivWatchListIllust {
    pub watch_list_id: i32,
    pub illust_id: i64,
    pub private_bookmark: bool,
    pub stale: bool,
    pub sort_index: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = pixiv_watch_list_history)]
#[diesel(belongs_to(PixivWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PixivWatchListHistory {
    pub id: i32,
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub next_bookmark_id: Option<i64>,
    pub updated_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = pixiv_watch_list_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewPixivWatchListHistory {
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub next_bookmark_id: Option<i64>,
}
