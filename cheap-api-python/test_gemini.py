import cheap_api
import requests
import time
import json
import os
from dotenv import load_dotenv

# Load environment variables from .env
load_dotenv()

GEMINI_API_KEY = os.getenv("GEMINI_API_KEY", "TEST_KEY_FOR_SDK")
MONGODB_URI = os.getenv("MONGODB_URI", "mongodb://localhost:27017")

# 1. Initialize the SDK (Points to your local MongoDB)
cheap_api.init(
    connection_uri=MONGODB_URI,
    database="cheap_api_test",
    collection="gemini_cache",
    cache_errors=True
)

def test_caching():
    url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent"
    headers = {"Content-Type": "application/json", "x-goog-api-key": GEMINI_API_KEY}
    payload = {
        "contents": [{"parts": [{"text": "Explain quantum computing to a 5 year old."}]}]
    }

    print("\n🚀 [Python] Request 1: Sending prompt to Gemini...")
    print(f"📡 Calling: {url}")
    start = time.time()
    
    # CheapAPI intercepts this requests.post call!
    try:
        r1 = requests.post(url, json=payload, headers=headers)
        latency1 = time.time() - start
        print(f"⏱️  Latency: {latency1:.4f}s")
        print(f"📦 Cache Status: {r1.headers.get('X-Cheap-API-Cache', 'NONE')}")
        
        print("\n🚀 [Python] Request 2: Sending SAME prompt again...")
        start = time.time()
        r2 = requests.post(url, json=payload, headers=headers)
        latency2 = time.time() - start
        print(f"⏱️  Latency: {latency2:.4f}s")
        print(f"📦 Cache Status: {r2.headers.get('X-Cheap-API-Cache', 'NONE')}")
        
        if latency2 < latency1:
            print(f"\n✅ SUCCESS: Cache HIT was {latency1/latency2:.1f}x faster!")
        else:
            print("\n⚠️ WARNING: Second request wasn't significantly faster. Check if MongoDB is running.")
            
    except Exception as e:
        print(f"❌ Error during test: {e}")

if __name__ == "__main__":
    test_caching()
