use serde_json::Value;
use thiserror::Error;

/// A non-success HTTP response returned by OpenCode.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    pub body: String,
    pub json: Option<Value>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpenCode API returned HTTP {}", self.status)?;
        if !self.body.is_empty() {
            write!(f, ": {}", self.body)?;
        }
        Ok(())
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(#[from] url::ParseError),
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error("SSE protocol error: {0}")]
    Sse(String),
    #[error("response decode error: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("not available on this OpenCode server: {0}")]
    Unsupported(&'static str),
}
