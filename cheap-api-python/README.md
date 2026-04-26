# CheapAPI Python SDK

A blazing fast API caching SDK powered by Rust. Transparently intercepts `requests` and `httpx` calls, caches responses in MongoDB, and returns identical results instantly — saving you money on repeated LLM/API calls.

## Installation

```bash
pip install cheap-api
```

## Quick Start

```python
import cheap_api

cheap_api.init(
    mongo_uri="mongodb://localhost:27017",
    db="cheap_api",
    collection="cache"
)

# All requests/httpx calls are now cached automatically
import requests
response = requests.get("https://api.openai.com/v1/models", headers={"Authorization": "Bearer sk-..."})
```

## Environment Variables

| Variable | Description |
|---|---|
| `CHEAP_API_MONGO_URI` | MongoDB connection string |
| `CHEAP_API_DB` | Database name |
| `CHEAP_API_COLLECTION` | Collection name |
| `CHEAP_API_CACHE_ERRORS` | Cache error responses (`true`/`false`) |
