use async_trait::async_trait;
use diesel::prelude::*;
use serde::Deserialize;

use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};

use bottle_core::feed::{Account, Feed};
use bottle_core::{feed::*, Error, Result};
use panda_client::{
    FavoriteSearchOption, GalleryListOffset, GalleryListResult, PandaClient, PandaCookie, SearchOption,
};

use crate::community::PandaAccount;
use crate::util;
use crate::{group, model};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PandaFeedParams {
    Search { option: SearchOption },
    Watched { option: SearchOption },
    Favorites { option: FavoriteSearchOption },
}

#[derive(Debug, Clone)]
pub struct PandaFetchContext {
    pub(crate) offset: Option<GalleryListOffset>,
    pub(crate) direction: Direction,
}

#[derive(Debug, Clone)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
pub struct PandaFeed {
    pub id: i32,
    pub name: Option<String>,
    pub watching: bool,
    pub first_fetch_limit: Option<i32>,
    pub account_id: i32,
    pub params: PandaFeedParams,
    pub reached_end: bool,
}

#[async_trait]
impl Feed for PandaFeed {
    type Params = PandaFeedParams;
    type Auth = PandaCookie;
    type Credential = PandaCookie;
    type Account = PandaAccount;
    type FetchResult = GalleryListResult;
    type FetchContext = PandaFetchContext;

    fn metadata() -> Vec<FeedMetadata> {
        vec![
            FeedMetadata {
                name: "search".to_string(),
                scheme: Scheme::Object(HashMap::from([("option".to_string(), util::search_option_scheme())])),
                need_auth: true,
            },
            FeedMetadata {
                name: "watched".to_string(),
                scheme: Scheme::Object(HashMap::from([("option".to_string(), util::search_option_scheme())])),
                need_auth: true,
            },
            FeedMetadata {
                name: "favorites".to_string(),
                scheme: Scheme::Object(HashMap::from([(
                    "option".to_string(),
                    util::favorite_search_option_scheme(),
                )])),
                need_auth: true,
            },
        ]
    }

    fn view(&self) -> FeedView {
        FeedView {
            feed_id: self.id,
            community: "panda".to_string(),
            name: self.name.clone(),
            watching: self.watching,
            description: self.params.to_string(),
        }
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_watch_list::dsl::*;
        panda_watch_list
            .load::<model::PandaWatchList>(db)?
            .into_iter()
            .map(Self::try_from)
            .collect()
    }

    fn get(db: Database, feed_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_watch_list::dsl::*;
        let result = panda_watch_list
            .filter(id.eq(feed_id))
            .first::<model::PandaWatchList>(db)
            .optional()?;
        result.map(Self::try_from).transpose()
    }

    fn delete(db: Database, feed_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_watch_list::dsl::*;
        diesel::delete(panda_watch_list.find(feed_id)).execute(db)?;
        tracing::info!("Deleted panda feed {}", feed_id);
        Ok(())
    }

