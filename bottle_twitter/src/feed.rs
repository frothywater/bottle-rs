use async_trait::async_trait;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use bottle_core::{feed::*, Error, Result};
use twitter_client::{SessionCookie, TimelineResult, TwitterClient};

use crate::community::TwitterAccount;
use crate::{group, model, util};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TwitterFeedParams {
    Timeline,
    Bookmarks,
    Likes { user_id: i64 },
    Posts { user_id: i64 },
    List { list_id: i64 },
    Search { query: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwitterFetchContext {
    pub(crate) cursor: Option<String>,
    pub(crate) direction: Direction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct TwitterFeed {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub params: TwitterFeedParams,
    pub reached_end: bool,
}

#[async_trait]
impl Feed for TwitterFeed {
    type Params = TwitterFeedParams;
    type Auth = SessionCookie;
    type Credential = SessionCookie;
    type Account = TwitterAccount;
    type FetchResult = TimelineResult;
    type FetchContext = TwitterFetchContext;

    fn metadata() -> Vec<FeedMetadata> {
        vec![
            FeedMetadata {
                name: "timeline".to_string(),
                scheme: Scheme::Null,
                need_auth: true,
            },
            FeedMetadata {
                name: "bookmarks".to_string(),
                scheme: Scheme::Null,
                need_auth: true,
            },
            FeedMetadata {
                name: "likes".to_string(),
                scheme: Scheme::Object(HashMap::from([("user_id".to_string(), Scheme::Bigint)])),
                need_auth: true,
            },
            FeedMetadata {
                name: "posts".to_string(),
                scheme: Scheme::Object(HashMap::from([("user_id".to_string(), Scheme::Bigint)])),
                need_auth: true,
            },
            FeedMetadata {
                name: "list".to_string(),
                scheme: Scheme::Object(HashMap::from([("list_id".to_string(), Scheme::Bigint)])),
                need_auth: true,
            },
            FeedMetadata {
                name: "search".to_string(),
                scheme: Scheme::Object(HashMap::from([("query".to_string(), Scheme::String)])),
                need_auth: true,
            },
        ]
    }

    fn view(&self) -> FeedView {
        FeedView {
            feed_id: self.id,
            community: "twitter".to_string(),
            name: self.name.clone(),
            watching: self.watching,
            description: match &self.params {
                TwitterFeedParams::Timeline => "Timeline".to_string(),
                TwitterFeedParams::Bookmarks => "Bookmarks".to_string(),
                TwitterFeedParams::Likes { user_id } => format!("Likes by {}", user_id),
                TwitterFeedParams::Posts { user_id } => format!("Posts by {}", user_id),
                TwitterFeedParams::List { list_id } => format!("List {}", list_id),
                TwitterFeedParams::Search { query } => format!("Search {}", query),
            },
        }
    }

    fn all(db: Database) -> Result<Vec<Self>> {
        use bottle_core::schema::twitter_watch_list::dsl::*;
        twitter_watch_list
            .load::<model::TwitterWatchList>(db)?
            .into_iter()
            .map(Self::try_from)
            .collect()
    }

    fn get(db: Database, feed_id: i32) -> Result<Option<Self>> {
        use bottle_core::schema::twitter_watch_list::dsl::*;
        let result = twitter_watch_list
            .filter(id.eq(feed_id))
            .first::<model::TwitterWatchList>(db)
            .optional()?;
        result.map(Self::try_from).transpose()
    }

    fn delete(db: Database, feed_id: i32) -> Result<()> {
        use bottle_core::schema::twitter_watch_list::dsl::*;
        diesel::delete(twitter_watch_list.find(feed_id)).execute(db)?;
        tracing::info!("Deleted twitter feed {}", feed_id);
        Ok(())
    }

    fn add(db: Database, params: &Self::Params, info: &FeedInfo, account_id: Option<i32>) -> Result<Self> {
        use bottle_core::schema::twitter_watch_list;
        let Some(account_id) = account_id else {
            return Err(Error::NotLoggedIn("Twitter feed needs an account".to_string()));
        };
        let new_watch_list = model::NewTwitterWatchList {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
            account_id,
            kind: params.kind(),
            user_id: params.user_id(),
            twitter_list_id: params.twitter_list_id(),
            search_query: params.search_query(),
        };
        let result = diesel::insert_into(twitter_watch_list::table)
            .values(&new_watch_list)
            .get_result::<model::TwitterWatchList>(db)?;
        tracing::info!(
            "Added twitter feed {}: {:?} {:?}, account {}",
            result.id,
            params,
            info,
            account_id
        );
        Self::try_from(result)
    }

    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView> {
        use bottle_core::schema::twitter_watch_list;
        let update = model::TwitterWatchListUpdate {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
        };
        diesel::update(twitter_watch_list::table.find(self.id))
            .set(&update)
            .execute(db)?;
        self.name = info.name.clone();
        self.watching = info.watching;
        self.first_fetch_limit = info.first_fetch_limit;
        tracing::info!("Modified twitter feed {}: {:?}", self.id, info);
        Ok(self.view())
    }

    fn save(&self, db: Database, fetched: &Self::FetchResult, ctx: &Self::FetchContext) -> Result<SaveResult> {
        use bottle_core::schema::{
            tweet, twitter_media, twitter_user, twitter_watch_list, twitter_watch_list_history,
            twitter_watch_list_tweet,
        };

        // (a) If response is empty, we should stop updating
        if fetched.tweets.is_empty() {
            let mut reached_end = false;
            // (a*) If we are fetching backward, this means the feed reached end and we should mark it
            if matches!(ctx.direction, Direction::Backward) {
                diesel::update(twitter_watch_list::table.find(self.id))
                    .set(twitter_watch_list::reached_end.eq(true))
                    .execute(db)?;
                reached_end = true;
                tracing::info!("Set twitter feed {} as reached end", self.id);
            }
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end,
            });
        }

        // 1. Filter out tweets already exist in database
        let fetched_ids = fetched.tweets.iter().map(|t| t.id as i64).collect::<Vec<_>>();
        let existing_ids = twitter_watch_list_tweet::table
            .filter(twitter_watch_list_tweet::watch_list_id.eq(self.id))
            .filter(twitter_watch_list_tweet::tweet_id.eq_any(&fetched_ids))
            .select(twitter_watch_list_tweet::tweet_id)
            .load::<i64>(db)?;
        let existing_ids = existing_ids.into_iter().map(|id| id as u64).collect::<HashSet<_>>();
        let tweets = fetched.tweets.iter().filter(|t| !existing_ids.contains(&t.id));

        // (b) If all tweets already exist in database, we should stop updating
        if tweets.clone().count() == 0 {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end: false,
            });
        }

        // 2. Prepare data for insertion
        // User, Tweet, Media
        let new_users = tweets
            .clone()
            .map(|t| model::NewTwitterUser::from(&t.user))
            .collect::<Vec<_>>();
        let new_tweets = tweets.clone().map(model::NewTweet::from).collect::<Vec<_>>();
        let media = tweets.clone().flat_map(util::media).collect::<Vec<_>>();

        // WatchListTweet
        let watch_list_tweets = fetched
            .tweets
            .iter()
            .zip(fetched.sort_indices.iter())
            .filter(|(tweet, _)| !existing_ids.contains(&tweet.id))
            .map(|(tweet, sort_index)| model::TwitterWatchListTweet {
                watch_list_id: self.id,
                tweet_id: tweet.id as i64,
                sort_index: Some(*sort_index as i64),
                stale: false,
            })
            .collect::<Vec<_>>();

        // WatchListHistory
        let tweet_ids = tweets.clone().map(|t| t.id.to_string()).collect::<Vec<_>>();
        let (top_cursor, bottom_cursor) = (fetched.top_cursor(), fetched.bottom_cursor());
        let history = model::NewTwitterWatchListHistory {
            watch_list_id: self.id,
            ids: tweet_ids.join(", "),
            count: tweets.clone().count() as i32,
            top_cursor: top_cursor.map(|c| c.value().to_string()),
            top_sort_index: top_cursor.map(|c| c.sort_index() as i64),
            bottom_cursor: bottom_cursor.map(|c| c.value().to_string()),
            bottom_sort_index: bottom_cursor.map(|c| c.sort_index() as i64),
        };

        // 3. Insert data
        db.transaction(|conn| -> Result<()> {
            diesel::insert_into(twitter_user::table)
                .values(&new_users)
                .execute(conn)?;
            diesel::insert_into(tweet::table).values(&new_tweets).execute(conn)?;
            diesel::insert_into(twitter_media::table).values(&media).execute(conn)?;
            diesel::insert_into(twitter_watch_list_tweet::table)
                .values(&watch_list_tweets)
                .execute(conn)?;
            diesel::insert_into(twitter_watch_list_history::table)
                .values(&history)
                .execute(conn)?;
            Ok(())
        })?;

        // TODO: If first fetch limit is reached, mark feed as reached end

        tracing::info!("Saved tweets for twitter feed {}: {}", self.id, history.ids);
        Ok(SaveResult {
            post_ids: tweet_ids,
            should_stop: !existing_ids.is_empty(),
            reached_end: false,
        })
    }

    fn handle_before_update(&self, _db: Database) -> Result<()> {
        unimplemented!()
    }

    fn handle_after_update<'a>(
        &self,
        _db: Database,
        _save_results: impl IntoIterator<Item = &'a SaveResult>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{tweet, twitter_media, twitter_user, twitter_watch_list_tweet};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch posts
        let (posts, total_items) = twitter_watch_list_tweet::table
            .inner_join(tweet::table)
            .filter(twitter_watch_list_tweet::watch_list_id.eq(self.id))
            .order(twitter_watch_list_tweet::sort_index.desc())
            .select(tweet::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::Tweet>(db)?;

        // 2. Fetch associated users
        let user_ids = posts.iter().map(|tweet| tweet.user_id);
        let users = twitter_user::table
            .filter(twitter_user::id.eq_any(user_ids))
            .load::<model::TwitterUser>(db)?;

        // 3. Fetch associated media
        let tweet_ids = posts.iter().map(|tweet| tweet.id);
        let media = twitter_media::table
            .filter(twitter_media::tweet_id.eq_any(tweet_ids.clone()))
            .order(twitter_media::page.asc())
            .load::<model::TwitterMedia>(db)?;

        // 4. Fetch associated works
        let tweet_ids = posts.iter().map(|tweet| tweet.id.to_string());
        let (works, images) = bottle_library::get_works_by_post_ids(db, "twitter", tweet_ids, false)?;

        Ok(GeneralResponse {
            posts: Some(posts.into_iter().map(PostView::from).collect()),
            users: Some(users.into_iter().map(UserView::from).collect()),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            works: Some(works),
            images: Some(images),
            total_items,
            page,
            page_size,
        })
    }

    fn get_account(&self, db: Database) -> Result<Self::Account> {
        TwitterAccount::get(db, self.account_id)?.ok_or(Error::NotLoggedIn("Invalid account".to_string()))
    }

    fn get_fetch_context(&self, db: Database) -> Result<Self::FetchContext> {
        let cursor = match self.params {
            TwitterFeedParams::Likes { .. } if self.reached_end => self.top_cursor(db)?,
            TwitterFeedParams::Likes { .. } if !self.reached_end => self.bottom_cursor(db)?,
            TwitterFeedParams::Search { .. } => None,
            _ => todo!(),
        };
        let direction = match self.params {
            TwitterFeedParams::Search { .. } => Direction::Backward,
            _ if self.reached_end => Direction::Forward,
            _ => Direction::Backward,
        };
        Ok(TwitterFetchContext { cursor, direction })
    }

    async fn fetch(&self, ctx: &mut Self::FetchContext, auth: Option<&Self::Auth>) -> Result<Self::FetchResult> {
        let Some(auth) = auth else {
            return Err(Error::NotLoggedIn("Twitter feed needs an account".to_string()));
        };
        let client = TwitterClient::new(auth.clone()).map_err(anyhow::Error::from)?;
        let cursor = ctx.cursor.as_deref();
        let result = match self.params {
            TwitterFeedParams::Likes { user_id } => client.likes(user_id as u64, cursor).await,
            TwitterFeedParams::Posts { user_id } => client.user_tweets(user_id as u64, cursor).await,
            TwitterFeedParams::Search { ref query } => client.search(query, cursor).await,
            _ => todo!(),
        }
        .map_err(anyhow::Error::from)?;
        // Update cursor according to direction
        let next_cursor = match ctx.direction {
            Direction::Forward => result.top_cursor(),
            Direction::Backward => result.bottom_cursor(),
        };
        ctx.cursor = next_cursor.map(|c| c.value().to_string());
        Ok(result)
    }

    fn archived_posts(db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::library::{ImageView, WorkView};
        use bottle_core::schema::{image, tweet, twitter_media, twitter_user, work};
        use bottle_library::model::{Image, Work};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch works
        let (works, total_items) = work::table
            .filter(work::source.eq("twitter"))
            .order(work::added_date.desc())
            .paginate(page, page_size)
            .load_and_count::<Work>(db)?;

        // 2. Fetch associated images
        let work_ids = works.iter().map(|work| work.id);
        let images = image::table
            .filter(image::work_id.eq_any(work_ids))
            .order(image::page_index.asc())
            .load::<Image>(db)?;

        // 3. Fetch associated posts
        let post_ids = works
            .iter()
            .filter_map(|work| work.post_id.as_ref()?.parse::<i64>().ok());
        let posts = tweet::table
            .filter(tweet::id.eq_any(post_ids.clone()))
            .load::<model::Tweet>(db)?;

        // 4. Fetch associated users
        let user_ids = posts.iter().map(|tweet| tweet.user_id);
        let users = twitter_user::table
            .filter(twitter_user::id.eq_any(user_ids))
            .load::<model::TwitterUser>(db)?;

        // 5. Fetch associated media
        let media = twitter_media::table
            .filter(twitter_media::tweet_id.eq_any(post_ids.clone()))
            .order(twitter_media::page.asc())
            .load::<model::TwitterMedia>(db)?;
        let works = works.into_iter().map(WorkView::from).collect::<Vec<_>>();
        let media = group::filter_media_by_works(&media, &works);

        Ok(GeneralResponse {
            posts: Some(posts.into_iter().map(PostView::from).collect()),
            users: Some(users.into_iter().map(UserView::from).collect()),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            works: Some(works),
            images: Some(images.into_iter().map(ImageView::from).collect()),
            total_items,
            page,
            page_size,
        })
    }

    fn archived_posts_grouped_by_user(
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse> {
        use diesel::dsl::sql_query;
        let query = sql_query(group::grouped_by_user_query(
            "select distinct tweet.* from tweet
            join work on tweet.id = work.post_id_int
            where work.source = 'twitter'",
            "order by created_date desc",
        ))
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count, true)
    }

    fn archived_posts_by_user(db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{tweet, work};
        use bottle_util::diesel_ext::Paginate;

        let user_id = user_id.parse::<i64>()?;
        let results = tweet::table
            .inner_join(work::table.on(work::post_id_int.eq(tweet::id.nullable())))
            .filter(tweet::user_id.eq(user_id))
            .filter(work::source.eq("twitter"))
            .order(tweet::created_date.desc())
            .select(tweet::all_columns)
            .distinct()
            .paginate(page, page_size)
            .load_and_count::<model::Tweet>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size, true)
    }

    fn feed_posts_grouped_by_user(
        &self,
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse> {
        use diesel::{dsl::sql_query, sql_types::Integer};
        let query = sql_query(group::grouped_by_user_query(
            "select tweet.*, sort_index from twitter_watch_list_tweet
            join tweet on tweet_id = tweet.id
            where watch_list_id = ?",
            "order by sort_index desc",
        ))
        .bind::<Integer, _>(self.id)
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count, false)
    }

    fn feed_posts_by_user(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{tweet, twitter_watch_list_tweet};
        use bottle_util::diesel_ext::Paginate;

        let user_id = user_id.parse::<i64>()?;
        let results = twitter_watch_list_tweet::table
            .inner_join(tweet::table)
            .filter(tweet::user_id.eq(user_id))
            .filter(twitter_watch_list_tweet::watch_list_id.eq(self.id))
            .order(twitter_watch_list_tweet::sort_index.desc())
            .select(tweet::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::Tweet>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size, false)
    }
}

