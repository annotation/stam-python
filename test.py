#!/usr/bin/env python3

import sys
from os import environ
import os.path
import unittest

from stam import AnnotationStore, Offset, AnnotationData, Selector, TextResource, DataKey, DataValue, AnnotationDataSet, Annotation, StamError, TextSelection, Cursor, TextSelectionOperator, Data,Annotations,TextSelections


class Test0(unittest.TestCase):
    def test_sanity_no_constructors(self):
        """Most stam types are references and can't be instantiated directly ('No constructor defined')"""
        with self.assertRaises(TypeError):
            Annotation()
        with self.assertRaises(TypeError):
            AnnotationDataSet()
        with self.assertRaises(TypeError):
            AnnotationData()
        with self.assertRaises(TypeError):
            TextResource()
        with self.assertRaises(TypeError):
            DataKey()
        with self.assertRaises(TypeError):
            TextSelection()

    def test_offset(self):
        offset = Offset.simple(0,5)
        self.assertEqual( offset.begin(), Cursor(0))
        self.assertEqual( offset.end(), Cursor(5))

    def test_offset_endaligned(self):
        offset = Offset(Cursor(0) , Cursor(0, endaligned=True) )
        self.assertEqual( offset.begin(), Cursor(0))
        self.assertEqual( offset.end(), Cursor(0, endaligned=True)) 

        offset2 = Offset.whole() #shortcut
        self.assertEqual( offset, offset2)