    fn add(db: Database, params: &Self::Params, info: &FeedInfo, account_id: Option<i32>) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_watch_list;
        let Some(account_id) = account_id else {
            return Err(Error::NotLoggedIn("Panda feed needs an account".to_string()));
        };
        let new_watch_list = model::NewPandaWatchList {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
            account_id,
            kind: params.kind(),
            query: Some(params.query()),
        };
        let result = diesel::insert_into(panda_watch_list::table)
            .values(&new_watch_list)
            .get_result::<model::PandaWatchList>(db)?;
        tracing::info!(
            "Added panda feed {}: {:?} {:?}, account {}",
            result.id,
            params,
            info,
            account_id
        );
        Self::try_from(result)
    }

    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView> {
        use bottle_core::schema::panda_watch_list;
        let update = model::PandaWatchListUpdate {
            name: info.name.clone(),
            watching: info.watching,
            first_fetch_limit: info.first_fetch_limit,
        };
        diesel::update(panda_watch_list::table.find(self.id))
            .set(&update)
            .execute(db)?;
        self.name = info.name.clone();
        self.watching = info.watching;
        self.first_fetch_limit = info.first_fetch_limit;
        tracing::info!("Modified panda feed {}: {:?}", self.id, info);
        Ok(self.view())
    }

    fn save(&self, db: Database, fetched: &Self::FetchResult, ctx: &Self::FetchContext) -> Result<SaveResult> {
        use bottle_core::schema::{
            panda_gallery, panda_gallery_tag, panda_tag, panda_watch_list, panda_watch_list_gallery,
            panda_watch_list_history,
        };

        // (a) If response is empty, we should stop updating
        let empty_result = fetched.galleries.is_empty();
        let no_more_result = match ctx.direction {
            Direction::Forward => fetched.prev_page_offset.is_none(),
            Direction::Backward => fetched.next_page_offset.is_none(),
        };
        // (a*) If we are fetching backward, this means the feed reached end and we should mark it
        let reached_end = matches!(ctx.direction, Direction::Backward) && (no_more_result || empty_result);
        if reached_end {
            diesel::update(panda_watch_list::table.find(self.id))
                .set(panda_watch_list::reached_end.eq(true))
                .execute(db)?;
            tracing::info!("Set panda feed {} as reached end", self.id);
        }
        if empty_result {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end,
            });
        }

        // 1. Filter out posts that are already in the feed
        let fetched_ids = fetched.galleries.iter().map(|g| g.gid as i64).collect::<Vec<_>>();
        let existing_ids = panda_watch_list_gallery::table
            .filter(panda_watch_list_gallery::watch_list_id.eq(self.id))
            .filter(panda_watch_list_gallery::gallery_id.eq_any(&fetched_ids))
            .select(panda_watch_list_gallery::gallery_id)
            .load::<i64>(db)?;
        let existing_ids = existing_ids.into_iter().collect::<HashSet<_>>();
        let has_existing_result = !existing_ids.is_empty();
        let galleries = fetched
            .galleries
            .iter()
            .filter(|g| !existing_ids.contains(&(g.gid as i64)));

        // (b) If no posts are new, stop
        if galleries.clone().count() == 0 {
            return Ok(SaveResult {
                post_ids: vec![],
                should_stop: true,
                reached_end,
            });
        }

        // 2. Prepare data to insert
        // Gallery, Tag, GalleryTag
        let new_galleries = galleries.clone().map(model::NewPandaGallery::from).collect::<Vec<_>>();
        let tags = galleries.clone().flat_map(util::tags).collect::<Vec<_>>();
        let gallery_tags = galleries.clone().flat_map(util::gallery_tags).collect::<Vec<_>>();

        // WatchListGallery
        let watch_list_galleries = galleries
            .clone()
            .map(|g| model::PandaWatchListGallery {
                watch_list_id: self.id,
                gallery_id: g.gid as i64,
                sort_index: None,
                stale: false,
            })
            .collect::<Vec<_>>();

        // WatchListHistory
        let gallery_ids = galleries.clone().map(|g| g.gid.to_string()).collect::<Vec<_>>();
        let prev_offset = match fetched.prev_page_offset.as_ref() {
            Some(GalleryListOffset::NewerThan(offset)) => Some(offset.to_string()),
            _ => gallery_ids.first().cloned(),
        };
        let next_offset = match fetched.next_page_offset.as_ref() {
            Some(GalleryListOffset::OlderThan(offset)) => Some(offset.to_string()),
            _ => gallery_ids.last().cloned(),
        };
        let history = model::NewPandaWatchListHistory {
            watch_list_id: self.id,
            ids: gallery_ids.join(", "),
            count: galleries.clone().count() as i32,
            prev_offset,
            next_offset,
        };

        // 3. Insert data
        db.transaction(|conn| -> Result<()> {
            diesel::insert_into(panda_gallery::table)
                .values(&new_galleries)
                .execute(conn)?;
            diesel::insert_into(panda_tag::table).values(&tags).execute(conn)?;
            diesel::insert_into(panda_gallery_tag::table)
                .values(&gallery_tags)
                .execute(conn)?;
            diesel::insert_into(panda_watch_list_gallery::table)
                .values(&watch_list_galleries)
                .execute(conn)?;
            diesel::insert_into(panda_watch_list_history::table)
                .values(&history)
                .execute(conn)?;
            Ok(())
        })?;

        // TODO: If first fetch limit is reached, mark feed as reached end

        tracing::info!("Saved posts to panda feed {}: {}", self.id, history.ids);
        Ok(SaveResult {
            post_ids: gallery_ids,
            should_stop: has_existing_result || no_more_result,
            reached_end: false,
        })
    }

    fn handle_before_update(&self, db: Database) -> Result<()> {
        // Delete watch list posts that don't have sort index
        use bottle_core::schema::panda_watch_list_gallery::dsl::*;
        use itertools::Itertools;

        let post_ids = panda_watch_list_gallery
            .filter(watch_list_id.eq(self.id))
            .filter(sort_index.is_null())
            .select(gallery_id)
            .load::<i64>(db)?;
        if post_ids.is_empty() {
            return Ok(());
        }

        diesel::delete(
            panda_watch_list_gallery
                .filter(watch_list_id.eq(self.id))
                .filter(sort_index.is_null()),
        )
        .execute(db)?;
        tracing::info!(
            "Deleted {} posts without sort index from panda feed {}: {}",
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
        use bottle_core::schema::panda_watch_list_gallery::dsl::*;

        // 1. Get the last sort index
        let last_sort_index = panda_watch_list_gallery
            .filter(watch_list_id.eq(self.id))
            .order(sort_index.desc())
            .select(sort_index)
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
            for (post_id, index) in post_ids.iter().zip(sort_indices) {
                diesel::update(panda_watch_list_gallery)
                    .filter(watch_list_id.eq(self.id))
                    .filter(gallery_id.eq(post_id.parse::<i64>()?))
                    .set(sort_index.eq(index))
                    .execute(conn)?;
            }
            Ok(())
        })?;

        tracing::info!(
            "Updated sort indices of {} posts in panda feed {}",
            post_ids.len(),
            self.id
        );
        Ok(())
    }

    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{panda_gallery, panda_media, panda_watch_list_gallery};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch posts
        let (posts, total_items) = panda_watch_list_gallery::table
            .inner_join(panda_gallery::table)
            .filter(panda_watch_list_gallery::watch_list_id.eq(self.id))
            .order(panda_watch_list_gallery::sort_index.desc())
            .select(panda_gallery::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::PandaGallery>(db)?;

        // 2. Fetch associated media
        let post_ids = posts.iter().map(|gallery| gallery.id);
        let post_id_strs = posts.iter().map(|gallery| gallery.id.to_string());
        let media = panda_media::table
            .filter(panda_media::gallery_id.eq_any(post_ids.clone()))
            .order(panda_media::media_index.asc())
            .load::<model::PandaMedia>(db)?;

        let tag_map = util::get_tag_map(db, post_ids.clone())?;
        let posts = posts
            .iter()
            .map(|gallery| gallery.post_view(tag_map.get(&gallery.id).cloned().unwrap_or_default()))
            .collect::<Vec<_>>();

        // 3. Fetch associated works
        let (works, images) = bottle_library::get_works_by_post_ids(db, "panda", post_id_strs, false)?;

        // 4. Fetch associated users
        let users = util::get_artist_views(db, post_ids.clone())?;

        Ok(GeneralResponse {
            posts: Some(posts),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            users: Some(users),
            works: Some(works),
            images: Some(images),
            total_items,
            page,
            page_size,
        })
    }

    fn get_account(&self, db: Database) -> Result<Self::Account> {
        PandaAccount::get(db, self.account_id)?.ok_or(Error::NotLoggedIn("Invalid account".to_string()))
    }

    fn get_fetch_context(&self, db: Database) -> Result<Self::FetchContext> {
        let offset = match self.params {
            PandaFeedParams::Search { .. } | PandaFeedParams::Watched { .. } if self.reached_end => {
                self.prev_offset(db)?
            }
            PandaFeedParams::Search { .. } | PandaFeedParams::Watched { .. } if !self.reached_end => {
                self.next_offset(db)?
            }
            // For favorites feed, offset is in format of "gid:timestamp"
            // But the timestamp can't be precisely retrieved from webpage
            // So we can't determine the offset for favorites feed
            _ => None,
        };
        let direction = match offset {
            Some(GalleryListOffset::NewerThan(_)) => Direction::Forward,
            _ => Direction::Backward,
        };
        Ok(Self::FetchContext { offset, direction })
    }

    async fn fetch(&self, ctx: &mut Self::FetchContext, auth: Option<&Self::Auth>) -> Result<Self::FetchResult> {
        let Some(auth) = auth else {
            return Err(Error::NotLoggedIn("Panda feed needs an account".to_string()));
        };
        let client = PandaClient::new(auth.clone()).map_err(anyhow::Error::from)?;
        let offset = ctx.offset.as_ref();
        let result = match self.params {
            PandaFeedParams::Search { ref option } => client.search(option, offset).await,
            PandaFeedParams::Watched { ref option } => client.watched(option, offset).await,
            PandaFeedParams::Favorites { ref option } => client.favorites(option, offset).await,
        }
        .map_err(anyhow::Error::from)?;
        ctx.offset = match ctx.direction {
            Direction::Forward => result.prev_page_offset.clone(),
            Direction::Backward => result.next_page_offset.clone(),
        };
        Ok(result)
    }

        fn archived_posts(db: Database, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::library::{ImageView, WorkView};
        use bottle_core::schema::{image, panda_gallery, panda_media, work};
        use bottle_library::model::{Image, Work};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch works
        let (works, total_items) = work::table
            .filter(work::source.eq("panda"))
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
        let post_ids = works.iter().filter_map(|work| work.post_id_int);
        let posts = panda_gallery::table
            .filter(panda_gallery::id.eq_any(post_ids.clone()))
            .load::<model::PandaGallery>(db)?;

        let tag_map = util::get_tag_map(db, post_ids.clone())?;
        let posts = posts
            .into_iter()
            .map(|gallery| gallery.post_view(tag_map.get(&gallery.id).cloned().unwrap_or_default()))
            .collect();

        // 4. Fetch associated media
        let media = panda_media::table
            .filter(panda_media::gallery_id.eq_any(post_ids.clone()))
            .order(panda_media::media_index.asc())
            .load::<model::PandaMedia>(db)?;

        // 5. Fetch associated users
        let users = util::get_artist_views(db, post_ids)?;

        Ok(GeneralResponse {
            posts: Some(posts),
            media: Some(media.into_iter().map(MediaView::from).collect()),
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
        use diesel::sql_query;
        let query = sql_query(group::grouped_by_user_query(
            "select distinct panda_gallery.* from panda_gallery
            join work on panda_gallery.id = work.post_id_int
            where work.source = 'panda'",
            "order by created_date desc",
        ))
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count)
    }

        fn archived_posts_by_user(db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{panda_gallery, panda_gallery_tag, work};
        use bottle_util::diesel_ext::Paginate;

        let results = panda_gallery::table
            .inner_join(panda_gallery_tag::table)
            .inner_join(work::table.on(work::post_id_int.eq(panda_gallery::id.nullable())))
            .filter(
                panda_gallery_tag::namespace
                    .eq("artist")
                    .and(panda_gallery_tag::name.eq(&user_id)),
            )
            .filter(work::source.eq("panda"))
            .order(panda_gallery::created_date.desc())
            .select(panda_gallery::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::PandaGallery>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size)
    }

        fn feed_posts_grouped_by_user(
        &self,
        db: Database,
        page: i64,
        page_size: i64,
        recent_count: i64,
    ) -> Result<GeneralResponse> {
        use diesel::{sql_query, sql_types::Integer};
        let query = sql_query(group::grouped_by_user_query(
            "select panda_gallery.*, sort_index from panda_watch_list_gallery
            join panda_gallery on panda_watch_list_gallery.gallery_id = panda_gallery.id
            where watch_list_id = ?",
            "order by sort_index desc",
        ))
        .bind::<Integer, _>(self.id)
        .into_boxed();
        group::posts_grouped_by_user(db, query, page, page_size, recent_count)
    }

        fn feed_posts_by_user(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{panda_gallery, panda_gallery_tag, panda_watch_list_gallery};
        use bottle_util::diesel_ext::Paginate;

        let results = panda_watch_list_gallery::table
            .inner_join(panda_gallery::table.inner_join(panda_gallery_tag::table))
            .filter(
                panda_watch_list_gallery::watch_list_id
                    .eq(self.id)
                    .and(panda_gallery_tag::namespace.eq("artist"))
                    .and(panda_gallery_tag::name.eq(&user_id)),
            )
            .order(panda_watch_list_gallery::sort_index.desc())
            .select(panda_gallery::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::PandaGallery>(db)?;
        group::posts_by_user(db, results, user_id, page, page_size)
    }
}

// MARK: Helpers

impl PandaFeed {
    fn prev_offset(&self, db: Database) -> Result<Option<GalleryListOffset>> {
        use bottle_core::schema::panda_watch_list_history::dsl::*;
        let result = panda_watch_list_history
            .filter(watch_list_id.eq(self.id))
            .order((prev_offset.desc(), updated_date.desc()))
            .select(prev_offset)
            .first::<Option<String>>(db)
            .optional()?;
        Ok(result.flatten().map(GalleryListOffset::NewerThan))
    }

    fn next_offset(&self, db: Database) -> Result<Option<GalleryListOffset>> {
        use bottle_core::schema::panda_watch_list_history::dsl::*;
        let result = panda_watch_list_history
            .filter(watch_list_id.eq(self.id))
            .order((next_offset.asc(), updated_date.desc()))
            .select(next_offset)
            .first::<Option<String>>(db)
            .optional()?;
        Ok(result.flatten().map(GalleryListOffset::OlderThan))
    }
}

impl PandaFeedParams {
    fn kind(&self) -> String {
        match self {
            PandaFeedParams::Search { .. } => "search".to_string(),
            PandaFeedParams::Watched { .. } => "watched".to_string(),
            PandaFeedParams::Favorites { .. } => "favorites".to_string(),
        }
    }

    fn query(&self) -> String {
        match self {
            PandaFeedParams::Search { option } => option.to_string(),
            PandaFeedParams::Watched { option } => option.to_string(),
            PandaFeedParams::Favorites { option } => option.to_string(),
        }
    }
}

impl Display for PandaFeedParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PandaFeedParams::Search { option } => option
                .keyword
                .as_ref()
                .map(|s| format!("Search: {}", s))
                .unwrap_or("Homepage".to_string()),
            PandaFeedParams::Watched { .. } => "Watched".to_string(),
            PandaFeedParams::Favorites { option } => format!(
                "Favorites {}{}",
                option
                    .category_index
                    .map(|i| i.to_string())
                    .unwrap_or("all".to_string()),
                option.keyword.as_ref().map(|s| format!(": {}", s)).unwrap_or_default()
            ),
        };
        write!(f, "{}", s)
    }
}
