// @generated automatically by Diesel CLI.

diesel::table! {
    album (id) {
        id -> Integer,
        name -> Text,
        folder_id -> Nullable<Integer>,
        position -> Integer,
        added_date -> Timestamp,
        modified_date -> Timestamp,
    }
}

diesel::table! {
    album_work (album_id, work_id) {
        album_id -> Integer,
        work_id -> Integer,
        position -> Integer,
    }
}

diesel::table! {
    folder (id) {
        id -> Integer,
        name -> Text,
        parent_id -> Nullable<Integer>,
        position -> Integer,
        added_date -> Timestamp,
        modified_date -> Timestamp,
    }
}

diesel::table! {
    image (id) {
        id -> Integer,
        work_id -> Integer,
        page_index -> Nullable<Integer>,
        filename -> Text,
        remote_url -> Nullable<Text>,
        path -> Nullable<Text>,
        thumbnail_path -> Nullable<Text>,
        small_thumbnail_path -> Nullable<Text>,
        width -> Nullable<Integer>,
        height -> Nullable<Integer>,
        size -> Nullable<Integer>,
    }
}

diesel::table! {
    panda_account (id) {
        id -> Integer,
        cookies -> Text,
        name -> Nullable<Text>,
        username -> Nullable<Text>,
    }
}

diesel::table! {
    panda_gallery (id) {
        id -> BigInt,
        token -> Text,
        title -> Text,
        thumbnail_url -> Text,
        category -> Integer,
        uploader -> Text,
        rating -> Float,
        media_count -> Integer,
        english_title -> Nullable<Text>,
        parent -> Nullable<Text>,
        visible -> Nullable<Bool>,
        language -> Nullable<Text>,
        file_size -> Nullable<Integer>,
        created_date -> Timestamp,
        added_date -> Timestamp,
        stale -> Bool,
    }
}

diesel::table! {
    panda_gallery_tag (gallery_id, namespace, name) {
        gallery_id -> BigInt,
        namespace -> Text,
        name -> Text,
    }
}

diesel::table! {
    panda_media (gallery_id, media_index) {
        gallery_id -> BigInt,
        media_index -> Integer,
        token -> Text,
        thumbnail_url -> Nullable<Text>,
        url -> Nullable<Text>,
        filename -> Nullable<Text>,
        file_size -> Nullable<Integer>,
        width -> Nullable<Integer>,
        height -> Nullable<Integer>,
    }
}

diesel::table! {
    panda_tag (namespace, name) {
        namespace -> Text,
        name -> Text,
    }
}

diesel::table! {
    panda_watch_list (id) {
        id -> Integer,
        name -> Nullable<Text>,
        watching -> Bool,
        first_fetch_limit -> Nullable<Integer>,
        account_id -> Integer,
        kind -> Text,
        query -> Nullable<Text>,
        reached_end -> Bool,
    }
}

diesel::table! {
    panda_watch_list_gallery (watch_list_id, gallery_id) {
        watch_list_id -> Integer,
        gallery_id -> BigInt,
        sort_index -> Nullable<Integer>,
        stale -> Bool,
    }
}

diesel::table! {
    panda_watch_list_history (id) {
        id -> Integer,
        watch_list_id -> Integer,
        ids -> Text,
        count -> Integer,
        prev_offset -> Nullable<Text>,
        next_offset -> Nullable<Text>,
        updated_date -> Timestamp,
    }
}

diesel::table! {
    pixiv_account (id) {
        id -> Integer,
        refresh_token -> Text,
        access_token -> Nullable<Text>,
        expiry -> Nullable<Timestamp>,
        user_id -> Nullable<BigInt>,
        name -> Nullable<Text>,
        username -> Nullable<Text>,
        profile_image_url -> Nullable<Text>,
    }
}

diesel::table! {
    pixiv_illust (id) {
        id -> BigInt,
        user_id -> BigInt,
        #[sql_name = "type"]
        type_ -> Text,
        title -> Text,
        caption -> Text,
        restrict -> Bool,
        sanity_level -> Integer,
        series_id -> Nullable<BigInt>,
        series_title -> Nullable<Text>,
        thumbnail_url -> Text,
        created_date -> Timestamp,
        added_date -> Timestamp,
    }
}

diesel::table! {
    pixiv_illust_tag (illust_id, tag) {
        illust_id -> BigInt,
        tag -> Text,
    }
}

diesel::table! {
    pixiv_media (illust_id, page) {
        illust_id -> BigInt,
        page -> Integer,
        square_medium_url -> Text,
        medium_url -> Text,
        large_url -> Text,
        original_url -> Text,
        width -> Integer,
        height -> Integer,
    }
}

diesel::table! {
    pixiv_user (id) {
        id -> BigInt,
        name -> Text,
        username -> Text,
        profile_image_url -> Nullable<Text>,
        description -> Text,
        url -> Nullable<Text>,
        pawoo_url -> Nullable<Text>,
        twitter_username -> Nullable<Text>,
        added_date -> Timestamp,
    }
}

