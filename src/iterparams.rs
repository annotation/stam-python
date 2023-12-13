use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::*;
use std::borrow::Cow;

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{dataoperator_from_kwargs, PyAnnotationData, PyData, PyDataKey};
use crate::error::PyStamError;
use crate::textselection::PyTextSelectionOperator;
use stam::*;

pub struct QueryBuilder<'a> {
    filters: Vec<Filter<'a>>,
    limit: Option<usize>,
}

fn add_filter<'a>(
    query: &mut Query<'a>,
    store: &'a AnnotationStore,
    filter: &'a PyAny,
    operator: &mut Option<DataOperator>,
) -> PyResult<()> {
    if filter.is_instance_of::<PyList>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(query, store, vec)?;
    } else if filter.is_instance_of::<PyTuple>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(query, store, vec)?;
    } else if filter.is_instance_of::<PyAnnotationData>() {
        let adata: PyRef<'_, PyAnnotationData> = filter.extract()?;
        query.constrain(Constraint::Filter(Filter::AnnotationData(
            adata.set,
            adata.handle,
        )));
    } else if filter.is_instance_of::<PyDataKey>() {
        let key: PyRef<'_, PyDataKey> = filter.extract()?;
        if operator.is_some() {
            query.constrain(Constraint::Filter(Filter::DataKeyAndOperator(
                key.set,
                key.handle,
                operator.take().unwrap(),
            )));
        } else {
            query.constrain(Constraint::Filter(Filter::DataKey(key.set, key.handle)));
        }
    } else if filter.is_instance_of::<PyAnnotation>() {
        let annotation: PyRef<'_, PyAnnotation> = filter.extract()?;
        query.constrain(Constraint::Filter(Filter::Annotation(annotation.handle)));
    } else if filter.is_instance_of::<PyTextSelectionOperator>() {
        let operator: PyRef<'_, PyTextSelectionOperator> = filter.extract()?;
        query.constrain(Constraint::Filter(Filter::TextSelectionOperator(
            operator.operator,
        )));
    } else if filter.is_instance_of::<PyAnnotations>() {
        let annotations: PyRef<'a, PyAnnotations> = filter.extract()?;
        query.constrain(Constraint::Filter(Filter::Annotations(Handles::from_iter(
            annotations.annotations.iter().copied(),
            store,
        ))));
    } else if filter.is_instance_of::<PyData>() {
        let data: PyRef<'_, PyData> = filter.extract()?;
        query.constrain(Constraint::Filter(Filter::Data(
            Handles::from_iter(data.data.iter().copied(), store),
            FilterMode::Any,
        )));
    } else {
        return Err(PyValueError::new_err(
            "Got argument of unexpected type for filter=/filters=",
        ));
    }
    Ok(())
}

fn add_multi_filter<'a>(
    query: &mut Query<'a>,
    store: &AnnotationStore,
    filter: Vec<&PyAny>,
) -> PyResult<()> {
    if filter.iter().all(|x| x.is_instance_of::<PyAnnotation>()) {
        query.constrain(Constraint::Filter(Filter::Annotations(Handles::from_iter(
            filter.iter().map(|x| {
                let annotation: PyRef<'_, PyAnnotation> = x.extract().unwrap();
                annotation.handle
            }),
            store,
        ))));
    } else if filter
        .iter()
        .all(|x| x.is_instance_of::<PyAnnotationData>())
    {
        query.constrain(Constraint::Filter(Filter::Data(
            Handles::from_iter(
                filter.iter().map(|x| {
                    let adata: PyRef<'_, PyAnnotationData> = x.extract().unwrap();
                    (adata.set, adata.handle)
                }),
                store,
            ),
            FilterMode::Any,
        )));
    }
    Ok(())
}

