pub mod list {
    use lazy_static::lazy_static;
    use scraper::Selector;

    lazy_static! {
        pub static ref GALLERY: Selector = Selector::parse("table.itg > tbody > tr").unwrap();
        pub static ref RESULT_STRING: Selector = Selector::parse("div.searchtext p").unwrap();
        pub static ref LINK_TO_FIRST: Selector = Selector::parse("#ufirst").unwrap();
        pub static ref LINK_TO_PREV: Selector = Selector::parse("#uprev").unwrap();
        pub static ref LINK_TO_NEXT: Selector = Selector::parse("#unext").unwrap();
        pub static ref LINK_TO_LAST: Selector = Selector::parse("#ulast").unwrap();
        pub static ref URL: Selector = Selector::parse("td.gl1e a").unwrap();
        pub static ref THUMBNAIL_URL: Selector = Selector::parse("td.gl1e img").unwrap();
        pub static ref CATEGORY: Selector = Selector::parse("div.gl3e > div:first-child").unwrap();
        pub static ref POSTED_DATE: Selector = Selector::parse("div.gl3e > div:nth-child(2)").unwrap();
        pub static ref RATING: Selector = Selector::parse("div.gl3e > div:nth-child(3)").unwrap();
        pub static ref UPLOADER: Selector = Selector::parse("div.gl3e > div:nth-child(4) > a").unwrap();
        pub static ref IMAGE_COUNT: Selector = Selector::parse("div.gl3e > div:nth-child(5)").unwrap();
        pub static ref FAVORITED_DATE: Selector =
            Selector::parse("div.gl3e > div:nth-child(7) > p:nth-child(2)").unwrap();
        pub static ref TAGS: Selector = Selector::parse("div.gl4e td div").unwrap();
        pub static ref FAVORITE_CATEGORIES: Selector = Selector::parse("div.fp").unwrap();
    }
}

pub mod gallery {
    use lazy_static::lazy_static;
    use scraper::Selector;

    lazy_static! {
        pub static ref THUMBNAIL_URL: Selector = Selector::parse("#gd1 > div").unwrap();
        pub static ref TITLE: Selector = Selector::parse("#gj").unwrap();
        pub static ref ENGLISH_TITLE: Selector = Selector::parse("#gn").unwrap();
        pub static ref CATEGORY: Selector = Selector::parse("#gdc > div").unwrap();
        pub static ref UPLOADER: Selector = Selector::parse("#gdn > a").unwrap();
        pub static ref POSTED_DATE: Selector = Selector::parse("#gdd tr:nth-child(1) > td:last-child").unwrap();
        pub static ref PARENT: Selector = Selector::parse("#gdd tr:nth-child(2) > td:last-child").unwrap();
        pub static ref VISIBLE: Selector = Selector::parse("#gdd tr:nth-child(3) > td:last-child").unwrap();
        pub static ref LANGUAGE: Selector = Selector::parse("#gdd tr:nth-child(4) > td:last-child").unwrap();
        pub static ref FILE_SIZE: Selector = Selector::parse("#gdd tr:nth-child(5) > td:last-child").unwrap();
        pub static ref IMAGE_COUNT: Selector = Selector::parse("#gdd tr:nth-child(6) > td:last-child").unwrap();
        pub static ref FAVORITED_COUNT: Selector = Selector::parse("#favcount").unwrap();
        pub static ref RATING: Selector = Selector::parse("#rating_label").unwrap();
        pub static ref RATING_COUNT: Selector = Selector::parse("#rating_count").unwrap();
        pub static ref FAVORITE_CATEGORY: Selector = Selector::parse("#fav div.i").unwrap();
        pub static ref TAGS: Selector = Selector::parse("#taglist a").unwrap();
        pub static ref URL: Selector = Selector::parse("table.ptt td:nth-child(2) > a").unwrap();
        pub static ref PREVIEW_PAGE_COUNT: Selector = Selector::parse("table.ptt td:nth-last-child(2) > a").unwrap();
        pub static ref PREVIEWS: Selector = Selector::parse("#gdt > a").unwrap();
        pub static ref PREVIEW_THUMBNAIL_URL: Selector = Selector::parse("div > div").unwrap();

        // Comment
    }
}

pub mod image {
    use lazy_static::lazy_static;
    use scraper::Selector;

    lazy_static! {
        pub static ref PAGE_NUMBER: Selector = Selector::parse("#i2 span:first-child").unwrap();
        pub static ref FILE_INFO: Selector = Selector::parse("#i2 > div:last-child").unwrap();
        pub static ref URL: Selector = Selector::parse("#i3 img").unwrap();
    }
}
