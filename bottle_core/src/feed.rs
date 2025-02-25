// The abstract feed interface for all communities.
// Feeds are the main interface to fetch and manage content from communities.
// This is the core part of the app.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::error::Result;
use crate::library::{ImageView, WorkView};

pub type Database<'a> = &'a mut diesel::SqliteConnection;

// MARK: Traits

/// A community is any web service that provides image posts, like Twitter or Pixiv.
/// Bottle can support many communities as a form of plugins.
/// Each community provides several kinds of feed, like home timeline, user timeline, or search results.
/// User can subscribe to a feed, track its posts, and add them to the library.
/// User can login to multiple accounts of the same community, and each account has its own feeds.
/// Note: Artist is not considered as a mandatory part of a community yet, because some communities don't have artists.
pub trait Community {
    type Auth;
    type Credential;
    type Account: Account<Auth = Self::Auth, Credential = Self::Credential>;
    type Feed: Feed<Auth = Self::Auth, Credential = Self::Credential>;

    fn metadata() -> CommunityMetadata
    where
        Self: Sized;
    fn use_account() -> bool
    where
        Self: Sized,
    {
        Self::metadata().account.is_some()
    }
}

/// An account is used for communities that require user to login for browsing content.
#[async_trait]
pub trait Account {
    /// The authentication information used for fetching content, like access token.
    type Auth;
    /// The credential information used for login, like username, password or cookies.
    type Credential;
    /// The raw account information fetched from the community, like username, avatar, etc.
    type InfoResponse;

    /// Static metadata of account that the community requires.
    fn metadata() -> Option<AccountMetadata>
    where
        Self: Sized;

    /// Prepare an account for the client.
    fn view(&self) -> AccountView;

    /// Get the account information.
    fn info(&self) -> Option<AccountInfo>;

    /// Check whether the account information has been fetched.
    fn fetched_info(&self) -> bool {
        self.info().is_some()
    }

    /// Check whether the account has expired and need to be logged in again.
    fn expired(&self) -> bool;

    /// Get all accounts of the community in the database.
    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized;
    /// Get an account by ID in the database.
    fn get(db: Database, account_id: i32) -> Result<Option<Self>>
    where
        Self: Sized;
    /// Delete an account by ID from the database.
    fn delete(db: Database, account_id: i32) -> Result<()>
    where
        Self: Sized;
    /// Add an account to the database.
    fn add(db: Database, credential: &Self::Credential) -> Result<Self>
    where
        Self: Sized;
    /// Update an account in the database.
    fn update(&self, db: Database, info: &Self::InfoResponse) -> Result<Self>
    where
        Self: Sized;
    /// Get the authentication information of the account.
    fn auth(&self, db: Database) -> Result<Option<Self::Auth>>;
    /// Get the credential information of the account.
    fn credential(&self, db: Database) -> Result<Self::Credential>;

    /// Fetch account information from the community.
    async fn fetch(credential: &Self::Credential) -> Result<Self::InfoResponse>;
}

/// A feed is a source of posts, like home timeline, user timeline, or search results.
/// User can subscribe to a feed, track its posts, and add them to the library.
/// When Bottle performs update, it will fetch latest posts from all feeds, and user can view them later.
#[async_trait]
pub trait Feed {
    /// The parameters used for fetching posts. This should be unique for each feed.
    type Params;
    type Auth;
    type Credential;
    type Account: Account<Auth = Self::Auth, Credential = Self::Credential>;
    /// The fetched result of a feed.
    type FetchResult;
    /// The context used across a fetching process, which may performs multiple requests.
    type FetchContext;

    /// Static metadata of feeds that the community provides.
    fn metadata() -> Vec<FeedMetadata>;

    /// Prepare a feed for the client.
    fn view(&self) -> FeedView;

    /// Get all feeds of the community in the database.
    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized;
    /// Get a feed by ID in the database.
    fn get(db: Database, feed_id: i32) -> Result<Option<Self>>
    where
        Self: Sized;
    /// Delete a feed by ID from the database.
    fn delete(db: Database, feed_id: i32) -> Result<()>
    where
        Self: Sized;
    /// Add a feed to the database.
    fn add(db: Database, params: &Self::Params, info: &FeedInfo, account_id: Option<i32>) -> Result<Self>
    where
        Self: Sized;
    /// Modify feed's general information in the database.
    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView>;

