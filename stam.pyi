from __future__ import annotations

from typing import Iterator, List, Optional, Union, Iterable

"""
[STAM](https://github.com/annotation/stam) is a standalone data model for stand-off text
annotation and described in detail [here](https://github.com/annotation/stam).
This is a python library (to be more specific; a python binding written in
Rust) to work with the model.

**What can you do with this library?**

* Keep, build and manipulate an efficient in-memory store of texts and annotations on texts
* Search in annotations, data and text, either programmatically or via the [STAM Query Language](https://github.com/annotation/stam/tree/master/extensions/stam-query).
    * Search annotations by data, textual content, relations between text fragments (overlap, embedding, adjacency, etc),
    * Search in text (incl. via regular expressions) and find annotations targeting found text selections.
    * Search in data (set,key,value) and find annotations that use the data.
    * Elementary text operations with regard for text offsets (splitting text on a delimiter, stripping text).
    * Convert between different kind of offsets (absolute, relative to other structures, UTF-8 bytes vs unicode codepoints, etc)
* Read and write resources and annotations from/to STAM JSON, STAM CSV, or an optimised binary (CBOR) representation
    * The underlying [STAM model](https://github.com/annotation/stam) aims to be clear and simple. It is flexible and 
      does not commit to any vocabulary or annotation paradigm other than stand-off annotation.

This STAM library is intended as a foundation upon which further applications
can be built that deal with stand-off annotations on text. We implement all 
the low-level logic in dealing this so you no longer have to and can focus on your 
actual application.
"""

