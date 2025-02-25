use async_trait::async_trait;
use diesel::prelude::*;
use serde::Serialize;

use bottle_core::{feed::*, Result};

use crate::cache::YandereCache;
use crate::feed::YandereFeed;
use crate::{model, util};

pub struct YandereCommunity;

impl Community for YandereCommunity {
    type Auth = ();
    type Credential = ();
    type Account = YandereAccount;
    type Feed = YandereFeed;

    fn metadata() -> CommunityMetadata
    where
        Self: Sized,
    {
        CommunityMetadata {
            name: "yandere".to_string(),
            feeds: YandereFeed::metadata(),
            account: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YandereAccount;

#[async_trait]
impl Account for YandereAccount {
    type Auth = ();
    type Credential = ();
    type InfoResponse = ();

    fn metadata() -> Option<AccountMetadata>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn view(&self) -> AccountView {
        unimplemented!()
    }

    fn info(&self) -> Option<AccountInfo> {
        unimplemented!()
    }

    fn expired(&self) -> bool {
        unimplemented!()
    }

    fn all(_db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn get(_db: Database, _account_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn delete(_db: Database, _account_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn add(_db: Database, _credential: &Self::Credential) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn update(&self, _db: Database, _info: &Self::InfoResponse) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn auth(&self, _db: Database) -> Result<Option<Self::Auth>> {
        unimplemented!()
    }

    fn credential(&self, _db: Database) -> Result<Self::Credential> {
        unimplemented!()
    }

    async fn fetch(_credential: &Self::Credential) -> Result<Self::InfoResponse> {
        unimplemented!()
    }
}

impl Post for model::YanderePost {
    type Cache = YandereCache;

    fn get(db: Database, cache: &Self::Cache, post_id: &str) -> Result<Option<Self>> {
        use bottle_core::schema::{yandere_pool, yandere_pool_post, yandere_post, yandere_post_tag, yandere_tag};
        let post_id = post_id.parse::<i64>()?;

        // 1. Try to get the post from database
        let mut result = yandere_post::table.find(post_id).first(db).optional()?;

        // 1.1 If not found, try to get from cache and save to database
        if result.is_none() {
            if let Some(post) = cache.posts.get(&(post_id as u64)) {
                let new_post = model::NewYanderePost::from(post);

                let post_tags = util::post_tags(post);
                let mut new_tags = Vec::new();
                for tag in post_tags.iter() {
                    if let Some(type_) = cache.tags.get(&tag.tag_name) {
                        new_tags.push(model::YandereTag {
                            name: tag.tag_name.clone(),
                            type_: type_.to_string(),
                        });
                    }
                }

                let mut new_pools = Vec::new();
                let mut pool_posts = Vec::new();
                if let Some(items) = cache.post_pools.get(&post.id) {
                    for item in items {
                        pool_posts.push(model::YanderePoolPost::from(item));
                        if let Some(pool) = cache.pools.get(&item.pool_id) {
                            new_pools.push(model::YanderePool::from(pool));
                        }
                    }
                }

                result = db.transaction(|conn| -> Result<Option<model::YanderePost>> {
                    let result = Some(
                        diesel::insert_into(yandere_post::table)
                            .values(&new_post)
                            .returning(model::YanderePost::as_returning())
                            .get_result(conn)?,
                    );
                    diesel::insert_into(yandere_tag::table)
                        .values(&new_tags)
                        .execute(conn)?;
                    diesel::insert_into(yandere_post_tag::table)
                        .values(&post_tags)
                        .execute(conn)?;
                    diesel::insert_into(yandere_pool::table)
                        .values(&new_pools)
                        .execute(conn)?;
                    diesel::insert_into(yandere_pool_post::table)
                        .values(&pool_posts)
                        .execute(conn)?;
                    Ok(result)
                })?;

                tracing::info!("Saved yandere post {} from cache", post_id);
            }
        }

        Ok(result)
    }

    fn add_to_library(&self, db: Database, _page: Option<i32>) -> Result<GeneralResponse> {
        let remote_work = self.clone().try_into()?;
        bottle_library::add_remote_work(db, &remote_work)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct YanderePostExtra {
    pub creator_id: Option<i64>,
    pub author: String,
    pub source: String,
    pub rating: String,
    pub file_size: i64,
    pub has_children: bool,
    pub parent_id: Option<i64>,
}
