use std::collections::HashMap;

use diesel::prelude::*;

use bottle_core::{Database, Error, Result};
use panda_client::{Gallery, GalleryDetail, GalleryPageResult, ImageResult};

use crate::model;
use crate::util;

// MARK: Functions for downloading

#[derive(Debug, Clone)]
pub struct PandaDownloadTask {
    pub gid: i64,
    pub token: String,
    pub title: String,
    pub has_detail: bool,
    pub media_count: i32,
    pub work_id: i32,
    pub image_tasks: Vec<PandaImageTask>,
}

#[derive(Debug, Clone)]
pub struct PandaImageTask {
    pub gid: i64,
    pub index: i32,
    pub token: String,
    pub image_id: Option<i32>,
    pub downloaded: bool,
}

pub fn get_download_task(db: Database, gid: i64) -> Result<PandaDownloadTask> {
    use bottle_core::schema::{image, panda_gallery, panda_media, work};
    use bottle_library::model::{Image, Work};

    // 1. Get work and images
    let work = work::table
        .filter(work::source.eq("panda"))
        .filter(work::post_id_int.eq(gid))
        .first::<Work>(db)?;
    let images = image::table
        .filter(image::work_id.eq(work.id))
        .order(image::page_index.asc())
        .load::<Image>(db)?;

    // 2. Skip if all images are downloaded
    let downloaded = images
        .iter()
        .all(|image| image.path.is_some() || image.thumbnail_path.is_some());
    if downloaded {
        return Err(Error::ObjectAlreadyExists(format!(
            "Panda gallery {} already downloaded",
            gid
        )));
    }

    // 3. Get gallery and media
    let gallery = panda_gallery::table.find(gid).first::<model::PandaGallery>(db)?;
    let media = panda_media::table
        .filter(panda_media::gallery_id.eq(gid))
        .order(panda_media::media_index.asc())
        .load::<model::PandaMedia>(db)?;

    // 4. Create image tasks for media without associated image
    let images = images
        .into_iter()
        .filter_map(|image| image.page_index.map(|index| (index, image)))
        .collect::<HashMap<_, _>>();
    let image_tasks = media
        .into_iter()
        .map(|m| {
            let image = images.get(&m.media_index);
            PandaImageTask {
                gid: gallery.id,
                index: m.media_index,
                token: m.token,
                image_id: image.map(|image| image.id),
                downloaded: image
                    .map(|image| image.path.is_some() || image.thumbnail_path.is_some())
                    .unwrap_or_default(),
            }
        })
        .collect();

    Ok(PandaDownloadTask {
        gid: gallery.id,
        token: gallery.token.clone(),
        title: gallery.title.clone(),
        has_detail: gallery.has_detail(),
        media_count: gallery.media_count,
        work_id: work.id,
        image_tasks,
    })
}