pub(crate) fn build_query<'py>(
    mut query: Query<'py>,
    args: &'py PyList, //TODO: implement!
    kwargs: Option<&'py PyDict>,
    store: &'py AnnotationStore,
) -> PyResult<Query<'py>> {
    if let Some(kwargs) = kwargs {
        let mut operator =
            dataoperator_from_kwargs(kwargs).map_err(|e| PyStamError::new_err(format!("{}", e)))?;
        if let Ok(Some(filter)) = kwargs.get_item("filter") {
            add_filter(&mut query, store, filter, &mut operator)?;
        } else if let Ok(Some(filter)) = kwargs.get_item("filters") {
            if filter.is_instance_of::<PyList>() {
                let vec = filter.downcast::<PyList>()?;
                for filter in vec {
                    add_filter(&mut query, store, filter, &mut operator)?;
                }
            } else if filter.is_instance_of::<PyTuple>() {
                let vec = filter.downcast::<PyTuple>()?;
                for filter in vec {
                    add_filter(&mut query, store, filter, &mut operator)?;
                }
            }
        }
        //if the operator has not been consumed yet in an earlier add_filter step, add a constraint for it now:
        if let Some(operator) = operator {
            query.constrain(Constraint::Filter(Filter::DataOperator(operator)));
        }
    }
    Ok(query)
}

impl<'py> QueryBuilder<'py> {
    pub fn new(
        resulttype: Type,
        store: &'py AnnotationStore,
        kwargs: Option<&'py PyDict>,
        name: Option<&'py str>,
    ) -> PyResult<Query<'py>> {
        let mut query = Query::new(QueryType::Select, Some(resulttype), name);

        if let Some(kwargs) = kwargs {
            let mut operator = dataoperator_from_kwargs(kwargs)
                .map_err(|e| PyStamError::new_err(format!("{}", e)))?;
            if let Ok(Some(filter)) = kwargs.get_item("filter") {
                add_filter(&mut query, store, filter, &mut operator)?;
            } else if let Ok(Some(filter)) = kwargs.get_item("filters") {
                if filter.is_instance_of::<PyList>() {
                    let vec = filter.downcast::<PyList>()?;
                    for filter in vec {
                        add_filter(&mut query, store, filter, &mut operator)?;
                    }
                } else if filter.is_instance_of::<PyTuple>() {
                    let vec = filter.downcast::<PyTuple>()?;
                    for filter in vec {
                        add_filter(&mut query, store, filter, &mut operator)?;
                    }
                }
            }
            //if the operator has not been consumed yet in an earlier add_filter step, add a constraint for it now:
            if let Some(operator) = operator {
                query.constrain(Constraint::Filter(Filter::DataOperator(operator)));
            }
        }
        Ok(query)
    }
}

pub(crate) fn has_filters(args: &PyList, kwargs: Option<&PyDict>) -> bool {
    if !args.is_empty() {
        return true;
    }
    if let Some(kwargs) = kwargs {
        for key in kwargs.keys() {
            if let Ok(Some("limit")) | Ok(Some("recursive")) = key.extract() {
                continue; //this doesn't count as a filter
            } else {
                return true;
            }
        }
    }
    false
}

pub(crate) fn get_recursive(kwargs: Option<&PyDict>, default: bool) -> bool {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(v)) = kwargs.get_item("recursive") {
            if let Ok(v) = v.extract::<bool>() {
                return v;
            }
        }
    }
    default
}

pub(crate) fn get_limit(kwargs: Option<&PyDict>) -> Option<usize> {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(limit)) = kwargs.get_item("limit") {
            if let Ok(limit) = limit.extract::<usize>() {
                return Some(limit);
            }
        }
    }
    None
}

pub(crate) struct LimitIter<I: Iterator> {
    inner: I,
    limit: Option<usize>,
}

impl<I> Iterator for LimitIter<I>
where
    I: Iterator,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(remainder) = self.limit.as_mut() {
            if *remainder > 0 {
                *remainder -= 1;
                self.inner.next()
            } else {
                None
            }
        } else {
            self.inner.next()
        }
    }
}

pub(crate) trait LimitIterator
where
    Self: Iterator,
    Self: Sized,
{
    fn limit(self, limit: Option<usize>) -> LimitIter<Self> {
        LimitIter { inner: self, limit }
    }
}

impl<I> LimitIterator for I where I: Iterator {}
