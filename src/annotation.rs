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

impl PyAnnotation {
    pub(crate) fn new(
        handle: AnnotationHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyAnnotation {
        PyAnnotation {
            handle,
            store: store.clone(),
        }
    }

    pub(crate) fn new_py<'py>(
        handle: AnnotationHandle,
        store: &Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::new(handle, store).into_py(py).into_ref(py)
    }
}

#[pymethods]
impl PyAnnotation {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|annotation| Ok(annotation.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the item
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
        self.map(|annotation| annotation.as_ref().selector().map(|sel| sel.into()))
    }

    /// Returns the text of the annotation.
    /// Note that this will always return a list (even it if only contains a single element),
    /// as an annotation may reference multiple texts.
    ///
    /// If you are sure an annotation only references a single contingent text slice or are okay with slices being concatenated, then you can use `str()` instead.
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
                let resource_handle = textselection.resource().handle();
                list.append(PyTextSelection::from_result_to_py(
                    textselection,
                    &self.store,
                    py,
                ))
                .ok();
            }
            Ok(())
        })
        .ok();
        list.into()
    }

    /// Returns a list of annotations this annotation refers to (i.e. using an AnnotationSelector)
    #[pyo3(signature = (recursive=false,limit=None))]
    fn annotations_in_targets<'py>(
        &self,
        recursive: bool,
        limit: Option<usize>,
        py: Python<'py>,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation
                .annotations_in_targets(recursive, false)
                .enumerate()
            {
                list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
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
    fn annotations<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation.annotations().enumerate() {
                list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
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
                list.append(PyTextResource::new_py(resource.handle(), &self.store, py))
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

    /// Returns the resources this annotation refers to (as metadata)
    /// They will be returned in a tuple.
    #[pyo3(signature = (limit=None))]
    fn datasets<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, dataset) in annotation.datasets().enumerate() {
                list.append(PyAnnotationDataSet::new_py(
                    dataset.handle(),
                    &self.store,
                    py,
                ))
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
    /// If you know what you are interested in; you can use :meth:`annotations`, :meth:`resources`, :meth:`annotationset`, :meth:`textselections` instead"""
    fn target(&self) -> PyResult<PySelector> {
        self.map_store(|store| {
            let annotation: &Annotation = store.get(self.handle)?;
            Ok(PySelector {
                selector: annotation.target().clone(),
            })
        })
    }

    /// Returns a list of annotation data instances this annotation refers to.
    #[pyo3(signature = (limit=None))]
    fn data<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, data) in annotation.data().enumerate() {
                list.append(PyAnnotationData::new_py(
                    data.handle(),
                    data.set().handle(),
                    &self.store,
                    py,
                ))
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

    /// Returns the number of data items under this annotation
    fn __len__(&self) -> usize {
        self.map(|annotation| Ok(annotation.as_ref().len()))
            .unwrap()
    }

    /// Applies a `TextSelectionOperator` to find all other text selections
    /// are in a specific relation with the ones from the current annotations. Returns all matching TextSelections in a list.
    ///
    /// If you are interested in the annotations associated with the found text selections, then
    /// use `find_annotations()` instead.
    #[pyo3(signature = (operator,limit=None))]
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, foundtextselection) in annotation.related_text(operator.operator).enumerate() {
                list.append(PyTextSelection::from_result_to_py(
                    foundtextselection,
                    &self.store,
                    py,
                ))
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
    fn annotations_by_related_text(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, annotation) in annotation
                .annotations_by_related_text(operator.operator)
                .enumerate()
            {
                list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
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
            if let Some((set, handle)) = annotation.as_ref().data_by_index(pyself.index - 1) {
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
        F: FnOnce(ResultItem<'_, Annotation>) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            if let Some(annotation) = store.annotation(self.handle) {
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
        F: FnOnce(ResultItem<'_, Annotation>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let annotation: ResultItem<'_, Annotation> = store
                .annotation(self.handle)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(annotation).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}
