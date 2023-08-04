from __future__ import annotations

from typing import Iterator, List, Optional, Union

class AnnotationStore:
    """
    An Annotation Store is an unordered collection of annotations, resources and
    annotation data sets. It can be seen as the *root* of the *graph model* and the glue
    that holds everything together. It is the entry point for any stam model.
    """
    def __init__(self, id=None,file=None, string=None,config=None) -> None:
        """
        Instantiates a new annotationstore.
        
        Parameters
        ----------------
        id: Optional[str], default: None
            The public ID for a *new* store
        file: Optional[str], default: None
            The STAM JSON or STAM CSV file to load
        string: Optional[str], default: None
            STAM JSON as a string
        config: Optional[dict]
            A python dictionary containing configuration parameters (see below)

        At least one of `id`, `file` or `string` must be specified.

        Configuration Parameters
        ---------------------------
        use_include: Optional[bool], default: True
            Use the `@include` mechanism to point to external files, if unset, all data will be kept in a single STAM JSON file.
        debug: Optional[bool], default: False
            Enable debug mode, outputs extra information to standard error output (verbose!)
        annotation_annotation_map: Optional[bool], default: True
            Enable/disable index for annotations that reference other annotations
        resource_annotation_map: Optional[bool], default: True
            Enable/disable reverse index for TextResource => Annotation. Holds only annotations that **directly** reference the TextResource (via [`crate::Selector::ResourceSelector`]), i.e. metadata
        dataset_annotation_map: Optional[bool], default: True
            Enable/disable reverse index for AnnotationDataSet => Annotation. Holds only annotations that **directly** reference the AnnotationDataSet (via [`crate::Selector::DataSetSelector`]), i.e. metadata
        textrelationmap: Optional[bool], default: True
            Enable/disable the reverse index for text, it maps TextResource => TextSelection => Annotation
        generate_ids: Optional[bool], default: False
            Generate pseudo-random public identifiers when missing (during deserialisation). Each will consist of 21 URL-friendly ASCII symbols after a prefix of A for Annotations, S for DataSets, D for AnnotationData, R for resources
        strip_temp_ids: Optional[bool], default: True
            Strip temporary IDs during deserialisation. Temporary IDs start with an exclamation mark, a capital ASCII letter denoting the type, and a number
        shrink_to_fit: Optional[bool], default: True
            Shrink data structures to optimize memory (at the cost of longer deserialisation times)
        milestone_interval: Optional[int], default: 100
            Milestone placement interval (in unicode codepoints) in indexing text resources. A low number above zero increases search performance at the cost of memory and increased initialisation time.
        """

    def id(self) -> Optional[str]:
        """Returns the public identifier (by value, aka a copy)"""

    def to_file(self, filename: str) -> None:
        """Saves the annotation store to file. Use either .json or .csv as extension."""

    def save(self) -> None:
        """Saves the annotation store to the same file it was loaded from or last saved to."""

    def to_json_string(self) -> str:
        """Returns the annotation store as one big STAM JSON string"""

    def dataset(self, id: str) -> AnnotationDataSet:
        """Basic retrieval method that returns an :class:`AnnotationDataSet` by ID"""

    def annotation(self, id: str) -> Annotation:
        """Basic retrieval method that returns an :class:`Annotation` by ID"""

    def resource(self, id: str) -> TextResource:
        """Basic retrieval method that returns a :class:`TextResource` by ID"""

    def add_resource(self, filename: Optional[str] = None, text: Optional[str] = None, id: Optional[str] = None) -> TextResource:
        """Create a new :class:`TextResource` and add it to the store. Returns the added instance."""

    def add_dataset(self, id: str) -> AnnotationDataSet:
        """Create a new :class:`AnnotationDataSet` and add it to the store. Returns the added instance."""

    def set_filename(self, filename: str) -> None:
        """Set the filename for the annotationstore, the format is derived from the extension, can be `.json` or `csv`"""
    
    def annotate(self, target: Selector, data: Union[dict,List[dict],AnnotationData,List[AnnotationData]], id: Optional[str]) -> Annotation:
        """Adds a new annotation. Returns the :obj:`Annotation` instance that was just created.
        
        Parameters
        -------------
            
        target: :class:`Selector`
            A target selector that determines the object of annotation
        data: Union[dict,List[dict],AnnotationData,List[AnnotationData]]
            A dictionary or list of dictionaries with data to set. The dictionary
            may have fields: `id`,`key`,`set`, and `value`.
            Alternatively, you can pass an existing`AnnotationData` instance.
        id: Optional[str]
            The public ID for the annotation. If unset, one may be autogenerated.
        """

    def annotations(self) -> Iterator[Annotation]:
        """Returns an iterator over all annotations (:class:`Annotation`) in this store"""

    def datasets(self) -> Iterator[AnnotationDataSet]:
        """Returns an iterator over all annotation data sets (:class:`AnnotationDataSet`) in this store"""

    def resources(self) -> Iterator[TextResource]:
        """Returns an iterator over all text resources (:class:`TextResource`) in this store"""

    def annotations_len(self) -> int:
        """Returns the number of annotations in the store (not substracting deletions)"""

    def datasets_len(self) -> int:
        """Returns the number of annotation data sets in the store (not substracting deletions)"""

    def resources_len(self) -> int:
        """Returns the number of text resources in the store (not substracting deletions)"""


