class AnnotationStore:
    """
    An Annotation Store is an unordered collection of annotations, resources and
    annotation data sets. It can be seen as the *root* of the *graph model* and the glue
    that holds everything together. It is the entry point for any stam model.
    
    Args:
        `id` (:obj:`str`, `optional`) - The public ID for a *new* store
        `file` (:obj:`str`, `optional`) - The STAM JSON or STAM CSV file to load
        `string` (:obj:`str`, `optional`) - STAM JSON as a string
        `config` (:obj:`dict`, `optional`) - A python dictionary containing configuration parameters
    
    At least one of `id`, `file` or `string` must be specified.
    """
    def __init__(self, id=None,file=None, string=None,config=None):
        pass