class Test1(unittest.TestCase):
    def setUp(self):
        """Create some data from scratch"""
        self.store = AnnotationStore(id="test")
        resource = self.store.add_resource(id="testres", text="Hello world")
        dataset = self.store.add_dataset(id="testdataset")
        dataset.add_key("pos")
        data = dataset.add_data("pos","noun","D1")
        self.store.annotate(id="A1", 
                            target=Selector.textselector(resource, Offset.simple(6,11)),
                            data=data)

    def test_sanity_1(self):
        self.assertIsInstance( self.store, AnnotationStore)
        self.assertEqual(self.store.id(), "test")
        self.assertEqual(self.store.annotations_len(), 1)
        self.assertEqual(self.store.datasets_len(), 1)
        self.assertEqual(self.store.resources_len(), 1)

    def test_sanity_2(self):
        resource = self.store.resource("testres")
        self.assertIsInstance( resource, TextResource)
        self.assertEqual(resource.id(), "testres")
        self.assertTrue(resource.has_id("testres")) #quicker than the above (no copy)

    def test_sanity_3(self):
        dataset = self.store.dataset("testdataset")
        self.assertIsInstance( dataset, AnnotationDataSet)
        key = dataset.key("pos")
        self.assertIsInstance( key, DataKey)
        self.assertEqual(str(key), "pos")
        data = dataset.annotationdata("D1")
        self.assertIsInstance( data, AnnotationData)
        self.assertTrue(data.has_id("D1"))

    def test_sanity_4_id_error(self):
        """Exceptions should be raised if IDs don't exist"""
        with self.assertRaises(StamError):
            self.store.dataset("non-existent-id")
        with self.assertRaises(StamError):
            self.store.annotation("non-existent-id")
        with self.assertRaises(StamError):
            self.store.resource("non-existent-id")

    def test_iter_data(self):
        """Iterates over the data in an annotation"""
        annotation = self.store.annotation("A1")
        self.assertIsInstance(annotation, Annotation)
        count = 0
        for annotationdata in annotation:
            count += 1
            #we can test in loop body because we only have one:
            self.assertIsInstance(annotationdata, AnnotationData)
            self.assertTrue(annotationdata.has_id("D1"))
            self.assertTrue(annotationdata.dataset().has_id("testdataset"))
            self.assertTrue(annotationdata.key().has_id("pos")) #this is the most performant in comparisons, it doesn't make a copy of the key
            self.assertEqual(str(annotationdata.key()), "pos") #force a string

            self.assertEqual(annotationdata.value().get(), "noun")
            self.assertTrue(annotationdata.test_value(DataValue("noun"))) #this is the most performant in comparisons, it doesn't make a copy of the value
            self.assertEqual(str(annotationdata.value()), "noun") #force a string
        self.assertEqual(count,1)

    def test_resource_text(self):
        """Get the text of an entire resource"""
        resource = self.store.resource("testres")
        self.assertIsInstance(resource, TextResource)
        self.assertEqual(str(resource), "Hello world")

    def test_direct_text_slice(self):
        """Get the text of a slice of a resource directly"""
        resource = self.store.resource("testres")
        self.assertEqual( resource[0:5], "Hello")

    def test_resource_text_slice(self):
        """Get the text of a slice of a resource"""
        resource = self.store.resource("testres")
        textselection = resource.textselection(Offset.simple(0,5))
        self.assertEqual( str(textselection), "Hello")

    def test_resource_slice_outofbounds(self):
        """Get an out of bound textselection"""
        resource = self.store.resource("testres")
        with self.assertRaises(StamError):
            resource.textselection(Offset.simple(0,999))

    def test_resource_find_text(self):
        """Find text"""
        resource = self.store.resource("testres")
        result = resource.find_text("world")
        self.assertIsInstance(result, list)
        self.assertEqual(len(result), 1)
        self.assertIsInstance(result[0], TextSelection)
        self.assertEqual(result[0].begin(), 6)
        self.assertEqual(result[0].end(), 11)
        self.assertEqual(str(result[0]), "world")

    def test_resource_split_text(self):
        """Split text"""
        resource = self.store.resource("testres")
        result = resource.split_text(" ")
        self.assertIsInstance(result, list)
        self.assertEqual(len(result), 2)
        self.assertIsInstance(result[0], TextSelection)
        self.assertEqual(result[0].begin(), 0)
        self.assertEqual(result[0].end(), 5)
        self.assertEqual(str(result[0]), "Hello")
        self.assertIsInstance(result[1], TextSelection)
        self.assertEqual(result[1].begin(), 6)
        self.assertEqual(result[1].end(), 11)
        self.assertEqual(str(result[1]), "world")

    def test_annotation_text(self):
        """Get the text of an annotation"""
        annotation = self.store.annotation("A1")
        count = 0
        for text in annotation.text():
            count += 1
            self.assertEqual(text, "world")
        self.assertEqual(count,1)

        #shortcut, will concatenate multiple text slices if needed
        self.assertEqual(str(annotation), "world")
            
    def test_annotation_textselections(self):
        """Get the textselections of an annotation"""
        annotation = self.store.annotation("A1")
        count = 0
        for textselection in annotation.textselections():
            count += 1
            self.assertEqual(str(textselection), "world")
            self.assertEqual(textselection.resource(), self.store.resource("testres"))
        self.assertEqual(count,1)

    def test_annotationset_iter(self):
        """Iterate over all data in an annotationset"""
        dataset = self.store.dataset("testdataset")
        count = 0
        for annotationdata in dataset:
            count += 1
            #we can test in loop body because we only have one:
            self.assertIsInstance(annotationdata, AnnotationData)
            self.assertTrue(annotationdata.has_id("D1"))
            self.assertTrue(annotationdata.key().has_id("pos")) #this is the most performant in comparisons, it doesn't make a copy of the key
            self.assertEqual(str(annotationdata.key()), "pos") #force a string
            self.assertEqual(annotationdata.dataset(), dataset)

            self.assertEqual(annotationdata.value().get(), "noun")
            self.assertTrue(annotationdata.test_value(DataValue("noun"))) #this is the most performant in comparisons, it doesn't make a copy of the value
            self.assertEqual(str(annotationdata.value()), "noun") #force a string
        self.assertEqual(count,1)

    def test_annotationset_iter_keys(self):
        """Iterate over all keys in an annotationset"""
        dataset = self.store.dataset("testdataset")
        count = 0
        for key in dataset.keys():
            count += 1
            #we can test in loop body because we only have one:
            self.assertIsInstance(key, DataKey)
            self.assertTrue(key.has_id("pos")) #this is the most performant in comparisons, it doesn't make a copy of the key
            self.assertEqual(key.dataset(), dataset)
        self.assertEqual(count,1)

    def test_annotationset_iter_data_by_key(self):
        """finds all annotation data that has key 'pos'"""
        dataset = self.store.dataset("testdataset")
        key = dataset.key("pos")
        count = 0
        for annotationdata in key.data():
            count += 1
            #we can test in loop body because we only have one:
            self.assertIsInstance(annotationdata, AnnotationData)
            self.assertTrue(annotationdata.has_id("D1"))
            self.assertTrue(annotationdata.key(),key) #this is the most performant in comparisons, it doesn't make a copy of the key
            self.assertEqual(str(annotationdata.key()), "pos") #force a string
            self.assertEqual(annotationdata.dataset(), dataset)

            self.assertEqual(annotationdata.value().get(), "noun")
            self.assertTrue(annotationdata.test_value(DataValue("noun"))) #this is the most performant in comparisons, it doesn't make a copy of the value
            self.assertEqual(str(annotationdata.value()), "noun") #force a string
        self.assertEqual(count,1)

    def test_annotations_by_data(self):
        """finds all annotations that have pos -> noun"""
        annotationset = self.store.dataset("testdataset")
        data = annotationset.annotationdata("D1")
        count = 0
        for annotation in data.annotations():
            count += 1
            #we can test in loop body because we only have one:
            self.assertIsInstance(annotation, Annotation)
            self.assertTrue(annotation.has_id("A1"))
        self.assertEqual(count,1)

    def test_find_data(self):
        """Find annotationdata by value"""
        dataset = self.store.dataset("testdataset")
        results = dataset.data(dataset.key("pos"), value="noun")
        self.assertIsInstance(results, Data)
        self.assertEqual(len(results), 1)
        self.assertIsInstance(results[0], AnnotationData)
        self.assertTrue(results[0].has_id("D1"))

    def test_find_data_from_key(self):
        """Find annotationdata by value, when key already known"""
        annotationset = self.store.dataset("testdataset")
        datakey = annotationset.key("pos")
        results = datakey.data(value="noun")
        self.assertIsInstance(results, Data)
        self.assertEqual(len(results), 1)
        self.assertIsInstance(results[0], AnnotationData)
        self.assertTrue(results[0].has_id("D1"))

    def test_find_data_missing(self):
        """Find annotationdata by value, test mismatches"""
        dataset = self.store.dataset("testdataset")
        results = dataset.data(dataset.key("pos"),value="non-existent")
        self.assertFalse(results) #empty evaluates to False

    def test_query(self):
        """Find annotationdata by value"""
        results = self.store.query("SELECT ANNOTATION ?a WHERE DATA \"testdataset\" \"pos\" = \"noun\";")
        self.assertIsInstance(results, list)
        self.assertEqual(len(results), 1)
        self.assertIsInstance(results[0], dict)
        self.assertIsInstance(results[0]["a"], Annotation)
        self.assertEqual(str(results[0]["a"]), "world")


