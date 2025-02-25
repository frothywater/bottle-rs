use std::{collections::HashMap, future::Future, result::Result, time::Duration};

use diesel::{connection::SimpleConnection, SqliteConnection};

use bottle_core::{feed::*, Database, Error as BottleError, Result as BottleResult};
use bottle_panda::*;
use bottle_pixiv::*;
use bottle_twitter::*;
use bottle_yandere::*;

use crate::{
    error::ServerError,
    payload::{FeedParams, NewFeedRequest},
};

pub const DEFAULT_PAGE_SIZE: i64 = 30;
pub const DEFAULT_RECENT_COUNT: i64 = 10;
pub const DEFAULT_TIMEOUT_MS: u64 = 30000;
pub const DEFAULT_RETRY_DELAY_MS: u64 = 1000;
pub const DEFAULT_RETRY_COUNT: usize = 5;

pub fn get_page_and_size(params: &HashMap<String, String>) -> (i64, i64) {
    let page = params.get("page").and_then(|p| p.parse::<i64>().ok()).unwrap_or(0);
    let page_size = params
        .get("page_size")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(DEFAULT_PAGE_SIZE);
    (page, page_size)
}

pub fn timeout<T, E: Into<ServerError>>(
    f: impl Future<Output = Result<T, E>>,
) -> impl Future<Output = Result<T, ServerError>> {
    use futures::FutureExt;
    tokio::time::timeout(std::time::Duration::from_millis(DEFAULT_TIMEOUT_MS), f).map(move |result| {
        result
            .map(|r| r.map_err(Into::into))
            .unwrap_or_else(|_| Err(BottleError::Timeout(format!("after {} ms", DEFAULT_TIMEOUT_MS)).into()))
    })
}

pub fn retry<R, T: Future<Output = Result<R, ServerError>>, F: FnMut() -> T>(
    f: F,
) -> impl Future<Output = Result<R, ServerError>> {
    use tokio_retry::{strategy::FixedInterval, RetryIf};
    let strategy = FixedInterval::from_millis(DEFAULT_RETRY_DELAY_MS).take(DEFAULT_RETRY_COUNT);
    RetryIf::spawn(strategy, f, |e: &ServerError| e.retryable())
}

// MARK: Database

/// https://stackoverflow.com/questions/57123453/how-to-use-diesel-with-sqlite-connections-and-avoid-database-is-locked-type-of
#[derive(Debug)]
pub struct ConnectionOptions {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for ConnectionOptions {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            if self.enable_wal {
                conn.batch_execute(
                    "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
                )?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

// MARK: Polymorphic wrapper for feeds

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FeedIdentifier {
    pub community: String,
    pub feed_id: i32,
}

impl std::fmt::Display for FeedIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.feed_id, self.community)
    }
}

