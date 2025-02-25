mod consts;
mod error;
mod response;
mod result;
#[cfg(test)]
mod test;
mod util;

use reqwest::{header, Client, Response, Url};
use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use consts::*;
use response::{AccountResponse, GraphqlResponse};
pub use result::*;

pub use crate::error::Error;
use crate::error::Result;

use bottle_util::parse_cookie_str;

#[derive(Debug, Clone)]
pub struct SessionCookie {
    pub ct0: String,
    pub auth_token: String,
}

impl Display for SessionCookie {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ct0={}; auth_token={}", self.ct0, self.auth_token)
    }
}

impl FromStr for SessionCookie {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut cookie_map = parse_cookie_str(s)?;
        let ct0 = cookie_map.remove("ct0").ok_or(Error::InvalidCookie(s.to_string()))?;
        let auth_token = cookie_map
            .remove("auth_token")
            .ok_or(Error::InvalidCookie(s.to_string()))?;
        Ok(SessionCookie { ct0, auth_token })
    }
}

#[derive(Debug, Clone)]
pub struct TwitterClient {
    pub session_cookie: SessionCookie,
    client: reqwest::Client,
    default_variables: serde_json::Map<String, Value>,
    default_features: serde_json::Map<String, Value>,
}

impl TwitterClient {
    pub fn new(session_cookie: SessionCookie) -> Result<TwitterClient> {
        let cookie_string = format!("ct0={}; auth_token={}", session_cookie.ct0, session_cookie.auth_token);

        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_static(BEARER_TOKEN));
        headers.insert(header::COOKIE, header::HeaderValue::from_str(&cookie_string).unwrap());
        headers.insert(
            "x-csrf-token",
            header::HeaderValue::from_str(&session_cookie.ct0).unwrap(),
        );
        headers.insert("x-twitter-active-user", header::HeaderValue::from_static("yes"));
        headers.insert("x-twitter-client-language", header::HeaderValue::from_static("en"));
        headers.insert("x-twitter-auth-type", header::HeaderValue::from_static("OAuth2Session"));

        let client = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()?;

        let default_variables = DEFAULT_GRAPHQL_VARIABLES
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_owned().into()))
            .collect();
        let default_features = DEFAULT_GRAPHQL_FEATURES
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_owned().into()))
            .collect();

        Ok(TwitterClient {
            session_cookie,
            client,
            default_variables,
            default_features,
        })
    }

    pub async fn accounts(&self) -> Result<Vec<Account>> {
        let response: AccountResponse = self.rest_get("/account/multi/list.json").await?;
        Ok(response.users)
    }

    pub async fn user_by_id(&self, user_id: u64) -> Result<User> {
        self.graphql_get("UserByRestId", [("userId", user_id)]).await
    }

    pub async fn users_by_ids(&self, user_ids: &[u64]) -> Result<Vec<User>> {
        self.graphql_get("UsersByRestIds", [("userIds", user_ids)]).await
    }

    pub async fn tweet_by_id(&self, tweet_id: u64) -> Result<Tweet> {
        self.graphql_get("TweetResultByRestId", [("tweetId", tweet_id)]).await
    }

    pub async fn user_tweets(&self, user_id: u64, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> =
            [("userId", user_id.into()), ("count", LIST_API_MAX_COUNT.into())].to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("UserTweets", variables).await
    }

    pub async fn user_media(&self, user_id: u64, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> =
            [("userId", user_id.into()), ("count", LIST_API_MAX_COUNT.into())].to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("UserMedia", variables).await
    }

    pub async fn likes(&self, user_id: u64, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> =
            [("userId", user_id.into()), ("count", LIST_API_MAX_COUNT.into())].to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("Likes", variables).await
    }

    pub async fn followers(&self, user_id: u64, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> =
            [("userId", user_id.into()), ("count", LIST_API_MAX_COUNT.into())].to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("Followers", variables).await
    }

    pub async fn following(&self, user_id: u64, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> =
            [("userId", user_id.into()), ("count", LIST_API_MAX_COUNT.into())].to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("Following", variables).await
    }

    pub async fn search(&self, query: &str, cursor: Option<&str>) -> Result<TimelineResult> {
        let mut variables: Vec<(&str, Value)> = [
            ("rawQuery", query.into()),
            ("count", SEARCH_API_MAX_COUNT.into()),
            ("product", "Latest".into()),
            ("querySource", "typed_query".into()),
        ]
        .to_vec();
        if let Some(cursor) = cursor {
            variables.push(("cursor", cursor.into()));
        }
        self.graphql_get("SearchTimeline", variables).await
    }
}

impl TwitterClient {
    async fn rest_get<R>(&self, path: &str) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = Url::parse(&format!("{}{}", REST_API, path))?;
        let response: Response = self.client.get(url).send().await?;
        let response = response.error_for_status()?;
        let content = response.text().await?;

        let name = path.strip_prefix('/').unwrap_or(path).replace('/', "_");
        log(&name, &content).await?;
        serde_json::from_str(&content).map_err(|e| e.into())
    }

    async fn graphql_get<I, V, R>(&self, endpoint: &str, variables: I) -> Result<R>
    where
        I: IntoIterator<Item = (&'static str, V)>,
        V: Into<Value>,
        R: TryFrom<GraphqlResponse, Error = Error>,
    {
        let Some(qid) = GRAPHQL_QIDS.get(endpoint) else {
            return Err(Error::InvalidEndpoint(endpoint.to_string()));
        };

        let mut all_variables = self.default_variables.clone();
        all_variables.extend(variables.into_iter().map(|(k, v)| (k.to_string(), v.into())));
        let variable_str = serde_json::to_string(&all_variables)?;
        let feature_str = serde_json::to_string(&self.default_features)?;
        let graphql_params = [("variables", variable_str), ("features", feature_str)];

        let base_url = format!("{}/{}/{}", GRAPHQL_API, qid, endpoint);
        let url = Url::parse_with_params(&base_url, &graphql_params)?;
        let response: Response = self.client.get(url).send().await?;

        let status_error = response.error_for_status_ref().err();
        let content = response.text().await?;
        log(endpoint, &content).await?;
        if let Some(status_error) = status_error {
            return Err(status_error.into());
        }

        let response: GraphqlResponse = serde_json::from_str(&content)?;
        response.try_into()
    }
}

async fn log(name: &str, content: &str) -> Result<()> {
    use std::path::PathBuf;
    use tokio::{fs::File, io::AsyncWriteExt};

    if let Ok(dir) = std::env::var("CLIENT_LOG_DIR") {
        let time = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filepath = PathBuf::from(dir).join(format!("twitter_{}_{}.json", name, time));
        let mut file = File::create(filepath).await?;
        file.write_all(content.as_bytes()).await?;
    }
    Ok(())
}
