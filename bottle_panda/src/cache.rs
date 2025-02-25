use std::collections::HashMap;

use diesel::prelude::*;

use bottle_core::{Database, Result};
use panda_client::{Gallery, GalleryDetail, ImagePreview, ImageResult};

use crate::model;

#[derive(Debug, Clone, Default)]
pub struct PandaCache {
    pub(crate) galleries: HashMap<u64, Gallery>,
    pub(crate) gallery_details: HashMap<u64, GalleryDetail>,
    pub(crate) image_previews: HashMap<u64, Vec<ImagePreview>>,
    pub(crate) images: HashMap<u64, Vec<ImageResult>>,
}

impl PandaCache {
    pub fn new() -> Self {
        Self {
            galleries: HashMap::new(),
            gallery_details: HashMap::new(),
            image_previews: HashMap::new(),
            images: HashMap::new(),
        }
    }
}

pub(crate) fn get_gallery(db: Database, cache: &PandaCache, post_id: i64) -> Result<Option<model::PandaGallery>> {
    use bottle_core::schema::panda_gallery;

    // 1. Try to get the post from database
    let mut gallery = panda_gallery::table
        .filter(panda_gallery::id.eq(post_id))
        .first::<model::PandaGallery>(db)
        .optional()?;

    // 2. If not found, try to get the post from entity cache and store it
    if gallery.is_none() {
        if let Some(g) = cache.galleries.get(&(post_id as u64)) {
            let new_gallery = model::NewPandaGallery::from(g);
            gallery = Some(
                diesel::insert_into(panda_gallery::table)
                    .values(&new_gallery)
                    .returning(model::PandaGallery::as_returning())
                    .get_result(db)?,
            );
            tracing::info!("Added panda gallery {} from cache", post_id)
        }
    }

    // 3. If found, try to update the post from entity cache
    if gallery.is_some() {
        if let Some(detail) = cache.gallery_details.get(&(post_id as u64)) {
            let update = model::PandaGalleryUpdate::from(detail);
            gallery = Some(
                diesel::update(panda_gallery::table.filter(panda_gallery::id.eq(post_id)))
                    .set(&update)
                    .returning(model::PandaGallery::as_returning())
                    .get_result(db)?,
            );
            tracing::info!("Updated panda gallery {} from cache", post_id)
        }
    }

    Ok(gallery)
}

pub(crate) fn get_media(db: Database, cache: &PandaCache, post_id: i64) -> Result<Vec<model::PandaMedia>> {
    use bottle_core::schema::panda_media;

    // 1. Try to get the post from database
    let mut media: Vec<model::PandaMedia> = panda_media::table
        .filter(panda_media::gallery_id.eq(post_id))
        .order(panda_media::media_index.asc())
        .load::<model::PandaMedia>(db)?;
    let mut media_dict = media
        .iter()
        .map(|m| (m.media_index as u32, m.clone()))
        .collect::<HashMap<_, _>>();

    // 2. Try to insert non-existing media from entity cache
    let mut updated_anything = false;
    if let Some(previews) = cache.image_previews.get(&(post_id as u64)) {
        let mut new_media_list = Vec::new();
        for preview in previews.iter() {
            media_dict.entry(preview.index).or_insert_with(|| {
                let new_media = model::PandaMedia {
                    gallery_id: post_id,
                    media_index: preview.index as i32,
                    token: preview.token.clone(),
                    thumbnail_url: Some(preview.thumbnail_url.clone()),
                    ..Default::default()
                };
                new_media_list.push(new_media.clone());
                new_media
            });
        }
        if !new_media_list.is_empty() {
            diesel::insert_into(panda_media::table)
                .values(&new_media_list)
                .execute(db)?;
            updated_anything = true;
            tracing::info!(
                "Added {} new media for panda gallery {} from cache",
                new_media_list.len(),
                post_id
            );
        }
    }

    // 3. Try to update media info from entity cache
    if let Some(images) = cache.images.get(&(post_id as u64)) {
        for image in images.iter() {
            if media_dict.contains_key(&image.index) && media_dict[&image.index].url.is_none() {
                let update = model::PandaMediaUpdate::from(image);
                diesel::update(
                    panda_media::table
                        .filter(panda_media::gallery_id.eq(post_id))
                        .filter(panda_media::media_index.eq(image.index as i32)),
                )
                .set(&update)
                .execute(db)?;
                updated_anything = true;
                tracing::info!("Updated panda media {}-{} from cache", post_id, image.index);
            }
        }
    }

    // 4. Refetch media from database if anything is updated
    if updated_anything {
        media = panda_media::table
            .filter(panda_media::gallery_id.eq(post_id))
            .order(panda_media::media_index.asc())
            .load::<model::PandaMedia>(db)?;
    }
    Ok(media)
}
