use bottle_core::{feed::*, Database, Result};

use crate::cache::TwitterCache;
use crate::community::TwitterAccount;
use crate::feed::{Direction, TwitterFeed, TwitterFeedParams, TwitterFetchContext};
use crate::util;

// MARK: Methods for temporary feed

/// Create a temporary feed from given request using default account.
fn from_request(db: Database, request: &EndpointRequest<TwitterFeedParams>) -> Result<TwitterFeed> {
    let account = TwitterAccount::default(db)?;
    Ok(TwitterFeed {
        id: -1, // Temporary feed
        name: None,
        first_fetch_limit: None,
        watching: false,
        account_id: account.id,
        params: request.params.clone(),
        reached_end: false,
    })
}

/// Fetch posts from temporary feed.
pub async fn fetch_posts<'a>(
    db: Database<'a>,
    cache: &'a mut TwitterCache,
    request: &EndpointRequest<TwitterFeedParams>,
) -> Result<EndpointResponse> {
    use itertools::Itertools;

    // 1. Fetch results
    let feed = from_request(db, request)?;
    let auth = feed.get_account(db)?.auth(db)?;
    let mut ctx = TwitterFetchContext {
        cursor: request.offset.clone(),
        direction: Direction::Backward,
    };
    let result = feed.fetch(&mut ctx, auth.as_ref()).await?;

    // 2. Store tweets to cache
    cache.tweets.extend(result.tweets.iter().map(|t| (t.id, t.clone())));
    tracing::info!("Stored {} tweets to cache", result.tweets.len());

    // 3. Prepare views
    let posts = result.tweets.iter().map(util::post_view).collect();
    let users = result
        .tweets
        .iter()
        .map(|t| &t.user)
        .unique_by(|u| u.id)
        .map(util::user_view)
        .collect();
    let media = result.tweets.iter().flat_map(util::media_views).collect();

    // 4. Get associated works and images
    let post_ids = result.tweets.iter().map(|t| t.id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "twitter", post_ids, false)?;

    Ok(EndpointResponse {
        posts,
        users,
        media,
        works,
        images,
        next_offset: ctx.cursor,
        reached_end: result.tweets.is_empty(),
        total_items: None,
    })
}
