mod consts;
mod error;
mod parsing;
mod result;
mod selectors;

use reqwest::{header, Client, Url};
use scraper::Html;
use serde::Deserialize;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use bottle_util::{build_params, parsing::parse_query_str};

use crate::consts::*;
pub use crate::error::Error;
use crate::error::Result;
use crate::parsing::*;
pub use crate::result::*;

#[derive(Debug, Clone)]
pub struct PandaClient {
    pub cookie: PandaCookie,
    client: reqwest::Client,
}

impl PandaClient {
    pub fn new(cookie: PandaCookie) -> Result<Self> {
        let cookie_string = cookie.to_string();
        let mut headers = header::HeaderMap::new();
        headers.insert(header::COOKIE, header::HeaderValue::from_str(&cookie_string).unwrap());

        let client = Client::builder().default_headers(headers).build()?;

        Ok(PandaClient { cookie, client })
    }

    pub async fn search(&self, option: &SearchOption, offset: Option<&GalleryListOffset>) -> Result<GalleryListResult> {
        let params = [option.query(), offset.map(|offset| offset.query()).unwrap_or_default()].concat();
        let doc = self.fetch("/", params).await?;
        parse_gallery_list(&doc)
    }

    pub async fn watched(
        &self,
        option: &SearchOption,
        offset: Option<&GalleryListOffset>,
    ) -> Result<GalleryListResult> {
        let params = [option.query(), offset.map(|offset| offset.query()).unwrap_or_default()].concat();
        let doc = self.fetch("/watched", params).await?;
        parse_gallery_list(&doc)
    }

    pub async fn favorites(
        &self,
        option: &FavoriteSearchOption,
        offset: Option<&GalleryListOffset>,
    ) -> Result<GalleryListResult> {
        let params = [option.query(), offset.map(|offset| offset.query()).unwrap_or_default()].concat();
        let doc = self.fetch("/favorites.php", params).await?;
        parse_gallery_list(&doc)
    }

    pub async fn gallery(&self, gid: u64, token: &str, page: u32) -> Result<GalleryPageResult> {
        let path = format!("/g/{}/{}/", gid, token);
        let params = build_params! { required p => page };
        let doc = self.fetch(&path, params).await?;
        parse_gallery_page(&doc)
    }

    pub async fn image(&self, gid: u64, token: &str, index: u32) -> Result<ImageResult> {
        let path = format!("/s/{}/{}-{}", token, gid, index + 1);
        let doc = self.fetch(&path, vec![]).await?;
        parse_image_page(&doc)
    }
}

impl PandaClient {
    async fn fetch(&self, path: &str, query: impl IntoIterator<Item = (String, String)>) -> Result<Html> {
        let mut url = Url::parse(BASE_URL)?;
        url.set_path(path);
        url.query_pairs_mut().extend_pairs(query);

        let response = self.client.get(url).send().await?.error_for_status()?;
        let html = response.text().await?;
        log(path, &html).await?;

        let doc = Html::parse_document(&html);
        if let Some(s) = parse_ban(&doc) {
            return Err(Error::RateLimit(s));
        }
        Ok(doc)
    }
}

async fn log(path: &str, content: &str) -> Result<()> {
    use std::path::PathBuf;
    use tokio::{fs::File, io::AsyncWriteExt};

    if let Ok(dir) = std::env::var("CLIENT_LOG_DIR") {
        let name = path.strip_prefix('/').unwrap_or(path).replace('/', "_");
        let name = if name.is_empty() { "home" } else { &name };
        let time = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filepath = PathBuf::from(dir).join(format!("panda_{}_{}.html", name, time));
        let mut file = File::create(filepath).await?;
        file.write_all(content.as_bytes()).await?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PandaCookie {
    pub content: String,
}

impl Display for PandaCookie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Set `sl=dm_2` to force extended page view!
        write!(f, "{}; sl=dm_2", self.content)
    }
}

