use diesel::prelude::*;

use bottle_core::{
    feed::{MediaView, PostView, UserView},
    library::{RemoteImage, RemoteWork},
    Database, Error, Result,
};
use yandere_client::{self as client};

use crate::model;
use crate::{
    community::YanderePostExtra,
    feed::{YandereFeed, YandereFeedParams},
};

pub(crate) fn artist_view(artist: impl Into<String>) -> UserView {
    let artist = artist.into();
    UserView {
        user_id: artist.clone(),
        name: Some(artist.replace("_", " ")),
        tag_name: Some(artist),
        community: "yandere".to_string(),
        ..Default::default()
    }
}

pub(crate) fn get_artist_views(db: Database, post_ids: impl Iterator<Item = i64>) -> Result<Vec<UserView>> {
    use bottle_core::schema::{yandere_post_tag, yandere_tag};
    let names = yandere_post_tag::table
        .inner_join(yandere_tag::table)
        .filter(yandere_post_tag::post_id.eq_any(post_ids))
        .filter(yandere_tag::type_.eq("artist"))
        .select(yandere_tag::name)
        .distinct()
        .load::<String>(db)?;
    let views = names.into_iter().map(artist_view).collect();
    Ok(views)
}

impl From<&client::PostResult> for model::NewYanderePost {
    fn from(post: &client::PostResult) -> Self {
        model::NewYanderePost {
            id: post.id as i64,
            tags: post.tags.clone(),
            creator_id: post.creator_id.map(|v| v as i64),
            author: post.author.clone(),
            url: post.file_url.clone(),
            thumbnail_url: post.sample_url.clone(),
            width: post.width as i32,
            height: post.height as i32,
            file_size: post.file_size as i64,
            file_ext: post.file_ext.clone(),
            rating: post.rating.clone(),
            md5: post.md5.clone(),
            source: post.source.clone(),
            has_children: post.has_children,
            parent_id: post.parent_id.map(|v| v as i64),
            created_date: post.created_at.naive_utc(),
        }
    }
}

pub(crate) fn post_extra_result(post: &client::PostResult) -> YanderePostExtra {
    YanderePostExtra {
        creator_id: post.creator_id.map(|v| v as i64),
        author: post.author.clone(),
        source: post.source.clone(),
        rating: post.rating.clone(),
        file_size: post.file_size as i64,
        has_children: post.has_children,
        parent_id: post.parent_id.map(|v| v as i64),
    }
}

pub(crate) fn post_view(post: &client::PostResult) -> PostView {
    PostView {
        post_id: post.id.to_string(),
        community: "yandere".to_string(),
        user_id: None,
        text: post.source.clone(),
        thumbnail_url: Some(post.sample_url.clone()),
        media_count: Some(1),
        tags: Some(post.tags.split_whitespace().map(|tag| tag.to_string()).collect()),
        created_date: post.created_at,
        added_date: None,
        extra: Some(post_extra_result(post).into()),
    }
}

pub(crate) fn media_view(post: &client::PostResult) -> MediaView {
    MediaView {
        media_id: post.id.to_string(),
        community: "yandere".to_string(),
        post_id: post.id.to_string(),
        page_index: 0,
        url: Some(post.file_url.clone()),
        width: Some(post.width as i32),
        height: Some(post.height as i32),
        thumbnail_url: Some(post.sample_url.clone()),
        ..Default::default()
    }
}

pub(crate) fn post_tags(post: &client::PostResult) -> Vec<model::YanderePostTag> {
    post.tags
        .split_whitespace()
        .map(|tag| model::YanderePostTag {
            post_id: post.id as i64,
            tag_name: tag.to_string(),
        })
        .collect::<Vec<_>>()
}

impl TryFrom<model::YandereWatchList> for YandereFeed {
    type Error = Error;

    fn try_from(watch_list: model::YandereWatchList) -> Result<Self> {
        let params = match watch_list.kind.as_str() {
            "search" => YandereFeedParams::Search {
                query: watch_list.search_query.ok_or(Error::ObjectNotComplete(
                    "yandere search query cannot be null".to_string(),
                ))?,
            },
            _ => Err(Error::UnknownField(format!(
                "yandere watch list kind {}",
                watch_list.kind
            )))?,
        };
        Ok(YandereFeed {
            id: watch_list.id,
            name: watch_list.name,
            watching: watch_list.watching,
            first_fetch_limit: watch_list.first_fetch_limit,
            params,
            reached_end: watch_list.reached_end,
        })
    }
}

