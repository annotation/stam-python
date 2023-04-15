use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotationdata::PyAnnotationData;
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use crate::resources::PyTextResource;
use crate::selector::PySelector;
use crate::textselection::{PyTextSelection, PyTextSelectionOperator};
use stam::*;

#[pyclass(dict, module = "stam", name = "Annotation")]
/// `Annotation` represents a particular *instance of annotation* and is the central
/// concept of the model. They can be considered the primary nodes of the graph model. The
/// instance of annotation is strictly decoupled from the *data* or key/value of the
/// annotation (:obj:`AnnotationData`). After all, multiple instances can be annotated
/// with the same label (multiple annotations may share the same annotation data).
/// Moreover, an `Annotation` can have multiple annotation data associated.
/// The result is that multiple annotations with the exact same content require less storage
/// space, and searching and indexing is facilitated.  
///
/// This structure is not instantiated directly, only returned.
pub(crate) struct PyAnnotation {
    pub(crate) handle: AnnotationHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyAnnotation {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|annotation| Ok(annotation.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|annotation| Ok(annotation.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    /// Returns a generator over all data in this annotation
    fn __iter__(&self) -> PyResult<PyDataIter> {
        Ok(PyDataIter {
            handle: self.handle,
            store: self.store.clone(),
            index: 0,
        })
    }

    /// Returns a Selector (AnnotationSelector) pointing to this Annotation
    /// If the annotation references any text, so will this
    fn selector(&self) -> PyResult<PySelector> {
        self.map(|annotation| annotation.selector().map(|sel| sel.into()))
    }

    /// Returns the text of the annotation.
    /// Note that this will always return a list (even it if only contains a single element),
    /// as an annotation may reference multiple texts.
    ///
    /// If you are sure an annotation only reference a single contingent text slice or are okay with slices being concatenated, then you can use `str()` instead.
    fn text<'py>(&self, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for text in annotation.text() {
                list.append(text).ok();
            }
            Ok(())
        })
        .ok();
        list.into()
    }

    /// Returns the text of the annotation.
    /// If the annotation references multiple text slices, they will be concatenated with a space as a delimiter,
    /// but note that in reality the different parts may be non-contingent!
    ///
    /// Use `text()` instead to retrieve a list of texts
    fn __str__(&self) -> PyResult<String> {
        self.map(|annotation| {
            let elements: Vec<&str> = annotation.text().collect();
            let result: String = elements.join(" ");
            Ok(result)
        })
    }

    /// Returns a list of all textselections of the annotation.
    /// Note that this will always return a list (even it if only contains a single element),
    /// as an annotation may reference multiple text selections.
    fn textselections<'py>(&self, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for textselection in annotation.textselections() {
                let resource_handle = textselection
                    .resource()
                    .handle()
                    .expect("Resource must have handle");
                list.append(
                    PyTextSelection {
                        textselection: if textselection.is_borrowed() {
                            textselection.unwrap().clone()
                        } else {
                            textselection.unwrap_owned()
                        },
                        resource_handle,
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

    /// Returns a list of annotations this annotation refers to (i.e. using an AnnotationSelector)
    #[pyo3(signature = (recursive=false,limit=None))]
    fn annotations<'py>(
        &self,
        recursive: bool,
        limit: Option<usize>,
        py: Python<'py>,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation.annotations(recursive, false).enumerate() {
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

    /// Returns a list of annotations that are referring to this annotation (i.e. others using an AnnotationSelector)
    #[pyo3(signature = (limit=None))]
    fn annotations_reverse<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation
                .annotations_reverse()
                .into_iter()
                .flatten()
                .enumerate()
            {
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

    /// Returns a list of resources this annotation refers to
    #[pyo3(signature = (limit=None))]
    fn resources<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, resource) in annotation.resources().enumerate() {
                list.append(
                    PyTextResource {
                        handle: resource.handle().expect("must have handle"),
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

    /// Returns the resources this annotation refers to
    /// They will be returned in a tuple.
    #[pyo3(signature = (limit=None))]
    fn annotationsets<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, dataset) in annotation.annotationsets().enumerate() {
                list.append(
                    PyAnnotationDataSet {
                        handle: dataset.handle().expect("must have handle"),
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

    /// Returns the target selector for this annotation
    fn target(&self) -> PyResult<PySelector> {
        self.map_store(|store| {
            let annotation: &Annotation = store.get(&self.handle.into())?;
            Ok(PySelector {
                selector: annotation.target().clone(),
            })
        })
    }

    /// Returns the resources this annotation refers to
    /// They will be returned in a tuple.
    #[pyo3(signature = (limit=None))]
    fn data<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, data) in annotation.data().enumerate() {
                list.append(
                    PyAnnotationData {
                        handle: data.handle().expect("annotationdata must have handle"),
                        set: data.set().handle().expect("set must have handle"),
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

    /// Applies a `TextSelectionOperator` to find all other annotations whose text selections
    /// are in a specific relation with the current one. Returns all matching TextSelections in a list
    ///
    /// If you are interested in the annotations associated with the found text selections, then
    /// use `find_annotations()` instead.
    #[pyo3(signature = (operator,limit=None))]
    fn find_textselections(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, foundtextselection) in annotation
                .find_textselections(operator.operator)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let resource_handle = foundtextselection
                    .resource()
                    .handle()
                    .expect("resource must have handle");
                list.append(
                    PyTextSelection {
                        textselection: if foundtextselection.is_borrowed() {
                            foundtextselection.unwrap().clone()
                        } else {
                            foundtextselection.unwrap_owned()
                        },
                        resource_handle,
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
    /// Applies a `TextSelectionOperator` to find all other annotations whose text selections
    /// are in a specific relation with the text selections of the current one. Returns all matching Annotations in a list
    #[pyo3(signature = (operator,limit=None))]
    fn find_annotations(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation
                .find_annotations(operator.operator)
                .into_iter()
                .flatten()
                .enumerate()
            {
                list.append(
                    PyAnnotation {
                        handle: annotation.handle().expect("annotation must have a handle"),
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

#[pyclass(name = "DataIter")]
struct PyDataIter {
    pub(crate) handle: AnnotationHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyDataIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotationData> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        pyself.map(|annotation| {
            if let Some((set, handle)) = annotation.data_by_index(pyself.index - 1) {
                //index is one ahead, prevents exclusive lock issues
                Some(PyAnnotationData {
                    set: *set,
                    handle: *handle,
                    store: pyself.store.clone(),
                })
            } else {
                None
            }
        })
    }
}

impl PyDataIter {
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(WrappedItem<'_, Annotation>) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            if let Some(annotation) = store.annotation(&self.handle.into()) {
                f(annotation)
            } else {
                None
            }
        } else {
            None //should never happen here
        }
    }
}

impl MapStore for PyAnnotation {
    fn get_store(&self) -> &Arc<RwLock<AnnotationStore>> {
        &self.store
    }
    fn get_store_mut(&mut self) -> &mut Arc<RwLock<AnnotationStore>> {
        &mut self.store
    }
}

impl PyAnnotation {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<'_, Annotation>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotation: WrappedItem<'_, Annotation> = store
                .annotation(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(annotation).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    /// Map function to act on the actual underlying store, helps reduce boilerplate
    fn map_mut<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&mut Annotation) -> Result<T, StamError>,
    {
        if let Ok(mut store) = self.store.write() {
            let annotation: &mut Annotation = store
                .get_mut(&self.handle.into())
                .map_err(|_| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(annotation).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}
