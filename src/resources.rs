use pyo3::exceptions::{PyException, PyIndexError, PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use crate::selector::PySelector;
use stam::*;

#[pyclass(name = "TextResource")]
pub(crate) struct PyTextResource {
    pub(crate) handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyTextResource {
    #[getter]
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|res| Ok(res.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|res| Ok(res.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    /// Returns the full text of the resource (by value, aka a copy)
    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|resource| Ok(PyString::new(py, resource.text())))
    }

    /// Returns the text slice at the specified offset, or of the text as a whole if omitted
    fn text<'py>(&self, offset: Option<&PyOffset>, py: Python<'py>) -> PyResult<&'py PyString> {
        if let Some(offset) = offset {
            self.map(|res| Ok(PyString::new(py, res.text_slice(&offset.offset)?)))
        } else {
            self.__str__(py)
        }
    }

    /// Returns a TextSelection instance referring to the specified offset
    fn textselection(&self, offset: &PyOffset) -> PyResult<PyTextSelection> {
        self.map(|res| Ok(self.wrap_textselection(res.textselection(&offset.offset)?)))
    }

    /// Returns a Selector (ResourceSelector) pointing to this TextResource
    fn selector(&self) -> PyResult<PySelector> {
        self.map(|res| res.selector().map(|sel| sel.into()))
    }

    // Iterates over all known textselections in this resource, shortcut for __iter__()
    fn textselections(&self) -> PyTextSelectionIter {
        self.__iter__()
    }

    // Iterates over all known textselections in this resource, in sorted order
    fn __iter__(&self) -> PyTextSelectionIter {
        PyTextSelectionIter {
            positions: self
                .map(|res| Ok(res.positions().map(|x| *x).collect::<Vec<usize>>()))
                .unwrap(),
            index: 0,
            subindex: 0,
            resource_handle: self.handle,
            store: self.store.clone(),
        }
    }

    /// Iterates over all known textselections that start in the spceified range, in sorted order
    //TODO:  this should be __getitem__() with a proper python slice
    fn range(&self, begin: usize, end: usize) -> PyResult<PyTextSelectionIter> {
        Ok(PyTextSelectionIter {
            positions: self
                .map(|res| {
                    Ok(res
                        .positions()
                        .filter_map(|x| {
                            if *x >= begin && *x < end {
                                Some(*x)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<usize>>())
                })
                .unwrap(),
            index: 0,
            subindex: 0,
            resource_handle: self.handle,
            store: self.store.clone(),
        })
    }
}

impl PyTextResource {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&TextResource) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource: &TextResource = store
                .resource(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(resource).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn wrap_textselection(&self, textselection: TextSelection) -> PyTextSelection {
        PyTextSelection {
            textselection,
            resource_handle: self.handle,
            store: self.store.clone(),
        }
    }
}

#[pyclass(name = "Cursor", frozen, freelist = 64)]
#[derive(Clone)]
pub(crate) struct PyCursor {
    cursor: Cursor,
}

#[pymethods]
impl PyCursor {
    #[new]
    fn new(index: isize, endaligned: Option<bool>) -> PyResult<Self> {
        if endaligned.unwrap_or(false) {
            if index <= 0 {
                Ok(Self {
                    cursor: Cursor::EndAligned(index),
                })
            } else {
                Err(PyValueError::new_err(
                    "End aligned cursor should be 0 or negative",
                ))
            }
        } else {
            if index >= 0 {
                Ok(Self {
                    cursor: Cursor::BeginAligned(index as usize),
                })
            } else {
                Err(PyValueError::new_err(
                    "Begin aligned cursor should be 0 or positive",
                ))
            }
        }
    }

    /// Tests if this is a begin-aligned cursor
    fn is_beginaligned(&self) -> bool {
        match self.cursor {
            Cursor::BeginAligned(_) => true,
            _ => false,
        }
    }

    /// Tests if this is an end-aligned cursor
    fn is_endaligned(&self) -> bool {
        match self.cursor {
            Cursor::EndAligned(_) => true,
            _ => false,
        }
    }

    /// Returns the actual cursor value
    fn value(&self) -> isize {
        match self.cursor {
            Cursor::BeginAligned(v) => v as isize,
            Cursor::EndAligned(v) => v,
        }
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.cursor == other.cursor).into_py(py),
            CompareOp::Ne => (self.cursor != other.cursor).into_py(py),
            _ => py.NotImplemented(),
        }
    }
}

#[pyclass(name = "Offset", frozen, freelist = 64)]
pub(crate) struct PyOffset {
    pub(crate) offset: Offset,
}

#[pymethods]
impl PyOffset {
    #[new]
    fn new(begin: PyCursor, end: PyCursor) -> Self {
        Self {
            offset: Offset {
                begin: begin.cursor,
                end: end.cursor,
            },
        }
    }

    #[staticmethod]
    /// Creates a simple offset with begin aligned cursors
    /// This is typically faster than using the normal constructor
    fn simple(begin: usize, end: usize) -> Self {
        Self {
            offset: Offset::simple(begin, end),
        }
    }

    #[staticmethod]
    /// Creates a offset that references the whole text
    /// This is typically faster than using the normal constructor
    fn whole() -> Self {
        Self {
            offset: Offset {
                begin: Cursor::BeginAligned(0),
                end: Cursor::EndAligned(0),
            },
        }
    }

    /// Return the begin cursor
    fn begin(&self) -> PyCursor {
        PyCursor {
            cursor: self.offset.begin,
        }
    }

    /// Return the end cursor
    fn end(&self) -> PyCursor {
        PyCursor {
            cursor: self.offset.end,
        }
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.offset.begin == other.offset.begin
                && self.offset.end == other.offset.end)
                .into_py(py),
            CompareOp::Ne => (self.offset.begin != other.offset.begin
                || self.offset.end != other.offset.end)
                .into_py(py),
            _ => py.NotImplemented(),
        }
    }
}

