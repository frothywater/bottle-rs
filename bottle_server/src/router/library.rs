use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{delete, get, post},
    Router,
};

use std::collections::HashMap;

use bottle_core::{
    feed::GeneralResponse,
    library::{AlbumView, FolderView},
};
use bottle_library::{Album, Folder};

use crate::{
    error::Result,
    state::AppState,
    util::{self, get_page_and_size},
};

pub fn library_router() -> Router<AppState> {
    Router::new()
        // Album
        .route("/album", post(add_album))
        .route("/albums", get(get_albums))
        .route("/album/:id/rename", post(rename_album))
        .route("/album/:id/reorder", post(reorder_album))
        .route("/album/:id", delete(delete_album))
        .route("/album/:id/works", post(add_album_works))
        .route("/album/:id/works", get(get_album_works))
        .route("/album/:id/works", delete(delete_album_works))
        // Folder
        .route("/folder", post(add_folder))
        .route("/folders", get(get_folders))
        .route("/folder/:id/rename", post(rename_folder))
        .route("/folder/:id/reorder", post(reorder_folder))
        .route("/folder/:id", delete(delete_folder))
}

// MARK: Album

async fn add_album(
    State(app_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlbumView>> {
    let name = params.get("name").ok_or(bottle_core::Error::InvalidEndpoint(
        "Album name is required".to_string(),
    ))?;
    let folder_id = match params.get("folder_id") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };

    let conn = &mut app_state.pool.get()?;
    let album = Album::add(conn, name, folder_id)?;

    Ok(Json(album))
}

async fn get_albums(State(app_state): State<AppState>) -> Result<Json<Vec<AlbumView>>> {
    let conn = &mut app_state.pool.get()?;
    let albums = Album::all(conn)?;

    Ok(Json(albums))
}

async fn rename_album(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlbumView>> {
    let name = params.get("name").ok_or(bottle_core::Error::InvalidEndpoint(
        "Album name is required".to_string(),
    ))?;

    let conn = &mut app_state.pool.get()?;
    let album = Album::rename(conn, id, name)?;

    Ok(Json(album))
}

async fn reorder_album(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<AlbumView>> {
    let folder_id = match params.get("folder_id") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };
    let position = match params.get("position") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };

    let conn = &mut app_state.pool.get()?;
    let album = Album::reorder(conn, id, folder_id, position)?;

    Ok(Json(album))
}

async fn delete_album(State(app_state): State<AppState>, Path(id): Path<i32>) -> Result<()> {
    let conn = &mut app_state.pool.get()?;
    Album::delete(conn, id)?;
    Ok(())
}

async fn add_album_works(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<()> {
    let work_ids = params
        .get("work_ids")
        .ok_or(bottle_core::Error::InvalidEndpoint("Work IDs are required".to_string()))?
        .split(',')
        .map(|s| s.parse::<i32>())
        .collect::<std::result::Result<Vec<_>, std::num::ParseIntError>>()?;

    let conn = &mut app_state.pool.get()?;
    Album::add_works(conn, id, work_ids.into_iter())?;

    Ok(())
}

async fn get_album_works(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GeneralResponse>> {
    let (page, page_size) = get_page_and_size(&params);

    let conn = &mut app_state.pool.get()?;
    let response = Album::works(conn, id, page, page_size)?;

    // Add community entities to the response
    let response = util::adding_community_entities(conn, response)?;

    Ok(Json(response))
}

async fn delete_album_works(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<()> {
    let work_ids = params
        .get("work_ids")
        .ok_or(bottle_core::Error::InvalidEndpoint("Work IDs are required".to_string()))?
        .split(',')
        .map(|s| s.parse::<i32>())
        .collect::<std::result::Result<Vec<_>, std::num::ParseIntError>>()?;

    let conn = &mut app_state.pool.get()?;
    Album::remove_works(conn, id, work_ids.into_iter())?;

    Ok(())
}

// MARK: Folder

async fn add_folder(
    State(app_state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<FolderView>> {
    let name = params.get("name").ok_or(bottle_core::Error::InvalidEndpoint(
        "Folder name is required".to_string(),
    ))?;
    let parent_id = match params.get("parent_id") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };

    let conn = &mut app_state.pool.get()?;
    let folder = Folder::add(conn, name, parent_id)?;

    Ok(Json(folder))
}

async fn get_folders(State(app_state): State<AppState>) -> Result<Json<Vec<FolderView>>> {
    let conn = &mut app_state.pool.get()?;
    let folders = Folder::all(conn)?;

    Ok(Json(folders))
}

async fn rename_folder(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<FolderView>> {
    let name = params.get("name").ok_or(bottle_core::Error::InvalidEndpoint(
        "Folder name is required".to_string(),
    ))?;

    let conn = &mut app_state.pool.get()?;
    let folder = Folder::rename(conn, id, name)?;

    Ok(Json(folder))
}

async fn reorder_folder(
    State(app_state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<FolderView>> {
    let parent_id = match params.get("parent_id") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };
    let position = match params.get("position") {
        Some(id) => Some(id.parse::<i32>()?),
        None => None,
    };

    let conn = &mut app_state.pool.get()?;
    let folder = Folder::reorder(conn, id, parent_id, position)?;

    Ok(Json(folder))
}

async fn delete_folder(State(app_state): State<AppState>, Path(id): Path<i32>) -> Result<()> {
    let conn = &mut app_state.pool.get()?;
    Folder::delete(conn, id)?;
    Ok(())
}
