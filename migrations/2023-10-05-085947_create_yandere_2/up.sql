-- Your SQL goes here
CREATE TABLE yandere_tag(
    name TEXT NOT NULL PRIMARY KEY ON CONFLICT IGNORE,
    type TEXT NOT NULL
);

CREATE TABLE yandere_post_tag(
    post_id  BIGINT NOT NULL REFERENCES yandere_post (id) ON DELETE CASCADE,
    tag_name TEXT   NOT NULL REFERENCES yandere_tag (name) ON DELETE RESTRICT,
    PRIMARY KEY (post_id, tag_name) ON CONFLICT IGNORE
);

CREATE TABLE yandere_pool(
    id           BIGINT   NOT NULL PRIMARY KEY ON CONFLICT REPLACE,
    name         TEXT     NOT NULL,
    description  TEXT     NOT NULL,
    user_id      BIGINT   NOT NULL,
    post_count   INTEGER  NOT NULL,
    created_date DATETIME NOT NULL,
    updated_date DATETIME NOT NULL
);

CREATE TABLE yandere_pool_post(
    pool_id      BIGINT  NOT NULL REFERENCES yandere_pool (id) ON DELETE CASCADE,
    post_id      BIGINT  NOT NULL REFERENCES yandere_post (id) ON DELETE RESTRICT,
    sequence     TEXT NOT NULL,
    prev_post_id BIGINT,
    next_post_id BIGINT,
    PRIMARY KEY (pool_id, post_id) ON CONFLICT IGNORE
);