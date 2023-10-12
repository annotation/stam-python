use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotationdata::{datavalue_from_py, PyAnnotationData, PyData, PyDataKey};
use crate::error::PyStamError;
use crate::iterparams::IterParams;
use crate::selector::{PySelector, PySelectorKind};
use stam::*;

#[pyclass(dict, module = "stam", name = "AnnotationDataSet")]
/// An `AnnotationDataSet` stores the keys :obj:`DataKey` and values
/// :obj:`AnnotationData`] (which in turn encapsulates :obj:`DataValue`) that are used by annotations.
/// It effectively defines a certain vocabulary, i.e. key/value pairs.
/// The `AnnotationDataSet` does not store the :obj:`Annotation` instances themselves, those are in
/// the :obj:`AnnotationStore`. The datasets themselves are also held by the `AnnotationStore`.
pub(crate) struct PyAnnotationDataSet {
    pub(crate) handle: AnnotationDataSetHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

impl PyAnnotationDataSet {
    pub(crate) fn new(
        handle: AnnotationDataSetHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyAnnotationDataSet {
        PyAnnotationDataSet {
            handle,
            store: store.clone(),
        }
    }

    pub(crate) fn new_py<'py>(
        handle: AnnotationDataSetHandle,
        store: &Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::new(handle, store).into_py(py).into_ref(py)
    }
}

#[pymethods]
impl PyAnnotationDataSet {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|annotationset| Ok(annotationset.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|annotationset| Ok(annotationset.id() == Some(other)))
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

    /// Save the annotation dataset to a STAM JSON file
    fn to_json_file(&self, filename: &str) -> PyResult<()> {
        self.map(|annotationset| {
            annotationset
                .as_ref()
                .to_json_file(filename, annotationset.as_ref().config())
        })
    }

    /// Returns the annotation stdataset as one big STAM JSON string
    fn to_json_string(&self) -> PyResult<String> {
        self.map(|annotationset| {
            annotationset
                .as_ref()
                .to_json_string(annotationset.as_ref().config())
        })
    }

    /// Get a DataKey instance by ID, raises an exception if not found
    fn key(&self, key: &str) -> PyResult<PyDataKey> {
        self.map(|annotationset| {
            Ok(annotationset
                .key(key)
                .map(|key| PyDataKey::new(key.handle(), self.handle, &self.store))
                .ok_or_else(|| StamError::IdNotFoundError(key.to_string(), "key not found"))?)
        })
    }

    /// Create a new DataKey and adds it to the dataset
    fn add_key(&self, key: &str) -> PyResult<PyDataKey> {
        self.map_mut(|annotationset| {
            let datakey = DataKey::new(key.to_string());
            let handle = annotationset.insert(datakey)?;
            Ok(PyDataKey::new(handle, self.handle, &self.store))
        })
    }