class AnnotationStore:
    """
    An Annotation Store is a collection of annotations, resources and
    annotation data sets. It can be seen as the *root* of the *graph model* and the glue
    that holds everything together. It is the entry point for any stam model.
    """
    def __init__(self, id=None,file=None, string=None,config=None) -> None:
        """
        To instantiate an AnnotationStore, at least one of `id`, `file` or `string` must be specified as keyword arguments:
        
        Keyword Arguments
        --------------------

        id: Optional[str], default: None
            The public ID for a *new* store. Only specify this if you want to create a new store, rather than load an existing one.
        file: Optional[str], default: None
            The STAM JSON, STAM CSV or STAM CBOR file to load, or if used in combination with `id`, the filename for the new store.
        string: Optional[str], default: None
            STAM JSON as a string
        config: Optional[dict]
            A python dictionary containing configuration parameters:

            * use_include: Optional[bool], default: True
                Use the `@include` mechanism to point to external files, if unset, all data will be kept in a single STAM JSON file.
            * workdir: Optional[str]
                Set the working directory, all relative filenames (also for `@include`) will be interpreted relative to this.
            * debug: Optional[bool], default: False
                Enable debug mode, outputs extra information to standard error output (verbose!)
            * annotation_annotation_map: Optional[bool], default: True
                Enable/disable index for annotations that reference other annotations
            * resource_annotation_map: Optional[bool], default: True
                Enable/disable reverse index for TextResource => Annotation. Holds only annotations that **directly** reference the TextResource (via a ResourceSelector), i.e. metadata
            * dataset_annotation_map: Optional[bool], default: True
                Enable/disable reverse index for AnnotationDataSet => Annotation. Holds only annotations that **directly** reference the AnnotationDataSet (via DataSetSelector), i.e. metadata
            * key_annotation_metamap: Optional[bool], default: True
                Enable/disable reverse index for DataKey  => Annotation. Holds only annotations that **directly** reference the DataKey (via DataKeySelector), i.e. metadata
            * data_annotation_metamap: Optional[bool], default: True
                Enable/disable reverse index for AnnotationData  => Annotation. Holds only annotations that **directly** reference the AnnotationData (via AnnotationDataSelector), i.e. metadata
            * textrelationmap: Optional[bool], default: True
                Enable/disable the reverse index for text, it maps TextResource => TextSelection => Annotation
            * generate_ids: Optional[bool], default: False
                Generate pseudo-random public identifiers when missing (during deserialisation). Each will consist of 21 URL-friendly ASCII symbols after a prefix of A for Annotations, S for DataSets, D for AnnotationData, R for resources
            * strip_temp_ids: Optional[bool], default: True
                Strip temporary IDs during deserialisation. Temporary IDs start with an exclamation mark, a capital ASCII letter denoting the type, and a number
            * shrink_to_fit: Optional[bool], default: True
                Shrink data structures to optimize memory (at the cost of longer deserialisation times)
            * milestone_interval: Optional[int], default: 100
                Milestone placement interval (in unicode codepoints) in indexing text resources. A low number above zero increases search performance at the cost of memory and increased initialisation time.


        Example
        ---------

        Load a store from file::

            store = AnnotationStore(file="hamlet.store.json")

        Instantiate a store from scratch and populate it with a resource and annotation::

            self.store = AnnotationStore(id="test")
            resource = self.store.add_resource(id="testres", text="Hello world")
            self.store.annotate(id="A1", 
                                target=Selector.textselector(resource, Offset.simple(6,11)),
                                data={ "id": "D1", "key": "pos", "value": "noun", "set": "testdataset"})
        """

    def id(self) -> Optional[str]:
        """Returns the public identifier (by value, aka a copy)"""

    def to_file(self, filename: str) -> None:
        """Saves the annotation store to file. Use either .json or .csv as extension."""

    def from_file(self, filename: str) -> None:
        """Load another annotation store (only in STAM JSON format currently) into the current one. This can be done multiple times and effectively merges annotations stores."""

    def save(self) -> None:
        """Saves the annotation store to the same file it was loaded from or last saved to."""

    def to_json_string(self) -> str:
        """Returns the annotation store as one big STAM JSON string"""

    def dataset(self, id: str) -> AnnotationDataSet:
        """Basic retrieval method that returns an :class:`AnnotationDataSet` by ID. Raises an exception if not found."""

    def annotation(self, id: str) -> Annotation:
        """Basic retrieval method that returns an :class:`Annotation` by ID. Raises an exception if not found."""

    def resource(self, id: str) -> TextResource:
        """Basic retrieval method that returns a :class:`TextResource` by ID. Raises an exception if not found."""

    def key(self, set_id: str, key_id: str) -> DataKey:
        """Shortcut retrieval method that returns an :class:`DataKey` by ID. Raises an exception if not found."""

    def annotationdata(self, set_id: str, data_id: str) -> AnnotationData:
        """Shortcut retrieval method that returns an :class:`AnnotationData` by ID"""

    def add_resource(self, filename: Optional[str] = None, text: Optional[str] = None, id: Optional[str] = None) -> TextResource:
        """Create a new :class:`TextResource` and add it to the store. Returns the added instance.

        If you want to store the resource as a stand-off text file, you can specify a filename. Make sure to set `use_include = True` in the Annotation Store's configuration then.
        Note that any relative paths will be interpreted relative to the directory the current (root) store is in.
        """

    def add_dataset(self, id: Optional[str] = None, filename: Optional[str] = None) -> AnnotationDataSet:
        """Create a new :class:`AnnotationDataSet` and add it to the store. Returns the added instance.

        If you want to store the dataset as a stand-off JSON file, you can specify a filename. The dataset will be loaded from file if it exists. Make sure to set `use_include = True` in the Annotation Store's configuration then.
        Note that any relative paths will be interpreted relative to the directory the current (root) store is in.
        """

    def add_substore(self, filename: str) -> AnnotationSubStore:
        """
        Load an existing annotation store as a dependency to this one. It will be store in an stand-off JSON file and included using the @include mechanism.
        Note that any relative paths will be interpreted relative to the directory the current (root) store is in.
        Returns the added substore.
        """

    def add_new_substore(self, id: str, filename: str) -> AnnotationSubStore:
        """
        Add a new empty annotation store as a dependency to this one.
        It will be stored in an stand-off JSON file and included using the @include mechanism.
        Note that any relative paths will be interpreted relative to the directory the current (root) store is in.
        Returns the added substore.
        """

    def set_filename(self, filename: str) -> None:
        """Set the filename for the annotationstore, the format is derived from the extension, can be `.json` or `csv`. This may be also be a full absolute or relative path."""
    
    def annotate(self, target: Selector, data: Union[dict,List[dict],AnnotationData,List[AnnotationData]], id: Optional[str] = None) -> Annotation:
        """Adds a new annotation. Returns the :obj:`Annotation` instance that was just created.
        
        Parameters
        -------------
            
        target: :class:`Selector`
            A target selector that determines the object of annotation
        data: Union[dict,List[dict],AnnotationData,List[AnnotationData]]
            A dictionary or list of dictionaries with data to set. The dictionary
            may have fields: `id` (optional),`key`,`set`, and `value`.
            Alternatively, you can pass an existing :class:`AnnotationData` instance.
        id: Optional[str]
            The public ID for the annotation. If unset, one may be autogenerated if this was
            explicitly enabled in the configuraiton.


        Example
        -----------

        Instantiate a store from scratch and populate it with a resource and annotation::

            self.store.annotate(id="A1", 
                                target=Selector.textselector(store.resource("testres"), Offset.simple(6,11)),
                                data={ "id": "D1", "key": "pos", "value": "noun", "set": "testdataset"})
        """

    def __iter__(self) -> Iterator[Annotation]:
        """Returns an iterator over all annotations (:class:`Annotation`) in this store.

        This iterator has little runtime overhead but does not provide any filtering options, use :meth:`annotations` instead if you plan to do any filtering, 
        or use the equally named method on other objects for more constrained and filterable annotations (e.g. :meth:`DataKey.annotations`, :meth:`AnnotationDataSet.annotations`, :meth:`TextResource.annotations`)
        """

    def annotations(self, *args, **kwargs) -> Annotations:
        """Returns an iterator over all annotations (:class:`Annotation`) in this store.

        Filtering can be applied using positional arguments and/or keyword arguments. It is recommended to only use this method if you apply further filtering, otherwise the memory overhead may be very large if you have many annotations.
        Otherwise you can fall back to a more low-level iterator, :meth:`__iter__` instead

        Parameters
        --------------

        *args: tuple, optional
            Filter arguments. These can any be of the following types:

            * :class:`DataKey`
                Returns annotations with data matching this key.
            * :class:`AnnotationData`
                Returns only annotations that have this exact data.
            * :class:`Annotations` | [:class:`Annotation`]
                Returns only annotations that match any of those specified here. 
            * :class:`Data` | [:class:`AnnotationData`]
                Returns only annotations with data  matching any of those specified here. 
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** - (see keyword arguments below)
        **kwargs: dict, optional
            * limit: (Optional[int] = None)
                The maximum number of results to return (default: unlimited)
            * substore: (Optional[bool] = None)
                Set this to False if you want to include only results from the root store and not from any substores (default: True)
            * set: (Optional[Union[str,AnnotationDataSet]] = None)
                An ID of a dataset (or an :class:`AnnotationDataSet` instance), only needed when specifying `key` as a string
            * key: (Optional[Union[str,DataKey]] = None)
                An ID of a key (or a :class:`DataKey` instance), make sure to specify `set` as well if you use a string value for this parameter.
            * value: (Optional[Union[str,int,float,bool]])
                Constrain the search to annotations with data of a certain value. This can only be used when you also pass a :class:`DataKey` as filter.
                This holds the exact value to search for, there are other variants of this keyword available, see :meth:`data` for a full list. 
        """



    def datasets(self) -> Iterator[AnnotationDataSet]:
        """Returns an iterator over all annotation data sets (:class:`AnnotationDataSet`) in this store"""

    def resources(self) -> Iterator[TextResource]:
        """Returns an iterator over all text resources (:class:`TextResource`) in this store"""

    def substores(self) -> Iterator[AnnotationSubStore]:
        """Returns an iterator over all substores (:class:`AnnotationSubStore`) in this store, i.e. stores that are included by this one as dependencies"""

    def annotations_len(self) -> int:
        """Returns the number of annotations in the store (not substracting deletions)"""

    def datasets_len(self) -> int:
        """Returns the number of annotation data sets in the store (not substracting deletions)"""

    def resources_len(self) -> int:
        """Returns the number of text resources in the store (not substracting deletions)"""

    def substores_len(self) -> int:
        """Returns the number of substores in the store"""

    def shrink_to_fit(self):
        """Reallocates internal data structures to tight fits to conserve memory space (if necessary). You can use this after having added lots of annotations to possibly reduce the memory consumption."""

    def data(self, *args, **kwargs) -> Data:
        """Returns an iterator over all data (:class:`AnnotationData`) in this store.

        Filtering can be applied using positional arguments and/or keyword arguments. It is recommended to only use this method if you apply further filtering, otherwise the memory overhead may be very large if you have a lot of data.

        Parameters
        -------------

        *args: tuple, optional
            Filter arguments, these can be of the following types: 

            * :class:`DataKey`
                Returns data matching this key
            * :class:`Annotation`
                Returns data referenced by the mentioned annotation 
            * :class:`AnnotationData`
                Returns only this exact data. Not very useful, use :meth:`test_data` instead.
            * :class:`Annotations` | [class:`Annotation`]
                Returns data references by annotations in the provided  collection.
            * :class:`Data` | [class:`AnnotationData`]
                Returns only data that is in the provided :obj:`Data` collection (intersection)
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** or variants (see keyword arguments below)
        **kwargs: dict, optional
            * limit: `Optional[int] = None`
                The maximum number of results to return (default: unlimited)
            * set: `Optional[Union[str,AnnotationDataSet]] = None`
                An ID of a dataset (or an :class:`AnnotationDataSet` instance), only needed when specifying `key` as a string
            * key: `Optional[Union[str,DataKey]] = None`
                An ID of a key (or a :class:`DataKey` instance), make sure to specify `set` as well if you use a string value for this parameter.
            * value: `Optional[Union[str,int,float,bool,List[Union[str,int,float,bool]]]]`
                Search for data matching a specific value.
                This holds exact value to search for. Further variants of this keyword are listed below:
            * value_not: `Optional[Union[str,int,float,bool]]`
                Value must not match
            * value_greater: `Optional[Union[int,float]]`
                Value must be greater than specified (int or float)
            * value_less: `Optional[Union[int,float]]`
                Value must be less than specified (int or float)
            * value_greatereq: `Optional[Union[int,float]]`
                Value must be greater than specified or equal (int or float)
            * value_lesseq: `Optional[Union[int,float]]`
                Value must be less than specified or equal (int or float)
            * value_in: `Optional[Tuple[Union[str,int,float,bool]]]`
                Value must match any in the tuple (this is a logical OR statement)
            * value_not_in: `Optional[Tuple[Union[str,int,float,bool]]]`
                Value must not match any in the tuple
            * value_in_range: `Optional[Tuple[Union[int,float]]]`
                Must be a numeric 2-tuple with min and max (inclusive) values
        """

    def query(self, query: str, **kwargs) -> list:
        """
        Query the data using STAMQL.

        Parameters
        --------------

        query: str
            Query in `STAMQL <https://github.com/annotation/stam/tree/master/extensions/stam-query>`_.
            Note that you *MUST* specify a variable to bind to in your `SELECT`
            statement (this is normally optional but is required for calling from
            Python).
        **kwargs: tuple, optional
            You can bind extra context variables using keyword arguments. The keys
            correspond to the variable names that these will be bound to and which
            you can subsequently use in the STAMQL query. These keys 
            should not carry the '?' prefix you may be accustomed to in STAMQL. The
            value must be instances of STAM objects such as :class:`Annotation`,
            :class:`AnnotationData`, :class:`DataKey`, :class`TextSelection` etc. These context variables
            are available to the query but not propagated to the output.

       Keyword arguments
       -------------------

       readonly: Optional[bool]
            If set to `True`, queries that would mutate the store are rejected (raise an Exception).
            In other words, only `SELECT` statements are allowed then.

        A query returns a list consisting of dictionaries, each corresponding one
        result row. The keys in the dictionaries match with the variable names
        in the STAMQL query, the values are result instances of whatever type
        the query returns, i.e. Annotation, AnnotationData, TextResource,
        TextSelection, AnnotationDataSet.

        Examples
        --------------

        Query for annotations with certain kind of data::

            for row in store.query('SELECT ANNOTATION ?a WHERE "some-set" "pos" = "noun";'):
                for result in row:
                    #just print out the text of the annotation
                    print(str(result['a']))
        """

    def remove(self, item: Union[Annotation,AnnotationDataSet,TextResource, AnnotationData,DataKey], **kwargs):
       """
       Remove any STAM item from the store.

       Keyword arguments
       -------------------
       strict: Optional[bool]
            In strict mode, any annotation that uses this item (where item is `AnnotationData` or `DataKey`) will be removed entirely, otherwise the annotation will be modified to remove the reference only.
       """

    def view(self, selectionquery, *args, **kwargs) -> str:
        """
        Execute a selection query and zero or more highlight queries, and visualise the result as HTML or ANSI-colored text.
        The results are returned as a string.

        Arguments
        ---------------

        selectionquery: str
            The main selection query in STAMQL. This selects what will be shown.

        *args: List[str]
           Each positional argument is a highlight query in STAMQL. It determines what portions of the results will be highlighted and how (attributes are support)

        Keyword arguments
        -----------------------

        format: Optional[str]
            The format to use, can be either `html` (default) or `ansi`.
        legend: Optional[bool]
            Show legend or not?
        titles: Optional[bool]
            Show titles or not (per result)
        use: Optional[str]
            The variable to use for the main selection (if not set, the first will be used) 
        interactive: Optional[bool]
            Output is slightly interactive (html only, insert some minimal javascript)
        autocollapse: Optional[bool]
            Collapse all tags on initial view (html only)
        """

    def split(self, queries: List[str], retain: bool):
        """
        Splits an annotation store by either retaining (if `retain == True`) or removing (if `retain == False`) the items selected by the queries.
        Queries must be STAMQL queries that select annotations, resources or datasets.
        This deletes items from the store along with all their dependencies and comes with a reasonable performance overhead.
        """

    def align_texts(self, *args: list[tuple[TextSelection,TextSelection]], **kwargs) -> list[list[Annotation]]:
        """
        Used to compute an alignment between two texts; it
        identifies which parts of the two texts are identical and computes a mapping
        between the two coordinate systems. Two related sequence alignments algorithms
        from bioinformatics are implemented to accomplish this:
        Smith-Waterman and Needleman-Wunsch.

        The resulting alignment is added to the store as an annotation, a so called transposition,
        according to the `STAM Transpose <https://github.com/annotation/stam/tree/master/extensions/stam-transpose>`_
        extension. These annotations are also returned by this function.

        Alignments between text selection pairs will be computed in parallel, it may be memory intensive.
        For the simpler sequential variant, use :meth:`TextSelection.align_texts()` instead.

        Positional Arguments
        -------------------

        Each argument is a two-tuple containing two text selections (:class:`TextSelection`) to align.

        Keyword Arguments
        -------------------
        
        case_insensitive: bool
            Case-insensitive matching has more performance overhead
        algorithm: str
            The alignment algorithm to use, can be `smithwaterman`/`local` (local alignment) or `needlemanwunsch`/`global` (global alignment).
        grow: bool
            Grow aligned parts into larger alignments by incorporating non-matching parts. If you set this, 
            the function will return translations rather than transpositions.
    		You'll want to set `max_errors` in combination with this one to prevent very low-quality alignments.
        max_errors: Union[int,float]
    		The maximum number of errors (max edit distance) that may occur for a transposition to be valid.
    		This is either an absolute integer or a relative ratio between 0.0 and 1.0, interpreted in relation to the length of the first text in the alignment.
    		In other words; this represents the number of characters in the search string that may be missed when matching in the larger text.
    		The transposition itself will only consist of fully matching parts, use `grow` if you want to include non-matching parts.
        minimal_align_length: int
    		The minimal number of characters that must be aligned (absolute number) for a transposition/translation to be valid.
        annotation_id_prefix: str
            Prefix to use when assigning annotation IDs. The actual ID will have a random component
        trim: bool
            Strip leading and trailing whitespace/newlines from aligned text selections, keeping them as minimal as possible (default is to be as greedy as possible in selecting)
            Setting this may lead to certain whitespaces not being covered even though they may align.
        simple_only: bool
            Only allow for alignments that consist of one contiguous text selection on either side. This is a so-called simple transposition.
        """




