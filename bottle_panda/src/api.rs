use std::collections::hash_map::Entry;
use std::collections::HashSet;

use diesel::prelude::*;

use bottle_core::{feed::*, Error, Result};
use panda_client::{GalleryListOffset, PandaClient, TagNamespace};

use crate::cache::PandaCache;
use crate::community::PandaAccount;
use crate::feed::{Direction, PandaFeed, PandaFeedParams, PandaFetchContext};
use crate::model;
use crate::util;

// MARK: Methods for temporary feed

/// Create a temporary feed from given request using default account.
fn from_request(db: Database, request: &EndpointRequest<PandaFeedParams>) -> Result<PandaFeed> {
    let account = PandaAccount::default(db)?;
    Ok(PandaFeed {
        id: -1, // Temporary feed
        name: None,
        first_fetch_limit: None,
        watching: false,
        account_id: account.id,
        params: request.params.clone(),
        reached_end: false,
    })
}

fn default_client(db: Database) -> Result<PandaClient> {
    let account = PandaAccount::default(db)?;
    let auth = account
        .auth(db)?
        .ok_or(Error::NotLoggedIn("Invalid account".to_string()))?;
    let client = PandaClient::new(auth).map_err(anyhow::Error::from)?;
    Ok(client)
}

/// Fetch posts from temporary feed.
pub async fn fetch_posts<'a>(
    db: Database<'a>,
    cache: &'a mut PandaCache,
    request: &EndpointRequest<PandaFeedParams>,
) -> Result<EndpointResponse> {
    use bottle_core::schema::panda_media;

    let feed = from_request(db, request)?;

    // 1. Fetch results
    let auth = feed.get_account(db)?.auth(db)?;
    let mut ctx = PandaFetchContext {
        offset: request
            .offset
            .as_ref()
            .map(|offset| GalleryListOffset::OlderThan(offset.clone())),
        direction: Direction::Backward,
    };
    let result = feed.fetch(&mut ctx, auth.as_ref()).await?;

    // 2. Store results in cache
    cache
        .galleries
        .extend(result.galleries.iter().map(|g| (g.gid, g.clone())));
    tracing::info!("Stored {} panda galleries to cache", result.galleries.len());

    // 3. Get associated media from database
    let post_ids = result.galleries.iter().map(|g| g.gid as i64);
    let media = panda_media::table
        .filter(panda_media::gallery_id.eq_any(post_ids))
        .order(panda_media::media_index.asc())
        .load::<model::PandaMedia>(db)?;

    // 4. Get associated works and images from database
    let post_ids = result.galleries.iter().map(|g| g.gid.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "panda", post_ids, false)?;

    // 5. Prepare views
    let posts = result.galleries.iter().map(util::post_view).collect();
    let media = media.into_iter().map(MediaView::from).collect();

    let users = result
        .galleries
        .iter()
        .flat_map(|g| g.tags.iter())
        .filter(|tag| matches!(tag.namespace, TagNamespace::Artist))
        .map(|tag| util::artist_view(&tag.name))
        .collect();

    let offset = match result.next_page_offset {
        Some(GalleryListOffset::OlderThan(offset)) => Some(offset),
        _ => None,
    };
    Ok(EndpointResponse {
        posts,
        media,
        users,
        works,
        images,
        reached_end: result.galleries.is_empty(),
        next_offset: offset,
        total_items: result.total_count.map(|c| c as i64),
    })
}

