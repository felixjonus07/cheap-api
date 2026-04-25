use std::collections::HashMap;

use async_trait::async_trait;
use mongodb::{
    bson::{self, doc, Document},
    options::{ClientOptions, IndexOptions, ReplaceOptions},
    Client, Collection, IndexModel,
};
use tracing::{debug, instrument};

use crate::error::{CheapApiError, Result};
use crate::store::{CacheStore, CachedResponse};

// ── Configuration ────────────────────────────────────────────────────────────

// All the settings needed to connect to MongoDB.
// You can use the defaults or customize each field.
#[derive(Debug, Clone)]
pub struct MongoAdapterConfig {
    // Full MongoDB connection string. Read this from an env variable — don't hardcode it.
    pub connection_uri: String,

    // The database where the cache collection will live.
    pub database: String,

    // The name of the collection (like a table in SQL).
    pub collection: String,

    // How long to keep cached entries (in seconds).
    // Set to None to keep them forever.
    // Example: Some(86_400) = keep for 24 hours.
    pub ttl_seconds: Option<u32>,
}

impl Default for MongoAdapterConfig {
    fn default() -> Self {
        Self {
            connection_uri: "mongodb://localhost:27017".into(),
            database: "cheap_api".into(),
            collection: "response_cache".into(),
            ttl_seconds: None,
        }
    }
}

// ── Internal document shape ───────────────────────────────────────────────

// This is what actually gets stored in MongoDB.
// The _id field is the SHA-256 cache key, so lookups are fast.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheDocument {
    #[serde(rename = "_id")]
    id: String,

    status: u16,

    headers: HashMap<String, String>,

    body: String,

    // MongoDB needs a BSON DateTime for TTL expiry to work.
    // We convert from/to Unix seconds when reading/writing.
    cached_at: bson::DateTime,
}

impl CacheDocument {
    // Build a CacheDocument from a cache key and a CachedResponse
    fn from_entry(key: &str, r: &CachedResponse) -> Self {
        // Convert Unix seconds to milliseconds (BSON DateTime uses ms)
        let millis = (r.cached_at as i64).saturating_mul(1_000);
        Self {
            id: key.to_owned(),
            status: r.status,
            headers: r.headers.clone(),
            body: r.body.clone(),
            cached_at: bson::DateTime::from_millis(millis),
        }
    }
}

// Convert a CacheDocument back into a CachedResponse
impl From<CacheDocument> for CachedResponse {
    fn from(doc: CacheDocument) -> Self {
        // Convert ms back to seconds, and clamp to 0 if the value is negative
        let cached_at = (doc.cached_at.timestamp_millis() / 1_000).max(0) as u64;
        Self {
            status: doc.status,
            headers: doc.headers,
            body: doc.body,
            cached_at,
        }
    }
}

// ── MongoAdapter ─────────────────────────────────────────────────────────────

// The MongoDB-backed cache. Implements CacheStore so the Interceptor can use it.
#[derive(Clone)]
pub struct MongoAdapter {
    col: Collection<Document>,
}

impl MongoAdapter {
    // Connect to MongoDB and make sure indexes are set up.
    // Call this once when your app starts up.
    pub async fn connect(cfg: MongoAdapterConfig) -> Result<Self> {
        let options = ClientOptions::parse(&cfg.connection_uri)
            .await
            .map_err(|e| CheapApiError::Store(format!("could not parse URI: {e}")))?;

        let client = Client::with_options(options)
            .map_err(|e| CheapApiError::Store(format!("could not create MongoDB client: {e}")))?;

        let col = client
            .database(&cfg.database)
            .collection::<Document>(&cfg.collection);

        // Create a TTL index if the user wants entries to expire automatically
        Self::setup_indexes(&col, cfg.ttl_seconds).await?;

        Ok(Self { col })
    }

    // Create a TTL index on the cached_at field so MongoDB can auto-delete old entries.
    // It's safe to call this every time — MongoDB ignores it if the index already exists.
    async fn setup_indexes(col: &Collection<Document>, ttl: Option<u32>) -> Result<()> {
        if let Some(seconds) = ttl {
            let opts = IndexOptions::builder()
                .expire_after(std::time::Duration::from_secs(seconds as u64))
                .build();

            let model = IndexModel::builder()
                .keys(doc! { "cached_at": 1 })
                .options(opts)
                .build();

            col.create_index(model)
                .await
                .map_err(|e| CheapApiError::Store(format!("could not create TTL index: {e}")))?;
        }
        Ok(())
    }

    // Helper: wrap a MongoDB/BSON error in our error type with context
    fn store_error(context: &str, e: impl std::fmt::Display) -> CheapApiError {
        CheapApiError::Store(format!("{context}: {e}"))
    }

    // Convert a CachedResponse into a BSON document for storage
    fn to_bson_doc(key: &str, value: &CachedResponse) -> Result<Document> {
        bson::to_document(&CacheDocument::from_entry(key, value))
            .map_err(|e| Self::store_error("bson serialize", e))
    }

    // Convert a raw BSON document back into a CachedResponse
    fn from_bson_doc(raw: Document) -> Result<CachedResponse> {
        bson::from_document::<CacheDocument>(raw)
            .map(Into::into)
            .map_err(|e| Self::store_error("bson deserialize", e))
    }
}

// ── CacheStore implementation ─────────────────────────────────────────────

#[async_trait]
impl CacheStore for MongoAdapter {
    // Look up a cached response by key. Fast because _id is indexed.
    #[instrument(skip(self), fields(key))]
    async fn get(&self, key: &str) -> Result<Option<CachedResponse>> {
        match self.col.find_one(doc! { "_id": key }).await {
            Ok(Some(raw)) => {
                debug!(key, "cache hit");
                Ok(Some(Self::from_bson_doc(raw)?))
            }
            Ok(None) => {
                debug!(key, "cache miss");
                Ok(None)
            }
            Err(e) => Err(Self::store_error("find_one", e)),
        }
    }

    // Save a response. Uses "upsert" so it works whether the key exists or not.
    #[instrument(skip(self, value), fields(key))]
    async fn set(&self, key: &str, value: &CachedResponse) -> Result<()> {
        let opts = ReplaceOptions::builder().upsert(true).build();

        self.col
            .replace_one(doc! { "_id": key }, Self::to_bson_doc(key, value)?)
            .with_options(opts)
            .await
            .map_err(|e| Self::store_error("replace_one", e))?;

        debug!(key, "upsert successful");
        Ok(())
    }

    // Delete a single entry. Does nothing if the key doesn't exist.
    #[instrument(skip(self), fields(key))]
    async fn delete(&self, key: &str) -> Result<()> {
        self.col
            .delete_one(doc! { "_id": key })
            .await
            .map_err(|e| Self::store_error("delete_one", e))?;

        debug!(key, "delete successful");
        Ok(())
    }

    // Delete everything in this collection.
    #[instrument(skip(self))]
    async fn flush(&self) -> Result<()> {
        self.col
            .delete_many(doc! {})
            .await
            .map_err(|e| Self::store_error("delete_many", e))?;

        debug!("flush successful");
        Ok(())
    }
}
