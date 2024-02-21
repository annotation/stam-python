use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{annotationdata_builder, PyAnnotationData, PyData, PyDataKey};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::config::get_config;
use crate::error::PyStamError;
use crate::query::*;
use crate::resources::PyTextResource;
use crate::selector::PySelector;
use crate::textselection::PyTextSelection;
use stam::*;

#[pyclass(dict, module = "stam", name = "AnnotationStore")]
/// An Annotation Store is an unordered collection of annotations, resources and
/// annotation data sets. It can be seen as the *root* of the *graph model* and the glue
/// that holds everything together. It is the entry point for any stam model.
///
/// Args:
///     `id` (:obj:`str`, `optional`) - The public ID for a *new* store
///     `file` (:obj:`str`, `optional`) - The STAM JSON or STAM CSV file to load
///     `string` (:obj:`str`, `optional`) - STAM JSON as a string
///     `config` (:obj:`dict`, `optional`) - A python dictionary containing configuration parameters
///
/// At least one of `id`, `file` or `string` must be specified.
pub struct PyAnnotationStore {
    store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyAnnotationStore {
    #[new]
    #[pyo3(signature = (**kwargs))]
    #[pyo3(text_signature = "(self, id=None, file=None, string=None, config=None)")]
    fn new<'py>(kwargs: Option<&PyDict>, py: Python<'py>) -> PyResult<Self> {
        if let Some(kwargs) = kwargs {
            let mut config: &PyDict = PyDict::new(py);
            for (key, value) in kwargs {
                if let Some(key) = key.extract().unwrap() {
                    match key {
                        "config" => {
                            if let Ok(Some(value)) = value.extract() {
                                config = value;
                            }
                        }
                        _ => continue,
                    }
                }
            }
            for (key, value) in kwargs {
                if let Some(key) = key.extract().unwrap() {
                    match key {
                        "config" => continue, //already handled
                        "file" => {
                            if let Ok(Some(value)) = value.extract() {
                                return match AnnotationStore::from_file(value, get_config(config)) {
                                    Ok(store) => Ok(PyAnnotationStore {
                                        store: Arc::new(RwLock::new(store)),
                                    }),
                                    Err(err) => Err(PyStamError::new_err(format!("{}", err))),
                                };
                            }
                        }
                        "string" => {
                            if let Ok(Some(value)) = value.extract() {
                                return match AnnotationStore::from_str(value, get_config(config)) {
                                    Ok(store) => Ok(PyAnnotationStore {
                                        store: Arc::new(RwLock::new(store)),
                                    }),
                                    Err(err) => Err(PyStamError::new_err(format!("{}", err))),
                                };
                            }
                        }
                        "id" => {
                            if let Ok(Some(value)) = value.extract::<Option<String>>() {
                                return Ok(PyAnnotationStore {
                                    store: Arc::new(RwLock::new(
                                        AnnotationStore::default()
                                            .with_id(value)
                                            .with_config(get_config(config)),
                                    )),
                                });
                            }
                        }
                        _ => eprintln!("Ignored unknown kwargs option {}", key),
                    }
                }
            }
        }
        Ok(PyAnnotationStore {
            store: Arc::new(RwLock::new(AnnotationStore::default())),
        })
    }

