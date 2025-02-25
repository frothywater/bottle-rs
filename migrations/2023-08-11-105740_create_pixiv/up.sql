-- Your SQL goes here
CREATE TABLE pixiv_account(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    refresh_token TEXT NOT NULL,
    access_token TEXT,
    expiry DATETIME,
    user_id BIGINT,
    name TEXT,
    username TEXT,
    profile_image_url TEXT
);

CREATE TABLE pixiv_user(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    name TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_image_url TEXT,
    description TEXT NOT NULL,
    url TEXT,
    pawoo_url TEXT,
    twitter_username TEXT,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE pixiv_illust(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    user_id BIGINT NOT NULL REFERENCES pixiv_user(id) ON DELETE CASCADE,
    /* Type: illust, manga, ugoira */
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    caption TEXT NOT NULL,
    restrict BOOLEAN NOT NULL,
    sanity_level INTEGER NOT NULL,
    series_id BIGINT,
    series_title TEXT,
    thumbnail_url TEXT NOT NULL,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE pixiv_media(
    illust_id BIGINT NOT NULL REFERENCES pixiv_illust(id) ON DELETE CASCADE,
    page INTEGER NOT NULL,
    square_medium_url TEXT NOT NULL,
    medium_url TEXT NOT NULL,
    large_url TEXT NOT NULL,
    original_url TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    PRIMARY KEY (illust_id, page) ON CONFLICT IGNORE
);

CREATE TABLE pixiv_illust_tag(
    illust_id BIGINT NOT NULL REFERENCES pixiv_illust(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    PRIMARY KEY (illust_id, tag) ON CONFLICT IGNORE
);

CREATE TABLE pixiv_watch_list(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    watching BOOLEAN NOT NULL DEFAULT 1,
    first_fetch_limit INTEGER,
    account_id INTEGER NOT NULL REFERENCES pixiv_account(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    restriction TEXT,
    user_id BIGINT REFERENCES pixiv_user(id) ON DELETE RESTRICT,
    search_query TEXT,
    bookmark_tag TEXT,
    illust_type TEXT,
    reached_end BOOLEAN NOT NULL DEFAULT 0,
    UNIQUE (account_id, kind, user_id, search_query),
    CHECK (
        (
            kind = 'timeline'
            AND user_id IS NULL
            AND search_query IS NULL
            AND restriction IN ('public', 'private', 'all')
        )
        OR (
            kind = 'bookmarks'
            AND user_id IS NOT NULL
            AND search_query IS NULL
            AND restriction IN ('public', 'private')
        )
        OR (
            kind = 'posts'
            AND user_id IS NOT NULL
            AND search_query IS NULL
            AND restriction IS NULL
        )
        OR (
            kind = 'search'
            AND user_id IS NULL
            AND search_query IS NOT NULL
            AND restriction IS NULL
        )
    )
);

CREATE TABLE pixiv_watch_list_illust(
    watch_list_id INTEGER NOT NULL REFERENCES pixiv_watch_list(id) ON DELETE CASCADE,
    illust_id BIGINT NOT NULL REFERENCES pixiv_illust(id) ON DELETE RESTRICT,
    private_bookmark BOOLEAN NOT NULL DEFAULT 0,
    sort_index INTEGER,
    /* If the illust is not in the watch list anymore, then it is stale. */
    stale BOOLEAN NOT NULL DEFAULT 0,
    PRIMARY KEY (watch_list_id, illust_id) ON CONFLICT IGNORE
);

CREATE TABLE pixiv_watch_list_history(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    watch_list_id INTEGER NOT NULL REFERENCES pixiv_watch_list(id) ON DELETE CASCADE,
    /* Comma-separated list of illust IDs */
    ids TEXT NOT NULL,
    count INTEGER NOT NULL,
    next_bookmark_id BIGINT,
    updated_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);