class Annotation:
    """
    `Annotation` represents a particular *instance of annotation* and is the central
    concept of the model. They can be considered the primary nodes of the graph model. The
    instance of annotation is strictly decoupled from the *data* or key/value of the
    annotation (:class:`AnnotationData`). After all, multiple instances can be annotated
    with the same label (multiple annotations may share the same annotation data).
    Moreover, an `Annotation` can have multiple annotation data associated.
    The result is that multiple annotations with the exact same content require less storage
    space, and searching and indexing is facilitated.  
   
    This structure is not instantiated directly, only returned. Use :meth:`AnnotationStore.annotate()` to instantiate a new Annotation.
    """

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> Optional[str]:
        """Tests the ID"""

    def __iter__(self) -> Iterator[AnnotationData]:
        """Returns a iterator over all data (:class:`AnnotationData`) in this annotation"""

    def __len__(self) -> Iterator[AnnotationData]:
        """Returns the number of data items (:class:`AnnotationData`) in this annotation"""

    def select(self) -> Selector:
        """Returns a selector pointing to this annotation"""

    def text(self) -> List[str]:
        """Returns the text of the annotation.
        Note that this will always return a list (even it if only contains a single element),
        as an annotation may reference multiple texts.
       
        If you are sure an annotation only reference a single contingent text slice or are okay with slices being concatenated, then you can use the `str()` function instead."""

    def __str__(self) -> str:
        """
        Returns the text of the annotation.
        If the annotation references multiple text slices, they will be concatenated with a space as a delimiter,
        but note that in reality the different parts may be non-contingent!
       
        Use `text()` instead to retrieve a list of texts
        """

    def textselections(self) -> List[TextSelection]:
        """
        Returns a list of all textselections (:class:`TextSelection`) referenced by the annotation (i.e. via a *TextSelector*).
        Note that this will always return a list (even it if only contains a single element),
        as an annotation may reference multiple text selections.
        """

    def annotations(self, recursive= False, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) this annotation refers to (i.e. using an *AnnotationSelector*)

        Parameters
        ------------

        recursive: bool
            Follow other Annotation Selectors when encounted
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_reverse(self,  limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that are referring to this annotation (i.e. others using an AnnotationSelector)

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def resources(self, limit: Optional[int] = None) -> List[TextResource]:
        """Returns a list of resources (:class:`TextResource`) this annotation refers to

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def datasets(self, limit: Optional[int] = None) -> List[AnnotationDataSet]:
        """Returns a list of annotation data sets (:class:`AnnotationDataSet`) this annotation refers to. This only returns the ones
        referred to via a *DataSetSelector*, i.e. as metadata.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def offset(self) -> Optional[Offset]:
        """Returns the offset this annotation's selector targets, exactly as specified"""

    def selector_kind(self) -> SelectorKind:
        """Returns the type of the selector of this annotation"""


    def data(self, limit: Optional[int] = None) -> List[AnnotationData]:
        """Returns a list of annotation data instances this annotation refers to."""

    def related_text(self, operator: TextSelectionOperator, limit: Optional[int] = None) -> List[TextSelection]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the ones from the current annotation. 
        Returns all matching :class:`TextSelection` instances in a list.
       
        If you are interested in the annotations associated with the found text selections, then
        use :meth:`find_annotations` instead.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_by_related_text(self, operator: TextSelectionOperator, limit: Optional[int] = None) -> List[Annotation]:
        """
        Applies a :class:`TextSelectionOperator` to find all other annotations whose text selections
        are in a specific relation with the ones from the current annotation. 
        Returns all matching :class:`Annotation` instances in a list, in textual order.
       
        If you want to additionally filter on data, use :meth:`annotations_by_related_text_and_data` instead.

        If you are primarily interested in the text selections and not so much in the annotatons,
        then use :meth:`related_text` instead.


        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing the underlying text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)

        See :meth:`find_data` for possible keyword arguments.
        """

    def annotations_by_related_text_and_data(self, operator: TextSelectionOperator, **kwargs) -> List[Annotation]:
        """
        Applies a :class:`TextSelectionOperator` to find all other annotations whose text selections
        are in a specific relation with the ones from the current annotation. Furthermore, annotations
        are immediately filtered on data.

        Returns all matching :class:`Annotation` instances in a list, in textual order.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing the underlying text selections
        """

    def find_data(self, **kwargs) -> List[AnnotationData]:
        """
        Find annotation data, pertaining to this Annotation, by key and value.
        Returns a list of :class:`AnnotationData` instances

        Keyword arguments
        -------------------

        key: Optional[Union[str,DataKey]]
            The key to search for; it will search all keys if not specified
        value: Optional[Union[str,int,float,bool]]
            The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
        value_not: Optional[Union[str,int,float,bool]]
            Value
        value_greater: Optional[Union[int,float]]
            Value must be greater than specified (int or float)
        value_less: Optional[Union[int,float]]
            Value must be less than specified (int or float)
        value_greatereq: Optional[Union[int,float]]
            Value must be greater than specified or equal (int or float)
        value_lesseq: Optional[Union[int,float]]
            Value must be less than specified or equal (int or float)
        value_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must match any in the tuple (this is a logical OR statement)
        value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must not match any in the tuple
        value_in_range: Optional[Tuple[Union[int,float]]]
            Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def test_data(self, **kwargs) -> bool:
        """
        Tests whether the annotation has certain keys and data.
        See :meth:`find_data` for possible keyword arguments.
        """

    def find_data_about(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for data *about* this annotation, i.e. data on other annotation that refer to this one.
        Do not confuse this with the data this annotation holds, which can be searched with :meth:`find_data`.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        See :meth:`find_data` for possible keyword arguments.
        """

    def test_data_about(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this annotation, i.e. data on other annotation that refer to this one.
        Do not confuse this with the data this annotation holds, which can be tested with :meth:`test_data`.
 
        See :meth:`find_data` for possible keyword arguments.
        """

    def related_text_with_data(self, operator: TextSelectionOperator, **kwargs) -> List[tuple[TextSelection,List[tuple[AnnotationData,Annotation]]]]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the text selections of the current annotation.
    
        Furthermore, it only returns text that has certain data describing it.
        It returns both the matching text and for each also the matching annotation data and matching annotation

        If you do not wish to return the data, but merely test for it, then use :meth:`related_text_test_data()` instead.

        This method effectively combines `related_text()` with `find_data_about()` on its results, into a single method.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)

        See :meth:`find_data_about` for possible keyword arguments to filter on data.
        """

    def related_text_test_data(self, operator: TextSelectionOperator,  **kwargs) -> List[tuple[TextSelection]]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the text selections of the current annotation.
    
        Furthermore, it only returns text that has certain data describing it.
        It returns only the matching text.

        If you wish to return the data as well, then use :meth:`related_text_with_data()` instead.

        This method effectively combines `related_text()` with `test_data_about()` on its results, into a single method.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)

        See :meth:`find_data_about` for possible keyword arguments to filter on data.
        """

class AnnotationDataSet:
    """
    An `AnnotationDataSet` stores the keys (:class:`DataKey`) and values
    :class:`AnnotationData` (which in turn encapsulates :class:`DataValue`) that are used by annotations.

    It effectively defines a certain vocabulary, i.e. key/value pairs.
    The `AnnotationDataSet` does not store the :class:`Annotation` instances, those are in
    the :class:`AnnotationStore`. The datasets themselves are also held by the `AnnotationStore`.

    Use :meth:`AnnotationStore.add_annotationset()` to instantiate a new AnnotationDataSet, it can not be constructed directly.
    """

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> Optional[str]:
        """Tests the ID"""

    def key(self, key: str) -> DataKey:
        """Basic retrieval method to obtain a key from a dataset"""

    def add_key(self, key: str) -> DataKey:
        """Create a new :class:`DataKey` and adds it to the dataset. Returns the added key."""

    def keys_len(self) -> int:
        """Returns the number of keys in the set"""

    def data_len(self) -> int:
        """Returns the number of annotation data instances in the set"""

    def add_data(self, key: str, value: Union[DataValue,str,float,int,list,bool], id: Optional[str] = None) -> AnnotationData:
        """Create a new :class:`AnnotationData` instances and add it to the dataset. Returns the added data."""

    def annotationdata(self, id: str) -> AnnotationData:
        """Basic retrieval method to obtain annotationdata from a dataset, by ID"""

    def keys(self) -> Iterator[DataKey]:
        """Returns an iterator over all :class:`DataKey` instances in the dataset"""

    def __iter__(self) -> Iterator[AnnotationData]:
        """Returns an iterator over all :class:`AnnotationData` in the dataset"""

    def select(self) -> Selector:
        """Returns a selector pointing to this annotation dataset (via a *DataSetSelector*)"""

    def find_data(self, **kwargs) -> List[AnnotationData]:
        """
        Find annotation data by key and value.
        Returns a list of :class:`AnnotationData` instances

        Keyword arguments
        -------------------

        key: Optional[Union[str,DataKey]]
            The key to search for; it will search all keys if not specified
        value: Optional[Union[str,int,float,bool]]
            The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
        value_not: Optional[Union[str,int,float,bool]]
            Value
        value_greater: Optional[Union[int,float]]
            Value must be greater than specified (int or float)
        value_less: Optional[Union[int,float]]
            Value must be less than specified (int or float)
        value_greatereq: Optional[Union[int,float]]
            Value must be greater than specified or equal (int or float)
        value_lesseq: Optional[Union[int,float]]
            Value must be less than specified or equal (int or float)
        value_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must match any in the tuple (this is a logical OR statement)
        value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must not match any in the tuple
        value_in_range: Optional[Tuple[Union[int,float]]]
            Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def test_data(self, **kwargs) -> bool:
        """
        Tests whether the dataset has certain keys and data.
        See :meth:`find_data` for possible keyword arguments.
        """

    def find_data_about(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for data *about* this dataset, i.e. data on annotations that refer to this dataset via a DataSetSelector.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        See :meth:`find_data` for possible keyword arguments.
        """

    def test_metadata_about(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this dataset, i.e. data on annotations that refer to this dataset via a DataSetSelector.
 
        See :meth:`find_data` for possible keyword arguments.
        """

class DataKey:
    """
    The DataKey class defines a vocabulary field, it
    belongs to a certain :class:`AnnotationDataSet`. A :class:`AnnotationData` instance
    in turn makes reference to a DataKey and assigns it a value.
    """

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> Optional[str]:
        """Tests the ID"""

    def dataset(self) -> AnnotationDataSet:
        """Returns the :class:`AnnotationDataSet` this key is part of"""

    def annotationdata(self) -> List[AnnotationData]:
        """
        Returns a list of :class:`AnnotationData` instances that use this key.
        """

    def find_data(self, **kwargs) -> List[AnnotationData]:
        """
        Find annotation data for the current key by value.
        Returns a list of :class:`AnnotationData` instances

        Keyword arguments
        -------------------

        key: Optional[Union[str,DataKey]]
            The key to search for; it will search all keys if not specified
        value: Optional[Union[str,int,float,bool]]
            The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
        value_not: Optional[Union[str,int,float,bool]]
            Value
        value_greater: Optional[Union[int,float]]
            Value must be greater than specified (int or float)
        value_less: Optional[Union[int,float]]
            Value must be less than specified (int or float)
        value_greatereq: Optional[Union[int,float]]
            Value must be greater than specified or equal (int or float)
        value_lesseq: Optional[Union[int,float]]
            Value must be less than specified or equal (int or float)
        value_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must match any in the tuple (this is a logical OR statement)
        value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must not match any in the tuple
        value_in_range: Optional[Tuple[Union[int,float]]]
            Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def test_data(self, **kwargs) -> bool:
        """
        Tests whether the DataKey has certain data.
        See :meth:`find_data` for possible keyword arguments.
        """

    def annotations(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that use this key.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_count(self, limit: Optional[int] = None) -> int:
        """Returns the number of annotations (:class:`Annotation`) that use this data.
        Note that this is much faster than doing `len(annotations())`!
        This method has suffix `_count` instead of `_len` because it is not O(1) but does actual counting (O(n) at worst).

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

class DataValue:
    """Encapsulates a value and its type. Held by :class:`AnnotationData`. This type is not a reference but holds the actual value."""


    def get(self) -> Union[str,bool,int,float,List]:
        """Get the actual value"""

    def __init__(self, value: Union[str,bool,int,float,List]) -> None:
        """Instantiate a new DataValue from a Python type. You usually don't need to do this explicitly"""

    def __str__(self) -> str:
        """Get the actual value as as string"""

class AnnotationData:
    """
    AnnotationData holds the actual content of an annotation; a key/value pair. (the
    term *feature* is regularly seen for this in certain annotation paradigms).
    Annotation Data is deliberately decoupled from the actual :class:`Annotation`
    instances so multiple annotation instances can point to the same content
    without causing any overhead in storage. Moreover, it facilitates indexing and
    searching. The annotation data is part of an :class:`AnnotationDataSet`, which
    effectively defines a certain user-defined vocabulary.
   
    Once instantiated, instances of this type are, by design, largely immutable.
    The key and value can not be changed. Create a new AnnotationData and new Annotation for edits.
    This class is not instantiated directly.
    """

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> Optional[str]:
        """Tests the ID"""

    def key(self) -> DataKey:
        """Basic retrieval method to obtain the key"""

    def value(self) -> DataValue:
        """Basic retrieval method to obtain the value"""

    def test_value(self, reference: DataValue) -> bool:
        """
        Tests whether the value equals another
        This is more efficient than calling :meth:`value`] and doing the comparison yourself.
        """

    def annotationset(self) -> AnnotationDataSet:
        """Returns the :class:`AnnotationDataSet` this data is part of"""

    def annotations(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that use this data.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_len(self, limit: Optional[int] = None) -> int:
        """Returns the number of annotations (:class:`Annotation`) that use this data.
        Note that this is much faster than doing `len(annotations())`!

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """


class Selector:
    """
    A *Selector* identifies the target of an annotation and the part of the
    target that the annotation applies to. Selectors can be considered the labelled edges of the graph model, tying all nodes together.
    There are multiple types of selectors, all captured in this class. There are several static methods available to instantiate a specific type of selector.
    """
    
    @staticmethod
    def textselector(resource: TextResource, offset: Offset) -> Selector:
        """Creates a *TextSelector*. Selects a target resource and a text span within it. 

        Parameters
        ------------

        resource: TextResource
            The text resource 
        offset: Offset
            An offset pointing to the slice of the text in the resource 
        """

    @staticmethod
    def annotationselector(annotation: Annotation, offset: Optional[Offset] = None) -> Selector:
        """Creates an *AnnotationSelector* - A selector pointing to another annotation. This we call higher-order annotation and is very common in STAM models. If the annotation that is being targeted eventually refers to a text (`TextSelector`), then offsets **MAY** be specified that select a subpart of this text. These offsets are now *relative* to the annotation.

        Parameters
        ------------

        annotation: Annotation
            The target annotation 
        offset: Optional[Offset]
            If sets, references a subpart of the annotation's text. If set to `None`, it applies to the annotation as such. 
        """

    @staticmethod
    def resourceselector(resource: TextResource) -> Selector:
        """Creates a *ResourceSelector* - A selector pointing to a resource as whole. These type
  of annotation can be interpreted as *metadata*.

        Parameters
        ------------

        resource: TextResource
            The resource 
        """

    @staticmethod
    def datasetselector(dataset: AnnotationDataSet) -> Selector:
        """Creates a *DataSetSelector* - A selector pointing to an annotation dataset as whole. These type
  of annotation can be interpreted as *metadata*.

        Parameters
        ------------

        dataset: AnnotationDataSet
            The annotation data set.
        """

    @staticmethod
    def multiselector(*subselectors: Selector) -> Selector:
        """Creates a *MultiSelector* - A selector that consists of multiple other selectors (subselectors) to select multiple targets. This *MUST* be interpreted as the annotation applying to each target *individually*, without any relation between the different targets.

        Parameters
        ------------

        *subselectors: Selector
            The underlying selectors.
        """

    @staticmethod
    def compositeselector(*subselectors: Selector) -> Selector:
        """Creates a *CompositeSelector* - A selector that consists of multiple other selectors (subselectors), these are used to select more complex targets that transcend the idea of a single simple selection. This *MUST* be interpreted as the annotation applying equally to the conjunction as a whole, its parts being inter-dependent and for any of them it goes that they *MUST NOT* be omitted for the annotation to make sense.

        Parameters
        ------------

        *subselectors: Selector
            The underlying selectors.
        """

    @staticmethod
    def directionalselector(*subselectors: Selector) -> Selector:
        """Creates a *DirectionalSelector* - Another selector that consists of multiple other
        selectors, but with an explicit direction (from -> to), used to select more
        complex targets that transcend the idea of a single simple selection.

        Parameters
        ------------

        *subselectors: Selector
            The underlying selectors.
        """

    def kind(self) -> SelectorKind:
        """Returns the type of selector"""

    def is_kind(self, kind: SelectorKind) -> bool:
        """Tests whether a selector is of a particular type"""

    def offset(self) -> Optional[Offset]:
        """
        Return offset information in the selector.
        Works for TextSelector and AnnotationSelector, returns None for others.
        """

    def resource(self, store: AnnotationStore) -> Optional[TextResource]:
        """
        Returns the resource this selector points at, if any.
        Works only for TextSelector and ResourceSelector, returns None otherwise.
        Requires to explicitly pass the store so the resource can be found.
        """

    def dataset(self, store: AnnotationStore) -> Optional[AnnotationDataSet]:
        """
        Returns the annotation dataset this selector points at, ff any.
        Works only for DataSetSelector, returns None otherwise.
        Requires to explicitly pass the store so the dataset can be found.
        """

    def annotation(self, store: AnnotationStore) -> Optional[Annotation]:
        """
        Returns the annotation this selector points at, if any.
        Works only for AnnotationSelector, returns None otherwise.
        Requires to explicitly pass the store so the resource can be found.
        """

class SelectorKind:
    """An enumeration of possible selector types"""

    RESOURCESELECTOR: SelectorKind
    ANNOTATIONSELECTOR: SelectorKind
    TEXTSELECTOR: SelectorKind
    DATASETSELECTOR: SelectorKind
    MULTISELECTOR: SelectorKind
    COMPOSITESELECTOR: SelectorKind
    DIRECTIONALSELECTOR: SelectorKind

class Offset:
    """
    Text selection offset. Specifies begin and end offsets to select a range of a text, via two :class:`Cursor` instances.
    The end-point is non-inclusive.
    """

    def __init__(self, begin: Cursor, end: Cursor) -> None:
        """Instantiate a new offset on the basis of two :class:`Cursor` instances"""

    @staticmethod
    def simple(begin: int, end: int) -> Offset:
        """Instantiate a new offset on the basis of two begin aligned cursors"""

    @staticmethod
    def whole() -> Offset:
        """Instantiate a new offset that targets an entire text from begin to end."""

    def begin(self) -> Cursor:
        """Returns the begin cursor"""

    def end(self) -> Cursor:
        """Returns the end cursor"""

    def __str__(self) -> str:
        """Get a string representation of the offset"""


class Cursor:
    """
    A cursor points to a specific point in a text. It is used to select offsets. Units are unicode codepoints (not bytes!)
    and are 0-indexed.
   
    The cursor can be either begin-aligned or end-aligned. Where BeginAlignedCursor(0)
    is the first unicode codepoint in a referenced text, and EndAlignedCursor(0) the last one.
    """

    def __init__(self, index, endaligned: bool = False):
        """Instantiate a new cursor.

        Parameters
        ------------

        index: int
            The value for the cursor. 
        endaligned: bool
            Signals you want an end-aligned cursor, otherwise it is begin-aligned. If set this to True the index value should be 0 or negative, otherwise 0 or positive.
        """

    def is_beginaligned(self) -> bool:
       """Tests if this is a begin-aligned cursor"""

    def is_endaligned(self) -> bool:
       """Tests if this is an end-aligned cursor"""

    def value(self) -> int:
        """Get the actual cursor value"""

    def __str__(self) -> str:
        """Get a string representation of the cursor"""

class TextResource:
    """
    This holds the textual resource to be annotated. It holds the full text in memory.
   
    The text *SHOULD* be in
    [Unicode Normalization Form C (NFC) (https://www.unicode.org/reports/tr15/) but
    *MAY* be in another unicode normalization forms.
    """

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> Optional[str]:
        """Tests the ID"""

    def __iter__(self) -> Iterator[TextSelection]:
        """Iterates over all known textselections in this resource, in sorted order. Same as :meth:`textselections`"""

    def textselections(self) -> Iterator[TextSelection]:
        """Iterates over all known textselections in this resource, in sorted order. Same as :meth:`__iter__`"""

    def select(self) -> Selector:
        """Returns a selector pointing to this resource"""

    def text(self) -> str:
        """Returns the text of the resource (by value, aka a copy)"""

    def textlen(self) -> int:
        """
        Returns the length of the resources's text in unicode points (same as `len(self.text())` but more performant)
        """

    def __str__(self) -> str:
        """Returns the text of the resource (by value, aka a copy), same as :meth:`text`"""

    def __getitem__(self, slice: slice) -> str:
        """Returns a text slice"""

    def textselection(self, offset: Offset) -> TextSelection:
        """
        Returns a :class:`TextSelection` instance covering the specified offset.
        """

    def find_text(self, fragment: str, limit: Optional[int] = None, case_sensitive: Optional[bool] = None) -> List[TextSelection]:
        """Searches for the text fragment and returns a list of :class:`TextSelection` instances with all matches (or up to the specified limit)

        Parameters
        ------------

        fragment: str
            The exact fragment to search for (case-sensitive)
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        case_sensitive: Optional[bool] = None
            Match case sensitive or not (default: True)
        """

    def find_text_regex(self, expressions: List[str], allow_overlap: Optional[bool] = False, limit: Optional[int] = None) -> List[dict]:
        """
        Searches the text using one or more regular expressions, returns a list of dictionaries like:

        code::

            { "textselections": [TextSelection], "expression_index": int, "capturegroups": [int] }
       
        Passing multiple regular expressions at once is more efficient than calling this function anew for each one.
        If capture groups are used in the regular expression, only those parts will be returned (the rest is context). If none are used,
        the entire expression is returned. The regular expressions are passed as strings and
         must follow this syntax: https://docs.rs/regex/latest/regex/#syntax , which may differ slightly from Python's regular expressions!
       
        The `allow_overlap` parameter determines if the matching expressions are allowed to
        overlap. It you are doing some form of tokenisation, you also likely want this set to
        false. All of this only matters if you supply multiple regular expressions.
       
        Results are returned in the exact order they are found in the text
        """

    def split_text(self, delimiter: str, limit: Optional[int] = None) -> List[TextSelection]:
        """
        Returns a list of :class:`TextSelection` instances that split the text according to the specified delimiter.

        Parameters
        ------------

        delimiter: str
           The delimiter to split on 
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def strip_text(self, chars: str) -> TextSelection:
        """
        Trims all occurrences of any character in `chars` from both the beginning and end of the text,
        returning a :class:`TextSelection`. No text is modified.
        """

    def range(self, begin, end) -> Iterator[TextSelection]:
        """Iterates over all known textselections that start in the specified range, in sorted order."""    

    def utf8byte(self, abscursor: int) -> int:
        """Converts a unicode character position to a UTF-8 byte position"""

    def utf8byte_to_charpos(self, bytecursor: int) -> int:
        """Converts a UTF-8 byte position into a unicode position"""

    def beginaligned_cursor(self, endalignedcursor: int) -> int:
        """
        Converts an end-aligned cursor to a begin-aligned cursor, resolving all relative end-aligned positions
        The parameter value must be 0 or negative.
        """

    def annotations(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that reference this resource via any selector.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_on_text(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that reference this resource via a *TextSelector* (if any).
        Does *NOT* include those that use a ResourceSelector, use :meth:`annotations_metadata` instead for those instead.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """


    def annotations_as_metadata(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that reference this resource via a *ResourceSelector* (if any).
        Does *NOT* include those that use a TextSelector, use :meth:`annotations` instead for those instead.

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def related_text(self, operator: TextSelectionOperator, referenceselections: List[TextSelection], limit: Optional[int] = None) -> List[TextSelection]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the ones from `referenceselections`.
        Returns all matching :class:`TextSelection` instances in a list.
       
        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        referenceselections: List[TextSelection]
            Text selections to use as reference
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def find_data_about(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for data *about* this resource, i.e. data on annotations that refer to this resource by any means.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        Two other specialised variants are available that further constrain the search:
            
            * :meth:`find_metadata_about`
            * :meth:`find_data_about_text`

        Keyword arguments
        -------------------

        key: Optional[Union[str,DataKey]]
            The key to search for; it will search all keys if not specified
        value: Optional[Union[str,int,float,bool]]
            The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
        value_not: Optional[Union[str,int,float,bool]]
            Value
        value_greater: Optional[Union[int,float]]
            Value must be greater than specified (int or float)
        value_less: Optional[Union[int,float]]
            Value must be less than specified (int or float)
        value_greatereq: Optional[Union[int,float]]
            Value must be greater than specified or equal (int or float)
        value_lesseq: Optional[Union[int,float]]
            Value must be less than specified or equal (int or float)
        value_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must match any in the tuple (this is a logical OR statement)
        value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must not match any in the tuple
        value_in_range: Optional[Tuple[Union[int,float]]]
            Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def test_data_about(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this resource, i.e. data on annotations that refer to this resource by any means.
 
        See :meth:`find_data_about` for possible keyword arguments.
        """

    def find_metadata_about(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for (meta)data *about* this resource, i.e. data on annotations that refers to this resource using a ResourceSelector.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        Two other variants are available:
            
            * :meth:`find_data_about` - More generic variant
            * :meth:`find_data_about_text` - Other specialized variant

        See :meth:`find_data_about` for possible keyword arguments.
        """

    def test_metadata_about(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this resource, i.e. data on annotations that refer to this resource by any means.
 
        See :meth:`find_data_about` for possible keyword arguments.
        """

    def find_data_about_text(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for data *about* this text in this resource, i.e. data on annotations that refers to this resource using a TextSelector.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        Two other variants are available:
            
            * :meth:`find_data_about` - More generic variant
            * :meth:`find_metadata_about` - Other specialized variant

        See :meth:`find_data_about` for possible keyword arguments.
        """

    def test_data_about_text(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this text in this resource, i.e. data on annotations that refers to this resource using a TextSelector.
 
        See :meth:`find_data_about` for possible keyword arguments.
        """

class TextSelection:
    """
    This holds a slice of a text.
    """

    def resource(self) -> TextResource:
        """Returns the :class:`TextResource` this textselection is from."""

    def begin(self) -> int:
        """Return the absolute begin position in unicode points"""

    def end(self) -> int:
        """Return the absolute end position in unicode points (non-inclusive)"""

    def select(self) -> Selector:
        """Returns a selector pointing to this resource"""

    def text(self) -> str:
        """Returns the text of the resource (by value, aka a copy)"""

    def textlen(self) -> int:
        """
        Returns the length of the resources's text in unicode points (same as `len(self.text())` but more performant)
        """

    def __str__(self) -> str:
        """Returns the text of the resource (by value, aka a copy), same as :meth:`text`"""

    def __getitem__(self, slice: slice) -> str:
        """Returns a text slice"""

    def textselection(self, offset: Offset) -> TextSelection:
        """
        Returns a :class:`TextSelection` that corresponds to the offset **WITHIN** the current textselection.
        This returns a :class:`TextSelection` with absolute coordinates in the resource.
        """

    def find_text(self, fragment: str, limit: Optional[int] = None, case_sensitive: Optional[bool] = None) -> List[TextSelection]:
        """Searches for the text fragment and returns a list of :class:`TextSelection` instances with all matches (or up to the specified limit)

        Parameters
        ------------

        fragment: str
            The exact fragment to search for
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        case_sensitive: Optional[bool] = None
            Match case sensitive or not (default: True)
        """

    def find_text_regex(self, expressions: List[str], allow_overlap: Optional[bool] = False, limit: Optional[int] = None) -> List[dict]:
        """
        Searches the text using one or more regular expressions, returns a list of dictionaries like:

        code::

            { "textselections": [TextSelection], "expression_index": int, "capturegroups": [int] }
       
        Passing multiple regular expressions at once is more efficient than calling this function anew for each one.
        If capture groups are used in the regular expression, only those parts will be returned (the rest is context). If none are used,
        the entire expression is returned. The regular expressions are passed as strings and
         must follow this syntax: https://docs.rs/regex/latest/regex/#syntax , which may differ slightly from Python's regular expressions!
       
        The `allow_overlap` parameter determines if the matching expressions are allowed to
        overlap. It you are doing some form of tokenisation, you also likely want this set to
        false. All of this only matters if you supply multiple regular expressions.
       
        Results are returned in the exact order they are found in the text
        """

    def find_text_sequence(self, fragments: List[str], case_sensitive: Optional[bool] = None, allow_skip_whitespace: Optional[bool] = True, allow_skip_punctuation: Optional[bool] = True, allow_skip_numeric: Optional[bool] = True, allow_skip_alphabetic: Optional[bool] = False) -> List[TextSelection]:
        """
        Searches for the multiple text fragment in sequence. Returns a list of :class:`TextSelection` instances.
        
        Matches must appear in the exact order specified, but *may* have other intermittent text,
        determined by the `allow_skip_*` parameters.

        Returns an empty list if the sequence does not match.

        Parameters
        ------------

        fragments: List[str]
            The fragments to search for, in sequence
        case_sensitive: Optional[bool] = None
            Match case sensitive or not (default: True)
        allow_skip_whitespace: Optional[bool] = True
            Allow gaps consisting of whitespace (space, tabs, newline, etc) (default: True)
        allow_skip_punctuation: Optional[bool] = True
            Allow gaps consisting of punctuation (default: True)
        allow_skip_numeric: Optional[bool] = True
            Allow gaps consisting of numbers (default: True)
        allow_skip_alphabetic: Optional[bool] = True
            Allow gaps consisting of alphabetic/ideographic characters (default: False)
        """

    def split_text(self, delimiter: str, limit: Optional[int] = None) -> List[TextSelection]:
        """
        Returns a list of :class:`TextSelection` instances that split the text according to the specified delimiter.

        Parameters
        ------------

        delimiter: str
           The delimiter to split on 
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def strip_text(self, chars: str) -> TextSelection:
        """
        Trims all occurrences of any character in `chars` from both the beginning and end of the text,
        returning a :class:`TextSelection`. No text is modified.
        """

    def utf8byte(self, abscursor: int) -> int:
        """Converts a unicode character position to a UTF-8 byte position"""

    def utf8byte_to_charpos(self, bytecursor: int) -> int:
        """Converts a UTF-8 byte position into a unicode position"""

    def beginaligned_cursor(self, endalignedcursor: int) -> int:
        """
        Converts an end-aligned cursor to a begin-aligned cursor, resolving all relative end-aligned positions
        The parameter value must be 0 or negative.
        """

    def annotations_len(self) -> int:
        """Returns the number of annotations this text selection references"""

    def annotations(self, limit: Optional[int] = None) -> List[Annotation]:
        """Returns a list of annotations (:class:`Annotation`) that reference this text selection via a *TextSelector* (if any).

        Parameters
        ------------

        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def related_text(self, operator: TextSelectionOperator, limit: Optional[int] = None) -> List[TextSelection]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with this one.
        Returns all matching :class:`TextSelection` instances in a list.
       
        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_by_related_text(self, operator: TextSelectionOperator, limit: Optional[int] = None) -> List[Annotation]:
        """
        Applies a :class:`TextSelectionOperator` to find all annotations whose text selections
        are in a specific relation with the this one.. 
        Returns all matching :class:`Annotation` instances in a list.
       
        If you are interested in the annotations associated with the found text selections, then
        use :meth:`find_annotations` instead.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing the underlying text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)
        """

    def annotations_by_related_text_and_data(self, operator: TextSelectionOperator, **kwargs) -> List[Annotation]:
        """
        Applies a :class:`TextSelectionOperator` to find all annotations whose text selections
        are in a specific relation with this one. Furthermore, annotations
        are immediately filtered on data.

        Returns all matching :class:`Annotation` instances in a list, in textual order.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing the underlying text selections
        """

    def relative_offset(self, container: TextSelection) -> Offset:
        """
        Returns the offset of this text selection relative to another in which it is *embedded*.
        Raises a `StamError` exception if they are not embedded, or not belonging to the same resource.
        """

    def test(self, operator: TextSelectionOperator, other: TextSelection) -> bool:
        """
        This method is called to test whether a specific spatial relation (as expressed by the
        passed operator) holds between a [`TextSelection`] and another.
        A boolean is returned with the test result.
        """

    def find_data_about(self, **kwargs) -> List[tuple[AnnotationData,Annotation]]:
        """
        Search for data *about* this text, i.e. data on annotations that refer to this text via a TextSelector.
 
        Both the matching data (:class:`AnnotationData`) as well as the matching annotation (:class:`Annotation`) will be returned in a list of two-tuples

        Keyword arguments
        -------------------

        key: Optional[Union[str,DataKey]]
            The key to search for; it will search all keys if not specified
        value: Optional[Union[str,int,float,bool]]
            The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
        value_not: Optional[Union[str,int,float,bool]]
            Value
        value_greater: Optional[Union[int,float]]
            Value must be greater than specified (int or float)
        value_less: Optional[Union[int,float]]
            Value must be less than specified (int or float)
        value_greatereq: Optional[Union[int,float]]
            Value must be greater than specified or equal (int or float)
        value_lesseq: Optional[Union[int,float]]
            Value must be less than specified or equal (int or float)
        value_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must match any in the tuple (this is a logical OR statement)
        value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
            Value must not match any in the tuple
        value_in_range: Optional[Tuple[Union[int,float]]]
            Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def test_data_about(self, **kwargs) -> bool:
        """
        Test for the presence of data *about* this text, i.e. data on annotations that refer to this text via a TextSelector.
 
        See :meth:`find_data_about` for possible keyword arguments.
        """

    def related_text_with_data(self, operator: TextSelectionOperator, **kwargs) -> List[tuple[TextSelection,List[tuple[AnnotationData,Annotation]]]]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with this one.
    
        Furthermore, it only returns text has certain data describing it.
        It returns both the matching text and for each also the matching annotation data and matching annotation

        If you do not wish to return the data, but merely test for it, then use :meth:`related_text_test_data()` instead.

        This method effectively combines `related_text()` with `find_data_about()` on its results, into a single method.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)

        See :meth:`find_data_about` for possible keyword arguments to filter on data.
        """

    def related_text_test_data(self, operator: TextSelectionOperator, **kwargs) -> List[tuple[TextSelection]]:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with this one.
    
        Furthermore, it only returns text has certain data describing it.
        It returns only the matching text.

        If you wish to return the data as well, then use :meth:`related_text_with_data()` instead.

        This method effectively combines `related_text()` with `test_data_about()` on its results, into a single method.

        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections
        limit: Optional[int] = None
            The maximum number of results to return (default: unlimited)

        See :meth:`find_data_about` for possible keyword arguments to filter on data.
        """


class TextSelectionOperator:
    """
    The TextSelectionOperator, simply put, allows comparison of two :class:`TextSelection' instances. It
    allows testing for all kinds of spatial relations (as embodied by this enum) in which two
    :class:`TextSelection` instances can be.
   
    Rather than operator on single :class:`TextSelection` instances, te implementation goes a bit
    further and can act also on the basis of multiple :class:`TextSelection` as a set;
    allowing you to compare two sets, each containing possibly multiple TextSelections, at once.

    The operator is instantiated via one of its static methods.
    """

    @staticmethod
    def equals(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) occupy cover the exact same TextSelections, and all are covered (cf. textfabric's `==`), commutative, transitive

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def overlaps(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) overlap.
        Each TextSelection in A overlaps with a TextSelection in B (cf. textfabric's `&&`), commutative
        If modifier `all` is set: Each TextSelection in A overlaps with all TextSelection in B (cf. textfabric's `&&`), commutative

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def embeds(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) are embedded.
        All TextSelections in B are embedded by a TextSelection in A (cf. textfabric's `[[`)
        If modifier `all` is set: All TextSelections in B are embedded by all TextSelection in A (cf. textfabric's `[[`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """


    @staticmethod
    def embedded(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) are embedded.
        All TextSelections in B are embedded by a TextSelection in A (cf. textfabric's `[[`)
        If modifier `all` is set: All TextSelections in B are embedded by all TextSelection in A (cf. textfabric's `[[`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def before(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if one textselection(sets) comes before another
        Each TextSelections in A comes before a textselection in B
        If modifier `all` is set: All TextSelections in A come before all textselections in B. There is no overlap (cf. textfabric's `<<`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def after(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if one textselection(sets) comes after another
        Each TextSeleciton In A  comes after a textselection in B
        If modifier `all` is set: All TextSelections in A come after all textselections in B. There is no overlap (cf. textfabric's `>>`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def precedes(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if one textselection(sets) is to the immediate left (precedes) of another
        Each TextSelection in A is ends where at least one TextSelection in B begins.
        If modifier `all` is set: The rightmost TextSelections in A end where the leftmost TextSelection in B begins  (cf. textfabric's `<:`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def succeeds(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if one textselection(sets) is to the immediate right (succeeds) of another
        Each TextSelection in A is begis where at least one TextSelection in A ends.
        If modifier `all` is set: The leftmost TextSelection in A starts where the rightmost TextSelection in B ends  (cf. textfabric's `:>`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def samebegin(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) have the same begin position
        Each TextSelection in A starts where a TextSelection in B starts
        If modifier `all` is set: The leftmost TextSelection in A starts where the leftmost TextSelection in B start  (cf. textfabric's `=:`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

    @staticmethod
    def sameend(all: Optional[bool] = False, negate: Optional[bool] = False) -> TextSelectionOperator:
        """
        Create an operator to test if two textselection(sets) have the same end position
        Each TextSelection in A ends where a TextSelection in B ends
        If modifier `all` is set: The rightmost TextSelection in A ends where the rights TextSelection in B ends  (cf. textfabric's `:=`)

        Parameters
        -----------------
        all: Optional[bool]
            If this is set, then for each `TextSelection` in A, the relationship must hold with **ALL** of the text selections in B. The normal behaviour, when this is set to false, is a match with any item suffices (and may be returned).
        negate: Optional[bool] 
            Inverses the operator (turns it into a negation).
        """

class StamError(Exception):
    """STAM Error"""
