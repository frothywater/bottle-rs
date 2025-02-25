-- Your SQL goes here
CREATE INDEX IF NOT EXISTS index_twitter_media_tweet_id ON twitter_media(tweet_id);

create index if not exists index_work_source_post_id on work(source, post_id);
create index if not exists index_work_source_post_id_int on work(source, post_id_int);

create index if not exists index_image_work_id_page_index on image(work_id, page_index);
