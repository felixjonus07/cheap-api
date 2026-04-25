import os
import json
from typing import Optional, Dict, Any
from ._core import CheapApi as _CheapApiCore

_interceptor = None

def init(
    connection_uri: str = "mongodb://localhost:27017",
    database: str = "cheap_api",
    collection: str = "cache",
    ttl_seconds: Optional[int] = None,
    cache_errors: bool = False,
    max_cacheable_body_bytes: int = 10 * 1024 * 1024
):
    """
    Initialize the Cheap API SDK and patch common HTTP libraries.
    """
    global _interceptor
    if _interceptor is not None:
        return _interceptor
    
    _interceptor = _CheapApiCore.with_mongodb(
        connection_uri,
        database,
        collection,
        ttl_seconds,
        cache_errors,
        max_cacheable_body_bytes
    )
    
    _patch_requests()
    _patch_httpx()
    
    print("[CheapAPI] Python SDK initialized and HTTP libraries patched.")
    return _interceptor

def auto_init():
    """
    Initialize the SDK using environment variables or defaults.
    CHEAP_API_MONGO_URI
    CHEAP_API_DB
    CHEAP_API_COLLECTION
    """
    return init(
        connection_uri=os.getenv("CHEAP_API_MONGO_URI", "mongodb://localhost:27017"),
        database=os.getenv("CHEAP_API_DB", "cheap_api"),
        collection=os.getenv("CHEAP_API_COLLECTION", "cache"),
        cache_errors=os.getenv("CHEAP_API_CACHE_ERRORS", "false").lower() == "true"
    )

def _patch_requests():
    """
    Monkey-patch the 'requests' library by wrapping Session.request.
    """
    try:
        import requests
        from requests.sessions import Session
        
        _original_request = Session.request
        
        def patched_request(self, method, url, **kwargs):
            if _interceptor is None:
                return _original_request(self, method, url, **kwargs)
            
            # Extract basic request data
            headers = dict(kwargs.get("headers", {}))
            body = kwargs.get("data") or kwargs.get("json")
            if isinstance(body, dict):
                body = json.dumps(body)
            
            # Attempt interception
            try:
                res_data = _interceptor.intercept(
                    str(url),
                    str(method).upper(),
                    headers,
                    str(body) if body else None
                )
                
                # Mock a requests.Response object
                from requests.models import Response
                response = Response()
                response.status_code = res_data["status"]
                response._content = res_data["body"].encode("utf-8")
                response.headers.update(res_data["headers"])
                response.headers["X-Cheap-API-Cache"] = "HIT" if res_data["from_cache"] else "MISS"
                response.headers["X-Cheap-API-Key"] = res_data["cache_key"]
                response.url = str(url)
                response.encoding = 'utf-8'
                
                return response
            except Exception as e:
                print(f"[CheapAPI] Interception error (requests fallback): {e}")
                return _original_request(self, method, url, **kwargs)

        Session.request = patched_request
    except ImportError:
        pass

def _patch_httpx():
    """
    Monkey-patch the 'httpx' library (used by OpenAI >= 1.0).
    """
    try:
        import httpx
        from httpx import Client
        
        _original_send = Client.send
        
        def patched_send(self, request, **kwargs):
            if _interceptor is None:
                return _original_send(self, request, **kwargs)
            
            # Extract data from httpx.Request
            url = str(request.url)
            method = request.method
            headers = dict(request.headers)
            body = request.read().decode("utf-8") if request.content else None
            
            try:
                res_data = _interceptor.intercept(
                    url,
                    method,
                    headers,
                    body
                )
                
                # Construct a httpx.Response
                response = httpx.Response(
                    status_code=res_data["status"],
                    headers=res_data["headers"],
                    content=res_data["body"].encode("utf-8"),
                    request=request
                )
                response.headers["X-Cheap-API-Cache"] = "HIT" if res_data["from_cache"] else "MISS"
                
                return response
            except Exception as e:
                print(f"[CheapAPI] Interception error (httpx fallback): {e}")
                return _original_send(self, request, **kwargs)

        Client.send = patched_send
    except ImportError:
        pass
