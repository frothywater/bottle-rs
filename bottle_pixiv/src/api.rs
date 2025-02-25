use bottle_core::{feed::*, Database, Result};

use crate::cache::PixivCache;
use crate::community::PixivAccount;
use crate::feed::{PixivFeed, PixivFeedParams, PixivFetchContext};
use crate::util;

// MARK: Methods for temporary feeds

/// Create a temporary feed from given request using default account.
fn from_request(db: Database, request: &EndpointRequest<PixivFeedParams>) -> Result<PixivFeed> {
    let account = PixivAccount::default(db)?;
    Ok(PixivFeed {
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
    cache: &'a mut PixivCache,
    request: &EndpointRequest<PixivFeedParams>,
) -> Result<EndpointResponse> {
    use itertools::Itertools;
    use pixiv_client::Paginated;

    // 1. Get feed and account
    let feed = from_request(db, request)?;
    let account = feed.get_account(db)?;

    // 2. Refresh account if expired
    if account.expired() {
        tracing::info!("Account expired, refreshing");
        let credential = account.credential(db)?;
        let info = PixivAccount::fetch(&credential).await?;
        let account = account.update(db, &info)?;
        tracing::info!("{:?}", account);
    }
    let auth = account.auth(db)?;

    // 3. Fetch posts
    let offset = request.offset.as_ref().map(|o| o.parse::<i64>()).transpose()?;
    let mut ctx = PixivFetchContext {
        offset,
        total_fetched: 0,
    };
    let result = feed.fetch(&mut ctx, auth.as_ref()).await?;

    // 3.1. Store results in cache
    cache
        .illusts
        .extend(result.illusts.iter().map(|illust| (illust.id, illust.clone())));
    tracing::info!("Stored {} pixiv illusts to cache", result.illusts.len());

    // 4. Prepare views
    let posts = result.illusts.iter().map(util::post_view).collect();
    let users = result
        .illusts
        .iter()
        .map(|illust| &illust.user)
        .unique_by(|user| user.id)
        .map(util::user_view)
        .collect();
    let media = result
        .illusts
        .iter()
        .flat_map(util::media)
        .map(MediaView::from)
        .collect();

    // 5. Get associated works and images
    let post_ids = result
        .illusts
        .iter()
        .map(|illust| illust.id.to_string())
        .collect::<Vec<_>>();
    let (works, images) = bottle_library::get_works_by_post_ids(db, "pixiv", post_ids, false)?;

    Ok(EndpointResponse {
        posts,
        users,
        media,
        works,
        images,
        next_offset: ctx.offset.map(|o| o.to_string()),
        reached_end: result.next_url().is_none(),
        total_items: None,
    })
}