class Test2(unittest.TestCase):
    def setUp(self):
        """Create some data from scratch"""
        #this is the very same data as in Test1, but constructed more implicitly via annotate()
        self.store = AnnotationStore(id="test")
        resource = self.store.add_resource(id="testres", text="Hello world")
        self.store.annotate(id="A1", 
                            target=Selector.textselector(resource, Offset.simple(6,11)),
                            data={ "id": "D1", "key": "pos", "value": "noun", "set": "testdataset"})

    def test_sanity_1(self):
        self.assertIsInstance( self.store, AnnotationStore)
        self.assertEqual(self.store.id(), "test")
        self.assertEqual(self.store.annotations_len(), 1)
        self.assertEqual(self.store.datasets_len(), 1)
        self.assertEqual(self.store.resources_len(), 1)

    def test_sanity_2(self):
        resource = self.store.resource("testres")
        self.assertIsInstance( resource, TextResource)
        self.assertEqual(resource.id(), "testres")
        self.assertTrue(resource.has_id("testres")) #quicker than the above (no copy)

    def test_sanity_3(self):
        dataset = self.store.dataset("testdataset")
        self.assertIsInstance( dataset, AnnotationDataSet)
        key = dataset.key("pos")
        self.assertIsInstance( key, DataKey)
        self.assertEqual(str(key), "pos")
        data = dataset.annotationdata("D1")
        self.assertIsInstance( data, AnnotationData)
        self.assertTrue(data.has_id("D1"))

    def test_serialisation_file(self):
        TMPDIR = environ.get('TMPDIR', "/tmp")
        filename = os.path.join(TMPDIR, "testoutput.stam.json")
        #doesn't test the actual output!
        self.store.to_file(filename)

    def test_serialisation_string(self):
        self.assertTrue(self.store.to_json_string()) #doesn't test the actual output!
 
