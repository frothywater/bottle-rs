use chrono::NaiveDateTime;
use diesel::prelude::*;

use bottle_core::schema::*;

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = twitter_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
    pub description: String,
    pub url: Option<String>,
    pub location: String,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = twitter_user)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTwitterUser {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: Option<String>,
    pub description: String,
    pub url: Option<String>,
    pub location: String,
    pub created_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Insertable, Debug, Clone)]
#[diesel(table_name = twitter_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterAccount {
    pub id: i32,
    pub cookies: String,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = twitter_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTwitterAccount {
    pub cookies: String,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = twitter_account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterAccountUpdate {
    pub user_id: i64,
    pub name: String,
    pub username: String,
    pub profile_image_url: String,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = tweet)]
#[diesel(belongs_to(TwitterUser, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Tweet {
    pub id: i64,
    pub user_id: i64,
    pub caption: String,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = tweet)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTweet {
    pub id: i64,
    pub user_id: i64,
    pub caption: String,
    pub created_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = twitter_media)]
#[diesel(belongs_to(Tweet))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterMedia {
    pub id: String,
    pub tweet_id: i64,
    pub type_: String,
    pub url: String,
    pub width: i32,
    pub height: i32,
    pub preview_image_url: Option<String>,
    pub duration: Option<i32>,
    pub page: i32,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = twitter_list)]
#[diesel(belongs_to(TwitterUser, foreign_key = user_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterList {
    pub id: i64,
    pub name: String,
    pub user_id: i64,
    pub description: String,
    pub member_count: i32,
    pub private: bool,
    pub created_date: NaiveDateTime,
    pub added_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = twitter_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTwitterList {
    pub id: i64,
    pub name: String,
    pub user_id: i64,
    pub description: String,
    pub member_count: i32,
    pub private: bool,
    pub created_date: NaiveDateTime,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, Debug, Clone)]
#[diesel(table_name = twitter_list_member)]
#[diesel(primary_key(list_id, user_id))]
#[diesel(belongs_to(TwitterList, foreign_key = list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterListMember {
    pub list_id: i64,
    pub user_id: i64,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = twitter_watch_list)]
#[diesel(belongs_to(TwitterAccount, foreign_key = account_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterWatchList {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub twitter_list_id: Option<i64>,
    pub user_id: Option<i64>,
    pub search_query: Option<String>,
    pub reached_end: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = twitter_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTwitterWatchList {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub kind: String,
    pub twitter_list_id: Option<i64>,
    pub user_id: Option<i64>,
    pub search_query: Option<String>,
}

#[derive(AsChangeset, Debug, Clone)]
#[diesel(table_name = twitter_watch_list)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterWatchListUpdate {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Insertable, Debug, Clone)]
#[diesel(primary_key(watch_list_id, tweet_id))]
#[diesel(table_name = twitter_watch_list_tweet)]
#[diesel(belongs_to(TwitterWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterWatchListTweet {
    pub watch_list_id: i32,
    pub tweet_id: i64,
    pub sort_index: Option<i64>,
    pub stale: bool,
}

#[derive(Queryable, Selectable, Identifiable, Associations, Debug, Clone)]
#[diesel(table_name = twitter_watch_list_history)]
#[diesel(belongs_to(TwitterWatchList, foreign_key = watch_list_id))]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct TwitterWatchListHistory {
    pub id: i32,
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub top_cursor: Option<String>,
    pub top_sort_index: Option<i64>,
    pub bottom_cursor: Option<String>,
    pub bottom_sort_index: Option<i64>,
    pub updated_date: NaiveDateTime,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = twitter_watch_list_history)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewTwitterWatchListHistory {
    pub watch_list_id: i32,
    pub ids: String,
    pub count: i32,
    pub top_cursor: Option<String>,
    pub top_sort_index: Option<i64>,
    pub bottom_cursor: Option<String>,
    pub bottom_sort_index: Option<i64>,
}
