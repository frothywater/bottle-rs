use std::collections::HashMap;

use yandere_client::{PoolPostResult, PoolResult, PostResult, TagType};

#[derive(Debug, Clone, Default)]
pub struct YandereCache {
    /// post_id -> PostResult
    pub(crate) posts: HashMap<u64, PostResult>,
    /// pool_id -> PoolResult
    pub(crate) pools: HashMap<u64, PoolResult>,
    /// post_id -> PoolPostResults
    pub(crate) post_pools: HashMap<u64, Vec<PoolPostResult>>,
    /// tag -> TagType
    pub(crate) tags: HashMap<String, TagType>,
}

impl YandereCache {
    pub fn new() -> Self {
        Self {
            posts: HashMap::new(),
            pools: HashMap::new(),
            post_pools: HashMap::new(),
            tags: HashMap::new(),
        }
    }
}
