/// Repository pattern for database operations.
/// This separates database access logic from business logic in Feed implementations.

use bottle_core::{Database, Result};

/// Generic repository trait for CRUD operations on community entities.
/// This provides a clean separation between business logic and data access.
pub trait Repository<T> {
    /// Get all entities from the database
    fn all(db: Database) -> Result<Vec<T>>
    where
        Self: Sized;
    
    /// Get a single entity by ID
    fn get(db: Database, id: i32) -> Result<Option<T>>
    where
        Self: Sized;
    
    /// Delete an entity by ID
    fn delete(db: Database, id: i32) -> Result<()>
    where
        Self: Sized;
}

/// Repository for post/tweet/illust entities
pub trait PostRepository<Post, User, Media> {
    /// Get posts by IDs
    fn get_by_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<Post>>;
    
    /// Get users by IDs
    fn get_users_by_ids(db: Database, user_ids: impl IntoIterator<Item = i64>) -> Result<Vec<User>>;
    
    /// Get media by post IDs
    fn get_media_by_post_ids(db: Database, post_ids: impl IntoIterator<Item = i64>) -> Result<Vec<Media>>;
    
    /// Insert posts, users, and media in a transaction
    fn insert_batch(
        db: Database,
        users: &[User],
        posts: &[Post],
        media: &[Media],
    ) -> Result<()>;
}

/// Repository for feed/watch list entities
pub trait FeedRepository<Feed> {
    /// Get existing post IDs for a feed
    fn get_existing_post_ids(
        db: Database,
        feed_id: i32,
        post_ids: &[i64],
    ) -> Result<Vec<i64>>;
    
    /// Mark a feed as reached end
    fn mark_reached_end(db: Database, feed_id: i32) -> Result<()>;
}

/// Macro to implement basic Repository trait for a type
#[macro_export]
macro_rules! impl_repository {
    ($type:ty, $model:ty, $table:ident) => {
        impl bottle_util::repository::Repository<$type> for $type {
            fn all(db: bottle_core::Database) -> bottle_core::Result<Vec<$type>>
            where
                Self: Sized,
            {
                use bottle_core::schema::$table::dsl::*;
                let results = $table
                    .load::<$model>(db)?
                    .into_iter()
                    .map(<$type>::from)
                    .collect();
                Ok(results)
            }

            fn get(db: bottle_core::Database, entity_id: i32) -> bottle_core::Result<Option<$type>>
            where
                Self: Sized,
            {
                use bottle_core::schema::$table::dsl::*;
                let result = $table
                    .filter(id.eq(entity_id))
                    .first::<$model>(db)
                    .optional()?;
                Ok(result.map(<$type>::from))
            }

            fn delete(db: bottle_core::Database, entity_id: i32) -> bottle_core::Result<()>
            where
                Self: Sized,
            {
                use bottle_core::schema::$table::dsl::*;
                diesel::delete($table.filter(id.eq(entity_id))).execute(db)?;
                Ok(())
            }
        }
    };
}