    /// Callback before fetching posts.
    fn handle_before_update(&self, db: Database) -> Result<()>;
    /// Get the account of the feed.
    fn get_account(&self, db: Database) -> Result<Self::Account>;
    /// Get the context of the feed.
    fn get_fetch_context(&self, db: Database) -> Result<Self::FetchContext>;
    /// Fetch posts from the feed with given context and authentication.
    async fn fetch(&self, ctx: &mut Self::FetchContext, auth: Option<&Self::Auth>) -> Result<Self::FetchResult>;
    /// Save fetched posts to the database.
    fn save(&self, db: Database, fetched: &Self::FetchResult, ctx: &Self::FetchContext) -> Result<SaveResult>;
    /// Callback after fetching posts, with the save results.
    fn handle_after_update<'a>(
        &self,
        db: Database,
        save_results: impl IntoIterator<Item = &'a SaveResult>,
    ) -> Result<()>;

    /// Get all the posts of the feed in the database.
    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse>;

    /// Get all the posts in the community's library. Static function.
    fn archived_posts(db: Database, page: i64, page_size: i64) -> Result<GeneralResponse>
    where
        Self: Sized;

    /// Get all the artists appeared in the community's library, if the community supports. Static function.
    fn archived_posts_grouped_by_user(
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse>
    where
        Self: Sized;

    /// Get all the posts of an artist in the community's library, if the community supports. Static function.
    fn archived_posts_by_user(db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse>
    where
        Self: Sized;

    /// Get all the artists appeared in the feed, if the community supports.
    fn feed_posts_grouped_by_user(
        &self,
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse>;

    /// Get all the posts of an artist in the feed, if the community supports.
    fn feed_posts_by_user(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse>;
}

/// A post is a piece of content containing one or more images, like a tweet or a Pixiv illustration.
pub trait Post {
    type Cache;

    /// Get the post by ID in the database.
    fn get(db: Database, cache: &Self::Cache, post_id: &str) -> Result<Option<Self>>
    where
        Self: Sized;

    /// Add the post to the library as a work.
    /// The `page` parameter indicates which page of the post should be added.
    /// If `page` is None, the post will be added as a whole with all pages.
    fn add_to_library(&self, db: Database, page: Option<i32>) -> Result<GeneralResponse>;
}

// MARK: Entities

/// Scheme is used to recursively define a object structure,
/// particularly a feed parameter or an account credential for now.
/// The schemes can be sent to the client via metadata,
/// and the client can create a new feed or account according to the scheme.
#[derive(Debug, Clone, Serialize)]
pub enum Scheme {
    Null,
    Bool,
    Int,
    Bigint,
    Double,
    String,
    Optional(Box<Scheme>),
    Array(Box<Scheme>),
    Object(HashMap<String, Scheme>),
}

/// Metadata of a community.
/// If the community doesn't require an account to work, `account` can be None.
#[derive(Debug, Clone, Serialize)]
pub struct CommunityMetadata {
    pub name: String,
    pub feeds: Vec<FeedMetadata>,
    pub account: Option<AccountMetadata>,
}

/// Metadata of an account.
#[derive(Debug, Clone, Serialize)]
pub struct AccountMetadata {
    /// Scheme of the account credential.
    pub credential_scheme: Scheme,
    /// Indicates whether Bottle can fetch detailed account info from the community using `Account::fetch()`.
    pub can_fetch_info: bool,
    /// Indicates whether the account can be expired after some time and need to be logged in again.
    pub need_refresh: bool,
}

/// Metadata of a feed.
#[derive(Debug, Clone, Serialize)]
pub struct FeedMetadata {
    pub name: String,
    pub scheme: Scheme,
    // TODO: Provide default params
    /// Indicates whether fetching the feed requires authentication.
    pub need_auth: bool,
}

/// App response of an account.
#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub account_id: i32,
    pub community: String,
}

/// Account information in the database processed from the raw data from community.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// App response of a feed.
#[derive(Debug, Clone, Serialize)]
pub struct FeedView {
    pub feed_id: i32,
    pub community: String,
    pub name: Option<String>,
    pub description: String,
    pub watching: bool,
}

/// General information needed to create or modify a feed.
#[derive(Debug, Clone, Deserialize)]
pub struct FeedInfo {
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
}

/// App response of a post.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PostView {
    pub post_id: String,
    pub community: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    pub created_date: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// App response of a media.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MediaView {
    pub media_id: String,
    pub community: String,
    pub post_id: String,
    pub page_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// App response of an artist.
#[derive(Debug, Clone, Serialize, Default)]
pub struct UserView {
    pub user_id: String,
    pub community: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_count: Option<i64>,
}

/// Generic response.
#[derive(Debug, Clone, Default, Serialize)]
pub struct GeneralResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts: Option<Vec<PostView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<MediaView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<Vec<UserView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub works: Option<Vec<WorkView>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<ImageView>>,
    pub total_items: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EndpointRequest<Params> {
    pub params: Params,
    /// Page, offset or cursor of the current request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EndpointResponse {
    pub posts: Vec<PostView>,
    pub media: Vec<MediaView>,
    pub users: Vec<UserView>,
    pub works: Vec<WorkView>,
    pub images: Vec<ImageView>,
    pub reached_end: bool,
    /// Page, offset or cursor for the next request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_items: Option<i64>,
}

/// Result of saved posts to the database when updating a feed,
/// with extra information like whether to stop updating and whether the feed has reached the end.
pub struct SaveResult {
    pub post_ids: Vec<String>,
    pub should_stop: bool,
    pub reached_end: bool,
}