impl FeedIdentifier {
    pub fn new(community: &str, feed_id: i32) -> Self {
        Self {
            community: community.to_string(),
            feed_id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FeedWrapper {
    Twitter(TwitterFeed),
    Pixiv(PixivFeed),
    Yandere(YandereFeed),
    Panda(PandaFeed),
}

#[derive(Debug, Clone)]
pub enum FeedContextWrapper {
    Twitter {
        auth: Option<<TwitterFeed as Feed>::Auth>,
        context: <TwitterFeed as Feed>::FetchContext,
    },
    Pixiv {
        auth: Option<<PixivFeed as Feed>::Auth>,
        context: <PixivFeed as Feed>::FetchContext,
    },
    Yandere {
        _auth: Option<<YandereFeed as Feed>::Auth>,
        context: <YandereFeed as Feed>::FetchContext,
    },
    Panda {
        auth: Option<<PandaFeed as Feed>::Auth>,
        context: <PandaFeed as Feed>::FetchContext,
    },
}

impl FeedWrapper {
    pub fn from_id(db: Database, id: &FeedIdentifier) -> BottleResult<Self> {
        match id.community.as_str() {
            "twitter" => {
                let feed = TwitterFeed::get(db, id.feed_id)?
                    .ok_or(BottleError::ObjectNotFound(format!("Twitter Feed {}", id.feed_id)))?;
                Ok(Self::Twitter(feed))
            }
            "pixiv" => {
                let feed = PixivFeed::get(db, id.feed_id)?
                    .ok_or(BottleError::ObjectNotFound(format!("Pixiv Feed {}", id.feed_id)))?;
                Ok(Self::Pixiv(feed))
            }
            "yandere" => {
                let feed = YandereFeed::get(db, id.feed_id)?
                    .ok_or(BottleError::ObjectNotFound(format!("Yandere Feed {}", id.feed_id)))?;
                Ok(Self::Yandere(feed))
            }
            "panda" => {
                let feed = PandaFeed::get(db, id.feed_id)?
                    .ok_or(BottleError::ObjectNotFound(format!("Panda Feed {}", id.feed_id)))?;
                Ok(Self::Panda(feed))
            }
            _ => Err(BottleError::InvalidEndpoint(format!("Community {}", id.community))),
        }
    }

    pub fn all(db: Database, community: &str) -> BottleResult<Vec<Self>> {
        match community {
            "twitter" => TwitterFeed::all(db).map(|feeds| feeds.into_iter().map(Self::Twitter).collect()),
            "pixiv" => PixivFeed::all(db).map(|feeds| feeds.into_iter().map(Self::Pixiv).collect()),
            "yandere" => YandereFeed::all(db).map(|feeds| feeds.into_iter().map(Self::Yandere).collect()),
            "panda" => PandaFeed::all(db).map(|feeds| feeds.into_iter().map(Self::Panda).collect()),
            _ => Err(BottleError::InvalidEndpoint(format!("Community {}", community))),
        }
    }

    pub fn id(&self) -> FeedIdentifier {
        match self {
            Self::Twitter(feed) => FeedIdentifier::new("twitter", feed.id),
            Self::Pixiv(feed) => FeedIdentifier::new("pixiv", feed.id),
            Self::Yandere(feed) => FeedIdentifier::new("yandere", feed.id),
            Self::Panda(feed) => FeedIdentifier::new("panda", feed.id),
        }
    }

    pub fn view(&self) -> FeedView {
        match self {
            Self::Twitter(feed) => feed.view(),
            Self::Pixiv(feed) => feed.view(),
            Self::Yandere(feed) => feed.view(),
            Self::Panda(feed) => feed.view(),
        }
    }

    pub fn add(db: Database, request: &NewFeedRequest) -> BottleResult<Self> {
        match &request.params {
            FeedParams::Twitter(params) => {
                let feed = TwitterFeed::add(db, params, &request.info, request.account_id)?;
                Ok(Self::Twitter(feed))
            }
            FeedParams::Pixiv(params) => {
                let feed = PixivFeed::add(db, params, &request.info, request.account_id)?;
                Ok(Self::Pixiv(feed))
            }
            FeedParams::Yandere(params) => {
                let feed = YandereFeed::add(db, params, &request.info, request.account_id)?;
                Ok(Self::Yandere(feed))
            }
            FeedParams::Panda(params) => {
                let feed = PandaFeed::add(db, params, &request.info, request.account_id)?;
                Ok(Self::Panda(feed))
            }
        }
    }

    pub fn delete(db: Database, id: &FeedIdentifier) -> BottleResult<()> {
        match id.community.as_str() {
            "twitter" => TwitterFeed::delete(db, id.feed_id),
            "pixiv" => PixivFeed::delete(db, id.feed_id),
            "yandere" => YandereFeed::delete(db, id.feed_id),
            "panda" => PandaFeed::delete(db, id.feed_id),
            _ => Err(BottleError::InvalidEndpoint(format!("Community {}", id.community))),
        }
    }

    pub fn modify(&mut self, db: Database, info: &FeedInfo) -> BottleResult<FeedView> {
        match self {
            Self::Twitter(feed) => feed.modify(db, info),
            Self::Pixiv(feed) => feed.modify(db, info),
            Self::Yandere(feed) => feed.modify(db, info),
            Self::Panda(feed) => feed.modify(db, info),
        }
    }

    pub fn get_context(&self, db: Database) -> BottleResult<FeedContextWrapper> {
        match self {
            Self::Twitter(feed) => {
                let account = feed.get_account(db)?;
                let auth = account.auth(db)?;
                let context = feed.get_fetch_context(db)?;
                Ok(FeedContextWrapper::Twitter { auth, context })
            }
            Self::Pixiv(feed) => {
                let account = feed.get_account(db)?;
                let auth = account.auth(db)?;
                let context = feed.get_fetch_context(db)?;
                Ok(FeedContextWrapper::Pixiv { auth, context })
            }
            Self::Yandere(feed) => {
                let context = feed.get_fetch_context(db)?;
                Ok(FeedContextWrapper::Yandere { _auth: None, context })
            }
            Self::Panda(feed) => {
                let account = feed.get_account(db)?;
                let auth = account.auth(db)?;
                let context = feed.get_fetch_context(db)?;
                Ok(FeedContextWrapper::Panda { auth, context })
            }
        }
    }

    pub async fn refresh_account<'a>(&self, db: Database<'a>) -> BottleResult<()> {
        if let Self::Pixiv(feed) = self {
            let account = feed.get_account(db)?;
            if account.expired() {
                let credential = account.credential(db)?;
                let info = PixivAccount::fetch(&credential).await?;
                let account = account.update(db, &info)?;
                tracing::info!("Refreshed Pixiv account {}", account.id);
            }
        }
        Ok(())
    }

    pub async fn fetch_and_save<'a>(
        &self,
        db: Database<'a>,
        context: &mut FeedContextWrapper,
    ) -> BottleResult<SaveResult> {
        match self {
            Self::Twitter(feed) => {
                let (auth, ctx) = match context {
                    FeedContextWrapper::Twitter { auth, context } => (auth, context),
                    _ => unreachable!(),
                };
                let tweets = feed.fetch(ctx, auth.as_ref()).await?;
                let result = feed.save(db, &tweets, ctx)?;
                Ok(result)
            }
            Self::Pixiv(feed) => {
                let (auth, ctx) = match context {
                    FeedContextWrapper::Pixiv { auth, context } => (auth, context),
                    _ => unreachable!(),
                };
                let illusts = feed.fetch(ctx, auth.as_ref()).await?;
                let result = feed.save(db, &illusts, ctx)?;
                Ok(result)
            }
            Self::Yandere(feed) => {
                let ctx = match context {
                    FeedContextWrapper::Yandere { _auth: _, context } => context,
                    _ => unreachable!(),
                };
                let posts = feed.fetch(ctx, None).await?;
                let result = feed.save(db, &posts, ctx)?;
                Ok(result)
            }
            Self::Panda(feed) => {
                let (auth, ctx) = match context {
                    FeedContextWrapper::Panda { auth, context } => (auth, context),
                    _ => unreachable!(),
                };
                let galleries = feed.fetch(ctx, auth.as_ref()).await?;
                let result = feed.save(db, &galleries, ctx)?;
                Ok(result)
            }
        }
    }

    pub fn handle_before_update(&self, db: Database) -> BottleResult<()> {
        match self {
            Self::Pixiv(feed) => feed.handle_before_update(db),
            Self::Yandere(feed) => feed.handle_before_update(db),
            Self::Panda(feed) => feed.handle_before_update(db),
            _ => Ok(()),
        }
    }

    pub fn handle_after_update<'a>(
        &self,
        db: Database,
        results: impl Iterator<Item = &'a SaveResult>,
    ) -> BottleResult<()> {
        match self {
            Self::Pixiv(feed) => feed.handle_after_update(db, results),
            Self::Yandere(feed) => feed.handle_after_update(db, results),
            Self::Panda(feed) => feed.handle_after_update(db, results),
            _ => Ok(()),
        }
    }

