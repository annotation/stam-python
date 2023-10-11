use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::*;
use std::borrow::Cow;

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{dataoperator_from_kwargs, PyAnnotationData, PyData, PyDataKey};
use crate::error::PyStamError;
use crate::textselection::PyTextSelectionOperator;
use stam::*;

pub enum Filter<'a> {
    Annotation(AnnotationHandle),
    Annotations((Vec<AnnotationHandle>, bool)), //the boolean expresses whether the data is sorted or not
    AnnotationData(AnnotationDataSetHandle, AnnotationDataHandle),
    Data((Vec<(AnnotationDataSetHandle, AnnotationDataHandle)>, bool)), //the boolean expresses whether the data is sorted or not
    DataKey(AnnotationDataSetHandle, DataKeyHandle),
    Value(DataOperator<'a>),
    TextRelation(TextSelectionOperator),
    //FindData(AnnotationDataSetHandle, DataKeyHandle, DataOperator<'a>),
}

pub struct IterParams<'a> {
    filters: Vec<Filter<'a>>,
    limit: Option<usize>,
}

fn add_filter<'py>(filters: &mut Vec<Filter<'py>>, filter: &'py PyAny) -> PyResult<()> {
    if filter.is_instance_of::<PyList>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(filters, vec)?;
    } else if filter.is_instance_of::<PyTuple>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(filters, vec)?;
    } else if filter.is_instance_of::<PyAnnotationData>() {
        let adata: PyRef<'_, PyAnnotationData> = filter.extract()?;
        filters.push(Filter::AnnotationData(adata.set, adata.handle));
    } else if filter.is_instance_of::<PyDataKey>() {
        let key: PyRef<'_, PyDataKey> = filter.extract()?;
        filters.push(Filter::DataKey(key.set, key.handle));
    } else if filter.is_instance_of::<PyAnnotation>() {
        let annotation: PyRef<'_, PyAnnotation> = filter.extract()?;
        filters.push(Filter::Annotation(annotation.handle));
    } else if filter.is_instance_of::<PyTextSelectionOperator>() {
        let operator: PyRef<'_, PyTextSelectionOperator> = filter.extract()?;
        filters.push(Filter::TextRelation(operator.operator));
    } else if filter.is_instance_of::<PyAnnotations>() {
        let annotations: PyRef<'py, PyAnnotations> = filter.extract()?;
        filters.push(Filter::Annotations((
            annotations.annotations.iter().copied().collect(),
            annotations.sorted,
        )));
    } else if filter.is_instance_of::<PyData>() {
        let data: PyRef<'_, PyData> = filter.extract()?;
        filters.push(Filter::Data((
            data.data.iter().copied().collect(),
            data.sorted,
        )));
    } else {
        return Err(PyValueError::new_err(
            "Got argument of unexpected type for filter=/filters=",
        ));
    }
    Ok(())
}

fn add_multi_filter<'a>(filters: &mut Vec<Filter<'a>>, filter: Vec<&PyAny>) -> PyResult<()> {
    if filter.iter().all(|x| x.is_instance_of::<PyAnnotation>()) {
        filters.push(Filter::Annotations((
            filter
                .iter()
                .map(|x| {
                    let annotation: PyRef<'_, PyAnnotation> = x.extract().unwrap();
                    annotation.handle
                })
                .collect(),
            false, //we don't know if the data is sorted
        )));
    } else if filter
        .iter()
        .all(|x| x.is_instance_of::<PyAnnotationData>())
    {
        filters.push(Filter::Data((
            filter
                .iter()
                .map(|x| {
                    let adata: PyRef<'_, PyAnnotationData> = x.extract().unwrap();
                    (adata.set, adata.handle)
                })
                .collect(),
            false, //we don't know if the data is sorted
        )));
    }
    Ok(())
}

