use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::borrow::Cow;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotations;
use crate::error::PyStamError;
use stam::*;

#[pyclass(dict, module = "stam", name = "AnnotationSubStore")]
/// This holds an annotation store that is included as a depenency into another one
///
/// The text *SHOULD* be in
/// [Unicode Normalization Form C (NFC)](https://www.unicode.org/reports/tr15/) but
/// *MAY* be in another unicode normalization forms.
pub(crate) struct PyAnnotationSubStore {
    pub(crate) handle: AnnotationSubStoreHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

impl PyAnnotationSubStore {
    pub(crate) fn new(
        handle: AnnotationSubStoreHandle,
        store: Arc<RwLock<AnnotationStore>>,
    ) -> Self {
        Self { handle, store }
    }

    pub(crate) fn new_py<'py>(
        handle: AnnotationSubStoreHandle,
        store: Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::new(handle, store).into_py(py).into_ref(py)
    }
}

#[pymethods]
impl PyAnnotationSubStore {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|substore| Ok(substore.id().map(|x| x.to_owned())))
    }

    fn filename(&self) -> PyResult<Option<String>> {
        self.map(|s| {
            Ok(s.as_ref()
                .filename()
                .map(|s| s.to_string_lossy().into_owned()))
        })
    }

    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|substore| Ok(substore.id() == Some(other)))
    }

    fn has_filename(&self, filename: &str) -> PyResult<bool> {
        self.map(|substore| {
            Ok(substore.as_ref().filename().map(|s| s.to_string_lossy())
                == Some(Cow::Borrowed(filename)))
        })
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.handle == other.handle,
            CompareOp::Ne => self.handle != other.handle,
            CompareOp::Lt => self.handle < other.handle,
            CompareOp::Gt => self.handle > other.handle,
            CompareOp::Le => self.handle <= other.handle,
            CompareOp::Ge => self.handle >= other.handle,
        }
    }

    fn __hash__(&self) -> usize {
        self.handle.as_usize()
    }
}

impl PyAnnotationSubStore {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    pub(crate) fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(ResultItem<AnnotationSubStore>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let substore = store
                .substore(self.handle)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve substore"))?;
            f(substore).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}
