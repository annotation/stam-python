<p align="center">
    <img src="https://github.com/annotation/stam/raw/master/logo.png" alt="stam logo" width="320" />
</p>

[![Docs](https://readthedocs.org/projects/stam-python/badge/?version=latest&style=flat)](https://stam-python.readthedocs.io)
[![PyPI](https://img.shields.io/pypi/v/stam.svg)](https://pypi.org/project/stam/)
[![PyPI](https://img.shields.io/pypi/dm/stam.svg)](https://pypi.org/project/stam/)
[![GitHub build](https://github.com/annotation/stam-rust/actions/workflows/stam.yml/badge.svg?branch=master)](https://github.com/annotation/stam-rust/actions/)
[![GitHub release](https://img.shields.io/github/release/annotation/stam-python.svg)](https://GitHub.com/annotation/stam-python/releases/)
[![Project Status: Active – The project has reached a stable, usable state and is being actively developed.](https://www.repostatus.org/badges/latest/active.svg)](https://www.repostatus.org/#active)
![Technology Readiness Level 7/9 - Release Candidate - Technology ready enough and in initial use by end-users in intended scholarly environments. Further validation in progress.](https://w3id.org/research-technology-readiness-levels/Level7ReleaseCandidate.svg)

# STAM Python binding

[STAM](https://github.com/annotation/stam) is a data model for stand-off text
annotation and described in detail [here](https://github.com/annotation/stam).
This is a python library (to be more specific; a python binding written in
Rust) to work with the model.

**What can you do with this library?**

* Keep, build and manipulate an efficient in-memory store of texts and annotations on texts
* Search in annotations, data and text:
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

## Installation

``$ pip install stam``

Or if you feel adventurous and have the necessary build-time dependencies
installed (Rust), you can try the latest development release from Github:

``$ pip install git+https://github.com/annotation/stam-python``

## Documentation

* [STAM Specification](https://github.com/annotation/stam) - the STAM specification itself
* [API Reference](https://stam-python.readthedocs.io)
* [STAM Tutorial: Standoff Text Annotation for Pythonistas](https://nbviewer.org/github/annotation/stam-python/blob/master/tutorial.ipynb) - An extensive tutorial showing how to work with this Python library, in the form of a Jupyter Notebook. **Recommended!**

## Usage 

Import the library

```python
import stam
```

Loading a STAM JSON (or CSV) file containing an annotation store:

```python
store = stam.AnnotationStore(file="example.stam.json")
```


The annotation store is your workspace, it holds all resources, annotation sets
(i.e. keys and annotation data) and of course the actual annotations. It is a
memory-based store and you can put as much as you like into it (as long as it fits
in memory).

You can optionally pass configuration parameters upon loading a store, as follows:

```python
store = stam.AnnotationStore(file="example.stam.json", config={"debug": True})
```

Once loaded, you can retrieve anything by its public ID:

```python
annotation = store.annotation("my-annotation")
resource = store.resource("my-resource")
dataset = store.dataset("my-annotationset")
key = dataset.key("my-key")
data = dataset.annotationdata("my-data")
```

You can also iterate through all annotations in the store, and output a simple tab separated format:

```python
for annotation in store.annotations():
    # get the text to which this annotation refers (if any)
    try:
        text = str(annotation)
    except stam.StamError:
        text = "n/a"
    for data in annotation:
        print("\t".join(( annotation.id(), data.key().id(), str(data.value()), text)))
```


Adding a resource:

```python
resource = store.add_resource(filename="my-text.txt")
```

Create a store and annotations from scratch:

```python
from stam import AnnotationStore, Selector, AnnotationDataBuilder

store = AnnotationStore(id="test")
resource = store.add_resource(id="testres", text="Hello world")
store.annotate(id="A1", 
                target=Selector.textselector(resource, Offset.simple(6,11)),
                data={ "id": "D1", "key": "pos", "value": "noun", "set": "testdataset"})
```

In the above example, the `AnnotationDataSet` , `DataKey` and `AnnotationData`
are created on-the-fly. You can also create them explicitly within the set first, as shown in the
next snippet, resulting in the exact same store:


```python
store = AnnotationStore(id="test")
resource = store.add_resource(id="testres", text="Hello world")
dataset = store.add_dataset(id="testdataset")
dataset.add_key("pos")
data = dataset.add_data("pos","noun","D1")
store.annotate(id="A1", 
    target=Selector.textselector(resource, Offset.simple(6,11)),
    data=data)
```

Providing the full data dictionary as in the earlier example would have
also worked fine, with the same end result, but would be less performant than passing an `AnnotationData` instance directly.
The implementation will always ensure any already existing `AnnotationData` will be reused if
possible, as not duplicating data is one of the core characteristics of the
STAM model.

You can serialize the entire annotation store (including all sets and annotations) to a STAM JSON file:

```python
store.set_filename("example.stam.json")
store.save()
```

For more documentation, please read: [STAM Tutorial: Standoff Text Annotation for Pythonistas](https://nbviewer.org/github/annotation/stam-python/blob/master/tutorial.ipynb).

## Differences between the rust library and python library and performance considerations

Although this Python binding builds on the Rust library, the API it exposes
differs in certain aspects to make it more pythonic and easier to work with.
This results in a higher-level API that hides some of the lower-level details
that are present in the Rust library. This approach does come at the cost of causing
some additional runtime overhead. 

The Rust methods will often return iterators, references or handles whenever they
can, moreover it will do so safely. The Python API is often forced to make a
local copy. For iterators we often decide to let the entire underlying Rust
iterator run its course and then return the result as a whole as a tuple, rather than
return a Python generator. Here you gain some speed at the cost of some memory.

Probably needless to say, but using Rust directly will always be more
performant than using this Python binding. However, using this Python binding
should still be way more performant than if the whole thing were implemented in
native Python. The trick is in letting the binding work for you as much as
possible, use higher-level methods whenever they are available rather than
implementing your logic in Python.

## Acknowledgements

This work is conducted at the [KNAW Humanities Cluster](https://huc.knaw.nl/)'s [Digital Infrastructure department](https://di.huc.knaw.nl/), and funded by the [CLARIAH](https://clariah.nl) project (CLARIAH-PLUS, NWO grant 184.034.023) as part of the FAIR Annotations track.