    /// Returns the number of keys in the set (not substracting deletions)
    fn keys_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.as_ref().keys_len()))
    }

    /// Returns the number of annotations in the set (not substracting deletions)
    fn data_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.as_ref().data_len()))
    }

    /// Create a new AnnotationData instance and adds it to the dataset
    fn add_data<'py>(
        &self,
        key: &str,
        value: &'py PyAny,
        id: Option<&str>,
    ) -> PyResult<PyAnnotationData> {
        let datakey = if let Ok(datakey) = self.key(key) {
            datakey
        } else {
            self.add_key(key)?
        };
        self.map_mut(|annotationset| {
            let value = datavalue_from_py(value)?;
            let mut databuilder = AnnotationDataBuilder::new()
                .with_key(datakey.handle.into())
                .with_value(value);
            if let Some(id) = id {
                databuilder = databuilder.with_id(id.into());
            }
            let handle = annotationset.build_insert_data(databuilder, true)?;
            Ok(PyAnnotationData::new(handle, self.handle, &self.store))
        })
    }

    /// Get a AnnotationData instance by id, raises an exception if not found
    fn annotationdata(&self, data_id: &str) -> PyResult<PyAnnotationData> {
        self.map(|annotationset| {
            Ok(annotationset
                .annotationdata(data_id)
                .map(|data| PyAnnotationData::new(data.handle(), self.handle, &self.store))
                .ok_or_else(|| {
                    StamError::IdNotFoundError(data_id.to_string(), "annotationdata not found")
                })?)
        })
    }

    /// Returns a generator over all keys in this store
    fn keys(&self) -> PyResult<PyDataKeyIter> {
        Ok(PyDataKeyIter {
            handle: self.handle,
            store: self.store.clone(),
            index: 0,
        })
    }

    /// Returns a generator over all data in this store
    fn __iter__(&self) -> PyResult<PyAnnotationDataIter> {
        Ok(PyAnnotationDataIter {
            handle: self.handle,
            store: self.store.clone(),
            index: 0,
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn data(&self, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|dataset| {
            let iter = dataset.data();
            iterparams.evaluate_to_pydata(iter, dataset.store(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_data(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|dataset| {
            let iter = dataset.data();
            Ok(iterparams.evaluate_data(iter, dataset.store())?.test())
        })
    }

    /// Returns a Selector (DataSetSelector) pointing to this AnnotationDataSet
    fn select(&self) -> PyResult<PySelector> {
        self.map(|dataset| {
            Ok(PySelector {
                kind: PySelectorKind {
                    kind: SelectorKind::DataSetSelector,
                },
                dataset: Some(dataset.handle()),
                annotation: None,
                resource: None,
                offset: None,
                subselectors: Vec::new(),
            })
        })
    }
}

impl PyAnnotationDataSet {
    /// Map function to act on the actual underlyingtore, helps reduce boilerplate
    pub(crate) fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(ResultItem<AnnotationDataSet>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotationset: ResultItem<AnnotationDataSet> = store
                .dataset(self.handle)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolved annotationset"))?;
            f(annotationset).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    /// Map function to act on the actual underlying store mutably, helps reduce boilerplate
    pub(crate) fn map_mut<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&mut AnnotationDataSet) -> Result<T, StamError>,
    {
        if let Ok(mut store) = self.store.write() {
            let annotationset: &mut AnnotationDataSet = store
                .get_mut(self.handle)
                .map_err(|err| PyStamError::new_err(format!("{}", err)))?;
            f(annotationset).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Can't get exclusive lock to write to store",
            ))
        }
    }
}

#[pyclass(name = "DataKeyIter")]
struct PyDataKeyIter {
    pub(crate) handle: AnnotationDataSetHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyDataKeyIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyDataKey> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|dataset| {
            let datakey_handle = DataKeyHandle::new(pyself.index - 1);
            if dataset.as_ref().has(datakey_handle) {
                //index is one ahead, prevents exclusive lock issues
                Some(PyDataKey {
                    set: pyself.handle,
                    handle: datakey_handle,
                    store: pyself.store.clone(),
                })
            } else {
                None
            }
        });
        if result.is_some() {
            result
        } else {
            if pyself.index
                >= pyself
                    .map(|dataset| Some(dataset.as_ref().keys_len()))
                    .unwrap()
            {
                None
            } else {
                Self::__next__(pyself)
            }
        }
    }
}

impl PyDataKeyIter {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(ResultItem<'_, AnnotationDataSet>) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            if let Some(annotationset) = store.dataset(self.handle) {
                f(annotationset)
            } else {
                None
            }
        } else {
            None //should never happen
        }
    }
}

#[pyclass(name = "AnnotationDataIter")]
struct PyAnnotationDataIter {
    pub(crate) handle: AnnotationDataSetHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyAnnotationDataIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotationData> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|dataset| {
            let data_handle = AnnotationDataHandle::new(pyself.index - 1);
            if dataset.as_ref().has(data_handle) {
                //index is one ahead, prevents exclusive lock issues
                Some(PyAnnotationData::new(
                    data_handle,
                    pyself.handle,
                    &pyself.store,
                ))
            } else {
                None
            }
        });
        if result.is_some() {
            result
        } else {
            if pyself.index
                >= pyself
                    .map(|dataset| Some(dataset.as_ref().keys_len()))
                    .unwrap()
            {
                None
            } else {
                Self::__next__(pyself)
            }
        }
    }
}

impl PyAnnotationDataIter {
    /// Map function to act on the actual underlyingtore, helps reduce boilerplate
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(ResultItem<'_, AnnotationDataSet>) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            if let Some(annotationset) = store.dataset(self.handle) {
                f(annotationset)
            } else {
                None
            }
        } else {
            None //should never happen
        }
    }
}
