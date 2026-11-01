`definition-before-use` (SMP020) no longer moves a class below a decorated function or class that names it in an
annotation, such as a pydantic model in a FastAPI route's return annotation: the decorator reads the annotation while
the module loads, so after the move it met an unresolved forward reference. A name a function uses only in annotations
inside its body no longer counts as a helper to place below it.
