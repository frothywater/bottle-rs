use bottle_core::{feed::*, Database, Result};
use yandere_client::TagType;

use crate::{
    cache::YandereCache,
    feed::{YandereFeed, YandereFeedParams, YandereFetchContext},
    util,
};

// MARK: Methods for temporary feeds

/// Fetch posts from temporary feed.
pub async fn fetch_posts<'a>(
    db: Database<'a>,
    cache: &'a mut YandereCache,
    request: &EndpointRequest<YandereFeedParams>,
) -> Result<EndpointResponse> {
    // 1. Fetch posts
    let feed = YandereFeed {
        id: -1, // Temporary feed
        name: None,
        first_fetch_limit: None,
        watching: false,
        params: request.params.clone(),
        reached_end: false,
    };
    let page = request
        .offset
        .as_ref()
        .map(|o| o.parse::<u32>())
        .transpose()?
        .unwrap_or(1);
    let mut ctx = YandereFetchContext { page };
    let result = feed.fetch(&mut ctx, None).await?;

    // 1.1. Store posts, tags and pools in cache
    cache.posts.extend(result.posts.iter().map(|p| (p.id, p.clone())));
    cache.tags.extend(result.tags.clone());
    cache
        .pools
        .extend(result.pools.iter().map(|pool| (pool.id, pool.clone())));
    for item in result.pool_posts.iter() {
        cache
            .post_pools
            .entry(item.post_id)
            .or_default()
            .push(item.clone());
    }
    tracing::info!(
        "Stored {} yandere posts, {} tags, {} pools, {} pool-posts to cache",
        result.posts.len(),
        result.tags.len(),
        result.pools.len(),
        result.pool_posts.len()
    );

    // 2. Prepare views
    let posts = result.posts.iter().map(util::post_view).collect();
    let media = result.posts.iter().map(util::media_view).collect();

    let users = result
        .tags
        .iter()
        .filter(|(_, type_)| matches!(type_, TagType::Artist))
        .map(|(tag, _)| util::artist_view(tag))
        .collect();

    // 3. Get associated works and images
    let post_ids = result.posts.iter().map(|post| post.id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "yandere", post_ids, false)?;

    Ok(EndpointResponse {
        posts,
        media,
        users,
        works,
        images,
        next_offset: Some(ctx.page.to_string()),
        reached_end: result.posts.is_empty(),
        total_items: None,
    })
}
