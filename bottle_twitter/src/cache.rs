use std::collections::HashMap;

use twitter_client::Tweet;

#[derive(Debug, Clone, Default)]
pub struct TwitterCache {
    pub(crate) tweets: HashMap<u64, Tweet>,
}

impl TwitterCache {
    pub fn new() -> Self {
        Self { tweets: HashMap::new() }
    }
}
