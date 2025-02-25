use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};
use scraper::{ElementRef, Html, Selector};
use url::Url;

use crate::error::{Error, Result};
use crate::result::*;

fn parse_gid_token(url: &str) -> Option<(u64, String)> {
    let mut parts = url.strip_prefix("https://")?.split('/');
    let gid = parts.nth(2)?.to_string().parse::<u64>().ok()?;
    let token = parts.next()?.to_string();
    Some((gid, token))
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
        .ok()
        .map(|dt| dt.and_utc())
}

fn parse_file_size_text(s: &str) -> Option<u32> {
    let mut parts = s.split_whitespace();
    let size = parts.next()?.replace(",", "").parse::<f32>().ok()?;
    let unit = parts.next()?;
    let result = match unit {
        "KiB" => size * 1024.0,
        "MiB" => size * 1024.0 * 1024.0,
        "GiB" => size * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some(result as u32)
}

fn parse_tag_text(s: &str) -> Option<GalleryTag> {
    let mut parts = s.split(':');
    let namespace = parts.next()?.to_string();
    let namespace = TagNamespace::from_str(&namespace).ok()?;
    let name = parts.next()?.to_string();
    Some(GalleryTag { namespace, name })
}

fn parse_list_offset(url: &str) -> Option<GalleryListOffset> {
    let url = Url::parse(url).ok()?;
    url.query_pairs().find_map(|(k, v)| match k.as_ref() {
        "prev" => Some(GalleryListOffset::NewerThan(v.to_string())),
        "next" => Some(GalleryListOffset::OlderThan(v.to_string())),
        "range" => Some(GalleryListOffset::Percentage(v.parse::<u32>().ok()?)),
        _ => None,
    })
}

pub fn parse_ban(doc: &Html) -> Option<String> {
    doc.root_element()
        .text()
        .find(|s| s.contains("The ban expires in"))
        .map(|s| s.to_string())
}

pub fn parse_gallery_list(doc: &Html) -> Result<GalleryListResult> {
    use super::selectors::list::*;

    fn parse_gallery_count(doc: &Html) -> Option<u32> {
        let text = doc.select(&RESULT_STRING).next()?.text().next()?.trim();
        if !text.starts_with("Found") {
            return None;
        }
        // Ignore thousands separator
        let text = text.replace(",", "");
        // Find the first number in the string
        text.split_whitespace().find_map(|s| s.parse::<u32>().ok())
    }

    fn parse_favorite_categories(doc: &Html) -> Option<Vec<FavoriteCategory>> {
        let categories = doc
            .select(&FAVORITE_CATEGORIES)
            .enumerate()
            .filter_map(|(index, e)| {
                let texts = e.text().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>();
                Some(FavoriteCategory {
                    index: index as u32,
                    name: texts.get(1)?.to_string(),
                    gallery_count: texts.first()?.parse::<u32>().ok()?,
                })
            })
            .collect::<Vec<_>>();
        (!categories.is_empty()).then_some(categories)
    }

    fn parse_gallery(e: ElementRef) -> Result<Gallery> {
        fn parse_url(e: ElementRef) -> Option<String> {
            Some(e.select(&URL).next()?.value().attr("href")?.to_string())
        }

        fn parse_title(e: ElementRef) -> Option<String> {
            Some(e.select(&THUMBNAIL_URL).next()?.value().attr("title")?.to_string())
        }

        fn parse_thumbnail_url(e: ElementRef) -> Option<String> {
            Some(e.select(&THUMBNAIL_URL).next()?.value().attr("src")?.to_string())
        }

        fn parse_category(e: ElementRef) -> Option<GalleryCategory> {
            let text = e.select(&CATEGORY).next()?.text().next()?.trim();
            GalleryCategory::from_str(text).ok()
        }

        fn parse_posted_date(e: ElementRef) -> Option<DateTime<Utc>> {
            let text = e.select(&POSTED_DATE).next()?.text().next()?.trim();
            parse_date(text)
        }

        fn parse_favorited_category_name(e: ElementRef) -> Option<String> {
            Some(e.select(&POSTED_DATE).next()?.value().attr("title")?.to_string())
        }

        fn parse_favorited_category_index(e: ElementRef) -> Option<u32> {
            let style = e.select(&POSTED_DATE).next()?.value().attr("style")?.trim();
            let border_color = style.strip_prefix("border-color: ")?;
            let result = match &border_color[..4] {
                "#000" => 0,
                "#f00" => 1,
                "#fa0" => 2,
                "#dd0" => 3,
                "#080" => 4,
                "#9f4" => 5,
                "#4bf" => 6,
                "#00f" => 7,
                "#508" => 8,
                "#e8e" => 9,
                _ => return None,
            };
            Some(result)
        }

        fn parse_coarse_rating(e: ElementRef) -> Option<f32> {
            let style = e.select(&RATING).next()?.value().attr("style")?;
            let mut result: f32;
            if style.contains("0px") {
                result = 5.0;
            } else if style.contains("-16px") {
                result = 4.0;
            } else if style.contains("-32px") {
                result = 3.0;
            } else if style.contains("-48px") {
                result = 2.0;
            } else if style.contains("-64px") {
                result = 1.0;
            } else if style.contains("-80px") {
                result = 0.0;
            } else {
                return None;
            }
            if style.contains("-21px") {
                result -= 0.5;
            }
            Some(result)
        }

        fn parse_uploader(e: ElementRef) -> Option<String> {
            let text = e.select(&UPLOADER).next()?.text().next()?.trim();
            (text != "(Disowned)").then_some(text.to_string())
        }

        fn parse_image_count(e: ElementRef) -> Option<u32> {
            let text = e.select(&IMAGE_COUNT).next()?.text().next()?.trim();
            text.split_whitespace().next()?.parse::<u32>().ok()
        }

        fn parse_favorited_date(e: ElementRef) -> Option<DateTime<Utc>> {
            let text = e.select(&FAVORITED_DATE).next()?.text().next()?.trim();
            parse_date(text)
        }

        fn parse_tags(e: ElementRef) -> Option<Vec<GalleryTag>> {
            e.select(&TAGS)
                .map(|e| e.value().attr("title").and_then(parse_tag_text))
                .collect()
        }

        let url = parse_url(e).ok_or(Error::InvalidHTML("URL".to_string()))?;
        let (gid, token) = parse_gid_token(&url).ok_or(Error::InvalidHTML("gid token".to_string()))?;

        let title = parse_title(e).ok_or(Error::InvalidHTML("title".to_string()))?;
        let thumbnail_url = parse_thumbnail_url(e).ok_or(Error::InvalidHTML("thumbnail url".to_string()))?;
        let category = parse_category(e).ok_or(Error::InvalidHTML("category".to_string()))?;
        let posted_date = parse_posted_date(e).ok_or(Error::InvalidHTML("posted date".to_string()))?;
        let favorited_category_index = parse_favorited_category_index(e);
        let favorited_category_name = parse_favorited_category_name(e);
        let rating = parse_coarse_rating(e).ok_or(Error::InvalidHTML("rating".to_string()))?;
        let uploader = parse_uploader(e);
        let image_count = parse_image_count(e).ok_or(Error::InvalidHTML("image count".to_string()))?;
        let favorited_date = parse_favorited_date(e);

        let tags = parse_tags(e).ok_or(Error::InvalidHTML("tags".to_string()))?;

        Ok(Gallery {
            gid,
            token,
            title,
            url,
            thumbnail_url,
            category,
            uploader,
            rating,
            image_count,
            tags,
            posted_date,
            favorited_date,
            favorited_category_index,
            favorited_category_name,
        })
    }

    // Check if the gallery list is empty
    let empty = if let Some(text) = doc.select(&GALLERY).next().and_then(|e| e.text().next()) {
        text.starts_with("No unfiltered results found.")
    } else {
        false
    };

    let galleries = if empty {
        Vec::new()
    } else {
        doc.select(&GALLERY).map(parse_gallery).collect::<Result<_>>()?
    };
    let total_count = parse_gallery_count(doc);

    let get_url = |selector: &Selector| doc.select(selector).next()?.value().attr("href").map(|s| s.to_string());
    let first_page_offset = get_url(&LINK_TO_FIRST).as_deref().and_then(parse_list_offset);
    let prev_page_offset = get_url(&LINK_TO_PREV).as_deref().and_then(parse_list_offset);
    let next_page_offset = get_url(&LINK_TO_NEXT).as_deref().and_then(parse_list_offset);
    let last_page_offset = get_url(&LINK_TO_LAST).as_deref().and_then(parse_list_offset);

    let favorite_categories = parse_favorite_categories(doc);

    Ok(GalleryListResult {
        galleries,
        total_count,
        first_page_offset,
        prev_page_offset,
        next_page_offset,
        last_page_offset,
        favorite_categories,
    })
}

pub fn parse_gallery_page(doc: &Html) -> Result<GalleryPageResult> {
    use super::selectors::gallery::*;

    fn parse_thumbnail_url(doc: &Html) -> Option<String> {
        let style = doc.select(&THUMBNAIL_URL).next()?.value().attr("style")?;
        let start = style.find("(")?;
        let end = style.find(")")?;
        Some(style[start + 1..end].to_string())
    }

    fn parse_title(doc: &Html) -> Option<String> {
        Some(doc.select(&TITLE).next()?.text().next()?.trim().to_string())
    }

    fn parse_english_title(doc: &Html) -> Option<String> {
        Some(doc.select(&ENGLISH_TITLE).next()?.text().next()?.trim().to_string())
    }

    fn parse_category(doc: &Html) -> Option<GalleryCategory> {
        let text = doc.select(&CATEGORY).next()?.text().next()?.trim();
        GalleryCategory::from_str(text).ok()
    }

    fn parse_uploader(doc: &Html) -> Option<String> {
        let text = doc.select(&UPLOADER).next()?.text().next()?.trim();
        (text != "(Disowned)").then_some(text.to_string())
    }

    fn parse_posted_date(doc: &Html) -> Option<DateTime<Utc>> {
        let text = doc.select(&POSTED_DATE).next()?.text().next()?.trim();
        parse_date(text)
    }

    fn parse_parent(doc: &Html) -> Option<String> {
        let text = doc.select(&PARENT).next()?.text().next()?.trim();
        (text != "None").then_some(text.to_string())
    }

    fn parse_visible(doc: &Html) -> Option<bool> {
        let text = doc.select(&VISIBLE).next()?.text().next()?.trim();
        Some(text == "Yes")
    }

    fn parse_language(doc: &Html) -> Option<String> {
        let text = doc.select(&LANGUAGE).next()?.text().next()?.trim();
        Some(text.split_whitespace().next()?.to_string())
    }

    fn parse_file_size(doc: &Html) -> Option<u32> {
        let text = doc.select(&FILE_SIZE).next()?.text().next()?.trim();
        parse_file_size_text(text)
    }

    fn parse_image_count(doc: &Html) -> Option<u32> {
        let text = doc.select(&IMAGE_COUNT).next()?.text().next()?.trim();
        text.split_whitespace().next()?.parse::<u32>().ok()
    }

    fn parse_favorited_count(doc: &Html) -> Option<u32> {
        let text = doc.select(&FAVORITED_COUNT).next()?.text().next()?.trim();
        if text == "Never" {
            return Some(0);
        } else if text == "Once" {
            return Some(1);
        }
        text.split_whitespace().next()?.parse::<u32>().ok()
    }

    fn parse_rating(doc: &Html) -> Option<f32> {
        let text = doc.select(&RATING).next()?.text().next()?.trim();
        text.split_whitespace().next_back()?.parse::<f32>().ok()
    }

    fn parse_rating_count(doc: &Html) -> Option<u32> {
        let text = doc.select(&RATING_COUNT).next()?.text().next()?.trim();
        text.parse::<u32>().ok()
    }

    fn parse_favorited_category_index(doc: &Html) -> Option<u32> {
        const PREFIX: &str = "background-position:0px -";
        let style = doc.select(&FAVORITE_CATEGORY).next()?.value().attr("style")?.trim();
        let start = style.find(PREFIX)?;
        let end = style.find("px;")?;
        let num = style[start + PREFIX.len()..end].parse::<u32>().ok()?;
        Some((num - 2) / 19)
    }

    fn parse_favorited_category_name(doc: &Html) -> Option<String> {
        Some(
            doc.select(&FAVORITE_CATEGORY)
                .next()?
                .value()
                .attr("title")?
                .to_string(),
        )
    }

    fn parse_tags(doc: &Html) -> Option<Vec<GalleryTag>> {
        fn parse_tag(s: &str) -> Option<GalleryTag> {
            let mut parts = s.split('\'');
            let raw = parts.nth(1)?;
            parse_tag_text(raw)
        }

        doc.select(&TAGS)
            .map(|e| e.value().attr("onclick").and_then(parse_tag))
            .collect()
    }

    fn parse_url(doc: &Html) -> Option<String> {
        Some(doc.select(&URL).next()?.value().attr("href")?.to_string())
    }

    fn parse_preview_page_count(doc: &Html) -> Option<u32> {
        let text = doc.select(&PREVIEW_PAGE_COUNT).next()?.text().next()?.trim();
        text.split_whitespace().next()?.parse::<u32>().ok()
    }

    fn parse_previews(doc: &Html) -> Option<Vec<ImagePreview>> {
        fn parse_preview(e: ElementRef) -> Option<ImagePreview> {
            let url = e.value().attr("href")?.to_string();
            let token = url.strip_prefix("https://")?.split('/').nth(2)?.to_string();
            let page = url.split('-').next_back()?.parse::<u32>().ok()?;

            // Change: image preview is now inside `a > div > div`
            let img_div = e.select(&PREVIEW_THUMBNAIL_URL).next()?.value();
            let title_str = img_div.attr("title")?;
            let start = title_str.find(": ")? + 2;
            let filename = title_str[start..].to_string();

            // Extract thumbnail URL from style `background: url(...)`
            let style_str = img_div.attr("style")?;
            let start = style_str.find("url(")? + 4;
            let end = style_str.rfind(")")?;
            let thumbnail_url = style_str[start..end].to_string();

            Some(ImagePreview {
                index: page - 1,
                token,
                url,
                thumbnail_url,
                filename,
            })
        }

        doc.select(&PREVIEWS).map(parse_preview).collect()
    }

    let thumbnail_url = parse_thumbnail_url(doc).ok_or(Error::InvalidHTML("thumbnail url".to_string()))?;
    let title = parse_title(doc)
        .or(parse_english_title(doc))
        .ok_or(Error::InvalidHTML("title".to_string()))?;
    let english_title = parse_english_title(doc).ok_or(Error::InvalidHTML("english title".to_string()))?;

    let category = parse_category(doc).ok_or(Error::InvalidHTML("category".to_string()))?;
    let uploader = parse_uploader(doc);
    let posted_date = parse_posted_date(doc).ok_or(Error::InvalidHTML("posted date".to_string()))?;
    let parent = parse_parent(doc);
    let visible = parse_visible(doc).ok_or(Error::InvalidHTML("visible".to_string()))?;
    let language = parse_language(doc).ok_or(Error::InvalidHTML("language".to_string()))?;
    let file_size = parse_file_size(doc).ok_or(Error::InvalidHTML("file size".to_string()))?;
    let image_count = parse_image_count(doc).ok_or(Error::InvalidHTML("image count".to_string()))?;

    let favorited_count = parse_favorited_count(doc).ok_or(Error::InvalidHTML("favorited count".to_string()))?;
    let rating = parse_rating(doc).ok_or(Error::InvalidHTML("rating".to_string()))?;
    let rating_count = parse_rating_count(doc).ok_or(Error::InvalidHTML("rating count".to_string()))?;
    let favorited_category_index = parse_favorited_category_index(doc);
    let favorited_category_name = parse_favorited_category_name(doc);

    let url = parse_url(doc).ok_or(Error::InvalidHTML("url".to_string()))?;
    let (gid, token) = parse_gid_token(&url).ok_or(Error::InvalidHTML("gid token".to_string()))?;
    let tags = parse_tags(doc).ok_or(Error::InvalidHTML("tags".to_string()))?;

    let preview_page_count =
        parse_preview_page_count(doc).ok_or(Error::InvalidHTML("preview page count".to_string()))?;
    let previews = parse_previews(doc).ok_or(Error::InvalidHTML("previews".to_string()))?;

    let gallery = Gallery {
        gid,
        token,
        title,
        url,
        thumbnail_url,
        category,
        uploader,
        rating,
        image_count,
        tags,
        posted_date,
        favorited_date: None,
        favorited_category_index,
        favorited_category_name,
    };
    let detail = GalleryDetail {
        english_title,
        parent,
        visible,
        language,
        file_size,
        favorited_count,
        rating_count,
    };
    Ok(GalleryPageResult {
        gallery,
        detail,
        preview_page_count,
        previews,
    })
}

pub fn parse_image_page(doc: &Html) -> Result<ImageResult> {
    use super::selectors::image::*;

    fn parse_page_number(doc: &Html) -> Option<u32> {
        let text = doc.select(&PAGE_NUMBER).next()?.text().next()?.trim();
        text.split_whitespace().next()?.parse::<u32>().ok()
    }

    fn parse_url(doc: &Html) -> Option<String> {
        Some(doc.select(&URL).next()?.value().attr("src")?.to_string())
    }

    fn parse_file_info(doc: &Html) -> Option<(String, u32, u32, u32)> {
        let text = doc.select(&FILE_INFO).next()?.text().next()?.trim();
        let mut parts = text.split(" :: ");
        let filename = parts.next()?.to_string();
        let resolution = parts.next()?;
        let file_size = parts.next()?;
        let mut parts = resolution.split(" x ");
        let width = parts.next()?.parse::<u32>().ok()?;
        let height = parts.next()?.parse::<u32>().ok()?;
        let file_size = parse_file_size_text(file_size)?;
        Some((filename, file_size, width, height))
    }

    let page_number = parse_page_number(doc).ok_or(Error::InvalidHTML("page number".to_string()))?;
    let url = parse_url(doc).ok_or(Error::InvalidHTML("url".to_string()))?;
    let (filename, file_size, width, height) =
        parse_file_info(doc).ok_or(Error::InvalidHTML("file info".to_string()))?;

    Ok(ImageResult {
        index: page_number - 1,
        url,
        filename,
        file_size,
        width,
        height,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_gallery_list() {
        let html = std::fs::read_to_string("log/html/favorite.html").unwrap();
        let doc = Html::parse_document(&html);
        let result = parse_gallery_list(&doc).unwrap();
        println!("{:?}", result);
    }

    #[test]
    fn test_parse_gallery_page() {
        let html = std::fs::read_to_string("log/html/gallery1.html").unwrap();
        let doc = Html::parse_document(&html);
        let result = parse_gallery_page(&doc).unwrap();
        println!("{:?}", result);
    }

    #[test]
    fn test_parse_image_page() {
        let html = std::fs::read_to_string("log/html/image.html").unwrap();
        let doc = Html::parse_document(&html);
        let result = parse_image_page(&doc).unwrap();
        println!("{:?}", result);
    }
}
