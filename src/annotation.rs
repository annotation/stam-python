use pyo3::exceptions::PyValueError;
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
use crate::query::*;
use crate::resources::{PyOffset, PyTextResource};
use crate::selector::{PySelector, PySelectorKind};
use crate::substore::PyAnnotationSubStore;
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
        store: Arc<RwLock<AnnotationStore>>,
    ) -> PyAnnotation {
        PyAnnotation { handle, store }
    }

    pub(crate) fn new_py<'py>(
        handle: AnnotationHandle,
        store: Arc<RwLock<AnnotationStore>>,
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
                key: None,
                data: None,
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
    fn textselections(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
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
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| {
                    PyTextSelections::from_query(query, annotation.store(), &self.store, limit)
                },
            )
        }
    }

    /// Returns annotations this annotation refers to (i.e. using an AnnotationSelector)
    #[pyo3(signature = (*args, **kwargs))]
    fn annotations_in_targets(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        let recursive = get_recursive(kwargs, AnnotationDepth::One);
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
                    SelectionQualifier::Metadata,
                    recursive,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| {
                    PyAnnotations::from_query(query, annotation.store(), &self.store, limit)
                },
            )
        }
    }

    /// Returns annotations that are referring to this annotation (i.e. others using an AnnotationSelector)
    #[pyo3(signature = (*args, **kwargs))]
    fn annotations(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
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
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| {
                    PyAnnotations::from_query(query, annotation.store(), &self.store, limit)
                },
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.annotations().test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query)?.test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations_in_targets(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<bool> {
        let recursive = get_recursive(kwargs, AnnotationDepth::One);
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.annotations_in_targets(recursive).test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Metadata,
                    recursive,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query)?.test()),
            )
        }
    }

    /// Returns a list of resources this annotation refers to
    #[pyo3(signature = (limit=None))]
    fn resources<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, resource) in annotation.resources().enumerate() {
                list.append(PyTextResource::new_py(
                    resource.handle(),
                    self.store.clone(),
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

    /// Returns the resources this annotation refers to (as metadata) in a list
    #[pyo3(signature = (limit=None))]
    fn datasets<'py>(&self, limit: Option<usize>, py: Python<'py>) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|annotation| {
            for (i, dataset) in annotation.datasets().enumerate() {
                list.append(PyAnnotationDataSet::new_py(
                    dataset.handle(),
                    self.store.clone(),
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
    fn data(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyData> {
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
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| {
                    PyData::from_query(query, annotation.store(), &self.store, limit)
                },
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_data(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotation| Ok(annotation.data().test()))
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |annotation, query| Ok(annotation.store().query(query)?.test()),
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
        args: &PyTuple,
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
                    PyTextSelections::from_query(query, annotation.store(), &self.store, limit)
                },
            )
        }
    }

    fn json(&self) -> PyResult<String> {
        self.map(|annotation| annotation.as_ref().to_json_string(annotation.store()))
    }

    /// Returns the annotation as a W3C Web Annotation in JSON-LD, as a string
    fn webannotation(&self, kwargs: Option<&PyDict>) -> PyResult<String> {
        let mut config = WebAnnoConfig::default();
        if let Some(kwargs) = kwargs {
            if let Ok(Some(v)) = kwargs.get_item("default_annotation_iri") {
                config.default_annotation_iri = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("default_resource_iri") {
                config.default_resource_iri = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("default_set_iri") {
                config.default_set_iri = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("auto_generated") {
                config.auto_generated = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("auto_generator") {
                config.auto_generator = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("extra_context") {
                config.extra_context = v.extract()?;
            }
            if let Ok(Some(v)) = kwargs.get_item("context_namespaces") {
                config.context_namespaces = {
                    let mut namespaces = Vec::new();
                    for assignment in v.extract::<Vec<String>>()? {
                        let result: Vec<_> = assignment.splitn(2, ":").collect();
                        if result.len() != 2 {
                            return Err(PyValueError::new_err(format!(
                                "Syntax for --ns should be `ns: uri_prefix`"
                            )));
                        }
                        namespaces
                            .push((result[1].trim().to_string(), result[0].trim().to_string()));
                    }
                    namespaces
                }
            }
        }
        self.map(|annotation| Ok(annotation.to_webannotation(&config)))
    }

    fn test_textselection(
        &self,
        operator: PyTextSelectionOperator,
        other: &PyTextSelection,
    ) -> PyResult<bool> {
        self.map(|annotation| {
            let store = annotation.store();
            let textselection = ResultTextSelection::Unbound(
                store,
                store.get(other.resource_handle)?,
                other.textselection.clone(),
            );
            Ok(annotation.test_textselection(&operator.operator, &textselection))
        })
    }

    fn test(&self, operator: PyTextSelectionOperator, other: &PyAnnotation) -> PyResult<bool> {
        self.map(|annotation| {
            let store = annotation.store();
            let other: &Annotation = store.get(other.handle)?;
            Ok(annotation.test(&operator.operator, &other.as_resultitem(store, store)))
        })
    }

    #[pyo3(signature = (via, **kwargs))]
    fn transpose(
        &mut self,
        via: &PyAnnotation,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyAnnotations> {
        let transpose_config = if let Some(kwargs) = kwargs {
            get_transpose_config(kwargs)
        } else {
            TransposeConfig::default()
        };
        let builders: Vec<AnnotationBuilder> = self.map(|annotation| {
            let store = annotation.store();
            let via = store.annotation(via.handle).or_fail()?;
            annotation.transpose(&via, transpose_config)
        })?;
        let annotations =
            self.map_store_mut(|store| store.annotate_from_iter(builders.into_iter()))?;
        Ok(PyAnnotations {
            annotations,
            store: self.store.clone(),
            cursor: 0,
        })
    }

    fn substore(&self) -> PyResult<Option<PyAnnotationSubStore>> {
        self.map(|annotation| {
            let substore = annotation.substore();
            Ok(substore.map(|s| PyAnnotationSubStore {
                handle: s.handle(),
                store: self.store.clone(),
            }))
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn alignments<'py>(
        &self,
        py: Python<'py>,
        kwargs: Option<&'py PyDict>,
    ) -> PyResult<&'py PyList> {
        let alignments = PyList::empty(py);
        let storeclone = self.store.clone();
        let mut annotations = false;
        if let Some(kwargs) = kwargs {
            if let Ok(Some(value)) = kwargs.get_item("annotations") {
                if let Ok(value) = value.extract::<bool>() {
                    annotations = value;
                }
            }
        }
        self.map(|annotation| {
            let mut annoiter = annotation.annotations_in_targets(AnnotationDepth::One);
            if let (Some(left), Some(right)) = (annoiter.next(), annoiter.next()) {
                //complex transposition
                for (text1, text2) in left.textselections().zip(right.textselections()) {
                    let alignment = PyList::empty(py);
                    if annotations {
                        alignment
                            .append(PyAnnotation::new_py(left.handle(), storeclone.clone(), py))
                            .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                        alignment
                            .append(PyAnnotation::new_py(right.handle(), storeclone.clone(), py))
                            .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                    } else {
                        alignment
                            .append(PyTextSelection::from_result_to_py(text1, &storeclone, py))
                            .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                        alignment
                            .append(PyTextSelection::from_result_to_py(text2, &storeclone, py))
                            .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                    }
                    alignments
                        .append(alignment)
                        .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                }
            } else {
                //simple transposition
                let mut textiter = annotation.textselections();
                if let (Some(text1), Some(text2)) = (textiter.next(), textiter.next()) {
                    let alignment = PyList::empty(py);
                    alignment
                        .append(PyTextSelection::from_result_to_py(text1, &storeclone, py))
                        .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                    alignment
                        .append(PyTextSelection::from_result_to_py(text2, &storeclone, py))
                        .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                    alignments
                        .append(alignment)
                        .map_err(|_| StamError::OtherError("failed to extract alignment"))?;
                }
            }
            Ok(alignments)
        })
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
            Some(PyAnnotation::new(*handle, pyself.store.clone()))
        } else {
            None
        }
    }

    fn __getitem__(pyself: PyRef<'_, Self>, mut index: isize) -> PyResult<PyAnnotation> {
        if index < 0 {
            index = pyself.annotations.len() as isize + index;
        }
        if let Some(handle) = pyself.annotations.get(index as usize) {
            Ok(PyAnnotation::new(*handle, pyself.store.clone()))
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
    fn data(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _store| {
                Ok(PyData::from_iter(
                    annotations.items().data().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |query, store| PyData::from_query(query, store, &self.store, limit),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_data(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| Ok(annotations.items().data().test()))
        } else {
            self.map_with_query(
                Type::AnnotationData,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |query, store| Ok(store.query(query)?.test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn annotations(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _store| {
                Ok(PyAnnotations::from_iter(
                    annotations.items().annotations().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |query, store| PyAnnotations::from_query(query, store, &self.store, limit),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<bool> {
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| Ok(annotations.items().annotations().test()))
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |query, store| Ok(store.query(query)?.test()),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn annotations_in_targets(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        let recursive = get_recursive(kwargs, AnnotationDepth::One);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _store| {
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
                Constraint::AnnotationVariable("main", SelectionQualifier::Normal, recursive, None),
                args,
                kwargs,
                |query, store| PyAnnotations::from_query(query, store, &self.store, limit),
            )
        }
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn test_annotations_in_targets(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<bool> {
        let recursive = get_recursive(kwargs, AnnotationDepth::One);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _| {
                Ok(annotations.items().annotations_in_targets(recursive).test())
            })
        } else {
            self.map_with_query(
                Type::Annotation,
                Constraint::AnnotationVariable("main", SelectionQualifier::Normal, recursive, None),
                args,
                kwargs,
                |query, store| Ok(store.query(query)?.test()),
            )
        }
    }

    #[pyo3(signature = (*args,**kwargs))]
    fn textselections(
        &self,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _store| {
                Ok(PyTextSelections::from_iter(
                    annotations.items().textselections().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(
                Type::TextSelection,
                Constraint::AnnotationVariable(
                    "main",
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                    None,
                ),
                args,
                kwargs,
                |query, store| PyTextSelections::from_query(query, store, &self.store, limit),
            )
        }
    }

    #[pyo3(signature = (operator, *args, **kwargs))]
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|annotations, _store| {
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
                |query, store| PyTextSelections::from_query(query, store, &self.store, limit),
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
    ) -> Result<Self, StamError> {
        Ok(Self {
            annotations: store
                .query(query)?
                .limit(limit)
                .map(|mut resultitems| {
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
        })
    }

    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(Handles<Annotation>, &AnnotationStore) -> Result<T, StamError>,
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

    fn map_with_query<T, F>(
        &self,
        resulttype: Type,
        constraint: Constraint,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
        f: F,
    ) -> Result<T, PyErr>
    where
        F: FnOnce(Query, &AnnotationStore) -> Result<T, StamError>,
    {
        self.map(|annotations, store| {
            let query = Query::new(QueryType::Select, Some(Type::Annotation), Some("main"))
                .with_constraint(Constraint::Annotations(
                    annotations,
                    SelectionQualifier::Normal,
                    AnnotationDepth::One,
                ))
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
    pub(crate) fn map<T, F>(&self, f: F) -> Result<T, PyErr>
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

    fn map_with_query<T, F>(
        &self,
        resulttype: Type,
        constraint: Constraint,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
        f: F,
    ) -> Result<T, PyErr>
    where
        F: FnOnce(ResultItem<Annotation>, Query) -> Result<T, StamError>,
    {
        self.map(|annotation| {
            let query = build_query(
                Query::new(QueryType::Select, Some(resulttype), Some("result"))
                    .with_constraint(constraint),
                args,
                kwargs,
                annotation.store(),
            )
            .map_err(|e| StamError::QuerySyntaxError(format!("{}", e), "(python to query)"))?
            .with_annotationvar("main", &annotation);
            f(annotation, query)
        })
    }
}

pub fn get_transpose_config(kwargs: &PyDict) -> TransposeConfig {
    let mut config = TransposeConfig::default();
    for (key, value) in kwargs {
        if let Some(key) = key.extract().unwrap() {
            match key {
                "debug" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.debug = value;
                    }
                }
                "allow_simple" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.allow_simple = value;
                    }
                }
                "no_transposition" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.no_transposition = value;
                    }
                }
                "no_resegmentation" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.no_resegmentation = value;
                    }
                }
                "transposition_id" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.transposition_id = value;
                    }
                }
                "resegmentation_id" => {
                    if let Ok(Some(value)) = value.extract() {
                        config.resegmentation_id = value;
                    }
                }
                _ => eprintln!("Ignored unknown kwargs option for transpose(): {}", key),
            }
        }
    }
    config
}
