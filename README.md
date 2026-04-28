# CheapAPI 🚀

> **Never pay for the same API call twice.**  
> A high-performance, Rust-powered caching layer that transparently intercepts your HTTP requests, stores responses in MongoDB, and replays them instantly — saving you money and reducing latency on every repeated API call.

[![Python](https://img.shields.io/pypi/v/cheap-api?label=pip%20install%20cheap-api&color=blue)](https://pypi.org/project/cheap-api/)
[![npm](https://img.shields.io/npm/v/@cheap-api/node?label=npm%20install%20%40cheap-api%2Fnode&color=red)](https://www.npmjs.com/package/@cheap-api/node)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ✨ Features

- **Universal Support**: Works with **any** HTTP-based API (OpenAI, Gemini, Anthropic, Mistral, or your own custom backends).
- **Quota Shield™**: Automatically detects OpenAI "insufficient_quota" errors and returns a fallback response to keep your app alive.
- **Smart Token Tracking**: Automatically parses and prints token usage for OpenAI, Anthropic, and Google Gemini in your terminal.
- **Transparent Interception**: Patches `fetch` in Node.js and `requests`/`httpx` in Python. Zero code changes required for your existing API logic.
- **Persistent Storage**: Uses MongoDB for scalable, persistent caching.

## 🛠️ How It Works

1. Your code makes an HTTP request (e.g., calling an LLM).
2. **CheapAPI** hashes the request (URL + Method + Body).
3. **Cache HIT** 🎯 → Response is served from MongoDB in milliseconds. No network call, no cost.
4. **Cache MISS** 🌐 → Request is forwarded to the real API, and the response is cached for next time.

---

## 🚀 Quick Start

### Python

```bash
pip install cheap-api
```

```python
import cheap_api
import openai

# 1. Initialize once (reads from env vars or defaults)
cheap_api.auto_init()

# 2. Use your favorite libraries as usual
client = openai.OpenAI(api_key="sk-...")
response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Hello!"}]
) # This call is now cached!
```

### Node.js

```bash
npm install @cheap-api/node
```

```javascript
require('dotenv').config();
const cheapApi = require('@cheap-api/node/sdk');

async function main() {
    // 1. Initialize (patches global fetch)
    await cheapApi.autoInit();

    // 2. Use fetch normally
    const response = await fetch("https://api.openai.com/v1/chat/completions", {
        method: "POST",
        headers: { "Authorization": "Bearer ..." },
        body: JSON.stringify({ ... })
    });
    
    console.log(response.headers.get('x-cheap-api-cache')); // "HIT" or "MISS"
}

main();
```

---

## ⚙️ Configuration

CheapAPI can be configured via environment variables or by passing options to the `init` function.

| Variable | Default | Description |
|---|---|---|
| `CHEAP_API_MONGO_URI` | `mongodb://localhost:27017` | MongoDB connection string |
| `CHEAP_API_DB` | `cheap_api` | Database name |
| `CHEAP_API_COLLECTION` | `cache` | Collection name |
| `CHEAP_API_CACHE_ERRORS` | `false` | Whether to cache error responses (4xx/5xx) |

---

## 🏗️ Repository Structure

- `cheap-api-core/`: The "Brain" - A high-performance Rust engine handling HTTP logic and MongoDB interaction.
- `cheap-api-python/`: Python wrapper using PyO3 for maximum performance.
- `cheap-api-node/`: Node.js wrapper using N-API.

## 📜 License

MIT