// MARK: Helpers

impl TwitterFeed {
    fn top_cursor(&self, db: Database) -> Result<Option<String>> {
        use bottle_core::schema::twitter_watch_list_history::dsl::*;
        let result = twitter_watch_list_history
            .filter(watch_list_id.eq(self.id))
            .order((top_sort_index.desc(), updated_date.desc()))
            .select(top_cursor)
            .first::<Option<String>>(db)
            .optional()?;
        Ok(result.flatten())
    }

    fn bottom_cursor(&self, db: Database) -> Result<Option<String>> {
        use bottle_core::schema::twitter_watch_list_history::dsl::*;
        let result = twitter_watch_list_history
            .filter(watch_list_id.eq(self.id))
            .order((bottom_sort_index.asc(), updated_date.desc()))
            .select(bottom_cursor)
            .first::<Option<String>>(db)
            .optional()?;
        Ok(result.flatten())
    }
}

impl TwitterFeedParams {
    fn kind(&self) -> String {
        match self {
            TwitterFeedParams::Timeline => "timeline".to_string(),
            TwitterFeedParams::Bookmarks => "bookmarks".to_string(),
            TwitterFeedParams::Likes { .. } => "likes".to_string(),
            TwitterFeedParams::Posts { .. } => "posts".to_string(),
            TwitterFeedParams::List { .. } => "list".to_string(),
            TwitterFeedParams::Search { .. } => "search".to_string(),
        }
    }

    fn twitter_list_id(&self) -> Option<i64> {
        match self {
            TwitterFeedParams::List { list_id } => Some(*list_id),
            _ => None,
        }
    }

    fn user_id(&self) -> Option<i64> {
        match self {
            TwitterFeedParams::Likes { user_id } => Some(*user_id),
            TwitterFeedParams::Posts { user_id } => Some(*user_id),
            _ => None,
        }
    }

    fn search_query(&self) -> Option<String> {
        match self {
            TwitterFeedParams::Search { query } => Some(query.clone()),
            _ => None,
        }
    }
}
