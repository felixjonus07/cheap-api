use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tracing::{debug, info, warn};

use crate::error::{CheapApiError, Result};
use crate::hasher::compute_cache_key;
use crate::store::{CacheStore, CachedResponse};

// ── Request and Response types ──────────────────────────────────────────────

// Represents a request coming in from Node.js or Python.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InterceptRequest {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: String,
}

// The response we send back to Node.js or Python — either from cache or live.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InterceptResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub from_cache: bool,  // true = served from cache, false = hit the real API
    pub cache_key: String, // the SHA-256 key (handy for debugging)
}

// ── Configuration ───────────────────────────────────────────────────────────

// Settings you can tweak when creating an Interceptor.
#[derive(Debug, Clone)]
pub struct InterceptorConfig {
    // Should we cache error responses (4xx, 5xx)? Default: no.
    pub cache_errors: bool,
    // Don't cache responses larger than this. Default: 10 MB.
    pub max_cacheable_body_bytes: usize,
}

impl Default for InterceptorConfig {
    fn default() -> Self {
        Self {
            cache_errors: false,
            max_cacheable_body_bytes: 10 * 1024 * 1024, // 10 MB
        }
    }
}

// ── Interceptor ─────────────────────────────────────────────────────────────

// The main struct. It holds the cache store, an HTTP client, and the config.
#[derive(Clone)]
pub struct Interceptor {
    store: Arc<dyn CacheStore>,
    client: reqwest::Client,
    config: InterceptorConfig,
}

impl Interceptor {
    // Create a new interceptor. Pass in your cache backend and config.
    pub fn new(store: Arc<dyn CacheStore>, config: InterceptorConfig) -> Self {
        // Build a reusable HTTP client (follows redirects, 60s timeout)
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build HTTP client");

        Self {
            store,
            client,
            config,
        }
    }

    // The main function — called once per API request from the language wrapper.
    // It handles the full cache-check → forward → store flow.
    pub async fn intercept(&self, req: InterceptRequest) -> Result<InterceptResponse> {
        // Step 1: Hash the request to get a unique cache key
        let cache_key = compute_cache_key(&req.url, &req.method, req.body.as_bytes());
        debug!(cache_key = %cache_key, url = %req.url, "computed cache key");

        // Step 2: Check if we already have this response in cache
        if let Some(cached) = self.store.get(&cache_key).await? {
            info!(cache_key = %cache_key, "CACHE HIT");
            self.print_token_usage(&cached.body, true);

            return Ok(InterceptResponse {
                status: cached.status,
                headers: cached.headers,
                body: cached.body,
                from_cache: true,
                cache_key,
            });
        }

        debug!(cache_key = %cache_key, "cache miss — calling upstream API");

        // Step 3: Forward the request to the real upstream API
        let live_result = self.forward_to_upstream(&req).await;

        // Quota Shield: if OpenAI says we're out of quota, return a fake response
        // instead of crashing the caller's app.
        if let Ok(ref live) = live_result {
            if live.status == 429 && live.body.contains("insufficient_quota") {
                warn!("OpenAI quota exceeded (429). Returning Quota Shield response.");

                let fake_body = r#"{"choices": [{"message": {"role": "assistant", "content": "[QUOTA SHIELD] Your prompt was intercepted. OpenAI returned 429, but CheapAPI is keeping your app alive."}}]}"#.to_string();

                return Ok(InterceptResponse {
                    status: 200,
                    headers: [("Content-Type".to_owned(), "application/json".to_owned())].into(),
                    body: fake_body,
                    from_cache: false,
                    cache_key: "QUOTA_SHIELD_ACTIVE".to_string(),
                });
            }
        }

        let live = live_result?;

        // Step 4: Save the response to cache (if it's worth saving)
        if self.should_cache(live.status, live.body.len()) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let entry = CachedResponse {
                status: live.status,
                headers: live.headers.clone(),
                body: live.body.clone(),
                cached_at: now,
            };

            match self.store.set(&cache_key, &entry).await {
                Ok(()) => info!(cache_key = %cache_key, "saved response to cache"),
                Err(e) => warn!(err = %e, "could not save to cache (non-fatal)"),
            }
        }

        // Step 5: Print token usage to terminal and return the live response
        self.print_token_usage(&live.body, false);

