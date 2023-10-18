use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotations;
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use crate::iterparams::IterParams;
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

impl PyDataKey {
    pub(crate) fn new(
        handle: DataKeyHandle,
        set: AnnotationDataSetHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyDataKey {
        PyDataKey {
            set,
            handle,
            store: store.clone(),
        }
    }
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
            CompareOp::Eq => (self.set == other.set && self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.set != other.set || self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let h = (self.set.as_usize(), self.handle.as_usize());
        h.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the AnnotationDataSet this key is part of
    fn dataset(&self) -> PyResult<PyAnnotationDataSet> {
        Ok(PyAnnotationDataSet {
            handle: self.set,
            store: self.store.clone(),
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn data(&self, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|key| {
            let iter = key.data();
            iterparams.evaluate_to_pydata(iter, key.rootstore(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_data(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|key| {
            let iter = key.data();
            Ok(iterparams.evaluate_data(iter, key.rootstore())?.test())
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn annotations(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|key| {
            let iter = key.annotations();
            iterparams.evaluate_to_pyannotations(iter, key.rootstore(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|key| {
            let iter = key.annotations();
            Ok(iterparams
                .evaluate_annotations(iter, key.rootstore())?
                .test())
        })
    }

    fn annotations_count(&self) -> usize {
        self.map(|key| Ok(key.annotations_count())).unwrap()
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
        F: FnOnce(ResultItem<DataKey>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset = store
                .dataset(self.set)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolved annotationset"))?;
            let datakey = annotationset
                .key(self.handle)
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

impl PyAnnotationData {
    pub(crate) fn new(
        handle: AnnotationDataHandle,
        set: AnnotationDataSetHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyAnnotationData {
        PyAnnotationData {
            set,
            handle,
            store: store.clone(),
        }
    }
}

pub(crate) fn datavalue_from_py<'py>(value: &'py PyAny) -> Result<DataValue, StamError> {
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
        if value.is_instance_of::<PyList>() {
            let value: &PyList = value.downcast().unwrap();
            let mut list: Vec<DataValue> = Vec::new();
            for item in value {
                let pyitem = datavalue_from_py(item)?;
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
            value: datavalue_from_py(value)
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
                handle: annotationdata.key().handle(),
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

    /// Returns the value as a string
    fn __str__(&self) -> PyResult<String> {
        self.map(|annotationdata| Ok(annotationdata.value().to_string()))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|annotationdata| Ok(annotationdata.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.set == other.set && self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.set != other.set || self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let h = (self.set.as_usize(), self.handle.as_usize());
        h.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the AnnotationDataSet this data is part of
    fn dataset(&self) -> PyResult<PyAnnotationDataSet> {
        Ok(PyAnnotationDataSet::new(self.set, &self.store))
    }

    #[pyo3(signature = (**kwargs))]
    fn annotations(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|data| {
            let iter = data.annotations();
            iterparams.evaluate_to_pyannotations(iter, data.rootstore(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|data| {
            let iter = data.annotations();
            Ok(iterparams
                .evaluate_annotations(iter, data.rootstore())?
                .test())
        })
    }

    fn annotations_len(&self) -> usize {
        self.map(|data| Ok(data.annotations_len())).unwrap()
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
        F: FnOnce(ResultItem<AnnotationData>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset = store
                .dataset(self.set)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            let data = annotationset
                .annotationdata(self.handle)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve annotationset"))?;
            f(data).map_err(|err| PyStamError::new_err(format!("{}", err)))
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
    if data.is_instance_of::<PyAnnotationData>() {
        let adata: PyRef<'_, PyAnnotationData> = data.extract()?;
        builder = builder.with_id(adata.handle.into());
        builder = builder.with_dataset(adata.set.into());
        Ok(builder)
    } else if data.is_instance_of::<PyDict>() {
        let data = data.downcast::<PyDict>()?;
        if let Ok(Some(id)) = data.get_item("id") {
            if id.is_instance_of::<PyAnnotationData>() {
                let adata: PyRef<'_, PyAnnotationData> = id.extract()?;
                builder = builder.with_id(adata.handle.into());
                builder = builder.with_dataset(adata.set.into());
            } else {
                let id: String = id.extract()?;
                builder = builder.with_id(id.into());
            }
        }
        if let Ok(Some(key)) = data.get_item("key") {
            if key.is_instance_of::<PyDataKey>() {
                let key: PyRef<'_, PyDataKey> = key.extract()?;
                builder = builder.with_key(key.handle.into());
            } else {
                let key: String = key.extract()?;
                builder = builder.with_key(key.into());
            }
        }
        if let Ok(Some(set)) = data.get_item("set") {
            if set.is_instance_of::<PyAnnotationDataSet>() {
                let set: PyRef<'_, PyAnnotationDataSet> = set.extract()?;
                builder = builder.with_dataset(set.handle.into());
            } else {
                let set: String = set.extract()?;
                builder = builder.with_dataset(set.into());
            }
        }
        if let Ok(Some(value)) = data.get_item("value") {
            builder = builder.with_value(
                datavalue_from_py(value)
                    .map_err(|_e| PyValueError::new_err("Invalid type for value"))?,
            )
        }
        Ok(builder)
    } else if data.is_instance_of::<PyString>() {
        let id = data.downcast::<PyString>()?;
        Ok(AnnotationDataBuilder::new().with_id(id.to_str()?.into()))
    } else {
        Err(PyValueError::new_err(
            "Argument to build AnnotationData must be a dictionary (with fields id, key, set, value), a string (with a public ID), or an AnnotationData instance. A list containing any multiple of those types is also allowed in certain circumstances.",
        ))
    }
}

pub(crate) fn dataoperator_from_kwargs<'a, 'py>(
    kwargs: &'py PyDict,
) -> Result<Option<DataOperator<'a>>, StamError>
where
    'py: 'a,
{
    if let Ok(Some(value)) = kwargs.get_item("value") {
        Ok(Some(dataoperator_from_py(value)?))
    } else if let Ok(Some(value)) = kwargs.get_item("value_not") {
        Ok(Some(DataOperator::Not(Box::new(dataoperator_from_py(
            value,
        )?))))
    } else if let Ok(Some(value)) = kwargs.get_item("value_greater") {
        Ok(Some(dataoperator_greater_from_py(value)?))
    } else if let Ok(Some(value)) = kwargs.get_item("value_not_greater") {
        Ok(Some(DataOperator::Not(Box::new(
            dataoperator_greater_from_py(value)?,
        ))))
    } else if let Ok(Some(value)) = kwargs.get_item("value_less") {
        Ok(Some(dataoperator_less_from_py(value)?))
    } else if let Ok(Some(value)) = kwargs.get_item("value_not_less") {
        Ok(Some(DataOperator::Not(Box::new(
            dataoperator_less_from_py(value)?,
        ))))
    } else if let Ok(Some(value)) = kwargs.get_item("value_greatereq") {
        Ok(Some(dataoperator_greatereq_from_py(value)?))
    } else if let Ok(Some(value)) = kwargs.get_item("value_not_greatereq") {
        Ok(Some(DataOperator::Not(Box::new(
            dataoperator_greatereq_from_py(value)?,
        ))))
    } else if let Ok(Some(value)) = kwargs.get_item("value_lesseq") {
        Ok(Some(dataoperator_lesseq_from_py(value)?))
    } else if let Ok(Some(value)) = kwargs.get_item("value_not_lesseq") {
        Ok(Some(DataOperator::Not(Box::new(
            dataoperator_lesseq_from_py(value)?,
        ))))
    } else if let Ok(Some(values)) = kwargs.get_item("value_in") {
        if values.is_instance_of::<PyTuple>() {
            let values: &PyTuple = values.downcast().unwrap();
            let mut suboperators = Vec::with_capacity(values.len());
            for value in values {
                suboperators.push(dataoperator_from_py(value)?)
            }
            Ok(Some(DataOperator::Or(suboperators)))
        } else {
            Err(StamError::OtherError("`value_in` must be a tuple"))
        }
    } else if let Ok(Some(values)) = kwargs.get_item("value_not_in") {
        if values.is_instance_of::<PyTuple>() {
            let values: &PyTuple = values.downcast().unwrap();
            let mut suboperators = Vec::with_capacity(values.len());
            for value in values {
                suboperators.push(dataoperator_from_py(value)?)
            }
            Ok(Some(DataOperator::Not(Box::new(DataOperator::Or(
                suboperators,
            )))))
        } else {
            Err(StamError::OtherError("`value_in` must be a tuple"))
        }
    } else if let Ok(Some(values)) = kwargs.get_item("value_in_range") {
        if let Ok((min, max)) = values.extract::<(isize, isize)>() {
            Ok(Some(DataOperator::And(vec![
                DataOperator::GreaterThanOrEqual(min),
                DataOperator::LessThanOrEqual(max),
            ])))
        } else if let Ok((min, max)) = values.extract::<(f64, f64)>() {
            Ok(Some(DataOperator::And(vec![
                DataOperator::GreaterThanOrEqualFloat(min),
                DataOperator::LessThanOrEqualFloat(max),
            ])))
        } else {
            Err(StamError::OtherError(
                "`value_in_range` must be a 2-tuple min,max (exclusive) with numbers (both int or both float)",
            ))
        }
    } else if let Ok(Some(values)) = kwargs.get_item("value_not_in_range") {
        if let Ok((min, max)) = values.extract::<(isize, isize)>() {
            Ok(Some(DataOperator::And(vec![
                DataOperator::LessThan(min),
                DataOperator::GreaterThan(max),
            ])))
        } else if let Ok((min, max)) = values.extract::<(f64, f64)>() {
            Ok(Some(DataOperator::And(vec![
                DataOperator::LessThanFloat(min),
                DataOperator::GreaterThanFloat(max),
            ])))
        } else {
            Err(StamError::OtherError(
                "`value_not_in_range` must be a 2-tuple min,max (exclusive) with numbers (both int or both float)",
            ))
        }
    } else {
        Ok(None)
    }
}

pub(crate) fn dataoperator_from_py(value: &PyAny) -> Result<DataOperator, StamError> {
    if value.is_none() {
        Ok(DataOperator::Null)
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::Equals(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::EqualsInt(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::EqualsFloat(value))
    } else if let Ok(value) = value.extract::<bool>() {
        if value {
            Ok(DataOperator::True)
        } else {
            Ok(DataOperator::False)
        }
    } else {
        Err(StamError::OtherError(
            "Could not convert value to a DataOperator",
        ))
    }
}

pub(crate) fn dataoperator_greater_from_py(value: &PyAny) -> Result<DataOperator, StamError> {
    if let Ok(value) = value.extract() {
        Ok(DataOperator::GreaterThan(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::GreaterThanFloat(value))
    } else {
        Err(StamError::OtherError(
            "Could not convert value to a greater than DataOperator",
        ))
    }
}

pub(crate) fn dataoperator_less_from_py(value: &PyAny) -> Result<DataOperator, StamError> {
    if let Ok(value) = value.extract() {
        Ok(DataOperator::LessThan(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::LessThanFloat(value))
    } else {
        Err(StamError::OtherError(
            "Could not convert value to a less than DataOperator",
        ))
    }
}

pub(crate) fn dataoperator_greatereq_from_py(value: &PyAny) -> Result<DataOperator, StamError> {
    if let Ok(value) = value.extract() {
        Ok(DataOperator::GreaterThanOrEqual(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::GreaterThanOrEqualFloat(value))
    } else {
        Err(StamError::OtherError(
            "Could not convert value to a greater-equal than DataOperator",
        ))
    }
}

pub(crate) fn dataoperator_lesseq_from_py(value: &PyAny) -> Result<DataOperator, StamError> {
    if let Ok(value) = value.extract() {
        Ok(DataOperator::LessThanOrEqual(value))
    } else if let Ok(value) = value.extract() {
        Ok(DataOperator::LessThanOrEqualFloat(value))
    } else {
        Err(StamError::OtherError(
            "Could not convert value to a less-equal than DataOperator",
        ))
    }
}

pub(crate) fn data_request_parser<'py, 'store>(
    kwargs: Option<&'py PyDict>,
    store: &'store AnnotationStore,
    mut set_handle: Option<AnnotationDataSetHandle>,
    mut key_handle: Option<DataKeyHandle>,
) -> Result<
    (
        Option<AnnotationDataSetHandle>,
        Option<DataKeyHandle>,
        DataOperator<'store>,
    ),
    StamError,
>
where
    'py: 'store,
{
    if let Some(kwargs) = kwargs {
        if let Ok(Some(set)) = kwargs.get_item("set") {
            if set.is_instance_of::<PyAnnotationDataSet>() {
                let set: PyRef<'py, PyAnnotationDataSet> =
                    set.extract().expect("extract should succeed");
                set_handle = Some(set.handle);
            } else if set.is_none() || set.is_instance_of::<PyBool>() {
                set_handle = None;
            } else if let Ok(set) = set.extract() {
                set_handle = Some(store.resolve_dataset_id(set)?);
            }
        }
        if let Ok(Some(key)) = kwargs.get_item("key") {
            if key.is_instance_of::<PyDataKey>() {
                let key: PyRef<'py, PyDataKey> = key.extract().expect("extract should succeed");
                set_handle = Some(key.set);
                key_handle = Some(key.handle);
            } else if key.is_none() || key.is_instance_of::<PyBool>() {
                key_handle = None;
            } else if let Ok(key) = key.extract::<&str>() {
                if let Some(set_handle) = set_handle {
                    let set = store.dataset(set_handle).expect("set handle must be valid");
                    if let Some(key) = set.key(key) {
                        key_handle = Some(key.handle());
                    } else {
                        return Err(StamError::IdNotFoundError(
                            key.to_string(),
                            "Specified key does not exist",
                        ));
                    }
                } else {
                    return Err(StamError::OtherError(
                        "Can not specify a key identifier without a set!",
                    ));
                }
            }
        }
        if let Some(op) = dataoperator_from_kwargs(kwargs)? {
            Ok((set_handle, key_handle, op))
        } else {
            Ok((set_handle, key_handle, DataOperator::Any))
        }
    } else {
        Err(StamError::OtherError(
            "No keyword arguments specified for search: provide a combination of `set`,`key`, and `value`",
        ))
    }
}

#[pyclass(name = "Data")]
pub struct PyData {
    pub(crate) data: Vec<(AnnotationDataSetHandle, AnnotationDataHandle)>,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) cursor: usize,
    /// Indicates whether the annotations are sorted chronologically (by handle)
    pub(crate) sorted: bool,
}

#[pymethods]
impl PyData {
    fn __iter__(mut pyself: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        pyself.cursor = 0;
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotationData> {
        pyself.cursor += 1; //increment first (prevent exclusive mutability issues)
        if let Some((set_handle, handle)) = pyself.data.get(pyself.cursor - 1) {
            //index is one ahead, prevents exclusive lock issues
            Some(PyAnnotationData::new(*handle, *set_handle, &pyself.store))
        } else {
            None
        }
    }

    fn __getitem__(pyself: PyRef<'_, Self>, mut index: isize) -> PyResult<PyAnnotationData> {
        if index < 0 {
            index = pyself.data.len() as isize + index;
        }
        if let Some((set_handle, handle)) = pyself.data.get(index as usize) {
            Ok(PyAnnotationData::new(*handle, *set_handle, &pyself.store))
        } else {
            Err(PyIndexError::new_err("data index out of bounds"))
        }
    }

    fn __len__(pyself: PyRef<'_, Self>) -> usize {
        pyself.data.len()
    }

    fn __bool__(pyself: PyRef<'_, Self>) -> bool {
        !pyself.data.is_empty()
    }

    #[pyo3(signature = (**kwargs))]
    fn annotations(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|data, store| {
            let iter = Data::from_handles(Cow::Borrowed(data), self.sorted, store)
                .iter()
                .annotations();
            iterparams.evaluate_to_pyannotations(iter, store, &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|data, store| {
            let iter = Data::from_handles(Cow::Borrowed(data), self.sorted, store)
                .iter()
                .annotations();
            Ok(iterparams.evaluate_annotations(iter, store)?.test())
        })
    }
}

impl PyData {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(
            &Vec<(AnnotationDataSetHandle, AnnotationDataHandle)>,
            &AnnotationStore,
        ) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            f(&self.data, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

impl<'py> IterParams<'py> {
    pub(crate) fn evaluate_to_pydata<'store>(
        self,
        iter: DataIter<'store>,
        store: &'store AnnotationStore,
        wrappedstore: &Arc<RwLock<AnnotationStore>>,
    ) -> Result<PyData, StamError>
    where
        'py: 'store,
    {
        let limit = self.limit();
        match self.evaluate_data(iter, store) {
            Ok(iter) => {
                let sorted = iter.returns_sorted();
                Ok(PyData {
                    data: if let Some(limit) = limit {
                        iter.to_collection_limit(limit).take()
                    } else {
                        iter.to_collection().take()
                    },
                    store: wrappedstore.clone(),
                    cursor: 0,
                    sorted,
                })
            }
            Err(e) => Err(e.into()),
        }
    }
}
