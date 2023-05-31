use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotation;
use crate::error::PyStamError;
use crate::selector::PySelector;
use crate::textselection::{PyTextSelection, PyTextSelectionIter, PyTextSelectionOperator};
use stam::*;

#[pyclass(dict, module = "stam", name = "TextResource")]
/// This holds the textual resource to be annotated. It holds the full text in memory.
///
/// The text *SHOULD* be in
/// [Unicode Normalization Form C (NFC)](https://www.unicode.org/reports/tr15/) but
/// *MAY* be in another unicode normalization forms.
pub(crate) struct PyTextResource {
    pub(crate) handle: TextResourceHandle,
    pub(crate) store: Arc<RwLock<AnnotationStore>>,
}

#[pymethods]
impl PyTextResource {
    /// Returns the public ID (by value, aka a copy)
    /// Don't use this for ID comparisons, use has_id() instead
    fn id(&self) -> PyResult<Option<String>> {
        self.map(|res| Ok(res.id().map(|x| x.to_owned())))
    }

    /// Tests the ID of the dataset
    fn has_id(&self, other: &str) -> PyResult<bool> {
        self.map(|res| Ok(res.id() == Some(other)))
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.handle == other.handle).into_py(py),
            CompareOp::Ne => (self.handle != other.handle).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    /// Returns the full text of the resource (by value, aka a copy)
    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.text(py)
    }

    /// Returns a string (by value, aka copy) of a slice of the text
    fn __getitem__<'py>(&self, slice: &PySlice, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|resource| {
            let slice = slice
                .indices(resource.textlen().try_into().unwrap())
                .expect("expected valid slice");
            Ok(PyString::new(
                py,
                resource
                    .text_by_offset(&Offset::simple(slice.start as usize, slice.stop as usize))?,
            ))
        })
    }

    /// 'Returns the full text of the resource (by value, aka a copy)
    fn text<'py>(&self, py: Python<'py>) -> PyResult<&'py PyString> {
        self.map(|resource| Ok(PyString::new(py, resource.text())))
    }

    /// Returns the length of the resources's text in unicode points (same as `len(self.text())` but more performant)
    fn textlen(&self) -> PyResult<usize> {
        self.map(|res| Ok(res.textlen()))
    }

    /// Returns a TextSelection instance covering the specified offset
    fn textselection(&self, offset: &PyOffset) -> PyResult<PyTextSelection> {
        self.map(|res| {
            let textselection = res.textselection(&offset.offset)?;
            Ok(PyTextSelection {
                textselection: if textselection.is_borrowed() {
                    textselection.unwrap().clone()
                } else {
                    textselection.unwrap_owned()
                },
                resource_handle: self.handle,
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
        self.map(|res| {
            if case_sensitive == Some(false) {
                for (i, textselection) in res.find_text_nocase(fragment).enumerate() {
                    list.append(
                        PyTextSelection {
                            textselection: if textselection.is_borrowed() {
                                textselection.unwrap().clone()
                            } else {
                                textselection.unwrap_owned()
                            },
                            resource_handle: self.handle,
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
                for (i, textselection) in res.find_text(fragment).enumerate() {
                    list.append(
                        PyTextSelection {
                            textselection: if textselection.is_borrowed() {
                                textselection.unwrap().clone()
                            } else {
                                textselection.unwrap_owned()
                            },
                            resource_handle: self.handle,
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

    /// Searches the text using one or more regular expressions, returns an list of dictionaries with items:
    /// `{ "textselections": [TextSelection], "expression_index": int, "capturegroups": [int] }
    ///
    /// Passing multiple regular expressions at once is more efficient than calling this function anew for each one.
    /// If capture groups are used in the regular expression, only those parts will be returned (the rest is context). If none are used,
    /// the entire expression is returned. The regular expressions are passed as strings and
    //// must follow this syntax: https://docs.rs/regex/latest/regex/#syntax , which may differ slightly from Python's regular expressions!
    ///
    /// The `allow_overlap` parameter determines if the matching expressions are allowed to
    /// overlap. It you are doing some form of tokenisation, you also likely want this set to
    /// false. All of this only matters if you supply multiple regular expressions.
    ///
    /// Results are returned in the exact order they are found in the text
    fn find_text_regex(
        &self,
        expressions: &PyList,
        allow_overlap: Option<bool>,
        limit: Option<usize>,
        py: Python,
    ) -> PyResult<Py<PyList>> {
        //MAYBE TODO: there's room for performance improvement here probably
        let mut regexps: Vec<Regex> = Vec::new();
        for expression in expressions.iter() {
            //MAYBE TODO: allow precompiled regexps
            let expression: &str = expression.extract()?;
            regexps.push(Regex::new(expression).map_err(|e| {
                PyValueError::new_err(format!(
                    "Unable to parse regular expression: {} - {}",
                    expression, e
                ))
            })?);
        }
        let list: &PyList = PyList::empty(py);
        self.map(|res| {
            for (i, regexmatch) in res
                .find_text_regex(&regexps, None, allow_overlap.unwrap_or(false))?
                .enumerate()
            {
                let textselections: &PyList = PyList::empty(py);
                for textselection in regexmatch.textselections() {
                    textselections
                        .append(
                            PyTextSelection {
                                textselection: if textselection.is_borrowed() {
                                    textselection.unwrap().clone()
                                } else {
                                    textselection.clone().unwrap_owned()
                                },
                                resource_handle: self.handle,
                                store: self.store.clone(),
                            }
                            .into_py(py)
                            .into_ref(py),
                        )
                        .ok();
                }
                let dict: &PyDict = PyDict::new(py);
                dict.set_item("textselections", textselections).unwrap();
                dict.set_item("expression_index", regexmatch.expression_index())
                    .unwrap();
                dict.set_item("capturegroups", Some(regexmatch.capturegroups()))
                    .unwrap();
                list.append(dict).ok();
                if Some(i + 1) == limit {
                    break;
                }
            }
            Ok(())
        })
        .ok();
        Ok(list.into())
    }

    /// Returns a list of [`TextSelection`] instances that split the text according to the specified delimiter.
    /// You can set `limit` to the max number of elements you want to return.
    fn split_text(&self, delimiter: &str, limit: Option<usize>, py: Python) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|res| {
            for (i, textselection) in res.split_text(delimiter).enumerate() {
                list.append(
                    PyTextSelection {
                        textselection: if textselection.is_borrowed() {
                            textselection.unwrap().clone()
                        } else {
                            textselection.unwrap_owned()
                        },
                        resource_handle: self.handle,
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

    /// Trims all occurrences of any character in `chars` from both the beginning and end of the text,
    /// returning a smaller TextSelection. No text is modified.
    fn strip_text(&self, chars: &str) -> PyResult<PyTextSelection> {
        let chars: Vec<char> = chars.chars().collect();
        self.map(|res| {
            res.trim_text(&chars).map(|textselection| PyTextSelection {
                textselection: if textselection.is_borrowed() {
                    textselection.unwrap().clone()
                } else {
                    textselection.unwrap_owned()
                },
                resource_handle: self.handle,
                store: self.store.clone(),
            })
        })
    }

    /// Returns a Selector (ResourceSelector) pointing to this TextResource
    fn selector(&self) -> PyResult<PySelector> {
        self.map(|res| res.selector().map(|sel| sel.into()))
    }

    // Iterates over all known textselections in this resource, shortcut for __iter__()
    fn textselections(&self) -> PyTextSelectionIter {
        self.__iter__()
    }

    // Iterates over all known textselections in this resource, in sorted order
    fn __iter__(&self) -> PyTextSelectionIter {
        PyTextSelectionIter {
            positions: self
                .map(|res| {
                    Ok(res
                        .positions(PositionMode::Begin)
                        .map(|x| *x)
                        .collect::<Vec<usize>>())
                })
                .unwrap(),
            index: 0,
            subindex: 0,
            resource_handle: self.handle,
            store: self.store.clone(),
        }
    }

    /// Iterates over all known textselections that start in the spceified range, in sorted order
    fn range(&self, begin: usize, end: usize) -> PyResult<PyTextSelectionIter> {
        Ok(PyTextSelectionIter {
            positions: self
                .map(|res| {
                    Ok(res
                        .positions(PositionMode::Begin)
                        .filter_map(|x| {
                            if *x >= begin && *x < end {
                                Some(*x)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<usize>>())
                })
                .unwrap(),
            index: 0,
            subindex: 0,
            resource_handle: self.handle,
            store: self.store.clone(),
        })
    }

    /// Converts a unicode character position to a UTF-8 byte position
    fn utf8byte(&self, abscursor: usize) -> PyResult<usize> {
        self.map(|res| res.utf8byte(abscursor))
    }

    /// Converts a UTF-8 byte position into a unicode position
    fn utf8byte_to_charpos(&self, bytecursor: usize) -> PyResult<usize> {
        self.map(|res| res.utf8byte_to_charpos(bytecursor))
    }

    /// Converts an end-aligned cursor to a begin-aligned cursor, resolving all relative end-aligned positions
    /// The parameter value must be 0 or negative.
    fn beginaligned_cursor(&self, endalignedcursor: isize) -> PyResult<usize> {
        self.map(|res| res.beginaligned_cursor(&Cursor::EndAligned(endalignedcursor)))
    }

    /// Returns a list of all annotations (:obj:`Annotation`) that reference this resource via a TextSelector (if any).
    /// Does *NOT* include those that use a ResourceSelector, use `annotations_metadata()` instead for those.
    #[pyo3(signature = (limit=None))]
    fn annotations(&self, limit: Option<usize>, py: Python) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|resource| {
            for (i, annotation) in resource.annotations().into_iter().flatten().enumerate() {
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

    /// Returns a list of all annotations (:obj:`Annotation`) that reference this resource via a ResourceSelector (if any).
    /// Does *NOT* include those that use a TextSelector, use `annotations()` instead for those.
    #[pyo3(signature = (limit=None))]
    fn annotations_metadata(&self, limit: Option<usize>, py: Python) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|resource| {
            for (i, annotation) in resource
                .annotations_metadata()
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

    /// Applies a `TextSelectionOperator` to find all other text selections that are in a specific
    /// relation with the reference selections (a list of :obj:`TextSelection` instances). Returns
    /// all matching TextSelections in a list
    ///
    /// If you are interested in the annotations associated with the found text selections, then use `find_annotations()` instead.
    #[pyo3(signature = (operator,referenceselections,limit=None))]
    fn find_textselections(
        &self,
        operator: PyTextSelectionOperator,
        referenceselections: Vec<PyTextSelection>,
        limit: Option<usize>,
        py: Python,
    ) -> Py<PyList> {
        let list: &PyList = PyList::empty(py);
        self.map(|textselection| {
            let mut refset = TextSelectionSet::new(self.handle);
            refset.extend(referenceselections.into_iter().map(|x| x.textselection));
            for (i, foundtextselection) in textselection
                .find_textselections(operator.operator, refset)
                .enumerate()
            {
                list.append(
                    PyTextSelection {
                        textselection: if foundtextselection.is_borrowed() {
                            foundtextselection.unwrap().clone()
                        } else {
                            foundtextselection.unwrap_owned()
                        },
                        resource_handle: self.handle,
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
}

impl PyTextResource {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    pub(crate) fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(WrappedItem<TextResource>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource = store
                .resource(&self.handle.into())
                .ok_or_else(|| PyRuntimeError::new_err("Failed to resolve textresource"))?;
            f(resource).map_err(|err| PyStamError::new_err(format!("{}", err)))
        } else {
            Err(PyRuntimeError::new_err(
                "Unable to obtain store (should never happen)",
            ))
        }
    }
}

#[pyclass(dict, module = "stam", name = "Cursor")]
/// A cursor points to a specific point in a text. I
/// Used to select offsets. Units are unicode codepoints (not bytes!)
/// and are 0-indexed.
///
/// The cursor can be either begin-aligned or end-aligned. Where BeginAlignedCursor(0)
/// is the first unicode codepoint in a referenced text, and EndAlignedCursor(0) the last one.
///
/// Args:
///     `index` (:obj:`int`) - The index for the cursor
///     `endaligned` (:obj:`bool`, `optional`) - For an end-aligned cursor, set this to True. The index value should be 0 or negative.
#[derive(Clone)]
#[pyo3(text_signature = "(self, index, endaliged=None)")]
pub(crate) struct PyCursor {
    cursor: Cursor,
}

#[pymethods]
impl PyCursor {
    #[new]
    fn new(index: isize, endaligned: Option<bool>) -> PyResult<Self> {
        if endaligned.unwrap_or(false) {
            if index <= 0 {
                Ok(Self {
                    cursor: Cursor::EndAligned(index),
                })
            } else {
                Err(PyValueError::new_err(
                    "End aligned cursor should be 0 or negative",
                ))
            }
        } else {
            if index >= 0 {
                Ok(Self {
                    cursor: Cursor::BeginAligned(index as usize),
                })
            } else {
                Err(PyValueError::new_err(
                    "Begin aligned cursor should be 0 or positive",
                ))
            }
        }
    }

    /// Tests if this is a begin-aligned cursor
    fn is_beginaligned(&self) -> bool {
        match self.cursor {
            Cursor::BeginAligned(_) => true,
            _ => false,
        }
    }

    /// Tests if this is an end-aligned cursor
    fn is_endaligned(&self) -> bool {
        match self.cursor {
            Cursor::EndAligned(_) => true,
            _ => false,
        }
    }

    /// Returns the actual cursor value
    fn value(&self) -> isize {
        match self.cursor {
            Cursor::BeginAligned(v) => v as isize,
            Cursor::EndAligned(v) => v,
        }
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.cursor == other.cursor).into_py(py),
            CompareOp::Ne => (self.cursor != other.cursor).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn __str__(&self) -> String {
        match self.cursor {
            Cursor::BeginAligned(v) => v.to_string(),
            Cursor::EndAligned(v) if v == 0 => format!("-{}", v),
            Cursor::EndAligned(v) => v.to_string(),
        }
    }
}

#[pyclass(dict, module = "stam", name = "Offset")]
pub(crate) struct PyOffset {
    pub(crate) offset: Offset,
}

#[pymethods]
impl PyOffset {
    #[new]
    fn new(begin: PyCursor, end: PyCursor) -> Self {
        Self {
            offset: Offset {
                begin: begin.cursor,
                end: end.cursor,
            },
        }
    }

    #[staticmethod]
    /// Creates a simple offset with begin aligned cursors
    /// This is typically faster than using the normal constructor
    fn simple(begin: usize, end: usize) -> Self {
        Self {
            offset: Offset::simple(begin, end),
        }
    }

    #[staticmethod]
    /// Creates a offset that references the whole text
    /// This is typically faster than using the normal constructor
    fn whole() -> Self {
        Self {
            offset: Offset {
                begin: Cursor::BeginAligned(0),
                end: Cursor::EndAligned(0),
            },
        }
    }

    /// Return the begin cursor
    fn begin(&self) -> PyCursor {
        PyCursor {
            cursor: self.offset.begin,
        }
    }

    /// Return the end cursor
    fn end(&self) -> PyCursor {
        PyCursor {
            cursor: self.offset.end,
        }
    }

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
        let py = other.py();
        match op {
            CompareOp::Eq => (self.offset.begin == other.offset.begin
                && self.offset.end == other.offset.end)
                .into_py(py),
            CompareOp::Ne => (self.offset.begin != other.offset.begin
                || self.offset.end != other.offset.end)
                .into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn __str__(&self) -> String {
        format!(
            "{}:{}",
            match self.offset.begin {
                Cursor::BeginAligned(v) => v.to_string(),
                Cursor::EndAligned(v) if v == 0 => format!("-{}", v),
                Cursor::EndAligned(v) => v.to_string(),
            },
            match self.offset.end {
                Cursor::BeginAligned(v) => v.to_string(),
                Cursor::EndAligned(v) if v == 0 => format!("-{}", v),
                Cursor::EndAligned(v) => v.to_string(),
            }
        )
    }
}
