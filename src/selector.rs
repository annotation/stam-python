use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;

use crate::annotation::PyAnnotation;
use crate::annotationdata::{PyAnnotationData, PyDataKey};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::annotationstore::{MapStore, PyAnnotationStore};
use crate::resources::{PyOffset, PyTextResource};
use stam::*;

#[pyclass(dict, module = "stam", name = "SelectorKind")]
#[derive(Clone, PartialEq)]
pub struct PySelectorKind {
    pub(crate) kind: SelectorKind,
}

#[pymethods]
impl PySelectorKind {
    #[classattr]
    const RESOURCESELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::ResourceSelector,
    };
    #[classattr]
    const ANNOTATIONSELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::AnnotationSelector,
    };
    #[classattr]
    const TEXTSELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::TextSelector,
    };
    #[classattr]
    const DATASETSELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::DataSetSelector,
    };
    #[classattr]
    const MULTISELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::MultiSelector,
    };
    #[classattr]
    const COMPOSITESELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::CompositeSelector,
    };
    #[classattr]
    const DIRECTIONALSELECTOR: PySelectorKind = PySelectorKind {
        kind: SelectorKind::DirectionalSelector,
    };

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (*self == *other).into_py(py),
            CompareOp::Ne => (*self != *other).into_py(py),
            _ => py.NotImplemented(),
        }
    }
}

#[pyclass(dict, module = "stam", name = "Selector")]
#[derive(Clone, PartialEq)]
/// This is the Python variant of SelectorBuilder, we can't just wrap SelectorBuiler itself because it has a lifetime
pub(crate) struct PySelector {
    pub(crate) kind: PySelectorKind,
    pub(crate) resource: Option<TextResourceHandle>,
    pub(crate) annotation: Option<AnnotationHandle>,
    pub(crate) dataset: Option<AnnotationDataSetHandle>,
    pub(crate) key: Option<(AnnotationDataSetHandle, DataKeyHandle)>,
    pub(crate) data: Option<(AnnotationDataSetHandle, AnnotationDataHandle)>,
    pub(crate) offset: Option<PyOffset>,
    pub(crate) subselectors: Vec<PySelector>,
}

impl PySelector {
    pub(crate) fn build(&self) -> SelectorBuilder<'_> {
        match self.kind.kind {
            SelectorKind::ResourceSelector => SelectorBuilder::ResourceSelector(
                self.resource
                    .expect("pyselector of type resourceselector must have resource, was checked on instantiation")
                    .into(),
            ),
            SelectorKind::TextSelector => SelectorBuilder::TextSelector(
                self.resource
                    .expect("pyselector of type textselector must have resource, was checked on instantiation")
                    .into(),
                self.offset.clone()
                    .expect("pyselector of type textselector must have offset, was checked on instantiation").offset
            ),
            SelectorKind::AnnotationSelector => SelectorBuilder::AnnotationSelector(
                self.annotation
                    .expect("pyselector of type annotationselector must have annotation, was checked on instantiation")
                    .into(),
                self.offset.clone().map(|offset| offset.offset)
            ),
            SelectorKind::DataSetSelector => SelectorBuilder::DataSetSelector(
                self.dataset
                    .expect("pyselector of type datasetselector must have dataset, was checked on instantiation")
                    .into(),
            ),
            SelectorKind::DataKeySelector => SelectorBuilder::DataKeySelector(
                self.key
                    .expect("pyselector of type datakeyselector must have key, was checked on instantiation")
                    .0
                    .into(),
                self.key
                    .expect("pyselector of type datakeyselector must have key, was checked on instantiation")
                    .1
                    .into(),
            ),
            SelectorKind::AnnotationDataSelector => SelectorBuilder::AnnotationDataSelector(
                self.data
                    .expect("pyselector of type annotationdataselector must have key, was checked on instantiation")
                    .0
                    .into(),
                self.data
                    .expect("pyselector of type annotationdataselector must have key, was checked on instantiation")
                    .1
                    .into(),
            ),
            SelectorKind::MultiSelector => {
                SelectorBuilder::MultiSelector(self.subselectors.iter().map(|subselector| subselector.build()).collect())
            }
            SelectorKind::CompositeSelector => {
                SelectorBuilder::CompositeSelector(self.subselectors.iter().map(|subselector| subselector.build()).collect())
            }
            SelectorKind::DirectionalSelector => {
                SelectorBuilder::DirectionalSelector(self.subselectors.iter().map(|subselector| subselector.build()).collect())
            }
            SelectorKind::InternalRangedSelector => unreachable!("internalrangedselector should never be passable from python")
        }
    }
}

