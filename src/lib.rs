use pyo3::prelude::*;

mod annotation;
mod annotationdata;
mod annotationdataset;
mod annotationstore;
mod config;
mod error;
mod resources;
mod selector;
mod textselection;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;

use crate::annotation::PyAnnotation;
use crate::annotationdata::{PyAnnotationData, PyDataKey, PyDataValue};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::PyAnnotationStore;
use crate::error::PyStamError;
use crate::resources::{PyCursor, PyOffset, PyTextResource};
use crate::selector::{PySelector, PySelectorKind};
use crate::textselection::{PyTextSelection, PyTextSelectionOperator};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[pymodule]
fn stam(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add("StamError", py.get_type::<PyStamError>())?;
    m.add("VERSION", VERSION)?;
    m.add_class::<PyAnnotationStore>()?;
    m.add_class::<PyAnnotationDataSet>()?;
    m.add_class::<PyAnnotationData>()?;
    m.add_class::<PyAnnotation>()?;
    m.add_class::<PyDataKey>()?;
    m.add_class::<PyDataValue>()?;
    m.add_class::<PyTextResource>()?;
    m.add_class::<PySelectorKind>()?;
    m.add_class::<PySelector>()?;
    m.add_class::<PyOffset>()?;
    m.add_class::<PyCursor>()?;
    m.add_class::<PyTextSelection>()?;
    m.add_class::<PyTextSelectionOperator>()?;
    Ok(())
}

pub(crate) fn get_limit(kwargs: Option<&PyDict>) -> Option<usize> {
    if let Some(kwargs) = kwargs {
        if let Some(limit) = kwargs.get_item("limit") {
            if let Ok(limit) = limit.extract::<usize>() {
                return Some(limit);
            }
        }
    }
    None
}
