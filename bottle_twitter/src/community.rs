use async_trait::async_trait;
use diesel::prelude::*;

use std::collections::HashMap;

use bottle_core::{
    feed::*,
    library::{RemoteImage, RemoteWork},
    Error, Result,
};
use twitter_client::{Account as AccountResult, SessionCookie, TwitterClient};

use crate::{cache::TwitterCache, feed::TwitterFeed};
use crate::{model, util};

pub struct TwitterCommunity;

impl Community for TwitterCommunity {
    type Auth = SessionCookie;
    type Credential = SessionCookie;
    type Account = TwitterAccount;
    type Feed = TwitterFeed;

    fn metadata() -> CommunityMetadata
    where
        Self: Sized,
    {
        CommunityMetadata {
            name: "twitter".to_string(),
            feeds: TwitterFeed::metadata(),
            account: TwitterAccount::metadata(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TwitterAccount {
    pub id: i32,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[async_trait]
impl Account for TwitterAccount {
    type Auth = SessionCookie;
    type Credential = SessionCookie;
    type InfoResponse = AccountResult;

    fn metadata() -> Option<AccountMetadata>
    where
        Self: Sized,
    {
        Some(AccountMetadata {
            credential_scheme: Scheme::Object(HashMap::from([
                ("ct0".to_string(), Scheme::String),
                ("auth_token".to_string(), Scheme::String),
            ])),
            can_fetch_info: true,
            need_refresh: false,
        })
    }

    fn view(&self) -> AccountView {
        AccountView {
            account_id: self.id,
            community: "twitter".to_string(),
        }
    }

    fn info(&self) -> Option<AccountInfo> {
        self.user_id.map(|_| AccountInfo {
            name: self.name.clone(),
            username: self.username.clone(),
            avatar_url: self.profile_image_url.clone(),
        })
    }

    fn expired(&self) -> bool {
        false
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::twitter_account::dsl::*;
        let results = twitter_account
            .load::<model::TwitterAccount>(db)?
            .into_iter()
            .map(Self::from)
            .collect();
        Ok(results)
    }

    fn get(db: Database, account_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::twitter_account::dsl::*;
        let result = twitter_account
            .filter(id.eq(account_id))
            .first::<model::TwitterAccount>(db)
            .optional()?;
        Ok(result.map(Self::from))
    }

    fn delete(db: Database, account_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::twitter_account::dsl::*;
        diesel::delete(twitter_account.filter(id.eq(account_id))).execute(db)?;
        tracing::info!("Deleted twitter account {}", account_id);
        Ok(())
    }

    fn add(db: Database, credential: &Self::Credential) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::twitter_account::dsl::*;
        let new_account = model::NewTwitterAccount {
            cookies: credential.to_string(),
        };
        let result = diesel::insert_into(twitter_account)
            .values(&new_account)
            .get_result::<model::TwitterAccount>(db)?;
        tracing::info!("Added twitter account {}", result.id);
        Ok(Self::from(result))
    }

    fn update(&self, db: Database, info: &Self::InfoResponse) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::twitter_account::dsl::*;
        let account_update = model::TwitterAccountUpdate {
            user_id: info.id as i64,
            name: info.name.clone(),
            username: info.username.clone(),
            profile_image_url: info.profile_image_url_https.clone().unwrap_or_default(),
        };
        let result = diesel::update(twitter_account.filter(id.eq(self.id)))
            .set(&account_update)
            .returning(model::TwitterAccount::as_returning())
            .get_result(db)?;
        tracing::info!("Updated twitter account {}: {:?}", self.id, info);
        Ok(Self::from(result))
    }

    fn auth(&self, db: Database) -> Result<Option<Self::Auth>> {
        Ok(Some(self.credential(db)?))
    }

    fn credential(&self, db: Database) -> Result<Self::Credential> {
        use bottle_core::schema::twitter_account::dsl::*;
        let cookie_str = twitter_account
            .select(cookies)
            .filter(id.eq(self.id))
            .first::<String>(db);
        let result = cookie_str?.parse::<Self::Credential>().map_err(anyhow::Error::from)?;
        Ok(result)
    }

    async fn fetch(credential: &Self::Credential) -> Result<Self::InfoResponse> {
        let client = TwitterClient::new(credential.clone()).map_err(anyhow::Error::from)?;
        let mut accounts = client.accounts().await.map_err(anyhow::Error::from)?;
        accounts.pop().ok_or(Error::NotLoggedIn("No account".to_string()))
    }
}

impl TwitterAccount {
    pub fn default(db: Database) -> Result<Self> {
        use bottle_core::schema::twitter_account::dsl::*;
        let result = twitter_account.first::<model::TwitterAccount>(db)?;
        Ok(Self::from(result))
    }
}

#[derive(Debug, Clone)]
pub struct TwitterPost {
    pub tweet: model::Tweet,
    pub media: Vec<model::TwitterMedia>,
}

impl Post for TwitterPost {
    type Cache = TwitterCache;

    fn get(db: Database, cache: &Self::Cache, post_id: &str) -> Result<Option<Self>> {
        use bottle_core::schema::{tweet, twitter_media, twitter_user};
        let post_id = post_id.parse::<i64>()?;

        // 1. Try to get the tweet from database
        let mut result = tweet::table.find(post_id).first::<model::Tweet>(db).optional()?;

        // 1.1 If not found, try to get from cache and save to database
        if result.is_none() {
            if let Some(tweet) = cache.tweets.get(&(post_id as u64)) {
                let new_tweet = model::NewTweet::from(tweet);
                let new_user = model::NewTwitterUser::from(&tweet.user);
                let media = util::media(tweet);

                result = db.transaction(|conn| -> Result<Option<model::Tweet>> {
                    diesel::insert_into(twitter_user::table)
                        .values(&new_user)
                        .execute(conn)?;
                    let result = Some(
                        diesel::insert_into(tweet::table)
                            .values(&new_tweet)
                            .returning(model::Tweet::as_returning())
                            .get_result(conn)?,
                    );
                    if !media.is_empty() {
                        diesel::insert_into(twitter_media::table).values(&media).execute(conn)?;
                    }
                    Ok(result)
                })?;

                tracing::info!("Saved tweet {} from cache", post_id);
            }
        }
        let Some(tweet) = result else {
            return Ok(None);
        };

        // 2. Fetch associated media
        let media = twitter_media::table
            .filter(twitter_media::tweet_id.eq(post_id))
            .order(twitter_media::page.asc())
            .load::<model::TwitterMedia>(db)?;

        Ok(Some(Self { tweet, media }))
    }

    fn add_to_library(&self, db: Database, page: Option<i32>) -> Result<GeneralResponse> {
        let media = if let Some(page) = page {
            let page = page as usize;
            self.media[page..=page].iter()
        } else {
            self.media.iter()
        };

        let images = media
            .map(|media| RemoteImage {
                url: match media.type_.as_str() {
                    "photo" => format!("{}?name=orig", media.url),
                    _ => media.url.clone(),
                },
                filename: bottle_util::parse_filename(&media.url).unwrap_or(format!("{}.jpg", media.id)),
                page_index: Some(media.page),
            })
            .collect::<Vec<_>>();

        let remote_work = RemoteWork {
            source: Some("twitter".to_string()),
            post_id: Some(self.tweet.id.to_string()),
            post_id_int: Some(self.tweet.id),
            page_index: page,
            media_count: images.len() as i32,
            images,
            caption: Some(self.tweet.caption.clone()),
            ..Default::default()
        };
        bottle_library::add_remote_work(db, &remote_work)
    }
}

pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
    use bottle_core::schema::{tweet, twitter_media, twitter_user};

    // 1. Fetch posts
    let posts = tweet::table
        .filter(tweet::id.eq_any(post_ids))
        .select(tweet::all_columns)
        .load::<model::Tweet>(db)?;

    // 2. Fetch associated users
    let user_ids = posts.iter().map(|post| post.user_id);
    let users = twitter_user::table
        .filter(twitter_user::id.eq_any(user_ids))
        .load::<model::TwitterUser>(db)?;

    // 3. Fetch associated media
    let post_ids = posts.iter().map(|post| post.id);
    let media = twitter_media::table
        .filter(twitter_media::tweet_id.eq_any(post_ids.clone()))
        .order(twitter_media::page.asc())
        .load::<model::TwitterMedia>(db)?;

    Ok(GeneralResponse {
        posts: Some(posts.into_iter().map(PostView::from).collect()),
        users: Some(users.into_iter().map(UserView::from).collect()),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        ..Default::default()
    })
}
