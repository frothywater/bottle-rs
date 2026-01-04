/// Repository implementation for Yandere database operations.
/// This separates data access logic from business logic.

use diesel::prelude::*;

use bottle_core::{
    feed::{GeneralResponse, MediaView, PostView, UserView},
    schema::{yandere_post, yandere_tag},
    Database, Result,
};
use bottle_util::repository::{FeedRepository, PostRepository};

use crate::model::{NewYanderePost, YanderePost, YandereTag};
use crate::util;

/// Repository for Yandere post entities
pub struct YanderePostRepository;

impl PostRepository<NewYanderePost, YandereTag, YanderePost> for YanderePostRepository {
    fn get_by_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<YanderePost>> {
        let results = yandere_post::table
            .filter(yandere_post::id.eq_any(post_ids))
            .load::<YanderePost>(db)?;
        Ok(results)
    }
    
    fn get_users_by_ids(db: Database, _user_ids: impl IntoIterator<Item = i64>) -> Result<Vec<YandereTag>> {
        // Yandere doesn't have separate user entities
        Ok(vec![])
    }
    
    fn get_media_by_post_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<YanderePost>> {
        // For Yandere, posts are the media
        Self::get_by_ids(db, post_ids)
    }
    
    fn insert_batch(
        db: Database,
        _users: &[YandereTag],
        posts: &[NewYanderePost],
        _media: &[YanderePost],
    ) -> Result<()> {
        db.transaction(|conn| -> Result<()> {
            if !posts.is_empty() {
                diesel::insert_into(yandere_post::table)
                    .values(posts)
                    .execute(conn)?;
            }
            Ok(())
        })
    }
}

/// Repository for Yandere feed entities
pub struct YandereFeedRepository;

impl FeedRepository<crate::feed::YandereFeed> for YandereFeedRepository {
    fn get_existing_post_ids(
        db: Database,
        feed_id: i32,
        post_ids: &[i64],
    ) -> Result<Vec<i64>> {
        use bottle_core::schema::yandere_watch_list_post;
        let existing_ids = yandere_watch_list_post::table
            .filter(yandere_watch_list_post::watch_list_id.eq(feed_id))
            .filter(yandere_watch_list_post::post_id.eq_any(post_ids))
            .select(yandere_watch_list_post::post_id)
            .load::<i64>(db)?;
        Ok(existing_ids)
    }
    
    fn mark_reached_end(db: Database, feed_id: i32) -> Result<()> {
        use bottle_core::schema::yandere_watch_list;
        diesel::update(yandere_watch_list::table.find(feed_id))
            .set(yandere_watch_list::reached_end.eq(true))
            .execute(db)?;
        Ok(())
    }
}

impl YanderePostRepository {
    /// Get entities (posts, users, media) for given post IDs
    pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
        let post_ids: Vec<i64> = post_ids.into_iter().collect();
        
        // Fetch posts
        let posts = Self::get_by_ids(db, post_ids.iter().copied())?;
        
        // Get artist views
        let users = util::get_artist_views(db, post_ids.iter().copied())?;
        
        Ok(GeneralResponse {
            posts: Some(posts.iter().map(PostView::from).collect()),
            users: Some(users),
            media: Some(posts.iter().map(MediaView::from).collect()),
            ..Default::default()
        })
    }
}
