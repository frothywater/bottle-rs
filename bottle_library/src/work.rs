use diesel::prelude::*;

use bottle_core::{
    feed::GeneralResponse,
    library::{ImageView, RemoteImage, RemoteWork, WorkView},
    Database, Error, Result,
};

use crate::model;
use crate::util::new_images;

// MARK: Work

/// Add a remote work to the database, and return a work view for the client.
pub fn add_remote_work(conn: Database, remote_work: &RemoteWork) -> Result<GeneralResponse> {
    use bottle_core::schema::{image, work};
    use itertools::Itertools;

    // Check if the work already exists
    if let Some(source) = &remote_work.source {
        let work = work::table
            .filter(work::source.eq(source))
            .filter(work::post_id.eq(&remote_work.post_id))
            .filter(work::page_index.eq(remote_work.page_index))
            .first::<model::Work>(conn)
            .optional()?;
        if work.is_some() {
            return Err(Error::ObjectAlreadyExists(remote_work.to_string()));
        }
    }

    let result = conn.transaction(|conn| -> Result<GeneralResponse> {
        // Insert the work
        let work: model::Work = diesel::insert_into(work::table)
            .values(model::NewWork::from(remote_work))
            .returning(model::Work::as_returning())
            .get_result(conn)?;

        // Insert the images
        let new_images = new_images(remote_work, work.id);
        diesel::insert_into(image::table).values(new_images).execute(conn)?;

        // Get the inserted images
        let images = image::table
            .filter(image::work_id.eq(work.id))
            .order_by(image::page_index.asc())
            .load::<model::Image>(conn)?;

        tracing::info!(
            "Added {} to the library. Work {}: Images: {}",
            remote_work,
            work.id,
            images.iter().map(|i| i.id).join(", ")
        );

        Ok(GeneralResponse {
            works: Some(vec![WorkView::from(work)]),
            images: Some(images.into_iter().map(ImageView::from).collect()),
            ..Default::default()
        })
    })?;

    Ok(result)
}

/// Delete a remote work from the database.
pub fn delete_work(conn: Database, work_id: i32) -> Result<()> {
    use bottle_core::schema::work;
    diesel::delete(work::table.find(work_id)).execute(conn)?;
    tracing::info!("Deleted work {}", work_id);
    Ok(())
}

/// Find the works and images in the database by the community name and post IDs.
pub fn get_works_by_post_ids(
    conn: Database,
    source: &str,
    post_ids: impl IntoIterator<Item = String> + Clone,
    first_image_only: bool,
) -> Result<(Vec<WorkView>, Vec<ImageView>)> {
    use bottle_core::schema::{image, work};
    use itertools::Itertools;

    // Get the works
    let works = work::table
        .filter(work::source.eq(source))
        .filter(work::post_id.eq_any(post_ids.clone()))
        .order_by(work::page_index.asc())
        .load::<model::Work>(conn)?;
    // Reorder works by the order of post_ids
    let works_grouped_by_post = works.into_iter().into_group_map_by(|work| work.post_id.clone());
    let mut works = Vec::new();
    for post_id in post_ids {
        let post_id = Some(post_id);
        if let Some(work_group) = works_grouped_by_post.get(&post_id) {
            works.extend(work_group.iter().cloned());
        }
    }

    // Get the images
    let work_ids = works.iter().map(|work| work.id);
    let images = if first_image_only {
        image::table
            .filter(image::work_id.eq_any(work_ids))
            .filter(image::page_index.eq(0))
            .load::<model::Image>(conn)?
    } else {
        image::table
            .filter(image::work_id.eq_any(work_ids))
            .order_by(image::page_index.asc())
            .load::<model::Image>(conn)?
    };

    let works = works.into_iter().map(WorkView::from).collect();
    let images = images.into_iter().map(ImageView::from).collect();
    Ok((works, images))
}

// MARK: Image

/// Add a remote image to the database, and return an image view for the client.
pub fn add_remote_image(conn: Database, remote_image: &RemoteImage, work_id: i32) -> Result<ImageView> {
    use bottle_core::schema::image;

    // Check if the image already exists
    let image = image::table
        .filter(image::work_id.eq(work_id))
        .filter(image::page_index.eq(remote_image.page_index))
        .first::<model::Image>(conn)
        .optional()?;
    if image.is_some() {
        return Err(Error::ObjectAlreadyExists(format!(
            "{}{}",
            work_id,
            remote_image.page_index.map(|i| format!(":{}", i)).unwrap_or_default()
        )));
    }

    let result = conn.transaction(|conn| -> Result<ImageView> {
        let new_image = model::NewImage {
            work_id,
            filename: remote_image.filename.clone(),
            remote_url: Some(remote_image.url.clone()),
            page_index: remote_image.page_index,
            ..Default::default()
        };
        let image = diesel::insert_into(image::table)
            .values(new_image)
            .returning(model::Image::as_returning())
            .get_result(conn)?;

        tracing::info!(
            "Added image {} to the work {}{}",
            image.id,
            work_id,
            remote_image.page_index.map(|i| format!(":{}", i)).unwrap_or_default()
        );

        Ok(image.into())
    })?;

    Ok(result)
}

/// Get the image in the database by the image ID.
pub fn get_image(conn: Database, image_id: i32) -> Result<model::Image> {
    use bottle_core::schema::image;
    let result = image::table.find(image_id).first::<model::Image>(conn)?;
    Ok(result)
}

/// Get all images.
pub fn get_images(conn: Database) -> Result<Vec<model::Image>> {
    use bottle_core::schema::image;
    let result = image::table.load::<model::Image>(conn)?;
    Ok(result)
}
