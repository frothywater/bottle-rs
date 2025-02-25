use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat};
use jpeg_encoder::{ColorType, Encoder};

use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub fn open_image_bytes(bytes: &[u8], filename: impl AsRef<Path>, mime_type: Option<&str>) -> Result<DynamicImage> {
    // Determine image format from mime type and then filename
    let format = mime_type
        .and_then(ImageFormat::from_mime_type)
        .or(ImageFormat::from_path(filename).ok());

    // 1. Decode image based on the determined format
    if let Some(format) = format {
        if let Ok(img) = ImageReader::with_format(Cursor::new(bytes), format).decode() {
            return Ok(img);
        }
    }
    // 2. If failed, decode image directly
    let img = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?.decode()?;
    Ok(img)
}

pub fn create_thumbnail(img: &DynamicImage, max_width: u32, max_height: u32) -> DynamicImage {
    img.thumbnail(max_width, max_height)
}

pub fn save_image(img: &DynamicImage, path: impl AsRef<Path>) -> Result<()> {
    // Make sure the directory exists
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let rgb = img.to_rgb8();
    let encoder = Encoder::new_file(path, 90)?;
    encoder.encode(rgb.as_raw(), img.width() as u16, img.height() as u16, ColorType::Rgb)?;
    Ok(())
}

/// Get the default thumbnail relpath for a given image path
/// For example, if the image relpath is `path/to/image.jpg`, the thumbnail relpath at 1200x1200 will be `thumb/path/to/image.1200.jpg`
pub fn get_default_thumbnail_relpath(
    subdir: impl AsRef<Path>,
    filename: impl AsRef<Path>,
    size: u32,
) -> Result<PathBuf> {
    let filename = filename.as_ref();
    let basename = filename
        .file_stem()
        .ok_or(Error::InvalidUrl(filename.to_string_lossy().to_string()))?;
    let new_basename = format!("{}.{}.jpg", basename.to_string_lossy(), size);

    let thumbnail_path = PathBuf::from("thumb").join(subdir).join(new_basename);
    Ok(thumbnail_path)
}
