[function-docstring](../src/rules/docstrings/function_docstring.md) drops `Params:` and `Yield:` sections when it turns
a docstring into a comment, as the reST docstring rules already read them, and keeps a line such as `: name: text`,
which reST does not parse as a field.
