require('dotenv').config();
const cheapApi = require('./sdk'); // <--- Point to sdk.js
// We use global fetch because the SDK patches it automatically

async function testCaching() {
    console.log("🛠️ Initializing CheapAPI Node SDK...");
    
    const MONGODB_URI = process.env.MONGODB_URI || "mongodb://localhost:27017";
    const GEMINI_API_KEY = process.env.GEMINI_API_KEY || "TEST_KEY_FOR_SDK";

    // 1. Initialize the SDK (This patches global fetch)
    await cheapApi.init({
        connectionUri: MONGODB_URI,
        database: "cheap_api_test",
        collection: "gemini_cache"
    }, {
        cacheErrors: true
    });

    const url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent";
    const payload = {
        contents: [{ parts: [{ text: "Explain quantum computing to a 5 year old." }] }]
    };

    console.log("\n🚀 [Node.js] Request 1: Sending prompt to Gemini...");
    let start = Date.now();
    
    // Use standard fetch!
    const r1 = await fetch(url, {
        method: 'POST',
        headers: { 
            'Content-Type': 'application/json',
            'x-goog-api-key': GEMINI_API_KEY 
        },
        body: JSON.stringify(payload)
    });
    
    let latency1 = (Date.now() - start) / 1000;
    console.log(`⏱️  Latency: ${latency1.toFixed(4)}s`);
    console.log(`📦 Cache Status: ${r1.headers.get('x-cheap-api-cache') || 'NONE'}`);

    console.log("\n🚀 [Node.js] Request 2: Sending SAME prompt again...");
    start = Date.now();
    
    const r2 = await fetch(url, {
        method: 'POST',
        headers: { 
            'Content-Type': 'application/json',
            'x-goog-api-key': GEMINI_API_KEY 
        },
        body: JSON.stringify(payload)
    });
    
    let latency2 = (Date.now() - start) / 1000;
    console.log(`⏱️  Latency: ${latency2.toFixed(4)}s`);
    console.log(`📦 Cache Status: ${r2.headers.get('x-cheap-api-cache') || 'NONE'}`);

    if (latency2 < latency1) {
        console.log(`\n✅ SUCCESS: Cache HIT was ${(latency1/latency2).toFixed(1)}x faster!`);
    }
}

testCaching().catch(console.error);
