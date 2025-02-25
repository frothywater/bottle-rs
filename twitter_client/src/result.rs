use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::util::twitter_date_format;

use crate::response::{
    self, Data, GraphqlResponse, TimelineCursor, TimelineEntryContent, TimelineInstruction, TimelineItemContent,
    TweetVariation, UserResponse,
};
pub use crate::response::{
    Account, HashtagEntity, Media, MediaOriginalInfo, MediaSizeMap, UrlEntity, VideoInfo, VideoVariant,
};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    Top {
        value: String,
        sort_index: u64,
    },
    Bottom {
        value: String,
        sort_index: u64,
        stop_on_empty_response: bool,
    },
}

impl Cursor {
    pub fn value(&self) -> &str {
        match self {
            Cursor::Top { value, .. } => value,
            Cursor::Bottom { value, .. } => value,
        }
    }

    pub fn sort_index(&self) -> u64 {
        match self {
            Cursor::Top { sort_index, .. } => *sort_index,
            Cursor::Bottom { sort_index, .. } => *sort_index,
        }
    }
}

impl TimelineCursor {
    fn into_cursor(self: TimelineCursor, sort_index: u64) -> Cursor {
        match self {
            TimelineCursor::Top { value } => Cursor::Top { value, sort_index },
            TimelineCursor::Bottom {
                value,
                stop_on_empty_response,
            } => Cursor::Bottom {
                value,
                sort_index,
                stop_on_empty_response,
            },
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum UserUrl {
    Url(String),
    Urls { urls: Vec<UrlEntity> },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub id: u64,
    #[serde(with = "twitter_date_format")]
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub screen_name: String,
    pub description: String,
    pub description_url_entities: Vec<UrlEntity>,
    pub url: Option<String>,
    pub url_entities: Vec<UrlEntity>,
    pub location: String,
    pub following: Option<bool>,
    pub followers_count: u32,
    pub friends_count: u32,
    pub listed_count: u32,
    pub favorite_count: u32,
    pub statuses_count: u32,
    pub media_count: u32,
    pub profile_banner_url: Option<String>,
    pub profile_image_url_https: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Tweet {
    pub id: u64,
    #[serde(with = "twitter_date_format")]
    pub created_at: DateTime<Utc>,
    pub full_text: String,
    pub media: Vec<Media>,
    pub urls: Vec<UrlEntity>,
    pub hashtags: Vec<HashtagEntity>,
    pub favorited: bool,
    pub retweeted: bool,
    pub conversation_id_str: String,
    pub favorite_count: u32,
    pub retweet_count: u32,
    pub reply_count: u32,
    pub quote_count: u32,
    pub user: User,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TimelineResult {
    pub tweets: Vec<Tweet>,
    pub users: Vec<User>,
    pub cursors: Vec<Cursor>,
    pub sort_indices: Vec<u64>,
}

// MARK: Helpers

impl TimelineResult {
    pub fn top_cursor(&self) -> Option<&Cursor> {
        self.cursors.iter().find(|c| matches!(c, Cursor::Top { .. }))
    }

    pub fn bottom_cursor(&self) -> Option<&Cursor> {
        self.cursors.iter().find(|c| matches!(c, Cursor::Bottom { .. }))
    }
}

impl Media {
    pub fn url(&self) -> &str {
        match self.type_.as_str() {
            "animated_gif" | "video" => self
                .video_info
                .as_ref()
                .and_then(|v| {
                    v.variants
                        .iter()
                        .filter(|v| v.content_type == "video/mp4")
                        .max_by_key(|v| v.bitrate)
                        .map(|v| v.url.as_str())
                })
                .unwrap_or(self.media_url_https.as_str()),
            _ => self.media_url_https.as_str(),
        }
    }

    pub fn preview_image_url(&self) -> Option<&str> {
        match self.type_.as_str() {
            "animated_gif" | "video" => Some(self.media_url_https.as_str()),
            _ => None,
        }
    }

    pub fn duration(&self) -> Option<u32> {
        match self.type_.as_str() {
            "video" => self.video_info.as_ref().and_then(|v| v.duration_millis),
            _ => None,
        }
    }
}

// MARK: Conversions

impl From<response::User> for User {
    fn from(user: response::User) -> Self {
        User {
            id: user.id,
            created_at: user.legacy.created_at,
            name: user.legacy.name,
            screen_name: user.legacy.screen_name,
            description: user.legacy.description,
            description_url_entities: user.legacy.entities.description.urls,
            url: user.legacy.url,
            url_entities: user.legacy.entities.url.map(|u| u.urls).unwrap_or_default(),
            location: user.legacy.location,
            following: user.legacy.following,
            followers_count: user.legacy.followers_count,
            friends_count: user.legacy.friends_count,
            listed_count: user.legacy.listed_count,
            favorite_count: user.legacy.favorite_count,
            statuses_count: user.legacy.statuses_count,
            media_count: user.legacy.media_count,
            profile_banner_url: user.legacy.profile_banner_url,
            profile_image_url_https: user.legacy.profile_image_url_https,
        }
    }
}

impl From<response::Tweet> for Tweet {
    fn from(tweet: response::Tweet) -> Self {
        Tweet {
            id: tweet.id,
            created_at: tweet.legacy.created_at,
            full_text: tweet.legacy.full_text,
            media: tweet.legacy.extended_entities.map(|e| e.media).unwrap_or_default(),
            urls: tweet.legacy.entities.urls,
            hashtags: tweet.legacy.entities.hashtags,
            favorited: tweet.legacy.favorited,
            retweeted: tweet.legacy.retweeted,
            conversation_id_str: tweet.legacy.conversation_id_str,
            favorite_count: tweet.legacy.favorite_count,
            retweet_count: tweet.legacy.retweet_count,
            reply_count: tweet.legacy.reply_count,
            quote_count: tweet.legacy.quote_count,
            user: tweet.core.user.result.into(),
        }
    }
}

impl From<TweetVariation> for Tweet {
    fn from(tweet: TweetVariation) -> Self {
        match tweet {
            TweetVariation::Tweet(tweet) => tweet.into(),
            TweetVariation::TweetWithVisibilityResults { tweet } => tweet.into(),
        }
    }
}

impl TryFrom<GraphqlResponse> for User {
    type Error = Error;

    fn try_from(value: GraphqlResponse) -> Result<Self, Self::Error> {
        match value.data {
            Data::User {
                result: UserResponse::User(user),
            } => Ok(user.into()),
            _ => Err(Error::InvalidGraphqlResponse),
        }
    }
}

impl TryFrom<GraphqlResponse> for Vec<User> {
    type Error = Error;

    fn try_from(value: GraphqlResponse) -> Result<Self, Self::Error> {
        match value.data {
            Data::Users(users) => Ok(users.into_iter().map(|u| u.result.into()).collect()),
            _ => Err(Error::InvalidGraphqlResponse),
        }
    }
}

impl TryFrom<GraphqlResponse> for Tweet {
    type Error = Error;

    fn try_from(value: GraphqlResponse) -> Result<Self, Self::Error> {
        match value.data {
            Data::Tweet(tweet) => Ok(tweet.result.into()),
            _ => Err(Error::InvalidGraphqlResponse),
        }
    }
}

impl TryFrom<GraphqlResponse> for TimelineResult {
    type Error = Error;

    fn try_from(value: GraphqlResponse) -> Result<Self, Self::Error> {
        let timeline = match value.data {
            Data::User {
                result: UserResponse::Timeline { timeline },
            } => Ok(timeline.timeline),
            Data::Search { search_timeline } => Ok(search_timeline.timeline),
            _ => Err(Error::InvalidGraphqlResponse),
        }?;

        let mut tweets: Vec<Tweet> = Vec::new();
        let mut users: Vec<User> = Vec::new();
        let mut cursors = Vec::new();
        let mut sort_indices = Vec::new();
        for instruction in timeline.instructions {
            match instruction {
                TimelineInstruction::TimelineAddEntries { entries } => {
                    for entry in entries {
                        if entry.entry_id.starts_with("promote") {
                            // Skip promoted tweets
                            continue;
                        }
                        let sort_index = entry.sort_index;
                        match entry.content {
                            TimelineEntryContent::Item(item) => {
                                sort_indices.push(sort_index);
                                match item.item_content {
                                    TimelineItemContent::Tweet { tweet_results } => {
                                        tweets.push(tweet_results.result.into())
                                    }
                                    TimelineItemContent::User { user_results } => {
                                        users.push(user_results.result.into())
                                    }
                                }
                            }
                            TimelineEntryContent::Cursor(cursor) => cursors.push(cursor.into_cursor(sort_index)),
                            _ => {}
                        }
                    }
                }
                TimelineInstruction::TimelineReplaceEntry { entry } => {
                    if let TimelineEntryContent::Cursor(cursor) = entry.content {
                        cursors.push(cursor.into_cursor(entry.sort_index))
                    }
                }
                _ => {}
            }
        }

        Ok(TimelineResult {
            tweets,
            users,
            cursors,
            sort_indices,
        })
    }
}
