use axum::{
    extract::{Path, State},
    response::Json,
    routing::get,
    Router,
};

use crate::{
    background_job::*,
    error::Result,
    state::AppState,
    util::{FeedIdentifier, FeedWrapper},
};

pub fn job_router() -> Router<AppState> {
    Router::new()
        .route("/jobs", get(get_jobs))
        .route("/:community/feed/:id/update", get(handle_update_feed))
        .route("/:community/feeds/update", get(handle_update_all_feed))
        .route("/images/download", get(handle_download_image))
        .route("/panda/galleries/download", get(handle_download_all_panda_gallery))
        .route("/panda/gallery/:id/download", get(handle_download_panda_gallery))
}

async fn handle_update_feed(
    State(app_state): State<AppState>,
    Path((community, id)): Path<(String, i32)>,
) -> Result<()> {
    let db = &mut app_state.pool.get()?;
    let id = FeedIdentifier::new(&community, id);
    // Check if the feed exists
    let _feed = FeedWrapper::from_id(db, &id)?;

    let did_send = send_feed_update(&app_state, id.clone()).await?;
    if !did_send {
        tracing::warn!("Feed {} update job is already running", id);
        return Err(anyhow::anyhow!("Feed {} update job is already running", id))?;
    }

    Ok(())
}

async fn handle_update_all_feed(State(app_state): State<AppState>, Path(community): Path<String>) -> Result<()> {
    let db = &mut app_state.pool.get()?;
    let feeds = FeedWrapper::all(db, &community)?;

    for feed in feeds.iter() {
        let did_send = send_feed_update(&app_state, feed.id()).await?;
        if !did_send {
            tracing::warn!("Feed {} update job is already running", feed.id());
        }
    }

    Ok(())
}

async fn handle_download_image(State(app_state): State<AppState>) -> Result<()> {
    send_image_download(&app_state).await
}

async fn handle_download_panda_gallery(State(app_state): State<AppState>, Path(id): Path<i64>) -> Result<()> {
    let db = &mut app_state.pool.get()?;
    let tasks = bottle_panda::download::get_download_task(db, id)?;

    let did_send = send_panda_download(&app_state, tasks).await?;
    if !did_send {
        tracing::warn!("Panda gallery {} download job is already running", id);
        return Err(anyhow::anyhow!("Panda gallery {} download job is already running", id))?;
    }

    Ok(())
}

async fn handle_download_all_panda_gallery(State(app_state): State<AppState>) -> Result<()> {
    let db = &mut app_state.pool.get()?;
    let tasks = bottle_panda::download::get_all_download_tasks(db)?;

    if tasks.is_empty() {
        tracing::info!("Panda download job done. No gallery to download");
    }

    for task in tasks {
        send_panda_download(&app_state, task).await?;
    }

    Ok(())
}

async fn get_jobs(State(app_state): State<AppState>) -> Json<JobsStateResponse> {
    let feed_update_state_map = app_state.feed_update_state_map.read().await.clone();

    let mut feed_update_jobs = Vec::new();
    for (id, rx) in feed_update_state_map.iter() {
        let state = rx.borrow().clone();
        feed_update_jobs.push(FeedUpdateJobStateResponse::new(id, &state));
    }

    let image_download_job = ImageDownloadJobStateResponse::from(&*app_state.image_download_job_state.borrow());

    let mut panda_download_jobs = Vec::new();
    let panda_state_map = app_state.panda_download_state_map.read().await.clone();
    let panda_title_map = app_state.panda_gallery_title_map.read().await.clone();
    for (id, rx) in panda_state_map.iter() {
        if let Some(title) = panda_title_map.get(id) {
            let state = rx.borrow().clone();
            panda_download_jobs.push(PandaDownloadJobStateResponse::new(id, title.clone(), &state));
        }
    }

    Json(JobsStateResponse {
        feed_update_jobs,
        image_download_job,
        panda_download_jobs,
    })
}
