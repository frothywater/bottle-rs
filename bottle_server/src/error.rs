use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use bottle_core::Error as BottleError;

pub type Result<T> = std::result::Result<T, ServerError>;

#[derive(Debug)]
pub struct ServerError(anyhow::Error);

impl<E> From<E> for ServerError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        tracing::error!("{}", self);
        let status = self.status_code();
        (status, self.to_string()).into_response()
    }
}

impl ServerError {
    fn status_code(&self) -> StatusCode {
        let err = &self.0;
        for cause in err.chain() {
            if let Some(err) = cause.downcast_ref::<bottle_download::Error>() {
                match err {
                    bottle_download::Error::InvalidUrl(_) => return StatusCode::BAD_REQUEST,
                    bottle_download::Error::NetworkError(_) => return StatusCode::BAD_GATEWAY,
                    bottle_download::Error::IncompleteDownload(_) => return StatusCode::BAD_GATEWAY,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            if let Some(err) = cause.downcast_ref::<twitter_client::Error>() {
                match err {
                    twitter_client::Error::InvalidGraphqlResponse => return StatusCode::BAD_GATEWAY,
                    twitter_client::Error::InvalidCookie(_) => return StatusCode::BAD_REQUEST,
                    twitter_client::Error::InvalidEndpoint(_) => return StatusCode::BAD_REQUEST,
                    twitter_client::Error::NetworkError(_) => return StatusCode::BAD_GATEWAY,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            if let Some(err) = cause.downcast_ref::<pixiv_client::Error>() {
                match err {
                    pixiv_client::Error::InvalidField(_) => return StatusCode::BAD_REQUEST,
                    pixiv_client::Error::NetworkError(_) => return StatusCode::BAD_GATEWAY,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            if let Some(err) = cause.downcast_ref::<yandere_client::Error>() {
                match err {
                    yandere_client::Error::NetworkError(_) => return StatusCode::BAD_GATEWAY,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            if let Some(err) = cause.downcast_ref::<panda_client::Error>() {
                match err {
                    panda_client::Error::RateLimit(_) => return StatusCode::TOO_MANY_REQUESTS,
                    panda_client::Error::NetworkError(_) => return StatusCode::BAD_GATEWAY,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
            if let Some(err) = cause.downcast_ref::<BottleError>() {
                match err {
                    BottleError::ObjectNotFound(_) => return StatusCode::NOT_FOUND,
                    BottleError::ObjectAlreadyExists(_) => return StatusCode::CONFLICT,
                    BottleError::ObjectNotComplete(_) => return StatusCode::BAD_REQUEST,
                    BottleError::InvalidEndpoint(_) => return StatusCode::BAD_REQUEST,
                    BottleError::NotLoggedIn(_) => return StatusCode::UNAUTHORIZED,
                    BottleError::RateLimit(_) => return StatusCode::TOO_MANY_REQUESTS,
                    BottleError::Timeout(_) => return StatusCode::GATEWAY_TIMEOUT,
                    _ => return StatusCode::INTERNAL_SERVER_ERROR,
                }
            }
        }
        StatusCode::INTERNAL_SERVER_ERROR
    }

    pub fn retryable(&self) -> bool {
        let status = self.status_code();
        matches!(status, StatusCode::BAD_GATEWAY | StatusCode::GATEWAY_TIMEOUT)
    }
}
