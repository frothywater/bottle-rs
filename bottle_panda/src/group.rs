use diesel::prelude::*;
use diesel::query_builder::{BoxedSqlQuery, QueryFragment};
use diesel::sql_types::{BigInt, Text};
use diesel::sqlite::Sqlite;

use std::collections::HashMap;

use bottle_core::{feed::*, Result};

use crate::model;
use crate::util;

// MARK: Internal methods for grouping post by artist

/// Sqlite row for recent posts query grouped by artist.
#[derive(QueryableByName)]
struct RecentRow {
    #[diesel(sql_type = Text)]
    artist: String,
    #[diesel(sql_type = BigInt)]
    post_id: i64,
    #[diesel(sql_type = BigInt)]
    post_count: i64,
    #[diesel(sql_type = BigInt)]
    artist_count: i64,
}

/// Generate query for artist-grouped recent post, with given source post query.
/// Binds are `page_size`, `offset` and `recent_count`.
pub(crate) fn grouped_by_user_query(post_query: &str, window_order_clause: &str) -> String {
    format!(
        "with posts as materialized (
                select *, panda_gallery_tag.name as artist from (
                    {}
                ) panda_post
                join panda_gallery_tag on panda_post.id = panda_gallery_tag.gallery_id
                where panda_gallery_tag.namespace = 'artist'
            ), artists as materialized (
                select *, count() over () as artist_count from (
                    select artist, count() as post_count
                    from posts
                    group by artist
                    order by post_count desc
                ) limit ? offset ?
            ), recent as materialized (
                select name as artist, id as post_id, rank () over (
                    partition by name
                    {}
                ) as rank
                from posts
            )
            select artists.artist, post_id, post_count, artist_count from artists
            join recent on recent.artist = artists.artist
            where rank <= ?
            order by post_count desc, artists.artist, rank;",
        post_query, window_order_clause
    )
}

/// Fetch recent posts grouped by artist with given source post query.
pub(crate) fn posts_grouped_by_user<Q: QueryFragment<Sqlite>>(
    db: Database,
    query: BoxedSqlQuery<'static, Sqlite, Q>,
    page: i64,
    page_size: i64,
    recent_count: i64,
) -> Result<GeneralResponse> {
    use bottle_core::schema::{panda_gallery, panda_media};
    use itertools::Itertools;

    // 1. Fetch row records from database
    let query = query
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(page * page_size)
        .bind::<BigInt, _>(recent_count);
    let records = query.load::<RecentRow>(db)?;

    let artists = records.iter().map(|r| r.artist.clone());
    let post_ids = records.iter().map(|r| r.post_id);
    let artist_count = records.first().map(|r| r.artist_count).unwrap_or(0);
    let artist_to_post_count = records
        .iter()
        .map(|r| (r.artist.clone(), r.post_count))
        .collect::<HashMap<_, _>>();

    // 2. Use tags as user info
    let mut users = artists.clone().unique().map(util::artist_view).collect::<Vec<_>>();
    // Add post_count field to users
    for user in &mut users {
        user.post_count = artist_to_post_count.get(&user.user_id).cloned();
    }
    // Sort users by post_count
    users.sort_by(|a, b| b.post_count.cmp(&a.post_count));

    // 3. Fetch associated posts
    let posts = panda_gallery::table
        .filter(panda_gallery::id.eq_any(post_ids.clone()))
        .load::<model::PandaGallery>(db)?;
    // Reorder posts by original order
    let posts_map = posts.into_iter().map(|post| (post.id, post)).collect::<HashMap<_, _>>();
    let posts = post_ids
        .clone()
        .filter_map(|id| posts_map.get(&id).cloned())
        .collect::<Vec<_>>();

    let tag_map = util::get_tag_map(db, post_ids.clone())?;
    let mut posts = posts
        .into_iter()
        .map(|gallery| gallery.post_view(tag_map.get(&gallery.id).cloned().unwrap_or_default()))
        .collect::<Vec<_>>();

    // Add user_id field to posts
    for (post, artist) in posts.iter_mut().zip(artists) {
        post.user_id = Some(artist.clone());
    }

    // 4. Fetch associated media (only the first one)
    let media = panda_media::table
        .filter(panda_media::gallery_id.eq_any(post_ids.clone()))
        .filter(panda_media::media_index.eq(0))
        .load::<model::PandaMedia>(db)?;

    // 5. Fetch associated works (only the first image)
    let post_ids = post_ids.clone().map(|id| id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "panda", post_ids, true)?;

    Ok(GeneralResponse {
        users: Some(users),
        posts: Some(posts),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        works: Some(works),
        images: Some(images),
        total_items: artist_count,
        page,
        page_size,
    })
}

/// Fetch artist with given post results.
pub(crate) fn posts_by_user(
    db: Database,
    results: (Vec<model::PandaGallery>, i64),
    user_id: String,
    page: i64,
    page_size: i64,
) -> Result<GeneralResponse> {
    use bottle_core::schema::panda_media;

    let (posts, total_items) = results;

    // 1. Fetch associated media
    let post_ids = posts.iter().map(|post| post.id).collect::<Vec<_>>();
    let media = panda_media::table
        .filter(panda_media::gallery_id.eq_any(post_ids.clone()))
        .order(panda_media::media_index.asc())
        .load::<model::PandaMedia>(db)?;

    let tag_map = util::get_tag_map(db, post_ids.clone())?;
    let posts = posts
        .into_iter()
        .map(|gallery| gallery.post_view(tag_map.get(&gallery.id).cloned().unwrap_or_default()))
        .collect();

    // 2. Fetch associated works
    let post_ids = post_ids.iter().map(|id| id.to_string());
    let (works, images) = bottle_library::get_works_by_post_ids(db, "panda", post_ids, false)?;

    Ok(GeneralResponse {
        posts: Some(posts),
        users: Some(vec![util::artist_view(user_id)]),
        media: Some(media.into_iter().map(MediaView::from).collect()),
        works: Some(works),
        images: Some(images),
        total_items,
        page,
        page_size,
    })
}
