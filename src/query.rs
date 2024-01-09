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

pub struct ContextVariables(Vec<String>);

impl ContextVariables {
    pub fn add(&mut self) -> &str {
        let var = format!("v{}", self.0.len() + 1);
        self.0.push(var);
        self.last().expect("was just added")
    }

    pub fn last(&self) -> Option<&str> {
        self.0.iter().last().map(|x| x.as_str())
    }
}

impl Default for ContextVariables {
    fn default() -> Self {
        Self(Vec::new())
    }
}

fn add_filter<'store, 'py>(
    query: &mut Query<'store>,
    store: &'store AnnotationStore,
    filter: &'py PyAny,
    operator: Option<DataOperator<'store>>,
    contextvariables: &mut ContextVariables,
) -> PyResult<()>
where
    'py: 'store,
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
                let varname = contextvariables.add();
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
                    "Passed key instance is invalid (should not happen)",
                ));
            }
        } else if filter.contains("set")? {
            if key.is_instance_of::<PyString>() {
                let key = key.downcast::<PyString>()?;
                let set = filter.get_item("set")?.expect("already checked");
                let key = if set.is_instance_of::<PyAnnotationDataSet>() {
                    let set: PyRef<'py, PyAnnotationDataSet> = filter.extract()?;
                    let key: &str = key.to_str()?;
                    set.map(|set| Ok(set.key(key))).unwrap()
                } else if set.is_instance_of::<PyString>() {
                    let set = set.downcast::<PyString>()?;
                    if let Some(set) = store.dataset(set.to_str()?) {
                        set.key(key.to_str()?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(key) = key {
                    let varname = key.as_ref().temp_id().expect("temp id must work");
                    query.bind_keyvar(varname.as_str(), key);
                    if operator.is_some() {
                        query.constrain(Constraint::KeyValueVariable(
                            varname.as_str(),
                            operator.take().unwrap(),
                            SelectionQualifier::Normal,
                        ));
                    } else {
                        query.constrain(Constraint::KeyVariable(
                            varname.as_str(),
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
        data.map(|data| {
            let varname = contextvariables.add();
            query.bind_datavar(varname, data);
            query.constrain(Constraint::DataVariable(
                varname,
                SelectionQualifier::Normal,
            ));
            Ok(())
        });
    } else if filter.is_instance_of::<PyDataKey>() {
        let key: PyRef<'py, PyDataKey> = filter.extract()?;
        key.map(|key| {
            let varname = contextvariables.add();
            query.bind_keyvar(varname, key);
            if operator.is_some() {
                query.constrain(Constraint::KeyValueVariable(
                    varname,
                    operator.take().unwrap(),
                    SelectionQualifier::Normal,
                ));
            } else {
                query.constrain(Constraint::KeyVariable(varname, SelectionQualifier::Normal));
            }
            Ok(())
        });
    } else if filter.is_instance_of::<PyAnnotation>() {
        let annotation: PyRef<'py, PyAnnotation> = filter.extract()?;
        annotation.map(|annotation| {
            let varname = annotation.as_ref().temp_id()?;
            query.bind_annotationvar(varname.as_str(), annotation);
            query.constrain(Constraint::AnnotationVariable(
                varname.as_str(),
                SelectionQualifier::Normal,
                AnnotationDepth::One,
            ));
            Ok(())
        });
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
    Ok(())
}

fn add_multi_filter<'a>(
    query: &mut Query<'a>,
    store: &AnnotationStore,
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
    args: &'py PyList, //TODO: implement!
    kwargs: Option<&'py PyDict>,
    store: &'store AnnotationStore,
) -> PyResult<(Query<'store>, ContextVariables)> {
    let mut contextvariables = ContextVariables::default();
    let operator = if let Some(kwargs) = kwargs {
        dataoperator_from_kwargs(kwargs).map_err(|e| PyStamError::new_err(format!("{}", e)))?
    } else {
        None
    };
    for filter in args {
        add_filter(&mut query, store, filter, operator, &mut contextvariables)?;
    }
    if let Some(kwargs) = kwargs {
        if let Ok(Some(filter)) = kwargs.get_item("filter") {
            add_filter(&mut query, store, filter, operator, &mut contextvariables)?;
        } else if let Ok(Some(filter)) = kwargs.get_item("filters") {
            if filter.is_instance_of::<PyList>() {
                let vec = filter.downcast::<PyList>()?;
                for filter in vec {
                    add_filter(&mut query, store, filter, operator, &mut contextvariables)?;
                }
            } else if filter.is_instance_of::<PyTuple>() {
                let vec = filter.downcast::<PyTuple>()?;
                for filter in vec {
                    add_filter(&mut query, store, filter, operator, &mut contextvariables)?;
                }
            }
        }
    }
    Ok((query, contextvariables))
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