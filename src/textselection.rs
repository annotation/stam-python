use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::hash::{Hash, Hasher};
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotation;
use crate::annotationdata::{data_request_parser, PyAnnotationData};
use crate::error::PyStamError;
use crate::resources::{PyOffset, PyTextResource};
use stam::*;

#[pyclass(name = "TextSelection")]
#[derive(Clone)]
pub(crate) struct PyTextSelection {
    pub(crate) textselection: TextSelection,
    pub(crate) resource_handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

impl PyTextSelection {
    pub(crate) fn new(
        textselection: TextSelection,
        resource: TextResourceHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyTextSelection {
        PyTextSelection {
            textselection,
            resource_handle: resource,
            store: store.clone(),
        }
    }

    pub(crate) fn new_py<'py>(
        textselection: TextSelection,
        resource: TextResourceHandle,
        store: &Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::new(textselection, resource, store)
            .into_py(py)
            .into_ref(py)
    }

    pub(crate) fn from_result(
        result: ResultTextSelection<'_>,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> Self {
        let resource_handle = result.resource().handle();
        PyTextSelection {
            textselection: match result {
                ResultTextSelection::Bound(item) => item.as_ref().clone(),
                ResultTextSelection::Unbound(_, _, item) => item,
            },
            resource_handle,
            store: store.clone(),
        }
    }

    pub(crate) fn from_result_to_py<'py>(
        result: ResultTextSelection<'_>,
        store: &Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::from_result(result, store).into_py(py).into_ref(py)
    }
}

