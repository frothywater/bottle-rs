use std::path::Path;

use crate::error::{Error, Result};
use crate::thumb::{create_thumbnail, get_default_thumbnail_relpath, open_image_bytes, save_image};
use crate::{DownloadTask, LocalImage};

const VIDEO_EXTENSIONS: [&str; 8] = ["mp4", "webm", "mkv", "avi", "flv", "mov", "wmv", "m4v"];
const THUMBNAIL_SIZE: u32 = 1200;
const SMALL_THUMBNAIL_SIZE: u32 = 300;

fn get_extension(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg")
        .to_lowercase()
}

async fn fetch(url: &str) -> Result<reqwest::Response> {
    let response = if url.contains("pximg.net") {
        // Workaround for Pixiv
        let client = reqwest::Client::new();
        client
            .get(url)
            .header("Referer", "https://www.pixiv.net/")
            .send()
            .await?
    } else {
        reqwest::get(url).await?
    };
    Ok(response)
}

/// Download an image, return the local image.
pub async fn download_image(task: &DownloadTask, overwrite: bool) -> Result<LocalImage> {
    // 1. If not overwrite, and the file exists, directly return the local image info
    let dest_path = task.root_dir.join(&task.subdir).join(&task.filename);
    if !overwrite && tokio::fs::try_exists(&dest_path).await? {
        return get_local_image_info(task).await;
    }

    // 2. Send request to the URL
    let mut response = fetch(&task.url).await?;
    let content_length = response.content_length();
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string());

    // 3. Download the image, write to a buffer
    let mut buffer = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        buffer.extend_from_slice(&chunk);
    }

    // 4. Check if the file is complete
    let size = buffer.len() as u64;
    if let Some(content_length) = content_length {
        if size != content_length {
            return Err(Error::IncompleteDownload(task.url.clone()));
        }
    }
    if size == 0 {
        return Err(Error::IncompleteDownload(task.url.clone()));
    }

    // 5. Save the temp file to the destination
    let dir = task.root_dir.join(&task.subdir);
    if !tokio::fs::try_exists(&dir).await? {
        tokio::fs::create_dir_all(&dir).await?;
    }
    tokio::fs::write(&dest_path, &buffer).await?;

    let extension = get_extension(&task.filename);
    let (mut width, mut height) = (None, None);
    let mut thumbnail_relpath = None;
    let mut small_thumbnail_relpath = None;

    // If the file is not a video, get the dimension of the image and generate thumbnails
    if !VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        // 7. Get the dimension of the image
        let img = open_image_bytes(&buffer, &task.filename, mime_type.as_deref())?;
        width = Some(img.width());
        height = Some(img.height());

        // 8. Generate thumbnails
        // 8.1. Large thumbnail
        let thumbnail = create_thumbnail(&img, THUMBNAIL_SIZE, THUMBNAIL_SIZE);
        let thumb_relpath = get_default_thumbnail_relpath(&task.subdir, &task.filename, THUMBNAIL_SIZE)?;
        save_image(&thumbnail, task.root_dir.join(&thumb_relpath))?;
        thumbnail_relpath = Some(thumb_relpath.to_string_lossy().to_string());

        // 8.2. Small thumbnail
        let thumbnail = create_thumbnail(&img, SMALL_THUMBNAIL_SIZE, SMALL_THUMBNAIL_SIZE);
        let thumb_relpath = get_default_thumbnail_relpath(&task.subdir, &task.filename, SMALL_THUMBNAIL_SIZE)?;
        save_image(&thumbnail, task.root_dir.join(&thumb_relpath))?;
        small_thumbnail_relpath = Some(thumb_relpath.to_string_lossy().to_string());
    }

    // NOTE: All paths are relative to the root directory
    Ok(LocalImage {
        relpath: task.subdir.join(&task.filename).to_string_lossy().to_string(),
        filename: task.filename.to_string_lossy().to_string(),
        thumbnail_relpath,
        small_thumbnail_relpath,
        width,
        height,
        size,
    })
}

pub async fn get_local_image_info(task: &DownloadTask) -> Result<LocalImage> {
    let image_path = task.root_dir.join(&task.subdir).join(&task.filename);
    let extension = get_extension(&task.filename);

    let (mut width, mut height) = (None, None);
    if !VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        let (w, h) = image::image_dimensions(&image_path)?;
        width = Some(w);
        height = Some(h);
    }
    let size = tokio::fs::metadata(&image_path).await?.len();

    // Find thumbnails at inferred paths
    let thumbnail_relpath = get_default_thumbnail_relpath(&task.subdir, &task.filename, THUMBNAIL_SIZE)?;
    let thumbnail_relpath = tokio::fs::try_exists(task.root_dir.join(&thumbnail_relpath))
        .await?
        .then_some(thumbnail_relpath.to_string_lossy().to_string());
    let small_thumbnail_relpath = get_default_thumbnail_relpath(&task.subdir, &task.filename, SMALL_THUMBNAIL_SIZE)?;
    let small_thumbnail_relpath = tokio::fs::try_exists(task.root_dir.join(&small_thumbnail_relpath))
        .await?
        .then_some(small_thumbnail_relpath.to_string_lossy().to_string());

    // NOTE: All paths are relative to the root directory
    Ok(LocalImage {
        relpath: task.subdir.join(&task.filename).to_string_lossy().to_string(),
        filename: task.filename.to_string_lossy().to_string(),
        thumbnail_relpath,
        small_thumbnail_relpath,
        width,
        height,
        size,
    })
}