impl<'py> IterParams<'py> {
    pub fn new(kwargs: Option<&'py PyDict>) -> PyResult<Self> {
        let mut filters = Vec::new();
        let mut limit: Option<usize> = None;
        if let Some(kwargs) = kwargs {
            if let Some(v) = kwargs.get_item("limit") {
                match v.extract() {
                    Ok(v) => limit = v,
                    Err(e) => {
                        return Err(PyValueError::new_err(format!(
                            "Limit should be an integer or None: {}",
                            e
                        )))
                    }
                }
            }
            if let Some(filter) = kwargs.get_item("filter") {
                add_filter(&mut filters, filter)?;
            } else if let Some(filter) = kwargs.get_item("filters") {
                if filter.is_instance_of::<PyList>() {
                    let vec = filter.downcast::<PyList>()?;
                    for filter in vec {
                        add_filter(&mut filters, filter)?;
                    }
                } else if filter.is_instance_of::<PyTuple>() {
                    let vec = filter.downcast::<PyTuple>()?;
                    for filter in vec {
                        add_filter(&mut filters, filter)?;
                    }
                }
            }
            if let Some(operator) = dataoperator_from_kwargs(kwargs)
                .map_err(|e| PyStamError::new_err(format!("{}", e)))?
            {
                filters.push(Filter::Value(operator));
            }
        }
        Ok(Self { filters, limit })
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn evaluate_annotations<'store>(
        self,
        mut iter: stam::AnnotationsIter<'store>,
        store: &'store AnnotationStore,
    ) -> Result<stam::AnnotationsIter<'store>, StamError>
    where
        'py: 'store,
    {
        let mut has_value_filter = false;
        let mut datakey_filter: Option<ResultItem<DataKey>> = None;
        for filter in self.filters.iter() {
            if let Filter::Value(op) = filter {
                if let DataOperator::Any = op {
                    //ignore
                } else {
                    has_value_filter = true;
                }
            }
        }
        for filter in self.filters.into_iter() {
            match filter {
                Filter::Annotation(handle) => {
                    iter = iter.filter_handle(handle);
                }
                Filter::AnnotationData(set_handle, data_handle) => {
                    iter = iter.filter_annotationdata_handle(set_handle, data_handle);
                }
                Filter::DataKey(set_handle, key_handle) => {
                    let dataset = store
                        .dataset(set_handle)
                        .ok_or_else(|| StamError::HandleError("Unable to find dataset"))?;
                    let key = dataset
                        .key(key_handle)
                        .ok_or_else(|| StamError::HandleError("Unable to find key"))?;
                    //check if we also have a value filter
                    if !has_value_filter {
                        iter = iter.filter_data(key.data().to_cache());
                    } else {
                        datakey_filter = Some(key); //will be handled further in Filter::Value arm
                    }
                }
                Filter::Annotations((annotations, sorted)) => {
                    let annotations =
                        Annotations::from_handles(Cow::Owned(annotations), sorted, store);
                    iter = iter.filter_annotations(annotations.iter());
                }
                Filter::Data((data, sorted)) => {
                    let data = Data::from_handles(Cow::Owned(data), sorted, store);
                    iter = iter.filter_data(data);
                }
                Filter::Value(op) => {
                    if let Some(key) = &datakey_filter {
                        iter = iter.filter_data(key.data().filter_value(op).to_cache());
                    } else {
                        return Err(StamError::OtherError(
                            "Python: You can specify a value filter only if you pass filter=DataKey (Annotations)",
                        ));
                    }
                }
                Filter::TextRelation(op) => {
                    iter = iter.filter_related_text(op);
                }
                _ => {
                    return Err(StamError::OtherError(
                        "Python: not a valid filter in this context (Annotations)",
                    ));
                }
            }
        }
        Ok(iter)
    }

    pub fn evaluate_data<'store>(
        self,
        mut iter: stam::DataIter<'store>,
        store: &'store AnnotationStore,
    ) -> Result<stam::DataIter<'store>, StamError>
    where
        'py: 'store,
    {
        for filter in self.filters.into_iter() {
            match filter {
                Filter::Annotation(handle) => {
                    if let Some(annotation) = store.annotation(handle) {
                        iter = iter.filter_annotation(&annotation);
                    }
                }
                Filter::Annotations((annotations, sorted)) => {
                    let annotations =
                        Annotations::from_handles(Cow::Owned(annotations), sorted, store);
                    iter = iter.filter_annotations(annotations.iter());
                }
                Filter::Data((data, sorted)) => {
                    let data = Data::from_handles(Cow::Owned(data), sorted, store);
                    iter = iter.filter_data(data.iter());
                }
                Filter::DataKey(set_handle, key_handle) => {
                    iter = iter.filter_key_handle(set_handle, key_handle);
                }
                Filter::Value(operator) => {
                    iter = iter.filter_value(operator.clone());
                }
                _ => {
                    return Err(StamError::OtherError(
                        "Python: not a valid filter in this context (Data)",
                    ));
                }
            }
        }
        Ok(iter)
    }

    pub fn evaluate_textselections<'store>(
        self,
        mut iter: stam::TextSelectionsIter<'store>,
        store: &'store AnnotationStore,
    ) -> Result<stam::TextSelectionsIter<'store>, StamError>
    where
        'py: 'store,
    {
        let mut has_value_filter = false;
        let mut datakey_filter: Option<ResultItem<DataKey>> = None;
        for filter in self.filters.iter() {
            if let Filter::Value(op) = filter {
                if let DataOperator::Any = op {
                    //ignore
                } else {
                    has_value_filter = true;
                }
            }
        }
        for filter in self.filters.into_iter() {
            match filter {
                Filter::TextRelation(op) => {
                    iter = iter.related_text(op);
                }
                Filter::Annotation(handle) => {
                    iter = iter.filter_annotation_handle(handle);
                }
                Filter::Annotations((annotations, sorted)) => {
                    let annotations =
                        Annotations::from_handles(Cow::Owned(annotations), sorted, store);
                    iter = iter.filter_annotations(annotations);
                }
                Filter::AnnotationData(set_handle, data_handle) => {
                    iter = iter.filter_annotationdata_handle(set_handle, data_handle);
                }
                Filter::Data((data, sorted)) => {
                    let data = Data::from_handles(Cow::Owned(data), sorted, store);
                    iter = iter.filter_data(data);
                }
                Filter::DataKey(set_handle, key_handle) => {
                    if let Some(dataset) = store.dataset(set_handle) {
                        if let Some(key) = dataset.key(key_handle) {
                            if !has_value_filter {
                                iter = iter.filter_data(key.data().to_cache());
                            } else {
                                datakey_filter = Some(key); //will be handled further in Filter::Value arm
                            }
                        }
                    }
                }
                Filter::Value(operator) => {
                    if let Some(key) = &datakey_filter {
                        iter = iter.filter_data(key.data().filter_value(operator).to_cache());
                    } else {
                        return Err(StamError::OtherError(
                            "Python: You can specify a value filter only if you pass filter=DataKey (TextSelections)",
                        ));
                    }
                }
                _ => {
                    return Err(StamError::OtherError(
                        "Python: not a valid filter in this context (TextSelections)",
                    ));
                }
            }
        }
        Ok(iter)
    }
}