#[pymethods]
impl PyTextSelection {
    /// Resolves a text selection to the actual underlying text
    fn text<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|textselection| Ok(PyString::new(py, textselection.text())))
    }

    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.text(py)
    }

    /// Returns a string (by value, aka copy) of a slice of the text
    fn __getitem__<'py>(&self, slice: &PySlice, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|textselection| {
            let slice = slice
                .indices(textselection.textlen().try_into().unwrap())
                .expect("expected valid slice");
            Ok(PyString::new(
                py,
                textselection
                    .text_by_offset(&Offset::simple(slice.start as usize, slice.stop as usize))?,
            ))
        })
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.resource_handle == other.resource_handle
                && self.textselection == other.textselection)
                .into_py(py),
            CompareOp::Ne => (self.resource_handle != other.resource_handle
                || self.textselection != other.textselection)
                .into_py(py),
            CompareOp::Lt => (self.textselection < other.textselection).into_py(py),
            CompareOp::Le => (self.textselection <= other.textselection).into_py(py),
            CompareOp::Gt => (self.textselection > other.textselection).into_py(py),
            CompareOp::Ge => (self.textselection >= other.textselection).into_py(py),
        }
    }

    fn __hash__(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let h = (self.resource_handle.as_usize(), self.textselection);
        h.hash(&mut hasher);
        hasher.finish()
    }

    /// Returns the resource this textselections points at
    fn resource(&self) -> PyResult<PyTextResource> {
        Ok(PyTextResource {
            handle: self.resource_handle,
            store: self.store.clone(),
        })
    }

    /// Return the absolute begin position in unicode points
    fn begin(&self) -> usize {
        self.textselection.begin()
    }

    /// Return the absolute end position in unicode points (non-inclusive)
    fn end(&self) -> usize {
        self.textselection.end()
    }

    /// Returns the length of the text selection in unicode points (same as `len(self.text())` but more performant)
    fn textlen(&self) -> PyResult<usize> {
        self.map(|textselection| Ok(textselection.textlen()))
    }

    /// Returns another TextSelection instance covering the specified offset *inside* the current
    /// textselection. The offset is specified relative to the current one.
    fn textselection(&self, offset: &PyOffset) -> PyResult<PyTextSelection> {
        self.map(|textselection| {
            let textselection = textselection.textselection(&offset.offset)?;
            Ok(PyTextSelection::from_result(textselection, &self.store))
        })
    }

    /// Searches for the text fragment and returns a list of [`TextSelection`] instances with all matches (or up to the specified limit)
    fn find_text(
        &self,
        fragment: &str,
        limit: Option<usize>,
        case_sensitive: Option<bool>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            if case_sensitive == Some(false) {
                for (i, textselection) in textselection.find_text_nocase(fragment).enumerate() {
                    list.append(PyTextSelection::from_result_to_py(
                        textselection,
                        &self.store,
                        py,
                    ))
                    .ok();
                    if Some(i + 1) == limit {
                        break;
                    }
                }
                Ok(())
            } else {
                for (i, textselection) in textselection.find_text(fragment).enumerate() {
                    list.append(PyTextSelection::from_result_to_py(
                        textselection,
                        &self.store,
                        py,
                    ))
                    .ok();
                    if Some(i + 1) == limit {
                        break;
                    }
                }
                Ok(())
            }
        })
        .ok();
        list.into()
    }

    fn find_text_sequence(
        &self,
        fragments: Vec<&str>,
        case_sensitive: Option<bool>,
        allow_skip_whitespace: Option<bool>,
        allow_skip_punctuation: Option<bool>,
        allow_skip_numeric: Option<bool>,
        allow_skip_alphabetic: Option<bool>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            let results = textselection.find_text_sequence(
                &fragments,
                |c| {
                    if (allow_skip_whitespace == Some(false) && c.is_whitespace())
                        || (allow_skip_punctuation == Some(false) && c.is_ascii_punctuation())
                        || (allow_skip_numeric == Some(false) && c.is_numeric())
                        || (allow_skip_alphabetic == Some(false) && c.is_alphabetic())
                    {
                        false
                    } else {
                        true
                    }
                },
                case_sensitive.unwrap_or(true),
            );
            if let Some(results) = results {
                for textselection in results {
                    list.append(PyTextSelection::from_result_to_py(
                        textselection,
                        &self.store,
                        py,
                    ))
                    .ok();
                }
            }
            Ok(())
        })
        .ok();
        list.into()
    }

    /// Returns a tuple of [`TextSelection`] instances that split the text according to the specified delimiter.
    /// You can set `limit` to the max number of elements you want to return.
    fn split_text(&self, delimiter: &str, limit: Option<usize>, py: Python) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            for (i, textselection) in textselection.split_text(delimiter).enumerate() {
                list.append(PyTextSelection::from_result_to_py(
                    textselection,
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

    /// Converts a unicode character position to a UTF-8 byte position
    fn utf8byte(&self, abscursor: usize) -> PyResult<usize> {
        self.map(|textselection| textselection.utf8byte(abscursor))
    }

    /// Converts a UTF-8 byte position into a unicode position
    fn utf8byte_to_charpos(&self, bytecursor: usize) -> PyResult<usize> {
        self.map(|textselection| textselection.utf8byte_to_charpos(bytecursor))
    }

    /// Resolves a begin-aligned cursor to an absolute cursor (i.e. relative to the TextResource).
    fn absolute_cursor(&self, cursor: usize) -> PyResult<usize> {
        self.map(|textselection| Ok(textselection.absolute_cursor(cursor)))
    }

    /// Resolves a relative offset (relative to another TextSelection) to an absolute one (in terms of to the underlying TextResource)
    fn absolute_offset(&self, offset: &PyOffset) -> PyResult<PyOffset> {
        self.map(|textselection| {
            let offset = textselection.absolute_offset(&offset.offset)?;
            Ok(PyOffset { offset })
        })
    }

    /// Converts the TextSelection to an :obj:`Offset` instance
    fn offset(&self) -> PyOffset {
        PyOffset {
            offset: Offset::simple(self.begin(), self.end()),
        }
    }

    /// Trims all occurrences of any character in `chars` from both the beginning and end of the text,
    /// returning a smaller TextSelection. No text is modified.
    fn strip_text(&self, chars: &str) -> PyResult<PyTextSelection> {
        let chars: Vec<char> = chars.chars().collect();
        self.map(|textselection| {
            textselection
                .trim_text(&chars)
                .map(|textselection| PyTextSelection::from_result(textselection, &self.store))
        })
    }

    /// Converts an end-aligned cursor to a begin-aligned cursor, resolving all relative end-aligned positions
    /// The parameter value must be 0 or negative.
    fn beginaligned_cursor(&self, endalignedcursor: isize) -> PyResult<usize> {
        self.map(|textselection| {
            textselection.beginaligned_cursor(&Cursor::EndAligned(endalignedcursor))
        })
    }

    /// Returns a list of all annotations (:obj:`Annotation`) that reference this TextSelection, if any.
    #[pyo3(signature = (limit=None))]
    fn annotations(&self, limit: Option<usize>, py: Python) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            for (i, annotation) in textselection
                .annotations()
                .into_iter()
                .flatten()
                .enumerate()
            {
                list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
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

    /// Applies a `TextSelectionOperator` to find all other text selections that
    /// are in a specific relation with the current one. Returns all matching TextSelections in a list
    ///
    /// If you are interested in the annotations associated with the found text selections, then use `find_annotations()` instead.
    #[pyo3(signature = (operator,limit=None))]
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            for (i, foundtextselection) in textselection.related_text(operator.operator).enumerate()
            {
                list.append(PyTextSelection::from_result_to_py(
                    foundtextselection,
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

    fn annotations_len(&self) -> usize {
        self.map(|textselection| Ok(textselection.annotations_len()))
            .unwrap()
    }

    /// Applies a `TextSelectionOperator` to find all other annotations whose text selections
    /// are in a specific relation with the current one. Returns all matching Annotations in a list
    #[pyo3(signature = (operator,limit=None))]
    fn annotations_by_related_text(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            for (i, annotation) in textselection
                .annotations_by_related_text(operator.operator)
                .into_iter()
                .enumerate()
            {
                list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
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

    #[pyo3(signature = (operator, **kwargs))]
    fn annotations_by_related_text_and_data<'py>(
        &self,
        operator: PyTextSelectionOperator,
        kwargs: Option<&'py PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        self.map(|textselection| {
            let list: &PyList = PyList::empty(py);
            match data_request_parser(kwargs, textselection.rootstore(), None, None) {
                Ok((sethandle, keyhandle, op)) => {
                    for annotation in textselection
                        .annotations_by_related_text_and_data(
                            operator.operator,
                            sethandle,
                            keyhandle,
                            &op,
                        )
                        .into_iter()
                    {
                        list.append(PyAnnotation::new_py(annotation.handle(), &self.store, py))
                            .ok();
                    }
                    Ok(list.into())
                }
                Err(StamError::IdNotFoundError(..)) => {
                    //we don't raise this error but just return an empty list
                    Ok(list)
                }
                Err(e) => Err(e),
            }
        })
    }

    /// Returns the offset of this text selection relative to another in which it is *embedded*.
    /// Raises a `StamError` exception if they are not embedded, or not belonging to the same resource.
    fn relative_offset(&self, container: &PyTextSelection) -> PyResult<PyOffset> {
        if self.resource_handle != container.resource_handle {
            return Err(PyStamError::new_err(
                "TextSelection and Container do not belong to the same resource",
            ));
        }
        self.map(|textselection| {
            let offset = textselection
                .inner()
                .relative_offset(&container.textselection, OffsetMode::default())
                .ok_or(StamError::OtherError(
                    "TextSelection is not embedded in specified container, cursor out of bounds",
                ))?;
            Ok(PyOffset { offset })
        })
    }

    /// Returns the begin cursor of this text selection relative to another with which the begin *overlaps*.
    /// Raises a `StamError` exception if they do not overlap, or not belonging to the same resource.
    fn relative_begin(&self, container: &PyTextSelection) -> PyResult<usize> {
        if self.resource_handle != container.resource_handle {
            return Err(PyStamError::new_err(
                "TextSelection and Container do not belong to the same resource",
            ));
        }
        self.map(|textselection| {
            let cursor = textselection
                .inner()
                .relative_begin(&container.textselection)
                .ok_or(StamError::OtherError(
                    "TextSelection begin does not overlap with specified container, cursor out of bounds",
                ))?;
            Ok(cursor )
        })
    }

    /// Returns the begin cursor of this text selection relative to another with which the begin *overlaps*.
    /// Raises a `StamError` exception if they do not overlap, or not belonging to the same resource.
    fn relative_end(&self, container: &PyTextSelection) -> PyResult<usize> {
        if self.resource_handle != container.resource_handle {
            return Err(PyStamError::new_err(
                "TextSelection and Container do not belong to the same resource",
            ));
        }
        self.map(|textselection| {
            let cursor = textselection
                .inner()
                .relative_end(&container.textselection)
                .ok_or(StamError::OtherError(
                "TextSelection end does not overlap with specified container, cursor out of bounds",
            ))?;
            Ok(cursor)
        })
    }

    fn test(&self, operator: PyTextSelectionOperator, other: &PyTextSelection) -> PyResult<bool> {
        self.map(|textselection| {
            Ok(textselection
                .inner()
                .test(&operator.operator, &other.textselection))
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn find_data_about<'py>(
        &self,
        kwargs: Option<&PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        self.map(|textselection| {
            let list: &PyList = PyList::empty(py);
            match data_request_parser(kwargs, textselection.rootstore(), None, None) {
                Ok((sethandle, keyhandle, op)) => {
                    for (annotationdata, annotation) in textselection
                        .find_data_about(sethandle, keyhandle, &op)
                        .into_iter()
                        .flatten()
                    {
                        list.append((
                            PyAnnotationData::new_py(
                                annotationdata.handle(),
                                annotationdata.set().handle(),
                                &self.store,
                                py,
                            ),
                            PyAnnotation::new_py(annotation.handle(), &self.store, py),
                        ))
                        .ok();
                    }
                    Ok(list.into())
                }
                Err(StamError::IdNotFoundError(..)) => {
                    //we don't raise this error but just return an empty list
                    Ok(list)
                }
                Err(e) => Err(e),
            }
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_data_about<'py>(&self, kwargs: Option<&'py PyDict>) -> PyResult<bool> {
        self.map(|textselection| {
            match data_request_parser(kwargs, textselection.rootstore(), None, None) {
                Ok((sethandle, keyhandle, op)) => {
                    Ok(textselection.test_data_about(sethandle, keyhandle, &op))
                }
                Err(StamError::IdNotFoundError(..)) => Ok(false),
                Err(e) => Err(e),
            }
        })
    }

    #[pyo3(signature = (operator,**kwargs))]
    fn related_text_with_data<'py>(
        &self,
        operator: PyTextSelectionOperator,
        kwargs: Option<&'py PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        self.map(|textselection| {
            let list: &PyList = PyList::empty(py);
            match data_request_parser(kwargs, textselection.rootstore(), None, None) {
                Ok((sethandle, keyhandle, op)) => {
                    for (textselection, data_and_annotations) in textselection
                        .related_text_with_data(operator.operator, sethandle, keyhandle, &op)
                        .into_iter()
                        .flatten()
                    {
                        let innerlist: &PyList = PyList::empty(py);
                        for (annotationdata, annotation) in data_and_annotations {
                            innerlist
                                .append((
                                    PyAnnotationData::new_py(
                                        annotationdata.handle(),
                                        annotationdata.set().handle(),
                                        &self.store,
                                        py,
                                    ),
                                    PyAnnotation::new_py(annotation.handle(), &self.store, py),
                                ))
                                .ok();
                        }
                        list.append((
                            PyTextSelection::from_result_to_py(textselection, &self.store, py),
                            innerlist,
                        ))
                        .ok();
                    }
                    Ok(list.into())
                }
                Err(StamError::IdNotFoundError(..)) => {
                    //we don't raise this error but just return an empty list
                    Ok(list)
                }
                Err(e) => Err(e),
            }
        })
    }

    #[pyo3(signature = (operator,**kwargs))]
    fn related_text_test_data<'py>(
        &self,
        operator: PyTextSelectionOperator,
        kwargs: Option<&'py PyDict>,
        py: Python<'py>,
    ) -> PyResult<&'py PyList> {
        self.map(|textselection| {
            let list: &PyList = PyList::empty(py);
            match data_request_parser(kwargs, textselection.rootstore(), None, None) {
                Ok((sethandle, keyhandle, op)) => {
                    for textselection in textselection
                        .related_text_test_data(operator.operator, sethandle, keyhandle, &op)
                        .into_iter()
                        .flatten()
                    {
                        list.append(PyTextSelection::from_result_to_py(
                            textselection,
                            &self.store,
                            py,
                        ))
                        .ok();
                    }
                    Ok(list.into())
                }
                Err(StamError::IdNotFoundError(..)) => {
                    //we don't raise this error but just return an empty list
                    Ok(list)
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl PyTextSelection {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(ResultTextSelection) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource = store
                .resource(self.resource_handle)
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            let textselection = resource
                .textselection(&self.textselection.into())
                .map_err(|err| PyStamError::new_err(format!("{}", err)))?;
            f(textselection).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

impl From<PyTextSelection> for TextSelection {
    fn from(other: PyTextSelection) -> Self {
        other.textselection
    }
}

#[pyclass(name = "TextSelectionIter")]
// this isn't based off the TextSelectionIter in the Rust library because that one holds a
// reference which we can't in Python. This one will have somewhat more overhead and only supports
// forward iteration
pub(crate) struct PyTextSelectionIter {
    pub(crate) positions: Vec<usize>,
    pub(crate) index: usize,
    pub(crate) subindex: usize,
    pub(crate) resource_handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyTextSelectionIter {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__(&mut self) -> Option<PyTextSelection> {
        self.next()
    }
}

impl Iterator for PyTextSelectionIter {
    type Item = PyTextSelection;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(store) = self.store.read() {
            if let Some(resource) = store.resource(self.resource_handle) {
                loop {
                    if let Some(position) = self.positions.get(self.index) {
                        if let Some(positionitem) = resource.as_ref().position(*position) {
                            if let Some((_, handle)) =
                                positionitem.iter_begin2end().nth(self.subindex)
                            {
                                //increment for next run
                                self.subindex += 1;
                                if self.subindex >= positionitem.len_begin2end() {
                                    self.index += 1;
                                    self.subindex = 0;
                                }

                                let textselection: Result<&TextSelection, _> =
                                    resource.as_ref().get(*handle);
                                if let Ok(textselection) = textselection {
                                    //forward iteration only
                                    return Some(PyTextSelection {
                                        textselection: textselection.clone(),
                                        resource_handle: self.resource_handle,
                                        store: self.store.clone(),
                                    });
                                }
                            }
                        }
                        self.index += 1;
                        self.subindex = 0;
                        //rely on loop to 'recurse'
                    } else {
                        break;
                    }
                }
            }
        }
        None
    }
}

#[pyclass(dict, module = "stam", name = "TextSelectionOperator")]
#[derive(Clone)]
pub(crate) struct PyTextSelectionOperator {
    pub(crate) operator: TextSelectionOperator,
}

#[pymethods]
impl PyTextSelectionOperator {
    #[staticmethod]
    /// Create an operator to test if two textselection(sets) occupy cover the exact same TextSelections, and all are covered (cf. textfabric's `==`), commutative, transitive
    fn equals(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Equals {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if two textselection(sets) overlap.
    /// Each TextSelection in A overlaps with a TextSelection in B (cf. textfabric's `&&`), commutative
    /// If modifier `all` is set: Each TextSelection in A overlaps with all TextSelection in B (cf. textfabric's `&&`), commutative
    fn overlaps(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Overlaps {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if two textselection(sets) are embedded.
    /// All TextSelections in B are embedded by a TextSelection in A (cf. textfabric's `[[`)
    /// If modifier `all` is set: All TextSelections in B are embedded by all TextSelection in A (cf. textfabric's `[[`)
    fn embeds(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Embeds {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if two textselection(sets) are embedded.
    /// All TextSelections in B are embedded by a TextSelection in A (cf. textfabric's `[[`)
    /// If modifier `all` is set: All TextSelections in B are embedded by all TextSelection in A (cf. textfabric's `[[`)
    fn embedded(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Embedded {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    fn before(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Before {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    fn after(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::After {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    fn precedes(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Precedes {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    fn succeeds(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Succeeds {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if two textselection(sets) have the same begin position
    /// Each TextSelection in A starts where a TextSelection in B starts
    /// If modifier `all` is set: The leftmost TextSelection in A starts where the leftmost TextSelection in B start  (cf. textfabric's `=:`)
    fn samebegin(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::SameBegin {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if two textselection(sets) have the same end position
    /// Each TextSelection in A ends where a TextSelection in B ends
    /// If modifier `all` is set: The rightmost TextSelection in A ends where the rights TextSelection in B ends  (cf. textfabric's `:=`)
    fn sameend(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::SameEnd {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }
}
