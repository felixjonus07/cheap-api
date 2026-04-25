use thiserror::Error;

// All the ways CheapAPI can fail. Each variant has a human-readable message.
#[derive(Debug, Error)]
pub enum CheapApiError {
    // Something went wrong reading/writing to the cache (e.g. MongoDB error)
    #[error("cache store error: {0}")]
    Store(String),

    // The HTTP request to the upstream API failed
    #[error("upstream HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    // We couldn't parse or build a JSON value
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    // A header or body value had invalid bytes we couldn't encode
    #[error("encoding error: {0}")]
    Encoding(String),

    // The caller passed in bad configuration (e.g. wrong URL, empty string)
    #[error("invalid configuration: {0}")]
    Config(String),
}

// A short alias so we don't have to write Result<T, CheapApiError> everywhere
pub type Result<T> = std::result::Result<T, CheapApiError>;
