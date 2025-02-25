use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use bottle_util::iso8601;

// MARK: User and Account

#[derive(Deserialize, Serialize, Debug)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: Account,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Account {
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    pub name: String,
    #[serde(rename = "account")]
    pub username: String,
    pub mail_address: String,
    pub is_premium: bool,
    pub profile_image_urls: ProfileImageUrls,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    #[serde(rename = "account")]
    pub username: String,
    pub profile_image_urls: ProfileImageUrls,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserDetail {
    pub user: User,
    pub profile: Profile,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProfileImageUrls {
    #[serde(alias = "px_16x16")]
    pub small: Option<String>,
    #[serde(alias = "px_50x50")]
    pub medium: Option<String>,
    #[serde(alias = "px_170x170")]
    pub large: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Profile {
    pub webpage: Option<String>,
    pub total_follow_users: u32,
    pub total_mypixiv_users: u32,
    pub total_illusts: u32,
    pub total_manga: u32,
    pub total_novels: u32,
    pub total_illust_bookmarks_public: u32,
    pub total_illust_series: u32,
    pub background_image_url: Option<String>,
    pub twitter_account: String,
    pub twitter_url: Option<String>,
    pub pawoo_url: Option<String>,
    pub is_premium: bool,
    pub is_using_custom_profile_image: bool,
}

// MARK: Illust

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Illust {
    pub id: u64,
    pub title: String,
    pub caption: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub restrict: u8,

    pub user: User,
    pub tags: Vec<Tag>,
    #[serde(with = "iso8601")]
    pub create_date: DateTime<Utc>,
    pub page_count: u32,
    pub width: u32,
    pub height: u32,
    pub sanity_level: u8,

    pub image_urls: ImageUrls,
    pub meta_single_page: MetaSinglePage,
    pub meta_pages: Vec<MetaPage>,
    pub series: Option<Series>,

    pub total_view: u32,
    pub total_bookmarks: u32,
    pub is_bookmarked: bool,
    #[serde(default)]
    pub illust_ai_type: u8,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MetaSinglePage {
    pub original_image_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MetaPage {
    pub image_urls: ImageUrls,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ImageUrls {
    pub square_medium: String,
    pub medium: String,
    pub large: String,
    pub original: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Series {
    pub id: u64,
    pub title: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub translated_name: Option<String>,
}

// MARK: Ugoira

#[derive(Deserialize, Serialize, Debug)]
pub struct Ugoira {
    pub ugoira_metadata: UgoiraMetadata,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UgoiraMetadata {
    pub zip_urls: ZipUrls,
    pub frames: Vec<Frame>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ZipUrls {
    pub medium: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Frame {
    pub file: String,
    pub delay: u32,
}

// MARK: Response

#[derive(Deserialize, Serialize, Debug)]
pub struct IllustList {
    pub illusts: Vec<Illust>,
    pub next_url: Option<String>,
    pub search_span_limit: Option<u32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookmarkDetail {
    pub bookmark_detail: BookmarkDetailInner,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookmarkDetailInner {
    pub is_bookmarked: bool,
    pub tags: Vec<Tag>,
    pub restrict: u8,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookmarkTagList {
    pub bookmark_tags: Vec<BookmarkTag>,
    pub next_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookmarkTag {
    pub name: String,
    pub count: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UserList {
    pub user_previews: Vec<UserPreview>,
    pub next_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UserPreview {
    pub user: User,
    pub illusts: Vec<Illust>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FollowDetail {
    pub follow_detail: FollowDetailInner,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FollowDetailInner {
    pub is_followed: bool,
    pub restrict: u8,
}
