use bottle_core::{
    feed::{MediaView, PostView, UserView},
    library::{RemoteImage, RemoteWork},
    Error, Result,
};

use twitter_client as client;

use crate::community::TwitterAccount;
use crate::feed::{TwitterFeed, TwitterFeedParams};
use crate::model;

impl From<&client::User> for model::NewTwitterUser {
    fn from(user: &client::User) -> Self {
        model::NewTwitterUser {
            id: user.id as i64,
            name: user.name.clone(),
            username: user.screen_name.clone(),
            profile_image_url: user.profile_image_url_https.clone(),
            description: user.description.clone(),
            location: user.location.clone(),
            url: user.url.clone(),
            created_date: user.created_at.naive_utc(),
        }
    }
}

pub(crate) fn user_view(user: &client::User) -> UserView {
    UserView {
        user_id: user.id.to_string(),
        community: "twitter".to_string(),
        name: Some(user.name.clone()),
        username: Some(user.screen_name.clone()),
        avatar_url: user.profile_image_url_https.clone(),
        description: Some(user.description.clone()),
        url: user.url.clone(),
        ..Default::default()
    }
}

impl From<&client::Tweet> for model::NewTweet {
    fn from(tweet: &client::Tweet) -> Self {
        model::NewTweet {
            id: tweet.id as i64,
            user_id: tweet.user.id as i64,
            caption: tweet.full_text.clone(),
            created_date: tweet.created_at.naive_utc(),
        }
    }
}

pub(crate) fn post_view(tweet: &client::Tweet) -> PostView {
    PostView {
        post_id: tweet.id.to_string(),
        community: "twitter".to_string(),
        user_id: Some(tweet.user.id.to_string()),
        text: tweet.full_text.clone(),
        thumbnail_url: None,
        media_count: Some(tweet.media.len() as i32),
        created_date: tweet.created_at,
        added_date: None,
        ..Default::default()
    }
}

pub(crate) fn media(tweet: &client::Tweet) -> Vec<model::TwitterMedia> {
    tweet
        .media
        .iter()
        .enumerate()
        .map(|(page, m)| model::TwitterMedia {
            id: m.media_key.clone(),
            tweet_id: tweet.id as i64,
            page: page as i32,
            type_: m.type_.to_string(),
            url: m.url().to_string(),
            width: m.original_info.width as i32,
            height: m.original_info.height as i32,
            preview_image_url: m.preview_image_url().map(|u| u.to_string()),
            duration: m.duration().map(|d| d as i32),
        })
        .collect()
}

pub(crate) fn media_views(tweet: &client::Tweet) -> Vec<MediaView> {
    tweet
        .media
        .iter()
        .enumerate()
        .map(|(page, m)| MediaView {
            media_id: m.media_key.clone(),
            community: "twitter".to_string(),
            post_id: tweet.id.to_string(),
            page_index: page as i32,
            url: original_url(m),
            thumbnail_url: thumbnail_url(m),
            width: Some(m.original_info.width as i32),
            height: Some(m.original_info.height as i32),
            extra: Some(serde_json::json!({"twitter": { "type": m.type_ }})),
        })
        .collect()
}

impl From<model::TwitterAccount> for TwitterAccount {
    fn from(account: model::TwitterAccount) -> Self {
        TwitterAccount {
            id: account.id,
            user_id: account.user_id,
            name: account.name,
            username: account.username,
            profile_image_url: account.profile_image_url,
        }
    }
}

impl TryFrom<model::TwitterWatchList> for TwitterFeed {
    type Error = Error;

