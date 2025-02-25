-- Your SQL goes here
CREATE TABLE work(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    /* Identifier of the community plugin which the work came from.
     If a work has external source, it's not necessarily imported from plugin
     because retrieving metadata of a manually added work via plugin will also connect it to the source. */
    source TEXT,
    post_id TEXT,
    post_id_int INTEGER,
    page_index INTEGER,
    /* A work with multiple images can be saved as images or an archive file. */
    as_archive BOOLEAN NOT NULL DEFAULT 0,
    /* A work can be a single image or a collection of images
     and can be saved as image(s) or an archive file.
     This field is used to record the number of images when it is saved as image files.
     Useful to determine if the work has images yet to be downloaded.
     */
    image_count INTEGER NOT NULL DEFAULT 1,
    name TEXT,
    caption TEXT,
    favorite BOOLEAN NOT NULL DEFAULT 0,
    rating INTEGER NOT NULL DEFAULT 0,
    thumbnail_path TEXT,
    small_thumbnail_path TEXT,
    added_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    modified_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    viewed_date DATETIME
);

CREATE TABLE image(
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    work_id INTEGER NOT NULL REFERENCES work(id) ON DELETE CASCADE,
    page_index INTEGER,
    filename TEXT NOT NULL,
    remote_url TEXT,
    path TEXT,
    thumbnail_path TEXT,
    small_thumbnail_path TEXT,
    width INTEGER,
    height INTEGER,
    size INTEGER
);