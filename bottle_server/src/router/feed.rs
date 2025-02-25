use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{delete, get, post},
    Router,
};
use serde_json::{json, Value};

use std::collections::HashMap;

use bottle_core::feed::*;
use bottle_panda::PandaCommunity;
use bottle_pixiv::PixivCommunity;
use bottle_twitter::TwitterCommunity;
use bottle_yandere::YandereCommunity;

use crate::{
    error::Result,
    payload::NewFeedRequest,
    state::AppState,
    util::{get_page_and_size, FeedIdentifier, FeedWrapper, DEFAULT_RECENT_COUNT},
};

pub fn feed_router() -> Router<AppState> {
    Router::new()
        .route("/metadata", get(metadata))
        .route("/feed", post(add_feed))
        .route("/:community/feeds", get(get_feeds))
        .route("/:community/feed/:id", get(get_feed))
        .route("/:community/feed/:id", delete(delete_feed))
        .route("/:community/feed/:id", post(modify_feed))
        .route("/:community/feed/:id/posts", get(get_feed_posts))
        .route("/:community/feed/:id/users", get(get_feed_users))
        .route("/:community/feed/:id/user/:user_id", get(get_feed_user_posts))
}

async fn metadata() -> Json<Value> {
    Json(json!({
        "communities": [
            TwitterCommunity::metadata(),
            PixivCommunity::metadata(),
            YandereCommunity::metadata(),
            PandaCommunity::metadata(),
        ]
    }))
}

async fn get_feeds(State(app_state): State<AppState>, Path(community): Path<String>) -> Result<Json<Vec<FeedView>>> {
    let db = &mut app_state.pool.get()?;

    let feeds = FeedWrapper::all(db, &community)?
        .into_iter()
        .map(|f| f.view())
        .collect();

    Ok(Json(feeds))
}

async fn get_feed(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
) -> Result<Json<FeedView>> {
    let db = &mut app_state.pool.get()?;

    let feed_id = FeedIdentifier::new(&community, id);
    let feed = FeedWrapper::from_id(db, &feed_id)?.view();

    Ok(Json(feed))
}

async fn add_feed(State(app_state): State<AppState>, Json(request): Json<NewFeedRequest>) -> Result<Json<FeedView>> {
    let db = &mut app_state.pool.get()?;
    let feed = FeedWrapper::add(db, &request)?.view();

    Ok(Json(feed))
}

async fn delete_feed(State(app_state): State<AppState>, Path((community, id)): Path<(String, i32)>) -> Result<()> {
    let db = &mut app_state.pool.get()?;
    let feed_id = FeedIdentifier::new(&community, id);
    FeedWrapper::delete(db, &feed_id)?;

    Ok(())
}

async fn modify_feed(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
    Json(info): Json<FeedInfo>,
) -> Result<Json<FeedView>> {
    let db = &mut app_state.pool.get()?;

    let feed_id = FeedIdentifier::new(&community, id);
    let feed = FeedWrapper::from_id(db, &feed_id)?.modify(db, &info)?;

    Ok(Json(feed))
}

async fn get_feed_posts(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);

    let db = &mut app_state.pool.get()?;
    let feed_id = FeedIdentifier::new(&community, id);
    let result = FeedWrapper::from_id(db, &feed_id)?.posts(db, page, page_size)?;

    Ok(Json(result))
}

async fn get_feed_users(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);
    let recent_count = params
        .get("recent_count")
        .and_then(|p| p.parse::<i64>().ok())
        .unwrap_or(DEFAULT_RECENT_COUNT);

    let db = &mut app_state.pool.get()?;
    let feed_id = FeedIdentifier::new(&community, id);
    let result = FeedWrapper::from_id(db, &feed_id)?.users(db, page, page_size, recent_count)?;

    Ok(Json(result))
}

async fn get_feed_user_posts(
    State(app_state): State<AppState>,
    Path((community, feed_id, user_id)): Path<(String, i32, String)>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);

    let db = &mut app_state.pool.get()?;
    let feed_id = FeedIdentifier::new(&community, feed_id);
    let result = FeedWrapper::from_id(db, &feed_id)?.user_posts(db, user_id, page, page_size)?;

    Ok(Json(result))
}
