-- Your SQL goes here
CREATE TABLE yandere_post(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    tags TEXT NOT NULL,
    creator_id BIGINT,
    author TEXT NOT NULL,
    url TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    file_size BIGINT NOT NULL,
    file_ext TEXT NOT NULL,
    rating TEXT NOT NULL,
    md5 TEXT NOT NULL,
    source TEXT NOT NULL,
    has_children BOOLEAN NOT NULL,
    parent_id BIGINT,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE yandere_watch_list(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    watching BOOLEAN NOT NULL DEFAULT 1,
    first_fetch_limit INTEGER,
    kind TEXT NOT NULL,
    search_query TEXT,
    pool_id INTEGER,
    reached_end BOOLEAN NOT NULL DEFAULT 0,
    UNIQUE (kind, search_query, pool_id)
);

CREATE TABLE yandere_watch_list_post(
    watch_list_id INTEGER NOT NULL REFERENCES yandere_watch_list(id) ON DELETE CASCADE,
    post_id BIGINT NOT NULL REFERENCES yandere_post(id) ON DELETE RESTRICT,
    sort_index INTEGER,
    PRIMARY KEY (watch_list_id, post_id) ON CONFLICT IGNORE
);

CREATE TABLE yandere_watch_list_history(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    watch_list_id INTEGER NOT NULL REFERENCES yandere_watch_list(id) ON DELETE CASCADE,
    ids TEXT NOT NULL,
    count INTEGER NOT NULL,
    updated_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);