        Ok(InterceptResponse {
            status: live.status,
            headers: live.headers,
            body: live.body,
            from_cache: false,
            cache_key,
        })
    }

    // Print token usage to the terminal so the user can see how many tokens were used.
    // Supports OpenAI/Anthropic format and Google Gemini format.
    fn print_token_usage(&self, body: &str, from_cache: bool) {
        // If we can't parse the body as JSON, just skip — it might not have token info
        let json: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return,
        };

        let source = if from_cache { "DATABASE" } else { "LIVE" };

        // OpenAI / Anthropic style: { usage: { prompt_tokens, completion_tokens } }
        if let Some(usage) = json.get("usage") {
            let prompt = usage
                .get("prompt_tokens")
                .and_then(|t| t.as_u64())
                .or_else(|| usage.get("input_tokens").and_then(|t| t.as_u64()));

            let completion = usage
                .get("completion_tokens")
                .and_then(|t| t.as_u64())
                .or_else(|| usage.get("output_tokens").and_then(|t| t.as_u64()));

            let total = usage.get("total_tokens").and_then(|t| t.as_u64());

            if let (Some(p), Some(c)) = (prompt, completion) {
                let total = total.unwrap_or(p + c);
                println!(
                    "\x1b[1;36m[CheapAPI]\x1b[0m \x1b[1;33m[{}]\x1b[0m Usage: {} prompt + {} completion = {} total tokens",
                    source, p, c, total
                );
                return;
            }
        }

        // Google Gemini style: { usageMetadata: { promptTokenCount, candidatesTokenCount } }
        if let Some(usage) = json.get("usageMetadata") {
            let prompt = usage.get("promptTokenCount").and_then(|t| t.as_u64());
            let completion = usage.get("candidatesTokenCount").and_then(|t| t.as_u64());
            let total = usage.get("totalTokenCount").and_then(|t| t.as_u64());

            if let (Some(p), Some(c)) = (prompt, completion) {
                let total = total.unwrap_or(p + c);
                println!(
                    "\x1b[1;36m[CheapAPI]\x1b[0m \x1b[1;33m[{}]\x1b[0m Usage: {} prompt + {} completion = {} total tokens",
                    source, p, c, total
                );
            }
        }
    }

    // Send the request to the real upstream API and collect the response.
    async fn forward_to_upstream(&self, req: &InterceptRequest) -> Result<InterceptResponse> {
        // Parse the HTTP method (e.g. "POST" → reqwest::Method::POST)
        let method = reqwest::Method::from_bytes(req.method.to_ascii_uppercase().as_bytes())
            .map_err(|_| CheapApiError::Config(format!("invalid HTTP method: {}", req.method)))?;

        let mut builder = self.client.request(method, &req.url);

        // Add all headers from the original request
        for (key, val) in &req.headers {
            let header_name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| CheapApiError::Encoding(e.to_string()))?;
            let header_value =
                HeaderValue::from_str(val).map_err(|e| CheapApiError::Encoding(e.to_string()))?;
            builder = builder.header(header_name, header_value);
        }

        // Attach body if there is one
        if !req.body.is_empty() {
            builder = builder.body(req.body.clone());
        }

        // Send the request and read the response
        let response = builder.send().await?;
        let status = response.status().as_u16();
        let headers = Self::collect_response_headers(response.headers());
        let body = response.text().await?;

        Ok(InterceptResponse {
            status,
            headers,
            body,
            from_cache: false,
            cache_key: String::new(), // filled in by the caller
        })
    }

    // Decide whether a response is worth saving to cache.
    fn should_cache(&self, status: u16, body_len: usize) -> bool {
        // Skip if the body is too large
        if body_len > self.config.max_cacheable_body_bytes {
            return false;
        }
        // Skip error responses unless the user opted in
        if !self.config.cache_errors && !(200..300).contains(&status) {
            return false;
        }
        true
    }

    // Convert reqwest's HeaderMap into a simple HashMap<String, String>.
    // If there are multiple values for the same header, join them with ", ".
    fn collect_response_headers(map: &HeaderMap) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for (name, value) in map {
            let value_str = value.to_str().unwrap_or("").to_owned();
            result
                .entry(name.as_str().to_owned())
                .and_modify(|existing: &mut String| {
                    existing.push_str(", ");
                    existing.push_str(&value_str);
                })
                .or_insert(value_str);
        }
        result
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::CachedResponse;
    use async_trait::async_trait;
    use std::sync::Mutex;

    // A simple in-memory store for tests — no database needed.
    struct MemoryStore(Mutex<HashMap<String, CachedResponse>>);

    impl MemoryStore {
        fn new() -> Arc<Self> {
            Arc::new(Self(Mutex::new(HashMap::new())))
        }
    }

    #[async_trait]
    impl CacheStore for MemoryStore {
        async fn get(&self, key: &str) -> crate::error::Result<Option<CachedResponse>> {
            Ok(self.0.lock().unwrap().get(key).cloned())
        }

        async fn set(&self, key: &str, value: &CachedResponse) -> crate::error::Result<()> {
            self.0.lock().unwrap().insert(key.to_owned(), value.clone());
            Ok(())
        }

        async fn delete(&self, key: &str) -> crate::error::Result<()> {
            self.0.lock().unwrap().remove(key);
            Ok(())
        }

        async fn flush(&self) -> crate::error::Result<()> {
            self.0.lock().unwrap().clear();
            Ok(())
        }
    }

    fn make_interceptor() -> Interceptor {
        Interceptor::new(MemoryStore::new(), InterceptorConfig::default())
    }

    #[test]
    fn should_not_cache_large_bodies() {
        let ix = make_interceptor();
        assert!(!ix.should_cache(200, ix.config.max_cacheable_body_bytes + 1));
    }

    #[test]
    fn should_not_cache_error_responses_by_default() {
        let ix = make_interceptor();
        assert!(!ix.should_cache(500, 100));
        assert!(!ix.should_cache(429, 100));
    }

    #[test]
    fn should_cache_successful_responses() {
        let ix = make_interceptor();
        assert!(ix.should_cache(200, 100));
        assert!(ix.should_cache(201, 100));
        assert!(ix.should_cache(204, 100));
    }

    #[test]
    fn should_cache_errors_when_flag_is_set() {
        let ix = Interceptor::new(
            MemoryStore::new(),
            InterceptorConfig {
                cache_errors: true,
                ..Default::default()
            },
        );
        assert!(ix.should_cache(500, 100));
        assert!(ix.should_cache(429, 100));
    }
}
