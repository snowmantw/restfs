#![feature(proc_macro, specialization)]
extern crate pyo3;

use pyo3::prelude::*;
use pyo3::pymodinit;

#[pymodinit]
fn restfslib(_py: Python, m: &PyModule) -> PyResult<()> {
    #[pyfn(m, "test")]
    // ``#[pyfn()]` converts the arguments from Python objects to Rust values
    // and the Rust return value back into a Python object.
    fn test_py(a:i64, b:i64) -> PyResult<String> {
       let out = test(a, b);
       Ok(out)
    }

    Ok(())
}

// The logic can be implemented as a normal rust function
fn test(a:i64, b:i64) -> String {
    format!("{}", a + b).to_string()
}
