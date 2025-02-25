use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, VecSkipError};

use crate::util::twitter_date_format;

// Media

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Media {
    pub media_key: String,
    pub media_url_https: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub sizes: MediaSizeMap,
    pub original_info: MediaOriginalInfo,
    pub video_info: Option<VideoInfo>,
    #[serde(flatten)]
    pub url: UrlEntity,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MediaSize {
    #[serde(rename = "w")]
    pub width: u32,
    #[serde(rename = "h")]
    pub height: u32,
    pub resize: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MediaSizeMap {
    pub large: MediaSize,
    pub medium: MediaSize,
    pub small: MediaSize,
    pub thumb: MediaSize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MediaOriginalInfo {
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VideoInfo {
    pub aspect_ratio: (u32, u32),
    pub duration_millis: Option<u32>,
    pub variants: Vec<VideoVariant>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VideoVariant {
    pub bitrate: Option<u32>,
    pub content_type: String,
    pub url: String,
}

// Other entities

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UrlEntity {
    pub url: String,
    pub display_url: String,
    pub expanded_url: String,
    pub indices: (u32, u32),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct HashtagEntity {
    pub text: String,
    pub indices: (u32, u32),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Entities {
    pub urls: Vec<UrlEntity>,
    pub hashtags: Vec<HashtagEntity>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExtendedEntities {
    pub media: Vec<Media>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserUrlEntities {
    pub urls: Vec<UrlEntity>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserEntities {
    pub description: UserUrlEntities,
    pub url: Option<UserUrlEntities>,
}

// User

#[derive(Deserialize, Serialize, Debug)]
pub struct LegacyUser {
    #[serde(with = "twitter_date_format")]
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub screen_name: String,
    pub description: String,
    pub url: Option<String>,
    pub location: String,
    pub entities: UserEntities,
    pub following: Option<bool>,
    pub followers_count: u32,
    pub friends_count: u32,
    pub listed_count: u32,
    #[serde(rename = "favourites_count")]
    pub favorite_count: u32,
    pub statuses_count: u32,
    pub media_count: u32,
    pub profile_banner_url: Option<String>,
    pub profile_image_url_https: Option<String>,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct User {
    #[serde(rename = "rest_id")]
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    pub legacy: LegacyUser,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UserResult {
    pub result: User,
}

// Tweet

#[derive(Deserialize, Serialize, Debug)]
pub struct TweetUser {
    #[serde(rename = "user_results")]
    pub user: UserResult,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LegacyTweet {
    #[serde(with = "twitter_date_format")]
    pub created_at: DateTime<Utc>,
    pub full_text: String,
    pub entities: Entities,
    pub extended_entities: Option<ExtendedEntities>,
    pub favorited: bool,
    pub retweeted: bool,
    pub conversation_id_str: String,
    pub favorite_count: u32,
    pub retweet_count: u32,
    pub reply_count: u32,
    pub quote_count: u32,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct Tweet {
    #[serde(rename = "rest_id")]
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    pub legacy: LegacyTweet,
    pub core: TweetUser,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "__typename")]
pub enum TweetVariation {
    Tweet(Tweet),
    TweetWithVisibilityResults { tweet: Tweet },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TweetResult {
    pub result: TweetVariation,
}

// Timeline

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "itemType")]
pub enum TimelineItemContent {
    #[serde(rename = "TimelineTweet")]
    Tweet { tweet_results: TweetResult },
    #[serde(rename = "TimelineUser")]
    User { user_results: UserResult },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TimelineItemEntryContent {
    #[serde(rename = "itemContent")]
    pub item_content: TimelineItemContent,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "cursorType")]
pub enum TimelineCursor {
    Top {
        value: String,
    },
    Bottom {
        value: String,
        #[serde(rename = "stopOnEmptyResponse", default)]
        stop_on_empty_response: bool,
    },
}

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "entryType")]
pub enum TimelineEntryContent {
    #[serde(rename = "TimelineTimelineItem")]
    Item(TimelineItemEntryContent),
    #[serde(rename = "TimelineTimelineCursor")]
    Cursor(TimelineCursor),
    #[serde(other)]
    Other,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
pub struct TimelineEntry {
    #[serde(rename = "entryId")]
    pub entry_id: String,
    #[serde(rename = "sortIndex")]
    #[serde_as(as = "DisplayFromStr")]
    pub sort_index: u64,
    #[serde(rename = "content")]
    pub content: TimelineEntryContent,
}

#[allow(clippy::large_enum_variant)]
#[serde_as]
#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum TimelineInstruction {
    TimelineAddEntries {
        #[serde_as(as = "VecSkipError<_>")]
        entries: Vec<TimelineEntry>,
    },
    TimelineReplaceEntry {
        entry: TimelineEntry,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TimelineL5 {
    pub instructions: Vec<TimelineInstruction>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TimelineL4 {
    pub timeline: TimelineL5,
}

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum UserResponse {
    User(User),
    Timeline {
        #[serde(alias = "timeline_v2")]
        timeline: TimelineL4,
    },
}

#[derive(Deserialize, Serialize, Debug)]
pub enum Data {
    #[serde(rename = "user")]
    User { result: UserResponse },
    #[serde(rename = "users")]
    Users(Vec<UserResult>),
    #[serde(rename = "tweetResult")]
    Tweet(TweetResult),
    #[serde(rename = "search_by_raw_query")]
    Search { search_timeline: TimelineL4 },
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GraphqlResponse {
    pub data: Data,
}

// Other

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Account {
    #[serde(rename = "user_id")]
    #[serde_as(as = "DisplayFromStr")]
    pub id: u64,
    pub name: String,
    #[serde(rename = "screen_name")]
    pub username: String,
    #[serde(rename = "avatar_image_url")]
    pub profile_image_url_https: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AccountResponse {
    pub users: Vec<Account>,
}
