use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::*;
use std::collections::HashSet;

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{dataoperator_from_kwargs, PyAnnotationData, PyData, PyDataKey};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::error::PyStamError;
use crate::textselection::PyTextSelectionOperator;
use stam::*;

const CONTEXTVARNAMES: [&str; 25] = [
    "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v8", "v9", "v10", "v11", "v12", "v13", "v14", "v15",
    "v16", "v17", "v18", "v19", "v20", "v21", "v22", "v23", "v24", "v25",
];

fn new_contextvar(used_contextvarnames: &mut usize) -> &'static str {
    let varname = CONTEXTVARNAMES
        .get(*used_contextvarnames)
        .map(|x| *x)
        .expect("no free context variables present");
    *used_contextvarnames += 1;
    varname
}

fn add_filter<'store, 'py, 'context>(
    query: &mut Query<'store>,
    store: &'store AnnotationStore,
    filter: &'py PyAny,
    operator: Option<DataOperator<'store>>,
    mut used_contextvarnames: usize,
) -> PyResult<usize>
where
    'py: 'store,
    'context: 'store,
{
    if filter.is_instance_of::<PyDict>() {
        let filter: &PyDict = filter.extract()?;
        let operator = dataoperator_from_kwargs(filter)
            .map_err(|err| PyValueError::new_err(format!("{}", err)))?
            .or(operator);
        let key = filter.get_item("key")?.expect("already checked");
        if key.is_instance_of::<PyDataKey>() {
            let key: PyRef<'py, PyDataKey> = filter.extract()?;
            if let Some(key) = store.key(key.set, key.handle) {
                let varname = new_contextvar(&mut used_contextvarnames);
                query.bind_keyvar(varname, key);
                if let Some(operator) = operator {
                    query.constrain(Constraint::KeyValueVariable(
                        varname,
                        operator,
                        SelectionQualifier::Normal,
                    ));
                } else {
                    query.constrain(Constraint::KeyVariable(varname, SelectionQualifier::Normal));
                }
            } else {
                return Err(PyValueError::new_err(
                    "Passed DataKey instance is invalid (should never happen)",
                ));
            }
        } else if filter.contains("set")? {
            if key.is_instance_of::<PyString>() {
                let key = key.downcast::<PyString>()?;
                let set = filter.get_item("set")?.expect("already checked");
                let key = if set.is_instance_of::<PyAnnotationDataSet>() {
                    let set: PyRef<'py, PyAnnotationDataSet> = filter.extract()?;
                    if let Some(dataset) = store.dataset(set.handle) {
                        if let Some(key) = dataset.key(key.to_str()?) {
                            Some(key)
                        } else {
                            return Err(PyValueError::new_err("specified key not found in set"));
                        }
                    } else {
                        return Err(PyValueError::new_err(
                            "Passed AnnotationDataSet instance is invalid (should never happen)",
                        ));
                    }
                } else if set.is_instance_of::<PyString>() {
                    let set = set.downcast::<PyString>()?;
                    if let Some(dataset) = store.dataset(set.to_str()?) {
                        if let Some(key) = dataset.key(key.to_str()?) {
                            Some(key)
                        } else {
                            return Err(PyValueError::new_err("specified key not found in set"));
                        }
                    } else {
                        return Err(PyValueError::new_err("specified dataset not found"));
                    }
                } else {
                    None
                };
                if let Some(key) = key {
                    let varname = new_contextvar(&mut used_contextvarnames);
                    query.bind_keyvar(varname, key);
                    if let Some(operator) = operator {
                        query.constrain(Constraint::KeyValueVariable(
                            varname,
                            operator,
                            SelectionQualifier::Normal,
                        ));
                    } else {
                        query.constrain(Constraint::KeyVariable(
                            varname,
                            SelectionQualifier::Normal,
                        ));
                    }
                }
            } else {
                return Err(PyValueError::new_err(
                    "'key' parameter in filter dictionary should be of type `str`",
                ));
            }
        } else {
            return Err(PyValueError::new_err(
                    "'key' parameter in filter dictionary should be of type DataKey, it can also be `str` if you also provide `set` as well",
            ));
        }
    } else if filter.is_instance_of::<PyList>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(query, store, vec)?;
    } else if filter.is_instance_of::<PyTuple>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        add_multi_filter(query, store, vec)?;
    } else if filter.is_instance_of::<PyAnnotationData>() {
        let data: PyRef<'_, PyAnnotationData> = filter.extract()?;
        if operator.is_some() {
            return Err(PyValueError::new_err(
                    "'value' parameter can not be used in combination with an AnnotationData instance (it already restrains to a single value)",
            ));
        }
        if let Some(data) = store.annotationdata(data.set, data.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_datavar(varname, data);
            query.constrain(Constraint::DataVariable(
                varname,
                SelectionQualifier::Normal,
            ));
        }
    } else if filter.is_instance_of::<PyDataKey>() {
        let key: PyRef<'py, PyDataKey> = filter.extract()?;
        if let Some(key) = store.key(key.set, key.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_keyvar(varname, key);
            if let Some(operator) = operator {
                query.constrain(Constraint::KeyValueVariable(
                    varname,
                    operator,
                    SelectionQualifier::Normal,
                ));
            } else {
                query.constrain(Constraint::KeyVariable(varname, SelectionQualifier::Normal));
            }
        } else {
            return Err(PyValueError::new_err(
                "Passed DataKey instance is invalid (should never happen)",
            ));
        }
    } else if filter.is_instance_of::<PyAnnotation>() {
        let annotation: PyRef<'py, PyAnnotation> = filter.extract()?;
        if let Some(annotation) = store.annotation(annotation.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_annotationvar(varname, annotation);
            query.constrain(Constraint::AnnotationVariable(
                varname,
                SelectionQualifier::Normal,
                AnnotationDepth::One,
            ));
        } else {
            return Err(PyValueError::new_err(
                "Passed DataKey instance is invalid (should never happen)",
            ));
        }
    } else if filter.is_instance_of::<PyAnnotations>() {
        let annotations: PyRef<'py, PyAnnotations> = filter.extract()?;
        query.constrain(Constraint::Annotations(
            Handles::from_iter(annotations.annotations.iter().copied(), store),
            SelectionQualifier::Normal,
            AnnotationDepth::One,
        ));
    } else if filter.is_instance_of::<PyData>() {
        let data: PyRef<'py, PyData> = filter.extract()?;
        query.constrain(Constraint::Data(
            Handles::from_iter(data.data.iter().copied(), store),
            SelectionQualifier::Normal,
        ));
    } else {
        return Err(PyValueError::new_err(
            "Got filter argument of unexpected type",
        ));
    }
    Ok(used_contextvarnames)
}