diesel::table! {
    pixiv_watch_list (id) {
        id -> Integer,
        name -> Nullable<Text>,
        watching -> Bool,
        first_fetch_limit -> Nullable<Integer>,
        account_id -> Integer,
        kind -> Text,
        restriction -> Nullable<Text>,
        user_id -> Nullable<BigInt>,
        search_query -> Nullable<Text>,
        bookmark_tag -> Nullable<Text>,
        illust_type -> Nullable<Text>,
        reached_end -> Bool,
    }
}

diesel::table! {
    pixiv_watch_list_history (id) {
        id -> Integer,
        watch_list_id -> Integer,
        ids -> Text,
        count -> Integer,
        next_bookmark_id -> Nullable<BigInt>,
        updated_date -> Timestamp,
    }
}

diesel::table! {
    pixiv_watch_list_illust (watch_list_id, illust_id) {
        watch_list_id -> Integer,
        illust_id -> BigInt,
        private_bookmark -> Bool,
        stale -> Bool,
        sort_index -> Nullable<Integer>,
    }
}

diesel::table! {
    tweet (id) {
        id -> BigInt,
        user_id -> BigInt,
        caption -> Text,
        created_date -> Timestamp,
        added_date -> Timestamp,
    }
}

diesel::table! {
    twitter_account (id) {
        id -> Integer,
        cookies -> Text,
        user_id -> Nullable<BigInt>,
        name -> Nullable<Text>,
        username -> Nullable<Text>,
        profile_image_url -> Nullable<Text>,
    }
}

diesel::table! {
    twitter_list (id) {
        id -> BigInt,
        name -> Text,
        user_id -> BigInt,
        description -> Text,
        member_count -> Integer,
        private -> Bool,
        created_date -> Timestamp,
        added_date -> Timestamp,
    }
}

diesel::table! {
    twitter_list_member (list_id, user_id) {
        list_id -> BigInt,
        user_id -> BigInt,
    }
}

diesel::table! {
    twitter_media (id) {
        id -> Text,
        tweet_id -> BigInt,
        #[sql_name = "type"]
        type_ -> Text,
        url -> Text,
        width -> Integer,
        height -> Integer,
        preview_image_url -> Nullable<Text>,
        duration -> Nullable<Integer>,
        page -> Integer,
    }
}

diesel::table! {
    twitter_user (id) {
        id -> BigInt,
        name -> Text,
        username -> Text,
        profile_image_url -> Nullable<Text>,
        description -> Text,
        url -> Nullable<Text>,
        location -> Text,
        created_date -> Timestamp,
        added_date -> Timestamp,
    }
}

diesel::table! {
    twitter_watch_list (id) {
        id -> Integer,
        name -> Nullable<Text>,
        watching -> Bool,
        first_fetch_limit -> Nullable<Integer>,
        account_id -> Integer,
        kind -> Text,
        twitter_list_id -> Nullable<BigInt>,
        user_id -> Nullable<BigInt>,
        search_query -> Nullable<Text>,
        reached_end -> Bool,
    }
}

diesel::table! {
    twitter_watch_list_history (id) {
        id -> Integer,
        watch_list_id -> Integer,
        ids -> Text,
        count -> Integer,
        top_cursor -> Nullable<Text>,
        top_sort_index -> Nullable<BigInt>,
        bottom_cursor -> Nullable<Text>,
        bottom_sort_index -> Nullable<BigInt>,
        updated_date -> Timestamp,
    }
}

diesel::table! {
    twitter_watch_list_tweet (watch_list_id, tweet_id) {
        watch_list_id -> Integer,
        tweet_id -> BigInt,
        sort_index -> Nullable<BigInt>,
        stale -> Bool,
    }
}

diesel::table! {
    work (id) {
        id -> Integer,
        source -> Nullable<Text>,
        post_id -> Nullable<Text>,
        post_id_int -> Nullable<BigInt>,
        page_index -> Nullable<Integer>,
        as_archive -> Bool,
        image_count -> Integer,
        name -> Nullable<Text>,
        caption -> Nullable<Text>,
        favorite -> Bool,
        rating -> Integer,
        thumbnail_path -> Nullable<Text>,
        small_thumbnail_path -> Nullable<Text>,
        added_date -> Timestamp,
        modified_date -> Timestamp,
        viewed_date -> Nullable<Timestamp>,
    }
}

diesel::table! {
    yandere_pool (id) {
        id -> BigInt,
        name -> Text,
        description -> Text,
        user_id -> BigInt,
        post_count -> Integer,
        created_date -> Timestamp,
        updated_date -> Timestamp,
    }
}

diesel::table! {
    yandere_pool_post (pool_id, post_id) {
        pool_id -> BigInt,
        post_id -> BigInt,
        sequence -> Text,
        prev_post_id -> Nullable<BigInt>,
        next_post_id -> Nullable<BigInt>,
    }
}

diesel::table! {
    yandere_post (id) {
        id -> BigInt,
        tags -> Text,
        creator_id -> Nullable<BigInt>,
        author -> Text,
        url -> Text,
        thumbnail_url -> Text,
        width -> Integer,
        height -> Integer,
        file_size -> BigInt,
        file_ext -> Text,
        rating -> Text,
        md5 -> Text,
        source -> Text,
        has_children -> Bool,
        parent_id -> Nullable<BigInt>,
        created_date -> Timestamp,
        added_date -> Timestamp,
    }
}

