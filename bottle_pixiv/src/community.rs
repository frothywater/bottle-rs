use async_trait::async_trait;
use diesel::prelude::*;
use serde::Serialize;

use bottle_core::{
    feed::*,
    library::{RemoteImage, RemoteWork},
    Result,
};
use pixiv_client::{LoginResponse, PixivClient};

use crate::cache::PixivCache;
use crate::feed::PixivFeed;
use crate::model;
use crate::util;

#[derive(Debug, Clone)]

pub struct RefreshToken(pub String);
#[derive(Debug, Clone)]

pub struct AccessToken(pub String);

pub struct PixivCommunity;

impl Community for PixivCommunity {
    type Auth = AccessToken;
    type Credential = RefreshToken;
    type Account = PixivAccount;
    type Feed = PixivFeed;

    fn metadata() -> CommunityMetadata
    where
        Self: Sized,
    {
        CommunityMetadata {
            name: "pixiv".to_string(),
            feeds: PixivFeed::metadata(),
            account: PixivAccount::metadata(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PixivAccount {
    pub id: i32,
    pub expiry: Option<chrono::NaiveDateTime>,
    pub user_id: Option<i64>,
    pub name: Option<String>,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[async_trait]
impl Account for PixivAccount {
    type Auth = AccessToken;
    type Credential = RefreshToken;
    type InfoResponse = LoginResponse;

    fn metadata() -> Option<AccountMetadata>
    where
        Self: Sized,
    {
        Some(AccountMetadata {
            credential_scheme: Scheme::String,
            can_fetch_info: true,
            need_refresh: true,
        })
    }

    fn view(&self) -> AccountView {
        AccountView {
            account_id: self.id,
            community: "pixiv".to_string(),
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
        self.expiry
            .map(|expiry| expiry < chrono::Utc::now().naive_utc())
            .unwrap_or(true)
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_account::dsl::*;
        let results = pixiv_account
            .load::<model::PixivAccount>(db)?
            .into_iter()
            .map(Self::from)
            .collect();
        Ok(results)
    }

    fn get(db: Database, account_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_account::dsl::*;
        let result = pixiv_account
            .filter(id.eq(account_id))
            .first::<model::PixivAccount>(db)
            .optional()?;
        Ok(result.map(Self::from))
    }

    fn delete(db: Database, account_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_account::dsl::*;
        diesel::delete(pixiv_account.filter(id.eq(account_id))).execute(db)?;
        tracing::info!("Deleted pixiv account {}", account_id);
        Ok(())
    }

    fn add(db: Database, credential: &Self::Credential) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_account::dsl::*;
        let new_account = model::NewPixivAccount {
            refresh_token: credential.0.clone(),
        };
        let result = diesel::insert_into(pixiv_account)
            .values(&new_account)
            .get_result::<model::PixivAccount>(db)?;
        tracing::info!("Added pixiv account {}", result.id);
        Ok(Self::from(result))
    }

    fn update(&self, db: Database, info: &Self::InfoResponse) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::pixiv_account::dsl::*;
        let account_update = model::PixivAccountUpdate {
            access_token: Some(info.access_token.clone()),
            refresh_token: Some(info.refresh_token.clone()),
            user_id: Some(info.user.id as i64),
            name: Some(info.user.name.clone()),
            username: Some(info.user.username.clone()),
            profile_image_url: info.user.profile_image_urls.large.clone(),
            expiry: Some(chrono::Utc::now().naive_utc() + chrono::Duration::seconds(info.expires_in as i64)),
        };
        let result = diesel::update(pixiv_account.filter(id.eq(self.id)))
            .set(&account_update)
            .returning(model::PixivAccount::as_returning())
            .get_result::<model::PixivAccount>(db)?;
        tracing::info!("Updated pixiv account {}: {:?}", self.id, info.user);
        Ok(Self::from(result))
    }

    fn auth(&self, db: Database) -> Result<Option<Self::Auth>> {
        use bottle_core::schema::pixiv_account::dsl::*;
        let result = pixiv_account
            .filter(id.eq(self.id))
            .select(access_token)
            .first::<Option<String>>(db)?;
        Ok(result.map(AccessToken))
    }

    fn credential(&self, db: Database) -> Result<Self::Credential> {
        use bottle_core::schema::pixiv_account::dsl::*;
        let result = pixiv_account
            .filter(id.eq(self.id))
            .select(refresh_token)
            .first::<String>(db)?;
        Ok(RefreshToken(result))
    }

    async fn fetch(credential: &Self::Credential) -> Result<Self::InfoResponse> {
        let result = PixivClient::login(&credential.0).await.map_err(anyhow::Error::from)?;
        Ok(result)
    }
}

impl PixivAccount {
    pub fn default(db: Database) -> Result<Self> {
        use bottle_core::schema::pixiv_account::dsl::*;
        let result = pixiv_account.first::<model::PixivAccount>(db)?;
        Ok(Self::from(result))
    }
}

#[derive(Debug, Clone)]
pub struct PixivPost {
    pub illust: model::PixivIllust,
    pub media: Vec<model::PixivMedia>,
}

impl Post for PixivPost {
    type Cache = PixivCache;

    fn get(db: Database, cache: &PixivCache, post_id: &str) -> Result<Option<Self>> {
        use bottle_core::schema::{pixiv_illust, pixiv_illust_tag, pixiv_media, pixiv_user};
        let post_id = post_id.parse::<i64>()?;

        // 1. Try to get the post from the database
        let mut result = pixiv_illust::table
            .find(post_id)
            .first::<model::PixivIllust>(db)
            .optional()?;

        // 1.1. If not found, try to get from cache and update the database
        if result.is_none() {
            if let Some(illust) = cache.illusts.get(&(post_id as u64)) {
                let new_illust = model::NewPixivIllust::from(illust);
                let new_user = model::NewPixivUser::from(&illust.user);
                let tags = util::tags(illust);
                let media = util::media(illust);

                result = db.transaction(|conn| -> Result<Option<model::PixivIllust>> {
                    diesel::insert_into(pixiv_user::table).values(&new_user).execute(conn)?;
                    let result = Some(
                        diesel::insert_into(pixiv_illust::table)
                            .values(&new_illust)
                            .returning(model::PixivIllust::as_returning())
                            .get_result(conn)?,
                    );
                    diesel::insert_into(pixiv_media::table).values(media).execute(conn)?;
                    diesel::insert_into(pixiv_illust_tag::table)
                        .values(tags)
                        .execute(conn)?;
                    Ok(result)
                })?;

                tracing::info!("Added pixiv illust {} from cache", post_id);
            }
        }
        let Some(illust) = result else { return Ok(None) };

        // 2. Get associated media
        let media = pixiv_media::table
            .filter(pixiv_media::illust_id.eq(post_id))
            .order(pixiv_media::page.asc())
            .load::<model::PixivMedia>(db)?;

        Ok(Some(Self { illust, media }))
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
                url: media.original_url.clone(),
                filename: bottle_util::parse_filename(&media.original_url)
                    .unwrap_or(format!("{}_{}.jpg", media.illust_id, media.page)),
                page_index: Some(media.page),
            })
            .collect::<Vec<_>>();
        let remote_work = RemoteWork {
            source: Some("pixiv".to_string()),
            post_id: Some(self.illust.id.to_string()),
            post_id_int: Some(self.illust.id),
            page_index: page,
            media_count: images.len() as i32,
            images,
            name: Some(self.illust.title.clone()),
            caption: Some(self.illust.caption.clone()),
        };

        bottle_library::add_remote_work(db, &remote_work)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PixivIllustExtra {
    pub title: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub restrict: bool,
    pub sanity_level: i32,
    pub series_id: Option<i64>,
    pub series_title: Option<String>,
}

pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
    use bottle_core::schema::{pixiv_illust, pixiv_media, pixiv_user};

    // 1. Fetch posts
    let posts = pixiv_illust::table
        .filter(pixiv_illust::id.eq_any(post_ids))
        .select(pixiv_illust::all_columns)
        .load::<model::PixivIllust>(db)?;

    // 2. Fetch associated users
    let user_ids = posts.iter().map(|post| post.user_id);
    let users = pixiv_user::table
        .filter(pixiv_user::id.eq_any(user_ids))
        .load::<model::PixivUser>(db)?;

    // 3. Fetch associated media
    let post_ids = posts.iter().map(|post| post.id);
    let media = pixiv_media::table
        .filter(pixiv_media::illust_id.eq_any(post_ids.clone()))
        .order(pixiv_media::page.asc())
        .load::<model::PixivMedia>(db)?;

    // 4. Fetch associated tags
    let tags = util::get_tag_map(db, post_ids.clone())?;
    let posts = posts
        .into_iter()
        .map(|illust| illust.post_view(tags.get(&illust.id).cloned().unwrap_or_default()))
        .collect();

    Ok(GeneralResponse {
        posts: Some(posts),
        users: Some(users.into_iter().map(UserView::from).collect()),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        ..Default::default()
    })
}
