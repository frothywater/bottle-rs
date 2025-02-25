-- Your SQL goes here
CREATE TABLE twitter_account(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    cookies TEXT NOT NULL,
    user_id BIGINT,
    name TEXT,
    username TEXT,
    profile_image_url TEXT
);

CREATE TABLE twitter_user(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    name TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_image_url TEXT,
    description TEXT NOT NULL,
    url TEXT,
    location TEXT NOT NULL,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tweet(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    user_id BIGINT NOT NULL REFERENCES twitter_user(id) ON DELETE CASCADE,
    caption TEXT NOT NULL,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE twitter_media(
    id TEXT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    tweet_id BIGINT NOT NULL REFERENCES tweet(id) ON DELETE CASCADE,
    page INTEGER NOT NULL,
    /* Media type: photo, video, animated_gif */
    type TEXT NOT NULL,
    url TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    /* For video */
    preview_image_url TEXT,
    duration INTEGER
);

CREATE TABLE twitter_list(
    id BIGINT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES twitter_user(id) ON DELETE RESTRICT,
    description TEXT NOT NULL,
    member_count INTEGER NOT NULL,
    private BOOLEAN NOT NULL,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE twitter_list_member(
    list_id BIGINT NOT NULL REFERENCES twitter_list(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES twitter_user(id) ON DELETE RESTRICT,
    PRIMARY KEY (list_id, user_id)
);

CREATE TABLE twitter_watch_list(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    watching BOOLEAN NOT NULL DEFAULT 1,
    first_fetch_limit INTEGER,
    account_id INTEGER NOT NULL REFERENCES twitter_account(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    twitter_list_id BIGINT REFERENCES twitter_list(id) ON DELETE RESTRICT,
    user_id BIGINT REFERENCES twitter_user(id) ON DELETE RESTRICT,
    search_query TEXT,
    reached_end BOOLEAN NOT NULL DEFAULT 0,
    UNIQUE (
        account_id,
        kind,
        twitter_list_id,
        user_id,
        search_query
    ),
    CHECK (
        (
            kind in ('timeline', 'bookmarks')
            AND twitter_list_id IS NULL
            AND user_id IS NULL
            AND search_query IS NULL
        )
        OR (
            kind in ('posts', 'likes')
            AND twitter_list_id IS NULL
            AND user_id IS NOT NULL
            AND search_query IS NULL
        )
        OR (
            kind = 'list'
            AND twitter_list_id IS NOT NULL
            AND user_id IS NULL
            AND search_query IS NULL
        )
        OR (
            kind = 'search'
            AND twitter_list_id IS NULL
            AND user_id IS NULL
            AND search_query IS NOT NULL
        )
    )
);

CREATE TABLE twitter_watch_list_tweet(
    watch_list_id INTEGER NOT NULL REFERENCES twitter_watch_list(id) ON DELETE CASCADE,
    tweet_id BIGINT NOT NULL REFERENCES tweet(id) ON DELETE RESTRICT,
    sort_index BIGINT,
    /* If the tweet is not in the watch list anymore, then it is stale. */
    stale BOOLEAN NOT NULL DEFAULT 0,
    PRIMARY KEY (watch_list_id, tweet_id) ON CONFLICT IGNORE
);

CREATE TABLE twitter_watch_list_history(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    watch_list_id INTEGER NOT NULL REFERENCES twitter_watch_list(id) ON DELETE CASCADE,
    /* Comma-separated list of tweet IDs */
    ids TEXT NOT NULL,
    count INTEGER NOT NULL,
    top_cursor TEXT,
    top_sort_index BIGINT,
    bottom_cursor TEXT,
    bottom_sort_index BIGINT,
    updated_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);