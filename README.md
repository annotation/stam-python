[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)

# STAM Python binding

[STAM](https:/github.com/annotation/stam) is a data model for stand-off text annotation and described in detail [here](https://github.com/annotation/stam). This is a python library (to be more specific; a python binding written in Rust) to work with the model.

This library offers a higher-level interface than the underlying Rust library. Implementation is currently in a preliminary stage. We aim to implement the full model and most extensions.

## Installation

``$ pip install stam``

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

Once loaded, you can retrieving anything by its public ID:

```python
annotation = store.annotation("my-annotation")
resource = store.resource("my-resource")
annotationset = store.annotationset("my-annotationset")
key = annotationset.key("my-key")
data = annotationset.annotationdata("my-data")
```

You can also iterating through all annotations in the store, and outputting a simple tab separated format:

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
annotationset = store.add_annotationset(id="testdataset")
annotationset.add_key("pos")
data = annotationset.add_data("pos","noun","D1")
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
store.to_file("example.stam.json")
```

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
