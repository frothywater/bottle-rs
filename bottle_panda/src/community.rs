use async_trait::async_trait;
use diesel::prelude::*;
use serde::Serialize;

use std::collections::HashMap;

use bottle_core::{
    feed::*,
    library::{RemoteImage, RemoteWork},
    Result,
};
use panda_client::PandaCookie;

use crate::cache::{self, PandaCache};
use crate::feed::PandaFeed;
use crate::model;

pub struct PandaCommunity;

impl Community for PandaCommunity {
    type Auth = PandaCookie;
    type Credential = PandaCookie;
    type Account = PandaAccount;
    type Feed = PandaFeed;

    fn metadata() -> CommunityMetadata
    where
        Self: Sized,
    {
        CommunityMetadata {
            name: "panda".to_string(),
            feeds: PandaFeed::metadata(),
            account: PandaAccount::metadata(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PandaAccount {
    pub id: i32,
    pub name: Option<String>,
    pub username: Option<String>,
}

#[async_trait]
impl Account for PandaAccount {
    type Auth = PandaCookie;
    type Credential = PandaCookie;
    type InfoResponse = ();

    fn metadata() -> Option<AccountMetadata>
    where
        Self: Sized,
    {
        Some(AccountMetadata {
            credential_scheme: Scheme::Object(HashMap::from([
                ("ipb_member_id".to_string(), Scheme::String),
                ("ipb_pass_hash".to_string(), Scheme::String),
                ("igneous".to_string(), Scheme::String),
            ])),
            can_fetch_info: false,
            need_refresh: false,
        })
    }

    fn view(&self) -> AccountView {
        AccountView {
            account_id: self.id,
            community: "panda".to_string(),
        }
    }

    fn info(&self) -> Option<AccountInfo> {
        Some(AccountInfo {
            name: self.name.clone(),
            username: self.username.clone(),
            ..Default::default()
        })
    }

    fn expired(&self) -> bool {
        false
    }

    fn all(db: Database) -> Result<Vec<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_account::dsl::*;
        let results = panda_account
            .load::<model::PandaAccount>(db)?
            .into_iter()
            .map(Self::from)
            .collect();
        Ok(results)
    }

    fn get(db: Database, account_id: i32) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_account::dsl::*;
        let result = panda_account
            .filter(id.eq(account_id))
            .first::<model::PandaAccount>(db)
            .optional()?;
        Ok(result.map(Self::from))
    }

    fn delete(db: Database, account_id: i32) -> Result<()>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_account::dsl::*;
        diesel::delete(panda_account.filter(id.eq(account_id))).execute(db)?;
        tracing::info!("Deleted panda account {}", account_id);
        Ok(())
    }

    fn add(db: Database, credential: &Self::Credential) -> Result<Self>
    where
        Self: Sized,
    {
        use bottle_core::schema::panda_account::dsl::*;
        let new_account = model::NewPandaAccount {
            cookies: credential.to_string(),
            ..Default::default()
        };
        let result = diesel::insert_into(panda_account)
            .values(&new_account)
            .get_result::<model::PandaAccount>(db)?;
        tracing::info!("Added panda account {}", result.id);
        Ok(Self::from(result))
    }

    fn update(&self, _db: Database, _info: &Self::InfoResponse) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }

    fn auth(&self, db: Database) -> Result<Option<Self::Auth>> {
        Ok(Some(self.credential(db)?))
    }

    fn credential(&self, db: Database) -> Result<Self::Credential> {
        use bottle_core::schema::panda_account::dsl::*;
        let cookie_str = panda_account
            .filter(id.eq(self.id))
            .select(cookies)
            .first::<String>(db)?;
        let result = cookie_str.parse::<PandaCookie>().map_err(anyhow::Error::from)?;
        Ok(result)
    }

    async fn fetch(_credential: &Self::Credential) -> Result<Self::InfoResponse> {
        unimplemented!()
    }
}

impl PandaAccount {
    pub fn default(db: Database) -> Result<Self> {
        use bottle_core::schema::panda_account::dsl::*;
        let result = panda_account.first::<model::PandaAccount>(db)?;
        Ok(Self::from(result))
    }
}

#[derive(Debug, Clone)]
pub struct PandaPost {
    pub gallery: model::PandaGallery,
    pub media: Vec<model::PandaMedia>,
}

impl Post for PandaPost {
    type Cache = PandaCache;

    fn get(db: Database, cache: &PandaCache, post_id: &str) -> Result<Option<Self>> {
        let post_id = post_id.parse::<i64>()?;

        let Some(gallery) = cache::get_gallery(db, cache, post_id)? else {
            return Ok(None);
        };
        let media = cache::get_media(db, cache, post_id)?;

        Ok(Some(Self { gallery, media }))
    }

    fn add_to_library(&self, db: Database, _page: Option<i32>) -> Result<GeneralResponse> {
        let mut images = Vec::new();
        for m in self.media.iter() {
            // Allow adding gallery that has incomplete media
            if let (Some(url), Some(filename)) = (&m.url, &m.filename) {
                images.push(RemoteImage {
                    url: url.clone(),
                    filename: filename.clone(),
                    page_index: Some(m.media_index),
                });
            }
        }
        let work = RemoteWork {
            source: Some("panda".to_string()),
            post_id: Some(self.gallery.id.to_string()),
            post_id_int: Some(self.gallery.id),
            page_index: None,
            media_count: self.gallery.media_count,
            images,
            name: Some(self.gallery.title.clone()),
            ..Default::default()
        };
        bottle_library::add_remote_work(db, &work)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct PandaGalleryExtra {
    pub token: String,
    pub category: String,
    pub uploader: String,
    pub rating: f32,
    pub english_title: Option<String>,
    pub parent: Option<String>,
    pub visible: Option<bool>,
    pub language: Option<String>,
    pub file_size: Option<i32>,
}
