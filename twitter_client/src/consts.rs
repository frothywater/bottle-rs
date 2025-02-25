use phf::phf_map;

pub const GRAPHQL_API: &str = "https://twitter.com/i/api/graphql";
pub const REST_API: &str = "https://api.twitter.com/1.1";
pub const BEARER_TOKEN: &str =
    "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs=1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
pub const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36";

pub const LIST_API_MAX_COUNT: u32 = 100;
pub const SEARCH_API_MAX_COUNT: u32 = 20;

pub const DEFAULT_GRAPHQL_VARIABLES: &[(&str, bool)] = &[
    ("withSafetyModeUserFields", true),
    ("includePromotedContent", true),
    ("withQuickPromoteEligibilityTweetFields", true),
    ("withVoice", true),
    ("withV2Timeline", true),
    ("withDownvotePerspective", false),
    ("withBirdwatchNotes", true),
    ("withCommunity", true),
    ("withSuperFollowsUserFields", true),
    ("withReactionsMetadata", false),
    ("withReactionsPerspective", false),
    ("withSuperFollowsTweetFields", true),
    ("isMetatagsQuery", false),
    ("withReplays", true),
    ("withClientEventToken", false),
    ("withAttachments", true),
    ("withConversationQueryHighlights", true),
    ("withMessageQueryHighlights", true),
    ("withMessages", true),
];

pub const DEFAULT_GRAPHQL_FEATURES: &[(&str, bool)] = &[
    ("blue_business_profile_image_shape_enabled", true),
    ("creator_subscriptions_tweet_preview_api_enabled", true),
    ("freedom_of_speech_not_reach_fetch_enabled", true),
    ("graphql_is_translatable_rweb_tweet_is_translatable_enabled", true),
    ("graphql_timeline_v2_bookmark_timeline", true),
    ("hidden_profile_likes_enabled", true),
    ("highlights_tweets_tab_ui_enabled", true),
    ("interactive_text_enabled", true),
    ("longform_notetweets_consumption_enabled", true),
    ("longform_notetweets_inline_media_enabled", true),
    ("longform_notetweets_rich_text_read_enabled", true),
    ("longform_notetweets_richtext_consumption_enabled", true),
    ("profile_foundations_tweet_stats_enabled", true),
    ("profile_foundations_tweet_stats_tweet_frequency", true),
    ("responsive_web_birdwatch_note_limit_enabled", true),
    ("responsive_web_edit_tweet_api_enabled", true),
    ("responsive_web_enhance_cards_enabled", false),
    ("responsive_web_graphql_exclude_directive_enabled", true),
    (
        "responsive_web_graphql_skip_user_profile_image_extensions_enabled",
        false,
    ),
    ("responsive_web_graphql_timeline_navigation_enabled", true),
    ("responsive_web_media_download_video_enabled", false),
    ("responsive_web_text_conversations_enabled", false),
    ("responsive_web_twitter_article_data_v2_enabled", true),
    ("responsive_web_twitter_article_tweet_consumption_enabled", false),
    ("responsive_web_twitter_blue_verified_badge_is_enabled", true),
    ("rweb_lists_timeline_redesign_enabled", true),
    ("spaces_2022_h2_clipping", true),
    ("spaces_2022_h2_spaces_communities", true),
    ("standardized_nudges_misinfo", true),
    ("subscriptions_verification_info_verified_since_enabled", true),
    ("tweet_awards_web_tipping_enabled", false),
    (
        "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled",
        true,
    ),
    ("tweetypie_unmention_optimization_enabled", true),
    ("verified_phone_label_enabled", false),
    ("vibe_api_enabled", true),
    ("view_counts_everywhere_api_enabled", true),
];

pub const GRAPHQL_QIDS: phf::Map<&str, &str> = phf_map! {
    "SearchTimeline" => "nK1dw4oV3k4w5TdtcAdSww",
    "AudioSpaceById" => "fYAuJHiY3TmYdBmrRtIKhA",
    "AudioSpaceSearch" => "NTq79TuSz6fHj8lQaferJw",
    "UserByScreenName" => "sLVLhk0bGj3MVFEKTdax1w",
    "UserTweets" => "HuTx74BxAnezK1gWvYY7zg",
    "ProfileSpotlightsQuery" => "9zwVLJ48lmVUk8u_Gh9DmA",
    "UserByRestId" => "GazOglcBvgLigl3ywt6b3Q",
    "UsersByRestIds" => "OJBgJQIrij6e3cjqQ3Zu1Q",
    "UserMedia" => "YqiE3JL1KNgf9nSljYdxaA",
    "UserTweetsAndReplies" => "RIWc55YCNyUJ-U3HHGYkdg",
    "TweetResultByRestId" => "D_jNhjWZeRZT5NURzfJZSQ",
    "TweetDetail" => "zXaXQgfyR4GxE21uwYQSyA",
    "TweetStats" => "EvbTkPDT-xQCfupPu0rWMA",
    "Likes" => "nXEl0lfN_XSznVMlprThgQ",
    "Followers" => "pd8Tt1qUz1YWrICegqZ8cw",
    "Following" => "wjvx62Hye2dGVvnvVco0xA",
    "Retweeters" => "0BoJlKAxoNPQUHRftlwZ2w",
    "Favoriters" => "XRRjv1-uj1HZn3o324etOQ",
    "HomeLatestTimeline" => "zhX91JE87mWvfprhYE97xA",
    "HomeTimeline" => "HCosKfLNW1AcOo3la3mMgg",
    "Bookmarks" => "tmd4ifV8RHltzn8ymGg1aw",
};