#   def find_data(self,  **kwargs) -> Data:
#       """
#       Find annotation data by set, key and value.
#       Returns :class:`Data`, which holds a collection of :class:`AnnotationData` instances.

#       Keyword arguments
#       -------------------

#       set: Optional[Union[str,AnnotationDataSet]]
#           The set to search for; it will search all sets if not specified
#       key: Optional[Union[str,DataKey]]
#           The key to search for; it will search all keys if not specified. If you specify a key, you must also specify a set!
#       value: Optional[Union[str,int,float,bool]]
#           The exact value to search for, if this or any of its variants mentioned below is omitted, it will search all values.
#       value_not: Optional[Union[str,int,float,bool]]
#           Value
#       value_greater: Optional[Union[int,float]]
#           Value must be greater than specified (int or float)
#       value_less: Optional[Union[int,float]]
#           Value must be less than specified (int or float)
#       value_greatereq: Optional[Union[int,float]]
#           Value must be greater than specified or equal (int or float)
#       value_lesseq: Optional[Union[int,float]]
#           Value must be less than specified or equal (int or float)
#       value_in: Optional[Tuple[Union[str,int,float,bool]]]
#           Value must match any in the tuple (this is a logical OR statement)
#       value_not_in: Optional[Tuple[Union[str,int,float,bool]]]
#           Value must not match any in the tuple
#       value_in_range: Optional[Tuple[Union[int,float]]]
#           Must be a numeric 2-tuple with min and max (inclusive) values


#       Examples
#       -------------

#       Query for specific annotation data::

#           for annotationdata in store.find_data(set="some-set", key="structuretype", value="word"):
#               # only returns one
#               ...

#       Query for all data for a key::

#           for annotationdata in store.find_data(set="some-set", key="structuretype"):
#               ...

#       Note, the latter can be accomplished more efficiently as::

#           for annotationdata in store.dataset("some-set").key("structuretype").data():
#               ...

#       `find_data` should be considered as a convenience/shortcut method.
#       """


