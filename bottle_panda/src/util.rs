use std::collections::HashMap;
use std::str::FromStr;

use diesel::prelude::*;

use bottle_core::{
    feed::{MediaView, PostView, Scheme, UserView},
    Database, Error, Result,
};
use panda_client::{
    FavoriteSearchOption, Gallery, GalleryCategory, GalleryDetail, GalleryPageResult, ImageResult, SearchOption,
};

use crate::community::{PandaAccount, PandaGalleryExtra};
use crate::feed::{PandaFeed, PandaFeedParams};
use crate::model;

pub(crate) fn get_tag_map(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<HashMap<i64, Vec<String>>> {
    use bottle_core::schema::panda_gallery_tag;

    let records = panda_gallery_tag::table
        .filter(panda_gallery_tag::gallery_id.eq_any(post_ids))
        .load::<model::PandaGalleryTag>(db)?;

    let mut tag_map = HashMap::new();
    for record in records {
        let tags = tag_map.entry(record.gallery_id).or_insert_with(Vec::new);
        tags.push(format!("{}:{}", record.namespace, record.name));
    }

    Ok(tag_map)
}

pub(crate) fn get_artist_views(db: Database, post_ids: impl Iterator<Item = i64>) -> Result<Vec<UserView>> {
    use bottle_core::schema::panda_gallery_tag::dsl::*;
    let names = panda_gallery_tag
        .filter(gallery_id.eq_any(post_ids))
        .filter(namespace.eq("artist"))
        .select(name)
        .distinct()
        .load::<String>(db)?;
    let views = names.into_iter().map(artist_view).collect();
    Ok(views)
}

pub(crate) fn artist_view(artist: impl Into<String>) -> UserView {
    let artist = artist.into();
    UserView {
        user_id: artist.clone(),
        name: Some(artist.clone()),
        tag_name: Some(format!("artist:{}", artist)),
        community: "panda".to_string(),
        ..Default::default()
    }
}

pub(crate) fn tags(gallery: &Gallery) -> Vec<model::PandaTag> {
    gallery
        .tags
        .iter()
        .map(|t| model::PandaTag {
            namespace: t.namespace.to_string(),
            name: t.name.clone(),
        })
        .collect()
}

pub(crate) fn gallery_tags(gallery: &Gallery) -> Vec<model::PandaGalleryTag> {
    gallery
        .tags
        .iter()
        .map(|t| model::PandaGalleryTag {
            gallery_id: gallery.gid as i64,
            namespace: t.namespace.to_string(),
            name: t.name.clone(),
        })
        .collect()
}

pub(crate) fn media(page: &GalleryPageResult) -> Vec<model::PandaMedia> {
    page.previews
        .iter()
        .map(|image| model::PandaMedia {
            gallery_id: page.gallery.gid as i64,
            media_index: image.index as i32,
            token: image.token.clone(),
            thumbnail_url: Some(image.thumbnail_url.clone()),
            ..Default::default()
        })
        .collect()
}

pub(crate) fn gallery_extra(gallery: &Gallery) -> PandaGalleryExtra {
    PandaGalleryExtra {
        token: gallery.token.clone(),
        category: gallery.category.to_string(),
        uploader: gallery.uploader.clone().unwrap_or_default(),
        rating: gallery.rating,
        ..Default::default()
    }
}

pub(crate) fn post_view(gallery: &Gallery) -> PostView {
    PostView {
        post_id: gallery.gid.to_string(),
        user_id: None,
        community: "panda".to_string(),
        text: gallery.title.clone(),
        media_count: Some(gallery.image_count as i32),
        thumbnail_url: Some(gallery.thumbnail_url.clone()),
        tags: Some(gallery.tags.iter().map(|tag| tag.to_string()).collect()),
        created_date: gallery.posted_date,
        added_date: None,
        extra: Some(gallery_extra(gallery).into()),
    }
}

impl From<&Gallery> for model::NewPandaGallery {
    fn from(gallery: &Gallery) -> Self {
        Self {
            id: gallery.gid as i64,
            token: gallery.token.clone(),
            title: gallery.title.clone(),
            thumbnail_url: gallery.thumbnail_url.clone(),
            category: gallery.category as i32,
            uploader: gallery.uploader.clone().unwrap_or_default(),
            rating: gallery.rating,
            media_count: gallery.image_count as i32,
            created_date: gallery.posted_date.naive_utc(),
        }
    }
}

impl From<&GalleryDetail> for model::PandaGalleryUpdate {
    fn from(gallery: &GalleryDetail) -> Self {
        Self {
            english_title: Some(gallery.english_title.clone()),
            parent: Some(gallery.parent.clone().unwrap_or_default()),
            visible: Some(gallery.visible),
            language: Some(gallery.language.clone()),
            file_size: Some(gallery.file_size as i32),
            ..Default::default()
        }
    }
}

impl From<model::PandaAccount> for PandaAccount {
    fn from(account: model::PandaAccount) -> Self {
        Self {
            id: account.id,
            name: account.name,
            username: account.username,
        }
    }
}

impl TryFrom<model::PandaWatchList> for PandaFeed {
    type Error = Error;
    fn try_from(watch_list: model::PandaWatchList) -> Result<PandaFeed> {
        Ok(PandaFeed {
            id: watch_list.id,
            name: watch_list.name,
            watching: watch_list.watching,
            first_fetch_limit: watch_list.first_fetch_limit,
            account_id: watch_list.account_id,
            params: match watch_list.kind.as_str() {
                "search" => PandaFeedParams::Search {
                    option: SearchOption::from_str(&watch_list.query.ok_or(Error::ObjectNotComplete(
                        "panda query cannot be null for search feed".to_string(),
                    ))?)
                    .map_err(anyhow::Error::from)?,
                },
                "watched" => PandaFeedParams::Watched {
                    option: SearchOption::from_str(&watch_list.query.ok_or(Error::ObjectNotComplete(
                        "panda query cannot be null for watched feed".to_string(),
                    ))?)
                    .map_err(anyhow::Error::from)?,
                },
                "favorites" => PandaFeedParams::Favorites {
                    option: FavoriteSearchOption::from_str(&watch_list.query.ok_or(Error::ObjectNotComplete(
                        "panda query cannot be null for favorites feed".to_string(),
                    ))?)
                    .map_err(anyhow::Error::from)?,
                },
                _ => Err(Error::UnknownField(format!(
                    "panda watch list kind {}",
                    watch_list.kind
                )))?,
            },
            reached_end: watch_list.reached_end,
        })
    }
}

impl model::PandaGallery {
    pub(crate) fn gallery_extra(&self) -> PandaGalleryExtra {
        PandaGalleryExtra {
            token: self.token.clone(),
            category: GalleryCategory::from_i32(self.category).to_string(),
            uploader: self.uploader.clone(),
            rating: self.rating,
            english_title: self.english_title.clone(),
            parent: self.parent.clone(),
            visible: self.visible,
            language: self.language.clone(),
            file_size: self.file_size,
        }
    }

    pub(crate) fn post_view(&self, tags: Vec<String>) -> PostView {
        PostView {
            post_id: self.id.to_string(),
            user_id: None,
            community: "panda".to_string(),
            text: self.title.clone(),
            media_count: Some(self.media_count),
            thumbnail_url: Some(self.thumbnail_url.clone()),
            tags: Some(tags),
            created_date: self.created_date.and_utc(),
            added_date: Some(self.added_date.and_utc()),
            extra: Some(self.gallery_extra().into()),
        }
    }

    pub(crate) fn has_detail(&self) -> bool {
        self.english_title.is_some()
            && self.parent.is_some()
            && self.visible.is_some()
            && self.language.is_some()
            && self.file_size.is_some()
    }
}

impl From<model::PandaMedia> for MediaView {
    fn from(media: model::PandaMedia) -> Self {
        Self {
            media_id: media.id(),
            community: "panda".to_string(),
            post_id: media.gallery_id.to_string(),
            page_index: media.media_index,
            url: media.url,
            width: media.width,
            height: media.height,
            thumbnail_url: media.thumbnail_url,
            extra: Some(serde_json::json!({"panda": {"token": media.token}})),
        }
    }
}

impl model::PandaMedia {
    fn id(&self) -> String {
        format!("{}-{}", self.gallery_id, self.media_index)
    }
}

impl From<&ImageResult> for model::PandaMediaUpdate {
    fn from(image: &ImageResult) -> Self {
        Self {
            url: Some(image.url.clone()),
            filename: Some(image.filename.clone()),
            file_size: Some(image.file_size as i32),
            width: Some(image.width as i32),
            height: Some(image.height as i32),
        }
    }
}

impl PandaGalleryExtra {
    pub(crate) fn with_detail(&self, detail: &GalleryDetail) -> Self {
        Self {
            english_title: Some(detail.english_title.clone()),
            parent: detail.parent.clone(),
            visible: Some(detail.visible),
            language: Some(detail.language.clone()),
            file_size: Some(detail.file_size as i32),
            ..self.clone()
        }
    }
}

impl From<PandaGalleryExtra> for serde_json::Value {
    fn from(extra: PandaGalleryExtra) -> Self {
        serde_json::json!({
            "panda": serde_json::to_value(extra).expect("cannot serialize gallery extra")
        })
    }
}

pub(crate) fn search_option_scheme() -> Scheme {
    Scheme::Object(HashMap::from([
        ("keyword".to_string(), Scheme::Optional(Box::new(Scheme::String))),
        ("categories".to_string(), Scheme::Array(Box::new(Scheme::String))),
        ("search_name".to_string(), Scheme::Bool),
        ("search_tags".to_string(), Scheme::Bool),
        ("search_description".to_string(), Scheme::Bool),
        ("search_torrent".to_string(), Scheme::Bool),
        ("search_low_power_tags".to_string(), Scheme::Bool),
        ("search_downvoted_tags".to_string(), Scheme::Bool),
        ("search_expunged".to_string(), Scheme::Bool),
        ("require_torrent".to_string(), Scheme::Bool),
        ("disable_language_filter".to_string(), Scheme::Bool),
        ("disable_uploader_filter".to_string(), Scheme::Bool),
        ("disable_tags_filter".to_string(), Scheme::Bool),
        ("min_rating".to_string(), Scheme::Optional(Box::new(Scheme::Int))),
        ("min_pages".to_string(), Scheme::Optional(Box::new(Scheme::Int))),
        ("max_pages".to_string(), Scheme::Optional(Box::new(Scheme::Int))),
    ]))
}

pub(crate) fn favorite_search_option_scheme() -> Scheme {
    Scheme::Object(HashMap::from([
        ("keyword".to_string(), Scheme::Optional(Box::new(Scheme::String))),
        ("category_index".to_string(), Scheme::Optional(Box::new(Scheme::Int))),
        ("search_name".to_string(), Scheme::Bool),
        ("search_tags".to_string(), Scheme::Bool),
        ("search_note".to_string(), Scheme::Bool),
    ]))
}