EXAMPLE3JSON = """{
    "@type": "AnnotationStore",
    "annotationsets": [{
        "@type": "AnnotationDataSet",
        "@id": "testdataset",
        "keys": [
            {
              "@type": "DataKey",
              "@id": "pos"
            }
        ],
        "data": [
            {
                "@type": "AnnotationData",
                "@id": "D1",
                "key": "pos",
                "value": {
                    "@type": "String",
                    "value": "noun"
                }
            }
        ]
    }],
    "resources": [{
        "@id": "testres",
        "text": "Hello world"
    }],
    "annotations": [{
        "@type": "Annotation",
        "@id": "A1",
        "target": {
            "@type": "TextSelector",
            "resource": "testres",
            "offset": {
                "begin": {
                    "@type": "BeginAlignedCursor",
                    "value": 6
                },
                "end": {
                    "@type": "BeginAlignedCursor",
                    "value": 11
                }
            }
        },
        "data": [{
            "@type": "AnnotationData",
            "@id": "D1",
            "set": "testdataset"
        }]
    }]
}"""

def common_sanity(self): 
    self.assertIsInstance( self.store, AnnotationStore)
    self.assertEqual(self.store.annotations_len(), 1)
    self.assertEqual(self.store.datasets_len(), 1)
    self.assertEqual(self.store.resources_len(), 1)

    resource = self.store.resource("testres")
    self.assertIsInstance( resource, TextResource)
    self.assertEqual(resource.id(), "testres")
    self.assertTrue(resource.has_id("testres")) #quicker than the above (no copy)

    dataset = self.store.dataset("testdataset")
    self.assertIsInstance( dataset, AnnotationDataSet)
    key = dataset.key("pos")
    self.assertIsInstance( key, DataKey)
    self.assertEqual(str(key), "pos")
    data = dataset.annotationdata("D1")
    self.assertIsInstance( data, AnnotationData)
    self.assertTrue(data.has_id("D1"))

class Test3a(unittest.TestCase):
    def test_parse_file(self):
        TMPDIR = environ.get('TMPDIR', "/tmp")
        filename = os.path.join(TMPDIR, "test.stam.json")
        with open(filename, 'w',encoding='utf-8') as f:
            f.write(EXAMPLE3JSON)
        self.store = AnnotationStore(file=filename)

        #test all sanity
        common_sanity(self)


class Test3b(unittest.TestCase):
    def test_parse_file(self):
        self.store = AnnotationStore(string=EXAMPLE3JSON)

        #test all sanity
        common_sanity(self)

