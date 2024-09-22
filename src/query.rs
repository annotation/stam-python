use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::*;
use std::sync::{Arc, RwLock};

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{dataoperator_from_kwargs, PyAnnotationData, PyData, PyDataKey};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::error::PyStamError;
use crate::resources::PyTextResource;
use crate::substore::PyAnnotationSubStore;
use crate::textselection::PyTextSelection;
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
        if filter.contains("substore")? {
            let substore = filter
                .get_item("substore")?
                .expect("substore field was checked to exist in filter");
            if substore.is_instance_of::<PyAnnotationSubStore>() {
                let substore: PyRef<'py, PyAnnotationSubStore> = substore.extract()?;
                if let Some(substore) = store.substore(substore.handle) {
                    let varname = new_contextvar(&mut used_contextvarnames);
                    query.bind_substorevar(varname, &substore);
                    query.constrain(Constraint::SubStoreVariable(varname));
                } else {
                    return Err(PyValueError::new_err(
                        "Passed AnnotationSubStore instance is invalid (should never happen)",
                    ));
                }
            } else if substore.is_instance_of::<PyBool>() {
                //root store only
                let substore: bool = substore.extract()?;
                if !substore {
                    query.constrain(Constraint::SubStore(None));
                }
            }
        } else if filter.contains("key")? {
            let key = filter
                .get_item("key")?
                .expect("key field was checked to exist in filter");
            if key.is_instance_of::<PyDataKey>() {
                let key: PyRef<'py, PyDataKey> = filter.extract()?;
                if let Some(key) = store.key(key.set, key.handle) {
                    let varname = new_contextvar(&mut used_contextvarnames);
                    query.bind_keyvar(varname, &key);
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
                                return Err(PyValueError::new_err(
                                    "specified key not found in set",
                                ));
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
                                return Err(PyValueError::new_err(
                                    "specified key not found in set",
                                ));
                            }
                        } else {
                            return Err(PyValueError::new_err("specified dataset not found"));
                        }
                    } else {
                        None
                    };
                    if let Some(key) = key {
                        let varname = new_contextvar(&mut used_contextvarnames);
                        query.bind_keyvar(varname, &key);
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
                    "`key` provided but `set` is missing, this is only valid if key is a DataKey instance, not if it is a string. Pass a `set` as well",
                ));
            }
        } else if let Some(operator) = operator {
            //no key specified but we do have an operator
            query.constrain(Constraint::Value(operator, SelectionQualifier::Normal));
        }
    } else if filter.is_instance_of::<PyList>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        used_contextvarnames = add_multi_filter(query, store, vec, used_contextvarnames)?;
    } else if filter.is_instance_of::<PyTuple>() {
        let vec: Vec<&PyAny> = filter.extract()?;
        used_contextvarnames = add_multi_filter(query, store, vec, used_contextvarnames)?;
    } else if filter.is_instance_of::<PyAnnotationData>() {
        let data: PyRef<'_, PyAnnotationData> = filter.extract()?;
        if operator.is_some() {
            return Err(PyValueError::new_err(
                    "'value' parameter can not be used in combination with an AnnotationData instance (it already restrains to a single value)",
            ));
        }
        if let Some(data) = store.annotationdata(data.set, data.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_datavar(varname, &data);
            query.constrain(Constraint::DataVariable(
                varname,
                SelectionQualifier::Normal,
            ));
        }
    } else if filter.is_instance_of::<PyDataKey>() {
        let key: PyRef<'py, PyDataKey> = filter.extract()?;
        if let Some(key) = store.key(key.set, key.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_keyvar(varname, &key);
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
            query.bind_annotationvar(varname, &annotation);
            query.constrain(Constraint::AnnotationVariable(
                varname,
                SelectionQualifier::Normal,
                AnnotationDepth::One,
                None,
            ));
        } else {
            return Err(PyValueError::new_err(
                "Passed Annotation instance is invalid (should never happen)",
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
    } else if filter.is_instance_of::<PyAnnotationSubStore>() {
        let substore: PyRef<'py, PyAnnotationSubStore> = filter.extract()?;
        if let Some(substore) = store.substore(substore.handle) {
            let varname = new_contextvar(&mut used_contextvarnames);
            query.bind_substorevar(varname, &substore);
            query.constrain(Constraint::SubStoreVariable(varname));
        } else {
            return Err(PyValueError::new_err(
                "Passed AnnotationSubStore instance is invalid (should never happen)",
            ));
        }
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
    filter: Vec<&'a PyAny>,
    mut used_contextvarnames: usize,
) -> PyResult<usize> {
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
    } else {
        for item in filter.iter() {
            used_contextvarnames = add_filter(query, store, item, None, used_contextvarnames)?;
        }
    }
    Ok(used_contextvarnames)
}

pub(crate) fn build_query<'store, 'py>(
    mut query: Query<'store>,
    args: &'py PyTuple,
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
    let mut has_args = false;
    for filter in args {
        has_args = true;
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
            //backwards compatibility
            add_filter(&mut query, store, filter, operator, used_contextvarnames)?;
        } else if let Ok(Some(filter)) = kwargs.get_item("filters") {
            //backwards compatibility
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
        } else if !has_args {
            //we have no args, handle kwargs standalone
            add_filter(
                &mut query,
                store,
                kwargs.as_ref(),
                None,
                used_contextvarnames,
            )?;
        }
    }
    Ok(query)
}

pub(crate) fn has_filters(args: &PyTuple, kwargs: Option<&PyDict>) -> bool {
    if !args.is_empty() {
        return true;
    }
    if let Some(kwargs) = kwargs {
        for key in kwargs.keys() {
            if let Ok(Some("limit")) | Ok(Some("recursive")) | Ok(Some("substore")) = key.extract()
            {
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

pub(crate) fn get_bool(kwargs: Option<&PyDict>, name: &str, default: bool) -> bool {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(v)) = kwargs.get_item(name) {
            if let Ok(v) = v.extract::<bool>() {
                return v;
            }
        }
    }
    default
}

pub(crate) fn get_opt_string(
    kwargs: Option<&PyDict>,
    name: &str,
    default: Option<&str>,
) -> Option<String> {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(v)) = kwargs.get_item(name) {
            if let Ok(v) = v.extract::<String>() {
                return Some(v.clone());
            }
        }
    }
    default.map(|s| s.to_string())
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

pub(crate) fn get_substore(kwargs: Option<&PyDict>) -> Option<bool> {
    if let Some(kwargs) = kwargs {
        if let Ok(Some(substore)) = kwargs.get_item("substore") {
            if let Ok(substore) = substore.extract::<bool>() {
                return Some(substore);
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

/// Converts a QueryIter to a Python list with dictionaries for each result, the dictionary keys correspond to the variable names from the query.
pub(crate) fn query_to_python<'py>(
    iter: QueryIter,
    store: Arc<RwLock<AnnotationStore>>,
    py: Python<'py>,
) -> Result<&'py PyList, StamError> {
    let results = PyList::empty(py);
    for resultitems in iter {
        let dict = PyDict::new(py);
        for (result, name) in resultitems.iter().zip(resultitems.names()) {
            if name.is_none() {
                continue;
            }
            let name = name.unwrap();
            match result {
                QueryResultItem::Annotation(annotation) => {
                    dict.set_item(
                        name,
                        PyAnnotation::new(annotation.handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::AnnotationData(data) => {
                    dict.set_item(
                        name,
                        PyAnnotationData::new(data.handle(), data.set().handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::DataKey(key) => {
                    dict.set_item(
                        name,
                        PyDataKey::new(key.handle(), key.set().handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::TextResource(resource) => {
                    dict.set_item(
                        name,
                        PyTextResource::new(resource.handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::AnnotationDataSet(dataset) => {
                    dict.set_item(
                        name,
                        PyAnnotationDataSet::new(dataset.handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::TextSelection(textselection) => {
                    dict.set_item(
                        name,
                        PyTextSelection::new(
                            textselection
                                .as_ref()
                                .expect("textselection must be bound")
                                .clone(),
                            textselection.resource().handle(),
                            store.clone(),
                        )
                        .into_py(py)
                        .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::AnnotationSubStore(substore) => {
                    dict.set_item(
                        name,
                        PyAnnotationSubStore::new(substore.handle(), store.clone())
                            .into_py(py)
                            .into_ref(py),
                    )
                    .unwrap();
                }
                QueryResultItem::None => {}
            }
        }
        let _ = results.append(dict);
    }
    Ok(results)
}
