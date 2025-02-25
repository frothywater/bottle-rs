-- Your SQL goes here
CREATE TABLE panda_account(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    cookies TEXT NOT NULL,
    name TEXT,
    username TEXT
);

CREATE TABLE panda_gallery(
    id BIGINT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    token TEXT NOT NULL,
    title TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    category INTEGER NOT NULL,
    uploader TEXT NOT NULL,
    rating REAL NOT NULL,
    media_count INTEGER NOT NULL,
    english_title TEXT,
    parent TEXT,
    language TEXT,
    file_size INTEGER,
    created_date DATETIME NOT NULL,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE panda_media(
    gallery_id BIGINT NOT NULL REFERENCES panda_gallery(id) ON DELETE CASCADE,
    media_index INTEGER NOT NULL,
    token TEXT NOT NULL,
    thumbnail_url TEXT NOT NULL,
    url TEXT,
    filename TEXT,
    file_size INTEGER,
    width INTEGER,
    height INTEGER,
    PRIMARY KEY (gallery_id, media_index) ON CONFLICT IGNORE
);

CREATE TABLE panda_tag(
    namespace TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (namespace, name) ON CONFLICT IGNORE
);

CREATE TABLE panda_gallery_tag(
    gallery_id BIGINT NOT NULL REFERENCES panda_gallery(id) ON DELETE CASCADE,
    namespace TEXT NOT NULL,
    name TEXT NOT NULL,
    FOREIGN KEY (namespace, name) REFERENCES panda_tag(namespace, name) ON DELETE RESTRICT,
    PRIMARY KEY (gallery_id, namespace, name) ON CONFLICT IGNORE
);

CREATE TABLE panda_watch_list(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT,
    watching BOOLEAN NOT NULL DEFAULT 1,
    first_fetch_limit INTEGER,
    account_id INTEGER NOT NULL REFERENCES panda_account(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    query TEXT,
    reached_end BOOLEAN NOT NULL DEFAULT 0,
    UNIQUE (account_id, kind, query)
);


CREATE TABLE panda_watch_list_gallery(
    watch_list_id INTEGER NOT NULL REFERENCES panda_watch_list(id) ON DELETE CASCADE,
    gallery_id BIGINT NOT NULL REFERENCES panda_gallery(id) ON DELETE RESTRICT,
    sort_index INTEGER,
    /* If the gallery is not in the watch list anymore, then it is stale. */
    stale BOOLEAN NOT NULL DEFAULT 0,
    PRIMARY KEY (watch_list_id, gallery_id) ON CONFLICT IGNORE
);

CREATE TABLE panda_watch_list_history(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    watch_list_id INTEGER NOT NULL REFERENCES panda_watch_list(id) ON DELETE CASCADE,
    /* Comma-separated list of gallery IDs */
    ids TEXT NOT NULL,
    count INTEGER NOT NULL,
    prev_offset TEXT,
    next_offset TEXT,
    updated_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