class Test4(unittest.TestCase):
    def setUp(self):
        """Create some data from scratch"""
        #this is the very same data as in Test1, but constructed more implicitly via annotate()
        self.store = AnnotationStore(id="test")
        resource = self.store.add_resource(id="testres", text="Hello world")
        self.store.annotate(id="A1", 
                            target=Selector.textselector(resource, Offset.simple(6,11)),
                            data={ "id": "D1", "key": "pos", "value": "noun", "set": "testdataset"})
        self.store.annotate(id="A2", 
                            target=Selector.textselector(resource, Offset.simple(0,5)),
                            data={ "id": "D2", "key": "pos", "value": "interjection", "set": "testdataset"})
        self.store.annotate(id="Word",
                            target=Selector.compositeselector(
                                Selector.annotationselector(self.store.annotation("A1"), Offset.whole()),
                                Selector.annotationselector(self.store.annotation("A2"), Offset.whole()),
                            ),
                            data={ "id": "D3", "key": "pos", "value": "word", "set": "testdataset"})

    def test_textselections_iter(self):
        resource = self.store.resource("testres")
        textselections = list(iter(resource))
        #print([ (x.begin(),x.end()) for x in textselections],file=sys.stderr)
        self.assertEqual(len(textselections), 2)
        self.assertEqual(str(textselections[0]), "Hello")
        self.assertEqual(str(textselections[1]), "world")

    def test_complexselector_iter(self):
        annotation = self.store.annotation("Word")

        #extract annotations we point to
        targetannotations = annotation.annotations_in_targets()
        self.assertIsInstance( targetannotations, Annotations)
        self.assertEqual( len(targetannotations), 2)
        #results are in textual order (which is deliberately counter to chronological order in this example)
        self.assertTrue( targetannotations[0].has_id("A2"))
        self.assertTrue( targetannotations[1].has_id("A1"))


        #extract textselections we point to
        textselections = annotation.textselections()
        self.assertEqual( len(textselections), 2)
        self.assertEqual(str(textselections[0]), "Hello")
        self.assertEqual(str(textselections[1]), "world")

    def test_textselections_by_annotations(self):
        annotation = self.store.annotation("A1")
        textselections = annotation.textselections()
        self.assertEqual(len(textselections), 1)
        self.assertIsInstance( textselections[0], TextSelection)
        self.assertEqual(str(textselections[0]), "world")

        #and the reverse:
        textselection = textselections[0]
        annotations = textselection.annotations()
        self.assertEqual(len(annotations), 2)
        self.assertIsInstance( annotations[0], Annotation)
        self.assertIsInstance( annotations[1], Annotation)
        self.assertEqual(annotations[0].id(), "A1")
        self.assertEqual(annotations[1].id(), "Word")

    def test_data_by_annotation(self):
        annotation = self.store.annotation("A1")
        data = annotation.data()
        self.assertEqual(len(data), 1)
        self.assertIsInstance( data[0], AnnotationData)
        self.assertEqual(data[0].id(), "D1")
        self.assertEqual(data[0].key().id(), "pos")
        self.assertEqual(str(data[0].value()), "noun")

