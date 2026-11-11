[`function-docstring`](../src/rules/docstrings/function_docstring.md) (SMP901) and
[`test-module-docstring`](../src/rules/docstrings/test_module_docstring.md) (SMP902) keep a comment after a removed
docstring on its own line, which before their fix deleted it along with the docstring. A tool directive such as
`# fmt: off`, or a comment holding one, goes with the docstring, as a pragma does; before, it stayed on its own line and
changed what the formatter does.
