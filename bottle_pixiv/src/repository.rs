/// Repository implementation for Pixiv database operations.
/// This separates data access logic from business logic.

use diesel::prelude::*;

use bottle_core::{
    feed::{GeneralResponse, MediaView, PostView, UserView},
    schema::{pixiv_illust, pixiv_media, pixiv_user},
    Database, Result,
};
use bottle_util::repository::{FeedRepository, PostRepository};

use crate::model::{NewPixivIllust, NewPixivUser, PixivIllust, PixivMedia, PixivUser};
use crate::util;

/// Repository for Pixiv post entities
pub struct PixivPostRepository;

impl PostRepository<NewPixivIllust, NewPixivUser, PixivMedia> for PixivPostRepository {
    fn get_by_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<PixivIllust>> {
        let results = pixiv_illust::table
            .filter(pixiv_illust::id.eq_any(post_ids))
            .load::<PixivIllust>(db)?;
        Ok(results)
    }
    
    fn get_users_by_ids(db: Database, user_ids: impl IntoIterator<Item = i64>) -> Result<Vec<PixivUser>> {
        let results = pixiv_user::table
            .filter(pixiv_user::id.eq_any(user_ids))
            .load::<PixivUser>(db)?;
        Ok(results)
    }
    
    fn get_media_by_post_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<PixivMedia>> {
        let results = pixiv_media::table
            .filter(pixiv_media::illust_id.eq_any(post_ids))
            .order(pixiv_media::page.asc())
            .load::<PixivMedia>(db)?;
        Ok(results)
    }
    
    fn insert_batch(
        db: Database,
        users: &[NewPixivUser],
        posts: &[NewPixivIllust],
        media: &[PixivMedia],
    ) -> Result<()> {
        db.transaction(|conn| -> Result<()> {
            if !users.is_empty() {
                diesel::insert_into(pixiv_user::table)
                    .values(users)
                    .execute(conn)?;
            }
            if !posts.is_empty() {
                diesel::insert_into(pixiv_illust::table)
                    .values(posts)
                    .execute(conn)?;
            }
            if !media.is_empty() {
                diesel::insert_into(pixiv_media::table)
                    .values(media)
                    .execute(conn)?;
            }
            Ok(())
        })
    }
}

/// Repository for Pixiv feed entities
pub struct PixivFeedRepository;

impl FeedRepository<crate::feed::PixivFeed> for PixivFeedRepository {
    fn get_existing_post_ids(
        db: Database,
        feed_id: i32,
        post_ids: &[i64],
    ) -> Result<Vec<i64>> {
        use bottle_core::schema::pixiv_watch_list_illust;
        let existing_ids = pixiv_watch_list_illust::table
            .filter(pixiv_watch_list_illust::watch_list_id.eq(feed_id))
            .filter(pixiv_watch_list_illust::illust_id.eq_any(post_ids))
            .select(pixiv_watch_list_illust::illust_id)
            .load::<i64>(db)?;
        Ok(existing_ids)
    }
    
    fn mark_reached_end(db: Database, feed_id: i32) -> Result<()> {
        use bottle_core::schema::pixiv_watch_list;
        diesel::update(pixiv_watch_list::table.find(feed_id))
            .set(pixiv_watch_list::reached_end.eq(true))
            .execute(db)?;
        Ok(())
    }
}

impl PixivPostRepository {
    /// Get entities (posts, users, media) for given post IDs
    pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
        let post_ids: Vec<i64> = post_ids.into_iter().collect();
        
        // Fetch posts
        let posts = Self::get_by_ids(db, post_ids.iter().copied())?;
        
        // Fetch associated users
        let user_ids: Vec<i64> = posts.iter().map(|post| post.user_id).collect();
        let users = Self::get_users_by_ids(db, user_ids)?;
        
        // Fetch associated media
        let media = Self::get_media_by_post_ids(db, post_ids.clone())?;
        
        // Get tags
        let tag_map = util::get_tag_map(db, post_ids)?;
        let posts: Vec<PostView> = posts
            .into_iter()
            .map(|illust| illust.post_view(tag_map.get(&illust.id).cloned().unwrap_or_default()))
            .collect();
        
        Ok(GeneralResponse {
            posts: Some(posts),
            users: Some(users.into_iter().map(UserView::from).collect()),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            ..Default::default()
        })
    }
}
