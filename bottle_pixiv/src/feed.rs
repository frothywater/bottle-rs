use async_trait::async_trait;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet};

use bottle_core::{feed::*, Error, Result};
use pixiv_client::{FollowingRestriction, IllustList, IllustType, Paginated, PixivClient, Restriction};

use crate::community::{AccessToken, PixivAccount, RefreshToken};
use crate::{group, model, util};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PixivFeedParams {
    Timeline {
        restriction: FollowingRestriction,
    },
    Bookmarks {
        user_id: i64,
        tag: Option<String>,
        restriction: Restriction,
    },
    Posts {
        user_id: i64,
        #[serde(rename = "type")]
        type_: IllustType,
    },
    Search {
        query: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixivFetchContext {
    pub(crate) offset: Option<i64>,
    pub(crate) total_fetched: usize,
}

#[derive(Debug, Clone)]
pub struct PixivFeed {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub params: PixivFeedParams,
    pub reached_end: bool,
}

#[async_trait]
impl Feed for PixivFeed {
    type Params = PixivFeedParams;
    type Auth = AccessToken;
    type Credential = RefreshToken;
    type Account = PixivAccount;
    type FetchResult = IllustList;
    type FetchContext = PixivFetchContext;

    fn metadata() -> Vec<FeedMetadata> {
        vec![
            FeedMetadata {
                name: "timeline".to_string(),
                scheme: Scheme::Object(HashMap::from([("restriction".to_string(), Scheme::String)])),
                need_auth: true,
            },
            FeedMetadata {
                name: "bookmarks".to_string(),
                scheme: Scheme::Object(HashMap::from([
                    ("user_id".to_string(), Scheme::Bigint),
                    ("tag".to_string(), Scheme::Optional(Box::new(Scheme::String))),
                    ("restriction".to_string(), Scheme::String),
                ])),
                need_auth: true,
            },
            FeedMetadata {
                name: "posts".to_string(),
                scheme: Scheme::Object(HashMap::from([
                    ("user_id".to_string(), Scheme::Bigint),
                    ("type".to_string(), Scheme::String),
                ])),
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
            community: "pixiv".to_string(),
            name: self.name.clone(),
            watching: self.watching,
            description: match &self.params {
                PixivFeedParams::Timeline { restriction } => format!("{} Timeline", restriction),
                PixivFeedParams::Bookmarks {
                    user_id,
                    tag: Some(tag),
                    restriction,
                } => format!("{} Bookmarks tagged #{} by {}", restriction, tag, user_id),
                PixivFeedParams::Bookmarks {
                    user_id,
                    tag: None,
                    restriction,
                } => format!("{} Bookmarks by {}", restriction, user_id),
                PixivFeedParams::Posts { user_id, type_ } => format!("{} by {}", type_, user_id),
                PixivFeedParams::Search { query } => format!("Search {}", query),
            },
        }
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_watch_list::dsl::*;
        pixiv_watch_list
            .load::<model::PixivWatchList>(db)?
            .into_iter()
            .map(Self::try_from)
            .collect()
    }

    fn get(db: Database, feed_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_watch_list::dsl::*;
        let result = pixiv_watch_list
            .filter(id.eq(feed_id))
            .first::<model::PixivWatchList>(db)
            .optional()?;
        result.map(Self::try_from).transpose()
    }

    fn delete(db: Database, feed_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_watch_list::dsl::*;
        diesel::delete(pixiv_watch_list.filter(id.eq(feed_id))).execute(db)?;
        tracing::info!("Deleted pixiv feed {}", feed_id);
        Ok(())
    }

    fn add(db: Database, params: &Self::Params, info: &FeedInfo, account_id: Option<i32>) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_watch_list;
        let Some(account_id) = account_id else {
            return Err(Error::NotLoggedIn("Pixiv feed needs an account".to_string()));
        };
        let new_watch_list = model::NewPixivWatchList {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
            account_id,
            kind: params.kind_str().to_string(),
            user_id: params.user_id(),
            bookmark_tag: params.bookmark_tag(),
            illust_type: params.illust_type(),
            search_query: params.search_query(),
            restriction: params.restriction(),
        };
        let result = diesel::insert_into(pixiv_watch_list::table)
            .values(&new_watch_list)
            .get_result::<model::PixivWatchList>(db)?;
        tracing::info!(
            "Added pixiv feed {}: {:?} {:?}, account {}",
            result.id,
            params,
            info,
            account_id
        );
        Self::try_from(result)
    }

    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView> {
        use bottle_core::schema::pixiv_watch_list;
        let update = model::PixivWatchListUpdate {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
        };
        diesel::update(pixiv_watch_list::table.find(self.id))
            .set(&update)
            .execute(db)?;
        self.name = info.name.clone();
        self.watching = info.watching;
        self.first_fetch_limit = info.first_fetch_limit;
        tracing::info!("Modified pixiv feed {}: {:?}", self.id, info);
        Ok(self.view())
    }

    fn save(&self, db: Database, fetched: &Self::FetchResult, ctx: &Self::FetchContext) -> Result<SaveResult> {
        use bottle_core::schema::{
            pixiv_illust, pixiv_illust_tag, pixiv_media, pixiv_user, pixiv_watch_list, pixiv_watch_list_history,
            pixiv_watch_list_illust,
        };

        // If reached end, mark feed as reached end
        let reached_end = fetched.next_url().is_none();
        if reached_end {
            diesel::update(pixiv_watch_list::table.find(self.id))
                .set(pixiv_watch_list::reached_end.eq(true))
                .execute(db)?;
            tracing::info!("Set pixiv feed {} as reached end", self.id);
        }
        // (a) If response is empty, we should stop updating
        if fetched.illusts.is_empty() {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end,
            });
        }

        // 1. Filter out posts already exist in database
        let fetched_ids = fetched
            .illusts
            .iter()
            .map(|illust| illust.id as i64)
            .collect::<Vec<_>>();
        let existing_ids = pixiv_watch_list_illust::table
            .filter(pixiv_watch_list_illust::watch_list_id.eq(self.id))
            .filter(pixiv_watch_list_illust::illust_id.eq_any(&fetched_ids))
            .select(pixiv_watch_list_illust::illust_id)
            .load::<i64>(db)?;
        let existing_ids = existing_ids.into_iter().map(|id| id as u64).collect::<HashSet<_>>();
        let illusts = fetched
            .illusts
            .iter()
            .filter(|illust| !existing_ids.contains(&illust.id));

        // (b) If all posts already exist in database, we should stop updating
        if illusts.clone().count() == 0 {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end: false,
            });
        }

        // 2. Prepare data for insertion
        // User, Illust, Media, Tag
        let new_users = illusts
            .clone()
            .map(|illust| model::NewPixivUser::from(&illust.user))
            .collect::<Vec<_>>();
        let new_illusts = illusts.clone().map(model::NewPixivIllust::from).collect::<Vec<_>>();
        let media = illusts.clone().flat_map(util::media).collect::<Vec<_>>();
        let tags = illusts.clone().flat_map(util::tags).collect::<Vec<_>>();

        // WatchListIllust
        let watch_list_illusts = illusts
            .clone()
            .map(|illust| model::PixivWatchListIllust {
                watch_list_id: self.id,
                illust_id: illust.id as i64,
                private_bookmark: self.params.is_private_bookmark(),
                stale: false,
                sort_index: None,
            })
            .collect::<Vec<_>>();

        // WatchListHistory
        let illust_ids = illusts.clone().map(|illust| illust.id.to_string()).collect::<Vec<_>>();
        let history = model::NewPixivWatchListHistory {
            watch_list_id: self.id,
            ids: illust_ids.join(", "),
            count: illusts.clone().count() as i32,
            next_bookmark_id: fetched.next_bookmark_id().map(|id| id as i64),
        };

        // 3. Insert data
        db.transaction(|conn| -> Result<()> {
            diesel::insert_into(pixiv_user::table)
                .values(&new_users)
                .execute(conn)?;
            diesel::insert_into(pixiv_illust::table)
                .values(&new_illusts)
                .execute(conn)?;
            diesel::insert_into(pixiv_media::table).values(&media).execute(conn)?;
            diesel::insert_into(pixiv_illust_tag::table)
                .values(&tags)
                .execute(conn)?;
            diesel::insert_into(pixiv_watch_list_illust::table)
                .values(&watch_list_illusts)
                .execute(conn)?;
            diesel::insert_into(pixiv_watch_list_history::table)
                .values(&history)
                .execute(conn)?;
            Ok(())
        })?;
        tracing::info!("Saved posts to pixiv feed {}: {}", self.id, history.ids);

        // If first fetch limit is reached, mark feed as reached end
        if let Some(limit) = self.first_fetch_limit {
            if !self.reached_end && (ctx.total_fetched as i32) >= limit {
                diesel::update(pixiv_watch_list::table.find(self.id))
                    .set(pixiv_watch_list::reached_end.eq(true))
                    .execute(db)?;
                tracing::info!(
                    "Set pixiv feed {} as reached end due to {} first fetch limit",
                    self.id,
                    limit
                );
                return Ok(SaveResult {
                    post_ids: illust_ids,
                    should_stop: true,
                    reached_end: true,
                });
            }
        }

        Ok(SaveResult {
            post_ids: illust_ids,
            should_stop: !existing_ids.is_empty(),
            reached_end,
        })
    }

    fn handle_before_update(&self, db: Database) -> Result<()> {
        use bottle_core::schema::pixiv_watch_list_illust::dsl::*;
        use itertools::Itertools;

        // Delete watch list posts that don't have sort index
        let post_ids = pixiv_watch_list_illust
            .filter(watch_list_id.eq(self.id))
            .filter(sort_index.is_null())
            .select(illust_id)
            .load::<i64>(db)?;
        if post_ids.is_empty() {
            return Ok(());
        }

        diesel::delete(
            pixiv_watch_list_illust
                .filter(watch_list_id.eq(self.id))
                .filter(sort_index.is_null()),
        )
        .execute(db)?;
        tracing::info!(
            "Deleted {} posts without sort index from pixiv feed {}: {}",
            post_ids.len(),
            self.id,
            post_ids.iter().join(", ")
        );
        Ok(())
    }

    fn handle_after_update<'a>(
        &self,
        db: Database,
        save_results: impl IntoIterator<Item = &'a SaveResult>,
    ) -> Result<()> {
        // Update sort index for new posts
        use bottle_core::schema::pixiv_watch_list_illust;

        // 1. Get the last sort index
        let last_sort_index = pixiv_watch_list_illust::table
            .filter(pixiv_watch_list_illust::watch_list_id.eq(self.id))
            .select(pixiv_watch_list_illust::sort_index)
            .order(pixiv_watch_list_illust::sort_index.desc())
            .first::<Option<i32>>(db)
            .optional()?
            .flatten()
            .unwrap_or(-1);

        // 2. Determine sort indices for new posts, which are in descending order
        let post_ids = save_results
            .into_iter()
            .flat_map(|r| r.post_ids.iter())
            .collect::<Vec<_>>();
        let sort_indices = ((last_sort_index + 1)..(last_sort_index + 1 + post_ids.len() as i32)).rev();

        // 3. Update sort indices
        db.transaction(|conn| -> Result<()> {
            for (post_id, sort_index) in post_ids.iter().zip(sort_indices) {
                diesel::update(pixiv_watch_list_illust::table)
                    .filter(pixiv_watch_list_illust::watch_list_id.eq(self.id))
                    .filter(pixiv_watch_list_illust::illust_id.eq(post_id.parse::<i64>()?))
                    .set(pixiv_watch_list_illust::sort_index.eq(sort_index))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        tracing::info!(
            "Updated sort indices for {} posts in pixiv feed {}",
            post_ids.len(),
            self.id,
        );
        Ok(())
    }

    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{pixiv_illust, pixiv_media, pixiv_user, pixiv_watch_list_illust};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch posts
        let (posts, total_items) = pixiv_watch_list_illust::table
            .inner_join(pixiv_illust::table)
            .filter(pixiv_watch_list_illust::watch_list_id.eq(self.id))
            .order(pixiv_watch_list_illust::sort_index.desc())
            .select(pixiv_illust::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::PixivIllust>(db)?;

        // 2. Fetch associated users
        let user_ids = posts.iter().map(|illust| illust.user_id);
        let users = pixiv_user::table
            .filter(pixiv_user::id.eq_any(user_ids))
            .load::<model::PixivUser>(db)?;

        // 3. Fetch associated media
        let illust_ids = posts.iter().map(|illust| illust.id).collect::<Vec<_>>();
        let media = pixiv_media::table
            .filter(pixiv_media::illust_id.eq_any(illust_ids.clone()))
            .order(pixiv_media::page.asc())
            .load::<model::PixivMedia>(db)?;

        let tags = util::get_tag_map(db, illust_ids.clone())?;
        let posts = posts
            .into_iter()
            .map(|illust| illust.post_view(tags.get(&illust.id).cloned().unwrap_or_default()))
            .collect();

        // 4. Fetch associated works
        let illust_ids = illust_ids.iter().map(|id| id.to_string());
        let (works, images) = bottle_library::get_works_by_post_ids(db, "pixiv", illust_ids, false)?;

        Ok(GeneralResponse {
            posts: Some(posts),
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
        PixivAccount::get(db, self.account_id)?.ok_or(Error::NotLoggedIn("Invalid account".to_string()))
    }

    fn get_fetch_context(&self, db: Database) -> Result<Self::FetchContext> {
        let last_bookmark_id = matches!(self.params, PixivFeedParams::Bookmarks { .. })
            .then_some(self.last_bookmark_id(db)?)
            .flatten();
        Ok(PixivFetchContext {
            offset: last_bookmark_id,
            total_fetched: 0,
        })
    }

    async fn fetch(&self, ctx: &mut Self::FetchContext, auth: Option<&Self::Auth>) -> Result<Self::FetchResult> {
        let Some(auth) = auth else {
            return Err(Error::NotLoggedIn("Pixiv feed needs an account".to_string()));
        };
        let client = PixivClient::new(&auth.0).map_err(anyhow::Error::from)?;

        let offset = ctx.offset;
        let result = match self.params.clone() {
            PixivFeedParams::Timeline { restriction } => {
                client.following_illusts(restriction, offset.map(|o| o as u32)).await
            }
            PixivFeedParams::Bookmarks {
                user_id,
                tag,
                restriction,
            } => {
                client
                    .user_bookmarks(user_id as u64, restriction, tag.as_deref(), offset.map(|o| o as u64))
                    .await
            }
            PixivFeedParams::Posts { user_id, type_ } => {
                client
                    .user_illusts(user_id as u64, type_, offset.map(|o| o as u32))
                    .await
            }
            _ => todo!(),
        }
        .map_err(anyhow::Error::from)?;

        let next_offset = match &self.params {
            PixivFeedParams::Timeline { .. } | PixivFeedParams::Search { .. } | PixivFeedParams::Posts { .. } => {
                result.next_offset()
            }
            PixivFeedParams::Bookmarks { .. } => result.next_bookmark_id(),
        };
        ctx.offset = next_offset.map(|o| o as i64);
        ctx.total_fetched += result.illusts.len();
        Ok(result)
    }

    fn archived_posts(db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::library::{ImageView, WorkView};
        use bottle_core::schema::{image, pixiv_illust, pixiv_media, pixiv_user, work};
        use bottle_library::model::{Image, Work};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch works
        let (works, total_items) = work::table
            .filter(work::source.eq("pixiv"))
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
        let posts = pixiv_illust::table
            .filter(pixiv_illust::id.eq_any(post_ids.clone()))
            .load::<model::PixivIllust>(db)?;

        // 4. Fetch associated users
        let user_ids = posts.iter().map(|post| post.user_id);
        let users = pixiv_user::table
            .filter(pixiv_user::id.eq_any(user_ids))
            .load::<model::PixivUser>(db)?;

        let tags = util::get_tag_map(db, post_ids.clone())?;
        let posts = posts
            .into_iter()
            .map(|illust| illust.post_view(tags.get(&illust.id).cloned().unwrap_or_default()))
            .collect();

        // 5. Fetch associated media
        let media = pixiv_media::table
            .filter(pixiv_media::illust_id.eq_any(post_ids.clone()))
            .order(pixiv_media::page.asc())
            .load::<model::PixivMedia>(db)?;
        let works = works.into_iter().map(WorkView::from).collect::<Vec<_>>();
        let media = group::filter_media_by_works(&media, &works);

        Ok(GeneralResponse {
            posts: Some(posts),
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
            "select distinct pixiv_illust.* from pixiv_illust
            join work on pixiv_illust.id = work.post_id_int
            where work.source = 'pixiv'",
            "order by created_date desc",
        ))
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count, true)
    }

    fn archived_posts_by_user(db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{pixiv_illust, work};
        use bottle_util::diesel_ext::Paginate;

        let user_id = user_id.parse::<i64>()?;
        let results = pixiv_illust::table
            .inner_join(work::table.on(work::post_id_int.eq(pixiv_illust::id.nullable())))
            .filter(pixiv_illust::user_id.eq(user_id))
            .filter(work::source.eq("pixiv"))
            .order(pixiv_illust::created_date.desc())
            .select(pixiv_illust::all_columns)
            .distinct()
            .paginate(page, page_size)
            .load_and_count::<model::PixivIllust>(db)?;
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
            "select pixiv_illust.*, sort_index from pixiv_watch_list_illust
            join pixiv_illust on illust_id = pixiv_illust.id
            where watch_list_id = ?",
            "order by sort_index desc",
        ))
        .bind::<Integer, _>(self.id)
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count, false)
    }

    fn feed_posts_by_user(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{pixiv_illust, pixiv_watch_list_illust};
        use bottle_util::diesel_ext::Paginate;

        let user_id = user_id.parse::<i64>()?;
        let results = pixiv_watch_list_illust::table
            .inner_join(pixiv_illust::table)
            .filter(pixiv_watch_list_illust::watch_list_id.eq(self.id))
            .filter(pixiv_illust::user_id.eq(user_id))
            .order(pixiv_watch_list_illust::sort_index.desc())
            .select(pixiv_illust::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::PixivIllust>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size, false)
    }
}

