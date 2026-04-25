/**
 * Cheap API SDK - Final JavaScript Wrapper
 * 
 * This file implements Step 4: The Interceptor.
 * It provides the high-level API to initialize the SDK and automatically
 * monkey-patches the global fetch API to route traffic through the Rust core.
 */

const { CheapApi } = require('./index');

let interceptor = null;

/**
 * Initialize the Cheap API SDK.
 * 
 * @param {Object} mongoCfg - MongoDB configuration
 * @param {string} mongoCfg.connectionUri - MongoDB connection URI
 * @param {string} mongoCfg.database - Database name
 * @param {string} mongoCfg.collection - Collection name
 * @param {number} [mongoCfg.ttlSeconds] - Optional TTL for cache entries
 * 
 * @param {Object} [options] - SDK options
 * @param {boolean} [options.cacheErrors=false] - Whether to cache non-2xx responses
 * @param {number} [options.maxCacheableBodyBytes=10485760] - Max body size to cache (10MB default)
 */
async function init(mongoCfg, options = {}) {
  if (interceptor) {
    console.warn('[CheapAPI] SDK is already initialized.');
    return interceptor;
  }

  try {
    interceptor = await CheapApi.withMongodb(mongoCfg, {
      cacheErrors: options.cacheErrors || false,
      maxCacheableBodyBytes: options.maxCacheableBodyBytes || 10 * 1024 * 1024
    });

    patchFetch();
    console.log('[CheapAPI] SDK initialized and global fetch patched.');
    return interceptor;
  } catch (err) {
    console.error('[CheapAPI] Failed to initialize SDK:', err);
    throw err;
  }
}

/**
 * Patch the global fetch API to route requests through the Rust interceptor.
 */
function patchFetch() {
  if (typeof globalThis.fetch !== 'function') {
    // Older Node.js versions might not have globalThis.fetch
    // We could support undici or other polyfills here if needed.
    console.warn('[CheapAPI] globalThis.fetch not found. Fetch interception skipped.');
    return;
  }

  const originalFetch = globalThis.fetch;

  globalThis.fetch = async (input, init = {}) => {
    if (!interceptor) {
      return originalFetch(input, init);
    }

    // 1. Normalize the request components
    let url;
    let method;
    let headers = {};
    let body = null;

    if (typeof input === 'string') {
      url = input;
    } else if (input instanceof URL) {
      url = input.toString();
    } else {
      url = input.url;
      method = input.method;
      // Copy headers from Request object
      for (const [key, value] of input.headers.entries()) {
        headers[key] = value;
      }
    }

    // 2. Overlay init options
    method = init.method || method || 'GET';
    if (init.headers) {
      if (init.headers instanceof Headers) {
        for (const [key, value] of init.headers.entries()) {
          headers[key] = value;
        }
      } else if (Array.isArray(init.headers)) {
        init.headers.forEach(([key, value]) => {
          headers[key] = value;
        });
      } else {
        Object.assign(headers, init.headers);
      }
    }
    body = init.body || body;

    // 3. Route through Rust Interceptor
    try {
      // The Rust side expects body as a string.
      // If it's a Buffer, Blob, or other, we need to convert.
      // For now, we handle basic strings and falls back for complex types.
      let bodyStr = null;
      if (body) {
        if (typeof body === 'string') {
          bodyStr = body;
        } else if (Buffer.isBuffer(body)) {
          bodyStr = body.toString('utf8');
        } else {
          // Fallback for complex bodies (Streams, etc) - bypassing cache for now
          console.debug('[CheapAPI] Complex body detected, bypassing cache.');
          return originalFetch(input, init);
        }
      }

      const rustRes = await interceptor.intercept({
        url,
        method: method.toUpperCase(),
        headers,
        body: bodyStr
      });

      // 4. Return a fetch-compatible Response object
      // This is a native fetch Response, so the user can call .json(), .text(), etc.
      const response = new Response(rustRes.body, {
        status: rustRes.status,
        headers: rustRes.headers
      });

      // Add a custom header to indicate cache hit for transparency
      // Note: Object.defineProperty is used because Response headers are usually immutable-ish
      // in some environments, but native Response allows the headers object to be read.
      // Actually, we should use the headers we passed to the constructor.
      response.headers.set('X-Cheap-API-Cache', rustRes.from_cache ? 'HIT' : 'MISS');
      response.headers.set('X-Cheap-API-Key', rustRes.cache_key);

      return response;
    } catch (err) {
      console.warn('[CheapAPI] Interception error, falling back to network:', err);
      return originalFetch(input, init);
    }
  };
}

/**
 * Automatically initialize the SDK using environment variables.
 * CHEAP_API_MONGO_URI, CHEAP_API_DB, CHEAP_API_COLLECTION
 */
async function autoInit() {
  return init({
    connectionUri: process.env.CHEAP_API_MONGO_URI || 'mongodb://localhost:27017',
    database: process.env.CHEAP_API_DB || 'cheap_api',
    collection: process.env.CHEAP_API_COLLECTION || 'cache'
  }, {
    cacheErrors: process.env.CHEAP_API_CACHE_ERRORS === 'true'
  });
}

module.exports = { init, autoInit };
