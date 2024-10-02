use pyo3::exceptions::{PyRuntimeError, PyTypeError, PyValueError};
use pyo3::ffi::PyVarObject;
use pyo3::prelude::*;
use pyo3::types::*;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::{PyAnnotation, PyAnnotations};
use crate::annotationdata::{annotationdata_builder, PyAnnotationData, PyData, PyDataKey};
use crate::annotationdataset::PyAnnotationDataSet;
use crate::config::{get_alignmentconfig, get_config};
use crate::error::PyStamError;
use crate::query::*;
use crate::resources::PyTextResource;
use crate::selector::PySelector;
use crate::substore::PyAnnotationSubStore;
use crate::textselection::PyTextSelection;
use stam::*;
use stamtools::align::{align_texts, AlignmentConfig};
use stamtools::split::{split, SplitMode};
use stamtools::view::{AnsiWriter, HtmlWriter};

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
                .map(|handle| PyAnnotationDataSet::new(handle, self.store.clone()))
        })
    }

    /// Returns an Annotation by ID
    fn annotation(&self, id: &str) -> PyResult<PyAnnotation> {
        self.map(|store| {
            store
                .resolve_annotation_id(id)
                .map(|handle| PyAnnotation::new(handle, self.store.clone()))
        })
    }

    /// Returns a TextResource by ID
    fn resource(&self, id: &str) -> PyResult<PyTextResource> {
        self.map(|store| {
            store
                .resolve_resource_id(id)
                .map(|handle| PyTextResource::new(handle, self.store.clone()))
        })
    }

    /// Returns a key by ID
    fn key(&self, set_id: &str, key_id: &str) -> PyResult<PyDataKey> {
        self.map(|store| {
            let key = store.key(set_id, key_id).or_fail()?;
            Ok(PyDataKey::new(
                key.handle(),
                key.set().handle(),
                self.store.clone(),
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
                self.store.clone(),
            ))
        })
    }

    /// Create a new TextResource or load an existing one and adds it to the store
    fn add_resource(
        &mut self,
        filename: Option<&str>,
        text: Option<String>,
        id: Option<&str>,
    ) -> PyResult<PyTextResource> {
        if id.is_none() && filename.is_none() {
            return Err(PyRuntimeError::new_err(
                "Incomplete, set either id and/or filename",
            ));
        }
        let store_clone = self.store.clone(); //just a smart pointer clone, not the whole store
        self.map_mut(|store| {
            let mut resource = TextResourceBuilder::new().with_id(
                id.unwrap_or_else(|| filename.expect("filename"))
                    .to_string(),
            );
            if let Some(text) = text {
                resource = resource.with_text(text);
            }
            if let Some(filename) = filename {
                resource = resource.with_filename(filename);
            }
            let handle = store.add_resource(resource)?;
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

    /// Create a new AnnotationDataSet or load an existing one and adds it to the store
    fn add_dataset(
        &mut self,
        id: Option<&str>,
        filename: Option<&str>,
    ) -> PyResult<PyAnnotationDataSet> {
        if id.is_none() && filename.is_none() {
            return Err(PyRuntimeError::new_err(
                "Incomplete, set either id or filename",
            ));
        }
        let store_clone = self.store.clone();
        self.map_mut(|store| {
            let mut dataset = AnnotationDataSetBuilder::new();
            if let Some(filename) = filename {
                dataset = dataset.with_filename(filename)
            };
            if let Some(id) = id {
                dataset = dataset.with_id(id);
            }
            let handle = store.add_dataset(dataset)?;
            Ok(PyAnnotationDataSet {
                handle,
                store: store_clone,
            })
        })
    }

    /// Load an existing annotation store as a dependency to this one
    fn add_substore(&mut self, filename: &str) -> PyResult<PyAnnotationSubStore> {
        let store_clone = self.store.clone();
        self.map_mut(|store| {
            let handle = store.add_substore(filename)?;
            Ok(PyAnnotationSubStore {
                handle,
                store: store_clone,
            })
        })
    }

    /// Create a new annotation store as a dependency of this one
    fn add_new_substore(&mut self, id: &str, filename: &str) -> PyResult<PyAnnotationSubStore> {
        let store_clone = self.store.clone();
        self.map_mut(|store| {
            let handle = store.add_new_substore(id, filename)?;
            Ok(PyAnnotationSubStore {
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
        let substore = get_substore(kwargs);
        if !has_filters(args, kwargs) {
            if substore == Some(false) {
                self.map(|store| {
                    Ok(PyAnnotations::from_iter(
                        store.annotations_no_substores().limit(limit),
                        &self.store,
                    ))
                })
            } else {
                self.map(|store| {
                    Ok(PyAnnotations::from_iter(
                        store.annotations().limit(limit),
                        &self.store,
                    ))
                })
            }
        } else {
            self.map_with_query(Type::Annotation, args, kwargs, |query, store| {
                PyAnnotations::from_query(query, store, &self.store, limit)
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

    /// Returns a generator over all substores in this store
    fn substores(&self) -> PyResult<PySubStoreIter> {
        Ok(PySubStoreIter {
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

    /// Returns the number of substores in the store (not substracting deletions)
    fn substores_len(&self) -> PyResult<usize> {
        self.map(|store| Ok(store.substores_len()))
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
                PyData::from_query(query, store, &self.store, limit)
            })
        }
    }

    #[pyo3(signature = (querystring, **kwargs))]
    fn query<'py>(
        &mut self,
        querystring: &str,
        kwargs: Option<&'py PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        let clonedstore = self.store.clone();
        self.map_mut(|store| {
            let (mut query, _) = Query::parse(querystring)?;
            let readonly = get_bool(kwargs, "readonly", false);
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
                                    query.bind_annotationvar(varname, &annotation);
                                }
                            } else if value.is_instance_of::<PyAnnotationData>() {
                                let data: PyResult<PyRef<'py, PyAnnotationData>> =
                                    value.extract();
                                if let Ok(data) = data {
                                    let data =
                                        store.annotationdata(data.set, data.handle).or_fail()?;
                                    query.bind_datavar(varname, &data);
                                }
                            } else if value.is_instance_of::<PyDataKey>() {
                                let key: PyResult<PyRef<'py, PyDataKey>> =
                                    value.extract();
                                if let Ok(key) = key {
                                    let key =
                                        store.key(key.set, key.handle).or_fail()?;
                                    query.bind_keyvar(varname, &key);
                                }
                            } else if value.is_instance_of::<PyTextResource>() {
                                let resource: PyResult<PyRef<'py, PyTextResource>> =
                                    value.extract();
                                if let Ok(resource) = resource {
                                    let resource =
                                        store.resource(resource.handle).or_fail()?;
                                    query.bind_resourcevar(varname, &resource);
                                }
                            } else if value.is_instance_of::<PyAnnotationDataSet>() {
                                let dataset: PyResult<PyRef<'py, PyAnnotationDataSet>> =
                                    value.extract();
                                if let Ok(dataset) = dataset {
                                    let dataset =
                                        store.dataset(dataset.handle).or_fail()?;
                                    query.bind_datasetvar(varname, &dataset);
                                }
                            } else if value.is_instance_of::<PyTextSelection>() {
                                let textselection: PyResult<PyRef<'py, PyTextSelection>> =
                                    value.extract();
                                if let Ok(textselection) = textselection {
                                    if let Some(handle) = textselection.textselection.handle() {
                                        if let Some(textselection) =
                                            store.textselection(textselection.resource_handle, handle) {
                                            query.bind_textvar(varname, &textselection);
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
            let iter = if readonly {
                store.query_mut(query)?
            } else {
                store.query(query)?
            };
            //run the query and convert the output to a python structure (list of dicts)
            query_to_python(iter, clonedstore, py)
        })
        .map_err(|err| err.into())
    }

    #[pyo3(signature = (item, **kwargs))]
    fn remove<'py>(&mut self, item: &'py PyAny, kwargs: Option<&'py PyDict>) -> PyResult<()> {
        let strict = get_bool(kwargs, "strict", false);
        if item.is_instance_of::<PyAnnotation>() {
            let item: PyRef<'py, PyAnnotation> = item.extract()?;
            self.map_store_mut(|store| store.remove(item.handle))
        } else if item.is_instance_of::<PyTextResource>() {
            let item: PyRef<'py, PyTextResource> = item.extract()?;
            self.map_store_mut(|store| store.remove(item.handle))
        } else if item.is_instance_of::<PyAnnotationDataSet>() {
            let item: PyRef<'py, PyAnnotationDataSet> = item.extract()?;
            self.map_store_mut(|store| store.remove(item.handle))
        } else if item.is_instance_of::<PyAnnotationData>() {
            let item: PyRef<'py, PyAnnotationData> = item.extract()?;
            self.map_store_mut(|store| store.remove_data(item.set, item.handle, strict))
        } else if item.is_instance_of::<PyDataKey>() {
            let item: PyRef<'py, PyDataKey> = item.extract()?;
            self.map_store_mut(|store| store.remove_key(item.set, item.handle, strict))
        } else {
            Err(PyTypeError::new_err("Expected a STAM item"))
        }
    }

    #[pyo3(signature = (querystring, *args, **kwargs))]
    fn view<'py>(
        &self,
        querystring: &str,
        args: &'py PyTuple,
        kwargs: Option<&'py PyDict>,
    ) -> PyResult<String> {
        let mut append_querystring = querystring.to_string();
        if querystring.trim().ends_with("}") {
            if !args.is_empty() {
                return Err(PyValueError::new_err("You can't supply positional arguments if the main query already contains subqueries (use either one or the other)"));
            }
        } else if !args.is_empty() {
            for (i, arg) in args.iter().enumerate() {
                let subquerystring: &str = arg.extract()?;
                if i == 0 {
                    append_querystring += " { ";
                } else {
                    append_querystring += " | ";
                }
                append_querystring += subquerystring;
            }
            append_querystring += " }";
        }

        let query: Query = append_querystring.as_str().try_into().map_err(|err| {
            PyStamError::new_err(format!(
                "{} -- full query was: {}",
                err,
                append_querystring.as_str()
            ))
        })?;
        let legend = get_bool(kwargs, "legend", true);
        let titles = get_bool(kwargs, "titles", true);
        let interactive = get_bool(kwargs, "interactive", true);
        let selectionvar = get_opt_string(kwargs, "use", None);
        let autocollapse = get_bool(kwargs, "autocollapse", false);
        let header = get_opt_string(kwargs, "header", None);
        let footer = get_opt_string(kwargs, "footer", None);
        let format = get_opt_string(kwargs, "format", Some("html"));
        match format.as_deref() {
            Some("html") => self
                .map_store(|store| {
                    let mut writer = HtmlWriter::new(store, query, selectionvar.as_deref())
                        .map_err(|e| StamError::QuerySyntaxError(e, ""))?
                        .with_legend(legend)
                        .with_titles(titles)
                        .with_interactive(interactive)
                        .with_autocollapse(autocollapse);
                    if header.is_some() {
                        writer = writer.with_header(header.as_deref());
                    };
                    if footer.is_some() {
                        writer = writer.with_footer(footer.as_deref());
                    };
                    Ok(format!("{}", writer))
                })
                .map_err(|err| PyStamError::new_err(format!("{}", err))),
            Some("ansi") => self
                .map_store(|store| {
                    let writer = AnsiWriter::new(store, query, selectionvar.as_deref())
                        .map_err(|e| StamError::QuerySyntaxError(e, ""))?
                        .with_legend(legend)
                        .with_titles(titles);
                    let mut out: Vec<u8> = Vec::new();
                    writer.write(&mut out)?;
                    String::from_utf8(out)
                        .map_err(|_| StamError::OtherError("Failed to turn buffer to string"))
                })
                .map_err(|err| PyStamError::new_err(format!("{}", err))),
            _ => Err(PyValueError::new_err(
                "Invalid format to view(): set 'html' or 'ansi'",
            )),
        }
    }

    #[pyo3(signature = (querystrings, retain))]
    fn split<'py>(&mut self, querystrings: Vec<&str>, retain: bool) -> PyResult<()> {
        let mode = if retain {
            SplitMode::Retain
        } else {
            SplitMode::Delete
        };
        let mut queries: Vec<Query> = Vec::new();
        for querystring in querystrings {
            let query: Query = querystring
                .try_into()
                .map_err(|err| PyStamError::new_err(format!("{}", err)))?;
            queries.push(query);
        }
        self.map_store_mut(|store| Ok(split(store, queries, mode, false)))
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn align_texts(
        &mut self,
        args: Vec<(PyTextSelection, PyTextSelection)>,
        kwargs: Option<&PyDict>,
    ) -> PyResult<Vec<Vec<PyAnnotation>>> {
        let alignmentconfig = if let Some(kwargs) = kwargs {
            get_alignmentconfig(kwargs)?
        } else {
            AlignmentConfig::default()
        };
        let results: Vec<Vec<AnnotationBuilder<'static>>> = args
            .into_par_iter()
            .filter_map(move |(textsel1, textsel2)| {
                match textsel1.map(|textselection| {
                    let store = textselection.rootstore();
                    let otherresource = store.resource(textsel2.resource_handle).or_fail()?;
                    let other = otherresource.textselection(&textsel2.offset().offset)?;
                    align_texts(&textselection, &other, &alignmentconfig)
                }) {
                    Ok(buildtranspositions) => Some(buildtranspositions),
                    Err(e) => {
                        eprintln!("[STAM align_texts] {}", e);
                        None
                    }
                }
            })
            .collect();
        let storepointer = self.store.clone();
        self.map_store_mut(move |store| {
            results
                .into_iter()
                .map(|buildtranspositions| {
                    let mut transpositions = Vec::with_capacity(buildtranspositions.len());
                    for builder in buildtranspositions {
                        let annotation_handle = store.annotate(builder)?;
                        let transposition_key = store.key(
                            "https://w3id.org/stam/extensions/stam-transpose/",
                            "Transposition",
                        );
                        let translation_key = store.key(
                            "https://w3id.org/stam/extensions/stam-translate/",
                            "Translation",
                        );
                        if transposition_key.is_some() || translation_key.is_some() {
                            let annotation = store.annotation(annotation_handle).or_fail()?;
                            if annotation.keys().any(|key| {
                                transposition_key
                                    .as_ref()
                                    .map(|k| &key == k)
                                    .unwrap_or(false)
                                    || translation_key.as_ref().map(|k| &key == k).unwrap_or(false)
                            }) {
                                transpositions.push(annotation_handle);
                            }
                        }
                    }
                    Ok(transpositions
                        .into_iter()
                        .map(|handle| PyAnnotation::new(handle, storepointer.clone()))
                        .collect::<Vec<_>>())
                })
                .collect()
        })
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

    pub(crate) fn map_with_query<T, F>(
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

#[pyclass(name = "SubStoreIter")]
struct PySubStoreIter {
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
    pub(crate) index: usize,
}

#[pymethods]
impl PySubStoreIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(mut pyself: PyRefMut<'_, Self>) -> Option<PyAnnotationSubStore> {
        pyself.index += 1; //increment first (prevent exclusive mutability issues)
        let result = pyself.map(|store| {
            let handle: AnnotationSubStoreHandle = AnnotationSubStoreHandle::new(pyself.index - 1);
            if let Ok(substore) = store.get(handle) {
                //index is one ahead, prevents exclusive lock issues
                let handle = substore.handle().expect("annotation must have an ID");
                Some(PyAnnotationSubStore {
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

impl PySubStoreIter {
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
