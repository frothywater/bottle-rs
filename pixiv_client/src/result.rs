use url::Url;

pub use crate::response::{
    Account, BookmarkDetail, BookmarkTagList, FollowDetail, Illust, IllustList, LoginResponse, Tag, Ugoira, User,
    UserDetail, UserList,
};

pub trait Paginated {
    fn next_url(&self) -> Option<&str>;

    fn reached_end(&self) -> bool {
        self.next_url().is_none()
    }

    fn next_url_query_items(&self) -> Vec<(String, String)> {
        let Some(next_url) = self.next_url() else {
            return vec![];
        };
        let Ok(url) = Url::parse(next_url) else {
            return vec![];
        };
        url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    fn item(&self, key: &str) -> Option<u64> {
        self.next_url_query_items()
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.parse::<u64>().ok())
    }

    fn items(&self, key: &str) -> Vec<u64> {
        self.next_url_query_items()
            .iter()
            .filter(|(k, _)| k == key)
            .filter_map(|(_, v)| v.parse::<u64>().ok())
            .collect()
    }

    /// Pagination offset for next request. Nil if there can be no more request.
    fn next_offset(&self) -> Option<u64> {
        self.item("offset")
    }

    /// Max bookmark ID for next request. Nil if there can be no more request.
    fn next_bookmark_id(&self) -> Option<u64> {
        self.item("max_bookmark_id")
    }

    /// IDs of viewed illustrations for next request.
    fn next_view_ids(&self) -> Vec<u64> {
        self.items("viewed")
    }
    /// IDs of seed illustrations for next request.
    fn next_seed_ids(&self) -> Vec<u64> {
        self.items("seed_illust_ids")
    }
}

impl Paginated for IllustList {
    fn next_url(&self) -> Option<&str> {
        self.next_url.as_deref()
    }
}

impl Paginated for UserList {
    fn next_url(&self) -> Option<&str> {
        self.next_url.as_deref()
    }
}

impl Paginated for BookmarkTagList {
    fn next_url(&self) -> Option<&str> {
        self.next_url.as_deref()
    }
}
