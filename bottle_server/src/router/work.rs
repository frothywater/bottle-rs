use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{delete, get, post},
    Router,
};

use std::collections::HashMap;

use bottle_core::feed::{Feed, GeneralResponse, Post};
use bottle_panda::{PandaFeed, PandaPost};
use bottle_pixiv::{PixivFeed, PixivPost};
use bottle_twitter::{TwitterFeed, TwitterPost};
use bottle_yandere::{YandereFeed, YanderePost};

use crate::{
    error::Result,
    state::AppState,
    util::{get_page_and_size, DEFAULT_RECENT_COUNT},
};

pub fn work_router() -> Router<AppState> {
    Router::new()
        .route("/:community/post/:id/work", post(add_work))
        .route("/work/:id", delete(delete_work))
        .route("/:community/works", get(get_archived_posts))
        .route("/:community/work/users", get(get_archived_users))
        .route("/:community/work/user/:user_id", get(get_archived_user_posts))
}

async fn add_work(
    State(app_state): State<AppState>,
    Path((community, post_id)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let page = params.get("page").and_then(|p| p.parse::<i32>().ok());

    let db = &mut app_state.pool.get()?;

    let result = match community.as_str() {
        "twitter" => {
            let cache_lock = app_state.twitter_cache.clone();
            let cache = &cache_lock.read().await;
            TwitterPost::get(db, cache, &post_id)?.map(|p| p.add_to_library(db, page))
        }
        "pixiv" => {
            let cache_lock = app_state.pixiv_cache.clone();
            let cache = &cache_lock.read().await;
            PixivPost::get(db, cache, &post_id)?.map(|p| p.add_to_library(db, page))
        }
        "yandere" => {
            let cache_lock = app_state.yandere_cache.clone();
            let cache = &cache_lock.read().await;
            YanderePost::get(db, cache, &post_id)?.map(|p| p.add_to_library(db, page))
        }
        "panda" => {
            let cache_lock = app_state.panda_cache.clone();
            let cache = &cache_lock.read().await;
            PandaPost::get(db, cache, &post_id)?.map(|p| p.add_to_library(db, page))
        }
        _ => None,
    }
    .ok_or(bottle_core::Error::ObjectNotFound(format!(
        "Post {} at Community {}",
        post_id, community
    )))??;

    Ok(Json(result))
}

async fn delete_work(State(app_state): State<AppState>, Path(work_id): Path<i32>) -> Result<()> {
    let conn = &mut app_state.pool.get()?;
    bottle_library::delete_work(conn, work_id)?;
    Ok(())
}

async fn get_archived_users(
    State(app_state): State<AppState>,
    Path(community): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);
    let recent_count = params
        .get("recent_count")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RECENT_COUNT);

    let db = &mut app_state.pool.get()?;
    let result = match community.as_str() {
        "twitter" => TwitterFeed::archived_posts_grouped_by_user(db, page, page_size, recent_count),
        "pixiv" => PixivFeed::archived_posts_grouped_by_user(db, page, page_size, recent_count),
        "yandere" => YandereFeed::archived_posts_grouped_by_user(db, page, page_size, recent_count),
        "panda" => PandaFeed::archived_posts_grouped_by_user(db, page, page_size, recent_count),
        _ => Err(bottle_core::Error::InvalidEndpoint(format!("Community {}", community))),
    }?;

    Ok(Json(result))
}

async fn get_archived_user_posts(
    State(app_state): State<AppState>,
    Path((community, user_id)): Path<(String, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);

    let db = &mut app_state.pool.get()?;
    let result = match community.as_str() {
        "twitter" => TwitterFeed::archived_posts_by_user(db, user_id, page, page_size),
        "pixiv" => PixivFeed::archived_posts_by_user(db, user_id, page, page_size),
        "yandere" => YandereFeed::archived_posts_by_user(db, user_id, page, page_size),
        "panda" => PandaFeed::archived_posts_by_user(db, user_id, page, page_size),
        _ => Err(bottle_core::Error::InvalidEndpoint(format!("Community {}", community))),
    }?;

    Ok(Json(result))
}

async fn get_archived_posts(
    State(app_state): State<AppState>,
    Path(community): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);

    let db = &mut app_state.pool.get()?;
    let result = match community.as_str() {
        "twitter" => TwitterFeed::archived_posts(db, page, page_size),
        "pixiv" => PixivFeed::archived_posts(db, page, page_size),
        "yandere" => YandereFeed::archived_posts(db, page, page_size),
        "panda" => PandaFeed::archived_posts(db, page, page_size),
        _ => Err(bottle_core::Error::InvalidEndpoint(format!("Community {}", community))),
    }?;

    Ok(Json(result))
}