// MARK: Helpers

impl PixivFeed {
    fn last_bookmark_id(&self, db: Database) -> Result<Option<i64>> {
        use bottle_core::schema::pixiv_watch_list_history::dsl::*;
        let result = pixiv_watch_list_history
            .filter(watch_list_id.eq(self.id))
            .select(next_bookmark_id)
            .order(next_bookmark_id.asc())
            .first::<Option<i64>>(db)
            .optional()?
            .flatten();
        Ok(result)
    }
}

impl PixivFeedParams {
    fn kind_str(&self) -> &str {
        match self {
            Self::Timeline { .. } => "timeline",
            Self::Bookmarks { .. } => "bookmarks",
            Self::Posts { .. } => "posts",
            Self::Search { .. } => "search",
        }
    }

    fn is_private_bookmark(&self) -> bool {
        matches!(
            self,
            Self::Bookmarks {
                restriction: Restriction::Private,
                ..
            }
        )
    }

    fn user_id(&self) -> Option<i64> {
        match self {
            Self::Bookmarks { user_id, .. } => Some(*user_id),
            Self::Posts { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }

    fn restriction(&self) -> Option<String> {
        match self {
            Self::Timeline { restriction } => Some(restriction.to_string()),
            Self::Bookmarks { restriction, .. } => Some(restriction.to_string()),
            _ => None,
        }
    }

    fn bookmark_tag(&self) -> Option<String> {
        match self {
            Self::Bookmarks { tag, .. } => tag.clone(),
            _ => None,
        }
    }

    fn illust_type(&self) -> Option<String> {
        match self {
            Self::Posts { type_, .. } => Some(type_.to_string()),
            _ => None,
        }
    }

    fn search_query(&self) -> Option<String> {
        match self {
            Self::Search { query } => Some(query.clone()),
            _ => None,
        }
    }
}
