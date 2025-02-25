use std::collections::HashMap;

use async_trait::async_trait;
use diesel::{dsl::sql_query, prelude::*, sql_types::Integer};
use serde::{Deserialize, Serialize};

use bottle_core::{feed::*, Database, Result};
use yandere_client::APIResult;

use crate::community::YandereAccount;
use crate::{group, model, util};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum YandereFeedParams {
    Search { query: String },
    Pool { pool_id: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YandereFetchContext {
    pub(crate) page: u32,
}

#[derive(Debug, Clone)]
pub struct YandereFeed {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub params: YandereFeedParams,
    pub reached_end: bool,
}

#[async_trait]
impl Feed for YandereFeed {
    type Params = YandereFeedParams;
    type Auth = ();
    type Credential = ();
    type Account = YandereAccount;
    type FetchResult = APIResult;
    type FetchContext = YandereFetchContext;

    fn metadata() -> Vec<FeedMetadata> {
        vec![
            FeedMetadata {
                name: "search".to_string(),
                scheme: Scheme::Object(HashMap::from([("query".to_string(), Scheme::String)])),
                need_auth: false,
            },
            FeedMetadata {
                name: "pool".to_string(),
                scheme: Scheme::Object(HashMap::from([("pool_id".to_string(), Scheme::Int)])),
                need_auth: false,
            },
        ]
    }

    fn view(&self) -> FeedView {
        FeedView {
            feed_id: self.id,
            community: "yandere".to_string(),
            name: self.name.clone(),
            watching: self.watching,
            description: match &self.params {
                YandereFeedParams::Search { query } => format!("Search {}", query),
                YandereFeedParams::Pool { pool_id } => format!("Pool {}", pool_id),
            },
        }
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::yandere_watch_list::dsl::*;
        yandere_watch_list
            .load::<model::YandereWatchList>(db)?
            .into_iter()
            .map(Self::try_from)
            .collect()
    }

    fn get(db: Database, feed_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::yandere_watch_list::dsl::*;
        let result = yandere_watch_list
            .filter(id.eq(feed_id))
            .first::<model::YandereWatchList>(db)
            .optional()?;
        result.map(Self::try_from).transpose()
    }

    fn delete(db: Database, feed_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::yandere_watch_list::dsl::*;
        diesel::delete(yandere_watch_list.filter(id.eq(feed_id))).execute(db)?;
        tracing::info!("Deleted yandere feed {}", feed_id);
        Ok(())
    }

    fn add(db: Database, params: &Self::Params, info: &FeedInfo, _account_id: Option<i32>) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::yandere_watch_list;
        let new_watch_list = model::NewYandereWatchList {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
            kind: params.kind_str().to_string(),
            search_query: params.search_query(),
            pool_id: params.pool_id(),
        };
        let result = diesel::insert_into(yandere_watch_list::table)
            .values(&new_watch_list)
            .get_result::<model::YandereWatchList>(db)?;
        tracing::info!("Added yandere feed {}: {:?} {:?}", result.id, params, info);
        Self::try_from(result)
    }

    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView> {
        use bottle_core::schema::yandere_watch_list;
        let update = model::YandereWatchListUpdate {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
        };
        diesel::update(yandere_watch_list::table.find(self.id))
            .set(&update)
            .execute(db)?;
        self.name = info.name.clone();
        self.watching = info.watching;
        self.first_fetch_limit = info.first_fetch_limit;
        tracing::info!("Updated yandere feed {}: {:?}", self.id, info);
        Ok(self.view())
    }

    fn save(&self, db: Database, fetched: &Self::FetchResult, _ctx: &Self::FetchContext) -> Result<SaveResult> {
        use bottle_core::schema::{
            yandere_pool, yandere_pool_post, yandere_post, yandere_post_tag, yandere_tag, yandere_watch_list,
            yandere_watch_list_history, yandere_watch_list_post,
        };

        // (a) If no posts are fetched, mark the feed as reached end
        if fetched.posts.is_empty() {
            diesel::update(yandere_watch_list::table)
                .filter(yandere_watch_list::id.eq(self.id))
                .set(yandere_watch_list::reached_end.eq(true))
                .execute(db)?;
            tracing::info!("Set yandere feed {} as reached end", self.id);
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end: true,
            });
        }

        // 1. Filter out posts that are already in the feed
        let fetched_ids = fetched.posts.iter().map(|post| post.id as i64);
        let existing_ids = yandere_watch_list_post::table
            .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
            .filter(yandere_watch_list_post::post_id.eq_any(fetched_ids))
            .select(yandere_watch_list_post::post_id)
            .load::<i64>(db)?;
        let existing_ids = existing_ids.into_iter().map(|id| id as u64).collect::<Vec<_>>();
        let posts = fetched.posts.iter().filter(|post| !existing_ids.contains(&post.id));

        // (b) If no posts are new, stop
        if posts.clone().count() == 0 {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end: false,
            });
        }

        // 2. Prepare data to insert
        // Post, Tag, PostTag
        let new_posts = posts.clone().map(model::NewYanderePost::from).collect::<Vec<_>>();
        let tags = util::tags(fetched);
        let post_tags = posts.clone().flat_map(util::post_tags).collect::<Vec<_>>();

        // Pool, PoolPost
        let pools = util::pools(fetched);
        let pool_posts = util::pool_posts(fetched);

        // WatchListPost
        let watch_list_posts = posts
            .clone()
            .map(|post| model::YandereWatchListPost {
                watch_list_id: self.id,
                post_id: post.id as i64,
                sort_index: None,
            })
            .collect::<Vec<_>>();

        // WatchListHistory
        let post_ids = posts.clone().map(|post| post.id.to_string()).collect::<Vec<_>>();
        let history = model::NewYandereWatchListHistory {
            watch_list_id: self.id,
            ids: post_ids.join(", "),
            count: posts.clone().count() as i32,
        };

        // 3. Insert data
        db.transaction(|conn| -> Result<()> {
            diesel::insert_into(yandere_tag::table).values(tags).execute(conn)?;
            diesel::insert_into(yandere_pool::table).values(pools).execute(conn)?;
            diesel::insert_into(yandere_post::table)
                .values(new_posts)
                .execute(conn)?;
            diesel::insert_into(yandere_pool_post::table)
                .values(pool_posts)
                .execute(conn)?;
            diesel::insert_into(yandere_post_tag::table)
                .values(post_tags)
                .execute(conn)?;
            diesel::insert_into(yandere_watch_list_post::table)
                .values(watch_list_posts)
                .execute(conn)?;
            diesel::insert_into(yandere_watch_list_history::table)
                .values(&history)
                .execute(conn)?;
            Ok(())
        })?;

        // TODO: If first fetch limit is reached, mark feed as reached end

        tracing::info!("Saved posts for yandere feed {}: {}", self.id, history.ids);
        Ok(SaveResult {
            post_ids,
            should_stop: !existing_ids.is_empty(),
            reached_end: false,
        })
    }

    fn handle_before_update(&self, db: Database) -> Result<()> {
        // Delete watch list posts that don't have sort index
        use bottle_core::schema::yandere_watch_list_post;
        use itertools::Itertools;

        let post_ids = yandere_watch_list_post::table
            .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
            .filter(yandere_watch_list_post::sort_index.is_null())
            .select(yandere_watch_list_post::post_id)
            .load::<i64>(db)?;
        if post_ids.is_empty() {
            return Ok(());
        }

        diesel::delete(
            yandere_watch_list_post::table
                .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
                .filter(yandere_watch_list_post::sort_index.is_null()),
        )
        .execute(db)?;
        tracing::info!(
            "Deleted {} posts without sort index for yandere feed {}: {}",
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
        use bottle_core::schema::yandere_watch_list_post;

        // 1. Get the last sort index
        let last_sort_index = yandere_watch_list_post::table
            .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
            .select(yandere_watch_list_post::sort_index)
            .order(yandere_watch_list_post::sort_index.desc())
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
                diesel::update(yandere_watch_list_post::table)
                    .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
                    .filter(yandere_watch_list_post::post_id.eq(post_id.parse::<i64>()?))
                    .set(yandere_watch_list_post::sort_index.eq(sort_index))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{yandere_post, yandere_watch_list_post};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch posts
        let (posts, total_items) = yandere_watch_list_post::table
            .inner_join(yandere_post::table)
            .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
            .order(yandere_watch_list_post::sort_index.desc())
            .select(yandere_post::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::YanderePost>(db)?;

        // 2. Fetch associated works
        let post_ids = posts.iter().map(|r| r.id.to_string());
        let (works, images) = bottle_library::get_works_by_post_ids(db, "yandere", post_ids, false)?;

        // 3. Fetch associated users
        let post_ids = posts.iter().map(|p| p.id);
        let users = util::get_artist_views(db, post_ids)?;

        Ok(GeneralResponse {
            posts: Some(posts.iter().map(PostView::from).collect()),
            media: Some(posts.iter().map(MediaView::from).collect()),
            users: Some(users),
            works: Some(works),
            images: Some(images),
            total_items,
            page,
            page_size,
        })
    }

    fn get_account(&self, _db: Database) -> Result<Self::Account> {
        unimplemented!()
    }

    fn get_fetch_context(&self, _db: Database) -> Result<Self::FetchContext> {
        Ok(YandereFetchContext { page: 1 })
    }

    async fn fetch(&self, ctx: &mut Self::FetchContext, _auth: Option<&Self::Auth>) -> Result<Self::FetchResult> {
        let result = match self.params {
            YandereFeedParams::Search { ref query } => yandere_client::fetch_posts(query, ctx.page).await,
            YandereFeedParams::Pool { pool_id } => {
                yandere_client::fetch_posts(&format!("pool:{}", pool_id), ctx.page).await
            }
        }
        .map_err(anyhow::Error::from)?;
        ctx.page += 1;
        Ok(result)
    }

    fn archived_posts(db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::library::{ImageView, WorkView};
        use bottle_core::schema::{image, work, yandere_post};
        use bottle_library::model::{Image, Work};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch works
        let (works, total_items) = work::table
            .filter(work::source.eq("yandere"))
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
            .filter_map(|work| work.post_id.as_ref())
            .filter_map(|id| id.parse::<i64>().ok());
        let posts = yandere_post::table
            .filter(yandere_post::id.eq_any(post_ids.clone()))
            .load::<model::YanderePost>(db)?;

        // 4. Fetch associated users
        let users = util::get_artist_views(db, post_ids)?;

        Ok(GeneralResponse {
            posts: Some(posts.iter().map(PostView::from).collect()),
            media: Some(posts.iter().map(MediaView::from).collect()),
            users: Some(users),
            works: Some(works.into_iter().map(WorkView::from).collect()),
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
            "select distinct yandere_post.* from yandere_post
            join work on yandere_post.id = work.post_id_int
            where work.source = 'yandere'",
            "order by created_date desc",
        ))
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count)
    }

    fn archived_posts_by_user(db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{work, yandere_post, yandere_post_tag, yandere_tag};
        use bottle_util::diesel_ext::Paginate;

        let results = yandere_post::table
            .inner_join(yandere_post_tag::table.inner_join(yandere_tag::table))
            .inner_join(work::table.on(work::post_id_int.eq(yandere_post::id.nullable())))
            .filter(yandere_tag::name.eq(&user_id).and(yandere_tag::type_.eq("artist")))
            .filter(work::source.eq("yandere"))
            .order(yandere_post::created_date.desc())
            .select(yandere_post::all_columns)
            .distinct()
            .paginate(page, page_size)
            .load_and_count::<model::YanderePost>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size)
    }

    fn feed_posts_grouped_by_user(
        &self,
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse> {
        let query = sql_query(group::grouped_by_user_query(
            "select yandere_post.*, sort_index from yandere_watch_list_post
            join yandere_post on yandere_watch_list_post.post_id = yandere_post.id
            where watch_list_id = ?",
            "order by sort_index desc",
        ))
        .bind::<Integer, _>(self.id)
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count)
    }

    fn feed_posts_by_user(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{yandere_post, yandere_post_tag, yandere_tag, yandere_watch_list_post};
        use bottle_util::diesel_ext::Paginate;

        let results = yandere_watch_list_post::table
            .inner_join(yandere_post::table.inner_join(yandere_post_tag::table.inner_join(yandere_tag::table)))
            .filter(yandere_watch_list_post::watch_list_id.eq(self.id))
            .filter(yandere_tag::name.eq(&user_id).and(yandere_tag::type_.eq("artist")))
            .order(yandere_watch_list_post::sort_index.desc())
            .select(yandere_post::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::YanderePost>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size)
    }
}

// MARK: Helpers

impl YandereFeedParams {
    fn kind_str(&self) -> &str {
        match self {
            YandereFeedParams::Search { .. } => "search",
            YandereFeedParams::Pool { .. } => "pool",
        }
    }

    fn search_query(&self) -> Option<String> {
        match self {
            YandereFeedParams::Search { query } => Some(query.clone()),
            _ => None,
        }
    }

    fn pool_id(&self) -> Option<i32> {
        match self {
            YandereFeedParams::Pool { pool_id } => Some(*pool_id),
            _ => None,
        }
    }
}
