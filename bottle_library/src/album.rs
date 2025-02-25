use diesel::prelude::*;

use bottle_core::{feed::GeneralResponse, library::*, Database, Error, Result};

use crate::model;

// MARK: Album & Folder

const POSITION_GAP: i32 = 100;

#[derive(Debug)]
pub struct Album;

impl Album {
    pub fn add(conn: Database, name: &str, folder_id: Option<i32>) -> Result<AlbumView> {
        use bottle_core::schema::{album, folder};

        // Check the requested folder
        if let Some(folder_id) = folder_id {
            let folder = folder::table.find(folder_id).first::<model::Folder>(conn).optional()?;
            if folder.is_none() {
                return Err(Error::ObjectNotFound(format!("Folder {}", folder_id)));
            }
        }

        // Get max position of existing album in the folder
        let max_position = album::table
            .filter(album::folder_id.is(folder_id))
            .select(diesel::dsl::max(album::position))
            .first::<Option<i32>>(conn)?
            .unwrap_or_default();

        let new_album = model::NewAlbum {
            name: name.to_string(),
            folder_id,
            position: max_position + POSITION_GAP,
        };
        let album = diesel::insert_into(album::table)
            .values(new_album)
            .returning(model::Album::as_returning())
            .get_result(conn)?;

        tracing::info!(
            "Added album {} \"{}\"{}, position {}",
            album.id,
            name,
            folder_id.map(|id| format!(" in folder {}", id)).unwrap_or_default(),
            album.position
        );

        Ok(album.into())
    }

    pub fn delete(conn: Database, album_id: i32) -> Result<()> {
        use bottle_core::schema::album;
        diesel::delete(album::table.find(album_id)).execute(conn)?;
        tracing::info!("Deleted album {}", album_id);
        Ok(())
    }

    pub fn all(conn: Database) -> Result<Vec<AlbumView>> {
        use bottle_core::schema::album;
        let albums = album::table
            .order_by(album::folder_id.asc())
            .order_by(album::position.asc())
            .load::<model::Album>(conn)?;
        Ok(albums.into_iter().map(AlbumView::from).collect())
    }

    pub fn rename(conn: Database, album_id: i32, name: &str) -> Result<AlbumView> {
        use bottle_core::schema::album;
        diesel::update(album::table.find(album_id))
            .set(album::name.eq(name))
            .execute(conn)?;
        let album = album::table.find(album_id).first::<model::Album>(conn)?;
        tracing::info!("Renamed album {} to \"{}\"", album_id, name);
        Ok(album.into())
    }

    pub fn reorder(conn: Database, album_id: i32, folder_id: Option<i32>, position: Option<i32>) -> Result<AlbumView> {
        use bottle_core::schema::album;

        // If position is not provided, use the next position in the folder
        let position = if let Some(position) = position {
            position
        } else {
            let max_position = album::table
                .filter(album::folder_id.is(folder_id))
                .select(diesel::dsl::max(album::position))
                .first::<Option<i32>>(conn)?
                .unwrap_or_default();
            max_position + POSITION_GAP
        };

        // TODO: Handle position impact on others

        let update = model::AlbumUpdate { folder_id, position };
        diesel::update(album::table.find(album_id)).set(&update).execute(conn)?;
        let album = album::table.find(album_id).first::<model::Album>(conn)?;

        tracing::info!("Reordered album {} to position {}", album_id, position);
        Ok(album.into())
    }

    pub fn add_works(conn: Database, album_id: i32, work_ids: impl IntoIterator<Item = i32>) -> Result<()> {
        use bottle_core::schema::{album, album_work};
        use itertools::Itertools;

        // Check if the album exists
        let album = album::table.find(album_id).first::<model::Album>(conn).optional()?;
        if album.is_none() {
            return Err(Error::ObjectNotFound(format!("Album {}", album_id)));
        }

        // Get max position of existing work in the album
        let max_position = album_work::table
            .filter(album_work::album_id.eq(album_id))
            .select(diesel::dsl::max(album_work::position))
            .first::<Option<i32>>(conn)?
            .unwrap_or_default();

        // Insert the works
        let work_ids = work_ids.into_iter().collect::<Vec<_>>();
        let positions = (1..=work_ids.len() as i32).map(|i| max_position + i * POSITION_GAP);
        let new_album_works = work_ids
            .into_iter()
            .zip(positions)
            .map(|(work_id, position)| model::AlbumWork {
                album_id,
                work_id,
                position,
            })
            .collect::<Vec<_>>();
        diesel::insert_into(album_work::table)
            .values(&new_album_works)
            .execute(conn)?;

        tracing::info!(
            "Added works {} to album {}",
            new_album_works.iter().map(|w| w.work_id).join(", "),
            album_id
        );
        Ok(())
    }

