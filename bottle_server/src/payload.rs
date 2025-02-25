use serde::Deserialize;

use bottle_core::feed::FeedInfo;
use bottle_panda::PandaFeedParams;
use bottle_pixiv::PixivFeedParams;
use bottle_twitter::TwitterFeedParams;
use bottle_yandere::YandereFeedParams;

/// Request for adding a new feed.
#[derive(Debug, Clone, Deserialize)]
pub struct NewFeedRequest {
    pub params: FeedParams,
    pub info: FeedInfo,
    /// If the community doesn't require authentication, `account_id` can be `None`.
    pub account_id: Option<i32>,
}

/// Enum of feed parameters for different community.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedParams {
    Twitter(TwitterFeedParams),
    Pixiv(PixivFeedParams),
    Panda(PandaFeedParams),
    Yandere(YandereFeedParams),
}
