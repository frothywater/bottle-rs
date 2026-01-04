/// Shared utilities for artist grouping operations.
/// These are commonly used across different community implementations.

use diesel::{
    prelude::*,
    query_builder::{BoxedSqlQuery, QueryFragment},
    sql_types::BigInt,
    sqlite::Sqlite,
};

use std::collections::HashSet;

use bottle_core::Database;

/// Sqlite row for recent posts query grouped by artist.
#[derive(QueryableByName)]
pub struct RecentRow {
    #[diesel(sql_type = BigInt)]
    pub user_id: i64,
    #[diesel(sql_type = BigInt)]
    pub post_id: i64,
    #[diesel(sql_type = BigInt)]
    pub post_count: i64,
    #[diesel(sql_type = BigInt)]
    pub user_count: i64,
}

/// Generate query for artist-grouped recent post, with given source post query.
/// Binds are `page_size`, `offset` and `recent_count`.
pub fn grouped_by_user_query(post_query: &str, window_order_clause: &str) -> String {
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

/// Execute a grouped by user query and return the results.
pub fn execute_grouped_query<Q: QueryFragment<Sqlite>>(
    db: Database,
    query: BoxedSqlQuery<'static, Sqlite, Q>,
    page: i64,
    page_size: i64,
    recent_count: i64,
) -> diesel::QueryResult<(Vec<RecentRow>, i64, std::collections::HashMap<String, i64>)> {
    let query = query
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(page * page_size)
        .bind::<BigInt, _>(recent_count);
    let records = query.load::<RecentRow>(db)?;

    let user_count = records.first().map(|r| r.user_count).unwrap_or(0);
    let user_to_post_count = records
        .iter()
        .map(|r| (r.user_id.to_string(), r.post_count))
        .collect::<std::collections::HashMap<_, _>>();

    Ok((records, user_count, user_to_post_count))
}

/// Generic trait for filtering media by works.
/// This is used to show only the media pages that correspond to saved works.
pub trait FilterMediaByWorks {
    type Media;
    
    fn filter_by_works(media: &[Self::Media], works: &[bottle_core::library::WorkView]) -> Vec<Self::Media>;
}

/// Helper to create a post-page set from works for filtering media.
pub fn create_post_page_set<F>(works: &[bottle_core::library::WorkView], parse_id: F) -> HashSet<(i64, Option<i32>)>
where
    F: Fn(&str) -> Option<i64>,
{
    works
        .iter()
        .filter_map(|work| {
            let post_id = parse_id(work.post_id.as_ref()?)?;
            Some((post_id, work.page_index))
        })
        .collect()
}
