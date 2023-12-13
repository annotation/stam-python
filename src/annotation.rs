use pyo3::exceptions::{PyIndexError, PyRuntimeError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::borrow::Cow;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotationdata::{PyAnnotationData, PyData};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::MapStore;
use crate::error::PyStamError;
use crate::iterparams::*;
use crate::resources::{PyOffset, PyTextResource};
use crate::selector::{PySelector, PySelectorKind};
use crate::textselection::{PyTextSelectionOperator, PyTextSelections};
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
    /// but note that in reality the different parts may be non-contingent or non-delimited!
    ///
    /// Use `text()` instead to retrieve a list of texts, which you can subsequently concatenate as you please.
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
    #[pyo3(signature = (*args, **kwargs))]
    fn textselections(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotation| {
                Ok(PyTextSelections::from_iter(
                    annotation.textselections().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::TextSelection,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |annotation, query| {
                    Ok(PyTextSelections::from_query(
                        query,
                        annotation.store(),
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    /// Returns annotations this annotation refers to (i.e. using an AnnotationSelector)
    #[pyo3(signature = (*args, **kwargs))]
    fn annotations_in_targets(
        &self,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        let recursive = get_recursive(kwargs, false);
        if !has_filters(args, kwargs) {
            self.map(|annotation| {
                Ok(PyAnnotations::from_iter(
                    annotation.annotations_in_targets(recursive).limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    if recursive {
                        AnnotationQualifier::RecursiveTarget
                    } else {
                        AnnotationQualifier::Target
                    },
                ),
                args,
                kwargs,
                |annotation, query| {
                    Ok(PyAnnotations::from_query(
                        query,
                        annotation.store(),
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    /// Returns annotations that are referring to this annotation (i.e. others using an AnnotationSelector)
    #[pyo3(signature = (*args, **kwargs))]
    fn annotations(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotation| {
                Ok(PyAnnotations::from_iter(
                    annotation.annotations().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |annotation, query| {
                    Ok(PyAnnotations::from_query(
                        query,
                        annotation.store(),
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.annotations().test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query).test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations_in_targets(
        &self,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<bool> {
        let recursive = get_recursive(kwargs, false);
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.annotations_in_targets(recursive).test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    if recursive {
                        AnnotationQualifier::RecursiveTarget
                    } else {
                        AnnotationQualifier::Target
                    },
                ),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query).test()),
            )
        }
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

    /// Returns the target selector
    fn target(&self) -> PyResult<PySelector> {
        self.map_store(|store| {
            let annotation: &Annotation = store.get(self.handle)?;
            let target = annotation.target();
            Ok(PySelector::from_selector(target, store))
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

    /// Returns annotation data instances that pertain to this annotation.
    #[pyo3(signature = (*args, **kwargs))]
    fn data(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotation| {
                Ok(PyData::from_iter(
                    annotation.data().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |annotation, query| {
                    Ok(PyData::from_query(
                        query,
                        annotation.store(),
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_data(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.data().test()))
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query).test()),
            )
        }
    }

    /// Returns the number of data items under this annotation
    fn __len__(&self) -> usize {
        self.map(|annotation| Ok(annotation.as_ref().len()))
            .unwrap()
    }

    #[pyo3(signature = (operator, *args, **kwargs))]
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotation| {
                Ok(PyTextSelections::from_iter(
                    annotation.related_text(operator.operator).limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::TextSelection,
                Constraint::TextRelation {
                    var: "main",
                    operator: operator.operator, //MAYBE TODO: check if we need to invert an operator here?
                },
                args,
                kwargs,
                |annotation, query| {
                    Ok(PyTextSelections::from_query(
                        query,
                        annotation.store(),
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }
}

#[pyclass(name = "Annotations")]
pub struct PyAnnotations {
    pub(crate) annotations: Vec<AnnotationHandle>,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) cursor: usize,
}

#[pymethods]
impl PyAnnotations {
    fn __iter__(mut pyself: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        pyself.cursor = 0;
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

    fn __getitem__(pyself: PyRef<'_, Self>, mut index: isize) -> PyResult<PyAnnotation> {
        if index < 0 {
            index = pyself.annotations.len() as isize + index;
        }
        if let Some(handle) = pyself.annotations.get(index as usize) {
            Ok(PyAnnotation::new(*handle, &pyself.store))
        } else {
            Err(PyIndexError::new_err("annotation index out of bounds"))
        }
    }

    fn __len__(pyself: PyRef<'_, Self>) -> usize {
        pyself.annotations.len()
    }

    fn __bool__(pyself: PyRef<'_, Self>) -> bool {
        !pyself.annotations.is_empty()
    }

    /// Returns annotation data instances used by the annotations in this collection.
    #[pyo3(signature = (*args, **kwargs))]
    fn data(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, store| {
                Ok(PyData::from_iter(
                    annotations.items().data().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |query, store| Ok(PyData::from_query(query, store, &self.store, limit)),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_data(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| Ok(annotations.items().data().test()))
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |query, store| Ok(store.query(query).test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn annotations(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, store| {
                Ok(PyAnnotations::from_iter(
                    annotations.items().annotations().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |query, store| Ok(PyAnnotations::from_query(query, store, &self.store, limit)),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| Ok(annotations.items().annotations().test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |query, store| Ok(store.query(query).test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn annotations_in_targets(
        &self,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        let recursive = get_recursive(kwargs, false);
        if !has_filters(args, kwargs) {
            self.map(|annotations, store| {
                Ok(PyAnnotations::from_iter(
                    annotations
                        .items()
                        .annotations_in_targets(recursive)
                        .limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    if recursive {
                        AnnotationQualifier::RecursiveTarget
                    } else {
                        AnnotationQualifier::Target
                    },
                ),
                args,
                kwargs,
                |query, store| Ok(PyAnnotations::from_query(query, store, &self.store, limit)),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations_in_targets(
        &self,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<bool> {
        let recursive = get_recursive(kwargs, false);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| {
                Ok(annotations.items().annotations_in_targets(recursive).test())
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    if recursive {
                        AnnotationQualifier::RecursiveTarget
                    } else {
                        AnnotationQualifier::Target
                    },
                ),
                args,
                kwargs,
                |query, store| Ok(store.query(query).test()),
            )
        }
    }

    #[pyo3(signature = (*args,**kwargs))]
    fn textselections(&self, args: &PyList, kwargs: Option<&PyDict>) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, store| {
                Ok(PyTextSelections::from_iter(
                    annotations.items().textselections().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::TextSelection,
                Constraint::AnnotationVariable("main", AnnotationQualifier::None),
                args,
                kwargs,
                |query, store| {
                    Ok(PyTextSelections::from_query(
                        query,
                        store,
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    #[pyo3(signature = (operator, *args, **kwargs))]
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        args: &PyList,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, store| {
                Ok(PyTextSelections::from_iter(
                    annotations
                        .items()
                        .related_text(operator.operator)
                        .limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::TextSelection,
                Constraint::TextRelation {
                    var: "main",
                    operator: operator.operator, //MAYBE TODO: check if we need to invert an operator here?
                },
                args,
                kwargs,
                |query, store| {
                    Ok(PyTextSelections::from_query(
                        query,
                        store,
                        &self.store,
                        limit,
                    ))
                },
            )
        }
    }

    fn textual_order(mut pyself: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        pyself
            .map_mut(|annotations, store| {
                annotations.sort_unstable_by(|a, b| {
                    let a = store
                        .annotation(*a)
                        .expect("annotation handle must be valid!");
                    let b = store
                        .annotation(*b)
                        .expect("annotation handle must be valid!");
                    compare_annotation_textual_order(&a, &b)
                });
                Ok(())
            })
            .unwrap();
        pyself
    }
}

impl PyAnnotations {
    pub(crate) fn from_iter<'store>(
        iter: impl Iterator<Item = ResultItem<'store, Annotation>>,
        wrappedstore: &Arc<RwLock<AnnotationStore>>,
    ) -> Self {
        Self {
            annotations: iter.map(|x| x.handle()).collect(),
            store: wrappedstore.clone(),
            cursor: 0,
        }
    }

    pub(crate) fn from_query<'store>(
        query: Query<'store>,
        store: &'store AnnotationStore,
        wrappedstore: &Arc<RwLock<AnnotationStore>>,
        limit: Option<usize>,
    ) -> Self {
        assert!(query.resulttype() == Some(Type::Annotation));
        Self {
            annotations: store
                .query(query)
                .limit(limit)
                .map(|resultitems| {
                    //we use the deepest item if there are multiple
                    if let Some(QueryResultItem::Annotation(annotation)) = resultitems.pop_last() {
                        annotation.handle()
                    } else {
                        unreachable!("Unexpected QueryResultItem");
                    }
                })
                .collect(),
            store: wrappedstore.clone(),
            cursor: 0,
        }
    }

    fn map<'store, T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(Handles<'store, Annotation>, &'store AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let handles = Annotations::new(Cow::Borrowed(&self.annotations), false, &store);
            f(handles, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn map_mut<T, F>(&mut self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&mut Vec<AnnotationHandle>, &AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            f(&mut self.annotations, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn map_with_query<'a, T, F>(
        &'a self,
        resulttype: Type,
        constraint: Constraint<'a>,
        args: &PyList,
        kwargs: Option<&PyDict>,
        f: F,
    ) -> Result<T, PyErr>
    where
        F: FnOnce(Query<'a>, &'a AnnotationStore) -> Result<T, StamError>,
    {
        self.map(|annotations, store| {
            let query = Query::new(QueryType::Select, Some(Type::Annotation), Some("main"))
                .with_constraint(Constraint::Handle(Filter::Annotations(
                    annotations,
                    FilterMode::Any,
                )))
                .with_subquery(
                    build_query(
                        Query::new(QueryType::Select, Some(resulttype), Some("sub"))
                            .with_constraint(constraint),
                        args,
                        kwargs,
                        store,
                    )
                    .map_err(|e| {
                        StamError::QuerySyntaxError(format!("{}", e), "(python to query)")
                    })?,
                );
            f(query, store)
        })
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

    fn map_with_query<'a, T, F>(
        &'a self,
        resulttype: Type,
        constraint: Constraint<'a>,
        args: &PyList,
        kwargs: Option<&PyDict>,
        f: F,
    ) -> Result<T, PyErr>
    where
        F: FnOnce(ResultItem<'a, Annotation>, Query<'a>) -> Result<T, StamError>,
    {
        self.map(|annotation| {
            let query = Query::new(QueryType::Select, Some(Type::Annotation), Some("main"))
                .with_constraint(Constraint::Handle(Filter::Annotation(annotation.handle())))
                .with_subquery(
                    build_query(
                        Query::new(QueryType::Select, Some(resulttype), Some("sub"))
                            .with_constraint(constraint),
                        args,
                        kwargs,
                        annotation.store(),
                    )
                    .map_err(|e| {
                        StamError::QuerySyntaxError(format!("{}", e), "(python to query)")
                    })?,
                );
            f(annotation, query)
        })
    }
}
