use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};

use std::collections::HashMap;

use bottle_core::feed::{EndpointRequest, EndpointResponse};
use bottle_panda::PandaFeedParams;
use bottle_pixiv::PixivFeedParams;
use bottle_twitter::TwitterFeedParams;
use bottle_yandere::YandereFeedParams;

use crate::{error::Result, state::AppState};

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/twitter/api", post(fetch_twitter_api))
        .route("/pixiv/api", post(fetch_pixiv_api))
        .route("/yandere/api", post(fetch_yandere_api))
        .route("/panda/api", post(fetch_panda_api))
        .route("/panda/api/post/:gid", get(fetch_panda_post))
        .route("/panda/api/post/:gid/media/:page", get(fetch_panda_media))
}

async fn fetch_twitter_api(
    State(app_state): State<AppState>,
    Json(payload): Json<EndpointRequest<TwitterFeedParams>>,
) -> Result<Json<EndpointResponse>> {
    use bottle_twitter::api::fetch_posts;

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.twitter_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_posts(db, cache, &payload).await?;
    Ok(Json(response))
}

async fn fetch_pixiv_api(
    State(app_state): State<AppState>,
    Json(payload): Json<EndpointRequest<PixivFeedParams>>,
) -> Result<Json<EndpointResponse>> {
    use bottle_pixiv::api::fetch_posts;

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.pixiv_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_posts(db, cache, &payload).await?;
    Ok(Json(response))
}

async fn fetch_yandere_api(
    State(app_state): State<AppState>,
    Json(payload): Json<EndpointRequest<YandereFeedParams>>,
) -> Result<Json<EndpointResponse>> {
    use bottle_yandere::api::fetch_posts;

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.yandere_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_posts(db, cache, &payload).await?;
    Ok(Json(response))
}

async fn fetch_panda_api(
    State(app_state): State<AppState>,
    Json(payload): Json<EndpointRequest<PandaFeedParams>>,
) -> Result<Json<EndpointResponse>> {
    use bottle_panda::api::fetch_posts;

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.panda_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_posts(db, cache, &payload).await?;
    Ok(Json(response))
}

async fn fetch_panda_post(
    State(app_state): State<AppState>,
    Path(gid): Path<u64>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<EndpointResponse>> {
    use bottle_panda::api::fetch_media_page;
    let page = params.get("page").and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.panda_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_media_page(db, cache, gid, page).await?;
    Ok(Json(response))
}

async fn fetch_panda_media(
    State(app_state): State<AppState>,
    Path((gid, page)): Path<(u64, u32)>,
) -> Result<Json<EndpointResponse>> {
    use bottle_panda::api::fetch_media;

    let db = &mut app_state.pool.get()?;
    let cache_lock = app_state.panda_cache.clone();
    let cache = &mut cache_lock.write().await;

    let response = fetch_media(db, cache, gid, page).await?;
    Ok(Json(response))
}
