[`rst-duplicate-param`](../src/rules/docstrings/rst_duplicate_param.md) follows the docstring Sphinx renders for each
class, resolved per project from a `:class-doc-from:` option on its `autoclass` directive, then
`autodoc_default_options`, then `autoclass_content` in the project's Sphinx configuration. Under `init` the rule reports
the copy in the class docstring and removes that one, and under `both` it keeps reporting the `__init__` copy. A
configuration counts when it sits beside an index page or names `extensions`, `master_doc` or `root_doc`. A value the
rule cannot read leaves every copy alone.
