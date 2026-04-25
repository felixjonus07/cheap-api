use crate::error::Result;
use async_trait::async_trait;

// This struct holds a saved HTTP response.
// We store everything we need to replay it later without hitting the API again.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedResponse {
    pub status: u16,                                     // e.g. 200, 404
    pub headers: std::collections::HashMap<String, String>, // response headers
    pub body: String,                                    // the response body text
    pub cached_at: u64,                                  // when it was saved (Unix seconds)
}

// This trait is the contract that every cache backend must follow.
// Right now we only have MongoDB, but this makes it easy to add Redis, SQLite, etc.
//
// All methods take &self (not &mut self) so the store can be safely
// shared between multiple async tasks at the same time.
#[async_trait]
pub trait CacheStore: Send + Sync + 'static {
    // Look up a key. Returns Some(response) on a hit, None on a miss.
    async fn get(&self, key: &str) -> Result<Option<CachedResponse>>;

    // Save a response. If the key already exists, overwrite it.
    async fn set(&self, key: &str, value: &CachedResponse) -> Result<()>;

    // Delete one entry. It's fine if the key doesn't exist — just do nothing.
    async fn delete(&self, key: &str) -> Result<()>;

    // Delete every entry in this store.
    async fn flush(&self) -> Result<()>;
}