fn add_multi_filter<'a>(
    query: &mut Query<'a>,
    store: &'a AnnotationStore,
    filter: Vec<&PyAny>,
) -> PyResult<()> {
    if filter.iter().all(|x| x.is_instance_of::<PyAnnotation>()) {
        query.constrain(Constraint::Annotations(
            Handles::from_iter(
                filter.iter().map(|x| {
                    let annotation: PyRef<'_, PyAnnotation> = x.extract().unwrap();
                    annotation.handle
                }),
                store,
            ),
            SelectionQualifier::Normal,
            AnnotationDepth::One,
        ));
    } else if filter
        .iter()
        .all(|x| x.is_instance_of::<PyAnnotationData>())
    {
        query.constrain(Constraint::Data(
            Handles::from_iter(
                filter.iter().map(|x| {
                    let adata: PyRef<'_, PyAnnotationData> = x.extract().unwrap();
                    (adata.set, adata.handle)
                }),
                store,
            ),
            SelectionQualifier::Normal,
        ));
    }
    Ok(())
}

pub(crate) fn build_query<'store, 'py>(
    mut query: Query<'store>,
    args: &'py PyList,
    kwargs: Option<&'py PyDict>,
    store: &'store AnnotationStore,
) -> PyResult<Query<'store>>
where
    'py: 'store,
{
    let mut used_contextvarnames: usize = 0;
    let operator = if let Some(kwargs) = kwargs {
        dataoperator_from_kwargs(kwargs).map_err(|e| PyStamError::new_err(format!("{}", e)))?
    } else {
        None
    };
    for filter in args {
        used_contextvarnames = add_filter(
            &mut query,
            store,
            filter,
            operator.clone(),
            used_contextvarnames,
        )?;
    }
    if let Some(kwargs) = kwargs {
        if let Ok(Some(filter)) = kwargs.get_item("filter") {
            add_filter(&mut query, store, filter, operator, used_contextvarnames)?;
        } else if let Ok(Some(filter)) = kwargs.get_item("filters") {
            if filter.is_instance_of::<PyList>() {
                let vec = filter.downcast::<PyList>()?;
                for filter in vec {
                    used_contextvarnames = add_filter(
                        &mut query,
                        store,
                        filter,
                        operator.clone(),
                        used_contextvarnames,
                    )?;
                }
            } else if filter.is_instance_of::<PyTuple>() {
                let vec = filter.downcast::<PyTuple>()?;
                for filter in vec {
                    used_contextvarnames = add_filter(
                        &mut query,
                        store,
                        filter,
                        operator.clone(),
                        used_contextvarnames,
                    )?;
                }
            }
        } else {
            add_filter(
                &mut query,
                store,
                kwargs.as_ref(),
                operator,
                used_contextvarnames,
            )?;
        }
    }
    Ok(query)
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

pub(crate) fn get_recursive(kwargs: Option<&PyDict>, default: AnnotationDepth) -> AnnotationDepth {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(v)) = kwargs.get_item("recursive") {
            if let Ok(v) = v.extract::<bool>() {
                if v {
                    return AnnotationDepth::Max;
                } else {
                    return AnnotationDepth::One;
                }
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
