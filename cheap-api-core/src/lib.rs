// cheap-api-core
//
// The Rust engine that powers the CheapAPI SDK.
//
// How it works (in plain English):
//   1. A language wrapper (Node.js or Python) sends us an HTTP request to make.
//   2. We hash the request (URL + method + body) to get a unique cache key.
//   3. We check the cache (MongoDB). If it's there, we return it immediately.
//   4. If it's not cached, we forward the request to the real API.
//   5. We save the response to the cache so next time is instant.
//
// The goal is to save money by never making the same API call twice.

pub mod bindings;
pub mod error;
pub mod hasher;
pub mod interceptor;
pub mod mongo_adapter;
pub mod store;

// Re-export the most important types so callers don't need to dig into submodules
pub use error::{CheapApiError, Result};
pub use interceptor::{InterceptRequest, InterceptResponse, Interceptor, InterceptorConfig};
pub use mongo_adapter::{MongoAdapter, MongoAdapterConfig};
pub use store::{CacheStore, CachedResponse};
