use std::collections::HashMap;
use thiserror::Error;

use url::Url;

#[derive(Debug, Clone, Error)]
pub enum ParsingError {
    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid key-value list: {0}")]
    InvalidKeyVal(String),
}

type Result<T> = std::result::Result<T, ParsingError>;

pub fn parse_cookie_str(cookie_str: &str) -> Result<HashMap<String, String>> {
    parse_kv_list(cookie_str, ';')
}

pub fn parse_query_str(query_str: &str) -> Result<HashMap<String, String>> {
    parse_kv_list(query_str, '&')
}

/// Parse the filename from a URL.
pub fn parse_filename(url: &str) -> Result<String> {
    let url = Url::parse(url)?;
    url.path_segments()
        .and_then(|segments| segments.last())
        .map(|s| s.to_string())
        .ok_or(ParsingError::InvalidUrl(url.to_string()))
}

/// Parse a list of key-value pairs separated by `sep`. Usually from a query string or cookie string.
fn parse_kv_list(s: &str, sep: char) -> Result<HashMap<String, String>> {
    let mut results = HashMap::new();
    for param in s.split(sep) {
        if param.contains('=') {
            let mut parts = param.splitn(2, '=');
            let key = parts.next().ok_or(ParsingError::InvalidKeyVal(param.to_string()))?;
            let value = parts.next().ok_or(ParsingError::InvalidKeyVal(param.to_string()))?;
            results.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(results)
}
