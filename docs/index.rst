STAM Python Binding - API Documentation
=========================================

`STAM <https:/github.com/annotation/stam>`_ is a data model for stand-off text
annotation and described in detail `here <https://github.com/annotation/stam>`_.
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
    * The underlying STAM modelaims to be clear and simple. It is flexible and 
      does not commit to any vocabulary or annotation paradigm other than stand-off annotation.

This STAM library is intended as a foundation upon which further applications
can be built that deal with stand-off annotations on text. We implement all 
the low-level logic in dealing this so you no longer have to and can focus on your 
actual application.

This library offers a higher-level interface than the underlying Rust library. 
We aim to implement the full model and most extensions.

Tutorial
------------

A tutorial for working with this API is available in the form of an interactive
Jupyter Notebook: `STAM Tutorial: Standoff Text Annotation for Pythonistas
<https://github.com/annotation/stam-python/blob/master/tutorial.ipynb>`_.

Contents
========

.. toctree::
    :maxdepth: 2

    API Reference <stam.rst>


Index
=====
* :ref:`genindex`