#[pymethods]
impl PySelector {
    #[new]
    #[pyo3(signature = (kind, resource=None, annotation=None, dataset=None, key=None, data=None, offset=None, subselectors=Vec::new()))]
    fn new(
        kind: &PySelectorKind,
        resource: Option<PyRef<PyTextResource>>,
        annotation: Option<PyRef<PyAnnotation>>,
        dataset: Option<PyRef<PyAnnotationDataSet>>,
        key: Option<PyRef<PyDataKey>>,
        data: Option<PyRef<PyAnnotationData>>,
        offset: Option<PyRef<PyOffset>>,
        subselectors: Vec<PyRef<PySelector>>,
    ) -> PyResult<Self> {
        match kind.kind {
            SelectorKind::ResourceSelector => {
                if let Some(resource) = resource {
                    Ok(PySelector {
                        kind: kind.clone(),
                        resource: Some(resource.handle),
                        annotation: None,
                        dataset: None,
                        key: None,
                        data: None,
                        offset: None,
                        subselectors: Vec::new(),
                    })
                } else {
                    Err(PyValueError::new_err("'resource' keyword argument must be specified for ResourceSelector and point to a TextResource instance"))
                }
            }
            SelectorKind::AnnotationSelector => {
                if let Some(annotation) = annotation {
                    if let Some(offset) = offset {
                        Ok(PySelector {
                            kind: kind.clone(),
                            annotation: Some(annotation.handle),
                            resource: None,
                            dataset: None,
                            key: None,
                            data: None,
                            offset: Some(offset.clone()),
                            subselectors: Vec::new(),
                        })
                    } else {
                        Ok(PySelector {
                            kind: kind.clone(),
                            annotation: Some(annotation.handle),
                            resource: None,
                            dataset: None,
                            key: None,
                            data: None,
                            offset: None,
                            subselectors: Vec::new(),
                        })
                    }
                } else {
                    Err(PyValueError::new_err("'annotation' keyword argument must be specified for AnnotationSelector and point to a annotation instance"))
                }
            }
            SelectorKind::TextSelector => {
                if let Some(resource) = resource {
                    if let Some(offset) = offset {
                        Ok(PySelector {
                            kind: kind.clone(),
                            resource: Some(resource.handle),
                            annotation: None,
                            dataset: None,
                            key: None,
                            data: None,
                            offset: Some(offset.clone()),
                            subselectors: Vec::new(),
                        })
                    } else {
                        Err(PyValueError::new_err("'offset' keyword argument must be specified for TextSelector and point to a Offset instance"))
                    }
                } else {
                    Err(PyValueError::new_err("'resource' keyword argument must be specified for TextSelector and point to a TextResource instance"))
                }
            }
            SelectorKind::DataSetSelector => {
                if let Some(dataset) = dataset {
                    Ok(PySelector {
                        kind: kind.clone(),
                        resource: None,
                        annotation: None,
                        dataset: Some(dataset.handle),
                        key: None,
                        data: None,
                        offset: None,
                        subselectors: Vec::new(),
                    })
                } else {
                    Err(PyValueError::new_err("'dataset' keyword argument must be specified for DataSetSelector and point to an AnnotationDataSet instance"))
                }
            }
            SelectorKind::DataKeySelector => {
                if let Some(key) = key {
                    Ok(PySelector {
                        kind: kind.clone(),
                        resource: None,
                        annotation: None,
                        dataset: None,
                        key: Some((key.set, key.handle)),
                        data: None,
                        offset: None,
                        subselectors: Vec::new(),
                    })
                } else {
                    Err(PyValueError::new_err("'key' keyword argument must be specified for DataKeySelector and point to a DataKey instance"))
                }
            }
            SelectorKind::AnnotationDataSelector => {
                if let Some(data) = data {
                    Ok(PySelector {
                        kind: kind.clone(),
                        resource: None,
                        annotation: None,
                        dataset: None,
                        data: Some((data.set, data.handle)),
                        key: None,
                        offset: None,
                        subselectors: Vec::new(),
                    })
                } else {
                    Err(PyValueError::new_err("'key' keyword argument must be specified for DataKeySelector and point to a DataKey instance"))
                }
            }
            SelectorKind::MultiSelector
            | SelectorKind::CompositeSelector
            | SelectorKind::DirectionalSelector => {
                if subselectors.is_empty() {
                    Err(PyValueError::new_err("'subselectors' keyword argument must be specified for MultiSelector/CompositeSelector/DirectionalSelector and point to a list of Selector instances"))
                } else {
                    Ok(PySelector {
                        kind: kind.clone(),
                        resource: None,
                        annotation: None,
                        dataset: None,
                        key: None,
                        data: None,
                        offset: None,
                        subselectors: subselectors.into_iter().map(|sel| sel.clone()).collect(),
                    })
                }
            }
            SelectorKind::InternalRangedSelector => Err(PyValueError::new_err(
                "Construction of InternalRangedSelector not allowed",
            )),
        }
    }

