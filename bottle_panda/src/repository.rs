/// Repository implementation for Panda database operations.
/// This separates data access logic from business logic.

use diesel::prelude::*;

use bottle_core::{
    feed::{GeneralResponse, MediaView, PostView, UserView},
    schema::{panda_gallery, panda_media},
    Database, Result,
};
use bottle_util::repository::{FeedRepository, PostRepository};

use crate::model::{NewPandaGallery, PandaGallery, PandaMedia};
use crate::util;

/// Repository for Panda post entities
pub struct PandaPostRepository;

impl PostRepository<NewPandaGallery, (), PandaMedia> for PandaPostRepository {
    fn get_by_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<PandaGallery>> {
        let results = panda_gallery::table
            .filter(panda_gallery::id.eq_any(post_ids))
            .load::<PandaGallery>(db)?;
        Ok(results)
    }
    
    fn get_users_by_ids(db: Database, _user_ids: impl IntoIterator<Item = i64>) -> Result<Vec<()>> {
        // Panda doesn't have separate user entities
        Ok(vec![])
    }
    
    fn get_media_by_post_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<PandaMedia>> {
        let results = panda_media::table
            .filter(panda_media::gallery_id.eq_any(post_ids))
            .order(panda_media::media_index.asc())
            .load::<PandaMedia>(db)?;
        Ok(results)
    }
    
    fn insert_batch(
        db: Database,
        _users: &[()],
        posts: &[NewPandaGallery],
        media: &[PandaMedia],
    ) -> Result<()> {
        db.transaction(|conn| -> Result<()> {
            if !posts.is_empty() {
                diesel::insert_into(panda_gallery::table)
                    .values(posts)
                    .execute(conn)?;
            }
            if !media.is_empty() {
                diesel::insert_into(panda_media::table)
                    .values(media)
                    .execute(conn)?;
            }
            Ok(())
        })
    }
}

/// Repository for Panda feed entities
pub struct PandaFeedRepository;

impl FeedRepository<crate::feed::PandaFeed> for PandaFeedRepository {
    fn get_existing_post_ids(
        db: Database,
        feed_id: i32,
        post_ids: &[i64],
    ) -> Result<Vec<i64>> {
        use bottle_core::schema::panda_watch_list_gallery;
        let existing_ids = panda_watch_list_gallery::table
            .filter(panda_watch_list_gallery::watch_list_id.eq(feed_id))
            .filter(panda_watch_list_gallery::gallery_id.eq_any(post_ids))
            .select(panda_watch_list_gallery::gallery_id)
            .load::<i64>(db)?;
        Ok(existing_ids)
    }
    
    fn mark_reached_end(db: Database, feed_id: i32) -> Result<()> {
        use bottle_core::schema::panda_watch_list;
        diesel::update(panda_watch_list::table.find(feed_id))
            .set(panda_watch_list::reached_end.eq(true))
            .execute(db)?;
        Ok(())
    }
}

impl PandaPostRepository {
    /// Get entities (posts, users, media) for given post IDs
    pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
        let post_ids: Vec<i64> = post_ids.into_iter().collect();
        
        // Fetch posts
        let posts = Self::get_by_ids(db, post_ids.iter().copied())?;
        
        // Fetch associated media
        let media = Self::get_media_by_post_ids(db, post_ids.clone())?;
        
        // Get tags and artist views
        let tag_map = util::get_tag_map(db, post_ids.iter().copied())?;
        let users = util::get_artist_views(db, post_ids.iter().copied())?;
        
        let posts: Vec<PostView> = posts
            .into_iter()
            .map(|gallery| gallery.post_view(tag_map.get(&gallery.id).cloned().unwrap_or_default()))
            .collect();
        
        Ok(GeneralResponse {
            posts: Some(posts),
            users: Some(users),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            ..Default::default()
        })
    }
}
