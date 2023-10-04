use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::borrow::Cow;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotationdata::{data_request_parser, PyAnnotationData, PyData};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use crate::get_limit;
use crate::iterparams::IterParams;
use crate::resources::{PyOffset, PyTextResource};
use crate::selector::{PySelector, PySelectorKind};
use crate::textselection::{PyTextSelection, PyTextSelectionOperator, PyTextSelections};
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
pub struct PyAnnotation {
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

    /// Returns a generator over all data in this annotation, this is quick and performant but less suitable if you want to do further filtering, use :meth:`data` instead for that
    fn __iter__(&self) -> PyResult<PyDataIter> {
        Ok(PyDataIter {
            handle: self.handle,
            store: self.store.clone(),
            index: 0,
        })
    }

    /// Returns a Selector (AnnotationSelector) pointing to this Annotation
    /// If the annotation references any text, so will this
    fn select(&self) -> PyResult<PySelector> {
        self.map(|annotation| {
            Ok(PySelector {
                kind: PySelectorKind {
                    kind: SelectorKind::AnnotationSelector,
                },
                annotation: Some(annotation.handle()),
                resource: None,
                dataset: None,
                offset: if annotation
                    .as_ref()
                    .target()
                    .offset(annotation.store())
                    .is_some()
                {
                    Some(PyOffset {
                        offset: Offset::whole(),
                    })
                } else {
                    None
                },
                subselectors: Vec::new(),
            })
        })
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
    fn textselections<'py>(&self, py: Python<'py>) -> PyResult<PyTextSelections> {
        self.map(|annotation| {
            let iter = annotation.textselections();
            Ok(PyTextSelections {
                textselections: iter.to_handles(),
                store: self.store.clone(),
                cursor: 0,
            })
        })
    }

    /// Returns annotations this annotation refers to (i.e. using an AnnotationSelector)
    fn annotations_in_targets<'py>(
        &self,
        kwargs: Option<&PyDict>,
        py: Python<'py>,
    ) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        let mut recursive: bool = false;
        if let Some(kwargs) = kwargs {
            if let Some(v) = kwargs.get_item("recursive") {
                if let Ok(v) = v.extract() {
                    recursive = v;
                }
            }
        }
        self.map(|annotation| {
            let mut iter = annotation.annotations_in_targets(recursive);
            iterparams.evaluate_to_pyannotations(iter, annotation.store(), &self.store)
        })
    }

    /// Returns annotations that are referring to this annotation (i.e. others using an AnnotationSelector)
    fn annotations<'py>(
        &self,
        kwargs: Option<&PyDict>,
        py: Python<'py>,
    ) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|annotation| {
            let mut iter = annotation.annotations();
            iterparams.evaluate_to_pyannotations(iter, annotation.store(), &self.store)
        })
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

    /// Returns the resources this annotation refers to (as metadata) in a list
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

    /// Returns the offset this annotation points to using its selector
    fn offset(&self) -> PyResult<Option<PyOffset>> {
        self.map_store(|store| {
            let annotation: &Annotation = store.get(self.handle)?;
            Ok(annotation
                .target()
                .offset(store)
                .map(|offset| PyOffset { offset }))
        })
    }

    /// Returns the type of the selector
    fn selector_kind(&self) -> PyResult<PySelectorKind> {
        self.map_store(|store| {
            let annotation: &Annotation = store.get(self.handle)?;
            Ok(PySelectorKind {
                kind: annotation.target().kind(),
            })
        })
    }

    /// Returns a (possibly heterogeneous) list of all the direct targets of this annotation,
    fn targets(&self) -> Py<PyList> {
        todo!() //TODO: implement
    }

    /// Returns annotation data instances that pertain to this annotation.
    fn data<'py>(&self, kwargs: Option<&PyDict>, py: Python<'py>) -> PyResult<PyData> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|annotation| {
            let mut iter = annotation.data();
            iterparams.evaluate_to_pydata(iter, annotation.store(), &self.store)
        })
    }

    /// Returns the number of data items under this annotation
    fn __len__(&self) -> usize {
        self.map(|annotation| Ok(annotation.as_ref().len()))
            .unwrap()
    }

    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        py: Python,
    ) -> PyResult<PyTextSelections> {
        self.map(|annotation| {
            let iter = annotation.related_text(operator.operator);
            Ok(PyTextSelections {
                textselections: iter.to_handles(),
                store: self.store.clone(),
                cursor: 0,
            })
        })
    }
}

#[pyclass(name = "Annotations")]
pub struct PyAnnotations {
    pub(crate) annotations: Vec<AnnotationHandle>,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) cursor: usize,
    /// Indicates whether the annotations are sorted chronologically (by handle)
    pub(crate) sorted: bool,
}

#[pymethods]
impl PyAnnotations {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotation> {
        pyself.cursor += 1; //increment first (prevent exclusive mutability issues)
        if let Some(handle) = pyself.annotations.get(pyself.cursor - 1) {
            //index is one ahead, prevents exclusive lock issues
            Some(PyAnnotation::new(*handle, &pyself.store))
        } else {
            None
        }
    }

    fn __len__(pyself: PyRef<'_, Self>) -> usize {
        pyself.annotations.len()
    }

    fn is_sorted(pyself: PyRef<'_, Self>) -> bool {
        pyself.sorted
    }

    /// Returns annotation data instances used by the annotations in this collection.
    fn data<'py>(&self, kwargs: Option<&PyDict>, py: Python<'py>) -> PyResult<PyData> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|annotations, store| {
            let mut iter =
                Annotations::from_handles(Cow::Borrowed(annotations), self.sorted, store)
                    .iter()
                    .data();
            iterparams.evaluate_to_pydata(iter, store, &self.store)
        })
    }

    fn annotations<'py>(
        &self,
        kwargs: Option<&PyDict>,
        py: Python<'py>,
    ) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|annotations, store| {
            let mut iter =
                Annotations::from_handles(Cow::Borrowed(annotations), self.sorted, store)
                    .iter()
                    .annotations();
            iterparams.evaluate_to_pyannotations(iter, store, &self.store)
        })
    }

    fn annotations_in_targets<'py>(
        &self,
        kwargs: Option<&PyDict>,
        py: Python<'py>,
    ) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        let mut recursive: bool = false;
        if let Some(kwargs) = kwargs {
            if let Some(v) = kwargs.get_item("recursive") {
                if let Ok(v) = v.extract() {
                    recursive = v;
                }
            }
        }
        self.map(|annotations, store| {
            let mut iter =
                Annotations::from_handles(Cow::Borrowed(annotations), self.sorted, store)
                    .iter()
                    .annotations_in_targets(recursive);
            iterparams.evaluate_to_pyannotations(iter, store, &self.store)
        })
    }
}

impl PyAnnotations {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&Vec<AnnotationHandle>, &AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            f(&self.annotations, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

#[pyclass(name = "DataIter")]
struct PyDataIter {
    //This is NOT the counterpart of DataIter in Rust
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

impl<'py> IterParams<'py> {
    pub(crate) fn evaluate_to_pyannotations<'store>(
        self,
        mut iter: AnnotationsIter<'store>,
        store: &'store AnnotationStore,
        wrappedstore: &Arc<RwLock<AnnotationStore>>,
    ) -> Result<PyAnnotations, StamError>
    where
        'py: 'store,
    {
        let limit = self.limit();
        match self.evaluate_annotations(iter, store) {
            Ok(iter) => {
                let sorted = iter.returns_sorted();
                Ok(PyAnnotations {
                    annotations: if let Some(limit) = limit {
                        iter.to_cache_limit(limit).take()
                    } else {
                        iter.to_cache().take()
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