pub(crate) fn post_extra(post: &model::YanderePost) -> YanderePostExtra {
    YanderePostExtra {
        creator_id: post.creator_id,
        author: post.author.clone(),
        source: post.source.clone(),
        rating: post.rating.clone(),
        file_size: post.file_size,
        has_children: post.has_children,
        parent_id: post.parent_id,
    }
}

impl From<&model::YanderePost> for PostView {
    fn from(post: &model::YanderePost) -> PostView {
        PostView {
            post_id: post.id.to_string(),
            user_id: None,
            community: "yandere".to_string(),
            text: post.source.clone(),
            thumbnail_url: Some(post.thumbnail_url.clone()),
            media_count: Some(1),
            tags: Some(post.tags.split_whitespace().map(|tag| tag.to_string()).collect()),
            created_date: post.created_date.and_utc(),
            added_date: Some(post.added_date.and_utc()),
            extra: Some(post_extra(post).into()),
        }
    }
}

impl From<&model::YanderePost> for MediaView {
    fn from(post: &model::YanderePost) -> Self {
        MediaView {
            media_id: post.id.to_string(),
            community: "yandere".to_string(),
            post_id: post.id.to_string(),
            page_index: 0,
            url: Some(post.url.clone()),
            width: Some(post.width),
            height: Some(post.height),
            thumbnail_url: Some(post.thumbnail_url.clone()),
            ..Default::default()
        }
    }
}

impl TryFrom<model::YanderePost> for RemoteWork {
    type Error = Error;

    fn try_from(post: model::YanderePost) -> Result<Self> {
        let url = urlencoding::decode(&post.url).map_err(anyhow::Error::from)?;
        let filename = bottle_util::parse_filename(&url).map_err(anyhow::Error::from)?;
        let image = RemoteImage {
            filename,
            url: url.into_owned(),
            page_index: None,
        };
        Ok(RemoteWork {
            source: Some("yandere".to_string()),
            post_id: Some(post.id.to_string()),
            post_id_int: Some(post.id),
            media_count: 1,
            images: vec![image],
            page_index: Some(0),
            ..Default::default()
        })
    }
}

pub(crate) fn tags(result: &client::APIResult) -> Vec<model::YandereTag> {
    result
        .tags
        .iter()
        .map(|(name, type_)| model::YandereTag {
            name: name.clone(),
            type_: type_.to_string(),
        })
        .collect::<Vec<_>>()
}

pub(crate) fn pools(result: &client::APIResult) -> Vec<model::YanderePool> {
    use itertools::Itertools;
    result
        .pools
        .iter()
        .unique_by(|pool| pool.id)
        .map(model::YanderePool::from)
        .collect::<Vec<_>>()
}

pub(crate) fn pool_posts(result: &client::APIResult) -> Vec<model::YanderePoolPost> {
    result
        .pool_posts
        .iter()
        .map(model::YanderePoolPost::from)
        .collect::<Vec<_>>()
}

impl From<&client::PoolResult> for model::YanderePool {
    fn from(pool: &client::PoolResult) -> Self {
        model::YanderePool {
            id: pool.id as i64,
            name: pool.name.clone(),
            description: pool.description.clone(),
            user_id: pool.user_id as i64,
            post_count: pool.post_count as i32,
            created_date: pool.created_at.naive_utc(),
            updated_date: pool.updated_at.naive_utc(),
        }
    }
}

impl From<&client::PoolPostResult> for model::YanderePoolPost {
    fn from(pool_post: &client::PoolPostResult) -> Self {
        model::YanderePoolPost {
            pool_id: pool_post.pool_id as i64,
            post_id: pool_post.post_id as i64,
            sequence: pool_post.sequence.clone(),
            prev_post_id: pool_post.prev_post_id.map(|v| v as i64),
            next_post_id: pool_post.next_post_id.map(|v| v as i64),
        }
    }
}

impl From<YanderePostExtra> for serde_json::Value {
    fn from(extra: YanderePostExtra) -> Self {
        serde_json::json!({
            "yandere": serde_json::to_value(extra).expect("cannot serialize yandere post extra")
        })
    }
}
