use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotation;
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use stam::*;

#[pyclass(dict, module = "stam", name = "DataKey")]
/// The DataKey class defines a vocabulary field, it
/// belongs to a certain :obj:`AnnotationDataSet`. An :obj:`AnnotationData` instance
/// in turn makes reference to a DataKey and assigns it a value.
pub(crate) struct PyDataKey {
    pub(crate) set: AnnotationDataSetHandle,
    pub(crate) handle: DataKeyHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyDataKey {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|datakey| Ok(datakey.id().map(|x| x.to_owned())))
    }

    /// Returns the public ID (by value, aka a copy)
    /// Use this sparingly
    fn __str__(&self) -> PyResult<Option<String>> {
        self.map(|datakey| Ok(datakey.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|datakey| Ok(datakey.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    /// Returns the AnnotationDataSet this key is part of
    fn annotationset(&self) -> PyResult<PyAnnotationDataSet> {
        Ok(PyAnnotationDataSet {
            handle: self.set,
            store: self.store.clone(),
        })
    }

    /// Returns a list of AnnotationData instances that use this key.
    /// This is a lookup in the reverse index.
    fn annotationdata<'py>(&self, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotationset| {
            for data in annotationset.data().into_iter().flatten() {
                list.append(
                    PyAnnotationData {
                        handle: data.handle().expect("must have handle"),
                        set: self.set,
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
                .ok();
            }
            Ok(())
        })
        .ok();
        list.into()
    }

    /// Find annotation data for the current key and specified value
    /// Returns an AnnotationData instance if found, None otherwise
    /// Use AnnotationDataSet.find_data() instead if you don't have a DataKey instance yet.
    fn find_data(&self, value: &PyAny) -> PyResult<Option<PyAnnotationData>> {
        let annotationset = self.annotationset()?;
        annotationset.map(|set| {
            let value = py_into_datavalue(value)?;
            if let Some(annotationdata) = set.find_data(self.handle.into(), &value) {
                Ok(Some(PyAnnotationData {
                    handle: annotationdata
                        .handle()
                        .expect("annotationdata must be bound"),
                    set: self.set,
                    store: self.store.clone(),
                }))
            } else {
                Ok(None)
            }
        })
    }
}

impl MapStore for PyDataKey {
    fn get_store(&self) -> &Arc<RwLock<AnnotationStore>> {
        &self.store
    }
    fn get_store_mut(&mut self) -> &mut Arc<RwLock<AnnotationStore>> {
        &mut self.store
    }
}

impl PyDataKey {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<DataKey>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset = store
                .annotationset(&self.set.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolved annotationset"))?;
            let datakey = annotationset
                .key(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolved annotationset"))?;
            f(datakey).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

#[pyclass(dict, module = "stam", name = "AnnotationData")]
/// AnnotationData holds the actual content of an annotation; a key/value pair. (the
/// term *feature* is regularly seen for this in certain annotation paradigms).
/// Annotation Data is deliberately decoupled from the actual :obj:`Annotation`
/// instances so multiple annotation instances can point to the same content
/// without causing any overhead in storage. Moreover, it facilitates indexing and
/// searching. The annotation data is part of an `AnnotationDataSet`, which
/// effectively defines a certain user-defined vocabulary.
///
/// Once instantiated, instances of this type are, by design, largely immutable.
/// The key and value can not be changed. Create a new AnnotationData and new Annotation for edits.
/// This class is not instantiated directly.
pub(crate) struct PyAnnotationData {
    pub(crate) set: AnnotationDataSetHandle,
    pub(crate) handle: AnnotationDataHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

pub(crate) fn py_into_datavalue<'py>(value: &'py PyAny) -> Result<DataValue, StamError> {
    if let Ok(value) = value.extract() {
        Ok(DataValue::String(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataValue::Int(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataValue::Float(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataValue::Bool(value))
    } else if let Ok(None) = value.extract::<Option<bool>>() {
        Ok(DataValue::Null)
    } else {
        if let Ok(true) = value.is_instance_of::<PyList>() {
            let value: &PyList = value.downcast().unwrap();
            let mut list: Vec<DataValue> = Vec::new();
            for item in value {
                let pyitem = py_into_datavalue(item)?;
                list.push(pyitem);
            }
            return Ok(DataValue::List(list));
        }
        Err(StamError::OtherError(
            "Can't convert supplied Python object to a DataValue",
        ))
    }
}

pub(crate) fn datavalue_into_py<'py>(
    datavalue: &DataValue,
    py: Python<'py>,
) -> Result<&'py PyAny, StamError> {
    match datavalue {
        DataValue::String(s) => Ok(s.into_py(py).into_ref(py)),
        DataValue::Float(f) => Ok(f.into_py(py).into_ref(py)),
        DataValue::Int(v) => Ok(v.into_py(py).into_ref(py)),
        DataValue::Bool(v) => Ok(v.into_py(py).into_ref(py)),
        DataValue::Null => {
            //feels a bit hacky, but I can't find a PyNone to return as PyAny
            let x: Option<bool> = None;
            Ok(x.into_py(py).into_ref(py))
        }
        DataValue::List(v) => {
            let pylist = PyList::empty(py);
            for item in v.iter() {
                let pyvalue = datavalue_into_py(item, py)?;
                pylist.append(pyvalue).expect("adding value to list");
            }
            Ok(pylist)
        }
    }
}

#[pyclass(dict, module = "stam", name = "DataValue")]
#[derive(Clone, Debug)]
/// Encapsulates a value and its type. Held by `AnnotationData`. This type is not a reference but holds the actual value.
pub(crate) struct PyDataValue {
    pub(crate) value: DataValue,
}

impl std::fmt::Display for PyDataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[pymethods]
impl PyDataValue {
    // Get the actual value
    fn get<'py>(&self, py: Python<'py>) -> PyResult<&'py PyAny> {
        datavalue_into_py(&self.value, py).map_err(|err| PyStamError::new_err(format!("{}", err)))
    }

    #[new]
    fn new<'py>(value: &PyAny) -> PyResult<Self> {
        Ok(PyDataValue {
            value: py_into_datavalue(value)
                .map_err(|err| PyStamError::new_err(format!("{}", err)))?,
        })
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.value == other.value).into_py(py),
            CompareOp::Ne => (self.value != other.value).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}

impl PyDataValue {
    fn new_cloned(value: &DataValue) -> Result<Self, StamError> {
        Ok(PyDataValue {
            value: value.clone(),
        })
    }

    fn test(&self, other: &DataValue) -> bool {
        self.value == *other
    }
}

//not sure if we really need these from implementations here

impl From<&str> for PyDataValue {
    fn from(other: &str) -> Self {
        PyDataValue {
            value: other.into(),
        }
    }
}

impl From<String> for PyDataValue {
    fn from(other: String) -> Self {
        PyDataValue {
            value: other.into(),
        }
    }
}

impl From<usize> for PyDataValue {
    fn from(other: usize) -> Self {
        PyDataValue {
            value: other.into(),
        }
    }
}

impl From<isize> for PyDataValue {
    fn from(other: isize) -> Self {
        PyDataValue {
            value: other.into(),
        }
    }
}

impl From<f64> for PyDataValue {
    fn from(other: f64) -> Self {
        PyDataValue {
            value: other.into(),
        }
    }
}

#[pymethods]
impl PyAnnotationData {
    /// Returns a DataKey instance
    fn key(&self) -> PyResult<PyDataKey> {
        self.map(|annotationdata| {
            Ok(PyDataKey {
                set: self.set,
                handle: annotationdata.key().handle().expect("key must have handle"),
                store: self.store.clone(),
            })
        })
    }

    /// Returns the value (makes a copy)
    /// In comparisons, use test_value() instead
    fn value(&self) -> PyResult<PyDataValue> {
        self.map(|annotationdata| PyDataValue::new_cloned(annotationdata.value()))
    }

    /// Tests whether the value equals another
    /// This is more efficient than calling [`value()`] and doing the comparison yourself
    fn test_value<'py>(&self, reference: &'py PyDataValue) -> PyResult<bool> {
        self.map(|annotationdata| Ok(reference.test(&annotationdata.value())))
    }

    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|annotationdata| Ok(annotationdata.id().map(|x| x.to_owned())))
    }

    /// Returns the public ID (by value, aka a copy)
    /// Use this sparingly
    fn __str__(&self) -> PyResult<Option<String>> {
        self.map(|annotationdata| Ok(annotationdata.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|annotationdata| Ok(annotationdata.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    /// Returns the AnnotationDataSet this data is part of
    fn annotationset(&self) -> PyResult<PyAnnotationDataSet> {
        Ok(PyAnnotationDataSet {
            handle: self.set,
            store: self.store.clone(),
        })
    }

    /// Returns a list of  Annotation instances that use this data
    /// This is a lookup in the reverse index.
    fn annotations<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map_with_store(|data, store| {
            for (i, annotation) in data.annotations(store).into_iter().flatten().enumerate() {
                list.append(
                    PyAnnotation {
                        handle: annotation.handle().expect("must have handle"),
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
                .ok();
                if Some(i + 1) == limit {
                    break;
                }
            }
            Ok(())
        })
        .ok();
        list.into()
    }
}

impl MapStore for PyAnnotationData {
    fn get_store(&self) -> &Arc<RwLock<AnnotationStore>> {
        &self.store
    }
    fn get_store_mut(&mut self) -> &mut Arc<RwLock<AnnotationStore>> {
        &mut self.store
    }
}

impl PyAnnotationData {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<AnnotationData>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset = store
                .annotationset(&self.set.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            let data = annotationset
                .annotationdata(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            f(data).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn map_with_store<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<AnnotationData>, &AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset = store
                .annotationset(&self.set.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            let data = annotationset
                .annotationdata(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            f(data, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

/// Build an AnnotationDataBuilder from a python dictionary (or string referring to an existing public ID)
/// Expects a Python dictionary with fields "id", "key","set", "value" (or a subpart thereof). The field values
/// may be STAM data types or plain strings with public IDs.
pub(crate) fn annotationdata_builder<'a>(data: &'a PyAny) -> PyResult<AnnotationDataBuilder<'a>> {
    let mut builder = AnnotationDataBuilder::new();
    if let Ok(true) = data.is_instance_of::<PyAnnotationData>() {
        let adata: PyRef<'_, PyAnnotationData> = data.extract()?;
        builder = builder.with_id(adata.handle.into());
        builder = builder.with_annotationset(adata.set.into());
        Ok(builder)
    } else if let Ok(true) = data.is_instance_of::<PyDict>() {
        let data = data.downcast::<PyDict>()?;
        if let Some(id) = data.get_item("id") {
            if let Ok(true) = id.is_instance_of::<PyAnnotationData>() {
                let adata: PyRef<'_, PyAnnotationData> = id.extract()?;
                builder = builder.with_id(adata.handle.into());
                builder = builder.with_annotationset(adata.set.into());
            } else {
                let id: String = id.extract()?;
                builder = builder.with_id(id.into());
            }
        }
        if let Some(key) = data.get_item("key") {
            if let Ok(true) = key.is_instance_of::<PyDataKey>() {
                let key: PyRef<'_, PyDataKey> = key.extract()?;
                builder = builder.with_key(key.handle.into());
            } else {
                let key: String = key.extract()?;
                builder = builder.with_key(key.into());
            }
        }
        if let Some(set) = data.get_item("set") {
            if let Ok(true) = set.is_instance_of::<PyAnnotationDataSet>() {
                let set: PyRef<'_, PyAnnotationDataSet> = set.extract()?;
                builder = builder.with_annotationset(set.handle.into());
            } else {
                let set: String = set.extract()?;
                builder = builder.with_annotationset(set.into());
            }
        }
        if let Some(value) = data.get_item("value") {
            builder = builder.with_value(
                py_into_datavalue(value)
                    .map_err(|_e| PyValueError::new_err("Invalid type for value"))?,
            )
        }
        Ok(builder)
    } else if let Ok(true) = data.is_instance_of::<PyString>() {
        let id = data.downcast::<PyString>()?;
        Ok(AnnotationDataBuilder::new().with_id(id.to_str()?.into()))
    } else {
        Err(PyValueError::new_err(
            "Argument to build AnnotationData must be a dictionary (with fields id, key, set, value), a string (with a public ID), or an AnnotationData instance. A list containing any multiple of those types is also allowed in certain circumstances.",
        ))
    }
}