    fn try_from(watch_list: model::TwitterWatchList) -> Result<Self> {
        Ok(TwitterFeed {
            id: watch_list.id,
            name: watch_list.name,
            first_fetch_limit: watch_list.first_fetch_limit,
            watching: watch_list.watching,
            account_id: watch_list.account_id,
            params: match watch_list.kind.as_str() {
                "timeline" => TwitterFeedParams::Timeline,
                "bookmarks" => TwitterFeedParams::Bookmarks,
                "likes" => TwitterFeedParams::Likes {
                    user_id: watch_list.user_id.ok_or(Error::ObjectNotComplete(
                        "twitter user id cannot be null for likes feed".to_string(),
                    ))?,
                },
                "posts" => TwitterFeedParams::Posts {
                    user_id: watch_list.user_id.ok_or(Error::ObjectNotComplete(
                        "twitter user id cannot be null for posts feed".to_string(),
                    ))?,
                },
                "list" => TwitterFeedParams::List {
                    list_id: watch_list.twitter_list_id.ok_or(Error::ObjectNotComplete(
                        "twitter list id cannot be null for list feed".to_string(),
                    ))?,
                },
                "search" => TwitterFeedParams::Search {
                    query: watch_list.search_query.ok_or(Error::ObjectNotComplete(
                        "twitter search query cannot be null for search feed".to_string(),
                    ))?,
                },
                _ => Err(Error::UnknownField(format!(
                    "twitter watch list kind {}",
                    watch_list.kind
                )))?,
            },
            reached_end: watch_list.reached_end,
        })
    }
}

impl From<model::TwitterUser> for UserView {
    fn from(user: model::TwitterUser) -> Self {
        UserView {
            user_id: user.id.to_string(),
            community: "twitter".to_string(),
            name: Some(user.name),
            username: Some(user.username),
            avatar_url: user.profile_image_url,
            description: Some(user.description),
            url: user.url,
            ..Default::default()
        }
    }
}

impl From<model::Tweet> for PostView {
    fn from(tweet: model::Tweet) -> Self {
        PostView {
            post_id: tweet.id.to_string(),
            user_id: Some(tweet.user_id.to_string()),
            community: "twitter".to_string(),
            text: tweet.caption,
            thumbnail_url: None,
            created_date: tweet.created_date.and_utc(),
            added_date: Some(tweet.added_date.and_utc()),
            ..Default::default()
        }
    }
}

impl From<model::TwitterMedia> for MediaView {
    fn from(media: model::TwitterMedia) -> Self {
        let url = media.original_url();
        let thumbnail_url = media.thumbnail_url();
        MediaView {
            media_id: media.id,
            community: "twitter".to_string(),
            post_id: media.tweet_id.to_string(),
            page_index: media.page,
            url: Some(url),
            thumbnail_url,
            width: Some(media.width),
            height: Some(media.height),
            extra: Some(serde_json::json!({"twitter": { "type": media.type_ }})),
        }
    }
}

/// Convert a Twitter media to a remote work.
/// Currently, the app only supports one Twitter media as one work.
impl TryFrom<&model::TwitterMedia> for RemoteWork {
    type Error = Error;
    fn try_from(media: &model::TwitterMedia) -> Result<Self> {
        let image = RemoteImage {
            filename: bottle_util::parse_filename(&media.url).map_err(anyhow::Error::from)?,
            url: media.original_url(),
            page_index: None,
        };
        Ok(RemoteWork {
            source: Some(String::from("twitter")),
            post_id: Some(media.tweet_id.to_string()),
            post_id_int: Some(media.tweet_id),
            page_index: Some(media.page),
            media_count: 1,
            images: vec![image],
            ..Default::default()
        })
    }
}

impl model::TwitterMedia {
    fn original_url(&self) -> String {
        match self.type_.as_str() {
            "photo" => format!("{}?name=orig", self.url.clone()),
            _ => self.url.clone(),
        }
    }

    fn thumbnail_url(&self) -> Option<String> {
        match self.type_.as_str() {
            "photo" => Some(format!("{}?name=medium", self.url)),
            _ => self.preview_image_url.clone(),
        }
    }
}

fn original_url(media: &client::Media) -> Option<String> {
    match media.type_.as_str() {
        "photo" => Some(format!("{}?name=orig", media.url())),
        _ => Some(media.url().to_string()),
    }
}

fn thumbnail_url(media: &client::Media) -> Option<String> {
    match media.type_.as_str() {
        "photo" => Some(format!("{}?name=medium", media.url())),
        _ => media.preview_image_url().map(|u| u.to_string()),
    }
}
