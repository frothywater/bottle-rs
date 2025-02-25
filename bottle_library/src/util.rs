use bottle_core::library::*;
use bottle_download::LocalImage;

use crate::model;

/// Prepare new images to insert into the database.
pub fn new_images(remote_work: &RemoteWork, work_id: i32) -> Vec<model::NewImage> {
    remote_work
        .images
        .iter()
        .enumerate()
        .map(|(index, image)| model::NewImage {
            work_id,
            filename: image.filename.clone(),
            remote_url: Some(image.url.clone()),
            page_index: Some(index as i32),
            ..Default::default()
        })
        .collect()
}

/// Prepare a not yet downloaded image to insert into the database.
impl From<&RemoteImage> for model::NewImage {
    fn from(image: &RemoteImage) -> Self {
        model::NewImage {
            filename: image.filename.clone(),
            remote_url: Some(image.url.clone()),
            page_index: image.page_index,
            ..Default::default()
        }
    }
}

/// Update the image if it is already downloaded.
impl From<&LocalImage> for model::ImageUpdate {
    fn from(image: &LocalImage) -> Self {
        model::ImageUpdate {
            path: Some(image.relpath.clone()),
            thumbnail_path: image.thumbnail_relpath.clone(),
            small_thumbnail_path: image.small_thumbnail_relpath.clone(),
            width: image.width.map(|v| v as i32),
            height: image.height.map(|v| v as i32),
            size: Some(image.size as i32),
        }
    }
}

/// Prepare a not yet downloaded work to insert into the database.
impl From<&RemoteWork> for model::NewWork {
    fn from(work: &RemoteWork) -> Self {
        model::NewWork {
            source: work.source.clone(),
            post_id: work.post_id.clone(),
            post_id_int: work.post_id_int,
            page_index: work.page_index,
            as_archive: false,
            image_count: work.media_count,
            name: work.name.clone(),
            caption: work.caption.clone(),
        }
    }
}

/// Prepare a `WorkView` of a work.
impl From<model::Work> for WorkView {
    fn from(work: model::Work) -> Self {
        WorkView {
            id: work.id,
            community: work.source,
            post_id: work.post_id,
            page_index: work.page_index,
            as_archive: work.as_archive,
            name: work.name,
            caption: work.caption,
            favorite: work.favorite,
            rating: work.rating,
            thumbnail_path: work.thumbnail_path,
            small_thumbnail_path: work.small_thumbnail_path,
            added_date: work.added_date.and_utc(),
            modified_date: work.modified_date.and_utc(),
            viewed_date: work.viewed_date.map(|d| d.and_utc()),
        }
    }
}

/// Prepare a `ImageView` of an image.
impl From<model::Image> for ImageView {
    fn from(image: model::Image) -> Self {
        ImageView {
            id: image.id,
            work_id: image.work_id,
            page_index: image.page_index,
            filename: image.filename,
            remote_url: image.remote_url,
            path: image.path,
            thumbnail_path: image.thumbnail_path,
            small_thumbnail_path: image.small_thumbnail_path,
            width: image.width,
            height: image.height,
            size: image.size,
        }
    }
}

/// Prepare a `AlbumView` of an album.
impl From<model::Album> for AlbumView {
    fn from(album: model::Album) -> AlbumView {
        AlbumView {
            id: album.id,
            name: album.name,
            folder_id: album.folder_id,
            position: album.position,
            added_date: album.added_date.and_utc(),
            modified_date: album.modified_date.and_utc(),
        }
    }
}

/// Prepare a `FolderView` of a folder.
impl From<model::Folder> for FolderView {
    fn from(folder: model::Folder) -> FolderView {
        FolderView {
            id: folder.id,
            name: folder.name,
            parent_id: folder.parent_id,
            position: folder.position,
            added_date: folder.added_date.and_utc(),
            modified_date: folder.modified_date.and_utc(),
        }
    }
}