pub fn get_all_download_tasks(db: Database) -> Result<Vec<PandaDownloadTask>> {
    use bottle_core::schema::{image, panda_gallery, panda_media, work};
    use bottle_library::model::{Image, Work};
    use diesel::{dsl::sql, sql_types::Bool};

    // 1. Get galleries in work table with incomplete images
    // Note: diesel seems to not support group by very well... have to do it separately
    let post_ids = work::table
        .left_join(
            image::table.on(image::work_id
                .eq(work::id)
                .and(image::path.is_not_null().or(image::thumbnail_path.is_not_null()))),
        )
        .filter(work::source.eq("panda"))
        .group_by(work::post_id_int)
        .having(sql::<Bool>("count(image.id) < work.image_count"))
        .select(work::post_id_int)
        .load::<Option<i64>>(db)?;
    let post_ids = post_ids.into_iter().flatten().collect::<Vec<_>>();
    let works = work::table
        .filter(work::source.eq("panda"))
        .filter(work::post_id_int.eq_any(&post_ids))
        .load::<Work>(db)?;

    let mut work_id_map = HashMap::new();
    for work in works {
        let post_id = work.post_id_int.expect("post_id_int should be present");
        work_id_map.insert(post_id, work.id);
    }

    // 2. Get galleries
    let galleries = panda_gallery::table
        .filter(panda_gallery::id.eq_any(post_ids.iter()))
        .load::<model::PandaGallery>(db)?;

    // 3. Get media
    let media = panda_media::table
        .filter(panda_media::gallery_id.eq_any(post_ids.iter()))
        .order(panda_media::gallery_id.asc())
        .order(panda_media::media_index.asc())
        .load::<model::PandaMedia>(db)?;

    let mut media_map = HashMap::new();
    for m in media {
        media_map.insert((m.gallery_id, m.media_index), m);
    }

    // 4. Get works and images
    let images = image::table
        .inner_join(work::table)
        .filter(work::source.eq("panda"))
        .filter(work::post_id_int.eq_any(post_ids.iter()))
        .select(image::all_columns)
        .load::<Image>(db)?;
    let mut image_map = HashMap::new();
    for image in images {
        if let Some(page_index) = image.page_index {
            image_map.insert((image.work_id, page_index), image);
        }
    }

    // 5. Combine all stuff
    let mut results = Vec::new();
    for gallery in galleries {
        let work_id = work_id_map
            .get(&gallery.id)
            .unwrap_or_else(|| panic!("Work not found for gallery {}", gallery.id));
        let mut image_tasks = Vec::new();
        for index in 0..gallery.media_count {
            let Some(media) = media_map.get(&(gallery.id, index)) else {
                continue;
            };
            let image = image_map.get(&(*work_id, index));
            image_tasks.push(PandaImageTask {
                gid: gallery.id,
                index,
                token: media.token.clone(),
                image_id: image.map(|image| image.id),
                downloaded: image
                    .map(|image| image.path.is_some() || image.thumbnail_path.is_some())
                    .unwrap_or_default(),
            });
        }
        results.push(PandaDownloadTask {
            gid: gallery.id,
            token: gallery.token.clone(),
            title: gallery.title.clone(),
            has_detail: gallery.has_detail(),
            media_count: gallery.media_count,
            work_id: *work_id,
            image_tasks,
        });
    }

    Ok(results)
}

pub fn update_gallery(db: Database, gallery: &Gallery, detail: &GalleryDetail) -> Result<model::PandaGallery> {
    use bottle_core::schema::panda_gallery;

    let mut update = model::PandaGalleryUpdate::from(detail);
    update.media_count = Some(gallery.image_count as i32);
    let gallery = diesel::update(panda_gallery::table.find(gallery.gid as i64))
        .set(&update)
        .returning(panda_gallery::all_columns)
        .get_result::<model::PandaGallery>(db)?;
    tracing::info!("Updated panda gallery {} detail", gallery.id);
    Ok(gallery)
}

pub fn save_previews(db: Database, page: &GalleryPageResult) -> Result<()> {
    use bottle_core::schema::panda_media;

    let media = util::media(page);
    diesel::insert_into(panda_media::table).values(&media).execute(db)?;
    tracing::info!("Saved {} previews for panda gallery {}", media.len(), page.gallery.gid);
    Ok(())
}

pub fn remove_previews(db: Database, gallery_id: i64) -> Result<()> {
    use bottle_core::schema::panda_media;

    diesel::delete(panda_media::table.filter(panda_media::gallery_id.eq(gallery_id))).execute(db)?;
    tracing::info!("Removed all previews for panda gallery {}", gallery_id);
    Ok(())
}

pub fn save_image(db: Database, gallery_id: i64, image: &ImageResult) -> Result<model::PandaMedia> {
    use bottle_core::schema::panda_media;

    let update = model::PandaMediaUpdate::from(image);
    let media = diesel::update(
        panda_media::table
            .filter(panda_media::gallery_id.eq(gallery_id))
            .filter(panda_media::media_index.eq(image.index as i32)),
    )
    .set(&update)
    .returning(panda_media::all_columns)
    .get_result(db)?;

    tracing::info!("Saved panda media {}-{}", gallery_id, image.index);
    Ok(media)
}