impl FromStr for PandaCookie {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(PandaCookie { content: s.to_string() })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchOption {
    pub keyword: Option<String>,
    pub categories: Vec<GalleryCategory>,
    pub search_name: bool,
    pub search_tags: bool,
    pub search_description: bool,
    pub search_torrent: bool,
    pub search_low_power_tags: bool,
    pub search_downvoted_tags: bool,
    pub search_expunged: bool,
    pub require_torrent: bool,
    pub disable_language_filter: bool,
    pub disable_uploader_filter: bool,
    pub disable_tags_filter: bool,
    pub min_rating: Option<u32>,
    pub min_pages: Option<u32>,
    pub max_pages: Option<u32>,
}

impl SearchOption {
    pub fn query(&self) -> Vec<(String, String)> {
        fn categories_query_str(categories: &[GalleryCategory]) -> String {
            let flags = categories.iter().fold(0, |acc, category| acc | (1 << *category as u32));
            (!flags & 0x3ff).to_string()
        }

        build_params! {
            optional f_search => self.keyword,
            required f_cats => categories_query_str(&self.categories),
            required advsearch => 1,
            optional f_sname => self.search_name.then_some("on"),
            optional f_stags => self.search_tags.then_some("on"),
            optional f_sdesc => self.search_description.then_some("on"),
            optional f_storr => self.search_torrent.then_some("on"),
            optional f_sdt1 => self.search_low_power_tags.then_some("on"),
            optional f_sdt2 => self.search_downvoted_tags.then_some("on"),
            optional f_sh => self.search_expunged.then_some("on"),
            optional f_sto => self.require_torrent.then_some("on"),
            optional f_sr => self.min_rating.is_some().then_some("on"),
            optional f_srdd => self.min_rating,
            optional f_sp => self.min_pages.or(self.max_pages).is_some().then_some("on"),
            optional f_spf => self.min_pages,
            optional f_spt => self.max_pages,
            optional f_sfl => self.disable_language_filter.then_some("on"),
            optional f_sfu => self.disable_uploader_filter.then_some("on"),
            optional f_sft => self.disable_tags_filter.then_some("on"),
        }
    }
}

impl Default for SearchOption {
    fn default() -> Self {
        SearchOption {
            keyword: None,
            categories: ALL_GALLERY_CATEGORIES.clone(),
            search_name: true,
            search_tags: true,
            search_description: false,
            search_torrent: false,
            search_low_power_tags: false,
            search_downvoted_tags: false,
            search_expunged: false,
            require_torrent: false,
            disable_language_filter: false,
            disable_uploader_filter: false,
            disable_tags_filter: false,
            min_rating: None,
            min_pages: None,
            max_pages: None,
        }
    }
}

impl FromStr for SearchOption {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        fn parse_categories(s: &str) -> Option<Vec<GalleryCategory>> {
            let flags = s.parse::<u32>().ok()?;
            let flags = !flags & 0x3ff;
            Some(
                ALL_GALLERY_CATEGORIES
                    .iter()
                    .filter(|category| flags & (**category as u32) != 0)
                    .cloned()
                    .collect(),
            )
        }

        fn parse(params: &HashMap<String, String>) -> SearchOption {
            SearchOption {
                keyword: params.get("f_search").cloned(),
                categories: params
                    .get("f_cats")
                    .and_then(|s| parse_categories(s))
                    .unwrap_or(ALL_GALLERY_CATEGORIES.clone()),
                search_name: params.get("f_sname").is_some(),
                search_tags: params.get("f_stags").is_some(),
                search_description: params.get("f_sdesc").is_some(),
                search_torrent: params.get("f_storr").is_some(),
                search_low_power_tags: params.get("f_sdt1").is_some(),
                search_downvoted_tags: params.get("f_sdt2").is_some(),
                search_expunged: params.get("f_sh").is_some(),
                require_torrent: params.get("f_sto").is_some(),
                disable_language_filter: params.get("f_sfl").is_some(),
                disable_uploader_filter: params.get("f_sfu").is_some(),
                disable_tags_filter: params.get("f_sft").is_some(),
                min_rating: params.get("f_srdd").and_then(|s| s.parse().ok()),
                min_pages: params.get("f_spf").and_then(|s| s.parse().ok()),
                max_pages: params.get("f_spt").and_then(|s| s.parse().ok()),
            }
        }

        let params = parse_query_str(s)?;
        Ok(parse(&params))
    }
}

impl Display for SearchOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.query()
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FavoriteSearchOption {
    pub keyword: Option<String>,
    pub category_index: Option<u32>,
    pub search_name: bool,
    pub search_tags: bool,
    pub search_note: bool,
}

impl FavoriteSearchOption {
    pub fn query(&self) -> Vec<(String, String)> {
        build_params! {
            optional favcat => self.category_index,
            optional f_search => self.keyword,
            optional sn => self.search_name.then_some("on"),
            optional st => self.search_tags.then_some("on"),
            optional sf => self.search_note.then_some("on"),
        }
    }
}

impl Default for FavoriteSearchOption {
    fn default() -> Self {
        FavoriteSearchOption {
            keyword: None,
            category_index: None,
            search_name: true,
            search_tags: true,
            search_note: true,
        }
    }
}

impl FromStr for FavoriteSearchOption {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        fn parse(params: &HashMap<String, String>) -> FavoriteSearchOption {
            FavoriteSearchOption {
                keyword: params.get("f_search").cloned(),
                category_index: params.get("favcat").and_then(|s| s.parse().ok()),
                search_name: params.get("sn").is_some(),
                search_tags: params.get("st").is_some(),
                search_note: params.get("sf").is_some(),
            }
        }

        let params = parse_query_str(s)?;
        Ok(parse(&params))
    }
}

impl Display for FavoriteSearchOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.query()
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        )
    }
}

impl GalleryListOffset {
    fn query(&self) -> Vec<(String, String)> {
        match self {
            GalleryListOffset::NewerThan(id) => build_params! { required prev => id },
            GalleryListOffset::OlderThan(id) => build_params! { required next => id },
            GalleryListOffset::Percentage(percent) => build_params! { required range => percent },
        }
    }
}
