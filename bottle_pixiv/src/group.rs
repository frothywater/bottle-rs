use std::collections::HashMap;

use diesel::prelude::*;
use diesel::query_builder::{BoxedSqlQuery, QueryFragment};
use diesel::sql_types::BigInt;
use diesel::sqlite::Sqlite;

use bottle_core::{
    feed::{GeneralResponse, MediaView, UserView},
    library::WorkView,
    Database, Result,
};

use crate::model;
use crate::util;

// MARK: Internal methods for grouping posts by artist

/// Filter media by choosing only media with page indices corresponding to given works.
pub(crate) fn filter_media_by_works(media: &[model::PixivMedia], works: &[WorkView]) -> Vec<model::PixivMedia> {
    let post_page_set = bottle_util::group::create_post_page_set(works, |post_id| post_id.parse::<i64>().ok());
    media
        .iter()
        .filter(|m| {
            post_page_set.contains(&(m.illust_id, Some(m.page))) || post_page_set.contains(&(m.illust_id, None))
        })
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
    use bottle_core::schema::{pixiv_illust, pixiv_media, pixiv_user};

    // 1. Execute grouped query
    let (records, user_count, user_post_counts) = 
        bottle_util::group::execute_grouped_query(db, query, page, page_size, recent_count)?;

    let user_ids = records.iter().map(|r| r.user_id);
    let post_ids = records.iter().map(|r| r.post_id);

    // 2. Fetch associated users
    let users = pixiv_user::table
        .filter(pixiv_user::id.eq_any(user_ids))
        .load::<model::PixivUser>(db)?;
    // Add post_count field to users
    let mut users: Vec<UserView> = users.into_iter().map(UserView::from).collect();
    for user in &mut users {
        user.post_count = user_post_counts.get(&user.user_id).cloned();
    }
    // Sort users by post_count
    users.sort_by(|a, b| b.post_count.cmp(&a.post_count));

    // 3. Fetch associated posts
    let posts = pixiv_illust::table
        .filter(pixiv_illust::id.eq_any(post_ids.clone()))
        .load::<model::PixivIllust>(db)?;
    // Reorder posts by original order
    let posts_map = posts.into_iter().map(|post| (post.id, post)).collect::<HashMap<_, _>>();
    let posts = post_ids
        .clone()
        .filter_map(|id| posts_map.get(&id).cloned())
        .collect::<Vec<_>>();

    let tags = util::get_tag_map(db, post_ids.clone())?;
    let posts = posts
        .into_iter()
        .map(|illust| illust.post_view(tags.get(&illust.id).cloned().unwrap_or_default()))
        .collect();

    // 4. Fetch associated media
    let mut media = pixiv_media::table
        .inner_join(pixiv_illust::table)
        .filter(pixiv_media::illust_id.eq_any(post_ids.clone()))
        .order(pixiv_media::page.asc())
        .select(pixiv_media::all_columns)
        .load::<model::PixivMedia>(db)?;

    // 5. Fetch associated works
    let post_ids = post_ids.map(|id| id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "pixiv", post_ids, false)?;

    if filter_by_works {
        media = filter_media_by_works(&media, &works);
    }

    Ok(GeneralResponse {
        posts: Some(posts),
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
    results: (Vec<model::PixivIllust>, i64),
    user_id: i64,
    page: i64,
    page_size: i64,
    filter_by_works: bool,
) -> Result<GeneralResponse> {
    use bottle_core::schema::{pixiv_media, pixiv_user};

    // 1. Fetch user
    let user = pixiv_user::table
        .filter(pixiv_user::id.eq(user_id))
        .first::<model::PixivUser>(db)?;

    let (posts, total_items) = results;

    // 2. Fetch associated media
    let post_ids = posts.iter().map(|t| t.id).collect::<Vec<_>>();
    let mut media = pixiv_media::table
        .filter(pixiv_media::illust_id.eq_any(post_ids.iter()))
        .order(pixiv_media::page.asc())
        .load::<model::PixivMedia>(db)?;

    let tags = util::get_tag_map(db, post_ids.clone())?;
    let posts = posts
        .into_iter()
        .map(|illust| illust.post_view(tags.get(&illust.id).cloned().unwrap_or_default()))
        .collect();

    // 3. Fetch associated works
    let post_ids = post_ids.iter().map(|id| id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "pixiv", post_ids, false)?;

    if filter_by_works {
        media = filter_media_by_works(&media, &works);
    }

    Ok(GeneralResponse {
        posts: Some(posts),
        users: Some(vec![user.into()]),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        works: Some(works),
        images: Some(images),
        total_items,
        page,
        page_size,
    })
}