    /// Returns the public ID (by value, aka a copy)
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|store| Ok(store.id().map(|x| x.to_owned())))
    }

    fn from_file(&mut self, filename: &str) -> PyResult<()> {
        self.map_mut(|store| store.merge_json_file(filename))
    }

    /// Saves the annotation store to file
    fn to_file(&mut self, filename: &str) -> PyResult<()> {
        self.set_filename(filename)?;
        self.save()
    }

    /// Saves the annotation store to file
    fn save(&self) -> PyResult<()> {
        self.map(|store| store.save())
    }

    /// Returns the annotation store as one big STAM JSON string
    fn to_json_string(&self) -> PyResult<String> {
        self.map(|store| store.to_json_string(store.config()))
    }

    /// Returns an AnnotationDataSet by ID
    fn dataset(&self, id: &str) -> PyResult<PyAnnotationDataSet> {
        self.map(|store| {
            store
                .resolve_dataset_id(id)
                .map(|handle| PyAnnotationDataSet::new(handle, &self.store))
        })
    }

    /// Returns an Annotation by ID
    fn annotation(&self, id: &str) -> PyResult<PyAnnotation> {
        self.map(|store| {
            store
                .resolve_annotation_id(id)
                .map(|handle| PyAnnotation::new(handle, &self.store))
        })
    }

    /// Returns a TextResource by ID
    fn resource(&self, id: &str) -> PyResult<PyTextResource> {
        self.map(|store| {
            store
                .resolve_resource_id(id)
                .map(|handle| PyTextResource::new(handle, &self.store))
        })
    }

    /// Returns a key by ID
    fn key(&self, set_id: &str, key_id: &str) -> PyResult<PyDataKey> {
        self.map(|store| {
            let key = store.key(set_id, key_id).or_fail()?;
            Ok(PyDataKey::new(
                key.handle(),
                key.set().handle(),
                &self.store,
            ))
        })
    }

    /// Returns data by ID
    fn annotationdata(&self, set_id: &str, data_id: &str) -> PyResult<PyAnnotationData> {
        self.map(|store| {
            let data = store.annotationdata(set_id, data_id).or_fail()?;
            Ok(PyAnnotationData::new(
                data.handle(),
                data.set().handle(),
                &self.store,
            ))
        })
    }

    /// Create a new TextResource and adds it to the store
    fn add_resource(
        &mut self,
        filename: Option<&str>,
        text: Option<String>,
        id: Option<&str>,
    ) -> PyResult<PyTextResource> {
        if id.is_none() && filename.is_none() {
            return Err(PyRuntimeError::new_err(
                "Incomplete, set either id or filename",
            ));
        }
        if filename.is_some() && text.is_some() {
            return Err(PyRuntimeError::new_err(
                "Set either filename or text keyword arguments, but not both",
            ));
        }
        let store_clone = self.store.clone(); //just a smart pointer clone, not the whole store
        self.map_mut(|store| {
            let mut resource = TextResourceBuilder::new()
                .with_id(
                    id.unwrap_or_else(|| filename.expect("filename"))
                        .to_string(),
                )
                .with_config(store.config().clone());
            if let Some(text) = text {
                resource = resource.with_text(text);
            } else if let Some(filename) = filename {
                resource = resource.with_filename(filename);
            }
            let handle = store.insert(resource.build()?)?;
            Ok(PyTextResource {
                handle,
                store: store_clone,
            })
        })
    }

    fn set_filename(&mut self, filename: &str) -> PyResult<()> {
        self.map_mut(|store| {
            store.set_filename(filename);
            Ok(())
        })
    }

    /// Create a new AnnotationDataSet and adds it to the store
    fn add_dataset(&mut self, id: String) -> PyResult<PyAnnotationDataSet> {
        let store_clone = self.store.clone();
        self.map_mut(|store| {
            let dataset = AnnotationDataSet::new(store.config().clone()).with_id(id);
            let handle = store.insert(dataset)?;
            Ok(PyAnnotationDataSet {
                handle,
                store: store_clone,
            })
        })
    }

    /// Adds an annotation. Returns an :obj:`Annotation` instance pointing to the added annotation.
    ///
    /// Args:
    ///       `target` (:obj:`Selector`) - A target selector
    ///       `data` (:obj:`dict`) - A dictionary or list of dictionaries with data to set. The dictionary
    ///                              has may have fields: `id`,`key`,`set`, and `value`.
    ///                              Alternatively, you can pass an existing`AnnotationData` instance.
    ///       `id` (:obj:`str`, `optional`) - The public ID for the annotation
    #[pyo3(signature = (target, data, id=None))]
    fn annotate(
        &mut self,
        target: PySelector,
        data: &PyAny, //dictionary or list of dictionaries
        id: Option<String>,
    ) -> PyResult<PyAnnotation> {
        let mut builder = AnnotationBuilder::new();
        if let Some(id) = id {
            builder = builder.with_id(id);
        }
        builder = builder.with_target(target.build());
        if data.is_instance_of::<PyList>() {
            let data: &PyList = data.downcast().unwrap();
            for databuilder in data.iter() {
                let databuilder = annotationdata_builder(databuilder)?;
                builder = builder.with_data_builder(databuilder);
            }
        } else {
            let databuilder = annotationdata_builder(data)?;
            builder = builder.with_data_builder(databuilder);
        }
        let store_clone = self.store.clone(); //just a smart pointer clone, not the whole store
        self.map_mut(|store| {
            Ok(PyAnnotation {
                handle: store.annotate(builder)?,
                store: store_clone,
            })
        })
    }

    /// Returns a generator over all annotations in this store
    fn __iter__(&self) -> PyResult<PyAnnotationIter> {
        Ok(PyAnnotationIter {
            store: self.store.clone(),
            index: 0,
        })
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn annotations(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|store| {
                Ok(PyAnnotations::from_iter(
                    store.annotations().limit(limit),
                    &self.store,
                ))
            })
        } else {
            self.map_with_query(Type::Annotation, args, kwargs, |query, store| {
                Ok(PyAnnotations::from_query(query, store, &self.store, limit))
            })
        }
    }

    /// Returns a generator over all annotations in this store
    fn datasets(&self) -> PyResult<PyAnnotationDataSetIter> {
        Ok(PyAnnotationDataSetIter {
            store: self.store.clone(),
            index: 0,
        })
    }

    /// Returns a generator over all resources in this store
    fn resources(&self) -> PyResult<PyResourceIter> {
        //TODO: transform to PyResources
        Ok(PyResourceIter {
            store: self.store.clone(),
            index: 0,
        })
    }

    /// Returns the number of annotations in the store (not substracting deletions)
    fn annotations_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.annotations_len()))
    }

    /// Returns the number of resources in the store (not substracting deletions)
    fn resources_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.resources_len()))
    }

    /// Returns the number of annotation data sets in the store (not substracting deletions)
    fn datasets_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.datasets_len()))
    }

    fn shrink_to_fit(&mut self) -> PyResult<()> {
        self.map_mut(|store| Ok(store.shrink_to_fit(true)))
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn data(&self, args: &PyTuple, kwargs: Option<&PyDict>) -> PyResult<PyData> {
        let limit = get_limit(kwargs);
        if !has_filters(args, kwargs) {
            self.map(|store| Ok(PyData::from_iter(store.data().limit(limit), &self.store)))
        } else {
            self.map_with_query(Type::AnnotationData, args, kwargs, |query, store| {
                Ok(PyData::from_query(query, store, &self.store, limit))
            })
        }
    }

    #[pyo3(signature = (querystring, **kwargs))]
    fn query<'py>(
        &self,
        querystring: &str,
        kwargs: Option<&'py PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        self.map(|store| {
            let (mut query, _) = Query::parse(querystring)?;
            if let Some(kwargs) = kwargs {
                //bind keyword arguments as variables in the query
                for (varname, value) in kwargs.iter() {
                    if let Ok(varname) = varname.downcast::<PyString>() {
                        if let Ok(varname) = varname.to_str() {
                            if value.is_instance_of::<PyAnnotation>() {
                                let annotation: PyResult<PyRef<'py, PyAnnotation>> =
                                    value.extract();
                                if let Ok(annotation) = annotation {
                                    let annotation =
                                        store.annotation(annotation.handle).or_fail()?;
                                    query.bind_annotationvar(varname, annotation);
                                }
                            } else if value.is_instance_of::<PyAnnotationData>() {
                                let data: PyResult<PyRef<'py, PyAnnotationData>> =
                                    value.extract();
                                if let Ok(data) = data {
                                    let data =
                                        store.annotationdata(data.set, data.handle).or_fail()?;
                                    query.bind_datavar(varname, data);
                                }
                            } else if value.is_instance_of::<PyDataKey>() {
                                let key: PyResult<PyRef<'py, PyDataKey>> =
                                    value.extract();
                                if let Ok(key) = key {
                                    let key =
                                        store.key(key.set, key.handle).or_fail()?;
                                    query.bind_keyvar(varname, key);
                                }
                            } else if value.is_instance_of::<PyTextResource>() {
                                let resource: PyResult<PyRef<'py, PyTextResource>> =
                                    value.extract();
                                if let Ok(resource) = resource {
                                    let resource =
                                        store.resource(resource.handle).or_fail()?;
                                    query.bind_resourcevar(varname, resource);
                                }
                            } else if value.is_instance_of::<PyAnnotationDataSet>() {
                                let dataset: PyResult<PyRef<'py, PyAnnotationDataSet>> =
                                    value.extract();
                                if let Ok(dataset) = dataset {
                                    let dataset =
                                        store.dataset(dataset.handle).or_fail()?;
                                    query.bind_datasetvar(varname, dataset);
                                }
                            } else if value.is_instance_of::<PyTextSelection>() {
                                let textselection: PyResult<PyRef<'py, PyTextSelection>> =
                                    value.extract();
                                if let Ok(textselection) = textselection {
                                    if let Some(handle) = textselection.textselection.handle() {
                                        if let Some(textselection) =
                                            store.textselection(textselection.resource_handle, handle) {
                                            query.bind_textvar(varname, textselection);
                                        }
                                    }
                                }
                            } else {
                                return Err(StamError::ValueError(format!("Keyword argument {} can not be bound to a variable because the value has an invalid type", varname),"stam-python"));
                            }
                        }
                    }
                }
            }

            //run the query and convert the output to a python structure (list of dicts)
            query_to_python(store.query(query), &self.store, py)
        })
        .map_err(|err| err.into())
    }
}

