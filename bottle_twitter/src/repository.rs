/// Repository implementation for Twitter database operations.
/// This separates data access logic from business logic.

use std::collections::HashSet;

use diesel::prelude::*;

use bottle_core::{
    feed::{GeneralResponse, MediaView, PostView, UserView},
    schema::{tweet, twitter_media, twitter_user, twitter_watch_list_tweet},
    Database, Result,
};
use bottle_util::repository::{FeedRepository, PostRepository};

use crate::model::{NewTweet, NewTwitterUser, Tweet, TwitterMedia, TwitterUser};

/// Repository for Twitter post entities
pub struct TwitterPostRepository;

impl PostRepository<NewTweet, NewTwitterUser, TwitterMedia> for TwitterPostRepository {
    fn get_by_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<Tweet>> {
        let results = tweet::table
            .filter(tweet::id.eq_any(post_ids))
            .load::<Tweet>(db)?;
        Ok(results)
    }
    
    fn get_users_by_ids(db: Database, user_ids: impl IntoIterator<Item = i64>) -> Result<Vec<TwitterUser>> {
        let results = twitter_user::table
            .filter(twitter_user::id.eq_any(user_ids))
            .load::<TwitterUser>(db)?;
        Ok(results)
    }
    
    fn get_media_by_post_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<TwitterMedia>> {
        let results = twitter_media::table
            .filter(twitter_media::tweet_id.eq_any(post_ids))
            .order(twitter_media::page.asc())
            .load::<TwitterMedia>(db)?;
        Ok(results)
    }
    
    fn insert_batch(
        db: Database,
        users: &[NewTwitterUser],
        posts: &[NewTweet],
        media: &[TwitterMedia],
    ) -> Result<()> {
        db.transaction(|conn| -> Result<()> {
            if !users.is_empty() {
                diesel::insert_into(twitter_user::table)
                    .values(users)
                    .execute(conn)?;
            }
            if !posts.is_empty() {
                diesel::insert_into(tweet::table)
                    .values(posts)
                    .execute(conn)?;
            }
            if !media.is_empty() {
                diesel::insert_into(twitter_media::table)
                    .values(media)
                    .execute(conn)?;
            }
            Ok(())
        })
    }
}

/// Repository for Twitter feed entities
pub struct TwitterFeedRepository;

impl FeedRepository<crate::feed::TwitterFeed> for TwitterFeedRepository {
    fn get_existing_post_ids(
        db: Database,
        feed_id: i32,
        post_ids: &[i64],
    ) -> Result<Vec<i64>> {
        let existing_ids = twitter_watch_list_tweet::table
            .filter(twitter_watch_list_tweet::watch_list_id.eq(feed_id))
            .filter(twitter_watch_list_tweet::tweet_id.eq_any(post_ids))
            .select(twitter_watch_list_tweet::tweet_id)
            .load::<i64>(db)?;
        Ok(existing_ids)
    }
    
    fn mark_reached_end(db: Database, feed_id: i32) -> Result<()> {
        use bottle_core::schema::twitter_watch_list;
        diesel::update(twitter_watch_list::table.find(feed_id))
            .set(twitter_watch_list::reached_end.eq(true))
            .execute(db)?;
        Ok(())
    }
}

impl TwitterPostRepository {
    /// Get entities (posts, users, media) for given post IDs
    pub fn get_entities(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<GeneralResponse> {
        let post_ids: Vec<i64> = post_ids.into_iter().collect();
        
        // Fetch posts
        let posts = Self::get_by_ids(db, post_ids.iter().copied())?;
        
        // Fetch associated users
        let user_ids: Vec<i64> = posts.iter().map(|post| post.user_id).collect();
        let users = Self::get_users_by_ids(db, user_ids)?;
        
        // Fetch associated media
        let media = Self::get_media_by_post_ids(db, post_ids)?;
        
        Ok(GeneralResponse {
            posts: Some(posts.into_iter().map(PostView::from).collect()),
            users: Some(users.into_iter().map(UserView::from).collect()),
            media: Some(media.into_iter().map(MediaView::from).collect()),
            ..Default::default()
        })
    }
}
