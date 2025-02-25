use std::collections::HashMap;
use std::path::{Path, PathBuf};

use diesel::prelude::*;

use bottle_core::{Database, Result};
use bottle_download::{DownloadTask, LocalImage};

use crate::model;

/// Find the works in the database which are not downloaded yet.
pub fn get_download_tasks(conn: Database, root_dir: impl AsRef<Path>) -> Result<Vec<DownloadTask>> {
    use bottle_core::schema::{image, pixiv_illust, tweet, work};
    use itertools::Itertools;

    // 1. Get the works and images
    let records = image::table
        .inner_join(work::table)
        .filter(work::source.is_not_null().and(work::post_id.is_not_null()))
        .filter(image::path.is_null().and(image::remote_url.is_not_null()))
        .filter(image::remote_url.not_like("%limit_unknown_360.png"))
        .filter(image::remote_url.not_like("%limit_sanity_level_360.png"))
        // Panda needs special handling, so exclude it here
        .filter(work::source.ne("panda"))
        .order_by(work::source.asc())
        .then_order_by(work::post_id.asc())
        .then_order_by(work::page_index.asc())
        .then_order_by(image::page_index.asc())
        .select((work::all_columns, image::all_columns))
        .load::<(model::Work, model::Image)>(conn)?;

    // 2. Get the user IDs of works from Twitter and Pixiv
    let twitter_post_ids = records
        .iter()
        .filter(|(work, _)| work.source.as_deref() == Some("twitter"))
        .filter_map(|(work, _)| work.post_id_int)
        .unique()
        .collect::<Vec<_>>();
    let twitter_post_user_map = tweet::table
        .filter(tweet::id.eq_any(twitter_post_ids))
        .select((tweet::id, tweet::user_id))
        .load::<(i64, i64)>(conn)?
        .into_iter()
        .collect::<HashMap<_, _>>();
    let pixiv_post_ids = records
        .iter()
        .filter(|(work, _)| work.source.as_deref() == Some("pixiv"))
        .filter_map(|(work, _)| work.post_id_int)
        .unique()
        .collect::<Vec<_>>();
    let pixiv_post_user_map = pixiv_illust::table
        .filter(pixiv_illust::id.eq_any(pixiv_post_ids))
        .select((pixiv_illust::id, pixiv_illust::user_id))
        .load::<(i64, i64)>(conn)?
        .into_iter()
        .collect::<HashMap<_, _>>();

    // 3. Prepare the download jobs
    let mut jobs = Vec::new();
    for (work, image) in records {
        // Final path is like: `community/user_id/filename` or `community/filename`
        let community = work.source.expect("Download job must have a community");
        let mut subdir = PathBuf::from(&community);
        let post_id = work.post_id_int.expect("Download job must have a post ID");
        let user_id = match community.as_str() {
            "twitter" => twitter_post_user_map.get(&post_id),
            "pixiv" => pixiv_post_user_map.get(&post_id),
            _ => None,
        };
        if let Some(user_id) = user_id {
            subdir.push(user_id.to_string());
        }

        jobs.push(DownloadTask {
            url: image.remote_url.expect("Download job must have a remote URL"),
            filename: PathBuf::from(image.filename),
            root_dir: root_dir.as_ref().to_path_buf(),
            subdir,
            image_id: image.id,
        });
    }

    Ok(jobs)
}

/// Update the downloaded image in the database, and return the updated image.
pub fn update_from_local_image(conn: Database, image_id: i32, local_image: &LocalImage) -> Result<model::Image> {
    use bottle_core::schema::image::dsl::*;
    let new_image = diesel::update(image.filter(id.eq(image_id)))
        .set(model::ImageUpdate::from(local_image))
        .returning(model::Image::as_returning())
        .get_result(conn)?;
    tracing::info!("Updated image {} from local image {}", image_id, local_image.relpath);
    Ok(new_image)
}

/// Update the downloaded work thumbnail paths in the database.
pub fn update_work_from_local_image(conn: Database, work_id: i32, local_image: &LocalImage) -> Result<()> {
    use bottle_core::schema::work;

    conn.transaction(|conn| -> Result<()> {
        if let Some(thumbnail_relpath) = &local_image.thumbnail_relpath {
            diesel::update(work::table.filter(work::id.eq(work_id)))
                .set(work::thumbnail_path.eq(thumbnail_relpath))
                .execute(conn)?;
            tracing::info!("Updated work {} thumbnail path to {}", work_id, thumbnail_relpath);
        }
        if let Some(small_thumbnail_relpath) = &local_image.small_thumbnail_relpath {
            diesel::update(work::table.filter(work::id.eq(work_id)))
                .set(work::small_thumbnail_path.eq(small_thumbnail_relpath))
                .execute(conn)?;
            tracing::info!(
                "Updated work {} small thumbnail path to {}",
                work_id,
                small_thumbnail_relpath
            );
        }
        Ok(())
    })?;

    Ok(())
}