    pub fn works(conn: Database, album_id: i32, page: i64, page_size: i64) -> Result<GeneralResponse> {
        use bottle_core::schema::{album_work, image, work};
        use bottle_util::diesel_ext::Paginate;

        // 1. Fetch works
        let (works, total_items) = album_work::table
            .inner_join(work::table)
            .filter(album_work::album_id.eq(album_id))
            .order_by(album_work::position.asc())
            .select(work::all_columns)
            .paginate(page, page_size)
            .load_and_count::<model::Work>(conn)?;

        // 2. Fetch images
        let work_ids = works.iter().map(|work| work.id);
        let images = image::table
            .filter(image::work_id.eq_any(work_ids))
            .order_by(image::page_index.asc())
            .load::<model::Image>(conn)?;

        Ok(GeneralResponse {
            works: Some(works.into_iter().map(WorkView::from).collect()),
            images: Some(images.into_iter().map(ImageView::from).collect()),
            total_items,
            page,
            page_size,
            ..Default::default()
        })
    }

    pub fn remove_works(conn: Database, album_id: i32, work_ids: impl IntoIterator<Item = i32>) -> Result<()> {
        use bottle_core::schema::album_work;
        use itertools::Itertools;
        let work_ids = work_ids.into_iter().collect::<Vec<_>>();
        diesel::delete(
            album_work::table.filter(
                album_work::album_id
                    .eq(album_id)
                    .and(album_work::work_id.eq_any(&work_ids)),
            ),
        )
        .execute(conn)?;
        tracing::info!("Removed works {} from album {}", work_ids.iter().join(", "), album_id);
        Ok(())
    }

    // TODO: reorder works in album
}

#[derive(Debug)]
pub struct Folder;

impl Folder {
    pub fn add(conn: Database, name: &str, parent_id: Option<i32>) -> Result<FolderView> {
        use bottle_core::schema::folder;

        // Check the requested parent folder
        if let Some(parent_id) = parent_id {
            let folder = folder::table.find(parent_id).first::<model::Folder>(conn).optional()?;
            if folder.is_none() {
                return Err(Error::ObjectNotFound(format!("Folder {}", parent_id)));
            }
        }

        // Get max position of existing folder in the parent folder
        let max_position = folder::table
            .filter(folder::parent_id.is(parent_id))
            .select(diesel::dsl::max(folder::position))
            .first::<Option<i32>>(conn)?
            .unwrap_or_default();

        let new_folder = model::NewFolder {
            name: name.to_string(),
            parent_id,
            position: max_position + POSITION_GAP,
        };
        let folder = diesel::insert_into(folder::table)
            .values(new_folder)
            .returning(model::Folder::as_returning())
            .get_result(conn)?;

        tracing::info!(
            "Added folder {} \"{}\"{}, position {}",
            folder.id,
            name,
            parent_id.map(|id| format!(" in folder {}", id)).unwrap_or_default(),
            folder.position
        );
        Ok(folder.into())
    }

    pub fn delete(conn: Database, folder_id: i32) -> Result<()> {
        use bottle_core::schema::folder;
        diesel::delete(folder::table.find(folder_id)).execute(conn)?;
        tracing::info!("Deleted folder {}", folder_id);
        Ok(())
    }

    pub fn all(conn: Database) -> Result<Vec<FolderView>> {
        use bottle_core::schema::folder;
        let folders = folder::table
            .order_by(folder::parent_id.asc())
            .order_by(folder::position.asc())
            .load::<model::Folder>(conn)?;
        Ok(folders.into_iter().map(FolderView::from).collect())
    }

    pub fn rename(conn: Database, folder_id: i32, name: &str) -> Result<FolderView> {
        use bottle_core::schema::folder;
        diesel::update(folder::table.find(folder_id))
            .set(folder::name.eq(name))
            .execute(conn)?;
        let folder = folder::table.find(folder_id).first::<model::Folder>(conn)?;
        tracing::info!("Renamed folder {} to \"{}\"", folder_id, name);
        Ok(folder.into())
    }

    pub fn reorder(
        conn: Database,
        folder_id: i32,
        parent_id: Option<i32>,
        position: Option<i32>,
    ) -> Result<FolderView> {
        use bottle_core::schema::folder;

        // If position is not provided, use the next position in the parent folder
        let position = if let Some(position) = position {
            position
        } else {
            let max_position = folder::table
                .filter(folder::parent_id.is(parent_id))
                .select(diesel::dsl::max(folder::position))
                .first::<Option<i32>>(conn)?
                .unwrap_or_default();
            max_position + POSITION_GAP
        };

        // TODO: Handle position impact on others

        let update = model::FolderUpdate { parent_id, position };
        diesel::update(folder::table.find(folder_id))
            .set(&update)
            .execute(conn)?;
        let folder = folder::table.find(folder_id).first::<model::Folder>(conn)?;

        tracing::info!("Reordered folder {} to position {}", folder_id, position);
        Ok(folder.into())
    }
}
