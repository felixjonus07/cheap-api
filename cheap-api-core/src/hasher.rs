use sha2::{Digest, Sha256};

// This function takes the URL, HTTP method, and request body
// and returns a unique string (cache key) that identifies this exact request.
// We use SHA-256 hashing so the same request always gets the same key.
pub fn compute_cache_key(url: &str, method: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();

    // We uppercase the method so "get" and "GET" produce the same key
    hasher.update(method.to_ascii_uppercase().as_bytes());

    // We use a null byte between parts so "POST/foo" and "POS/Tfoo" can't collide
    hasher.update(b"\x00");
    hasher.update(url.as_bytes());
    hasher.update(b"\x00");
    hasher.update(body);

    // Return the hash as a 64-character hex string
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_produce_same_key() {
        let k1 = compute_cache_key("https://api.openai.com/v1/chat/completions", "POST", b"{\"model\":\"gpt-4\"}");
        let k2 = compute_cache_key("https://api.openai.com/v1/chat/completions", "POST", b"{\"model\":\"gpt-4\"}");
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_bodies_produce_different_keys() {
        let k1 = compute_cache_key("https://api.openai.com/v1/chat/completions", "POST", b"body_a");
        let k2 = compute_cache_key("https://api.openai.com/v1/chat/completions", "POST", b"body_b");
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_urls_produce_different_keys() {
        let k1 = compute_cache_key("https://api.openai.com/v1/chat/completions", "POST", b"same");
        let k2 = compute_cache_key("https://generativelanguage.googleapis.com/v1beta/models", "POST", b"same");
        assert_ne!(k1, k2);
    }

    #[test]
    fn method_case_does_not_matter() {
        let k1 = compute_cache_key("https://example.com/", "get", b"");
        let k2 = compute_cache_key("https://example.com/", "GET", b"");
        assert_eq!(k1, k2);
    }

    #[test]
    fn key_is_64_hex_chars() {
        let key = compute_cache_key("https://example.com/", "POST", b"test");
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
