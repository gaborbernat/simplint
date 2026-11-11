[`function-docstring`](../src/rules/docstrings/function_docstring.md) (SMP901) keeps the docstring of a class whose
body, or the header of a method, lambda or nested class in it, reads `__doc__`, which is that docstring there. Removing
it changed the value the class reads.
