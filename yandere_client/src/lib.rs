mod error;
mod result;

use reqwest::Url;

use bottle_util::build_params;

pub use crate::error::Error;
use crate::error::Result;
pub use crate::result::*;

const BASE_URL: &str = "https://yande.re";

pub async fn fetch_posts(query: &str, page: u32) -> Result<APIResult> {
    let params = build_params! {
        required api_version => 2,
        required tags => query,
        required page,
        required limit => 100,
        required include_tags => 1,
        required include_pools => 1
    };
    let url = Url::parse_with_params(&format!("{}/post.json", BASE_URL), &params)?;

    let response = reqwest::get(url).await?.error_for_status()?;
    let content = response.text().await?;

    log(query, &content).await?;
    let result: APIResult = serde_json::from_str(&content)?;
    Ok(result)
}

async fn log(name: &str, content: &str) -> Result<()> {
    use std::path::PathBuf;
    use tokio::{fs::File, io::AsyncWriteExt};

    if let Ok(dir) = std::env::var("CLIENT_LOG_DIR") {
        let name = name.replace(':', "_");
        let time = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filepath = PathBuf::from(dir).join(format!("yandere_{}_{}.json", name, time));
        let mut file = File::create(filepath).await?;
        file.write_all(content.as_bytes()).await?;
    }
    Ok(())
}