    #[staticmethod]
    /// Shortcut static method to construct a TextSelector
    fn textselector(resource: PyRef<PyTextResource>, offset: PyRef<PyOffset>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::TEXTSELECTOR,
            Some(resource),
            None,
            None,
            None,
            None,
            Some(offset),
            Vec::new(),
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a AnnotationSelector
    fn annotationselector(
        annotation: PyRef<PyAnnotation>,
        offset: Option<PyRef<PyOffset>>,
    ) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::ANNOTATIONSELECTOR,
            None,
            Some(annotation),
            None,
            None,
            None,
            offset,
            Vec::new(),
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a ResourceSelector
    fn resourceselector(resource: PyRef<PyTextResource>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::RESOURCESELECTOR,
            Some(resource),
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a DataSetSelector
    fn datasetselector(annotationset: PyRef<PyAnnotationDataSet>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::DATASETSELECTOR,
            None,
            None,
            Some(annotationset),
            None,
            None,
            None,
            Vec::new(),
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a MultiSelector
    #[pyo3(signature = (*subselectors))]
    fn multiselector(subselectors: Vec<PyRef<PySelector>>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::MULTISELECTOR,
            None,
            None,
            None,
            None,
            None,
            None,
            subselectors,
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a CompositeSelector
    #[pyo3(signature = (*subselectors))]
    fn compositeselector(subselectors: Vec<PyRef<PySelector>>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::COMPOSITESELECTOR,
            None,
            None,
            None,
            None,
            None,
            None,
            subselectors,
        )
    }

    #[staticmethod]
    /// Shortcut static method to construct a DirectionalSelector
    #[pyo3(signature = (*subselectors))]
    fn directionalselector(subselectors: Vec<PyRef<PySelector>>) -> PyResult<Self> {
        PySelector::new(
            &PySelectorKind::DIRECTIONALSELECTOR,
            None,
            None,
            None,
            None,
            None,
            None,
            subselectors,
        )
    }

    /// Returns the selector kind, use is_kind() instead if you want to test
    fn kind(&self) -> PySelectorKind {
        self.kind.clone()
    }

    fn is_kind(&self, kind: &PySelectorKind) -> bool {
        self.kind.kind == kind.kind
    }

    /// Quicker test for specified selector kind
    fn is_resourceselector(&self) -> bool {
        self.kind.kind == SelectorKind::ResourceSelector
    }

    /// Quicker test for specified selector kind
    fn is_textselector(&self) -> bool {
        self.kind.kind == SelectorKind::TextSelector
    }

    /// Quicker test for specified selector kind
    fn is_annotationselector(&self) -> bool {
        self.kind.kind == SelectorKind::AnnotationSelector
    }

    /// Quicker test for specified selector kind
    fn is_datasetselector(&self) -> bool {
        self.kind.kind == SelectorKind::DataSetSelector
    }

    /// Quicker test for specified selector kind
    fn is_multiselector(&self) -> bool {
        self.kind.kind == SelectorKind::MultiSelector
    }

    /// Quicker test for specified selector kind
    fn is_directionalselector(&self) -> bool {
        self.kind.kind == SelectorKind::DirectionalSelector
    }

    /// Quicker test for specified selector kind
    fn is_compositeselector(&self) -> bool {
        self.kind.kind == SelectorKind::CompositeSelector
    }

    /// Return offset information in the selector.
    /// Works for TextSelector and AnnotationSelector, returns None for others.
    fn offset(&self) -> PyResult<Option<PyOffset>> {
        Ok(self.offset.clone())
    }

    /// Returns the resource this selector points at, if any.
    /// Works only for TextSelector and ResourceSelector, returns None otherwise.
    /// Requires to explicitly pass the store so the resource can be found.
    fn resource(&self, store: PyRef<PyAnnotationStore>) -> Option<PyTextResource> {
        self.resource.map(|resource_handle| PyTextResource {
            handle: resource_handle,
            store: store.get_store().clone(),
        })
    }

    /// Returns the key this selector points at, if any.
    /// Works only for DataKeySelector, returns None otherwise.
    /// Requires to explicitly pass the store so the resource can be found.
    fn key(&self, store: PyRef<PyAnnotationStore>) -> Option<PyDataKey> {
        self.key.map(|(set_handle, key_handle)| PyDataKey {
            set: set_handle,
            handle: key_handle,
            store: store.get_store().clone(),
        })
    }

    /// Returns the annotationdata this selector points at, if any.
    /// Works only for AnnotationDataSelector, returns None otherwise.
    /// Requires to explicitly pass the store so the resource can be found.
    fn annotationdata(&self, store: PyRef<PyAnnotationStore>) -> Option<PyAnnotationData> {
        self.data.map(|(set_handle, data_handle)| PyAnnotationData {
            set: set_handle,
            handle: data_handle,
            store: store.get_store().clone(),
        })
    }

    /// Returns the annotation this selector points at, if any.
    /// Works only for AnnotationSelector, returns None otherwise.
    /// Requires to explicitly pass the store so the resource can be found.
    fn annotation(&self, store: PyRef<PyAnnotationStore>) -> Option<PyAnnotation> {
        self.annotation.map(|annotation_handle| PyAnnotation {
            handle: annotation_handle,
            store: store.get_store().clone(),
        })
    }

    /// Returns the annotation dataset this selector points at, ff any.
    /// Works only for DataSetSelector, returns None otherwise.
    /// Requires to explicitly pass the store so the dataset can be found.
    fn dataset(&self, store: PyRef<PyAnnotationStore>) -> Option<PyAnnotationDataSet> {
        self.dataset.map(|dataset_handle| PyAnnotationDataSet {
            handle: dataset_handle,
            store: store.get_store().clone(),
        })
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (*self == *other).into_py(py),
            CompareOp::Ne => (*self != *other).into_py(py),
            _ => py.NotImplemented(),
        }
    }
}

impl PySelector {
    pub(crate) fn from_selector(selector: &Selector, store: &AnnotationStore) -> Self {
        Self {
            kind: PySelectorKind {
                kind: selector.kind(),
            },
            resource: match selector {
                Selector::TextSelector(res_id, ..)
                | Selector::ResourceSelector(res_id)
                | Selector::AnnotationSelector(_, Some((res_id, _, _))) => Some(*res_id),
                _ => None,
            },
            dataset: match selector {
                Selector::DataSetSelector(set_id) => Some(*set_id),
                _ => None,
            },
            annotation: match selector {
                Selector::AnnotationSelector(a_id, ..) => Some(*a_id),
                _ => None,
            },
            key: match selector {
                Selector::DataKeySelector(set_id, key_id) => Some((*set_id, *key_id)),
                _ => None,
            },
            data: match selector {
                Selector::AnnotationDataSelector(set_id, data_id) => Some((*set_id, *data_id)),
                _ => None,
            },
            offset: selector.offset(store).map(|offset| PyOffset { offset }),
            subselectors: if selector.is_complex() {
                if let Some(subselectors) = selector.subselectors() {
                    subselectors
                        .into_iter()
                        .map(|x| Self::from_selector(x, store))
                        .collect()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            },
        }
    }
}