#[pyclass(name = "TextSelection")]
#[derive(Clone)]
pub(crate) struct PyTextSelection {
    pub(crate) textselection: TextSelection,
    pub(crate) resource_handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyTextSelection {
    /// Resolves a text selection to the actual underlying text
    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|res| {
            Ok(PyString::new(
                py,
                res.text_of(&(self.textselection.into()))?,
            ))
        })
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.resource_handle == other.resource_handle
                && self.textselection == other.textselection)
                .into_py(py),
            CompareOp::Ne => (self.resource_handle != other.resource_handle
                || self.textselection != other.textselection)
                .into_py(py),
            CompareOp::Lt => (self.textselection < other.textselection).into_py(py),
            CompareOp::Le => (self.textselection <= other.textselection).into_py(py),
            CompareOp::Gt => (self.textselection > other.textselection).into_py(py),
            CompareOp::Ge => (self.textselection >= other.textselection).into_py(py),
        }
    }

    /// Returns the resource this textselections points at
    fn resource(&self) -> PyResult<PyTextResource> {
        Ok(PyTextResource {
            handle: self.resource_handle,
            store: self.store.clone(),
        })
    }

    /// Return the absolute begin position in unicode points
    fn begin(&self) -> usize {
        self.textselection.begin()
    }

    /// Return the absolute end position in unicode points (non-inclusive)
    fn end(&self) -> usize {
        self.textselection.end()
    }
}

impl PyTextSelection {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&TextResource) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource: &TextResource = store
                .resource(&self.resource_handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(resource).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

impl From<PyTextSelection> for TextSelection {
    fn from(other: PyTextSelection) -> Self {
        other.textselection
    }
}

#[pyclass(name = "TextSelectionIter")]
// this isn't based off the TextSelectionIter in the Rust library because that one holds a
// reference which we can't in Python. This one will have somewhat more overhead and only supports
// forward iteration
pub(crate) struct PyTextSelectionIter {
    pub(crate) positions: Vec<usize>,
    pub(crate) index: usize,
    pub(crate) subindex: usize,
    pub(crate) resource_handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyTextSelectionIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&mut self) -> Option<PyTextSelection> {
        self.next()
    }
}

impl Iterator for PyTextSelectionIter {
    type Item = PyTextSelection;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(store) = self.store.read() {
            if let Some(resource) = store.resource(&AnyId::Handle(self.resource_handle)) {
                loop {
                    if let Some(position) = self.positions.get(self.index) {
                        if let Some(positionitem) = resource.position(*position) {
                            if let Some((_, handle)) =
                                positionitem.iter_begin2end().nth(self.subindex)
                            {
                                self.subindex += 1;
                                let textselection: Result<&TextSelection, _> =
                                    resource.get(*handle);
                                if let Ok(textselection) = textselection {
                                    return Some(PyTextSelection {
                                        textselection: textselection.clone(),
                                        resource_handle: self.resource_handle,
                                        store: self.store.clone(),
                                    });
                                }
                            }
                        }
                        self.index += 1;
                        self.subindex = 0;
                        //rely on loop to 'recurse'
                    } else {
                        break;
                    }
                }
            }
        }
        None
    }
}
