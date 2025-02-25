use std::collections::HashMap;

use pixiv_client::Illust;

#[derive(Debug, Clone, Default)]
pub struct PixivCache {
    pub(crate) illusts: HashMap<u64, Illust>,
}

impl PixivCache {
    pub fn new() -> Self {
        Self {
            illusts: HashMap::new(),
        }
    }
}
