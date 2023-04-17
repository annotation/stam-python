---
jupyter:
  jupytext:
    text_representation:
      extension: .md
      format_name: markdown
      format_version: '1.3'
      jupytext_version: 1.14.5
  kernelspec:
    display_name: Python 3 (ipykernel)
    language: python
    name: python3
---

# STAM Tutorial: Standoff Text Annotation for Pythonistas

## Introduction

[STAM](https://github.com/annotation/stam) is a data model for stand-off text
annotation that allows researchers and developers to model annotations on text.

An *annotation* is any kind of remark, classification/tagging on any particular
portion(s) of a text, on the resource or annotation set as a whole, in which
case we can interpret annotations as *metadata*, or on another annotation
(*higher-order annotation*).

Examples of annotation may be linguistic annotation, structure/layout
annotation, editorial annotation, technical annotation, or whatever comes to
mind.STAM define any vocabularies whatsoever. Instead, it provides a
framework upon which you can model your annotations using whatever
you see fit.

The model is thoroughly explained [in its specification
document](https://github.com/annotation/stam/blob/master/README.md). We
summarize only the most important data structures here, these have direct
counterparts (classes) in the python library we will be teaching in this
tutorial: 

* `Annotation`  - A instance of annotation. Associated with an annotation is a
  `Selector` to select the target of the annotation, and one or more
  `AnnotationData` instances that hold the *body* or *content* of the
  annotation. This is explicitly decoupled from the annotation instance itself
  as multiple annotations may hold the very same content.
* `Selector` - A selector identifies the target of an annotation and the part of the target that the annotation applies to. There are multiple types that are described [here](https://github.com/annotation/stam/blob/master/README.md#class-selector). The `TextSelector` is an important one that selects a target resource and a specific text selection within it by specifying an offset. 
* `AnnotationData` - A key/value pair that acts as *body* or *content* for one or more annotations. The key is a reference to `DataKey`, the value is a `DataValue`. (The term *feature* is also seen for this in certain annotation paradigms)
* `DataKey` - A key as referenced by `AnnotationData`.
* `DataValue` - A value with some type information (e.g. string, integer, float).
* `TextResource` - A textual resource that is made available for annotation. This holds the actual textual content.
* `TextSelection` - A particular selection of text within a resource, i.e. a subslice of the text.
* `AnnotationDataSet` - An Annotation Data Set stores the keys (`DataKey`) and
  values (`AnnotationData`) that are used by annotations. It effectively
  defines a certain vocabulary, i.e. key/value pairs. How broad or narrow the
  scope of the vocabulary is not defined by STAM but entirely up to the user. 
* `AnnotationStore` - The annotation store is essentially your *workspace*, it holds all
  resources, annotation sets (i.e. keys and annotation data) and of course the
  actual annotations. In the Python implementation it is a memory-based store
  and you can put as much as you like into it (as long as it fits in memory).

STAM is more than just a theoretical model, we offer practical implementations
that allow you to work with it directly. In this tutorial we will be using Python and
the Python library `stam`.

**Note**: The STAM Python library is a so-called Python binding to a STAM library
written in Rust. This means the library is not written in Python but is
compiled to machine code and as such offers much better performance.

## Installation

First of all, you will need to install the STAM Python library from the [Python Package Index](https://pypi.org/project/stam/) as follows:

```python
!pip install stam
```

## Annotating from scratch

### Adding a text

Let us start with a mini corpus consisting of two quotes from the book *"Consider Phlebas"* by renowned sci-fi author Iain M. Banks.

```python
text = """
# Consider Phlebas
$ author=Iain M. Banks

## 1
Everything about us,
everything around us,
everything we know [and can know of] is composed ultimately of patterns of nothing;
that’s the bottom line, the final truth.

So where we find we have any control over those patterns,
why not make the most elegant ones, the most enjoyable and good ones,
in our own terms?

## 2
Besides,
it left the humans in the Culture free to take care of the things that really mattered in life,
such as [sports, games, romance,] studying dead languages,
barbarian societies and impossible problems,
and climbing high mountains without the aid of a safety harness.
"""
```

This format of the text for STAM is in no way prescribed other than:

* It must be plain text
* It must be UTF-8 encoded
* It should ideally be in Unicode Normalization Form C. (don't worry if this means nothing to you yet)

Let's add this text resource to an annotation store so we can annotate it:

```python
store = stam.AnnotationStore(id="tutorial")
resource_banks = store.add_resource(id="banks", text=text)
```

Here we passed the text as a string, but it could just as well have been an
external text file instead, the filename of which can be passed via the `file=` keyword
argument.

### Creating an annotation dataset (vocabulary)

Our example text is a bit Markdown-like, we have a title header *"Consider Phlebas"*, and 
two subheaders (*1* and *2*) containing one quote from the book each. 

As our first annotations, let's try to annotate this coarse structure. At this
point we're already in need of some vocabulary to express the notions of *title
header*, *section header* and *quote*, as STAM does not define any vocabulary.
It is up to you to make these choices on how to represent the data.

An annotation data set effectively defines an vocabulary. Let's invent our own
simple Annotation Data Set that defines the keys and values we use in this
tutorial. In our `AnnotationDataSet` We can define a `DataKey` with ID `structuretype`, and have it
takes values like `titleheader`, `sectionheader` and `quote`.

We can explicitly add the set and the key. We give the dataset a public ID
(*tutorial-set*), just as we previously assigned a public ID to both the
annotationstore (*tutorial*) and the text resource (*banks*). It is good
practise to assign IDs, though you can also let the library auto-generate them
for you:

```python
dataset = store.add_annotationset("tutorial-set")
key_structuretype = dataset.add_key("structuretype")
```

### The first annotations with text selectors

To annotate the title header, we need to select the part of the text where it
occurs by finding the offset, which consists of a *begin* and *end* position. STAM
follows the same indexing format Python does, in which positions are 0-indexed
*unicode character points* (as opposed to (UTF-8) bytes) and where the end is
non-inclusive. After some clumsy manual counting on the source text we discover
the following coordinates hold:

```
assert text[1:19] == "# Consider Phlebas"
```

And we make the annotation:

```python
annotation = store.annotate(
    target=stam.Selector.textselector(resource_banks, stam.Offset.simple(1,19)),
    data={"id": "Data1", "key": key_structuretype, "value": "titleheader", "set": dataset }
    id="Annotation1")
```

A fair amount happened there. We selected a part of the text of
`resource_banks` by offset, and associated `AnnotationData` with the annotation
saying that the `structuretype` key has the value `titleheader`, both of which
we invented as part of our `AnnotationDataSet` with ID `tutorial-set`. Last, we
assigned an ID to both the `AnnotationData`, as well as to the `Annotation` as
a whole. In this example we reused some of the variables we had created
earlier, but we could have also written out in full as shown below: 

```python
%%script false

annotation = store.annotate(
    target=stam.Selector.textselector("banks", stam.Offset.simple(1,19)),
    data={"id": "Data1", "key": "structuretype", "value": "titleheader", "set": "tutorial-set" }
    id="Annotation1")
```

This would also have been perfectly fine, and moreover, it would also work fine
without us explicitly creating the `AnnotationDataSet` and the key as we did
before! Those would have been automatically created on-the-fly for us. The
only disadvantage is that under the hood more lookups are needed, so this is
slightly less performant than passing python variables.

### Inspecting data (1)

We can inspect the annotation we just added:

```python
print("Annotation ID: ", annotation.id())
print("Target text: ", str(annotation))
print("Data: ")
for data in annotation.data():
    print(" - Data ID: ", data.id())
    print("   Data Key: ", data.key().id())
    print("   Data Value: ", str(data.value()))
```

In the above example, we obtained an `Annotation` instance from the return value of the `annotate()` method. Once any annotation is in the store, we can retrieve it simply by its public ID using the `annotation()` method. An exception will be raised if the ID does not exist.

```python
annotation = store.annotation("Annotation1")
```

A similar pattern holds for almost all other data structures in the STAM model:

```python
dataset = store.annotationset("tutorial-set")      #AnnotationDataSet
resource_banks = store.resource("banks")           #TextResource
key_structuretype = dataset.key("structuretype")   #DataKey
data = dataset.annotationdata("Data1")             #AnnotationData
```

### Annotating via `find_text()`

We now continue by adding annotations for the two section headers. Counting offsets
manually is rather cumbersome, so we use the `find_text()` method on `TextResource` to find our target for annotation:

```python
results = resource_banks.find_text("## 1")
section1 = results[0]
print(f"Text {str(section1)} found at {section1.begin()}:{section1.end()}")

annotation = store.annotate(
    target=stam.Selector.textselector("banks", section1),
    data={"id": "Data2", "key": "structuretype", "value": "sectionheader", "set": "tutorial-set" }
    id="Annotation2")
```

The `find_text()` method returns a list of `TextSelection` instances. These
carry an `Offset` which is returned by the `offset()` method. Hooray, no more
manual counting!

We do the same for the last header:

```python
results = resource_banks.find_text("## 2")
section2 = results[0]
print(f"Text {str(section2)} found at {section2.begin()}:{section2.end()}")

annotation = store.annotate(
    target=stam.Selector.textselector("banks", section2.offset()),
    data={"id": "Data2", "key": "structuretype", "value": "sectionheader", "set": "tutorial-set" }
    id="Annotation3")
```

### Inspecting data (2)

In the previous code the attentive reader may have noted that we are reusing the `Data2` ID
rather than introducing a new `Data3` ID, because the data for both
`Annotation2` and `Annotation3` is in fact, identical.

This is an important feature of STAM; annotations and their data are
decoupled precisely because the data may be referenced by multiple annotations, and
if that's the case, we only want to keep the data in memory once. We don't want
a copy for every annotation. Say we have `AnnotationData` with key
`structuretype` and value `word`, and use that to tag all words in the
text, then it would be a huge amount of redundancy if there was no such
decoupling between data and annotations. The fact that they all share the same data, also
enables us to quickly look up all those annotations:

```python
annotationdata = dataset.find_data("structuretype", "sectionheader")
for annotation in annotationdata.annotations():
    assert annotation.id() in ("Annotation2","Annotation3")
```

### Annotations via text selections

Now we annotate the quotes themselves. The first one starts after the first
subheader (Annotation2) and ends just before the next subheader (Annotation3).
That would include some ugly leading and trailing whitespace/newlines, though.
We use the `textselection()` method to obtain a textselection to our computed
offset and subsequently strip the whitespace using the `strip_text()` method,
effectively shrinking our textselection a bit:

```python
quote1_selection = resource_banks.textselection(stam.Offset.simple(section1.end(), section2.begin() - 1).strip_text(" \t\r\n")
quote1 = store.annotate(
    target=stam.Selector.textselector(resource_banks, quote1_selection.offset()),
    data={"id": "Data3", "key": "structuretype", "value": "quote", "set": "tutorial-set" }
    id="AnnotationQuote1")
```

The second quote goes until the end of the text, which we can retrieve using
the `textlen()` method (This method is preferred over doing things in native
python like `len(str(banks))` because it is way more efficient):

```python
quote2_selection = resource_banks.textselection(stam.Offset.simple(section2.end(), resource_banks.textlen()).strip_text(" \t\r\n")
quote2 = store.annotate(
    target=stam.Selector.textselector(resource_banks, quote2_selection.offset()),
    data={"id": "Data3"}
    id="AnnotationQuote2")
```

In this example we also show that, since we reference existing
`AnnotationData`, just specifying the ID suffices. Even better, you could pass
a variable that is an instance of `AnnotationData`.

There is another structural type we could annotate: the lines with
corresponding line numbers. This is easy to do by splitting the text on
newlines, for which we use the method `split_text()` on `TextResource`. As you
see, various Python methods such as `split()`, `strip()`, `find()` have
counterparts in STAM that have a `*_text()` suffix and which return
`TextSelection` instances and carry offset information:


```python
for linenr, line in enumerate(resource.banks.split_text("\n")):
    linenr += 1      #make it 1-indexed as is customary for line numbers
    store.annotate(
        target=stam.Selector.textselector(resource_banks, line.offset()),
        data=[ 
            {"id": "Data4", "key": "structuretype", "value": "line", "set": "tutorial-set" },
            {"id": "Data5", "key": "linenr", "value": linenr, "set": "tutorial-set" }
        ]
        id=f"AnnotationLine{linenr}")
```

In this example we also extended our vocabulary on-the-fly with a new field `linenr`. All line annotations carry two `AnnotationData` elements. Remember we an easily retrieve the data and any annotations on it with `find_data()`:

```python
annotationdata = dataset.find_data("linenr", 8)
line8 = annotationdata.annotations()[0]
print(str(line8))
```

When annotating, we don't have to work with the resource as a whole but can
also start relative from any text selection we have.  Let's take line eight and
annotate the first word of it (*"everything"*) manually:

```python
line8_textselection = line8.textselections()[0] #there could be multiple, but in our cases thus-far we only have one
firstword = line8_textselection.textselection(Offset::simple(0,10))  #we make a textselection on a textselection

#internally, the text selection will always use absolute coordinates for the resource:
print(f"Text selection spans: {firstword.begin()}:{firstword.end()}")

store.annotate(
    target=stam.Selector.textselector(resource_banks, firstword.offset()),
    data= {"key": "structuretype", "value": "word", "set": "tutorial-set" },
    id=f"AnnotationLine8Word1")
```

### Converting offsets

We know the first word of line eight is also part of quote one, for which we already made an annotation (`AnnotationQuote1`) before.
Say we are interested in knowing *where* in quote one the first word of line eight is, we can now easily compute so as follows:

```python
offset = firstword.relative_offset(quote1_selection)
print(f"Offset in quote one: {offset.begin()}:{offset.end()}")
```

While we are at it, another conversion option that may come handy when working
on a lower-level is the conversion from/to UTF-8 byte offsets. Both STAM and
Python use unicode character points. Internally STAM already maps these to
UTF-8 byte offsets for things like text slicing, but if you need this
information you can extract it explicitly:

```python
beginbyte = resource_banks.utf8byte(firstword.begin())
endbyte = resource_banks.utf8byte(firstword.end())
print(f"Byte offset: {beginbyte}:{endbyte}")

#and back again:
beginpos = resource_banks.utf8byte_to_charpos(beginbyte)
endpos = resource_banks.utf8byte_to_charpos(endbyte)

assert beginpos = firstword.begin()
assert endpos = firstword.end()
```

In this case they happen to be equal because we're basically only using ASCII
in our text, but as soon as you deal with multibyte characters (diacritics,
other scripts, etc), they will not!

### Tokenisation via regular expressions

What else can we annotate? We can mark all individual words or tokens,
effectively performing simple *tokenisation*. For this, we will use the regular
expression search that is built into the STAM library, `find_text_regex()`. The
regular expressions follow [Rust's regular expression
syntax](https://docs.rs/regex/latest/regex/#syntax) which may differ slightly
from Python's native implementation.

```python
expressions = [
    r"\w+(?:[-_]\w+)*", #this detects words,possibly with hyphens or underscores as part of it
    r"[\.\?,/]+", #this detects a variety of punctuation
    r"[0-9]+(?:[,\.][0-9]+)", #this detects numbers, possibly with a fractional part
]
structuretypes = ["word", "punctuation", "number"]

for i, matchresult in enumerate(resource_banks.find_text_regex(expressions)):
    #(we only have one textselection per match, but an regular expression may result in multiple textselections if capture groups are used)
    textselection = matchresult['textselections'][0]
    store.annotate(
        target=stam.Selector.textselector(resource_banks, line.offset()),
        data=[ 
            {"key": "structuretype", "value": structuretypes[matchresult['expression_index']], "set": "tutorial-set" },
        ]
        id=f"AnnotationToken{i+1}")
```

In this code, each `matchresult` tracks which of the three expressions was
matches, in `matchresult['expression_index']`. We conveniently use that
information to tie new values for `structuretype`, all of which will be added
to our vocabulary (`AnnotationDataSet`) on-the-fly.


### Annotating Metadata

Thus-far we have only seen annotations directly on the text, using
`Selector.textselector()`, but STAM has various other selectors. Users may
appreciate if you add a bit of metadata about your texts. In STAM, these are
annotations that point at the resource as a whole using a
`Selector.resourceselector()`, rather than at the text specifically. We add one
metadata annotation with various new fields:

```python
store.annotate(
    target=stam.Selector.resourceselector(resource_banks),
    data=[ 
        {"key": "name", "value": "Culture quotes from Iain Banks", "set": "tutorial-set" },
        {"key": "compiler", "value": "Dirk Roorda", "set": "tutorial-set" },
        {"key": "source", "value": "https://www.goodreads.com/work/quotes/14366-consider-phlebas", "set": "tutorial-set" },
        {"key": "version", "value": "0.2", "set": "tutorial-set" },
    ],
    id="Metadata1")
```

Similarly, we could annotate an `AnnotationDataSet` (our vocabulary) with metadata, using a `Selector.datasetselector()`.


## Searching





