use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotation;
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
            Ok(PyTextSelection {
                textselection: if textselection.is_borrowed() {
                    textselection.unwrap().clone()
                } else {
                    textselection.unwrap_owned()
                },
                resource_handle: self.resource_handle,
                store: self.store.clone(),
            })
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
                    list.append(
                        PyTextSelection {
                            textselection: if textselection.is_borrowed() {
                                textselection.unwrap().clone()
                            } else {
                                textselection.unwrap_owned()
                            },
                            resource_handle: self.resource_handle,
                            store: self.store.clone(),
                        }
                        .into_py(py)
                        .into_ref(py),
                    )
                    .ok();
                    if Some(i + 1) == limit {
                        break;
                    }
                }
                Ok(())
            } else {
                for (i, textselection) in textselection.find_text(fragment).enumerate() {
                    list.append(
                        PyTextSelection {
                            textselection: if textselection.is_borrowed() {
                                textselection.unwrap().clone()
                            } else {
                                textselection.unwrap_owned()
                            },
                            resource_handle: self.resource_handle,
                            store: self.store.clone(),
                        }
                        .into_py(py)
                        .into_ref(py),
                    )
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
                    list.append(
                        PyTextSelection {
                            textselection: if textselection.is_borrowed() {
                                textselection.unwrap().clone()
                            } else {
                                textselection.unwrap_owned()
                            },
                            resource_handle: self.resource_handle,
                            store: self.store.clone(),
                        }
                        .into_py(py)
                        .into_ref(py),
                    )
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
                list.append(
                    PyTextSelection {
                        textselection: if textselection.is_borrowed() {
                            textselection.unwrap().clone()
                        } else {
                            textselection.unwrap_owned()
                        },
                        resource_handle: self.resource_handle,
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
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
                .map(|textselection| PyTextSelection {
                    textselection: if textselection.is_borrowed() {
                        textselection.unwrap().clone()
                    } else {
                        textselection.unwrap_owned()
                    },
                    resource_handle: self.resource_handle,
                    store: self.store.clone(),
                })
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
        self.map_with_store(|textselection, store| {
            for (i, annotation) in textselection
                .annotations(store)
                .into_iter()
                .flatten()
                .enumerate()
            {
                list.append(
                    PyAnnotation {
                        handle: annotation.handle().expect("annotation must have a handle"),
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
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
    fn find_textselections(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            for (i, foundtextselection) in textselection
                .find_textselections(operator.operator)
                .enumerate()
            {
                list.append(
                    PyTextSelection {
                        textselection: if foundtextselection.is_borrowed() {
                            foundtextselection.unwrap().clone()
                        } else {
                            foundtextselection.unwrap_owned()
                        },
                        resource_handle: self.resource_handle,
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
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
        self.map_with_store(|textselection, store| Ok(textselection.annotations_len(store)))
            .unwrap()
    }
    /// Applies a `TextSelectionOperator` to find all other annotations whose text selections
    /// are in a specific relation with the current one. Returns all matching Annotations in a list
    #[pyo3(signature = (operator,limit=None))]
    fn find_annotations(
        &self,
        operator: PyTextSelectionOperator,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map_with_store(|textselection, store| {
            for (i, annotation) in textselection
                .find_annotations(operator.operator, store)
                .enumerate()
            {
                list.append(
                    PyAnnotation {
                        handle: annotation.handle().expect("annotation must have a handle"),
                        store: self.store.clone(),
                    }
                    .into_py(py)
                    .into_ref(py),
                )
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
                .relative_offset(&container.textselection)
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
                .relative_end(&container.textselection)
                .ok_or(StamError::OtherError(
                "TextSelection end does not overlap with specified container, cursor out of bounds",
            ))?;
            Ok(cursor)
        })
    }

    fn test(&self, operator: PyTextSelectionOperator, other: &PyTextSelection) -> PyResult<bool> {
        self.map(|textselection| Ok(textselection.test(&operator.operator, &other.textselection)))
    }
}

impl PyTextSelection {
    fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<TextSelection>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource = store
                .resource(&self.resource_handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            let textselection = resource
                .wrap_owned(self.textselection)
                .expect("wrap of textselection must succeed");
            f(textselection).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }

    fn map_with_store<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<TextSelection>, &AnnotationStore) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource = store
                .resource(&self.resource_handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            let textselection = resource
                .wrap_owned(self.textselection)
                .expect("wrap of textselection must succeed");
            f(textselection, &store).map_err(|err| PyStamError::new_err(format!("{}", err)))
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
            if let Some(resource) = store.resource(&self.resource_handle.into()) {
                loop {
                    if let Some(position) = self.positions.get(self.index) {
                        if let Some(positionitem) = resource.position(*position) {
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
                                    resource.get(&handle.into());
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
    /// Create an operator to test if one textselection(sets) precedes another
    /// Each TextSelections in A precedes (comes before) a textselection in B
    /// If modifier `all` is set: All TextSelections in A precede (come before) all textselections in B. There is no overlap (cf. textfabric's `<<`)
    fn precedes(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Precedes {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if one textselection(sets) succeeds another
    /// Each TextSeleciton In A succeeds (comes after) a textselection in B
    /// If modifier `all` is set: All TextSelections in A succeed (come after) all textselections in B. There is no overlap (cf. textfabric's `>>`)
    fn succeeds(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::Succeeds {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if one textselection(sets) is to the immediate left of another
    /// Each TextSelection in A is ends where at least one TextSelection in B begins.
    /// If modifier `all` is set: The rightmost TextSelections in A end where the leftmost TextSelection in B begins  (cf. textfabric's `<:`)
    fn leftadjacent(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::LeftAdjacent {
                all: all.unwrap_or(false),
                negate: negate.unwrap_or(false),
            },
        })
    }

    #[staticmethod]
    /// Create an operator to test if one textselection(sets) is to the immediate right of another
    /// Each TextSelection in A is begis where at least one TextSelection in A ends.
    /// If modifier `all` is set: The leftmost TextSelection in A starts where the rightmost TextSelection in B ends  (cf. textfabric's `:>`)
    fn rightadjacent(all: Option<bool>, negate: Option<bool>) -> PyResult<Self> {
        Ok(Self {
            operator: TextSelectionOperator::RightAdjacent {
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
