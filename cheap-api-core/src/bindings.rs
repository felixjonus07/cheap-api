use crate::interceptor::{InterceptRequest, Interceptor, InterceptorConfig};
use crate::mongo_adapter::{MongoAdapter, MongoAdapterConfig};
use std::sync::Arc;

// ── Node.js Bindings ──────────────────────────────────────────────────────────

#[cfg(feature = "node")]
pub mod node {
    use super::*;
    use napi::bindgen_prelude::*;
    use napi_derive::napi;

    #[napi(js_name = "CheapApi")]
    pub struct CheapApiNode {
        inner: Interceptor,
    }

    #[napi]
    impl CheapApiNode {
        #[napi(factory)]
        pub async fn with_mongodb(
            connection_uri: String,
            database: String,
            collection: String,
            ttl_seconds: Option<u32>,
            cache_errors: bool,
            max_cacheable_body_bytes: u32,
        ) -> Result<CheapApiNode> {
            let store = MongoAdapter::connect(MongoAdapterConfig {
                connection_uri,
                database,
                collection,
                ttl_seconds,
            })
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

            let config = InterceptorConfig {
                cache_errors,
                max_cacheable_body_bytes: max_cacheable_body_bytes as usize,
            };

            Ok(CheapApiNode {
                inner: Interceptor::new(Arc::new(store), config),
            })
        }

        #[napi]
        pub async fn intercept(
            &self,
            url: String,
            method: String,
            headers: std::collections::HashMap<String, String>,
            body: Option<String>,
        ) -> Result<serde_json::Value> {
            let req = InterceptRequest {
                url,
                method,
                headers,
                body: body.unwrap_or_default(),
            };

            let res = self
                .inner
                .intercept(req)
                .await
                .map_err(|e| Error::from_reason(e.to_string()))?;

            serde_json::to_value(res).map_err(|e| Error::from_reason(e.to_string()))
        }
    }
}

// ── Python Bindings ──────────────────────────────────────────────────────────

#[cfg(feature = "python")]
pub mod python {
    use super::*;
    use pyo3::prelude::*;
    use pyo3::types::PyDict;

    #[pyclass(name = "CheapApi")]
    pub struct CheapApiPython {
        inner: Interceptor,
    }

    #[pymethods]
    impl CheapApiPython {
        #[staticmethod]
        #[pyo3(signature = (connection_uri, database, collection, ttl_seconds=None, cache_errors=False, max_cacheable_body_bytes=10485760))]
        pub fn with_mongodb(
            connection_uri: String,
            database: String,
            collection: String,
            ttl_seconds: Option<u64>,
            cache_errors: bool,
            max_cacheable_body_bytes: usize,
        ) -> PyResult<PyObject> {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let store = rt
                .block_on(async {
                    MongoAdapter::connect(MongoAdapterConfig {
                        connection_uri,
                        database,
                        collection,
                        ttl_seconds: ttl_seconds.map(|s| s as u32),
                    })
                    .await
                })
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            let config = InterceptorConfig {
                cache_errors,
                max_cacheable_body_bytes,
            };

            Python::with_gil(|py| {
                let instance = CheapApiPython {
                    inner: Interceptor::new(Arc::new(store), config),
                };
                Ok(instance.into_py(py))
            })
        }

        pub fn intercept(
            &self,
            url: String,
            method: String,
            headers: std::collections::HashMap<String, String>,
            body: Option<String>,
        ) -> PyResult<PyObject> {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let req = InterceptRequest {
                url,
                method,
                headers,
                body: body.unwrap_or_default(),
            };

            let res = rt
                .block_on(async { self.inner.intercept(req).await })
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            Python::with_gil(|py| {
                let dict = PyDict::new_bound(py);
                dict.set_item("status", res.status)?;
                dict.set_item("headers", res.headers)?;
                dict.set_item("body", res.body)?;
                dict.set_item("from_cache", res.from_cache)?;
                dict.set_item("cache_key", res.cache_key)?;
                Ok(dict.to_object(py))
            })
        }
    }

    #[pymodule]
    fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<CheapApiPython>()?;
        Ok(())
    }
}