class Test6(unittest.TestCase):
    def setUp(self):
        """Create some data from scratch"""
        #this is the very same data as in Test1, but constructed more implicitly via annotate()
        self.store = AnnotationStore(id="example6")
        resource = self.store.add_resource(id="humanrights", text="All human beings are born free and equal in dignity and rights.")
        self.store.annotate(id="Sentence1", 
                            target=Selector.textselector(resource, Offset.whole()),
                            data={"set": "testdataset", "key": "type", "value": "sentence"})
        self.store.annotate(id="Phrase1", 
                            target=Selector.textselector(resource, Offset.simple(17,40)),
                            data={"set": "testdataset", "key": "type", "value": "phrase"})

    def test_find_textselections_embedded(self):
        phrase1 = self.store.annotation("Phrase1")
        textselections = phrase1.related_text(TextSelectionOperator.embedded())
        self.assertIsInstance(textselections, TextSelections)
        self.assertEqual(len(textselections), 1)
        self.assertEqual(textselections[0].begin(), 0)
        self.assertEqual(textselections[0].end(), 63)

    def test_find_textselections_embeds(self):
        sentence1 = self.store.annotation("Sentence1")
        textselections = sentence1.related_text(TextSelectionOperator.embeds())
        self.assertEqual(len(textselections), 1)
        self.assertEqual(textselections[0].begin(), 17)
        self.assertEqual(textselections[0].end(), 40)

    def test_find_annotation_embedded(self):
        phrase1 = self.store.annotation("Phrase1")
        annotations = phrase1.related_text(TextSelectionOperator.embedded()).annotations()
        self.assertIsInstance(annotations, Annotations)
        self.assertEqual(len(annotations),1)
        self.assertEqual(annotations[0].id(), "Sentence1")

    def test_find_annotation_embeds(self):
        phrase1 = self.store.annotation("Sentence1")
        annotations = phrase1.related_text(TextSelectionOperator.embeds()).annotations()
        self.assertIsInstance(annotations, Annotations)
        self.assertEqual(len(annotations),1)
        self.assertEqual(annotations[0].id(), "Phrase1")

    def setup_example_6b(self):
        resource = self.store.resource("humanrights")
        phrase2 = self.store.annotate(Selector.textselector(resource, Offset.simple(4,25)), {"set": "testdataset", "key": "type", "value": "phrase"}, "Phrase2") #"human beings are born"
        phrase3 = self.store.annotate(Selector.textselector(resource, Offset.simple(44,62)), {"set": "testdataset", "key": "type", "value": "phrase"}, "Phrase3") #"dignity and rights"
        self.assertEqual(str(phrase2), "human beings are born")
        self.assertEqual(str(phrase3), "dignity and rights")

    def test_find_annotation_before(self):
        self.setup_example_6b()
        phrase2 = self.store.annotation("Phrase2")
        annotations = phrase2.related_text(TextSelectionOperator.before()).annotations()
        self.assertEqual(len(annotations),1)
        self.assertEqual(annotations[0].id(), "Phrase3")

    def test_find_annotation_after(self):
        self.setup_example_6b()
        phrase3 = self.store.annotation("Phrase3")
        annotations = phrase3.related_text(TextSelectionOperator.after()).annotations()
        self.assertEqual(len(annotations),2)
        self.assertTrue(any(annotation.id() == "Phrase2" for annotation in annotations))
        self.assertTrue(any(annotation.id() == "Phrase1" for annotation in annotations))


    def test_find_annotation_overlaps(self):
        self.setup_example_6b()
        phrase1 = self.store.annotation("Phrase1")
        annotations = phrase1.related_text(TextSelectionOperator.overlaps()).annotations()
        self.assertEqual(len(annotations),2)
        self.assertTrue(any(annotation.id() == "Phrase2" for annotation in annotations))
        self.assertTrue(any(annotation.id() == "Sentence1" for annotation in annotations))

    def test_find_annotation_overlaps_2(self):
        self.setup_example_6b()
        phrase2 = self.store.annotation("Phrase2")
        annotations = phrase2.related_text(TextSelectionOperator.overlaps()).annotations()
        self.assertEqual(len(annotations),2)
        self.assertTrue(any(annotation.id() == "Phrase1" for annotation in annotations))
        self.assertTrue(any(annotation.id() == "Sentence1" for annotation in annotations))

class Test7Regex(unittest.TestCase):
    def setUp(self):
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
        self.store = AnnotationStore(id="tutorial")
        self.resource = self.store.add_resource(id="banks", text=text)

    def test_regex_tokens(self):
        expressions = [
            r"\w+(?:[-_]\w+)*", #this detects words,possibly with hyphens or underscores as part of it
            r"[\.\?,/]+", #this detects a variety of punctuation
            r"[0-9]+(?:[,\.][0-9]+)", #this detects numbers, possibly with a fractional part
        ]
        structuretypes = ["word", "punctuation", "number"]

        for i, matchresult in enumerate(self.resource.find_text_regex(expressions)):
            #(we only have one textselection per match, but an regular expression may result in multiple textselections if capture groups are used)
            textselection = matchresult['textselections'][0]
            structuretype = structuretypes[matchresult['expression_index']]
            #print(f"Annotating \"{textselection}\" at {textselection.offset()} as {structuretype}", file=sys.stderr)
            self.store.annotate(
                target=Selector.textselector(self.resource, textselection.offset()),
                data=[ 
                    {"key": "structuretype", "value": structuretype, "set": "tutorial-set" }
                ],
                id=f"AnnotationToken{i+1}")

        period = self.resource.textselection(Offset.simple(35,36))
        self.assertTrue(str(period),".")
        annotation = period.annotations()[0]
        self.assertTrue(any(data.key().id() == "structuretype" and str(data.value()) == "punctuation" for data in annotation))





if __name__ == "__main__":
    unittest.main()