pub(crate) trait MapStore {
    fn get_store(&self) -> &Arc<RwLock<AnnotationStore>>;
    fn get_store_mut(&mut self) -> &mut Arc<RwLock<AnnotationStore>>;

    /// Map function only on the store
    fn map_store<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.get_store().read() {
            f(&store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn map_store_mut<T, F>(&mut self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&mut AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(mut store) = self.get_store_mut().write() {
            f(&mut store).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "unable to obtain exclusive lock for writing to store",
            ))
        }
    }
}

impl MapStore for PyAnnotationStore {
    fn get_store(&self) -> &Arc<RwLock<AnnotationStore>> {
        &self.store
    }
    fn get_store_mut(&mut self) -> &mut Arc<RwLock<AnnotationStore>> {
        &mut self.store
    }
}

impl PyAnnotationStore {
    /// Map function to act on the actual unlderyling store, helps reduce boilerplate
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&AnnotationStore) -> Result<T, StamError>,
    {
        self.map_store(f)
    }

    fn map_mut<T, F>(&mut self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(&mut AnnotationStore) -> Result<T, StamError>,
    {
        self.map_store_mut(f)
    }

    fn map_with_query<T, F>(
        &self,
        resulttype: Type,
        args: &PyTuple,
        kwargs: Option<&PyDict>,
        f: F,
    ) -> Result<T, PyErr>
    where
        F: FnOnce(Query, &AnnotationStore) -> Result<T, StamError>,
    {
        self.map_store(|store| {
            let query = build_query(
                Query::new(QueryType::Select, Some(resulttype), None),
                args,
                kwargs,
                store,
            )
            .map_err(|e| StamError::QuerySyntaxError(format!("{}", e), "(python to query)"))?;
            f(query, store)
        })
    }
}

