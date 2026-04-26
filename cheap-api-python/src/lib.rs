use pyo3::prelude::*;

// Re-export the CheapApi class from the core crate and register it
// as the Python extension module entry point.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<cheap_api_core::bindings::python::CheapApiPython>()?;
    Ok(())
}
