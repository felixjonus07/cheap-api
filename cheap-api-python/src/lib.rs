use pyo3::prelude::*;

// Register CheapApiPython as the Python extension module entry point.
// This is the only #[pymodule] in the entire project — one per shared library.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<cheap_api_core::bindings::python::CheapApiPython>()?;
    Ok(())
}
