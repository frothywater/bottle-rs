use std::collections::{HashMap, HashSet};

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

/// Sqlite row for recent posts query grouped by artist.
#[derive(QueryableByName)]
struct RecentRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = BigInt)]
    post_id: i64,
    #[diesel(sql_type = BigInt)]
    post_count: i64,
    #[diesel(sql_type = BigInt)]
    user_count: i64,
}

/// Generate query for artist-grouped recent post, with given source post query.
/// Binds are `page_size`, `offset` and `recent_count`.
pub(crate) fn grouped_by_user_query(post_query: &str, window_order_clause: &str) -> String {
    format!(
        "with posts as materialized (
                {}
            ), users as materialized (
                select *, count() over () as user_count from (
                    select user_id, count() as post_count
                    from posts
                    group by user_id
                    order by post_count desc
                ) limit ? offset ?
            ), recent as materialized (
                select user_id, id as post_id, rank () over (
                    partition by user_id
                    {}
                ) as rank
                from posts
            )
            select users.user_id, post_id, post_count, user_count from users
            join recent on recent.user_id = users.user_id
            where rank <= ?
            order by post_count desc, users.user_id, rank;",
        post_query, window_order_clause
    )
}

/// Filter media by choosing only media with page indices corresponding to given works.
pub(crate) fn filter_media_by_works(media: &[model::PixivMedia], works: &[WorkView]) -> Vec<model::PixivMedia> {
    let post_page_set = works
        .iter()
        .filter_map(|work| {
            let post_id = work.post_id.as_ref()?.parse::<i64>().ok()?;
            Some((post_id, work.page_index))
        })
        .collect::<HashSet<_>>();
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

    // 1. Fetch row records from database
    let query = query
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(page * page_size)
        .bind::<BigInt, _>(recent_count);
    let records = query.load::<RecentRow>(db)?;

    let user_ids = records.iter().map(|r| r.user_id);
    let post_ids = records.iter().map(|r| r.post_id);
    let user_count = records.first().map(|r| r.user_count).unwrap_or(0);
    let user_post_counts = records
        .iter()
        .map(|r| (r.user_id.to_string(), r.post_count))
        .collect::<HashMap<_, _>>();

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
