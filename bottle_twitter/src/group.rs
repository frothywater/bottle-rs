use diesel::{
    prelude::*,
    query_builder::{BoxedSqlQuery, QueryFragment},
    sql_types::BigInt,
    sqlite::Sqlite,
};

use std::collections::{HashMap, HashSet};

use bottle_core::{
    feed::{GeneralResponse, MediaView, PostView, UserView},
    library::WorkView,
    Database, Result,
};

use crate::model;

// MARK: Internal methods for artist grouping

/// Filter media by choosing only media with page indices corresponding to given works.
pub(crate) fn filter_media_by_works(media: &[model::TwitterMedia], works: &[WorkView]) -> Vec<model::TwitterMedia> {
    let post_page_set = bottle_util::group::create_post_page_set(works, |post_id| post_id.parse::<i64>().ok());
    media
        .iter()
        .filter(|m| post_page_set.contains(&(m.tweet_id, Some(m.page))) || post_page_set.contains(&(m.tweet_id, None)))
        .cloned()
        .collect()
}

/// Fetch recent posts grouped by artist with given source post query.
pub(crate) fn posts_grouped_by_user<Q: QueryFragment<Sqlite>>(
    db: Database,
    query: BoxedSqlQuery<'static, Sqlite, Q>,
    page: i64,
    page_size: i64,
    recent_count: i64,
    filter_by_works: bool,
) -> Result<GeneralResponse> {
    use bottle_core::schema::{tweet, twitter_media, twitter_user};

    // 1. Execute grouped query
    let (records, user_count, user_to_post_count) = 
        bottle_util::group::execute_grouped_query(db, query, page, page_size, recent_count)?;

    let user_ids = records.iter().map(|r| r.user_id);
    let post_ids = records.iter().map(|r| r.post_id);

    // 2. Fetch associated users
    let users = twitter_user::table
        .filter(twitter_user::id.eq_any(user_ids))
        .load::<model::TwitterUser>(db)?;
    // Add post_count field to users
    let mut users = users.into_iter().map(UserView::from).collect::<Vec<_>>();
    for user in &mut users {
        user.post_count = user_to_post_count.get(&user.user_id).cloned();
    }
    // Sort users by post_count
    users.sort_by(|a, b| b.post_count.cmp(&a.post_count));

    // 3. Fetch associated posts
    let posts = tweet::table
        .filter(tweet::id.eq_any(post_ids.clone()))
        .load::<model::Tweet>(db)?;
    // Reorder posts by original order
    let posts_map = posts.into_iter().map(|post| (post.id, post)).collect::<HashMap<_, _>>();
    let posts = post_ids
        .clone()
        .filter_map(|id| posts_map.get(&id).cloned())
        .collect::<Vec<_>>();

    // 4. Fetch associated media
    let mut media = twitter_media::table
        .filter(twitter_media::tweet_id.eq_any(post_ids.clone()))
        .order(twitter_media::page.asc())
        .select(twitter_media::all_columns)
        .load::<model::TwitterMedia>(db)?;

    // 5. Fetch associated works
    let tweet_ids = post_ids.map(|id| id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "twitter", tweet_ids, false)?;

    if filter_by_works {
        media = filter_media_by_works(&media, &works);
    }

    Ok(GeneralResponse {
        posts: Some(posts.into_iter().map(PostView::from).collect()),
        users: Some(users),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        works: Some(works),
        images: Some(images),
        total_items: user_count,
        page,
        page_size,
    })
}

/// Fetch artist with given post results.
pub(crate) fn posts_by_user(
    db: Database,
    results: (Vec<model::Tweet>, i64),
    user_id: i64,
    page: i64,
    page_size: i64,
    filter_by_works: bool,
) -> Result<GeneralResponse> {
    use bottle_core::schema::{twitter_media, twitter_user};

    // 1. Fetch user
    let user = twitter_user::table
        .filter(twitter_user::id.eq(user_id))
        .first::<model::TwitterUser>(db)?;

    let (tweets, total_items) = results;

    // 2. Fetch associated media
    let tweet_ids = tweets.iter().map(|t| t.id);
    let mut media = twitter_media::table
        .filter(twitter_media::tweet_id.eq_any(tweet_ids))
        .order(twitter_media::page.asc())
        .load::<model::TwitterMedia>(db)?;

    // 3. Fetch associated works
    let tweet_ids = tweets.iter().map(|t| t.id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "twitter", tweet_ids, false)?;

    if filter_by_works {
        media = filter_media_by_works(&media, &works);
    }

    Ok(GeneralResponse {
        posts: Some(tweets.into_iter().map(PostView::from).collect()),
        users: Some(vec![UserView::from(user)]),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        works: Some(works),
        images: Some(images),
        total_items,
        page,
        page_size,
    })
}