diesel::table! {
    yandere_post_tag (post_id, tag_name) {
        post_id -> BigInt,
        tag_name -> Text,
    }
}

diesel::table! {
    yandere_tag (name) {
        name -> Text,
        #[sql_name = "type"]
        type_ -> Text,
    }
}

diesel::table! {
    yandere_watch_list (id) {
        id -> Integer,
        name -> Nullable<Text>,
        watching -> Bool,
        first_fetch_limit -> Nullable<Integer>,
        kind -> Text,
        search_query -> Nullable<Text>,
        pool_id -> Nullable<Integer>,
        reached_end -> Bool,
    }
}

diesel::table! {
    yandere_watch_list_history (id) {
        id -> Integer,
        watch_list_id -> Integer,
        ids -> Text,
        count -> Integer,
        updated_date -> Timestamp,
    }
}

diesel::table! {
    yandere_watch_list_post (watch_list_id, post_id) {
        watch_list_id -> Integer,
        post_id -> BigInt,
        sort_index -> Nullable<Integer>,
    }
}

diesel::joinable!(album -> folder (folder_id));
diesel::joinable!(album_work -> album (album_id));
diesel::joinable!(album_work -> work (work_id));
diesel::joinable!(image -> work (work_id));
diesel::joinable!(panda_gallery_tag -> panda_gallery (gallery_id));
diesel::joinable!(panda_media -> panda_gallery (gallery_id));
diesel::joinable!(panda_watch_list -> panda_account (account_id));
diesel::joinable!(panda_watch_list_gallery -> panda_gallery (gallery_id));
diesel::joinable!(panda_watch_list_gallery -> panda_watch_list (watch_list_id));
diesel::joinable!(panda_watch_list_history -> panda_watch_list (watch_list_id));
diesel::joinable!(pixiv_illust -> pixiv_user (user_id));
diesel::joinable!(pixiv_illust_tag -> pixiv_illust (illust_id));
diesel::joinable!(pixiv_media -> pixiv_illust (illust_id));
diesel::joinable!(pixiv_watch_list -> pixiv_account (account_id));
diesel::joinable!(pixiv_watch_list -> pixiv_user (user_id));
diesel::joinable!(pixiv_watch_list_history -> pixiv_watch_list (watch_list_id));
diesel::joinable!(pixiv_watch_list_illust -> pixiv_illust (illust_id));
diesel::joinable!(pixiv_watch_list_illust -> pixiv_watch_list (watch_list_id));
diesel::joinable!(tweet -> twitter_user (user_id));
diesel::joinable!(twitter_list -> twitter_user (user_id));
diesel::joinable!(twitter_list_member -> twitter_list (list_id));
diesel::joinable!(twitter_list_member -> twitter_user (user_id));
diesel::joinable!(twitter_media -> tweet (tweet_id));
diesel::joinable!(twitter_watch_list -> twitter_account (account_id));
diesel::joinable!(twitter_watch_list -> twitter_list (twitter_list_id));
diesel::joinable!(twitter_watch_list -> twitter_user (user_id));
diesel::joinable!(twitter_watch_list_history -> twitter_watch_list (watch_list_id));
diesel::joinable!(twitter_watch_list_tweet -> tweet (tweet_id));
diesel::joinable!(twitter_watch_list_tweet -> twitter_watch_list (watch_list_id));
diesel::joinable!(yandere_pool_post -> yandere_pool (pool_id));
diesel::joinable!(yandere_pool_post -> yandere_post (post_id));
diesel::joinable!(yandere_post_tag -> yandere_post (post_id));
diesel::joinable!(yandere_post_tag -> yandere_tag (tag_name));
diesel::joinable!(yandere_watch_list_history -> yandere_watch_list (watch_list_id));
diesel::joinable!(yandere_watch_list_post -> yandere_post (post_id));
diesel::joinable!(yandere_watch_list_post -> yandere_watch_list (watch_list_id));

diesel::allow_tables_to_appear_in_same_query!(
    album,
    album_work,
    folder,
    image,
    panda_account,
    panda_gallery,
    panda_gallery_tag,
    panda_media,
    panda_tag,
    panda_watch_list,
    panda_watch_list_gallery,
    panda_watch_list_history,
    pixiv_account,
    pixiv_illust,
    pixiv_illust_tag,
    pixiv_media,
    pixiv_user,
    pixiv_watch_list,
    pixiv_watch_list_history,
    pixiv_watch_list_illust,
    tweet,
    twitter_account,
    twitter_list,
    twitter_list_member,
    twitter_media,
    twitter_user,
    twitter_watch_list,
    twitter_watch_list_history,
    twitter_watch_list_tweet,
    work,
    yandere_pool,
    yandere_pool_post,
    yandere_post,
    yandere_post_tag,
    yandere_tag,
    yandere_watch_list,
    yandere_watch_list_history,
    yandere_watch_list_post,
);
