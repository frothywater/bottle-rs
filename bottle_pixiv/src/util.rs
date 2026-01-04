use std::collections::HashMap;
use std::str::FromStr;

use diesel::prelude::*;

use crate::community::PixivAccount;
use crate::feed::{PixivFeed, PixivFeedParams};
use crate::{model, PixivIllustExtra};

use bottle_core::{
    feed::{MediaView, PostView, UserView},
    library::{RemoteImage, RemoteWork},
    Database, Error, Result,
};
use pixiv_client::{self as client, FollowingRestriction, IllustType, Restriction};

pub(crate) fn get_tag_map(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<HashMap<i64, Vec<String>>> {
    use bottle_core::schema::pixiv_illust_tag;

    let records = pixiv_illust_tag::table
        .filter(pixiv_illust_tag::illust_id.eq_any(post_ids))
        .load::<model::PixivIllustTag>(db)?;

    let mut tag_map = HashMap::new();
    for record in records {
        tag_map
            .entry(record.illust_id)
            .or_insert_with(Vec::new)
            .push(record.tag);
    }

    Ok(tag_map)
}

impl From<&client::User> for model::NewPixivUser {
    fn from(user: &client::User) -> Self {
        Self {
            id: user.id as i64,
            name: user.name.clone(),
            username: user.username.clone(),
            profile_image_url: user.profile_image_urls.medium.clone(),
            description: String::from(""),
            url: None,
            pawoo_url: None,
            twitter_username: None,
        }
    }
}

pub(crate) fn user_view(user: &client::User) -> UserView {
    bottle_util::UserViewBuilder::new(user.id.to_string(), "pixiv")
        .name(Some(user.name.clone()))
        .username(Some(user.username.clone()))
        .avatar_url(user.profile_image_urls.medium.clone())
        .build()
}

impl From<&client::Illust> for model::NewPixivIllust {
    fn from(illust: &client::Illust) -> Self {
        Self {
            id: illust.id as i64,
            user_id: illust.user.id as i64,
            type_: illust.type_.clone(),
            title: illust.title.clone(),
            caption: illust.caption.clone(),
            restrict: illust.restrict > 0,
            sanity_level: illust.sanity_level as i32,
            series_id: illust.series.as_ref().map(|s| s.id as i64),
            series_title: illust.series.as_ref().map(|s| s.title.clone()),
            thumbnail_url: illust.image_urls.large.clone(),
            created_date: illust.create_date.naive_utc(),
        }
    }
}

pub(crate) fn illust_extra(illust: &client::Illust) -> PixivIllustExtra {
    PixivIllustExtra {
        title: illust.title.clone(),
        type_: illust.type_.clone(),
        restrict: illust.restrict > 0,
        sanity_level: illust.sanity_level as i32,
        series_id: illust.series.as_ref().map(|s| s.id as i64),
        series_title: illust.series.as_ref().map(|s| s.title.clone()),
    }
}

pub(crate) fn post_view(illust: &client::Illust) -> PostView {
    bottle_util::PostViewBuilder::new(illust.id.to_string(), "pixiv")
        .user_id(Some(illust.user.id.to_string()))
        .text(illust.title.clone())
        .thumbnail_url(Some(illust.image_urls.large.clone()))
        .tags(Some(illust.tags.iter().map(|tag| tag.name.clone()).collect()))
        .media_count(Some(illust.page_count as i32))
        .created_date(illust.create_date)
        .extra(Some(illust_extra(illust).into()))
        .build()
}

pub(crate) fn media(illust: &client::Illust) -> Vec<model::PixivMedia> {
    if illust.page_count == 1 {
        vec![model::PixivMedia {
            illust_id: illust.id as i64,
            page: 0,
            square_medium_url: illust.image_urls.square_medium.clone(),
            medium_url: illust.image_urls.medium.clone(),
            large_url: illust.image_urls.large.clone(),
            original_url: illust.meta_single_page.original_image_url.clone().unwrap_or_default(),
            width: illust.width as i32,
            height: illust.height as i32,
        }]
    } else {
        illust
            .meta_pages
            .iter()
            .enumerate()
            .map(|(page, meta)| model::PixivMedia {
                illust_id: illust.id as i64,
                page: page as i32,
                square_medium_url: meta.image_urls.square_medium.clone(),
                medium_url: meta.image_urls.medium.clone(),
                large_url: meta.image_urls.large.clone(),
                original_url: meta.image_urls.original.clone().unwrap_or_default(),
                width: illust.width as i32,
                height: illust.height as i32,
            })
            .collect()
    }
}

pub(crate) fn tags(illust: &client::Illust) -> Vec<model::PixivIllustTag> {
    illust
        .tags
        .iter()
        .map(|tag| model::PixivIllustTag {
            illust_id: illust.id as i64,
            tag: tag.name.clone(),
        })
        .collect()
}

impl From<model::PixivAccount> for PixivAccount {
    fn from(account: model::PixivAccount) -> Self {
        PixivAccount {
            id: account.id,
            expiry: account.expiry,
            user_id: account.user_id,
            name: account.name,
            username: account.username,
            profile_image_url: account.profile_image_url,
        }
    }
}

impl TryFrom<model::PixivWatchList> for PixivFeed {
    type Error = Error;
    fn try_from(watch_list: model::PixivWatchList) -> Result<Self> {
        Ok(PixivFeed {
            id: watch_list.id,
            name: watch_list.name,
            first_fetch_limit: watch_list.first_fetch_limit,
            watching: watch_list.watching,
            account_id: watch_list.account_id,
            params: match watch_list.kind.as_str() {
                "timeline" => PixivFeedParams::Timeline {
                    restriction: FollowingRestriction::from_str(&watch_list.restriction.ok_or(
                        Error::ObjectNotComplete("pixiv restriction cannot be null for timeline feed".to_string()),
                    )?)
                    .map_err(anyhow::Error::from)?,
                },
                "bookmarks" => PixivFeedParams::Bookmarks {
                    user_id: watch_list.user_id.ok_or(Error::ObjectNotComplete(
                        "pixiv user id cannot be null for bookmarks feed".to_string(),
                    ))?,
                    tag: watch_list.bookmark_tag,
                    restriction: Restriction::from_str(&watch_list.restriction.ok_or(Error::ObjectNotComplete(
                        "pixiv restriction cannot be null for bookmarks feed".to_string(),
                    ))?)
                    .map_err(anyhow::Error::from)?,
                },
                "posts" => PixivFeedParams::Posts {
                    user_id: watch_list.user_id.ok_or(Error::ObjectNotComplete(
                        "pixiv user id cannot be null for posts feed".to_string(),
                    ))?,
                    type_: IllustType::from_str(&watch_list.illust_type.ok_or(Error::ObjectNotComplete(
                        "pixiv restriction cannot be null for posts feed".to_string(),
                    ))?)
                    .map_err(anyhow::Error::from)?,
                },
                "search" => PixivFeedParams::Search {
                    query: watch_list.search_query.ok_or(Error::ObjectNotComplete(
                        "pixiv search query cannot be null for search feed".to_string(),
                    ))?,
                },
                _ => Err(Error::UnknownField(format!(
                    "pixiv watch list kind {}",
                    watch_list.kind
                )))?,
            },
            reached_end: watch_list.reached_end,
        })
    }
}

impl From<model::PixivUser> for UserView {
    fn from(user: model::PixivUser) -> Self {
        bottle_util::UserViewBuilder::new(user.id.to_string(), "pixiv")
            .name(Some(user.name))
            .username(Some(user.username))
            .avatar_url(user.profile_image_url)
            .description(Some(user.description))
            .url(user.url)
            .build()
    }
}

impl model::PixivIllust {
    pub(crate) fn illust_extra(&self) -> PixivIllustExtra {
        PixivIllustExtra {
            title: self.title.clone(),
            type_: self.type_.clone(),
            restrict: self.restrict,
            sanity_level: self.sanity_level,
            series_id: self.series_id,
            series_title: self.series_title.clone(),
        }
    }

    pub fn post_view(&self, tags: Vec<String>) -> PostView {
        bottle_util::PostViewBuilder::new(self.id.to_string(), "pixiv")
            .user_id(Some(self.user_id.to_string()))
            .text(self.title.clone())
            .thumbnail_url(Some(self.thumbnail_url.clone()))
            .tags(Some(tags))
            .created_date_naive(self.created_date)
            .added_date_naive(Some(self.added_date))
            .extra(Some(self.illust_extra().into()))
            .build()
    }
}

impl From<model::PixivMedia> for MediaView {
    fn from(media: model::PixivMedia) -> Self {
        bottle_util::MediaViewBuilder::new(
            format!("{}_{}", media.illust_id, media.page),
            "pixiv",
            media.illust_id.to_string(),
            media.page,
        )
        .url(Some(media.original_url))
        .dimensions(Some(media.width), Some(media.height))
        .thumbnail_url(Some(media.medium_url))
        .build()
    }
}

/// Convert a Pixiv media to a remote work.
/// Currently, the app only supports one Pixiv media as one work.
impl TryFrom<&model::PixivMedia> for RemoteWork {
    type Error = Error;
    fn try_from(media: &model::PixivMedia) -> Result<Self> {
        let image = RemoteImage {
            filename: bottle_util::parse_filename(&media.original_url).map_err(anyhow::Error::from)?,
            url: media.original_url.clone(),
            page_index: None,
        };
        Ok(RemoteWork {
            source: Some(String::from("pixiv")),
            post_id: Some(media.illust_id.to_string()),
            post_id_int: Some(media.illust_id),
            page_index: Some(media.page),
            media_count: 1,
            images: vec![image],
            ..Default::default()
        })
    }
}

impl From<PixivIllustExtra> for serde_json::Value {
    fn from(extra: PixivIllustExtra) -> Self {
        serde_json::json!({
            "pixiv": serde_json::to_value(extra).expect("cannot serialize pixiv illust extra")
        })
    }
}
