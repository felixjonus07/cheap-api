# CheapAPI 🚀

> **Never pay for the same API call twice.**  
> A Rust-powered caching layer that transparently intercepts your HTTP requests, stores responses in MongoDB, and replays them instantly — saving you money on every repeated API call.

[![Python](https://img.shields.io/pypi/v/cheap-api?label=pip%20install%20cheap-api&color=blue)](https://pypi.org/project/cheap-api/)
[![npm](https://img.shields.io/npm/v/@cheap-api/node?label=npm%20install%20%40cheap-api%2Fnode&color=red)](https://www.npmjs.com/package/@cheap-api/node)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## How It Works

1. Your code makes an HTTP request (OpenAI, Gemini, any API)
2. CheapAPI hashes the request (URL + method + body)
3. **Cache HIT** → response returned from MongoDB in milliseconds, no charge
4. **Cache MISS** → request forwarded to the real API, response saved for next time

## Installation

### Python

```bash
pip install cheap-api
```

```python
import cheap_api

# Initialize once (uses env vars or defaults)
cheap_api.auto_init()

# Now all requests/httpx calls are transparently cached!
import openai
client = openai.OpenAI(api_key="sk-...")
response = client.chat.completions.create(...)  # Cached automatically
```

### Node.js

```bash
npm install @cheap-api/node
```

```js
const { CheapApi } = require('@cheap-api/node')

const api = await CheapApi.withMongodb('mongodb://localhost:27017', 'cheap_api', 'cache')
const result = await api.intercept(url, method, headers, body)
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `CHEAP_API_MONGO_URI` | `mongodb://localhost:27017` | MongoDB connection string |
| `CHEAP_API_DB` | `cheap_api` | Database name |
| `CHEAP_API_COLLECTION` | `cache` | Collection name |
| `CHEAP_API_CACHE_ERRORS` | `false` | Whether to cache error responses |

## Requirements

- **MongoDB** running locally or remotely
- **Python 3.8+** (for Python SDK)
- **Node.js 18+** (for Node SDK)

## Repository Structure

```
cheapapi/
├── cheap-api-core/     # Rust core (caching + forwarding engine)
├── cheap-api-python/   # Python SDK (pip install cheap-api)
└── cheap-api-node/     # Node.js SDK (npm install @cheap-api/node)
```

## Development

### Building from Source

**Prerequisites:** Rust + Cargo (`rustup.rs`)

```bash
# Python SDK
cd cheap-api-python
pip install maturin
maturin develop

# Node SDK
cd cheap-api-node
npm install
npm run build
```

## License

MIT
