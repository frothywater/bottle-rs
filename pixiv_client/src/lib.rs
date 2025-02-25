mod consts;
mod error;
mod response;
mod result;

use reqwest::{header, Client, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use bottle_util::build_params;

use crate::consts::*;
pub use crate::error::Error;
use crate::error::Result;
pub use crate::result::*;

#[derive(Debug, Clone)]
pub struct PixivClient {
    client: reqwest::Client,
}

impl PixivClient {
    pub fn new(access_token: &str) -> Result<PixivClient> {
        let auth_str = format!("Bearer {}", access_token);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&auth_str).unwrap());

        let client = Client::builder().default_headers(headers).build()?;

        Ok(PixivClient { client })
    }

    pub async fn login(refresh_token: &str) -> Result<LoginResponse> {
        let datetime = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let digest = md5::compute(format!("{}{}", datetime, HASH_SECRET));
        let digest = format!("{:x}", digest);

        let mut headers = header::HeaderMap::new();
        headers.insert("X-Client-Time", header::HeaderValue::from_str(&datetime).unwrap());
        headers.insert("X-Client-Hash", header::HeaderValue::from_str(&digest).unwrap());

        let mut form = HashMap::new();
        form.insert("client_id", CLIENT_ID);
        form.insert("client_secret", CLIENT_SECRET);
        form.insert("get_secure_url", "1");
        form.insert("grant_type", "refresh_token");
        form.insert("refresh_token", refresh_token);

        let client = Client::builder().default_headers(headers).build()?;
        let response = client.post(LOGIN_URL).form(&form).send().await?.error_for_status()?;
        let content = response.text().await?;

        log("login", &content).await?;
        let result: LoginResponse = serde_json::from_str(&content)?;
        Ok(result)
    }

    pub async fn illust(&self, id: u64) -> Result<Illust> {
        let params = build_params! { required illust_id => id };
        self.get("/", params).await
    }

    pub async fn ugoira(&self, id: u64) -> Result<Ugoira> {
        let params = build_params! { required illust_id => id };
        self.get("/v1/ugoira/metadata", params).await
    }

    pub async fn user(&self, id: u64) -> Result<UserDetail> {
        let params = build_params! { required user_id => id };
        self.get("/v1/user/detail", params).await
    }

    pub async fn following(&self, user_id: u64, restriction: Restriction, offset: Option<u32>) -> Result<UserList> {
        let params = build_params! {
            required user_id,
            required restrict => restriction,
            optional offset,
        };
        self.get("/v1/user/following", params).await
    }

    pub async fn followers(&self, user_id: u64, offset: Option<u32>) -> Result<UserList> {
        let params = build_params! {
            required user_id,
            optional offset,
        };
        self.get("/v1/user/follower", params).await
    }

    pub async fn user_illusts(&self, user_id: u64, type_: IllustType, offset: Option<u32>) -> Result<IllustList> {
        let params = build_params! {
            required user_id,
            required type => type_,
            optional offset,
        };
        self.get("/v1/user/illusts", params).await
    }

    pub async fn following_illusts(
        &self,
        restriction: FollowingRestriction,
        offset: Option<u32>,
    ) -> Result<IllustList> {
        let params = build_params! {
            required restrict => restriction,
            optional offset,
        };
        self.get("/v2/illust/follow", params).await
    }

    pub async fn user_bookmarks(
        &self,
        user_id: u64,
        restriction: Restriction,
        tag: Option<&str>,
        max_bookmark_id: Option<u64>,
    ) -> Result<IllustList> {
        let params = build_params! {
            required user_id,
            required restrict => restriction,
            optional tag,
            optional max_bookmark_id,
        };
        self.get("/v1/user/bookmarks/illust", params).await
    }

    pub async fn bookmark_tags(&self, restriction: Restriction, offset: Option<u32>) -> Result<BookmarkTagList> {
        let params = build_params! {
            required restrict => restriction,
            optional offset,
        };
        self.get("/v1/user/bookmark-tags/illust", params).await
    }

    pub async fn bookmark_detail(&self, illust_id: u64) -> Result<BookmarkDetail> {
        let params = build_params! { required illust_id };
        self.get("/v2/illust/bookmark/detail", params).await
    }

    pub async fn related_illusts(
        &self,
        illust_id: u64,
        seed_ids: Vec<u64>,
        viewed_ids: Vec<u64>,
    ) -> Result<IllustList> {
        let params = build_params! {
            required illust_id,
            repeated seed_illust_ids => seed_ids,
            repeated viewed => viewed_ids,
        };
        self.get("/v2/illust/related", params).await
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum IllustType {
    Illust,
    Manga,
    Ugoira,
}

impl Display for IllustType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IllustType::Illust => write!(f, "illust"),
            IllustType::Manga => write!(f, "manga"),
            IllustType::Ugoira => write!(f, "ugoira"),
        }
    }
}

impl FromStr for IllustType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "illust" => Ok(IllustType::Illust),
            "manga" => Ok(IllustType::Manga),
            "ugoira" => Ok(IllustType::Ugoira),
            _ => Err(Error::InvalidField(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Restriction {
    Public,
    Private,
}

impl Display for Restriction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Restriction::Public => write!(f, "public"),
            Restriction::Private => write!(f, "private"),
        }
    }
}

impl FromStr for Restriction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "public" => Ok(Restriction::Public),
            "private" => Ok(Restriction::Private),
            _ => Err(Error::InvalidField(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum FollowingRestriction {
    Public,
    Private,
    All,
}

impl Display for FollowingRestriction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FollowingRestriction::Public => write!(f, "public"),
            FollowingRestriction::Private => write!(f, "private"),
            FollowingRestriction::All => write!(f, "all"),
        }
    }
}

impl FromStr for FollowingRestriction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "public" => Ok(FollowingRestriction::Public),
            "private" => Ok(FollowingRestriction::Private),
            "all" => Ok(FollowingRestriction::All),
            _ => Err(Error::InvalidField(s.to_string())),
        }
    }
}

impl PixivClient {
    async fn get<T, I>(&self, path: &str, query: I) -> Result<T>
    where
        T: DeserializeOwned,
        I: IntoIterator<Item = (String, String)>,
    {
        let mut url = Url::parse(APP_API)?;
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(query);

        let response = self.client.get(url).send().await?.error_for_status()?;
        let content = response.text().await?;

        log(path, &content).await?;
        let result = serde_json::from_str::<T>(&content)?;
        Ok(result)
    }
}

async fn log(path: &str, content: &str) -> Result<()> {
    use std::path::PathBuf;
    use tokio::{fs::File, io::AsyncWriteExt};

    if let Ok(dir) = std::env::var("CLIENT_LOG_DIR") {
        let name = path.strip_prefix('/').unwrap_or(path).replace('/', "_");
        let time = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filepath = PathBuf::from(dir).join(format!("pixiv_{}_{}.json", name, time));
        let mut file = File::create(filepath).await?;
        file.write_all(content.as_bytes()).await?;
    }
    Ok(())
}
