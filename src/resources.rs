use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::*;
use std::ops::FnOnce;
use std::sync::{Arc, RwLock};

use crate::annotation::PyAnnotations;
use crate::error::PyStamError;
use crate::iterparams::IterParams;
use crate::selector::{PySelector, PySelectorKind};
use crate::textselection::{
    PyTextSelection, PyTextSelectionIter, PyTextSelectionOperator, PyTextSelections,
};
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

impl PyTextResource {
    pub(crate) fn new(
        handle: TextResourceHandle,
        store: &Arc<RwLock<AnnotationStore>>,
    ) -> PyTextResource {
        PyTextResource {
            handle,
            store: store.clone(),
        }
    }

    pub(crate) fn new_py<'py>(
        handle: TextResourceHandle,
        store: &Arc<RwLock<AnnotationStore>>,
        py: Python<'py>,
    ) -> &'py PyAny {
        Self::new(handle, store).into_py(py).into_ref(py)
    }
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

    fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.handle == other.handle,
            CompareOp::Ne => self.handle != other.handle,
            CompareOp::Lt => self.handle < other.handle,
            CompareOp::Gt => self.handle > other.handle,
            CompareOp::Le => self.handle <= other.handle,
            CompareOp::Ge => self.handle >= other.handle,
        }
    }

    fn __hash__(&self) -> usize {
        self.handle.as_usize()
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
        self.map(|res| {
            if case_sensitive == Some(false) {
                for (i, textselection) in res.find_text_nocase(fragment).enumerate() {
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
                for (i, textselection) in res.find_text(fragment).enumerate() {
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
        self.map(|res| {
            let results = res.find_text_sequence(
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
                        .append(PyTextSelection::from_result_to_py(
                            textselection.clone(),
                            &self.store,
                            py,
                        ))
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

    /// Trims all occurrences of any character in `chars` from both the beginning and end of the text,
    /// returning a smaller TextSelection. No text is modified.
    fn strip_text(&self, chars: &str) -> PyResult<PyTextSelection> {
        let chars: Vec<char> = chars.chars().collect();
        self.map(|res| {
            res.trim_text(&chars)
                .map(|textselection| PyTextSelection::from_result(textselection, &self.store))
        })
    }

    /// Returns a Selector (ResourceSelector) pointing to this TextResource
    fn select(&self) -> PyResult<PySelector> {
        self.map(|resource| {
            Ok(PySelector {
                kind: PySelectorKind {
                    kind: SelectorKind::ResourceSelector,
                },
                resource: Some(resource.handle()),
                annotation: None,
                dataset: None,
                offset: None,
                subselectors: Vec::new(),
            })
        })
    }

    fn textselections(&self) -> PyResult<PyTextSelections> {
        self.map(|resource| {
            let iter = resource.textselections();
            Ok(PyTextSelections {
                textselections: iter.to_handles(),
                store: self.store.clone(),
                cursor: 0,
            })
        })
    }

    // Iterates over all known textselections in this resource, in sorted order
    fn __iter__(&self) -> PyTextSelectionIter {
        PyTextSelectionIter {
            positions: self
                .map(|res| {
                    Ok(res
                        .as_ref()
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
                        .as_ref()
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

    /// Returns all annotations (:obj:`Annotation`) that reference this resource via either a TextSelector or a ResourceSelector (if any).
    #[pyo3(signature = (**kwargs))]
    fn annotations(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations();
            iterparams.evaluate_to_pyannotations(iter, resource.store(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn annotations_as_metadata(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations_as_metadata();
            iterparams.evaluate_to_pyannotations(iter, resource.store(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn annotations_on_text(&self, kwargs: Option<&PyDict>) -> PyResult<PyAnnotations> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations_on_text();
            iterparams.evaluate_to_pyannotations(iter, resource.store(), &self.store)
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations();
            Ok(iterparams
                .evaluate_annotations(iter, resource.store())?
                .test())
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations_as_metadata(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations_as_metadata();
            Ok(iterparams
                .evaluate_annotations(iter, resource.store())?
                .test())
        })
    }

    #[pyo3(signature = (**kwargs))]
    fn test_annotations_on_text(&self, kwargs: Option<&PyDict>) -> PyResult<bool> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|resource| {
            let iter = resource.annotations_on_text();
            Ok(iterparams
                .evaluate_annotations(iter, resource.store())?
                .test())
        })
    }

    /// Applies a `TextSelectionOperator` to find all other text selections that are in a specific
    /// relation with the reference selections (a list of :obj:`TextSelection` instances). Returns
    /// all matching TextSelections in a list
    ///
    /// If you are interested in the annotations associated with the found text selections, then use `find_annotations()` instead.
    fn related_text(
        &self,
        operator: PyTextSelectionOperator,
        referenceselections: Vec<PyTextSelection>,
        kwargs: Option<&PyDict>,
    ) -> PyResult<PyTextSelections> {
        let iterparams = IterParams::new(kwargs)?;
        self.map(|textselection| {
            let mut refset = TextSelectionSet::new(self.handle);
            refset.extend(referenceselections.into_iter().map(|x| x.textselection));
            let iter = textselection.related_text(operator.operator, refset);
            iterparams.evaluate_to_pytextselections(iter, textselection.rootstore(), &self.store)
        })
    }
}

impl PyTextResource {
    /// Map function to act on the actual underlying store, helps reduce boilerplate
    pub(crate) fn map<T, F>(&self, f: F) -> Result<T, PyErr>
    where
        F: FnOnce(ResultItem<TextResource>) -> Result<T, StamError>,
    {
        if let Ok(store) = self.store.read() {
            let resource = store
                .resource(self.handle)
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
pub(crate) struct PyCursor {
    cursor: Cursor,
}

#[pymethods]
impl PyCursor {
    #[new]
    #[pyo3(text_signature = "(self, index, endaliged=None)")]
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
#[derive(Clone, PartialEq)]
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

    pub(crate) fn __richcmp__(&self, other: PyRef<Self>, op: CompareOp) -> Py<PyAny> {
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
