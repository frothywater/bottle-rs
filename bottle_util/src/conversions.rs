/// Helper utilities for common data conversions between layers.
/// These reduce repetitive conversion code across community implementations.

use bottle_core::feed::{MediaView, PostView, UserView};
use chrono::{DateTime, NaiveDateTime, Utc};

/// Helper to construct a PostView with common fields.
pub struct PostViewBuilder {
    post_id: String,
    community: String,
    user_id: Option<String>,
    text: String,
    media_count: Option<i32>,
    thumbnail_url: Option<String>,
    tags: Option<Vec<String>>,
    created_date: DateTime<Utc>,
    added_date: Option<DateTime<Utc>>,
    extra: Option<serde_json::Value>,
}

impl PostViewBuilder {
    pub fn new(post_id: String, community: impl Into<String>) -> Self {
        Self {
            post_id,
            community: community.into(),
            user_id: None,
            text: String::new(),
            media_count: None,
            thumbnail_url: None,
            tags: None,
            created_date: Utc::now(),
            added_date: None,
            extra: None,
        }
    }

    pub fn user_id(mut self, user_id: impl Into<Option<String>>) -> Self {
        self.user_id = user_id.into();
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn media_count(mut self, count: impl Into<Option<i32>>) -> Self {
        self.media_count = count.into();
        self
    }

    pub fn thumbnail_url(mut self, url: impl Into<Option<String>>) -> Self {
        self.thumbnail_url = url.into();
        self
    }

    pub fn tags(mut self, tags: impl Into<Option<Vec<String>>>) -> Self {
        self.tags = tags.into();
        self
    }

    pub fn created_date(mut self, date: DateTime<Utc>) -> Self {
        self.created_date = date;
        self
    }

    pub fn created_date_naive(mut self, date: NaiveDateTime) -> Self {
        self.created_date = date.and_utc();
        self
    }

    pub fn added_date(mut self, date: impl Into<Option<DateTime<Utc>>>) -> Self {
        self.added_date = date.into();
        self
    }

    pub fn added_date_naive(mut self, date: Option<NaiveDateTime>) -> Self {
        self.added_date = date.map(|d| d.and_utc());
        self
    }

    pub fn extra(mut self, extra: impl Into<Option<serde_json::Value>>) -> Self {
        self.extra = extra.into();
        self
    }

    pub fn build(self) -> PostView {
        PostView {
            post_id: self.post_id,
            community: self.community,
            user_id: self.user_id,
            text: self.text,
            media_count: self.media_count,
            thumbnail_url: self.thumbnail_url,
            tags: self.tags,
            created_date: self.created_date,
            added_date: self.added_date,
            extra: self.extra,
        }
    }
}

/// Helper to construct a MediaView with common fields.
pub struct MediaViewBuilder {
    media_id: String,
    community: String,
    post_id: String,
    page_index: i32,
    url: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    thumbnail_url: Option<String>,
    extra: Option<serde_json::Value>,
}

impl MediaViewBuilder {
    pub fn new(media_id: String, community: impl Into<String>, post_id: String, page_index: i32) -> Self {
        Self {
            media_id,
            community: community.into(),
            post_id,
            page_index,
            url: None,
            width: None,
            height: None,
            thumbnail_url: None,
            extra: None,
        }
    }

    pub fn url(mut self, url: impl Into<Option<String>>) -> Self {
        self.url = url.into();
        self
    }

    pub fn dimensions(mut self, width: Option<i32>, height: Option<i32>) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn thumbnail_url(mut self, url: impl Into<Option<String>>) -> Self {
        self.thumbnail_url = url.into();
        self
    }

    pub fn extra(mut self, extra: impl Into<Option<serde_json::Value>>) -> Self {
        self.extra = extra.into();
        self
    }

    pub fn build(self) -> MediaView {
        MediaView {
            media_id: self.media_id,
            community: self.community,
            post_id: self.post_id,
            page_index: self.page_index,
            url: self.url,
            width: self.width,
            height: self.height,
            thumbnail_url: self.thumbnail_url,
            extra: self.extra,
        }
    }
}

/// Helper to construct a UserView with common fields.
pub struct UserViewBuilder {
    user_id: String,
    community: String,
    name: Option<String>,
    username: Option<String>,
    tag_name: Option<String>,
    avatar_url: Option<String>,
    description: Option<String>,
    url: Option<String>,
    post_count: Option<i64>,
}

impl UserViewBuilder {
    pub fn new(user_id: String, community: impl Into<String>) -> Self {
        Self {
            user_id,
            community: community.into(),
            name: None,
            username: None,
            tag_name: None,
            avatar_url: None,
            description: None,
            url: None,
            post_count: None,
        }
    }

    pub fn name(mut self, name: impl Into<Option<String>>) -> Self {
        self.name = name.into();
        self
    }

    pub fn username(mut self, username: impl Into<Option<String>>) -> Self {
        self.username = username.into();
        self
    }

    pub fn tag_name(mut self, tag_name: impl Into<Option<String>>) -> Self {
        self.tag_name = tag_name.into();
        self
    }

    pub fn avatar_url(mut self, url: impl Into<Option<String>>) -> Self {
        self.avatar_url = url.into();
        self
    }

    pub fn description(mut self, desc: impl Into<Option<String>>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn url(mut self, url: impl Into<Option<String>>) -> Self {
        self.url = url.into();
        self
    }

    pub fn post_count(mut self, count: impl Into<Option<i64>>) -> Self {
        self.post_count = count.into();
        self
    }

    pub fn build(self) -> UserView {
        UserView {
            user_id: self.user_id,
            community: self.community,
            name: self.name,
            username: self.username,
            tag_name: self.tag_name,
            avatar_url: self.avatar_url,
            description: self.description,
            url: self.url,
            post_count: self.post_count,
        }
    }
}
