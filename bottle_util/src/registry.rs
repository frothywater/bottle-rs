/// Feed Registry Pattern for true polymorphism.
/// This replaces the FeedWrapper enum approach with a more extensible registry system.

use std::collections::HashMap;
use std::sync::RwLock;

use bottle_core::{
    feed::{FeedInfo, FeedView, GeneralResponse},
    Database, Result,
};

/// Trait for feed operations that can be performed polymorphically
pub trait FeedOperations: Send + Sync {
    /// Get the feed view
    fn view(&self) -> FeedView;
    
    /// Modify feed information
    fn modify(&mut self, db: Database, info: &FeedInfo) -> Result<FeedView>;
    
    /// Get posts for the feed
    fn posts(&self, db: Database, page: i64, page_size: i64) -> Result<GeneralResponse>;
    
    /// Get users grouped view
    fn users(&self, db: Database, page: i64, page_size: i64, recent_count: i64) -> Result<GeneralResponse>;
    
    /// Get posts by user
    fn user_posts(&self, db: Database, user_id: String, page: i64, page_size: i64) -> Result<GeneralResponse>;
    
    /// Get the community name
    fn community(&self) -> &str;
    
    /// Get the feed ID
    fn id(&self) -> i32;
}

/// Feed factory trait for creating feeds from parameters
pub trait FeedFactory: Send + Sync {
    /// Get all feeds for this community
    fn all(&self, db: Database) -> Result<Vec<Box<dyn FeedOperations>>>;
    
    /// Get a specific feed by ID
    fn get(&self, db: Database, feed_id: i32) -> Result<Option<Box<dyn FeedOperations>>>;
    
    /// Delete a feed by ID
    fn delete(&self, db: Database, feed_id: i32) -> Result<()>;
    
    /// Get the community name
    fn community_name(&self) -> &str;
}

/// Global feed registry
pub struct FeedRegistry {
    factories: RwLock<HashMap<String, Box<dyn FeedFactory>>>,
}

impl FeedRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a feed factory for a community
    pub fn register(&self, community: String, factory: Box<dyn FeedFactory>) {
        let mut factories = self.factories.write().unwrap();
        factories.insert(community, factory);
    }
    
    /// Get a feed by community and ID
    pub fn get_feed(&self, db: Database, community: &str, feed_id: i32) -> Result<Option<Box<dyn FeedOperations>>> {
        let factories = self.factories.read().unwrap();
        if let Some(factory) = factories.get(community) {
            factory.get(db, feed_id)
        } else {
            Ok(None)
        }
    }
    
    /// Get all feeds for a community
    pub fn get_all_feeds(&self, db: Database, community: &str) -> Result<Vec<Box<dyn FeedOperations>>> {
        let factories = self.factories.read().unwrap();
        if let Some(factory) = factories.get(community) {
            factory.all(db)
        } else {
            Ok(vec![])
        }
    }
    
    /// Delete a feed
    pub fn delete_feed(&self, db: Database, community: &str, feed_id: i32) -> Result<()> {
        let factories = self.factories.read().unwrap();
        if let Some(factory) = factories.get(community) {
            factory.delete(db, feed_id)
        } else {
            Err(bottle_core::Error::InvalidEndpoint(format!("Community {}", community)))
        }
    }
    
    /// Get all registered communities
    pub fn communities(&self) -> Vec<String> {
        let factories = self.factories.read().unwrap();
        factories.keys().cloned().collect()
    }
}

impl Default for FeedRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to implement FeedOperations for a specific feed type
#[macro_export]
macro_rules! impl_feed_operations {
    ($feed_type:ty) => {
        impl bottle_util::registry::FeedOperations for $feed_type {
            fn view(&self) -> bottle_core::feed::FeedView {
                <$feed_type as bottle_core::feed::Feed>::view(self)
            }
            
            fn modify(&mut self, db: bottle_core::Database, info: &bottle_core::feed::FeedInfo) -> bottle_core::Result<bottle_core::feed::FeedView> {
                <$feed_type as bottle_core::feed::Feed>::modify(self, db, info)
            }
            
            fn posts(&self, db: bottle_core::Database, page: i64, page_size: i64) -> bottle_core::Result<bottle_core::feed::GeneralResponse> {
                <$feed_type as bottle_core::feed::Feed>::posts(self, db, page, page_size)
            }
            
            fn users(&self, db: bottle_core::Database, page: i64, page_size: i64, recent_count: i64) -> bottle_core::Result<bottle_core::feed::GeneralResponse> {
                <$feed_type as bottle_core::feed::Feed>::feed_posts_grouped_by_user(self, db, page, page_size, recent_count)
            }
            
            fn user_posts(&self, db: bottle_core::Database, user_id: String, page: i64, page_size: i64) -> bottle_core::Result<bottle_core::feed::GeneralResponse> {
                <$feed_type as bottle_core::feed::Feed>::feed_posts_by_user(self, db, user_id, page, page_size)
            }
            
            fn community(&self) -> &str {
                // This should be implemented per community
                stringify!($feed_type).split("Feed").next().unwrap_or("unknown").to_lowercase().as_ref()
            }
            
            fn id(&self) -> i32 {
                self.id
            }
        }
    };
}