/// Fetch a gallery preview page, and store the result in cache.
pub async fn fetch_media_page<'a>(
    db: Database<'a>,
    cache: &'a mut PandaCache,
    gid: u64,
    page: u32,
) -> Result<EndpointResponse> {
    // 1. Fetch gallery token from cache or database
    let token = if cache.galleries.contains_key(&gid) {
        cache.galleries[&gid].token.clone()
    } else {
        use bottle_core::schema::panda_gallery;
        panda_gallery::table
            .find(gid as i64)
            .select(panda_gallery::token)
            .first::<String>(db)
            .optional()?
            .ok_or(Error::ObjectNotFound(format!("Gallery {} not found", gid)))?
    };

    // 2. Fetch gallery page
    let client = default_client(db)?;
    let result = client.gallery(gid, &token, page).await.map_err(anyhow::Error::from)?;

    // 3. Store gallery detail to cache
    if let Entry::Vacant(e) = cache.galleries.entry(gid) {
        e.insert(result.gallery.clone());
        tracing::info!("Stored panda gallery {} to cache", gid);
    }
    if let Entry::Vacant(e) = cache.gallery_details.entry(gid) {
        e.insert(result.detail.clone());
        tracing::info!("Stored panda gallery detail {} to cache", gid);
    }
    if let Entry::Vacant(e) = cache.image_previews.entry(gid) {
        e.insert(result.previews.clone());
        tracing::info!(
            "Stored {} new panda gallery {} previews to cache",
            result.previews.len(),
            gid
        );
    } else {
        // Only store new previews
        let existing_indices = cache.image_previews[&gid]
            .iter()
            .map(|p| p.index)
            .collect::<HashSet<_>>();
        let new_previews = result
            .previews
            .iter()
            .filter(|p| !existing_indices.contains(&p.index))
            .cloned()
            .collect::<Vec<_>>();
        let inserted_count = new_previews.len();
        cache.image_previews.get_mut(&gid).unwrap().extend(new_previews);
        tracing::info!("Stored {} new panda gallery {} previews to cache", inserted_count, gid);
    }

    // 4. Prepare post and media
    let mut post = util::post_view(&result.gallery);
    let extra = util::gallery_extra(&result.gallery);
    post.extra = Some(extra.with_detail(&result.detail).into());
    let media = result
        .previews
        .iter()
        .map(|preview| MediaView {
            media_id: format!("{}-{}", gid, preview.index),
            community: "panda".to_string(),
            post_id: gid.to_string(),
            page_index: preview.index as i32,
            thumbnail_url: Some(preview.thumbnail_url.clone()),
            extra: Some(serde_json::json!({"panda": { "token": preview.token }})),
            ..Default::default()
        })
        .collect();

    Ok(EndpointResponse {
        posts: vec![post],
        media,
        total_items: Some(result.preview_page_count as i64),
        ..Default::default()
    })
}

/// Fetch a gallery image, and store the result in cache.
pub async fn fetch_media<'a>(
    db: Database<'a>,
    cache: &'a mut PandaCache,
    gid: u64,
    page: u32,
) -> Result<EndpointResponse> {
    // 1. Fetch media token from cache and database
    let token = (|| -> Result<String> {
        if cache.image_previews.contains_key(&gid) {
            let previews = cache.image_previews.get(&gid).unwrap();
            let preview = previews.iter().find(|p| p.index == page);
            if let Some(preview) = preview {
                return Ok(preview.token.clone());
            }
        } else {
            use bottle_core::schema::panda_media;
            let token = panda_media::table
                .find((gid as i64, page as i32))
                .select(panda_media::token)
                .first::<String>(db)
                .optional()?;
            if let Some(token) = token {
                return Ok(token);
            }
        }
        Err(Error::ObjectNotFound(format!("Media {}-{} not found", gid, page)))
    })()?;

    // 2. Fetch page
    let client = default_client(db)?;
    let result = client.image(gid, &token, page).await.map_err(anyhow::Error::from)?;

    // 3. Store image to cache
    if let Entry::Vacant(e) = cache.images.entry(gid) {
        e.insert(vec![result.clone()]);
        tracing::info!("Stored panda media {}-{} to cache", gid, page);
    } else {
        // Only store new images
        let existing_indices = cache.images[&gid].iter().map(|i| i.index).collect::<HashSet<_>>();
        if !existing_indices.contains(&result.index) {
            cache.images.get_mut(&gid).unwrap().push(result.clone());
            tracing::info!("Stored panda media {}-{} to cache", gid, page);
        }
    }

    // 4. Prepare media object
    let media = MediaView {
        media_id: format!("{}-{}", gid, result.index),
        community: "panda".to_string(),
        post_id: gid.to_string(),
        page_index: result.index as i32,
        url: Some(result.url),
        width: Some(result.width as i32),
        height: Some(result.height as i32),
        thumbnail_url: None,
        ..Default::default()
    };

    Ok(EndpointResponse {
        media: vec![media],
        ..Default::default()
    })
}