    pub fn posts(&self, db: Database, page: i64, page_size: i64) -> BottleResult<GeneralResponse> {
        match self {
            Self::Twitter(feed) => feed.posts(db, page, page_size),
            Self::Pixiv(feed) => feed.posts(db, page, page_size),
            Self::Yandere(feed) => feed.posts(db, page, page_size),
            Self::Panda(feed) => feed.posts(db, page, page_size),
        }
    }

    pub fn users(&self, db: Database, page: i64, page_size: i64, recent_count: i64) -> BottleResult<GeneralResponse> {
        match self {
            Self::Twitter(feed) => feed.feed_posts_grouped_by_user(db, page, page_size, recent_count),
            Self::Pixiv(feed) => feed.feed_posts_grouped_by_user(db, page, page_size, recent_count),
            Self::Yandere(feed) => feed.feed_posts_grouped_by_user(db, page, page_size, recent_count),
            Self::Panda(feed) => feed.feed_posts_grouped_by_user(db, page, page_size, recent_count),
        }
    }

    pub fn user_posts(
        &self,
        db: Database,
        user_id: String,
        page: i64,
        page_size: i64,
    ) -> BottleResult<GeneralResponse> {
        match self {
            Self::Twitter(feed) => feed.feed_posts_by_user(db, user_id, page, page_size),
            Self::Pixiv(feed) => feed.feed_posts_by_user(db, user_id, page, page_size),
            Self::Yandere(feed) => feed.feed_posts_by_user(db, user_id, page, page_size),
            Self::Panda(feed) => feed.feed_posts_by_user(db, user_id, page, page_size),
        }
    }
}

pub fn adding_community_entities(db: Database, response: GeneralResponse) -> BottleResult<GeneralResponse> {
    let mut users = Vec::new();
    let mut posts = Vec::new();
    let mut media = Vec::new();
    let mut add_entities = |res: GeneralResponse| {
        users.extend(res.users.unwrap_or_default());
        posts.extend(res.posts.unwrap_or_default());
        media.extend(res.media.unwrap_or_default());
    };

    let twitter_ids = response
        .works
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter(|work| work.community.as_deref() == Some("twitter"))
        .filter_map(|work| work.post_id.as_ref().and_then(|id| id.parse::<i64>().ok()))
        .collect::<Vec<_>>();
    if !twitter_ids.is_empty() {
        let twitter_response = bottle_twitter::get_entities(db, twitter_ids)?;
        add_entities(twitter_response);
    }

    let pixiv_ids = response
        .works
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .filter(|work| work.community.as_deref() == Some("pixiv"))
        .filter_map(|work| work.post_id.as_ref().and_then(|id| id.parse::<i64>().ok()))
        .collect::<Vec<_>>();
    if !pixiv_ids.is_empty() {
        let pixiv_response = bottle_pixiv::get_entities(db, pixiv_ids)?;
        add_entities(pixiv_response);
    }

    Ok(GeneralResponse {
        users: Some(users),
        posts: Some(posts),
        media: Some(media),
        ..response
    })
}
