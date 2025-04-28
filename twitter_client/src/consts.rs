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
    ("articles_preview_enabled", true),
    ("c9s_tweet_anatomy_moderator_badge_enabled", true),
    ("communities_web_enable_tweet_community_results_fetch", true),
    ("creator_subscriptions_quote_tweet_preview_enabled", false),
    ("creator_subscriptions_tweet_preview_api_enabled", true),
    ("freedom_of_speech_not_reach_fetch_enabled", true),
    ("graphql_is_translatable_rweb_tweet_is_translatable_enabled", true),
    ("longform_notetweets_consumption_enabled", true),
    ("longform_notetweets_inline_media_enabled", true),
    ("longform_notetweets_rich_text_read_enabled", true),
    ("premium_content_api_read_enabled", false),
    ("profile_label_improvements_pcf_label_in_post_enabled", true),
    ("responsive_web_edit_tweet_api_enabled", true),
    ("responsive_web_enhance_cards_enabled", false),
    (
        "responsive_web_graphql_skip_user_profile_image_extensions_enabled",
        false,
    ),
    ("responsive_web_graphql_timeline_navigation_enabled", true),
    ("responsive_web_grok_analysis_button_from_backend", true),
    ("responsive_web_grok_analyze_button_fetch_trends_enabled", false),
    ("responsive_web_grok_analyze_post_followups_enabled", true),
    ("responsive_web_grok_image_annotation_enabled", true),
    ("responsive_web_grok_share_attachment_enabled", true),
    ("responsive_web_grok_show_grok_translated_post", false),
    ("responsive_web_jetfuel_frame", false),
    ("responsive_web_twitter_article_tweet_consumption_enabled", true),
    ("rweb_tipjar_consumption_enabled", true),
    ("rweb_video_screen_enabled", false),
    ("standardized_nudges_misinfo", true),
    ("tweet_awards_web_tipping_enabled", false),
    (
        "tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled",
        true,
    ),
    ("verified_phone_label_enabled", false),
    ("view_counts_everywhere_api_enabled", true),
];

pub const GRAPHQL_QIDS: phf::Map<&str, &str> = phf_map! {
    "SearchTimeline" => "AIdc203rPpK_k_2KWSdm7g",
    "AudioSpaceById" => "fYAuJHiY3TmYdBmrRtIKhA",
    "AudioSpaceSearch" => "NTq79TuSz6fHj8lQaferJw",
    "UserByScreenName" => "1VOOyvKkiI3FMmkeDNxM9A",
    "UserTweets" => "HeWHY26ItCfUmm1e6ITjeA",
    "ProfileSpotlightsQuery" => "9zwVLJ48lmVUk8u_Gh9DmA",
    "UserByRestId" => "WJ7rCtezBVT6nk6VM5R8Bw",
    "UsersByRestIds" => "OJBgJQIrij6e3cjqQ3Zu1Q",
    "UserMedia" => "vFPc2LVIu7so2uA_gHQAdg",
    "UserTweetsAndReplies" => "RIWc55YCNyUJ-U3HHGYkdg",
    "TweetResultByRestId" => "D_jNhjWZeRZT5NURzfJZSQ",
    "TweetDetail" => "_8aYOgEDz35BrBcBal1-_w",
    "TweetStats" => "EvbTkPDT-xQCfupPu0rWMA",
    "Likes" => "eQl7iWsCr2fChppuJdAeRw",
    "Followers" => "Elc_-qTARceHpztqhI9PQA",
    "Following" => "C1qZ6bs-L3oc_TKSZyxkXQ",
    "Retweeters" => "0BoJlKAxoNPQUHRftlwZ2w",
    "Favoriters" => "XRRjv1-uj1HZn3o324etOQ",
    "HomeLatestTimeline" => "CRprHpVA12yhsub-KRERIg",
    "HomeTimeline" => "Q_P3YVnmHunGFkZ8ISM-7w",
    "Bookmarks" => "-LGfdImKeQz0xS_jjUwzlA",
};