#[pyclass(name = "AnnotationIter")]
struct PyAnnotationIter {
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyAnnotationIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotation> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|store| {
            let handle: AnnotationHandle = AnnotationHandle::new(pyself.index - 1);
            if let Ok(annotation) = store.get(handle) {
                //index is one ahead, prevents exclusive lock issues
                let handle = annotation.handle().expect("annotation must have a handle");
                Some(PyAnnotation {
                    handle,
                    store: pyself.store.clone(),
                })
            } else {
                None
            }
        });
        if result.is_some() {
            result
        } else {
            if pyself.index >= pyself.map(|store| Some(store.annotations_len())).unwrap() {
                None
            } else {
                Self::__next__(pyself)
            }
        }
    }
}

impl PyAnnotationIter {
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&AnnotationStore) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            f(&store)
        } else {
            None //should never happen here
        }
    }
}

#[pyclass(name = "AnnotationDataSetIter")]
struct PyAnnotationDataSetIter {
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyAnnotationDataSetIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotationDataSet> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|store| {
            let handle: AnnotationDataSetHandle = AnnotationDataSetHandle::new(pyself.index - 1);
            if let Ok(annotationset) = store.get(handle) {
                //index is one ahead, prevents exclusive lock issues
                let handle = annotationset.handle().expect("annotation must have an ID");
                Some(PyAnnotationDataSet {
                    handle,
                    store: pyself.store.clone(),
                })
            } else {
                None
            }
        });
        if result.is_some() {
            result
        } else {
            if pyself.index >= pyself.map(|store| Some(store.datasets_len())).unwrap() {
                None
            } else {
                Self::__next__(pyself)
            }
        }
    }
}

impl PyAnnotationDataSetIter {
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&AnnotationStore) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            f(&store)
        } else {
            None //should never happen here
        }
    }
}

#[pyclass(name = "ResourceIter")]
struct PyResourceIter {
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PyResourceIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyTextResource> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|store| {
            let handle: TextResourceHandle = TextResourceHandle::new(pyself.index - 1);
            if let Ok(res) = store.get(handle) {
                //index is one ahead, prevents exclusive lock issues
                let handle = res.handle().expect("annotation must have an ID");
                Some(PyTextResource {
                    handle,
                    store: pyself.store.clone(),
                })
            } else {
                None
            }
        });
        if result.is_some() {
            result
        } else {
            if pyself.index >= pyself.map(|store| Some(store.annotations_len())).unwrap() {
                None
            } else {
                Self::__next__(pyself)
            }
        }
    }
}

impl PyResourceIter {
    fn map<T, F>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&AnnotationStore) -> Option<T>,
    {
        if let Ok(store) = self.store.read() {
            f(&store)
        } else {
            None //should never happen here
        }
    }
}
