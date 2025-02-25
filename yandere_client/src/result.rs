use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

use chrono::{serde::ts_seconds, DateTime, Utc};
use serde::{Deserialize, Serialize};

use bottle_util::iso8601;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct APIResult {
    pub posts: Vec<PostResult>,
    pub tags: HashMap<String, TagType>,
    pub pools: Vec<PoolResult>,
    pub pool_posts: Vec<PoolPostResult>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PostResult {
    pub id: u64,
    pub tags: String,
    #[serde(with = "ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub creator_id: Option<u64>,
    pub author: String,
    pub source: String,
    pub score: i32,
    pub md5: String,
    pub file_size: u64,
    pub file_ext: String,
    pub file_url: String,
    pub preview_url: String,
    pub preview_width: u32,
    pub preview_height: u32,
    pub actual_preview_width: u32,
    pub actual_preview_height: u32,
    pub sample_url: String,
    pub sample_width: u32,
    pub sample_height: u32,
    pub sample_file_size: u64,
    pub jpeg_url: String,
    pub jpeg_width: u32,
    pub jpeg_height: u32,
    pub jpeg_file_size: u64,
    pub rating: String,
    pub has_children: bool,
    pub parent_id: Option<u64>,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PoolResult {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub user_id: u64,
    pub post_count: u32,
    // pub posts: Vec<PostResult>,
    #[serde(with = "iso8601")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "iso8601")]
    pub updated_at: DateTime<Utc>,
    pub is_public: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PoolPostResult {
    pub id: u64,
    pub pool_id: u64,
    pub post_id: u64,
    pub sequence: String,
    pub prev_post_id: Option<u64>,
    pub next_post_id: Option<u64>,
    pub active: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TagType {
    General,
    Artist,
    Copyright,
    Character,
    #[serde(other)]
    Other,
}

impl Display for TagType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TagType::General => "general",
            TagType::Artist => "artist",
            TagType::Character => "character",
            TagType::Copyright => "copyright",
            TagType::Other => "other",
        };
        write!(f, "{}", s)
    }
}