class Annotation:
    """
    `Annotation` represents a particular *instance of annotation* and is the central
    concept of the model. Annotations can be considered the primary nodes of the graph model. The
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

    def has_id(self, id: str) -> bool:
        """Tests the ID"""

    def __iter__(self) -> Iterator[AnnotationData]:
        """Returns a iterator over all data (:class:`AnnotationData`) in this annotation; this has little overhead but is less suitable if you want to do further filtering, use :meth:`data` instead for that."""

    def __len__(self) -> int:
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

    def textselections(self, **kwargs) -> TextSelections:
        """
        Returns a collection of all textselections (:class:`TextSelection`) referenced by the annotation (i.e. via a *TextSelector*).
        Note that this will always return a collection (even it if only contains a single element),
        as an annotation may reference multiple text selections. 

        Text selections will be returned in textual order, except if a DirectionalSelector was used.

        Text selections may be filtered using the following positionl and/or keyword arguments:

        Parameters
        -------------------

        *args: tuple, optional
            Filter arguments, can be of the following types:

            * :class:`DataKey`
                Returns text selections referenced by annotations with data matching this key
            * :class:`AnnotationData`
                Returns text selections referenced by annotations that have this exact data
            * :class:`Annotations` | [:class:`Annotation`]
                Returns text selections referenced by any annotations that are already in the provided :obj:`Annotations` collection (intersection)
            * :class:`Data` | [:class:`AnnotationData`]
                Returns only textselections referenced by annotations with data that is in the provided  collection.
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** (see keyword arguments below)
        **kwargs: dict, optional
            limit: Optional[int] = None
                The maximum number of results to return (default: unlimited)
            value: Optional[Union[str,int,float,bool]]
                Constrain the search to text selections referenced by annotations with data of a certain value. This is usually used together with passing a :obj:`DataKey` as filter in the positional arguments.
                This holds the exact value to search for, there are other variants of this keyword available, see :meth:`data` for a full list. 
        """

    def annotations_in_targets(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing
        :class:`Annotation` instances) this annotation refers to (i.e. using an
        *AnnotationSelector*)

        The annotations can be filtered using positional and/or keyword
        arguments; see :meth:`annotations` for full documentation. One extra keyword argument is
        available for this method (see below).

        Annotations will returned be in textual order unless recursive is set
        or a DirectionalSelector is involved.

        Keyword Arguments
        -------------------

        recursive: bool
            Follow AnnotationSelectors recursively (default False)
        """

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing
        :class:`Annotation` instances) that are referring to this annotation (i.e. others
        using an AnnotationSelector).

        The annotations can be filtered using positional and/or keyword
        arguments.

        Parameters
        -----------

        *args: tuple, optional
            These arguments can any be of the following types:

            * :class:`DataKey`
                Returns annotations with data matching this key.
            * :class:`AnnotationData`
                Returns only annotations that have this exact data.
            * :class:`Annotations` | :class:`Annotation`
                Returns only annotations that match any of those specified here. 
            * :class:`Data` | :class:`AnnotationData`
                Returns only annotations with data  matching any of those specified here. 
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** - (see keyword arguments below)
        **kwargs: dict, optional
            * limit: (Optional[int] = None)
                The maximum number of results to return (default: unlimited)
            * set: (Optional[Union[str,AnnotationDataSet]] = None)
                An ID of a dataset (or an :class:`AnnotationDataSet` instance), only needed when specifying `key` as a string
            * key: (Optional[Union[str,DataKey]] = None)
                An ID of a key (or a :class:`DataKey` instance), make sure to specify `set` as well if you use a string value for this parameter.
            * value: (Optional[Union[str,int,float,bool]])
                Constrain the search to annotations with data of a certain value. This can only be used when you also pass a :class:`DataKey` as filter.
                This holds the exact value to search for, there are other variants of this keyword available, see :meth:`data` for a full list. 
            * limit: (Optional[int] = None)
                The maximum number of results to return (default: unlimited)

        Example
        ---------

        Filter by data key and value::

            key = store.dataset("linguistic-set").key("part-of-speech")
            for annotation in store.annotations(key, value="noun"):
                 ...

        But if you already have the key, like in the example above, you may just as well do (more efficient)::

            for annotation in key.annotations(value="noun"):
                 ...
        """

    def test_annotations(self, *args, **kwargs) -> bool:
        """
        Tests whwther there are annotations (:class:`Annotations` containing :class:`Annotation`) that are referring to this annotation (i.e. others using an AnnotationSelector).
        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using keyword arguments. See :meth:`Annotation.annotations`.

        Example
        ---------

        Filter by data key and value::

            key = store.dataset("linguistic-set").key("part-of-speech")
            for annotation in store.annotations_in_targets(filter=key, value="noun"):
                 ...
       """

    def resources(self, limit: Optional[int] = None) -> List[TextResource]:
        """Returns a list of resources (:class:`TextResource`) this annotation refers to

        Parameters
        ------------

        `limit`: `Optional[int] = None`
            The maximum number of results to return (default: unlimited)
        """

    def datasets(self, limit: Optional[int] = None) -> List[AnnotationDataSet]:
        """Returns a list of annotation data sets (:class:`AnnotationDataSet`) this annotation refers to. This only returns the ones
        referred to via a *DataSetSelector*, i.e. as metadata.

        Parameters
        ------------

        `limit`: `Optional[int] = None`
            The maximum number of results to return (default: unlimited)
        """

    def offset(self) -> Optional[Offset]:
        """Returns the offset this annotation's selector targets, exactly as specified"""

    def target(self) -> Selector:
        """Returns the target selector (:class:`Selector`) for this annotation. This is mainly useful if you want to add another annotation pointing to the same target."""

    def selector_kind(self) -> SelectorKind:
        """Returns the type of the selector of this annotation"""

    def data(self, *args, **kwargs) -> Data:
        """
        Returns annotation data (:class:`Data` containing :class:`AnnotationData`) used by this annotation.

        The data can be filtered using keyword arguments. If you don't care for any filtering and just want a simple iterator overlap
        the data, then just iterating over the annotation directly (:meth:`__iter__`) will be more efficient. Do note that implementing
        any filtering yourself in Python is much less performant than letting this data method do it for you.

        Parameters
        -------------

        *args: tuple, optional
            Filter arguments, these can be of the following types: 

            * :class:`DataKey`
                Returns data matching this key
            * :class:`Annotation`
                Returns data referenced by the mentioned annotation 
            * :class:`AnnotationData`
                Returns only this exact data. Not very useful, use :meth:`test_data` instead.
            * :class:`Annotations` | [class:`Annotation`]
                Returns data references by annotations in the provided  collection.
            * :class:`Data` | [class:`AnnotationData`]
                Returns only data that is in the provided :obj:`Data` collection (intersection)
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** or variants (see keyword arguments below)
        **kwargs: dict, optional
            * limit: `Optional[int] = None`
                The maximum number of results to return (default: unlimited)
            * set: `Optional[Union[str,AnnotationDataSet]] = None`
                An ID of a dataset (or an :class:`AnnotationDataSet` instance), only needed when specifying `key` as a string
            * key: `Optional[Union[str,DataKey]] = None`
                An ID of a key (or a :class:`DataKey` instance), make sure to specify `set` as well if you use a string value for this parameter.
            * value: `Optional[Union[str,int,float,bool,List[Union[str,int,float,bool]]]]`
                Search for data matching a specific value.
                This holds exact value to search for. Further variants of this keyword are listed below:
            * value_not: `Optional[Union[str,int,float,bool]]`
                Value must not match
            * value_greater: `Optional[Union[int,float]]`
                Value must be greater than specified (int or float)
            * value_less: `Optional[Union[int,float]]`
                Value must be less than specified (int or float)
            * value_greatereq: `Optional[Union[int,float]]`
                Value must be greater than specified or equal (int or float)
            * value_lesseq: `Optional[Union[int,float]]`
                Value must be less than specified or equal (int or float)
            * value_in: `Optional[Tuple[Union[str,int,float,bool]]]`
                Value must match any in the tuple (this is a logical OR statement)
            * value_not_in: `Optional[Tuple[Union[str,int,float,bool]]]`
                Value must not match any in the tuple
            * value_in_range: `Optional[Tuple[Union[int,float]]]`
                Must be a numeric 2-tuple with min and max (inclusive) values
            * limit: `Optional[int] = None`
                The maximum number of results to return (default: unlimited)

        Example
        -----------

        Get all part-of-speech data pertaining to this annotation::

            key = store.dataset("linguistic-set").key("part-of-speech")
            for data in annotation.data(filter=key):
                ...

        """

    def test_data(self, *args, **kwargs) -> bool:
        """
        Tests whether certain annotation data is used by this annotation.
        The data can be filtered using positional and/or keyword arguments. See :meth:`data`.
        Unlike :meth:`data`, this method merely tests without returning the data, and as such is more performant.
        """

    def related_text(self, operator: TextSelectionOperator, *args, **kwargs) -> TextSelections:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the ones from the current annotation. 
        Returns a collection :class:`TextSelections` containing all matching :class:`TextSelection` instances.

        Text selections will be returned in textual order. They may be filtered via positional and/or keyword arguments. See :meth:`Annotation.textselections`.
       
        If you are interested in the annotations associated with the found text selections, then
        add `.annotations()` to the result.

        Parameters
        ------------

        `operator`: :class:`TextSelectionOperator`
            The operator to apply when comparing text selections


        Keyword Arguments
        -------------------

        `limit`: `Optional[int] = None`
            The maximum number of results to return (default: unlimited)


        See :meth:`Annotation.textselections` for further keyword arguments to filter.

        Examples
        -------------

        Find all text selections that overlap with the annotation::

            for textselection in annotation.related_text(TextSelectionOperator.overlaps()):
                ...

        If you want to get the annotations instead, just add ``.annotations()``::

            for annotations in annotation.related_text(TextSelectionOperator.overlaps()).annotations():
                ...

        Assume `sentence` is an annotation representing a sentence, we can find text selections inside (embedded in) the sentence as follows::

            for textselection in sentence.related_text(TextSelectionOperator.embeds()):
                ...

        Like above, but now we actively look for annotations that are marked as words, effectively selecting
        all words in a sentence::

            data_word = store.dataset("structural-set").key("type").data(value="word", limit=1)[0]
            for word in sentence.related_text(TextSelectionOperator.embeds()).annotations(filter=data_word):
                ...
        """

    def json(self) -> str:
        """Returns the annotation as STAM JSON in a string with appropriate pretty-print formatting."""


    def webannotation(self, **kwargs) -> str:
        """
        Returns the annotation as a W3C Web Annotation in JSON-LD, as a compact single-line string without pretty formatting (immediately usable for output to JSONL).

        Keywords Arguments
        --------------------

        `default_annotation_iri`: `str`
            IRI prefix for Annotation Identifiers. Will be prepended if the annotations public ID is not an IRI yet.

        `generate_annotation_iri`: `bool`
            Generate a random annotation IRI if it does not exist yet? (non-deterministic!)

        `default_set_iri`: `str`
            IRI prefix for Annotation Data Sets. Will be prepended if the annotation data set public ID is not an IRI yet.

        `default_resource_iri`: `str`
            IRI prefix for Text Resources. Will be prepended if the resource public ID is not an IRI yet.

        'extra_context`: `[str]`
            Extra JSON-LD context to export, these must be URLs to JSONLD files.

        `auto_generated`: `bool`
            Automatically add a 'generated' triple for each annotation, with the timestamp of serialisation

        `auto_generator`: `bool`
            Automatically add a 'generator' triple for each annotation, with the software details

        `context_namespaces`: `[(str,str)]`
            Automatically generate a JSON-LD context alias for all URIs in keys, maps URI prefixes to namespace prefixes
        """

    def test(self, operator: TextSelectionOperator, other: Annotation) -> bool:
        """
        This method is called to test whether a specific spatial relation (as expressed by the
        passed operator) holds between an :class:`Annotation` and another.
        A boolean is returned with the test result.
        """

    def test_textselection(self, operator: TextSelectionOperator, other: TextSelection) -> bool:
        """
        This method is called to test whether a specific spatial relation (as expressed by the
        passed operator) holds between an :class:`Annotation` and a :class:`Textselection`.
        A boolean is returned with the test result.
        """

    def transpose(self, via: Annotation, **kwargs) -> Annotations:
        """
       The transpose function maps an annotation, textselection, or textselection set from
       one coordinate system to another. These mappings are defined in annotations called
       **transpositions** and are documented here: https://github.com/annotation/stam/blob/master/extensions/stam-transpose/README.md
       Transpositions link identical textual parts across resources, any annotations within
       the bounds of such a mapping can then be *transposed* using this function to the other coordinate system.
       
       The `via` parameter expresses the transposition that is being used.
       The result of a transpose operation is itself again a transposition.

       Keyword arguments
       ------------------

       allow_simple: bool    
            Allow a simple transposition as output, by default this is set to `false` as we usually want to have an transposed annotation

       no_transposition: bool
            Do not produce a transposition annotation, only output the transposed annotation (allow_simple must be set to false)
            This effectively throws away the provenance information.

       no_resegmentation: bool
            Do not produce a resegmentation annotation. If needed for a complex transposition, a resegmented annotation is still created, but
            the resegmented version (used as source in the transposition) is not linked to the original source annotation. This effectively throws away provenance information.
            This only comes into play if `no_transposition == False`

       transposition_id: Optional[str]
            An ID to assign to the transposition that is outputted

       resegmentation_id: Optional[str]
            An ID to assign to the resegmentation that is outputted (if any)

       debug: bool
            Output debug information to stderr
       """

    def substore(self) -> Optional[AnnotationSubStore]:
        """
        Returns the substore this annotation is a part of, or `None` if the annotation is part of the root store.
        """

    def alignments(self) -> list[list[Union[TextSelection,Annotation]]]:
        """
        If this annotation describes a transposition (https://github.com/annotation/stam/blob/master/extensions/stam-transpose/README.md), 
        this will extract the alignments in the transposition to a list of lists. Each inner lists hold `TextSelection` instances that are in alignment.
        If you want to return `Annotation` instances instead, set the keyword argument:

        Keyword arguments
        ------------------

        annotations: bool    
           Return annotations instead of text selections, note that this only works for complex transpositions, for simple transpositions you always get text selections regardless of this setting.
        """


class Annotations:
    """
    An `Annotations` object holds an arbitrary collection of annotations. 
    The annotations are references to items in an AnnotationStore, not copies.
    You can iterate over it to retrieve :class:`Annotation` instances.
    """

    def __iter__(self) -> Iterator[Annotation]:
        """Iterator over all annotations in this collection"""

    def __len__(self) -> int:
        """Returns the number of annotations in the collection"""

    def __next__(self) -> Annotation:
        """Return the next item in the iterator"""

    def __getitem__(self, int) -> Annotation:
        """Returns an annotation in the collection by index"""

    def is_sorted(self) -> bool:
        """Returns a boolean indicating whether the annotations in this collection are sorted chronologically (earlier annotations before later once). Note that this is distinct from any textual ordering."""

    def data(self, *args, **kwargs) -> Data:
        """
        Returns annotation data (:class:`Data` containing :class:`AnnotationData`) used by annotations in this collection.

        The data can be filtered using positional and/or keyword arguments; see :meth:`Annotation.data`.
        If no filters are set (default), all data from all annotations are returned (without duplicates).
        """

    def test_data(self, *args, **kwargs) -> bool:
        """
        Tests whether certain annotation data is used by any annotation in this collection.
        The data can be filtered using keyword arguments. See :meth:`data`.
        Unlike :meth:`data`, this method merely tests without returning the data, and as such is more performant.
        """

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that reference annotations in the current collection (e.g. annotations that target of the current any annotations using an AnnotationSelector).

        The annotations can be filtered using positional and/or keyword arguments; see :meth:`Annotation.annotations`.
        If no filters are set (default), all annotations are returned (without duplicates) in chronological order.

        Example
        -----------

        Say `annotation` represents a word, we can get all annotations that with key "part-of-speech", that point to this annotation::

            key = store.dataset("linguistic-set").key("part-of-speech")
            for pos_annotation in annotation.annotations(filter=key):
                data = annotation.data(filter=key,limit=1)[0]
                ...
        """

    def annotations_in_targets(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that are being referenced by annotations in the current collection (e.g. annotations we target using an AnnotationSelector).

        The annotations can be filtered using positional and/or keyword arguments; see :meth:`Annotation.annotations`. One extra keyword argument is available and explained below.
        If no filters are set (default), all annotations are returned (without duplicates). 
        Annotations are returned in chronological order.

        Keyword Arguments
        -------------------

        recursive: bool
            Follow AnnotationSelectors recursively (default False)
        """

    def test_annotation(self, *args, **kwargs) -> bool:
        """
        Tests whether certain annotations reference any annotation in this collection.
        The annotation can be filtered using positional and/or keyword arguments. See :meth:`annotations`.
        Unlike :meth:`annotations`, this method merely tests without returning the data, and as such is more performant.
        """

    def test_annotations_in_targets(self, *args, **kwargs) -> Annotations:
        """
        Tests whether annotations in this collection targets the specified annotation.
        The annotation can be filtered using positional and/or keyword arguments. See :meth:`annotations`.
        Unlike :meth:`annotations_in_targets`, this method merely tests without returning the data, and as such is more performant.
        """

    def textselections(self, limit: Optional[int] = None) -> TextSelections:
        """
        Returns a collection of all textselections associated with the annotations in this collection.
        """

    def related_text(self, operator: TextSelectionOperator, **kwargs) -> TextSelections:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with any from the current collection of annotations. 
        Returns a collection of all matching :class:`TextSelection` instances.

        Text selections will be returned in textual order. They may be filtered via keyword arguments. See :meth:`Annotation.textselections`.

        See :meth:`Annotation.related_text` for allowed parameters/keyword arguments and examples.
        """

    def textual_order(self) -> Annotations:
        """
        Sorts the annotations in textual order (provided they refer to any text at all)

        This has some performance cost, so prevent calling this method on methods like :meth:`Annotation.annotations_in_targets` which already produce textual order (in most cases)
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

    def has_id(self, id: str) -> bool:
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
        """Returns an iterator over all :class:`AnnotationData` in the dataset. If you want to do any filtering, use :meth:`data` instead."""

    def data(self, *args, **kwargs) -> Data:
        """
        Returns annotation data (:class:`Data` containing :class:`AnnotationData`) used by this key.

        The data can be filtered using positional and/or keyword arguments. See :meth:`Annotation.data`. 
        If you don't intend to do any filtering at all, then just using :meth:`__iter__` may be faster.
        """

    def test_data(self, *args, **kwargs) -> bool:
        """
        Tests whether certain annotation data exists in this set.
        The data can be filtered using positional and/or keyword arguments. See :meth:`Annotation.data`.
        This method is like :meth:`data`, but merely tests without returning the data, and as such is more performant.
        """

    def select(self) -> Selector:
        """Returns a selector pointing to this annotation dataset (via a *DataSetSelector*)"""

    def substores(self) -> List[AnnotationSubStore]:
        """
        Returns the substores this dataset is a part of (if any).
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

    def has_id(self, id: str) -> bool:
        """Tests the ID"""

    def dataset(self) -> AnnotationDataSet:
        """Returns the :class:`AnnotationDataSet` this key is part of"""

    def data(self, *args, **kwargs) -> Data:
        """
        Returns annotation data (:class:`Data` containing :class:`AnnotationData`) used by this key.

        The data can be filtered using positional and/or keyword arguments. See :meth:`Annotation.data`. Note that only a subset makes sense in this context, set and key are already fixed.

        Example
        --------

        Assume the key represents part-of-speech tags, get all annotations for value "noun"::

            for data in key.data(value="noun"):
                # returns only one
        """

    def test_data(self, *args, **kwargs) -> bool:
        """
        Tests whether certain annotation data exists for this key
        The data can be filtered using keyword arguments. See :meth:`Annotation.data`. Note that only a subset makes sense in this context, set and key are already fixed.

        This method is like :meth:`data`, but merely tests without returning the data, and as such is more performant.

        Example
        --------

        Assume the key represents part-of-speech tags, get all annotations for value "noun"::

            if key.test_data(value="noun"):
                #value exists
                ...
        """

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that make use of this key.

        The annotations can be filtered on value using keyword arguments. See :meth:`Annotation.annotations`, but note that not all keyword arguments apply in this context (set and key are predetermined already).

        Example
        --------

        Assume the key represents part-of-speech tags, get all annotations for value "noun"::

            for annotation in key.annotations(value="noun"):
                 ...
        """

    def test_annotations(self, *args, **kwargs) -> bool:
        """
        Tests whether there are any annotations that make use of this key.
        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using keyword arguments. See :meth:`Annotation.annotations`.

        Example
        --------

        Assume the key represents part-of-speech tags, test if there are annotations for data value "noun":

            if key.test_annotations(value="noun"):
                 ...
       """

    def annotations_count(self, limit: Optional[int] = None) -> int:
        """Returns the number of annotations (:class:`Annotation`) that use this data.
        Note that this is much faster than doing `len(annotations())`!
        This method has suffix `_count` instead of `_len` because it is not O(1) but does actual counting (O(n) at worst).

        Parameters
        ------------

        `limit`: `Optional[int] = None`
            The maximum number of results to return (default: unlimited)
        """

    def select(self) -> Selector:
        """Returns a selector pointing to this key (DataKeySelector)"""

class DataValue:
    """Encapsulates a value and its type. Held by :class:`AnnotationData`. This type is not a reference but holds the actual value."""


    def get(self) -> Union[str,bool,int,float,List]:
        """Get the actual value"""

    def __init__(self, value: Union[str,bool,int,float,List]) -> None:
        """You can instantiate a new DataValue from a supported Python type, but you usually don't need to do this explicitly."""

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

    def has_id(self, id: str) -> bool:
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

    def dataset(self) -> AnnotationDataSet:
        """Returns the :class:`AnnotationDataSet` this data is part of"""

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that make use of this data.

        The annotations can be filtered using positional and/or keyword arguments.

        Parameters
        -----------

        *args: tuple, optional
            Filter arguments, can any be of the following types:

            * :class:`DataKey`
                Returns annotations with data matching this key.
            * :class:`AnnotationData`
                Returns only annotations that have this exact data.
            * :class:`Annotations` | :class:`Annotation`
                Returns only annotations that match any of those specified here. 
            * :class:`Data` | :class:`AnnotationData`
                Returns only annotations with data  matching any of those specified here. 
            * :class:`dict` with keys:
                * **set** - An ID of a dataset (or a :class:`DataAnnotationSet` instance), only needed when specifying `key` as a string (see below)
                * **key** - A key, either an instance of :class:`DataKey` or a string, in the latter case you need to specify `set` as well.
                * **value** - (see keyword arguments below)
        **kwargs: dict, optional
            * limit: (Optional[int] = None)
                The maximum number of results to return (default: unlimited)
            * set: (Optional[Union[str,AnnotationDataSet]] = None)
                An ID of a dataset (or an :class:`AnnotationDataSet` instance), only needed when specifying `key` as a string
            * key: (Optional[Union[str,DataKey]] = None)
                An ID of a key (or a :class:`DataKey` instance), make sure to specify `set` as well if you use a string value for this parameter.
            * value: (Optional[Union[str,int,float,bool]])
                Constrain the search to annotations with data of a certain value. This can only be used when you also pass a :class:`DataKey` as filter.
                This holds the exact value to search for, there are other variants of this keyword available, see :meth:`data` for a full list. 
            * limit: (Optional[int] = None)
                The maximum number of results to return (default: unlimited)
        """

    def test_annotations(self, *args, **kwargs) -> bool:
        """
        Tests whether there are any annotations that make use of this data.
        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using keyword arguments. See :meth:`Annotation.annotations`.
       """

    def annotations_len(self, limit: Optional[int] = None) -> int:
        """Returns the number of annotations (:class:`Annotation`) that use this data.
        Note that this is much faster than doing `len(annotations())`!

        Parameters
        ------------

        `limit`: `Optional[int] = None`
            The maximum number of results to return (default: unlimited)
        """

    def select(self) -> Selector:
        """Returns a selector pointing to this data (AnnotationDataSelector)"""

class Data:
    """
    A `Data` object holds an arbitrary collection of annotation data. 
    The data are references to items in an AnnotationStore, not copies.
    You can iterate over it to retrieve :class:`AnnotationData` instances.
    """

    def __iter__(self) -> Iterator[AnnotationData]:
        """Iterator over all data in this collection"""

    def __next__(self) -> AnnotationData:
        """Return the next item in the iterator"""

    def __len__(self) -> int:
        """Returns the number of data items in the collection"""

    def __getitem__(self, int) -> AnnotationData:
        """Returns data in the collection by index"""

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that are make use of any of the data in this collection

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

    def test_annotations(self, *args, **kwargs) -> bool:
        """
        Tests whether there are any annotations that make use of any of the data in this collection
        This method is like :meth:`annotations`, but does only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

class TextSelections:
    """
    A `TextSelections` object holds an arbitrary collection of text selections. 
    You can iterate over it to retrieve :class:`TextSelection` instances.
    """

    def __iter__(self) -> Iterator[TextSelection]:
        """Iterator over all text selections in this collection"""

    def __next__(self) -> TextSelection:
        """Return the next item in the iterator"""

    def __len__(self) -> int:
        """Returns the number of data items in the collection"""

    def __getitem__(self, int) -> TextSelection:
        """Returns a textselection in the collection by index"""

    def __str__(self) -> str:
        """Returns the text of all textselections.

        The results are space-delimited, use :meth:`text_join` instead if you want another delimiter.
        """

    def text_join(self, delimiter: str) -> str:
        """Returns the text of all textselections, separated by the provider delimiter. This is more efficient than calling `.text().join()` yourself."""

    def text(self, delimiter: str) -> List[str]:
        """Returns the text of all textselections in a list"""

    def annotations(self, *args, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that refer to any of the text selections in this collection

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

    def test_annotations(self, **kwargs) -> bool:
        """
        Tests whether there are any annotations that refer to any of the text selections in this collection

        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

    def data(self, *args, **kwargs) -> Data:
        """
        Returns annotation data (:class:`Data` containing :class:`AnnotationData`) used by annotations referring to the text selections in this collection.

        The data can be filtered using positional and/or keyword arguments; see :meth:`Annotation.data`.
        If no filters are set (default), all data from all annotations on all text selections are returned (without duplicates).
        """

    def test_data(self, *args, **kwargs) -> bool:
        """
        Tests whether there are any annotations that reference any of the text selections in the iterator, with data that passes the provided filters.
        The result is functionally equivalent to doing `.annotations().test_data()`, but this shortcut method is implemented much more efficiently and therefore recommended.

        The data can be filtered using positional and/or keyword arguments. See :meth:`Annotations.data`.
        """

    def related_text(self, operator: TextSelectionOperator, *args, **kwargs) -> TextSelections:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with the ones from the current collections. 
        Returns a collection of all matching :class:`TextSelection` instances.

        Text selections will be returned in textual order. They may be filtered via positional and/or keyword arguments. See :meth:`Annotation.textselections`.
       
        If you are interested in the annotations associated with the found text selections, then
        add `.annotations()` to the result.

        See :meth:`Annotation.related_text` for allowed keyword arguments and examples.
        """

    def textual_order(self) -> TextSelections:
        """
        Sorts the annotations in textual order.

        This has some performance cost, so prevent calling this method on methods that already promise to return textual order (which most textselection methods do!) 
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


        Example
        ------------

        Instantiation::

            Selector.textselector(store.resource("testres"), Offset.simple(6,11))
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


        Example
        ------------

        Instantiation::

            Selector.textselector(store.annotation("A1"), Offset.whole())
        """

    @staticmethod
    def resourceselector(resource: TextResource) -> Selector:
        """Creates a *ResourceSelector* - A selector pointing to a resource as whole. These type
        of annotation can be interpreted as *metadata*.

        Parameters
        ------------

        resource: TextResource
            The resource 


        Example
        ------------

        Instantiation::

            Selector.resourceselector(store.resource("my-resource"))
        """

    @staticmethod
    def datasetselector(dataset: AnnotationDataSet) -> Selector:
        """Creates a *DataSetSelector* - A selector pointing to an annotation dataset as whole. These type
        of annotation can be interpreted as *metadata*.


        Parameters
        -----------------

        dataset: AnnotationDataSet
            The annotation data set.

        Example
        ------------

        Instantiation::

            Selector.datasetselector(store.dataset("my-dataset"))
        """

    @staticmethod
    def multiselector(*subselectors: Selector) -> Selector:
        """Creates a *MultiSelector* - A selector that consists of multiple other selectors (subselectors) to select multiple targets. This *MUST* be interpreted as the annotation applying to each target *individually*, without any relation between the different targets.

        Parameters
        --------------------

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


        Example
        ---------

        Instantiation of a composite selector over two annotation selectors::

            Selector.compositeselector(
                Selector.annotationselector(self.store.annotation("A1"), Offset.whole()),
                Selector.annotationselector(self.store.annotation("A2"), Offset.whole()),
            )
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
    DATAKEYSELECTOR: SelectorKind
    ANNOTATIONDATASELECTOR: SelectorKind
    MULTISELECTOR: SelectorKind
    COMPOSITESELECTOR: SelectorKind
    DIRECTIONALSELECTOR: SelectorKind

class Offset:
    """
    Text selection offset. Specifies begin and end offsets to select a range of a text, via two :class:`Cursor` instances.
    The end-point is non-inclusive.
    """

    def __init__(self, begin: Cursor, end: Cursor) -> None:
        """You can instantiate a new offset on the basis of two :class:`Cursor` instances"""

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

    def shift(self, distance: int) -> Offset:
        """
        Move an offset a specified distance (character points) to the right (positive integer) or to the left (negative integer).
        Will raise a ValueError if the offsets exceeds its natural bounds.
        """

    def __len__(self) -> int:
        """
        Returns the length of the offset (in character points).
        If the underlying cursor types are not of the same type, the length
        is undefined and a ValueError is raised.
        """

class Cursor:
    """
    A cursor points to a specific point in a text. It is used to select offsets. Units are unicode codepoints (not bytes!)
    and are 0-indexed.
   
    The cursor can be either begin-aligned or end-aligned. Where BeginAlignedCursor(0)
    is the first unicode codepoint in a referenced text, and EndAlignedCursor(0) the last one.
    """

    def __init__(self, index, endaligned: bool = False):
        """
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

    def shift(self, distance: int) -> Cursor:
        """
        Move a cursor a specified distance (character points) to the right (positive integer) or to the left (negative integer).
        Will raise a ValueError if the offsets exceeds its natural bounds.
        """

class TextResource:
    """
    This holds the textual resource to be annotated. It holds the full text in memory.
   
    The text *SHOULD* be in
    [Unicode Normalization Form C (NFC) (https://www.unicode.org/reports/tr15/) but
    *MAY* be in another unicode normalization forms.
    """

    def set_filename(self, filename: str):
        """Set the filename for stand-off file specified using @include (if any)"""

    def filename(self, filename: str) -> Optional[str]:
        """Returns the filename for the stand-off file specified using @include (if any). This allocates a copy, use has_filename() for checking."""

    def has_filename(self, filename: str) -> bool:
        """Tests the filename for the stand-off file specified using @include (if any)."""

    def id(self) -> Optional[str]:
        """Returns the public ID (by value, aka a copy)
        Don't use this for extensive ID comparisons, use :meth:`has_id` instead as it is more performant (no copy)."""

    def has_id(self, id: str) -> bool:
        """Tests the ID"""

    def __iter__(self) -> Iterator[TextSelection]:
        """Iterates over all known textselections in this resource, in sorted order. This is a low-level iterator, :meth:`textselections` provides a higher-level interface."""

    def textselections(self) -> TextSelections:
        """Iterates over all known textselections in this resource, in sorted order."""

    def select(self) -> Selector:
        """Returns a selector pointing to this resource"""

    def text(self) -> str:
        """Returns the text of the resource (by value, aka a copy)"""

    def textlen(self) -> int:
        """
        Returns the length of the resources's text in unicode points (same as `len(self.text())` but more performant)
        """

    def __len__(self) -> int:
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

    def annotations(self, *args, **kwargs) -> Annotations:
        """Returns a collection of annotations (:class:`Annotation`) that reference this resource via a *TextSelector* (if any).
        Does *NOT* include those that use a ResourceSelector, use :meth:`annotations_metadata` instead for those instead.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
        """


    def annotations_as_metadata(self, *args, **kwargs) -> Annotations:
        """Returns a collection of annotations (:class:`Annotation`) that reference this resource via a *ResourceSelector* (if any).
        Does *NOT* include those that use a TextSelector, use :meth:`annotations` instead for those instead.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
        """

    def test_annotations(self,*args,  **kwargs) -> bool:
        """
        Tests whether there are any annotations that reference the text of this resource (via a TextSelector). 

        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

    def test_annotations_as_metadata(self, *args, **kwargs) -> bool:
        """
        Tests whether there are any annotations that reference this resource as metadata (via a ResourceSelector). 

        This method is like :meth:`annotations_as_metadata`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using positional and/or keyword arguments. See :meth:`Annotation.annotations`.
       """

    def segmentation(self) -> List[TextSelection]:
        """
        Returns a sequence of minimum-length non-overlapping TextSelections, covering the full resource, in textual order
        """

    def segmentation_in_range(self, begin: int, end: int) -> List[TextSelection]:
        """
        Returns a sequence of minimum-length non-overlapping TextSelections, covering the selected range in the resource, in textual order
        """

    def substores(self) -> List[AnnotationSubStore]:
        """
        Returns the substores this resource is a part of (if any).
        """

#   def related_text(self, operator: TextSelectionOperator, referenceselections: List[TextSelection], **kwargs) -> TextSelections:
#       """
#       Applies a :class:`TextSelectionOperator` to find all other
#       text selections who are in a specific relation with the ones from `referenceselections`.
#       Returns all matching :class:`TextSelection` instances in a collection :class:`TextSelections`.

#       Text selections will be returned in textual order. They may be filtered via keyword arguments. See :meth:`Annotation.textselections`.
#      
#       Parameters
#       ------------

#       operator: TextSelectionOperator
#           The operator to apply when comparing text selections
#       referenceselections: List[TextSelection]
#           Text selections to use as reference
#

#       See :meth:`Annotation.related_text` for allowed keyword arguments.
#       """

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

    def offset(self) -> Offset:
        """Converts the TextSelection to an :class:`Offset` instance"""

    def select(self) -> Selector:
        """Returns a selector pointing to this resource"""

    def text(self) -> str:
        """Returns the text of the resource (by value, aka a copy)"""

    def textlen(self) -> int:
        """
        Returns the length of the text selection in unicode points (same as `len(self.text())` but more performant)
        """

    def __len__(self) -> int:
        """
        Returns the length of the text selection in unicode points (same as `len(self.text())` but more performant)
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

    def annotations(self, **kwargs) -> Annotations:
        """
        Returns annotations (:class:`Annotations` containing :class:`Annotation`) that reference this text selection via a *TextSelector* (if any).

        The annotations can be filtered using keyword arguments. See :meth:`Annotation.annotations`
        """

    def test_annotations(self, **kwargs) -> bool:
        """
        Tests whether there are any annotations that reference this text selection via a *TextSelector* (if any).

        This method is like :meth:`annotations`, but only tests and does not return the annotations, as such it is more performant.

        The annotations can be filtered using keyword arguments. See :meth:`Annotation.annotations`.
       """

    def test_data(self, **kwargs) -> bool:
        """
        Tests whether there are any annotations that reference this text selection with data that passes the provided filters.
        The result is functionally equivalent to doing `.annotations().test_data()`, but this shortcut method is implemented much more efficiently and therefore recommended.

        The data can be filtered using keyword arguments. See :meth:`Annotations.data`.
        """

    def related_text(self, operator: TextSelectionOperator, **kwargs) -> TextSelections:
        """
        Applies a :class:`TextSelectionOperator` to find all other
        text selections who are in a specific relation with this one.
        Returns all matching :class:`TextSelection` instances in a collection :class:`TextSelections`.

        Text selections will be returned in textual order. They may be filtered via keyword arguments. See :meth:`Annotation.textselections`.
       
        Parameters
        ------------

        operator: TextSelectionOperator
            The operator to apply when comparing text selections


        See :meth:`Annotation.related_text` for allowed keyword arguments and examples.
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

    def test_annotation(self, operator: TextSelectionOperator, other: Annotation) -> bool:
        """
        This method is called to test whether a specific spatial relation (as expressed by the
        passed operator) holds between an :class:`TextSelection` and an :class:`Annotation`.
        A boolean is returned with the test result.
        """

    def segmentation(self) -> List[TextSelection]:
        """
        Returns a sequence of minimum-length non-overlapping TextSelections, covering the selected text selection, in textual order
        """

    def align_texts(self, other: TextSelection, **kwargs) -> list[Annotation]:
        """
        Used to compute an alignment between two texts; it
        identifies which parts of the two texts are identical and computes a mapping
        between the two coordinate systems. Two related sequence alignments algorithms
        from bioinformatics are implemented to accomplish this:
        Smith-Waterman and Needleman-Wunsch.

        The resulting alignment is added to the store as an annotation, a so called transposition,
        according to the `STAM Transpose <https://github.com/annotation/stam/tree/master/extensions/stam-transpose>`_
        extension. These annotations are also returned by this function.

        If you want to parallelise execution rather than sequentially call this
        function, then use :meth:`AnnotationStore.align_texts()` instead.

        Parameters
        --------------
        other: TextSelection
            The other text selection to compare against

        Keyword Arguments
        -------------------
        
        case_insensitive: bool
            Case-insensitive matching has more performance overhead
        algorithm: str
            The alignment algorithm to use, can be `smithwaterman`/`local` (local alignment) or `needlemanwunsch`/`global` (global alignment).
        grow: bool
            Grow aligned parts into larger alignments by incorporating non-matching parts. If you set this, 
            the function will return translations rather than transpositions.
    		You'll want to set `max_errors` in combination with this one to prevent very low-quality alignments.
        max_errors: Union[int,float]
    		The maximum number of errors (max edit distance) that may occur for a transposition to be valid.
    		This is either an absolute integer or a relative ratio between 0.0 and 1.0, interpreted in relation to the length of the first text in the alignment.
    		In other words; this represents the number of characters in the search string that may be missed when matching in the larger text.
    		The transposition itself will only consist of fully matching parts, use `grow` if you want to include non-matching parts.
        minimal_align_length: int
    		The minimal number of characters that must be aligned (absolute number) for a transposition/translation to be valid.
        annotation_id_prefix: str
            Prefix to use when assigning annotation IDs. The actual ID will have a random component
        trim: bool
            Strip leading and trailing whitespace/newlines from aligned text selections, keeping them as minimal as possible (default is to be as greedy as possible in selecting)
            Setting this may lead to certain whitespaces not being covered even though they may align.
        simple_only: bool
            Only allow for alignments that consist of one contiguous text selection on either side. This is a so-called simple transposition.
        """

class TextSelectionOperator:
    """
    The TextSelectionOperator, simply put, allows comparison of two :class:`TextSelection` instances. It
    allows testing for all kinds of spatial relations (as embodied by this class) in which two
    :class:`TextSelection` instances can be.
   
    Rather than operate on single :class:`TextSelection` instances, the implementation goes a bit
    further and can act also on the basis of multiple :class:`TextSelection` instances as a set;
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
    def embedded(all: Optional[bool] = False, negate: Optional[bool] = False, limit: Optional[int] = None) -> TextSelectionOperator:
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
        limit: Optional[usize]
            Constrain the lookup to at most this many unicode points (increases performance)
        """

    @staticmethod
    def before(all: Optional[bool] = False, negate: Optional[bool] = False, limit: Optional[int] = None) -> TextSelectionOperator:
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
        limit: Optional[usize]
            Constrain the lookup to at most this many unicode points (increases performance)
        """

    @staticmethod
    def after(all: Optional[bool] = False, negate: Optional[bool] = False, limit: Optional[int] = None) -> TextSelectionOperator:
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
        limit: Optional[usize]
            Constrain the lookup to at most this many unicode points (increases performance)
        """

    @staticmethod
    def precedes(all: Optional[bool] = False, negate: Optional[bool] = False, allow_whitespace: Optional[bool] = True) -> TextSelectionOperator:
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
        allow_whitespace: Optional[bool] 
            Allow whitespace between the two text selections, this defaults to `True` 
        """

    @staticmethod
    def succeeds(all: Optional[bool] = False, negate: Optional[bool] = False, allow_whitespace: Optional[bool] = True) -> TextSelectionOperator:
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
        allow_whitespace: Optional[bool] 
            Allow whitespace between the two text selections, this defaults to `True` 
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

class AnnotationSubStore:
    """
    A substore is a sub-collection of annotations that is serialised as an independent AnnotationStore,
    In STAM JSON it is included using the @include mechanism.
    """

    def id(self) -> Optional[str]:
        """Returns the public identifier (by value, aka a copy)"""

    def filename(self, filename: str) -> Optional[str]:
        """Returns the filename for the stand-off annotation store. This allocates a copy, use has_filename() for checking."""

    def has_filename(self, filename: str) -> bool:
        """Tests the filename for the stand-off file specified using @include (if any)."""

    def associate(self, item: Union[Annotation,TextResource,AnnotationDataSet]):
        """Associates an annotation, text resource or annotation dataset with this substore"""


class StamError(Exception):
    """STAM Error"""
