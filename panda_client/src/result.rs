use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::error::Error;

#[derive(Debug, Clone, Serialize)]
pub struct Gallery {
    pub gid: u64,
    pub token: String,
    pub title: String,
    pub url: String,
    pub thumbnail_url: String,
    pub category: GalleryCategory,
    pub uploader: Option<String>,
    pub rating: f32,
    pub image_count: u32,
    pub tags: Vec<GalleryTag>,
    pub posted_date: DateTime<Utc>,
    pub favorited_date: Option<DateTime<Utc>>,
    pub favorited_category_index: Option<u32>,
    pub favorited_category_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalleryDetail {
    pub english_title: String,
    pub parent: Option<String>,
    pub visible: bool,
    pub language: String,
    pub file_size: u32,
    pub favorited_count: u32,
    pub rating_count: u32,
}

#[derive(Debug, Clone)]
pub struct GalleryListResult {
    pub galleries: Vec<Gallery>,
    pub total_count: Option<u32>,
    pub first_page_offset: Option<GalleryListOffset>,
    pub prev_page_offset: Option<GalleryListOffset>,
    pub next_page_offset: Option<GalleryListOffset>,
    pub last_page_offset: Option<GalleryListOffset>,
    pub favorite_categories: Option<Vec<FavoriteCategory>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalleryPageResult {
    pub gallery: Gallery,
    pub detail: GalleryDetail,
    pub preview_page_count: u32,
    pub previews: Vec<ImagePreview>,
}

#[derive(Debug, Clone)]
pub struct ImageResult {
    pub index: u32,
    pub url: String,
    pub filename: String,
    pub file_size: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct FavoriteCategory {
    pub index: u32,
    pub name: String,
    pub gallery_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImagePreview {
    pub index: u32,
    pub token: String,
    pub url: String,
    pub thumbnail_url: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GalleryTag {
    pub namespace: TagNamespace,
    pub name: String,
}

impl Display for GalleryTag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.namespace, self.name)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum TagNamespace {
    Reclass,
    Language,
    Parody,
    Character,
    Group,
    Artist,
    Male,
    Female,
    Mixed,
    Cosplayer,
    Temp,
    Other,
}

impl FromStr for TagNamespace {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "reclass" => Ok(TagNamespace::Reclass),
            "language" => Ok(TagNamespace::Language),
            "parody" => Ok(TagNamespace::Parody),
            "character" => Ok(TagNamespace::Character),
            "group" => Ok(TagNamespace::Group),
            "artist" => Ok(TagNamespace::Artist),
            "male" => Ok(TagNamespace::Male),
            "female" => Ok(TagNamespace::Female),
            "mixed" => Ok(TagNamespace::Mixed),
            "cosplayer" => Ok(TagNamespace::Cosplayer),
            "temp" => Ok(TagNamespace::Temp),
            _ => Ok(TagNamespace::Other),
        }
    }
}

impl Display for TagNamespace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TagNamespace::Reclass => "reclass",
            TagNamespace::Language => "language",
            TagNamespace::Parody => "parody",
            TagNamespace::Character => "character",
            TagNamespace::Group => "group",
            TagNamespace::Artist => "artist",
            TagNamespace::Male => "male",
            TagNamespace::Female => "female",
            TagNamespace::Mixed => "mixed",
            TagNamespace::Cosplayer => "cosplayer",
            TagNamespace::Temp => "temp",
            TagNamespace::Other => "other",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GalleryCategory {
    Misc = 0,
    Doujinshi = 1,
    Manga = 2,
    ArtistCG = 3,
    GameCG = 4,
    ImageSet = 5,
    Cosplay = 6,
    AsianPorn = 7,
    NonH = 8,
    Western = 9,
}

impl FromStr for GalleryCategory {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "Doujinshi" => Ok(GalleryCategory::Doujinshi),
            "Manga" => Ok(GalleryCategory::Manga),
            "Artist CG" => Ok(GalleryCategory::ArtistCG),
            "Game CG" => Ok(GalleryCategory::GameCG),
            "Western" => Ok(GalleryCategory::Western),
            "Non-H" => Ok(GalleryCategory::NonH),
            "Image Set" => Ok(GalleryCategory::ImageSet),
            "Cosplay" => Ok(GalleryCategory::Cosplay),
            "Asian Porn" => Ok(GalleryCategory::AsianPorn),
            _ => Ok(GalleryCategory::Misc),
        }
    }
}

impl Display for GalleryCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            GalleryCategory::Misc => "Misc",
            GalleryCategory::Doujinshi => "Doujinshi",
            GalleryCategory::Manga => "Manga",
            GalleryCategory::ArtistCG => "Artist CG",
            GalleryCategory::GameCG => "Game CG",
            GalleryCategory::ImageSet => "Image Set",
            GalleryCategory::Cosplay => "Cosplay",
            GalleryCategory::AsianPorn => "Asian Porn",
            GalleryCategory::NonH => "Non-H",
            GalleryCategory::Western => "Western",
        };
        write!(f, "{}", s)
    }
}

impl GalleryCategory {
    pub fn from_i32(i: i32) -> GalleryCategory {
        match i {
            0 => GalleryCategory::Misc,
            1 => GalleryCategory::Doujinshi,
            2 => GalleryCategory::Manga,
            3 => GalleryCategory::ArtistCG,
            4 => GalleryCategory::GameCG,
            5 => GalleryCategory::ImageSet,
            6 => GalleryCategory::Cosplay,
            7 => GalleryCategory::AsianPorn,
            8 => GalleryCategory::NonH,
            9 => GalleryCategory::Western,
            _ => GalleryCategory::Misc,
        }
    }
}

lazy_static! {
    pub static ref ALL_GALLERY_CATEGORIES: Vec<GalleryCategory> = vec![
        GalleryCategory::Misc,
        GalleryCategory::Doujinshi,
        GalleryCategory::Manga,
        GalleryCategory::ArtistCG,
        GalleryCategory::GameCG,
        GalleryCategory::ImageSet,
        GalleryCategory::Cosplay,
        GalleryCategory::AsianPorn,
        GalleryCategory::NonH,
        GalleryCategory::Western,
    ];
}

#[derive(Debug, Clone)]
pub enum GalleryListOffset {
    NewerThan(String),
    OlderThan(String),
    Percentage(u32),
}
